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
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::app::agent_framework::utils::registry::get_global_aws_client;
use crate::app::resource_explorer::state::{
    AccountSelection, QueryScope, RegionSelection, ResourceEntry, ResourceExplorerState,
    ResourceTypeSelection,
};
use crate::app::resource_explorer::unified_query::{
    BookmarkInfo, DetailLevel, QueryError, QueryWarning, ResourceFull, ResourceSummary,
    ResourceWithTags, UnifiedQueryResult,
};
use crate::app::resource_explorer::{get_global_bookmark_manager, get_global_explorer_state};

// ============================================================================
// Context-Optimized Resource Query Structs
// ============================================================================

/// JavaScript function call arguments for loadCache()
///
/// Queries AWS resources and returns counts per scope combination.
/// Designed to minimize LLM context usage by returning only count metadata
/// instead of full resource arrays (~99% reduction).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCacheArgs {
    /// Account IDs to query (undefined = all configured accounts)
    pub accounts: Option<Vec<String>>,

    /// Region codes to query (undefined = all enabled regions)
    pub regions: Option<Vec<String>>,

    /// CloudFormation resource types (REQUIRED, cannot be empty)
    pub resource_types: Vec<String>,
}

/// Result from loadCache() - returns counts instead of resource data
///
/// This structure is designed for minimal context consumption. Instead of
/// returning massive resource arrays, it returns count breakdowns per
/// account:region:resourceType combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCacheResult {
    /// Query status: 'success', 'partial', 'error'
    pub status: String,

    /// Count breakdown per account:region:resourceType combination
    /// Key format: "account_id:region_code:resource_type"
    /// Example: {"123456789012:us-east-1:AWS::EC2::Instance": 45}
    pub count_by_scope: HashMap<String, usize>,

    /// Total count across all scopes
    pub total_count: usize,

    /// Non-fatal warnings (rate limiting, timeouts, etc.)
    pub warnings: Vec<QueryWarning>,

    /// Fatal errors per account/region
    pub errors: Vec<QueryError>,

    /// Actual accounts queried (resolved from input)
    pub accounts_queried: Vec<String>,

    /// Actual regions queried (resolved from input)
    pub regions_queried: Vec<String>,

    /// ISO timestamp when load completed
    pub load_timestamp_utc: String,
}

/// JavaScript function call arguments for getResourceSchema()
///
/// Returns ONE example resource from cache to show structure and available properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResourceSchemaArgs {
    /// CloudFormation resource type to get schema for
    pub resource_type: String,
}

/// Tag key-value pair for example resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTag {
    pub key: String,
    pub value: String,
}

/// Example resource showing structure and available properties
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExampleResource {
    pub resource_id: String,
    pub display_name: String,
    pub account_id: String,
    pub region: String,
    pub properties: serde_json::Value,
    pub tags: Vec<ResourceTag>,
    pub status: Option<String>,
}

/// Cache statistics for a resource type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub total_count: usize,
    pub account_count: usize,
    pub region_count: usize,
}

/// Result from getResourceSchema()
///
/// Returns ONE example resource from the cache to demonstrate the structure
/// and available properties for a given resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResourceSchemaResult {
    /// Status: 'success' if example found, 'not_found' if no resources in cache
    pub status: String,

    /// The resource type requested
    pub resource_type: String,

    /// Example resource (FIRST resource found in cache, deterministic)
    pub example_resource: Option<ExampleResource>,

    /// Cache statistics for this resource type
    pub cache_stats: Option<CacheStats>,

    /// Message if status is 'not_found'
    pub message: Option<String>,
}

/// Grouping mode for Explorer window visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum GroupingMode {
    ByAccount,
    ByRegion,
    ByResourceType,
    ByTag { key: String },
    ByTagHierarchy { keys: Vec<String> },
    ByProperty { path: String },
    ByPropertyHierarchy { paths: Vec<String> },
}

/// Tag filter for filtering resources by tag values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagFilter {
    pub tag_key: String,
    pub filter_type: String, // 'Equals', 'NotEquals', 'Contains', 'StartsWith', 'EndsWith', 'Regex', 'Exists', 'NotExists', 'In', 'NotIn'
    pub values: Option<Vec<String>>,
    pub pattern: Option<String>,
}

/// Tag filter group with boolean logic (AND/OR) and nested groups
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagFilterGroup {
    pub operator: String, // 'And' | 'Or'
    pub filters: Vec<TagFilter>,
    pub sub_groups: Option<Vec<TagFilterGroup>>,
}

