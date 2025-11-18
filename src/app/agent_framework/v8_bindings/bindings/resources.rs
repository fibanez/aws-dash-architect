//! Resource query function bindings
//!
//! Provides JavaScript access to AWS resource querying functionality.
//! This is a code-first approach where JavaScript code calls bound Rust functions
//! to query AWS resources across accounts, regions, and resource types.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::app::agent_framework::tools_registry::get_global_aws_client;
use crate::app::resource_explorer::state::{
    AccountSelection, QueryScope, RegionSelection, ResourceEntry, ResourceTypeSelection,
};

/// Global resource cache shared across all queryResources() calls
/// This cache prevents redundant API calls when the same resources are queried multiple times
static GLOBAL_RESOURCE_CACHE: once_cell::sync::Lazy<
    Arc<tokio::sync::RwLock<HashMap<String, Vec<ResourceEntry>>>>,
> = once_cell::sync::Lazy::new(|| Arc::new(tokio::sync::RwLock::new(HashMap::new())));

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
}

/// Resource information exposed to JavaScript
/// Simplified version of ResourceEntry for V8 consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInfo {
    /// CloudFormation resource type (e.g., "AWS::EC2::Instance")
    pub resource_type: String,

    /// AWS Account ID
    pub account_id: String,

    /// AWS Region code
    pub region: String,

    /// Resource identifier (ARN, ID, name)
    pub resource_id: String,

    /// Human-readable display name
    pub display_name: String,

    /// Resource status (e.g., "running", "stopped")
    pub status: Option<String>,

    /// Normalized resource properties (JSON object)
    pub properties: serde_json::Value,

    /// Original AWS API response (from List queries)
    pub raw_properties: serde_json::Value,

    /// Detailed properties from Describe queries (if available)
    pub detailed_properties: Option<serde_json::Value>,

    /// Resource tags (key-value pairs)
    pub tags: Vec<ResourceTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTag {
    pub key: String,
    pub value: String,
}

