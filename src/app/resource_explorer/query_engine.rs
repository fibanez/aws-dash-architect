//! Modular Resource Query Engine
//!
//! Decouples resource querying logic from UI state management via callback-based architecture.
//! This allows the same query engine to be used by:
//! - UI panes (via UIQueryAdapter)
//! - Agent framework (via AgentQueryAdapter)
//! - Future webview integration
//!
//! Reference implementation: window.rs:3476-3815 (spawn_parallel_query)

use super::aws_client::AWSResourceClient;
use super::cache::SharedResourceCache;
use super::state::{QueryScope, ResourceEntry};
use super::GlobalServiceRegistry;
use std::sync::Arc;

/// Progress events emitted during query execution
///
/// Consumers register callbacks to receive these events and update their state accordingly.
#[derive(Debug, Clone)]
pub enum QueryProgress {
    /// Phase 1 started - resource listing beginning
    Phase1Started {
        /// Total number of queries expected (account × region × resource_type)
        total_queries: usize,
        /// List of query keys to track (format: "account_id:region:resource_type")
        query_keys: Vec<String>,
    },

    /// A single Phase 1 query completed successfully
    Phase1QueryCompleted {
        /// Query key that completed (format: "account_id:region:resource_type")
        query_key: String,
        /// Cumulative resources collected so far (all queries combined)
        /// Arc wrapper reduces memory overhead from repeated clones
        resources: Arc<Vec<ResourceEntry>>,
    },

    /// A single Phase 1 query failed
    Phase1QueryFailed {
        /// Query key that failed (format: "account_id:region:resource_type")
        query_key: String,
        /// Error message
        error: String,
    },

    /// Tag fetching progress (part of Phase 1)
    TagFetchingProgress {
        /// Resource type being enriched with tags
        resource_type: String,
        /// Number of items processed so far
        items_processed: usize,
        /// Estimated total items
        estimated_total: usize,
    },

    /// Tag fetching completed for a resource type
    TagFetchingCompleted {
        /// Resource type that finished tag fetching
        resource_type: String,
    },

    /// Phase 1 completed - all resource listing finished
    Phase1Completed {
        /// Total resources collected
        /// Arc wrapper reduces memory overhead from repeated clones
        resources: Arc<Vec<ResourceEntry>>,
    },

    /// Phase 2 started - detailed properties enrichment beginning
    Phase2Started {
        /// Total number of resources to enrich
        total_resources: usize,
    },

    /// Phase 2 progress update - enriching resources with detailed properties
    Phase2Progress {
        /// Resource type being enriched (e.g., "AWS::S3::Bucket")
        resource_type: String,
        /// Number of resources processed so far
        processed: usize,
        /// Total resources to process
        total: usize,
    },

    /// Phase 2 completed - all detailed properties fetched
    Phase2Completed {
        /// Resources with detailed_properties populated
        /// Arc wrapper reduces memory overhead from repeated clones
        resources: Arc<Vec<ResourceEntry>>,
    },

    /// Entire query process completed (Phase 1 + potential Phase 2)
    Completed {
        /// Final resource list
        resources: Vec<ResourceEntry>,
    },

    /// Query execution failed
    Failed {
        /// Error message
        error: String,
    },
}

/// Handle for a running query (future: cancellation support)
#[derive(Debug, Clone)]
pub struct QueryHandle {
    /// Unique identifier for this query
    pub cache_key: String,
}

/// Modular resource query engine - UI-independent
///
/// Executes AWS resource queries and reports progress via callbacks.
/// Does NOT directly mutate any UI state.
pub struct ResourceQueryEngine {
    /// AWS client for making API calls
    aws_client: Arc<AWSResourceClient>,
    /// Shared cache for query results
    cache: Arc<SharedResourceCache>,
}

impl ResourceQueryEngine {
    /// Create a new query engine
    pub fn new(aws_client: Arc<AWSResourceClient>, cache: Arc<SharedResourceCache>) -> Self {
        Self {
            aws_client,
            cache,
        }
    }