/// JavaScript function call arguments for showInExplorer()
///
/// Opens the Explorer window with dynamic configuration. This is a V8 function,
/// NOT a tool. The Explorer window queries data independently (cache-aware).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowInExplorerArgs {
    /// Account IDs to display (undefined = all)
    pub accounts: Option<Vec<String>>,

    /// Region codes to display (undefined = all)
    pub regions: Option<Vec<String>>,

    /// CloudFormation resource types (undefined = all)
    pub resource_types: Option<Vec<String>>,

    /// Grouping mode for visualization
    pub grouping: Option<GroupingMode>,

    /// Tag filters for resource filtering
    pub tag_filters: Option<TagFilterGroup>,

    /// Search filter text
    pub search_filter: Option<String>,

    /// Display title for the view
    pub title: Option<String>,
}

/// Result from showInExplorer()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowInExplorerResult {
    /// Status: 'success' if window opened, 'error' otherwise
    pub status: String,

    /// Optional message (error details if status is 'error')
    pub message: Option<String>,

    /// Number of resources to be displayed (if known)
    pub resources_displayed: Option<usize>,
}

/// Arguments for queryCachedResources()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryCachedResourcesArgs {
    /// Account IDs to filter (null = all cached accounts)
    pub accounts: Option<Vec<String>>,

    /// Region codes to filter (null = all cached regions)
    pub regions: Option<Vec<String>>,

    /// Resource types to query (REQUIRED, can be multiple)
    pub resource_types: Vec<String>,
}

/// Result from queryCachedResources()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryCachedResourcesResult {
    /// Status: 'success' if resources found, 'not_found' if cache is empty
    pub status: String,

    /// Array of cached resources (full ResourceEntry objects serialized to JSON)
    /// Always present - empty array if no resources
    pub resources: Vec<serde_json::Value>,

    /// Total count of resources returned
    pub count: usize,

    /// Accounts that had cached data
    pub accounts_with_data: Vec<String>,

    /// Regions that had cached data
    pub regions_with_data: Vec<String>,

    /// Resource types found
    pub resource_types_found: Vec<String>,

    /// Message (helpful if status is not_found)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Register resource-related functions into V8 context