/// Register resource-related functions into V8 context
pub fn register(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Register queryResources() function
    let query_resources_fn = v8::Function::new(scope, query_resources_callback)
        .expect("Failed to create queryResources function");

    let fn_name =
        v8::String::new(scope, "queryResources").expect("Failed to create function name string");
    global.set(scope, fn_name.into(), query_resources_fn.into());

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
                "queryResources() requires an object argument with { accounts, regions, resourceTypes }",
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
    let resources = match execute_query(query_args) {
        Ok(resources) => resources,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Query failed: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Serialize to JSON string
    let json_str = match serde_json::to_string(&resources) {
        Ok(json) => json,
        Err(e) => {
            let msg =
                v8::String::new(scope, &format!("Failed to serialize resources: {}", e)).unwrap();
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

/// Execute AWS resource query (synchronous wrapper for async code)
fn execute_query(args: QueryResourcesArgs) -> Result<Vec<ResourceInfo>> {
    // Use the current runtime's handle and block_in_place to avoid nested runtime error
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async { query_resources_internal(args).await })
    })
}

/// Internal async implementation of resource query
async fn query_resources_internal(args: QueryResourcesArgs) -> Result<Vec<ResourceInfo>> {
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

    // Create result channel
    let (result_tx, mut result_rx) = mpsc::channel(1000);

    // Use global resource cache to avoid redundant API calls
    let cache = Arc::clone(&GLOBAL_RESOURCE_CACHE);

    // Start parallel query (spawns background tasks)
    let client_clone = Arc::clone(&client);
    tokio::spawn(async move {
        if let Err(e) = client_clone
            .query_aws_resources_parallel(
                &scope, result_tx, None, // No progress updates needed
                cache,
            )
            .await
        {
            warn!("Query error: {}", e);
        }
    });

    // Collect all results
    let mut resources = Vec::new();

    while let Some(result) = result_rx.recv().await {
        match result.resources {
            Ok(entries) => {
                // Convert ResourceEntry to ResourceInfo
                for entry in entries {
                    resources.push(ResourceInfo {
                        resource_type: entry.resource_type,
                        account_id: entry.account_id,
                        region: entry.region,
                        resource_id: entry.resource_id,
                        display_name: entry.display_name,
                        status: entry.status,
                        properties: entry.properties,
                        raw_properties: entry.raw_properties,
                        detailed_properties: entry.detailed_properties,
                        tags: entry
                            .tags
                            .into_iter()
                            .map(|t| ResourceTag {
                                key: t.key,
                                value: t.value,
                            })
                            .collect(),
                    });
                }
            }
            Err(e) => {
                warn!(
                    "Query error for {}/{}/{}: {}",
                    result.account_id, result.region, result.resource_type, e
                );
                // Continue collecting other results (partial results on error)
            }
        }
    }

    info!("Collected {} resources total", resources.len());
    Ok(resources)
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

Query AWS resources across accounts, regions, and resource types.

**Signature:**
```typescript
function queryResources(options: QueryOptions): ResourceInfo[]

interface QueryOptions {
  accounts?: string[] | null;      // Account IDs (null = use listAccounts to select one)
  regions?: string[] | us-east-1;       // Region codes (null = use if null us-east-1)
  resourceTypes: string[];         // CloudFormation resource types (required)
}

interface ResourceInfo {
  resourceType: string;            // CloudFormation type (e.g., "AWS::EC2::Instance")
  accountId: string;               // AWS Account ID
  region: string;                  // AWS Region code
  resourceId: string;              // Resource identifier (ARN, ID, name)
  displayName: string;             // Human-readable name
  status: string | null;           // Resource status (e.g., "running", "stopped")
  properties: object;              // Normalized resource properties
  tags: Array<{key: string, value: string}>;  // Resource tags
}
```

**Description:**
Executes parallel AWS API queries across specified accounts, regions, and resource types.
Returns a unified list of resources with normalized properties.

**Default Behavior:**
- If `accounts` is `null` or empty: the model shouls use listAccounts and pick ONE random account from configured accounts
- If `regions` is `null` or empty: Uses `us-east-1` region only
- `resourceTypes` is REQUIRED and cannot be empty

**Resource Types:**
Use CloudFormation format (e.g., `AWS::EC2::Instance`, `AWS::S3::Bucket`, `AWS::IAM::Role`).
We support 93 services and 183 resource types, most likely we can query for it.
See AWS CloudFormation documentation for full list of resource types.

**Return value structure:**
```json
[
  {
    "resourceType": "AWS::EC2::Instance",
    "accountId": "123456789012",
    "region": "us-east-1",
    "resourceId": "i-1234567890abcdef0",
    "displayName": "web-server-01",
    "status": "running",
    "properties": {
      "InstanceType": "t3.micro",
      "LaunchTime": "2024-01-15T10:30:00Z",
      "PublicIpAddress": "203.0.113.25"
    },
    "tags": [
      {"key": "Environment", "value": "Production"},
      {"key": "Application", "value": "WebServer"}
    ]
  }
]
```

**Example usage:**
```javascript
// Query EC2 instances in specific account and region
const instances = queryResources({
  accounts: ["123456789012"],
  regions: ["us-east-1"],
  resourceTypes: ["AWS::EC2::Instance"]
});

console.log(`Found ${instances.length} EC2 instances`);
instances.forEach(i => {
  console.log(`${i.displayName}: ${i.status} (${i.properties.InstanceType})`);
});

// Query S3 buckets across all configured accounts (random account selected)
const buckets = queryResources({
  accounts: null,  // Random account
  regions: null,   // S3 is global, queried from us-east-1
  resourceTypes: ["AWS::S3::Bucket"]
});

console.log(`Found ${buckets.length} S3 buckets in random account`);

// Query multiple resource types in multiple regions
const resources = queryResources({
  accounts: listAccounts().map(a => a.id),  // All accounts
  regions: ["us-east-1", "us-west-2"],
  resourceTypes: [
    "AWS::EC2::Instance",
    "AWS::RDS::DBInstance",
    "AWS::Lambda::Function"
  ]
});

// Group by resource type
const byType = {};
resources.forEach(r => {
  byType[r.resourceType] = (byType[r.resourceType] || 0) + 1;
});
console.log("Resources by type:", JSON.stringify(byType, null, 2));

// Filter running EC2 instances
const runningInstances = resources
  .filter(r => r.resourceType === "AWS::EC2::Instance")
  .filter(r => r.status === "running");

// Find resources with specific tag
const prodResources = resources.filter(r =>
  r.tags.some(t => t.key === "Environment" && t.value === "Production")
);

// Extract specific properties
const instanceDetails = resources
  .filter(r => r.resourceType === "AWS::EC2::Instance")
  .map(r => ({
    name: r.displayName,
    type: r.properties.InstanceType,
    ip: r.properties.PublicIpAddress,
    account: r.accountId
  }));
```

**Edge cases:**
- Returns empty array `[]` if no resources found
- Partial results returned if some queries fail (errors logged to console)
- Global services (IAM, S3, Route53) are queried from us-east-1 only, regardless of regions parameter
- Resource properties vary by resource type - check AWS API documentation

**Error handling:**
```javascript
const resources = queryResources({
  accounts: null,
  regions: null,
  resourceTypes: ["AWS::EC2::Instance"]
});

if (resources.length === 0) {
  console.error("No EC2 instances found");
  return null;
}

// Safe property access with defaults
resources.forEach(r => {
  const instanceType = r.properties.InstanceType || "unknown";
  const status = r.status || "unknown";
  console.log(`${r.displayName}: ${instanceType} (${status})`);
});
```

**Performance considerations:**
- Queries are executed in parallel across all account/region/resource type combinations
- Large queries (many accounts × many regions × many resource types) may take 10-60 seconds
- Consider filtering in JavaScript to reduce result size
- Use specific accounts/regions when possible instead of null defaults
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

        // Verify required documentation elements
        assert!(docs.contains("queryResources()"));
        assert!(docs.contains("function queryResources("));
        assert!(docs.contains("QueryOptions"));
        assert!(docs.contains("ResourceInfo"));
        assert!(docs.contains("Return value structure:"));
        assert!(docs.contains("```json"));
        assert!(docs.contains("Example usage:"));
        assert!(docs.contains("Edge cases:"));
        assert!(docs.contains("Error handling:"));
        assert!(docs.contains("Performance considerations:"));
    }
}