    /// Update the AWS client (called when user changes credentials)
    pub fn set_aws_client(&mut self, aws_client: Arc<AWSResourceClient>) {
        self.aws_client = aws_client;
    }

    /// Execute a resource query with callback-based progress reporting
    ///
    /// Based on window.rs:3476-3815 (spawn_parallel_query)
    ///
    /// # Arguments
    /// - `scope`: Query scope (accounts, regions, resource types)
    /// - `cache_key`: Unique identifier for this query (for loading state tracking)
    /// - `progress_callback`: Called for each progress event
    ///
    /// # Returns
    /// QueryHandle for future cancellation support
    pub fn execute_query<F>(
        &self,
        scope: QueryScope,
        cache_key: String,
        progress_callback: F,
    ) -> QueryHandle
    where
        F: Fn(QueryProgress) + Send + Sync + 'static,
    {
        // Wrap callback in Arc for sharing between tasks
        let progress_callback = Arc::new(progress_callback);
        // Build list of query keys to track for Phase 1 progress
        // Based on window.rs:3483-3514
        let query_keys = Self::build_query_keys(&scope);

        // Notify Phase 1 started
        progress_callback(QueryProgress::Phase1Started {
            total_queries: query_keys.len(),
            query_keys: query_keys.clone(),
        });

        // Clone resources for async operation
        let aws_client = self.aws_client.clone();
        let cache = self.cache.clone();
        let cache_key_clone = cache_key.clone();

        // Spawn query thread (based on window.rs:3536-3814)
        std::thread::spawn(move || {
            // Check memory budget before starting query
            if let Err(msg) = super::memory_budget::MemoryBudget::get().check_usage() {
                tracing::warn!("Memory budget exceeded before query start: {}", msg);
                progress_callback(QueryProgress::Failed {
                    error: "Dash can only use up to 80% of your memory".to_string(),
                });
                return;
            }

            let runtime = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create Tokio runtime: {}", e);
                    progress_callback(QueryProgress::Failed {
                        error: format!("Failed to create Tokio runtime: {}", e),
                    });
                    return;
                }
            };

            let result: Result<Vec<ResourceEntry>, anyhow::Error> = runtime.block_on(async {
                let (result_sender, mut result_receiver) =
                    tokio::sync::mpsc::channel::<super::aws_client::QueryResult>(1000);
                let (progress_sender, mut progress_receiver) =
                    tokio::sync::mpsc::channel::<super::aws_client::QueryProgress>(100);

                let aws_client_clone = aws_client.clone();
                let scope_clone = scope.clone();
                let cache_clone = cache.clone();

                let query_future = aws_client_clone.query_aws_resources_parallel(
                    &scope_clone,
                    result_sender,
                    Some(progress_sender),
                    cache_clone,
                );

                let all_resources = Arc::new(tokio::sync::Mutex::new(Vec::new()));
                let all_resources_clone = all_resources.clone();

                // Result processing task (based on window.rs:3601-3687)
                let progress_cb_for_results = progress_callback.clone();
                let result_processing = async move {
                    tracing::info!("Query Engine: result_processing task started");
                    let mut result_count = 0;

                    while let Some(result) = result_receiver.recv().await {
                        result_count += 1;
                        tracing::info!("Query Engine: Received result #{} from channel", result_count);

                        // Build full query key for Phase 1 tracking
                        let query_key = make_query_key(
                            &result.account_id,
                            &result.region,
                            &result.resource_type,
                        );

                        match result.resources {
                            Ok(resources) => {
                                tracing::info!("Query Engine: Processing {} resources for {}", resources.len(), query_key);

                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!("query_callback.before_lock", &query_key);

                                let mut all_res = all_resources_clone.lock().await;

                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!("query_callback.after_lock", &query_key);

                                all_res.extend(resources);

                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!(
                                    "query_callback.after_extend",
                                    &format!("{} (count: {})", &query_key, all_res.len())
                                );

                                // Share ONE Arc across callbacks (not creating new Arc each time)
                                let resources_arc = Arc::new(all_res.clone());

                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!("query_callback.after_arc_new", &query_key);

                                // Release lock before callback to avoid contention
                                drop(all_res);

                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!("query_callback.before_callback", &query_key);

                                tracing::info!("Query Engine: Invoking Phase1QueryCompleted callback for {}", query_key);
                                // Callback: query completed
                                progress_cb_for_results(QueryProgress::Phase1QueryCompleted {
                                    query_key: query_key.clone(),
                                    resources: resources_arc,
                                });

                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!("query_callback.after_callback", &query_key);

                                tracing::info!("Query Engine: Phase1QueryCompleted callback invoked for {}", query_key);
                            }
                            Err(e) => {
                                tracing::error!("Query failed for {}: {}", query_key, e);

                                // Performance timing: Log query failure for debugging
                                #[cfg(debug_assertions)]
                                {
                                    crate::perf_checkpoint!(
                                        &format!("QUERY_FAILED.{}", &query_key),
                                        &format!("{}", e)
                                    );
                                }

                                // Callback: query failed
                                progress_cb_for_results(QueryProgress::Phase1QueryFailed {
                                    query_key: query_key.clone(),
                                    error: format!("{}", e),
                                });
                            }
                        }
                    }

                    let final_count = all_resources_clone.lock().await.len();
                    tracing::info!(
                        "Query Engine: Result receiver channel closed, collected {} total resources from {} results",
                        final_count, result_count
                    );
                };

                // Progress processing task (based on window.rs:3689-3739)
                let progress_cb_for_progress = progress_callback.clone();
                let progress_processing = async move {
                    while let Some(progress) = progress_receiver.recv().await {
                        tracing::debug!(
                            "Progress: {} - {} - {} (status: {:?})",
                            progress.account,
                            progress.region,
                            progress.message,
                            progress.status
                        );

                        // Handle FetchingTags progress
                        if matches!(
                            progress.status,
                            super::aws_client::QueryStatus::FetchingTags
                        ) {
                            tracing::info!(
                                "FetchingTags progress: {} ({}/{})",
                                progress.resource_type,
                                progress.items_processed.unwrap_or(0),
                                progress.estimated_total.unwrap_or(0)
                            );

                            progress_cb_for_progress(QueryProgress::TagFetchingProgress {
                                resource_type: progress.resource_type.clone(),
                                items_processed: progress.items_processed.unwrap_or(0),
                                estimated_total: progress.estimated_total.unwrap_or(0),
                            });
                        } else if matches!(
                            progress.status,
                            super::aws_client::QueryStatus::Completed
                                | super::aws_client::QueryStatus::Failed
                        ) {
                            // Tag fetching completed for this resource type
                            progress_cb_for_progress(QueryProgress::TagFetchingCompleted {
                                resource_type: progress.resource_type.clone(),
                            });
                        }
                    }

                    tracing::debug!("Progress receiver channel closed");
                };

                // Run query and processing tasks (based on window.rs:3741-3758)
                tracing::info!("Query Engine: Starting tokio::join! for query_future + result_processing + progress_processing");
                tokio::join!(
                    async {
                        tracing::info!("Query Engine: query_future branch started");
                        match query_future.await {
                            Ok(()) => {
                                tracing::info!("Query Engine: query_future completed successfully")
                            }
                            Err(e) => tracing::error!("Parallel query execution failed: {}", e),
                        }
                        tracing::info!("Query Engine: query_future branch exiting");
                    },
                    async {
                        tracing::info!("Query Engine: result_processing branch wrapper started");
                        result_processing.await;
                        tracing::info!("Query Engine: result_processing branch wrapper exiting");
                    },
                    async {
                        tracing::info!("Query Engine: progress_processing branch wrapper started");
                        progress_processing.await;
                        tracing::info!("Query Engine: progress_processing branch wrapper exiting");
                    }
                );

                tracing::info!("Query Engine: tokio::join! completed, all branches finished");
                let final_resources = all_resources.lock().await.clone();
                tracing::info!("Query Engine: Locked and cloned final_resources ({} items)", final_resources.len());

                Ok(final_resources)
            });

            // Handle query completion (based on window.rs:3761-3813)
            match result {
                Ok(mut resources) => {
                    tracing::debug!(
                        "Parallel query completed: {} total resources for cache_key={}",
                        resources.len(),
                        cache_key_clone
                    );

                    // Callback: Phase 1 completed
                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("phase1_completed.before_arc_new", &cache_key_clone);

                    let resources_arc = Arc::new(resources.clone());

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("phase1_completed.before_callback", &cache_key_clone);

                    progress_callback(QueryProgress::Phase1Completed {
                        resources: resources_arc,
                    });

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("phase1_completed.after_callback", &cache_key_clone);

                    // Check if any resources need Phase 2 enrichment
                    let enrichable_types = super::state::ResourceExplorerState::enrichable_resource_types();
                    let resources_needing_enrichment: Vec<ResourceEntry> = resources
                        .iter()
                        .filter(|r| {
                            enrichable_types.contains(&r.resource_type.as_str())
                                && r.detailed_timestamp.is_none()
                        })
                        .cloned()
                        .collect();

                    if !resources_needing_enrichment.is_empty() {
                        tracing::info!(
                            "Starting Phase 2 enrichment for {} resources",
                            resources_needing_enrichment.len()
                        );

                        // Check memory budget before Phase 2
                        if let Err(msg) = super::memory_budget::MemoryBudget::get().check_usage() {
                            tracing::warn!("Memory budget exceeded before Phase 2: {}", msg);
                            progress_callback(QueryProgress::Failed {
                                error: "Dash can only use up to 80% of your memory".to_string(),
                            });
                            return;
                        }

                        // Callback: Phase 2 started
                        progress_callback(QueryProgress::Phase2Started {
                            total_resources: resources_needing_enrichment.len(),
                        });

                        // Execute Phase 2 enrichment
                        let enrichment_result = runtime.block_on(async {
                            Self::execute_phase2_enrichment(
                                aws_client.clone(),
                                cache.clone(),
                                resources_needing_enrichment,
                                progress_callback.clone(),
                            )
                            .await
                        });

                        match enrichment_result {
                            Ok(enriched) => {
                                tracing::info!("Phase 2 enrichment completed: {} resources enriched", enriched.len());

                                // Merge enriched resources back into the main list
                                // The enriched resources have merged properties (raw + detailed)
                                for enriched_resource in enriched {
                                    if let Some(existing) = resources.iter_mut().find(|r| {
                                        r.account_id == enriched_resource.account_id
                                            && r.region == enriched_resource.region
                                            && r.resource_type == enriched_resource.resource_type
                                            && r.resource_id == enriched_resource.resource_id
                                    }) {
                                        // Replace properties with merged version
                                        existing.properties = enriched_resource.properties;
                                        existing.detailed_timestamp = enriched_resource.detailed_timestamp;
                                    }
                                }

                                // Callback: Phase 2 completed
                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!("phase2_completed.before_arc_new", &cache_key_clone);

                                let resources_arc = Arc::new(resources.clone());

                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!("phase2_completed.before_callback", &cache_key_clone);

                                progress_callback(QueryProgress::Phase2Completed {
                                    resources: resources_arc,
                                });

                                #[cfg(debug_assertions)]
                                crate::perf_checkpoint!("phase2_completed.after_callback", &cache_key_clone);

                                // Memory checkpoint: After Phase 2 query completion
                                crate::app::memory_profiling::memory_checkpoint(
                                    &format!("after_phase2_complete_{}_resources", resources.len())
                                );
                            }
                            Err(e) => {
                                tracing::error!("Phase 2 enrichment failed: {}", e);
                                // Continue with Phase 1 results even if Phase 2 fails
                            }
                        }
                    }

                    // Callback: Entire query completed
                    progress_callback(QueryProgress::Completed { resources });
                }
                Err(e) => {
                    tracing::error!("Failed to execute parallel queries: {}", e);

                    // Callback: Query failed
                    progress_callback(QueryProgress::Failed {
                        error: format!("{}", e),
                    });
                }
            }
        });

        QueryHandle { cache_key }
    }

    /// Execute Phase 2 enrichment - fetch detailed properties for enrichable resources
    ///
    /// Based on window.rs:4202-4432 and aws_client.rs:4337-4586
    async fn execute_phase2_enrichment<F>(
        aws_client: Arc<AWSResourceClient>,
        cache: Arc<SharedResourceCache>,
        resources: Vec<ResourceEntry>,
        progress_callback: Arc<F>,
    ) -> anyhow::Result<Vec<ResourceEntry>>
    where
        F: Fn(QueryProgress) + Send + Sync + 'static,
    {
        use futures::stream::{FuturesUnordered, StreamExt};

        let total = resources.len();
        tracing::info!("Phase 2: Enriching {} resources with detailed properties", total);

        // Build (cache_key, resource) pairs to track which cache entry each resource belongs to
        let mut work_items: Vec<(String, ResourceEntry)> = Vec::with_capacity(total);
        for resource in resources {
            let cache_region = if super::global_services::is_global_service(&resource.resource_type) {
                "Global".to_string()
            } else {
                resource.region.clone()
            };
            let cache_key = format!(
                "{}:{}:{}",
                resource.account_id, cache_region, resource.resource_type
            );
            work_items.push((cache_key, resource));
        }

        // Group work items by cache_key for efficient cache updates
        let mut items_by_cache_key: std::collections::HashMap<String, Vec<ResourceEntry>> =
            std::collections::HashMap::new();
        for (cache_key, resource) in work_items {
            items_by_cache_key
                .entry(cache_key)
                .or_insert_with(Vec::new)
                .push(resource);
        }

        let mut all_enriched = Vec::new();
        let mut processed = 0;

        // Process each cache key sequentially
        for (cache_key, cache_resources) in items_by_cache_key {
            let type_total = cache_resources.len();
            let resource_type = cache_resources[0].resource_type.clone();
            tracing::info!(
                "Phase 2: Enriching {} resources for cache_key: {}",
                type_total,
                cache_key
            );

            // Create futures for parallel processing within this cache key
            let mut futures = FuturesUnordered::new();
            for resource in cache_resources {
                let client = aws_client.clone();
                let resource_clone = resource.clone();

                futures.push(async move {
                    // Fetch detailed properties
                    match client.describe_resource(&resource_clone).await {
                        Ok(detailed) => {
                            let mut enriched = resource_clone.clone();
                            let now = chrono::Utc::now();

                            // Merge Phase 2 detailed properties on top of existing Phase 1 properties
                            // properties already contains complete Phase 1 data
                            if let Some(properties_obj) = enriched.properties.as_object_mut() {
                                if let Some(detailed_obj) = detailed.as_object() {
                                    for (key, value) in detailed_obj {
                                        properties_obj.insert(key.clone(), value.clone());
                                    }
                                }
                            }
                            enriched.detailed_timestamp = Some(now);

                            tracing::debug!(
                                "Phase 2: Merged properties for {} ({})",
                                enriched.resource_id,
                                enriched.resource_type
                            );

                            Ok(enriched)
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to fetch detailed properties for {}: {}",
                                resource_clone.resource_id,
                                e
                            );
                            Err(e)
                        }
                    }
                });
            }

            // Collect enriched resources for this cache key
            let mut enriched_for_cache_key = Vec::new();
            let mut type_enriched = 0;
            while let Some(result) = futures.next().await {
                match result {
                    Ok(enriched_resource) => {
                        enriched_for_cache_key.push(enriched_resource.clone());
                        all_enriched.push(enriched_resource);
                        type_enriched += 1;
                        processed += 1;

                        // Report progress periodically (every 5 resources or at completion)
                        if type_enriched % 5 == 0 || type_enriched == type_total {
                            progress_callback(QueryProgress::Phase2Progress {
                                resource_type: resource_type.clone(),
                                processed,
                                total,
                            });
                        }
                    }
                    Err(_) => {
                        processed += 1;
                        // Continue with other resources even if one fails
                    }
                }
            }

            tracing::info!(
                "Phase 2: Completed {} - enriched {}/{} resources",
                resource_type,
                type_enriched,
                type_total
            );

            // Update the main resource cache with enriched resources
            // This is critical - without this, the cache still has old resources without merged properties
            if !enriched_for_cache_key.is_empty() {
                // Get current cached resources
                if let Some(mut cached_resources) = cache.get_resources_owned(&cache_key) {
                    // Build lookup map of enriched resources for O(1) access
                    let mut enriched_lookup: std::collections::HashMap<String, ResourceEntry> =
                        std::collections::HashMap::new();
                    for enriched in enriched_for_cache_key {
                        enriched_lookup.insert(enriched.resource_id.clone(), enriched);
                    }

                    // Update cached resources with merged properties from enriched resources
                    for cached in &mut cached_resources {
                        if let Some(enriched) = enriched_lookup.get(&cached.resource_id) {
                            // Replace properties with merged version (raw + detailed)
                            cached.properties = enriched.properties.clone();
                            cached.detailed_timestamp = enriched.detailed_timestamp;
                        }
                    }

                    // Write back the modified list to cache
                    cache.insert_resources_owned(cache_key.clone(), cached_resources);
                    tracing::info!(
                        "Phase 2: Updated cache for key {} with {} enriched resources (merged properties)",
                        cache_key,
                        enriched_lookup.len()
                    );
                } else {
                    tracing::warn!(
                        "Phase 2: Cache key {} not found when trying to write back enriched resources",
                        cache_key
                    );
                }
            }
        }

        Ok(all_enriched)
    }

    /// Build list of query keys for progress tracking
    ///
    /// Based on window.rs:3483-3514
    ///
    /// Format: "account_id:region:resource_type"
    /// - Global services: "account_id:Global:resource_type"
    /// - Regional services: "account_id:region_code:resource_type"
    fn build_query_keys(scope: &QueryScope) -> Vec<String> {
        let global_registry = GlobalServiceRegistry::new();
        let mut queries_to_track: Vec<String> = Vec::new();

        for account in &scope.accounts {
            for resource_type in &scope.resource_types {
                if global_registry.is_global(&resource_type.resource_type) {
                    // Global services: one query per account with region "Global"
                    let query_key =
                        make_query_key(&account.account_id, "Global", &resource_type.resource_type);

                    if !queries_to_track.contains(&query_key) {
                        queries_to_track.push(query_key);
                    }
                } else {
                    // Regional services: one query per account × region
                    for region in &scope.regions {
                        let query_key = make_query_key(
                            &account.account_id,
                            &region.region_code,
                            &resource_type.resource_type,
                        );
                        queries_to_track.push(query_key);
                    }
                }
            }
        }

        queries_to_track
    }
}