pub fn register(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // CONTEXT-OPTIMIZED RESOURCE QUERY FUNCTIONS
    // Minimize LLM context by returning counts/schemas instead of full resource arrays

    // Register loadCache() function
    let load_cache_fn =
        v8::Function::new(scope, load_cache_callback).expect("Failed to create loadCache function");
    let fn_name =
        v8::String::new(scope, "loadCache").expect("Failed to create function name string");
    global.set(scope, fn_name.into(), load_cache_fn.into());

    // Register getResourceSchema() function
    let get_schema_fn = v8::Function::new(scope, get_resource_schema_callback)
        .expect("Failed to create getResourceSchema function");
    let fn_name =
        v8::String::new(scope, "getResourceSchema").expect("Failed to create function name string");
    global.set(scope, fn_name.into(), get_schema_fn.into());

    // Register showInExplorer() function
    let show_explorer_fn = v8::Function::new(scope, show_in_explorer_callback)
        .expect("Failed to create showInExplorer function");
    let fn_name =
        v8::String::new(scope, "showInExplorer").expect("Failed to create function name string");
    global.set(scope, fn_name.into(), show_explorer_fn.into());

    // Register queryCachedResources() function
    let query_cached_fn = v8::Function::new(scope, query_cached_resources_callback)
        .expect("Failed to create queryCachedResources function");
    let fn_name = v8::String::new(scope, "queryCachedResources")
        .expect("Failed to create function name string");
    global.set(scope, fn_name.into(), query_cached_fn.into());

    // BOOKMARK FUNCTIONS
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

/// Callback for loadCache() JavaScript function
///
/// Queries AWS resources but returns counts per account:region:resourceType
/// instead of full resource data. Designed for minimal LLM context consumption
/// (~99% reduction compared to returning full resource arrays).
fn load_cache_callback(
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
                "loadCache() requires an object argument with { accounts?, regions?, resourceTypes }",
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

    // Parse JSON into LoadCacheArgs
    let load_args: LoadCacheArgs = match serde_json::from_str(&json_str) {
        Ok(args) => args,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to parse arguments: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Execute query and count results
    let result = match execute_load_cache(load_args) {
        Ok(result) => result,
        Err(e) => {
            // Return error result instead of throwing exception
            let error_result = LoadCacheResult {
                status: "error".to_string(),
                count_by_scope: HashMap::new(),
                total_count: 0,
                warnings: vec![],
                errors: vec![QueryError {
                    account: "all".to_string(),
                    region: "all".to_string(),
                    code: "LoadCacheFailed".to_string(),
                    message: e.to_string(),
                }],
                accounts_queried: vec![],
                regions_queried: vec![],
                load_timestamp_utc: chrono::Utc::now().to_rfc3339(),
            };
            if let Ok(json) = serde_json::to_string(&error_result) {
                if let Some(v8_str) = v8::String::new(scope, &json) {
                    if let Some(v8_value) = v8::json::parse(scope, v8_str) {
                        rv.set(v8_value);
                        return;
                    }
                }
            }
            let msg = v8::String::new(scope, &format!("Load cache failed: {}", e)).unwrap();
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

/// Execute load cache query (synchronous wrapper for async code)
pub fn execute_load_cache(args: LoadCacheArgs) -> Result<LoadCacheResult> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async { load_cache_internal(args).await })
    })
}

/// Execute complete resource query with automatic enrichment (BLACK BOX API)
///
/// This is the canonical way to query AWS resources - it encapsulates Explorer's
/// complete workflow including Phase 1 queries, Phase 2 enrichment, and caching.
///
/// # Architecture
///
/// This function acts as a **black box API** that agent framework code calls without
/// needing to understand Explorer's internal implementation. If Explorer changes its
/// query strategy, enrichment logic, or caching approach, this function's signature
/// stays the same.
///
/// # What This Function Does (Transparent to Caller)
///
/// 1. **Phase 1: Parallel AWS Queries**
///    - Calls `query_aws_resources_parallel()` for all account/region/type combinations
///    - Results stream back via channels and accumulate in cache
///    - Uses Explorer's proven concurrent collection pattern
///
/// 2. **Phase 2: Automatic Enrichment** (for security-critical resources)
///    - Security groups, S3 buckets, IAM roles automatically get detailed properties
///    - Calls AWS Describe APIs to get full configuration (e.g., security group rules)
///    - Enrichment happens transparently - caller doesn't need to request it
///
/// 3. **Cache Management**
///    - Updates Explorer's global cache atomically
///    - Ensures consistency between Explorer UI and agent queries
///    - Cache keyed by "account:region:resourceType"
///
/// 4. **Resource Filtering**
///    - Returns only resources matching the requested scope
///    - Filters happen AFTER enrichment to avoid partial data
///
/// # Integration Points
///
/// - **Explorer UI**: Can call this from UI thread (via tokio runtime)
/// - **Agent Framework**: Calls this from async context (already in tokio)
/// - **Future APIs**: Any code needing AWS resource data should call this
///
/// # Cache Strategy
///
/// The function clones Explorer's cache at the start, runs queries updating the clone,
/// then atomically syncs the final cache back. This prevents:
/// - Race conditions between concurrent queries
/// - Partial cache states visible to other callers
/// - Inconsistency between loadCache() and queryCachedResources()
///
/// # Why Phase 2 is Automatic
///
/// Security analysis (the primary agent use case) requires detailed properties:
/// - Security Groups: Need IpPermissions with FromPort, ToPort, IpRanges
/// - S3 Buckets: Need bucket policies and ACLs
/// - IAM Roles: Need trust policies and permissions
///
/// Without Phase 2, agents would see simplified data like:
/// ```json
/// "IpPermissions": [{"IpProtocol": "-1"}]  // Missing port ranges!
/// ```
///
/// With Phase 2, agents see complete data:
/// ```json
/// "IpPermissions": [{
///   "IpProtocol": "tcp",
///   "FromPort": 22,
///   "ToPort": 22,
///   "IpRanges": [{"CidrIp": "0.0.0.0/0"}]
/// }]
/// ```
///
/// # Parameters
///
/// - `account_ids`: AWS account IDs to query (must not be empty)
/// - `region_codes`: AWS regions to query (must not be empty)
/// - `resource_types`: CloudFormation types like "AWS::EC2::SecurityGroup"
///
/// # Returns
///
/// Vector of `ResourceEntry` with:
/// - Phase 1 data (basic properties from List APIs)
/// - Phase 2 data (detailed properties from Describe APIs) for enrichable types
/// - Filtered to exactly match the requested scope
///
/// # Errors
///
/// Returns error if:
/// - AWS client not initialized
/// - Explorer state not initialized
/// - Query task panics (should never happen)
///
/// Individual query failures (e.g., access denied for one region) are logged
/// but don't fail the entire operation - you get partial results.
///
/// # Example Usage
///
/// ```rust
/// // Agent framework example:
/// let resources = execute_complete_query(
///     vec!["123456789012".to_string()],
///     vec!["us-east-1".to_string()],
///     vec!["AWS::EC2::SecurityGroup".to_string()],
/// ).await?;
///
/// // Now resources contains ALL security groups with FULL rules (Phase 2 enriched)
/// // Agent can analyze: resources[0].detailed_properties["IpPermissions"]
/// ```
async fn execute_complete_query(
    account_ids: Vec<String>,
    region_codes: Vec<String>,
    resource_types: Vec<String>,
) -> Result<Vec<ResourceEntry>> {
    // Get global AWS client
    let client = get_global_aws_client().ok_or_else(|| anyhow!("AWS client not initialized"))?;

    // Validate resource types
    if resource_types.is_empty() {
        return Err(anyhow!("resource_types array cannot be empty"));
    }

    // Build QueryScope
    let scope = build_query_scope(&account_ids, &region_codes, &resource_types)?;

    // Use the shared Moka cache - same cache used by Explorer UI
    // This provides unified caching between Agent and Explorer queries
    let cache = crate::app::resource_explorer::cache::shared_cache();

    // Create channels
    let (result_tx, mut result_rx) = mpsc::channel(1000);

    // Collect resources like Explorer does
    let all_resources = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let all_resources_clone = all_resources.clone();

    // Start parallel query
    let client_clone = Arc::clone(&client);
    let cache_clone = cache.clone();
    let scope_clone = scope.clone();

    let query_future = async move {
        client_clone
            .query_aws_resources_parallel(&scope_clone, result_tx, None, cache_clone)
            .await
    };

    // Process results (Explorer pattern)
    let result_processing = async move {
        while let Some(result) = result_rx.recv().await {
            match result.resources {
                Ok(resources) => {
                    let mut all_res = all_resources_clone.lock().await;
                    all_res.extend(resources);
                }
                Err(e) => {
                    warn!(
                        "Query failed for {}:{}:{}: {}",
                        result.account_id, result.region, result.resource_type, e
                    );
                }
            }
        }
    };

    // Wait for both to complete (Explorer uses tokio::join!)
    let (query_result, _) = tokio::join!(query_future, result_processing);
    if let Err(e) = query_result {
        warn!("Query error: {}", e);
    }

    // Get final resources from Phase 1
    let phase1_resources = all_resources.lock().await.clone();

    // PHASE 2: Automatic enrichment for security-critical resources
    //
    // This is transparent to the caller - they just get fully-enriched resources.
    // Explorer determines which types need enrichment (security groups, S3 buckets, IAM roles, etc.)
    let enrichable_types = ResourceExplorerState::enrichable_resource_types();
    let needs_enrichment = resource_types
        .iter()
        .any(|rt| enrichable_types.contains(&rt.as_str()));

    if needs_enrichment {
        info!("Phase 2: Auto-enriching security-critical resource types");

        // Find resources from Phase 1 that need enrichment
        let resources_to_enrich: Vec<ResourceEntry> = phase1_resources
            .iter()
            .filter(|r| {
                enrichable_types.contains(&r.resource_type.as_str())
                    && r.detailed_properties.is_none() // Only enrich if not already enriched
            })
            .cloned()
            .collect();

        if !resources_to_enrich.is_empty() {
            info!(
                "Phase 2: Enriching {} resources with detailed properties",
                resources_to_enrich.len()
            );

            // Create channels for Phase 2 progress tracking
            let (progress_tx, mut progress_rx) = mpsc::channel(100);
            let (result_tx, _result_rx) = mpsc::channel(100);

            // Use shared cache directly - no need to clone/sync since it's shared globally
            let phase2_cache = cache.clone();

            // Start Phase 2 enrichment (spawns background AWS Describe API calls)
            client.start_phase2_enrichment(
                resources_to_enrich,
                result_tx,
                Some(progress_tx),
                phase2_cache,
            );

            // Wait for Phase 2 to complete
            // This blocks until all Describe API calls finish
            while let Some(progress) = progress_rx.recv().await {
                if matches!(
                    progress.status,
                    super::super::super::super::resource_explorer::aws_client::QueryStatus::EnrichmentCompleted
                ) {
                    info!("Phase 2: Enrichment completed successfully");
                    break;
                }
            }
            // No sync needed - SharedResourceCache is shared globally between Agent and Explorer
        } else {
            info!("Phase 2: All resources already enriched (cached from previous query)");
        }
    }

    // Filter resources to match the requested scope
    // This uses Explorer's exact filtering logic
    let filtered_resources: Vec<ResourceEntry> = phase1_resources
        .iter()
        .filter(|resource| {
            // Match account
            account_ids.contains(&resource.account_id)
                // Match region
                && region_codes.contains(&resource.region)
                // Match resource type
                && resource_types.contains(&resource.resource_type)
        })
        .cloned()
        .collect();

    info!(
        "Query complete: {} resources (Phase 1: {}, enriched: {})",
        filtered_resources.len(),
        phase1_resources.len(),
        filtered_resources
            .iter()
            .filter(|r| r.detailed_properties.is_some())
            .count()
    );

    Ok(filtered_resources)
}

/// Internal async implementation of load cache
///
/// Queries AWS resources but returns counts instead of full data to minimize context.
/// This achieves ~99% context reduction by returning metadata instead of resource arrays.
pub async fn load_cache_internal(args: LoadCacheArgs) -> Result<LoadCacheResult> {
    let start_time = chrono::Utc::now();

    // Resolve accounts: if None/empty, get ALL configured accounts
    let account_ids = match args.accounts {
        Some(ids) if !ids.is_empty() => ids,
        _ => {
            // Get ALL accounts from GLOBAL_AWS_IDENTITY
            let identity = super::accounts::get_global_aws_identity()
                .ok_or_else(|| anyhow!("AWS Identity Center not initialized"))?;

            let identity_guard = identity
                .lock()
                .map_err(|e| anyhow!("Failed to lock AwsIdentityCenter: {}", e))?;

            if identity_guard.accounts.is_empty() {
                return Err(anyhow!("No AWS accounts configured"));
            }

            identity_guard
                .accounts
                .iter()
                .map(|acc| acc.account_id.clone())
                .collect()
        }
    };

    // Resolve regions: if None/empty, use common AWS regions
    let region_codes = match args.regions {
        Some(regions) if !regions.is_empty() => regions,
        _ => {
            info!("No regions specified, using common AWS regions");
            vec![
                "us-east-1".to_string(),
                "us-west-2".to_string(),
                "eu-west-1".to_string(),
                "ap-southeast-1".to_string(),
            ]
        }
    };

    // Call the black-box API - it handles everything (Phase 1, Phase 2, caching)
    let all_resources = execute_complete_query(
        account_ids.clone(),
        region_codes.clone(),
        args.resource_types.clone(),
    )
    .await?;

    // Calculate counts by scope from returned resources
    let mut count_by_scope: HashMap<String, usize> = HashMap::new();
    for resource in &all_resources {
        let cache_key = format!(
            "{}:{}:{}",
            resource.account_id, resource.region, resource.resource_type
        );
        *count_by_scope.entry(cache_key).or_insert(0) += 1;
    }

    let total_count = all_resources.len();
    info!(
        "loadCache: Returning summary for {} total resources",
        total_count
    );

    // Return success result
    Ok(LoadCacheResult {
        status: "success".to_string(),
        count_by_scope,
        total_count,
        warnings: Vec::new(), // Shared function handles errors internally
        errors: Vec::new(),
        accounts_queried: account_ids,
        regions_queried: region_codes,
        load_timestamp_utc: start_time.to_rfc3339(),
    })
}

/// Callback for getResourceSchema() JavaScript function
///
/// Returns ONE example resource from the cache to show structure and available properties.
fn get_resource_schema_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Parse resourceType from first argument
    let resource_type = match args.get(0).to_string(scope) {
        Some(s) => s.to_rust_string_lossy(scope),
        None => {
            let msg = v8::String::new(
                scope,
                "getResourceSchema() requires a resource type string as first argument",
            )
            .unwrap();
            let error = v8::Exception::type_error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Execute schema lookup
    let result = match execute_get_resource_schema(&resource_type) {
        Ok(result) => result,
        Err(e) => {
            // Return not_found result instead of throwing exception
            let error_result = GetResourceSchemaResult {
                status: "not_found".to_string(),
                resource_type: resource_type.clone(),
                example_resource: None,
                cache_stats: None,
                message: Some(format!("Failed to get schema: {}", e)),
            };
            if let Ok(json) = serde_json::to_string(&error_result) {
                if let Some(v8_str) = v8::String::new(scope, &json) {
                    if let Some(v8_value) = v8::json::parse(scope, v8_str) {
                        rv.set(v8_value);
                        return;
                    }
                }
            }
            let msg = v8::String::new(scope, &format!("Get schema failed: {}", e)).unwrap();
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

/// Execute get resource schema (synchronous wrapper for async code)
pub fn execute_get_resource_schema(resource_type: &str) -> Result<GetResourceSchemaResult> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { get_resource_schema_internal(resource_type).await })
    })
}

/// Convert a serde_json::Value to a type string representation
fn value_to_type_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "array".to_string()
            } else {
                // Get the type of array elements (use first element)
                let element_type = value_to_type_string(&arr[0]);
                format!("array<{}>", element_type)
            }
        }
        serde_json::Value::Object(_) => "object".to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

