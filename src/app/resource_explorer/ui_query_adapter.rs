//! UI Query Adapter - Bridges query engine to pane state updates
//!
//! This adapter translates callback-based QueryProgress events from the query engine
//! into ResourceExplorerState updates for UI rendering. It handles:
//! - Lock contention retry logic (10 retries, 5ms sleep)
//! - UI repaint requests via egui::Context
//! - Phase 1/Phase 2 progress tracking
//! - Tag fetching progress updates
//!
//! Reference implementation: window.rs:3615-3680 (retry logic for state updates)

use super::query_engine::{QueryHandle, QueryProgress, ResourceQueryEngine};
use super::state::{QueryScope, ResourceExplorerState};
use std::sync::Arc;
use tokio::sync::RwLock;

/// UI-specific adapter for the query engine
///
/// Connects QueryProgress callbacks to ResourceExplorerState updates with
/// retry logic for lock contention.
pub struct UIQueryAdapter {
    /// The underlying query engine
    engine: Arc<ResourceQueryEngine>,
}

impl UIQueryAdapter {
    /// Create a new UI query adapter
    pub fn new(engine: Arc<ResourceQueryEngine>) -> Self {
        Self { engine }
    }

    /// Execute a query for a pane with UI-specific state updates
    ///
    /// This method:
    /// 1. Spawns a query via the engine
    /// 2. Registers callbacks that update the pane's ResourceExplorerState
    /// 3. Handles lock contention with retry logic (10 retries, 5ms sleep)
    /// 4. Requests UI repaints after each state update
    ///
    /// Based on window.rs:3476-3815 (spawn_parallel_query)
    /// and window.rs:3615-3680 (retry logic)
    ///
    /// # Arguments
    /// - `pane_state`: Arc to the pane's ResourceExplorerState
    /// - `scope`: Query scope (accounts, regions, resource types)
    /// - `cache_key`: Unique identifier for this query
    /// - `ctx`: egui Context for requesting repaints
    ///
    /// # Returns
    /// QueryHandle for future cancellation support
    pub fn execute_for_pane(
        &self,
        pane_state: Arc<RwLock<ResourceExplorerState>>,
        scope: QueryScope,
        cache_key: String,
        ctx: egui::Context,
    ) -> QueryHandle {
        // Clone for use in callbacks
        let pane_state_for_callback = pane_state.clone();
        let ctx_for_callback = ctx.clone();
        let cache_key_for_callback = cache_key.clone();

        // Create progress callback that updates pane state
        let progress_callback = move |progress: QueryProgress| {
            match progress {
                QueryProgress::Phase1Started {
                    total_queries,
                    query_keys,
                } => {
                    // Initialize Phase 1 tracking in pane state
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            state.start_phase1_tracking(query_keys);
                            tracing::info!(
                                "Phase 1 started: {} queries expected",
                                total_queries
                            );
                        },
                    );
                }

                QueryProgress::Phase1QueryCompleted {
                    query_key,
                    resources,
                } => {
                    tracing::info!("UI Adapter: Phase1QueryCompleted callback received for {}", query_key);

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase1_query.before_unwrap", &query_key);

                    // Clone query_key for use after closure
                    let query_key_clone = query_key.clone();

                    // Try to unwrap Arc, falling back to clone if other references exist
                    let resources_vec = std::sync::Arc::try_unwrap(resources)
                        .unwrap_or_else(|arc| {
                            #[cfg(debug_assertions)]
                            crate::perf_checkpoint!("ui_adapter.phase1_query.arc_clone_fallback", &query_key);
                            (*arc).clone()
                        });

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase1_query.after_unwrap", &query_key);

                    // Update resources and mark query completed
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            #[cfg(debug_assertions)]
                            crate::perf_checkpoint!("ui_adapter.phase1_query.in_state_update", &query_key);

                            tracing::info!("UI Adapter: Updating state with {} resources", resources_vec.len());
                            state.resources = resources_vec;
                            state.mark_phase1_query_completed(&query_key);
                            // Increment enrichment version to invalidate tree cache
                            state.increment_enrichment_version_force();
                        },
                    );

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase1_query.after_state_update", &query_key_clone);

