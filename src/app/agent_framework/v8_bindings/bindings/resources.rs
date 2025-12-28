//! Resource query function bindings
//!
//! Provides JavaScript access to AWS resource querying functionality.
//! This is a code-first approach where JavaScript code calls bound Rust functions
//! to query AWS resources across accounts, regions, and resource types.
//!
//! Uses unified caching via ResourceExplorerState for consistency between
//! Explorer UI and Agent Framework queries.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::app::agent_framework::tools_registry::get_global_aws_client;
use crate::app::resource_explorer::state::{
    AccountSelection, QueryScope, RegionSelection, ResourceEntry, ResourceExplorerState,
    ResourceTypeSelection,
};
use crate::app::resource_explorer::unified_query::{
    BookmarkInfo, DetailLevel, QueryError, QueryWarning, ResourceFull, ResourceSummary,
    ResourceWithTags, UnifiedQueryResult,
};
use crate::app::resource_explorer::{get_global_bookmark_manager, get_global_explorer_state};

/// JavaScript function call arguments for queryResources()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResourcesArgs {
    /// Account IDs to query (null = random account)
    pub accounts: Option<Vec<String>>,

    /// Region codes to query (null = us-east-1)
    pub regions: Option<Vec<String>>,

    /// CloudFormation resource types (required)
    pub resource_types: Vec<String>,

    /// Detail level for returned data: "count", "summary" (default), "tags", "full"
    pub detail: Option<String>,
}

// Note: Resource types (ResourceSummary, ResourceWithTags, ResourceFull) are now
// imported from unified_query.rs for consistency between Explorer and Agent Framework.

/// Register resource-related functions into V8 context
pub fn register(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Register queryResources() function
    let query_resources_fn = v8::Function::new(scope, query_resources_callback)
        .expect("Failed to create queryResources function");
    let fn_name =
        v8::String::new(scope, "queryResources").expect("Failed to create function name string");
    global.set(scope, fn_name.into(), query_resources_fn.into());

    // Register listBookmarks() function
    let list_bookmarks_fn = v8::Function::new(scope, list_bookmarks_callback)
        .expect("Failed to create listBookmarks function");
    let fn_name =
        v8::String::new(scope, "listBookmarks").expect("Failed to create function name string");
    global.set(scope, fn_name.into(), list_bookmarks_fn.into());

    // Register queryBookmarks() function
    let query_bookmarks_fn = v8::Function::new(scope, query_bookmarks_callback)
        .expect("Failed to create queryBookmarks function");
    let fn_name =
        v8::String::new(scope, "queryBookmarks").expect("Failed to create function name string");
    global.set(scope, fn_name.into(), query_bookmarks_fn.into());

    Ok(())
}