/// Build a query key for tracking
///
/// Format: "account_id:region:resource_type"
fn make_query_key(account_id: &str, region: &str, resource_type: &str) -> String {
    format!("{}:{}:{}", account_id, region, resource_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::resource_explorer::state::{
        AccountSelection, RegionSelection, ResourceTypeSelection,
    };

    #[test]
    fn test_build_query_keys_regional_services() {
        let scope = QueryScope {
            accounts: vec![
                AccountSelection {
                    account_id: "111111111111".to_string(),
                    display_name: "Account 1".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
                AccountSelection {
                    account_id: "222222222222".to_string(),
                    display_name: "Account 2".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
            ],
            regions: vec![
                RegionSelection {
                    region_code: "us-east-1".to_string(),
                    display_name: "US East (N. Virginia)".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
                RegionSelection {
                    region_code: "eu-west-1".to_string(),
                    display_name: "EU (Ireland)".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
            ],
            resource_types: vec![ResourceTypeSelection {
                resource_type: "AWS::EC2::Instance".to_string(),
                display_name: "EC2 Instances".to_string(),
                service_name: "EC2".to_string(),
            }],
        };

        let query_keys = ResourceQueryEngine::build_query_keys(&scope);

        // Regional service: 2 accounts × 2 regions × 1 resource type = 4 queries
        assert_eq!(query_keys.len(), 4);
        assert!(query_keys.contains(&"111111111111:us-east-1:AWS::EC2::Instance".to_string()));
        assert!(query_keys.contains(&"111111111111:eu-west-1:AWS::EC2::Instance".to_string()));
        assert!(query_keys.contains(&"222222222222:us-east-1:AWS::EC2::Instance".to_string()));
        assert!(query_keys.contains(&"222222222222:eu-west-1:AWS::EC2::Instance".to_string()));
    }

    #[test]
    fn test_build_query_keys_global_services() {
        let scope = QueryScope {
            accounts: vec![
                AccountSelection {
                    account_id: "111111111111".to_string(),
                    display_name: "Account 1".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
                AccountSelection {
                    account_id: "222222222222".to_string(),
                    display_name: "Account 2".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
            ],
            regions: vec![
                RegionSelection {
                    region_code: "us-east-1".to_string(),
                    display_name: "US East (N. Virginia)".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
                RegionSelection {
                    region_code: "eu-west-1".to_string(),
                    display_name: "EU (Ireland)".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
            ],
            resource_types: vec![ResourceTypeSelection {
                resource_type: "AWS::S3::Bucket".to_string(),
                display_name: "S3 Buckets".to_string(),
                service_name: "S3".to_string(),
            }],
        };

        let query_keys = ResourceQueryEngine::build_query_keys(&scope);

        // Global service: 2 accounts × 1 global region × 1 resource type = 2 queries
        assert_eq!(query_keys.len(), 2);
        assert!(query_keys.contains(&"111111111111:Global:AWS::S3::Bucket".to_string()));
        assert!(query_keys.contains(&"222222222222:Global:AWS::S3::Bucket".to_string()));
    }

    #[test]
    fn test_build_query_keys_mixed_services() {
        let scope = QueryScope {
            accounts: vec![AccountSelection {
                account_id: "111111111111".to_string(),
                display_name: "Account 1".to_string(),
                color: egui::Color32::from_rgb(100, 150, 200),
            }],
            regions: vec![
                RegionSelection {
                    region_code: "us-east-1".to_string(),
                    display_name: "US East (N. Virginia)".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
                RegionSelection {
                    region_code: "eu-west-1".to_string(),
                    display_name: "EU (Ireland)".to_string(),
                    color: egui::Color32::from_rgb(100, 150, 200),
                },
            ],
            resource_types: vec![
                ResourceTypeSelection {
                    resource_type: "AWS::EC2::Instance".to_string(),
                    display_name: "EC2 Instances".to_string(),
                    service_name: "EC2".to_string(),
                },
                ResourceTypeSelection {
                    resource_type: "AWS::S3::Bucket".to_string(),
                    display_name: "S3 Buckets".to_string(),
                    service_name: "S3".to_string(),
                },
            ],
        };

        let query_keys = ResourceQueryEngine::build_query_keys(&scope);

        // 1 account × (2 regions × 1 regional + 1 global) = 2 + 1 = 3 queries
        assert_eq!(query_keys.len(), 3);
        assert!(query_keys.contains(&"111111111111:us-east-1:AWS::EC2::Instance".to_string()));
        assert!(query_keys.contains(&"111111111111:eu-west-1:AWS::EC2::Instance".to_string()));
        assert!(query_keys.contains(&"111111111111:Global:AWS::S3::Bucket".to_string()));
    }

    #[test]
    fn test_make_query_key() {
        assert_eq!(
            make_query_key("123456789012", "us-east-1", "AWS::EC2::Instance"),
            "123456789012:us-east-1:AWS::EC2::Instance"
        );

        assert_eq!(
            make_query_key("123456789012", "Global", "AWS::S3::Bucket"),
            "123456789012:Global:AWS::S3::Bucket"
        );
    }
}