/// Convert a serde_json::Value to a schema example by recursively replacing values with type indicators
fn value_to_schema_example(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(obj) => {
            let mut schema_obj = serde_json::Map::new();
            for (key, val) in obj {
                schema_obj.insert(key.clone(), value_to_schema_example(val));
            }
            serde_json::Value::Object(schema_obj)
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                serde_json::Value::Array(vec![])
            } else {
                // Show schema for first element
                serde_json::Value::Array(vec![value_to_schema_example(&arr[0])])
            }
        }
        _ => serde_json::Value::String(value_to_type_string(value)),
    }
}

/// Internal async implementation of get resource schema
///
/// Searches the ResourceExplorerState cache for up to 1000 resources of the given type.
/// Merges all properties from all resources to show ALL possible properties.
/// Replaces actual values with type indicators (string, number, boolean, array, object, null).
pub async fn get_resource_schema_internal(resource_type: &str) -> Result<GetResourceSchemaResult> {
    // Get global explorer state
    let explorer_state = get_global_explorer_state()
        .ok_or_else(|| anyhow!("Explorer state not initialized (login required)"))?;

    let state_guard = explorer_state.read().await;

    // Collect up to 1000 resources of this type
    const MAX_RESOURCES_FOR_SCHEMA: usize = 1000;
    let mut collected_resources: Vec<&ResourceEntry> = Vec::new();
    let mut unique_accounts: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut unique_regions: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (_cache_key, entries) in &state_guard.cached_queries {
        for entry in entries {
            if entry.resource_type == resource_type {
                if collected_resources.len() < MAX_RESOURCES_FOR_SCHEMA {
                    collected_resources.push(entry);
                }
                unique_accounts.insert(entry.account_id.clone());
                unique_regions.insert(entry.region.clone());
            }
        }
    }

    // Build result
    if !collected_resources.is_empty() {
        info!(
            "Building schema from {} resources of type {}",
            collected_resources.len(),
            resource_type
        );

        // Merge properties from ALL collected resources
        // Track all property keys and their example values
        let mut merged_properties = serde_json::Map::new();

        for entry in &collected_resources {
            // Layer 1: properties (normalized minimal data)
            if let Some(props) = entry.properties.as_object() {
                for (key, value) in props {
                    // Only insert if not already present (keep first occurrence)
                    if !merged_properties.contains_key(key) {
                        merged_properties.insert(key.clone(), value.clone());
                    }
                }
            }

            // Layer 2: raw_properties (Phase 1 List API response)
            if let Some(raw) = entry.raw_properties.as_object() {
                for (key, value) in raw {
                    if !merged_properties.contains_key(key) {
                        merged_properties.insert(key.clone(), value.clone());
                    }
                }
            }

            // Layer 3: detailed_properties (Phase 2 Describe API response)
            if let Some(detailed) = entry.detailed_properties.as_ref() {
                if let Some(detailed_obj) = detailed.as_object() {
                    for (key, value) in detailed_obj {
                        if !merged_properties.contains_key(key) {
                            merged_properties.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
        }

        info!(
            "Schema for {} contains {} unique properties",
            resource_type,
            merged_properties.len()
        );

        // Convert merged properties to schema (replace values with type indicators)
        let schema_properties = value_to_schema_example(&serde_json::Value::Object(merged_properties));

        Ok(GetResourceSchemaResult {
            status: "success".to_string(),
            resource_type: resource_type.to_string(),
            example_resource: Some(ExampleResource {
                resource_id: format!("<{}>", value_to_type_string(&serde_json::Value::String(String::new()))),
                display_name: format!("<{}>", value_to_type_string(&serde_json::Value::String(String::new()))),
                account_id: format!("<{}>", value_to_type_string(&serde_json::Value::String(String::new()))),
                region: format!("<{}>", value_to_type_string(&serde_json::Value::String(String::new()))),
                properties: schema_properties,
                tags: vec![ResourceTag {
                    key: "<string>".to_string(),
                    value: "<string>".to_string(),
                }],
                status: Some("<string>".to_string()),
            }),
            cache_stats: Some(CacheStats {
                total_count: collected_resources.len(),
                account_count: unique_accounts.len(),
                region_count: unique_regions.len(),
            }),
            message: Some(format!(
                "Schema merged from {} resources showing all possible properties",
                collected_resources.len()
            )),
        })
    } else {
        // Build helpful error message with cache statistics
        use std::collections::HashSet;

        let cache_size = state_guard.cached_queries.len();
        let mut cache_types = HashSet::new();

        for cache_key in state_guard.cached_queries.keys() {
            let parts: Vec<&str> = cache_key.split(':').collect();
            if parts.len() == 3 {
                cache_types.insert(parts[2].to_string());
            }
        }

        let mut message = format!(
            "No resources of type '{}' found in cache.\n\n",
            resource_type
        );

        if cache_size == 0 {
            message.push_str("Cache is empty. Did you call loadCache() first?");
        } else {
            message.push_str(&format!(
                "Cache contains {} other resource types:\n  ",
                cache_types.len()
            ));

            let mut available_types: Vec<_> = cache_types.iter().map(|s| s.as_str()).collect();
            available_types.sort();
            message.push_str(&available_types.join(", "));
            message.push_str(&format!(
                "\n\nTip: Call loadCache() with resourceTypes: ['{}'] to populate this type.",
                resource_type
            ));
        }

        Ok(GetResourceSchemaResult {
            status: "not_found".to_string(),
            resource_type: resource_type.to_string(),
            example_resource: None,
            cache_stats: None,
            message: Some(message),
        })
    }
}

/// Callback for showInExplorer() JavaScript function
///
/// Opens the Explorer window with dynamic configuration from JavaScript.
/// Enqueues an action for the Explorer window to poll and apply.
fn show_in_explorer_callback(
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
                "showInExplorer() requires an object argument with optional { accounts?, regions?, resourceTypes?, grouping?, tagFilters?, searchFilter?, title? }",
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

    // Parse JSON into ShowInExplorerArgs
    let show_args: ShowInExplorerArgs = match serde_json::from_str(&json_str) {
        Ok(args) => args,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to parse arguments: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Enqueue action for Explorer window
    crate::app::resource_explorer::enqueue_explorer_action(
        crate::app::resource_explorer::ExplorerAction::OpenWithConfig(show_args),
    );

    // Return success result
    let result = ShowInExplorerResult {
        status: "success".to_string(),
        message: Some("Explorer window action enqueued".to_string()),
        resources_displayed: None, // Unknown until Explorer processes the action
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

/// Callback for queryCachedResources() JavaScript function
///
/// Queries cached resources by account, region, and resource type.
/// Returns actual ResourceEntry objects (not just counts) for filtering/analysis.
fn query_cached_resources_callback(
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
                "queryCachedResources() requires an object argument with { accounts?, regions?, resourceTypes }",
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

    // Parse JSON into QueryCachedResourcesArgs
    let query_args: QueryCachedResourcesArgs = match serde_json::from_str(&json_str) {
        Ok(args) => args,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to parse arguments: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Execute query synchronously (wraps async code)
    let result = match execute_query_cached_resources(query_args) {
        Ok(r) => r,
        Err(e) => {
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

/// Execute queryCachedResources (synchronous wrapper for async code)
pub fn execute_query_cached_resources(
    args: QueryCachedResourcesArgs,
) -> Result<QueryCachedResourcesResult> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { query_cached_resources_internal(args).await })
    })
}

/// Internal async implementation of queryCachedResources
///
/// Thin wrapper around Explorer's query infrastructure (same as loadCache).
/// The query engine handles caching transparently - we just return full resources
/// instead of counts.
pub async fn query_cached_resources_internal(
    args: QueryCachedResourcesArgs,
) -> Result<QueryCachedResourcesResult> {
    use std::collections::HashSet;

    // Resolve accounts: if None/empty, get ALL configured accounts
    let account_ids = match &args.accounts {
        Some(ids) if !ids.is_empty() => ids.clone(),
        _ => {
            // Get ALL accounts from GLOBAL_AWS_IDENTITY
            let identity = super::accounts::get_global_aws_identity()
                .ok_or_else(|| anyhow!("AWS Identity Center not initialized"))?;

            let identity_guard = identity
                .lock()
                .map_err(|e| anyhow!("Failed to lock AwsIdentityCenter: {}", e))?;

            if identity_guard.accounts.is_empty() {
                return Err(anyhow!("No AWS accounts configured"));
            }

            identity_guard
                .accounts
                .iter()
                .map(|acc| acc.account_id.clone())
                .collect()
        }
    };

    // Resolve regions: if None/empty, use common AWS regions
    let region_codes = match &args.regions {
        Some(regions) if !regions.is_empty() => regions.clone(),
        _ => {
            info!("No regions specified, using common AWS regions");
            vec![
                "us-east-1".to_string(),
                "us-west-2".to_string(),
                "eu-west-1".to_string(),
                "ap-southeast-1".to_string(),
            ]
        }
    };

    // Execute the complete query (same as loadCache)
    // Both functions execute the query, but return different data:
    // - loadCache returns counts/metadata
    // - queryCachedResources returns actual resource objects
    let all_resources =
        execute_complete_query(account_ids, region_codes, args.resource_types.clone()).await?;

    // Format resources for JavaScript consumption
    // Convert ResourceEntry objects to JavaScript-friendly JSON with merged properties
    let mut js_resources = Vec::new();
    let mut accounts_found = HashSet::new();
    let mut regions_found = HashSet::new();
    let mut types_found = HashSet::new();

    for entry in &all_resources {
        // Merge all property fields into single object for JavaScript
        // Layer 1: properties (normalized minimal)
        // Layer 2: raw_properties (Phase 1 List API data)
        // Layer 3: detailed_properties (Phase 2 Describe API data)
        let mut merged_properties = serde_json::Map::new();

        // Layer 1: properties
        if let Some(props) = entry.properties.as_object() {
            for (key, value) in props {
                merged_properties.insert(key.clone(), value.clone());
            }
        }

        // Layer 2: raw_properties
        if let Some(raw) = entry.raw_properties.as_object() {
            for (key, value) in raw {
                merged_properties.insert(key.clone(), value.clone());
            }
        }

        // Layer 3: detailed_properties (Phase 2 enrichment)
        if let Some(detailed) = entry.detailed_properties.as_ref() {
            if let Some(detailed_obj) = detailed.as_object() {
                for (key, value) in detailed_obj {
                    merged_properties.insert(key.clone(), value.clone());
                }
            }
        }

        // Serialize tags
        let tags_json: Vec<serde_json::Value> = entry
            .tags
            .iter()
            .map(|tag| {
                json!({
                    "key": tag.key,
                    "value": tag.value
                })
            })
            .collect();

        // Create JavaScript-friendly resource
        let js_resource = json!({
            "resourceId": entry.resource_id,
            "displayName": entry.display_name,
            "accountId": entry.account_id,
            "region": entry.region,
            "resourceType": entry.resource_type,
            "properties": serde_json::Value::Object(merged_properties),
            "tags": tags_json,
            "status": entry.status,
        });

        js_resources.push(js_resource);
        accounts_found.insert(entry.account_id.clone());
        regions_found.insert(entry.region.clone());
        types_found.insert(entry.resource_type.clone());
    }

    let count = js_resources.len();

    if count == 0 {
        // Build error message
        let message = format!(
            "No resources found for: {}\n\nQuery completed but returned no matching resources.",
            args.resource_types.join(", ")
        );

        Ok(QueryCachedResourcesResult {
            status: "not_found".to_string(),
            resources: Vec::new(),
            count: 0,
            accounts_with_data: Vec::new(),
            regions_with_data: Vec::new(),
            resource_types_found: Vec::new(),
            message: Some(message),
        })
    } else {
        Ok(QueryCachedResourcesResult {
            status: "success".to_string(),
            resources: js_resources,
            count,
            accounts_with_data: accounts_found.into_iter().collect(),
            regions_with_data: regions_found.into_iter().collect(),
            resource_types_found: types_found.into_iter().collect(),
            message: None,
        })
    }
}

/// Categorize an error message into a code and whether it's a warning (used by loadCache)
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
///
/// This function is public to allow direct use from the webview API
/// without needing to go through the V8 blocking wrapper.
pub async fn query_bookmark_internal(
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

    // Get global AWS client
    let client = get_global_aws_client().ok_or_else(|| anyhow!("AWS client not initialized"))?;

    // Get accounts from bookmark
    let account_ids = if bookmark.account_ids.is_empty() {
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
            "Using random account for bookmark: {} ({})",
            random_account.account_name, random_account.account_id
        );
        vec![random_account.account_id.clone()]
    } else {
        bookmark.account_ids.clone()
    };

    // Get regions from bookmark
    let region_codes = if bookmark.region_codes.is_empty() {
        info!("No regions specified in bookmark, using us-east-1");
        vec!["us-east-1".to_string()]
    } else {
        bookmark.region_codes.clone()
    };

    // Validate resource types are not empty
    if bookmark.resource_type_ids.is_empty() {
        return Err(anyhow!("Bookmark has no resource types configured"));
    }

    // Build QueryScope
    let scope = build_query_scope(&account_ids, &region_codes, &bookmark.resource_type_ids)?;

    // Use the shared Moka cache - same cache used by Explorer UI
    // This provides unified caching between Agent and Explorer queries
    let cache = crate::app::resource_explorer::cache::shared_cache();

    // Create result channel
    let (result_tx, mut result_rx) = mpsc::channel(1000);

    // Track warnings and errors per account/region
    let mut warnings: Vec<QueryWarning> = Vec::new();
    let mut errors: Vec<QueryError> = Vec::new();
    let mut success_count = 0usize;
    let mut error_count = 0usize;

    // Start parallel query (spawns background tasks)
    let client_clone = Arc::clone(&client);
    let cache_clone = Arc::clone(&cache);
    let scope_for_query = scope.clone();
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
            warn!("Bookmark query error: {}", e);
        }
    });

    // Collect all results
    let mut all_entries: Vec<ResourceEntry> = Vec::new();

    while let Some(result) = result_rx.recv().await {
        match result.resources {
            Ok(entries) => {
                success_count += 1;
                // No sync needed - SharedResourceCache is shared globally
                all_entries.extend(entries);
            }
            Err(e) => {
                error_count += 1;
                let error_msg = e.to_string();
                warn!(
                    "Bookmark query error for {}/{}/{}: {}",
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
        "Bookmark query collected {} resources (success: {}, errors: {})",
        total_count, success_count, error_count
    );

    // Apply detail level filtering and convert to JSON
    let data = match detail_level {
        DetailLevel::Count => serde_json::json!(null),
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

    // Build result
    let result = UnifiedQueryResult::from_results(
        data,
        total_count,
        success_count,
        error_count,
        warnings,
        errors,
    );

    Ok(result)
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