/// Callback for queryResources() JavaScript function
fn query_resources_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Parse JavaScript arguments
    let args_obj = match args.get(0).to_object(scope) {
        Some(obj) => obj,
        None => {
            let msg = v8::String::new(
                scope,
                "queryResources() requires an object argument with { accounts, regions, resourceTypes, detail? }",
            )
            .unwrap();
            let error = v8::Exception::type_error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Convert V8 object to JSON string for parsing
    let json_str = match v8::json::stringify(scope, args_obj.into()) {
        Some(s) => s.to_rust_string_lossy(scope),
        None => {
            let msg = v8::String::new(scope, "Failed to stringify arguments").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse JSON into QueryResourcesArgs
    let query_args: QueryResourcesArgs = match serde_json::from_str(&json_str) {
        Ok(args) => args,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to parse arguments: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Execute async query in Tokio runtime
    let result = match execute_query(query_args) {
        Ok(result) => result,
        Err(e) => {
            // Return error result instead of throwing exception
            // This allows the caller to check status and errors
            let error_result: UnifiedQueryResult<Vec<serde_json::Value>> =
                UnifiedQueryResult::error(vec![QueryError {
                    account: "all".to_string(),
                    region: "all".to_string(),
                    code: "QueryFailed".to_string(),
                    message: e.to_string(),
                }]);
            if let Ok(json) = serde_json::to_string(&error_result) {
                if let Some(v8_str) = v8::String::new(scope, &json) {
                    if let Some(v8_value) = v8::json::parse(scope, v8_str) {
                        rv.set(v8_value);
                        return;
                    }
                }
            }
            let msg = v8::String::new(scope, &format!("Query failed: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Serialize result to JSON string
    let json_str = match serde_json::to_string(&result) {
        Ok(json) => json,
        Err(e) => {
            let msg =
                v8::String::new(scope, &format!("Failed to serialize result: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Create V8 string from JSON
    let v8_str = match v8::String::new(scope, &json_str) {
        Some(s) => s,
        None => {
            let msg = v8::String::new(scope, "Failed to create V8 string").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse JSON in V8 to create JavaScript object
    let v8_value = match v8::json::parse(scope, v8_str) {
        Some(v) => v,
        None => {
            let msg = v8::String::new(scope, "Failed to parse JSON in V8").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    rv.set(v8_value);
}

/// Execute AWS resource query (synchronous wrapper for async code)
fn execute_query(args: QueryResourcesArgs) -> Result<UnifiedQueryResult<serde_json::Value>> {
    // Use the current runtime's handle and block_in_place to avoid nested runtime error
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async { query_resources_internal(args).await })
    })
}

/// Internal async implementation of resource query
async fn query_resources_internal(
    args: QueryResourcesArgs,
) -> Result<UnifiedQueryResult<serde_json::Value>> {
    // Parse detail level (default: summary)
    let detail_level = DetailLevel::from_str_opt(args.detail.as_deref());
    debug!("Query detail level: {:?}", detail_level);

    // Get global AWS client
    let client = get_global_aws_client().ok_or_else(|| anyhow!("AWS client not initialized"))?;

    // Get accounts: if null, pick one random account
    let account_ids = match args.accounts {
        Some(ids) if !ids.is_empty() => ids,
        _ => {
            // Get one random account from GLOBAL_AWS_IDENTITY
            let identity = super::accounts::get_global_aws_identity()
                .ok_or_else(|| anyhow!("AWS Identity Center not initialized"))?;

            let identity_guard = identity
                .lock()
                .map_err(|e| anyhow!("Failed to lock AwsIdentityCenter: {}", e))?;

            if identity_guard.accounts.is_empty() {
                return Err(anyhow!("No AWS accounts configured"));
            }

            // Pick a random account
            let mut rng = rand::thread_rng();
            let random_account = identity_guard
                .accounts
                .choose(&mut rng)
                .ok_or_else(|| anyhow!("Failed to select random account"))?;

            info!(
                "Using random account: {} ({})",
                random_account.account_name, random_account.account_id
            );
            vec![random_account.account_id.clone()]
        }
    };

    // Get regions: if null, use us-east-1
    let region_codes = match args.regions {
        Some(regions) if !regions.is_empty() => regions,
        _ => {
            info!("No regions specified, using us-east-1");
            vec!["us-east-1".to_string()]
        }
    };

    // Validate resource types are not empty
    if args.resource_types.is_empty() {
        return Err(anyhow!("resource_types array cannot be empty"));
    }

    // Build QueryScope
    let scope = build_query_scope(&account_ids, &region_codes, &args.resource_types)?;

    // Get global explorer state for unified caching
    let explorer_state = get_global_explorer_state();

    // Create a temporary cache for the parallel query
    // We'll sync results with ResourceExplorerState after
    let query_cache: Arc<tokio::sync::RwLock<HashMap<String, Vec<ResourceEntry>>>> =
        Arc::new(tokio::sync::RwLock::new(HashMap::new()));

    // Check if any results are already cached in explorer state
    if let Some(state) = &explorer_state {
        let state_guard = state.read().await;
        for account in &scope.accounts {
            for region in &scope.regions {
                for resource_type in &scope.resource_types {
                    let cache_key = format!(
                        "{}:{}:{}",
                        account.account_id, region.region_code, resource_type.resource_type
                    );
                    if let Some(cached_entries) = state_guard.cached_queries.get(&cache_key) {
                        // Cache hit - use cached entries
                        debug!("Cache hit for {}", cache_key);
                        let mut cache_guard = query_cache.write().await;
                        cache_guard.insert(cache_key, cached_entries.clone());
                    }
                }
            }
        }
    }

    // Create result channel
    let (result_tx, mut result_rx) = mpsc::channel(1000);

    // Track warnings and errors per account/region
    let mut warnings: Vec<QueryWarning> = Vec::new();
    let mut errors: Vec<QueryError> = Vec::new();
    let mut success_count = 0usize;
    let mut error_count = 0usize;

    // Start parallel query (spawns background tasks)
    let client_clone = Arc::clone(&client);
    let cache_clone = Arc::clone(&query_cache);
    let scope_for_query = scope.clone(); // Clone for spawn, keep original for later use
    tokio::spawn(async move {
        if let Err(e) = client_clone
            .query_aws_resources_parallel(
                &scope_for_query,
                result_tx,
                None, // No progress updates needed
                cache_clone,
            )
            .await
        {
            warn!("Query error: {}", e);
        }
    });

    // Collect all results
    let mut all_entries: Vec<ResourceEntry> = Vec::new();

    while let Some(result) = result_rx.recv().await {
        match result.resources {
            Ok(entries) => {
                success_count += 1;
                // Store in explorer state cache for future queries
                if let Some(state) = &explorer_state {
                    let cache_key = format!(
                        "{}:{}:{}",
                        result.account_id, result.region, result.resource_type
                    );
                    let mut state_guard = state.write().await;
                    state_guard
                        .cached_queries
                        .insert(cache_key, entries.clone());
                }
                all_entries.extend(entries);
            }
            Err(e) => {
                error_count += 1;
                let error_msg = e.to_string();
                warn!(
                    "Query error for {}/{}/{}: {}",
                    result.account_id, result.region, result.resource_type, error_msg
                );

                // Categorize error
                let (code, is_warning) = categorize_error(&error_msg);
                if is_warning {
                    warnings.push(QueryWarning {
                        account: result.account_id,
                        region: result.region,
                        message: error_msg,
                    });
                } else {
                    errors.push(QueryError {
                        account: result.account_id,
                        region: result.region,
                        code,
                        message: error_msg,
                    });
                }
            }
        }
    }

    let total_count = all_entries.len();
    info!(
        "Collected {} resources total (success: {}, errors: {})",
        total_count, success_count, error_count
    );

    // Phase 2 wait logic for detail="full"
    let (mut details_loaded, mut details_pending) = (false, false);

    if detail_level == DetailLevel::Full {
        // Check if any enrichable resources need Phase 2
        let enrichable_types = ResourceExplorerState::enrichable_resource_types();
        let needs_phase2 = all_entries.iter().any(|e| {
            enrichable_types.contains(&e.resource_type.as_str())
                && e.detailed_properties.is_none()
        });

        if needs_phase2 {
            if let Some(state) = &explorer_state {
                // Check if Phase 2 is in progress
                let should_wait = {
                    let guard = state.read().await;
                    guard.phase2_enrichment_in_progress
                };

                if should_wait {
                    info!("Waiting for Phase 2 enrichment to complete...");
                    let start = std::time::Instant::now();
                    let timeout = std::time::Duration::from_secs(60);

                    loop {
                        if start.elapsed() > timeout {
                            warn!("Phase 2 wait timeout after 60 seconds");
                            details_pending = true;
                            break;
                        }

                        let still_waiting = {
                            let guard = state.read().await;
                            guard.phase2_enrichment_in_progress
                        };

                        if !still_waiting {
                            info!("Phase 2 enrichment completed, refreshing entries");
                            // Refresh entries from cache with enriched data
                            all_entries = refresh_entries_from_cache(&scope, state.clone()).await;
                            details_loaded = true;
                            break;
                        }

                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                } else {
                    // Phase 2 not running - check if details already loaded
                    let completed = {
                        let guard = state.read().await;
                        guard.phase2_enrichment_completed
                    };
                    if completed {
                        // Refresh to get enriched data
                        all_entries = refresh_entries_from_cache(&scope, state.clone()).await;
                        details_loaded = true;
                    }
                }
            }
        } else {
            // No enrichable resources need Phase 2, or all already have details
            details_loaded = all_entries.iter().all(|e| {
                !enrichable_types.contains(&e.resource_type.as_str())
                    || e.detailed_properties.is_some()
            });
        }
    }

    // Update total count after potential refresh
    let total_count = all_entries.len();

    // Apply detail level filtering and convert to JSON
    let data = match detail_level {
        DetailLevel::Count => {
            // Just return count in data field
            serde_json::json!(null)
        }
        DetailLevel::Summary => {
            let summaries: Vec<ResourceSummary> =
                all_entries.iter().map(ResourceSummary::from).collect();
            serde_json::to_value(summaries).unwrap_or(serde_json::json!([]))
        }
        DetailLevel::Tags => {
            let with_tags: Vec<ResourceWithTags> =
                all_entries.iter().map(ResourceWithTags::from).collect();
            serde_json::to_value(with_tags).unwrap_or(serde_json::json!([]))
        }
        DetailLevel::Full => {
            let full: Vec<ResourceFull> = all_entries.iter().map(ResourceFull::from).collect();
            serde_json::to_value(full).unwrap_or(serde_json::json!([]))
        }
    };

    // Build result with appropriate status and Phase 2 metadata
    let result = UnifiedQueryResult::from_results_with_phase2_status(
        data,
        total_count,
        success_count,
        error_count,
        warnings,
        errors,
        details_loaded,
        details_pending,
    );

    Ok(result)
}

/// Refresh entries from the explorer state cache
/// This is called after Phase 2 completes to get the enriched data
async fn refresh_entries_from_cache(
    scope: &QueryScope,
    state: Arc<tokio::sync::RwLock<crate::app::resource_explorer::state::ResourceExplorerState>>,
) -> Vec<ResourceEntry> {
    let guard = state.read().await;
    let mut entries = Vec::new();

    for account in &scope.accounts {
        for region in &scope.regions {
            for resource_type in &scope.resource_types {
                let cache_key = format!(
                    "{}:{}:{}",
                    account.account_id, region.region_code, resource_type.resource_type
                );
                if let Some(cached) = guard.cached_queries.get(&cache_key) {
                    entries.extend(cached.clone());
                }
            }
        }
    }

    entries
}

/// Categorize an error message into a code and whether it's a warning
fn categorize_error(error_msg: &str) -> (String, bool) {
    let lower = error_msg.to_lowercase();
    if lower.contains("access denied") || lower.contains("not authorized") {
        ("AccessDenied".to_string(), false)
    } else if lower.contains("rate") || lower.contains("throttl") {
        ("RateLimitExceeded".to_string(), true) // Warning - retryable
    } else if lower.contains("timeout") || lower.contains("timed out") {
        ("Timeout".to_string(), true) // Warning - retryable
    } else if lower.contains("not found") || lower.contains("does not exist") {
        ("NotFound".to_string(), false)
    } else if lower.contains("invalid") {
        ("InvalidRequest".to_string(), false)
    } else {
        ("UnknownError".to_string(), false)
    }
}

// ============================================================================
// Bookmark Functions
// ============================================================================

/// Callback for listBookmarks() JavaScript function
/// Returns a flat list of all bookmarks (no folder hierarchy)
fn list_bookmarks_callback(
    scope: &mut v8::PinScope<'_, '_>,
    _args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Get global bookmark manager
    let manager = match get_global_bookmark_manager() {
        Some(m) => m,
        None => {
            let msg = v8::String::new(
                scope,
                "listBookmarks() failed: Bookmark manager not initialized (login required)",
            )
            .unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Get bookmarks from manager (read lock)
    let bookmarks: Vec<BookmarkInfo> = match manager.read() {
        Ok(guard) => guard
            .get_bookmarks()
            .iter()
            .map(BookmarkInfo::from)
            .collect(),
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to read bookmarks: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Serialize to JSON string
    let json_str = match serde_json::to_string(&bookmarks) {
        Ok(json) => json,
        Err(e) => {
            let msg =
                v8::String::new(scope, &format!("Failed to serialize bookmarks: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Create V8 string from JSON
    let v8_str = match v8::String::new(scope, &json_str) {
        Some(s) => s,
        None => {
            let msg = v8::String::new(scope, "Failed to create V8 string").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse JSON in V8 to create JavaScript array
    let v8_value = match v8::json::parse(scope, v8_str) {
        Some(v) => v,
        None => {
            let msg = v8::String::new(scope, "Failed to parse JSON in V8").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    rv.set(v8_value);
}

/// JavaScript function call arguments for queryBookmarks()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryBookmarksArgs {
    /// Detail level for returned data: "count", "summary" (default), "tags", "full"
    pub detail: Option<String>,
}

/// Callback for queryBookmarks() JavaScript function
/// Executes a bookmark's saved query and returns resources
fn query_bookmarks_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Parse bookmark ID from first argument
    let bookmark_id = match args.get(0).to_string(scope) {
        Some(s) => s.to_rust_string_lossy(scope),
        None => {
            let msg = v8::String::new(
                scope,
                "queryBookmarks() requires a bookmark ID as first argument",
            )
            .unwrap();
            let error = v8::Exception::type_error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse optional options object from second argument
    let options: QueryBookmarksArgs = if args.length() > 1 {
        match args.get(1).to_object(scope) {
            Some(obj) => match v8::json::stringify(scope, obj.into()) {
                Some(s) => {
                    let json_str = s.to_rust_string_lossy(scope);
                    serde_json::from_str(&json_str).unwrap_or(QueryBookmarksArgs { detail: None })
                }
                None => QueryBookmarksArgs { detail: None },
            },
            None => QueryBookmarksArgs { detail: None },
        }
    } else {
        QueryBookmarksArgs { detail: None }
    };

    // Execute async query
    let result = match execute_bookmark_query(&bookmark_id, options) {
        Ok(result) => result,
        Err(e) => {
            // Return error result instead of throwing exception
            let error_result: UnifiedQueryResult<Vec<serde_json::Value>> =
                UnifiedQueryResult::error(vec![QueryError {
                    account: "all".to_string(),
                    region: "all".to_string(),
                    code: "BookmarkQueryFailed".to_string(),
                    message: e.to_string(),
                }]);
            if let Ok(json) = serde_json::to_string(&error_result) {
                if let Some(v8_str) = v8::String::new(scope, &json) {
                    if let Some(v8_value) = v8::json::parse(scope, v8_str) {
                        rv.set(v8_value);
                        return;
                    }
                }
            }
            let msg = v8::String::new(scope, &format!("Bookmark query failed: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Serialize result to JSON string
    let json_str = match serde_json::to_string(&result) {
        Ok(json) => json,
        Err(e) => {
            let msg =
                v8::String::new(scope, &format!("Failed to serialize result: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Create V8 string from JSON
    let v8_str = match v8::String::new(scope, &json_str) {
        Some(s) => s,
        None => {
            let msg = v8::String::new(scope, "Failed to create V8 string").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse JSON in V8 to create JavaScript object
    let v8_value = match v8::json::parse(scope, v8_str) {
        Some(v) => v,
        None => {
            let msg = v8::String::new(scope, "Failed to parse JSON in V8").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    rv.set(v8_value);
}

/// Execute a bookmark query (synchronous wrapper for async code)
fn execute_bookmark_query(
    bookmark_id: &str,
    options: QueryBookmarksArgs,
) -> Result<UnifiedQueryResult<serde_json::Value>> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { query_bookmark_internal(bookmark_id, options).await })
    })
}

/// Internal async implementation of bookmark query
async fn query_bookmark_internal(
    bookmark_id: &str,
    options: QueryBookmarksArgs,
) -> Result<UnifiedQueryResult<serde_json::Value>> {
    // Parse detail level (default: summary)
    let detail_level = DetailLevel::from_str_opt(options.detail.as_deref());
    debug!("Bookmark query detail level: {:?}", detail_level);

    // Get the bookmark from global manager
    let manager =
        get_global_bookmark_manager().ok_or_else(|| anyhow!("Bookmark manager not initialized"))?;

    let bookmark = {
        let guard = manager
            .read()
            .map_err(|e| anyhow!("Failed to lock bookmark manager: {}", e))?;
        guard
            .get_bookmark(bookmark_id)
            .cloned()
            .ok_or_else(|| anyhow!("Bookmark not found: {}", bookmark_id))?
    };

    debug!(
        "Executing bookmark '{}' query: accounts={:?}, regions={:?}, types={:?}",
        bookmark.name, bookmark.account_ids, bookmark.region_codes, bookmark.resource_type_ids
    );

    // Build QueryResourcesArgs from bookmark
    let query_args = QueryResourcesArgs {
        accounts: if bookmark.account_ids.is_empty() {
            None
        } else {
            Some(bookmark.account_ids.clone())
        },
        regions: if bookmark.region_codes.is_empty() {
            None
        } else {
            Some(bookmark.region_codes.clone())
        },
        resource_types: bookmark.resource_type_ids.clone(),
        detail: options.detail,
    };

    // Execute the query using the existing internal function
    query_resources_internal(query_args).await
}

/// Build QueryScope from arguments
fn build_query_scope(
    account_ids: &[String],
    region_codes: &[String],
    resource_types: &[String],
) -> Result<QueryScope> {
    // Build account selections
    let accounts: Vec<AccountSelection> = account_ids
        .iter()
        .map(|id| AccountSelection {
            account_id: id.clone(),
            display_name: id.clone(), // We don't have the name in V8 context
            color: egui::Color32::from_rgb(100, 100, 100), // Default gray
        })
        .collect();

    // Build region selections
    let regions: Vec<RegionSelection> = region_codes
        .iter()
        .map(|code| RegionSelection {
            region_code: code.clone(),
            display_name: code.clone(),
            color: egui::Color32::from_rgb(100, 100, 100), // Default gray
        })
        .collect();

    // Build resource type selections
    let resource_type_selections: Vec<ResourceTypeSelection> = resource_types
        .iter()
        .map(|rt| {
            // Extract service name from CloudFormation type (AWS::EC2::Instance -> EC2)
            let service_name = rt.split("::").nth(1).unwrap_or("Unknown").to_string();

            ResourceTypeSelection {
                resource_type: rt.clone(),
                display_name: rt.clone(),
                service_name,
            }
        })
        .collect();

    Ok(QueryScope {
        accounts,
        regions,
        resource_types: resource_type_selections,
    })
}

/// Get LLM documentation for resource query functions
pub fn get_documentation() -> String {
    r#"
### queryResources()

Query AWS resources across accounts, regions, and resource types with configurable detail levels.

**Signature:**
```typescript
function queryResources(options: QueryOptions): QueryResult

interface QueryOptions {
  accounts?: string[] | null;      // Account IDs (null = random account)
  regions?: string[] | null;       // Region codes (null = us-east-1)
  resourceTypes: string[];         // CloudFormation resource types (required)
  detail?: 'count' | 'summary' | 'tags' | 'full';  // Detail level (default: 'summary')
}

interface QueryResult {
  status: 'success' | 'partial' | 'error';  // Query status
  data: ResourceInfo[] | null;               // Resources (null for count)
  count: number;                             // Total resource count
  warnings: QueryWarning[];                  // Non-fatal issues (rate limiting, timeouts)
  errors: QueryError[];                      // Errors per account/region
}

interface QueryWarning {
  account: string;
  region: string;
  message: string;
}

interface QueryError {
  account: string;
  region: string;
  code: string;    // e.g., 'AccessDenied', 'NotFound', 'InvalidRequest'
  message: string;
}
```

**Detail Levels:**
- `count`: Just totals, data is null - use for "how many instances exist?"
- `summary` (DEFAULT): Minimal fields: resourceId, displayName, resourceType, accountId, region, status
- `tags`: Summary fields plus tags array
- `full`: Complete data including properties, rawProperties, detailedProperties

**Resource Info by Detail Level:**
```typescript
// detail: 'summary' (default)
interface ResourceSummary {
  resourceId: string;
  displayName: string;
  resourceType: string;
  accountId: string;
  region: string;
  status: string | null;
}

// detail: 'tags'
interface ResourceWithTags extends ResourceSummary {
  tags: Array<{key: string, value: string}>;
}

// detail: 'full'
interface ResourceFull extends ResourceWithTags {
  properties: object;           // Normalized properties from Phase 1
  rawProperties: object;        // Original AWS API response from Phase 1
  detailedProperties: object | null;  // Phase 2 enrichment data (only for enrichable types)
                                      // Contains: policies, encryption settings, configurations
                                      // null for non-enrichable types (EC2, VPC, etc.)
}
```

**Status Meanings:**
- `success`: All account/region queries succeeded
- `partial`: Some queries succeeded, some failed (check errors array)
- `error`: All queries failed (check errors array)

**Description:**
Executes parallel AWS API queries across specified accounts, regions, and resource types.
Returns a result object with status, data, count, warnings, and errors.
Results are cached and shared with AWS Explorer UI for efficiency.

**Default Behavior:**
- If `accounts` is `null` or empty: selects ONE random account from configured accounts
- If `regions` is `null` or empty: Uses `us-east-1` region only
- If `detail` is not specified: Uses `summary` level
- `resourceTypes` is REQUIRED and cannot be empty

**Resource Types:**
Use CloudFormation format (e.g., `AWS::EC2::Instance`, `AWS::S3::Bucket`, `AWS::IAM::Role`).
We support 93 services and 183 resource types.

**Return value structure:**
```json
{
  "status": "success",
  "data": [
    {
      "resourceId": "i-1234567890abcdef0",
      "displayName": "web-server-01",
      "resourceType": "AWS::EC2::Instance",
      "accountId": "123456789012",
      "region": "us-east-1",
      "status": "running"
    }
  ],
  "count": 1,
  "warnings": [],
  "errors": []
}
```

**Example usage:**
```javascript
// Quick count - how many EC2 instances?
const countResult = queryResources({
  accounts: ["123456789012"],
  regions: ["us-east-1"],
  resourceTypes: ["AWS::EC2::Instance"],
  detail: "count"
});
console.log(`Found ${countResult.count} EC2 instances`);

// Default summary query
const result = queryResources({
  accounts: ["123456789012"],
  regions: ["us-east-1"],
  resourceTypes: ["AWS::EC2::Instance"]
});

if (result.status === "error") {
  console.error("Query failed:", result.errors);
  return null;
}

if (result.status === "partial") {
  console.warn("Some queries failed:", result.errors);
}

console.log(`Found ${result.count} instances`);
result.data.forEach(i => {
  console.log(`${i.displayName}: ${i.status}`);
});

// Query with tags to filter by environment
const tagResult = queryResources({
  accounts: listAccounts().map(a => a.id),
  regions: ["us-east-1", "us-west-2"],
  resourceTypes: ["AWS::EC2::Instance"],
  detail: "tags"
});

const prodInstances = tagResult.data.filter(r =>
  r.tags.some(t => t.key === "Environment" && t.value === "Production")
);

// Full details for deep inspection
const fullResult = queryResources({
  accounts: ["123456789012"],
  regions: ["us-east-1"],
  resourceTypes: ["AWS::EC2::Instance"],
  detail: "full"
});

fullResult.data.forEach(r => {
  console.log(`Instance: ${r.displayName}`);
  console.log(`  Type: ${r.properties.InstanceType}`);
  console.log(`  Launch: ${r.properties.LaunchTime}`);
  console.log(`  Tags: ${r.tags.length}`);
});
```

**Error handling:**
```javascript
const result = queryResources({
  accounts: null,
  regions: null,
  resourceTypes: ["AWS::EC2::Instance"]
});

// Check status first
if (result.status === "error") {
  result.errors.forEach(e => {
    console.error(`Error in ${e.account}/${e.region}: ${e.code} - ${e.message}`);
  });
  return null;
}

// Handle partial results
if (result.status === "partial") {
  console.warn("Partial results - some queries failed:");
  result.errors.forEach(e => console.warn(`  ${e.account}/${e.region}: ${e.code}`));
  result.warnings.forEach(w => console.warn(`  Warning: ${w.message}`));
}

// Empty results are valid (status: success, count: 0)
if (result.count === 0) {
  console.log("No resources found matching criteria");
  return [];
}

return result.data;
```

**Performance considerations:**
- Use `detail: "count"` for existence checks - no data transferred
- Use `detail: "summary"` (default) for list views
- Use `detail: "tags"` only when filtering by tags
- Use `detail: "full"` only when you need all properties
- Results are cached and shared with AWS Explorer UI
- Large queries (many accounts x regions x types) may take 10-60 seconds

**Global Services (region parameter has no effect):**
Some AWS services are global - they return the same resources regardless of which region you query:
- `AWS::S3::Bucket` - list-buckets returns ALL buckets in the account, not region-specific
- `AWS::IAM::Role`, `AWS::IAM::User`, `AWS::IAM::Policy` - IAM is global
- `AWS::Route53::HostedZone` - Route53 is global DNS
- `AWS::CloudFront::Distribution` - CloudFront is global CDN
- `AWS::Organizations::*` - Organizations is global

For global services, the region parameter doesn't filter results. The system automatically
queries once per account regardless of how many regions you specify.

**Two-Phase Loading and detailedProperties:**
Some resource types require two phases to load complete data:
- Phase 1: Quick list query returns basic info immediately
- Phase 2: Background enrichment fetches detailed properties (policies, configurations, etc.)

The `detailedProperties` field is ONLY populated for "enrichable" resource types after Phase 2:
- AWS::S3::Bucket (bucket policies, encryption, versioning)
- AWS::Lambda::Function (function configuration, environment variables)
- AWS::IAM::Role, AWS::IAM::User, AWS::IAM::Policy (inline policies, attached policies)
- AWS::KMS::Key (key policies, rotation status)
- AWS::SQS::Queue (queue policies, attributes)
- AWS::SNS::Topic (topic policies, subscriptions)
- AWS::DynamoDB::Table (table settings, GSIs)
- AWS::ECS::Cluster, AWS::ECS::Service
- AWS::CloudFormation::Stack (stack resources, outputs)
- And others (Cognito, CodeCommit, ELBv2, EMR, EventBridge, Glue)

Non-enrichable resources (EC2 instances, VPCs, Subnets) have `detailedProperties: null`
because their full data is available in Phase 1.

When using `detail: "full"`, the query waits for Phase 2 to complete (up to 60s timeout).
The result includes Phase 2 status:
```typescript
interface QueryResult {
  // ... other fields ...
  detailsLoaded: boolean;   // true if Phase 2 completed for all enrichable resources
  detailsPending: boolean;  // true if Phase 2 is still running (timeout occurred)
}
```

**Common Error Codes:**
Errors in the `errors` array have specific codes:
- `AccessDenied` - IAM permissions insufficient for this resource type
- `InvalidToken` - Region not enabled or not accessible (common for opt-in regions like me-south-1, af-south-1)
- `OptInRequired` - Region requires explicit opt-in in AWS Account settings
- `Timeout` - Network timeout (retryable)
- `RateLimitExceeded` - API throttled (retryable, appears as warning not error)
- `NotFound` - Resource or service not found
- `InvalidRequest` - Invalid parameters

Region-related errors (InvalidToken, OptInRequired) are normal for opt-in regions that
haven't been enabled in the account. These can be safely ignored for most use cases.

---

### listBookmarks()

List all saved bookmarks (flat list, no folder hierarchy).

**Signature:**
```typescript
function listBookmarks(): BookmarkInfo[]

interface BookmarkInfo {
  id: string;                   // Unique bookmark ID (UUID)
  name: string;                 // Display name
  description: string | null;   // Optional description
  accountIds: string[];         // Saved account IDs
  regionCodes: string[];        // Saved region codes
  resourceTypes: string[];      // Saved resource types
  hasTagFilters: boolean;       // Whether bookmark has tag filters
  hasSearchFilter: boolean;     // Whether bookmark has search filter
  accessCount: number;          // Times this bookmark was accessed
  lastAccessed: string | null;  // ISO timestamp of last access
}
```

**Description:**
Returns all user-created bookmarks as a flat list. Bookmarks are saved queries
that store account, region, resource type selections along with filters.
Use with `queryBookmarks()` to execute a bookmark's query.

**Example usage:**
```javascript
// List all bookmarks
const bookmarks = listBookmarks();
console.log(`Found ${bookmarks.length} bookmarks`);

// Find bookmarks by name
const prodBookmark = bookmarks.find(b => b.name.includes("Production"));
if (prodBookmark) {
  console.log(`Found: ${prodBookmark.name} (${prodBookmark.resourceTypes.join(", ")})`);
}

// List bookmarks with their configurations
bookmarks.forEach(b => {
  console.log(`${b.name}:`);
  console.log(`  Accounts: ${b.accountIds.length || "all"}`);
  console.log(`  Regions: ${b.regionCodes.join(", ") || "default"}`);
  console.log(`  Types: ${b.resourceTypes.join(", ")}`);
  console.log(`  Has filters: tags=${b.hasTagFilters}, search=${b.hasSearchFilter}`);
});
```

---

### queryBookmarks()

Execute a saved bookmark's query and return resources.

**Signature:**
```typescript
function queryBookmarks(
  bookmarkId: string,
  options?: { detail?: 'count' | 'summary' | 'tags' | 'full' }
): QueryResult

// Returns same QueryResult as queryResources()
interface QueryResult {
  status: 'success' | 'partial' | 'error';
  data: ResourceInfo[] | null;
  count: number;
  warnings: QueryWarning[];
  errors: QueryError[];
}
```

**Description:**
Executes the saved query from a bookmark. This is a convenience function that
loads the bookmark's configuration (accounts, regions, resource types) and
runs `queryResources()` with those parameters.

**Parameters:**
- `bookmarkId` (required): The bookmark's UUID from `listBookmarks()`
- `options.detail`: Detail level for results (same as queryResources)

**Example usage:**
```javascript
// Get bookmarks and execute one
const bookmarks = listBookmarks();
const myBookmark = bookmarks.find(b => b.name === "Production EC2");

if (myBookmark) {
  // Quick count
  const count = queryBookmarks(myBookmark.id, { detail: "count" });
  console.log(`Production has ${count.count} EC2 instances`);

  // Full query with tags
  const result = queryBookmarks(myBookmark.id, { detail: "tags" });
  if (result.status === "success") {
    result.data.forEach(r => {
      console.log(`${r.displayName} - ${r.status}`);
    });
  }
}

// Execute bookmark by ID directly (if you know the ID)
const result = queryBookmarks("a1b2c3d4-e5f6-7890-abcd-ef1234567890");
console.log(`Found ${result.count} resources`);

// Handle errors
if (result.status === "error") {
  console.error("Bookmark query failed:", result.errors);
}
```

**Error handling:**
- Returns error result if bookmark ID not found
- Returns error result if AWS client not initialized (login required)
- Partial results if some account/region queries fail
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_query_scope() {
        let accounts = vec!["123456789012".to_string()];
        let regions = vec!["us-east-1".to_string()];
        let resource_types = vec!["AWS::EC2::Instance".to_string()];

        let scope = build_query_scope(&accounts, &regions, &resource_types).unwrap();

        assert_eq!(scope.accounts.len(), 1);
        assert_eq!(scope.regions.len(), 1);
        assert_eq!(scope.resource_types.len(), 1);
        assert_eq!(scope.accounts[0].account_id, "123456789012");
        assert_eq!(scope.regions[0].region_code, "us-east-1");
        assert_eq!(scope.resource_types[0].resource_type, "AWS::EC2::Instance");
        assert_eq!(scope.resource_types[0].service_name, "EC2");
    }

    #[test]
    fn test_extract_service_name() {
        let resource_types = vec![
            "AWS::EC2::Instance".to_string(),
            "AWS::S3::Bucket".to_string(),
            "AWS::IAM::Role".to_string(),
        ];

        let scope = build_query_scope(
            &["123456789012".to_string()],
            &["us-east-1".to_string()],
            &resource_types,
        )
        .unwrap();

        assert_eq!(scope.resource_types[0].service_name, "EC2");
        assert_eq!(scope.resource_types[1].service_name, "S3");
        assert_eq!(scope.resource_types[2].service_name, "IAM");
    }

    #[test]
    fn test_documentation_format() {
        let docs = get_documentation();

        // Verify required documentation elements for queryResources
        assert!(docs.contains("queryResources()"));
        assert!(docs.contains("function queryResources("));
        assert!(docs.contains("QueryOptions"));
        assert!(docs.contains("QueryResult"));
        assert!(docs.contains("detail?:"));
        assert!(docs.contains("'count' | 'summary' | 'tags' | 'full'"));
        assert!(docs.contains("status: 'success' | 'partial' | 'error'"));
        assert!(docs.contains("warnings:"));
        assert!(docs.contains("errors:"));
        assert!(docs.contains("Return value structure:"));
        assert!(docs.contains("```json"));
        assert!(docs.contains("Example usage:"));
        assert!(docs.contains("Error handling:"));
        assert!(docs.contains("Performance considerations:"));

        // Verify listBookmarks documentation
        assert!(docs.contains("listBookmarks()"));
        assert!(docs.contains("function listBookmarks(): BookmarkInfo[]"));
        assert!(docs.contains("interface BookmarkInfo"));
        assert!(docs.contains("accountIds: string[]"));
        assert!(docs.contains("regionCodes: string[]"));
        assert!(docs.contains("resourceTypes: string[]"));
        assert!(docs.contains("hasTagFilters: boolean"));

        // Verify queryBookmarks documentation
        assert!(docs.contains("queryBookmarks()"));
        assert!(docs.contains("function queryBookmarks("));
        assert!(docs.contains("bookmarkId: string"));
    }

    #[test]
    fn test_categorize_error() {
        // Test access denied
        let (code, is_warning) = categorize_error("Access Denied: You are not authorized");
        assert_eq!(code, "AccessDenied");
        assert!(!is_warning);

        // Test rate limiting (should be warning)
        let (code, is_warning) = categorize_error("Rate exceeded, please slow down");
        assert_eq!(code, "RateLimitExceeded");
        assert!(is_warning);

        // Test throttling (should be warning)
        let (code, is_warning) = categorize_error("Request throttled");
        assert_eq!(code, "RateLimitExceeded");
        assert!(is_warning);

        // Test timeout (should be warning)
        let (code, is_warning) = categorize_error("Connection timed out");
        assert_eq!(code, "Timeout");
        assert!(is_warning);

        // Test not found
        let (code, is_warning) = categorize_error("Resource not found");
        assert_eq!(code, "NotFound");
        assert!(!is_warning);

        // Test invalid request
        let (code, is_warning) = categorize_error("Invalid parameter value");
        assert_eq!(code, "InvalidRequest");
        assert!(!is_warning);

        // Test unknown error
        let (code, is_warning) = categorize_error("Something unexpected happened");
        assert_eq!(code, "UnknownError");
        assert!(!is_warning);
    }

    #[test]
    fn test_query_resources_args_deserialize() {
        // Test with all fields
        let json = r#"{
            "accounts": ["123456789012"],
            "regions": ["us-east-1"],
            "resourceTypes": ["AWS::EC2::Instance"],
            "detail": "full"
        }"#;
        let args: QueryResourcesArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.accounts, Some(vec!["123456789012".to_string()]));
        assert_eq!(args.regions, Some(vec!["us-east-1".to_string()]));
        assert_eq!(args.resource_types, vec!["AWS::EC2::Instance".to_string()]);
        assert_eq!(args.detail, Some("full".to_string()));

        // Test with minimal fields (detail optional)
        let json_minimal = r#"{
            "resourceTypes": ["AWS::S3::Bucket"]
        }"#;
        let args_minimal: QueryResourcesArgs = serde_json::from_str(json_minimal).unwrap();
        assert_eq!(args_minimal.accounts, None);
        assert_eq!(args_minimal.regions, None);
        assert_eq!(args_minimal.detail, None);
    }

    #[test]
    fn test_query_bookmarks_args_deserialize() {
        // Test with detail
        let json = r#"{"detail": "tags"}"#;
        let args: QueryBookmarksArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.detail, Some("tags".to_string()));

        // Test with empty object
        let json_empty = r#"{}"#;
        let args_empty: QueryBookmarksArgs = serde_json::from_str(json_empty).unwrap();
        assert_eq!(args_empty.detail, None);
    }
}