                    tracing::info!("UI Adapter: Phase1QueryCompleted state update requested");
                }

                QueryProgress::Phase1QueryFailed { query_key, error } => {
                    // Mark query as failed
                    tracing::error!("Query failed for {}: {}", query_key, error);
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            state.mark_phase1_query_failed(&query_key);
                        },
                    );
                }

                QueryProgress::TagFetchingProgress {
                    resource_type,
                    items_processed,
                    estimated_total,
                } => {
                    // Update tag fetching progress
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            state.phase1_tag_fetching = true;
                            state.phase1_tag_resource_type = Some(resource_type);
                            state.phase1_tag_progress_count = items_processed;
                            state.phase1_tag_progress_total = estimated_total;
                        },
                    );
                }

                QueryProgress::TagFetchingCompleted { .. } => {
                    // Clear tag fetching state
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            state.phase1_tag_fetching = false;
                            state.phase1_tag_resource_type = None;
                        },
                    );
                }

                QueryProgress::Phase1Completed { resources } => {
                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase1_completed.before_unwrap", "");

                    // Try to unwrap Arc, falling back to clone if other references exist
                    let resources_vec = std::sync::Arc::try_unwrap(resources)
                        .unwrap_or_else(|arc| {
                            #[cfg(debug_assertions)]
                            crate::perf_checkpoint!("ui_adapter.phase1_completed.arc_clone_fallback", "");
                            (*arc).clone()
                        });

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase1_completed.after_unwrap", "");

                    // Phase 1 complete - update final resource list
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            #[cfg(debug_assertions)]
                            crate::perf_checkpoint!("ui_adapter.phase1_completed.in_state_update", "");

                            state.resources = resources_vec;
                            state.reset_phase1_state();
                            // Increment enrichment version to invalidate tree cache
                            state.increment_enrichment_version_force();
                            tracing::info!("Phase 1 completed");
                        },
                    );

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase1_completed.after_state_update", "");
                }

                QueryProgress::Phase2Started { total_resources } => {
                    // Phase 2 enrichment started
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            state.phase2_enrichment_in_progress = true;
                            state.phase2_enrichment_completed = false;
                            state.phase2_progress_total = total_resources;
                            state.phase2_progress_count = 0;
                            tracing::info!("Phase 2 started: {} resources to enrich", total_resources);
                        },
                    );
                }

                QueryProgress::Phase2Progress {
                    resource_type,
                    processed,
                    total,
                } => {
                    // Phase 2 enrichment progress update
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            state.phase2_current_service = Some(resource_type);
                            state.phase2_progress_count = processed;
                            state.phase2_progress_total = total;
                            // Update enrichment version with debouncing to reduce UI flicker
                            state.increment_enrichment_version_debounced();
                        },
                    );
                }

                QueryProgress::Phase2Completed { resources } => {
                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase2_completed.before_unwrap", "");

                    // Try to unwrap Arc, falling back to clone if other references exist
                    let resources_vec = std::sync::Arc::try_unwrap(resources)
                        .unwrap_or_else(|arc| {
                            #[cfg(debug_assertions)]
                            crate::perf_checkpoint!("ui_adapter.phase2_completed.arc_clone_fallback", "");
                            (*arc).clone()
                        });

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase2_completed.after_unwrap", "");

                    // Phase 2 enrichment completed - update resources with detailed properties
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            #[cfg(debug_assertions)]
                            crate::perf_checkpoint!("ui_adapter.phase2_completed.in_state_update", "");

                            state.resources = resources_vec;
                            state.phase2_enrichment_in_progress = false;
                            state.phase2_enrichment_completed = true;
                            state.phase2_current_service = None;
                            // Increment enrichment version to invalidate tree cache
                            state.increment_enrichment_version_force();
                            tracing::info!("Phase 2 completed");
                        },
                    );

                    #[cfg(debug_assertions)]
                    crate::perf_checkpoint!("ui_adapter.phase2_completed.after_state_update", "");
                }

                QueryProgress::Completed { resources } => {
                    // Entire query complete (including Phase 2 if applicable)
                    let cache_key_clone = cache_key_for_callback.clone();
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            state.resources = resources;
                            state.finish_loading_task(&cache_key_clone);
                            state.reset_phase1_state();
                            state.reset_phase2_state();
                            state.update_tag_popularity();
                            // Increment enrichment version to invalidate tree cache
                            state.increment_enrichment_version_force();
                            tracing::info!(
                                "Query completed: {} total resources",
                                state.resources.len()
                            );
                        },
                    );
                }

                QueryProgress::Failed { error } => {
                    // Query failed
                    tracing::error!("Query failed: {}", error);
                    let cache_key_clone = cache_key_for_callback.clone();
                    Self::retry_state_update(
                        pane_state_for_callback.clone(),
                        &ctx_for_callback,
                        10,
                        move |state| {
                            state.loading_tasks.remove(&cache_key_clone);
                            state.reset_phase1_state();
                        },
                    );
                }
            }
        };

        // Execute query via engine
        self.engine
            .execute_query(scope, cache_key, progress_callback)
    }

    /// Retry state update with lock contention handling
    ///
    /// Callbacks are invoked from within runtime.block_on(async {...}), which means:
    /// - Can't use blocking_write() (Tokio runtime context)
    /// - Can't use tokio::spawn() (synchronous callback)
    /// - Can use try_write() with std::thread::sleep for retry
    ///
    /// # Arguments
    /// - `pane_state`: Arc to the pane's ResourceExplorerState
    /// - `ctx`: egui Context for requesting repaints
    /// - `max_retries`: Maximum number of retry attempts
    /// - `update_fn`: Function that modifies the state
    fn retry_state_update<F>(
        pane_state: Arc<RwLock<ResourceExplorerState>>,
        ctx: &egui::Context,
        max_retries: usize,
        update_fn: F,
    ) where
        F: FnOnce(&mut ResourceExplorerState) + Send + 'static,
    {
        let mut success = false;
        let mut update_fn = Some(update_fn);

        for attempt in 0..max_retries {
            if let Ok(mut state) = pane_state.try_write() {
                if let Some(f) = update_fn.take() {
                    f(&mut state);
                }
                success = true;
                if attempt > 0 {
                    tracing::debug!("Lock acquired after {} retries", attempt);
                }
                break;
            }
            // Use std::thread::sleep since we can't use tokio::time::sleep
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        if !success {
            tracing::error!("Failed to update state after {} retries!", max_retries);
        } else {
            // Request UI repaint after successful state update
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
mod tests {
    // Note: Full integration tests for the UI adapter will be in integration tests
    // since they require async runtime and actual AWS client setup.
    // Unit tests here focus on basic construction.
}
