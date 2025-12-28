# AWS Service Integration Patterns

Standardized integration patterns for AWS services providing consistent client interfaces, error handling, and data transformation across 82 integrated services with parallel processing capabilities and hierarchical parent-child resource support.

## Core Functionality

**Service Integration Architecture:**
- Consistent service client pattern across all AWS services
- Lazy service instantiation for optimal memory usage and startup time
- Credential coordinator integration for multi-account access
- Standardized error handling and context propagation
- Parallel query execution with configurable concurrency limits

**Key Features:**
- Service factory pattern with on-demand client creation
- Consistent `list_resources` and `describe_resource` method signatures
- AWS SDK configuration creation per account and region
- JSON serialization of AWS responses for consistent data handling
- Progress tracking and status reporting for long-running operations

**Main Components:**
- **AWSResourceClient**: Central coordinator with lazy service creation
- **Service Classes**: Individual AWS service wrappers (EC2Service, S3Service, etc.)
- **CredentialCoordinator**: Multi-account authentication management
- **PaginationConfig**: Configurable pagination for large result sets
- **QueryProgress**: Status tracking for parallel operations

**Integration Points:**
- Resource Explorer System for resource discovery queries
- Credential Management System for secure account access
- Resource Normalizers for consistent data transformation
- Progress reporting system for UI status updates

## Implementation Details

**Key Files:**
- `src/app/resource_explorer/aws_client.rs` - Central service coordinator with lazy instantiation
- `src/app/resource_explorer/aws_services/{service}.rs` - Individual service implementations

**Service Pattern Template:**
```rust
pub struct NewService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl NewService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self { credential_coordinator }
    }
    
    pub async fn list_resources(&self, account_id: &str, region: &str) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator.create_aws_config_for_account(account_id, region).await?;
        let client = aws_sdk_newservice::Client::new(&aws_config);
        let response = client.list_resources().send().await?;
        
        // Transform response to JSON
        let resources = response.resources.unwrap_or_default()
            .into_iter()
            .map(|resource| self.resource_to_json(&resource, account_id, region))
            .collect::<Result<Vec<_>>>()?;
        
        Ok(resources)
    }
}
```

**Lazy Service Creation:**
```rust
impl AWSResourceClient {
    fn get_new_service(&self) -> NewService {
        NewService::new(Arc::clone(&self.credential_coordinator))
    }
    
    pub async fn query_resources_by_type(&self, resource_type: &str, account_id: &str, region: &str) -> Result<Vec<ResourceEntry>> {
        match resource_type {
            "AWS::NewService::Resource" => {
                let service = self.get_new_service();
                let raw_resources = service.list_resources(account_id, region).await?;
                self.normalize_resources(raw_resources, resource_type, account_id, region).await
            }
            // Additional service mappings...
        }
    }
}
```

**Parallel Processing Pattern:**
- Configurable concurrency limits (default: 20 concurrent requests)
- Semaphore-based rate limiting to prevent API throttling
- FuturesUnordered for efficient parallel execution
- Progress tracking with mpsc channels for UI updates

**Error Handling Standards:**
- Context propagation using `anyhow::Context`
- Service-specific error mapping to user-friendly messages
- Graceful degradation for partial failures in batch operations
- Retry logic for transient network failures

**Pagination Configuration:**
```rust
pub struct PaginationConfig {
    pub page_size: i32,              // 50 items per request
    pub max_items: usize,            // 1000 total items limit
    pub max_concurrent_requests: usize, // 20 concurrent requests
}
```

## Developer Notes

**Extension Points for Adding New AWS Services:**

1. **Create Service Implementation**:
   ```rust
   // In aws_services/newservice.rs
   use super::super::credentials::CredentialCoordinator;
   use anyhow::{Context, Result};
   use aws_sdk_newservice as newservice;
   use std::sync::Arc;
   
   pub struct NewService {
       credential_coordinator: Arc<CredentialCoordinator>,
   }
   
   impl NewService {
       pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
           Self { credential_coordinator }
       }
       
       pub async fn list_resources(&self, account_id: &str, region: &str) -> Result<Vec<serde_json::Value>> {
           let aws_config = self.credential_coordinator
               .create_aws_config_for_account(account_id, region)
               .await
               .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;
               
           let client = newservice::Client::new(&aws_config);
           let response = client.list_resources().send().await?;
           
           let mut resources = Vec::new();
           if let Some(resource_list) = response.resources {
               for resource in resource_list {
                   let resource_json = self.resource_to_json(&resource, account_id, region).await?;
                   resources.push(resource_json);
               }
           }
           
           Ok(resources)
       }
       
       async fn resource_to_json(&self, resource: &newservice::types::Resource, account_id: &str, region: &str) -> Result<serde_json::Value> {
           // Convert AWS types to JSON format
       }
   }
   ```

2. **Add Service to Client**:
   ```rust
   // In aws_client.rs
   fn get_new_service(&self) -> NewService {
       NewService::new(Arc::clone(&self.credential_coordinator))
   }
   
   // In query_resources_by_type method
   "AWS::NewService::Resource" => {
       let service = self.get_new_service();
       let raw_resources = service.list_resources(account_id, region).await?;
       self.normalize_resources(raw_resources, resource_type, account_id, region).await
   }
   ```

3. **Register Module**:
   ```rust
   // In aws_services/mod.rs
   pub mod newservice;
   pub use newservice::*;
   ```

**JSON Conversion Patterns:**
```rust
async fn resource_to_json(&self, resource: &ResourceType, account_id: &str, region: &str) -> Result<serde_json::Value> {
    let mut resource_json = serde_json::Map::new();
    
    // Standard fields
    resource_json.insert("ResourceId".to_string(), serde_json::Value::String(resource.id().unwrap_or("unknown").to_string()));
    resource_json.insert("Name".to_string(), serde_json::Value::String(resource.name().unwrap_or("").to_string()));
    resource_json.insert("AccountId".to_string(), serde_json::Value::String(account_id.to_string()));
    resource_json.insert("Region".to_string(), serde_json::Value::String(region.to_string()));
    
    // Service-specific fields
    if let Some(status) = resource.status() {
        resource_json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
    }
    
    Ok(serde_json::Value::Object(resource_json))
}
```

**Parallel Query Integration:**
```rust
pub async fn query_multiple_accounts_parallel(&self, accounts: Vec<String>, regions: Vec<String>, resource_types: Vec<String>) -> Result<Vec<ResourceEntry>> {
    let semaphore = Arc::new(Semaphore::new(self.pagination_config.max_concurrent_requests));
    let mut tasks = FuturesUnordered::new();
    
    for account in accounts {
        for region in &regions {
            for resource_type in &resource_types {
                let permit = semaphore.clone().acquire_owned().await?;
                let task = self.query_single_resource_type(account.clone(), region.clone(), resource_type.clone());
                tasks.push(async move {
                    let _permit = permit;  // Hold permit for duration
                    task.await
                });
            }
        }
    }
    
    let mut all_resources = Vec::new();
    while let Some(result) = tasks.next().await {
        match result {
            Ok(resources) => all_resources.extend(resources),
            Err(e) => warn!("Query failed: {}", e),
        }
    }
    
    Ok(all_resources)
}
```

**Performance Considerations:**
- Lazy service creation reduces memory usage for unused services
- Connection pooling through AWS SDK client reuse
- Configurable pagination limits prevent memory exhaustion
- Parallel execution with concurrency controls for optimal throughput

**Security Best Practices:**
- All AWS clients use credential coordinator for secure access
- Account-specific credential isolation
- No hardcoded credentials or access keys
- Proper error message sanitization

## Adding Child Resources (Parent-Child Hierarchies)

Child resources are automatically queried when their parent is discovered. They appear nested under their parent in the tree view. Child resources are queried during **Phase 1** (fast discovery), not Phase 2.

**When to use child resources:**
- Resource requires parent ID to query (e.g., `list_aliases(function_name)`)
- Resource cannot exist without parent
- Resource is logically nested under parent in AWS Console

**Step 1: Add to child_resources.rs**

```rust
// In src/app/resource_explorer/child_resources.rs

// Single parent parameter (most common)
parent_to_children.insert(
    "AWS::Lambda::Function".to_string(),
    vec![
        ChildResourceDef {
            child_type: "AWS::Lambda::Alias".to_string(),
            query_method: ChildQueryMethod::SingleParent {
                param_name: "function_name",  // Key in parent's raw_data
            },
        },
        ChildResourceDef {
            child_type: "AWS::Lambda::Version".to_string(),
            query_method: ChildQueryMethod::SingleParent {
                param_name: "function_name",
            },
        },
    ],
);

// Multiple parent parameters (for grandchildren or complex hierarchies)
parent_to_children.insert(
    "AWS::Glue::Database".to_string(),
    vec![ChildResourceDef {
        child_type: "AWS::Glue::Table".to_string(),
        query_method: ChildQueryMethod::MultiParent {
            params: vec!["catalog_id", "database_name"],
        },
    }],
);
```

**Step 2: Add query method to aws_client.rs**

```rust
// In query_child_resources() method
async fn query_child_resources(
    &self,
    parent: &ResourceEntry,
    child_config: &ChildResourceConfig,
) -> Result<Vec<ResourceEntry>> {
    // ...existing code...

    // Add match arm for new child type
    "AWS::Lambda::Alias" => {
        let function_name = parent.raw_data.get("FunctionName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing FunctionName in parent"))?;

        let service = self.get_lambda_service();
        service.list_aliases(account, region, function_name).await?
    }
}
```

**Step 3: Implement service method**

```rust
// In aws_services/lambda.rs
pub async fn list_aliases(
    &self,
    account_id: &str,
    region: &str,
    function_name: &str,
) -> Result<Vec<serde_json::Value>> {
    let aws_config = self.credential_coordinator
        .create_aws_config_for_account(account_id, region)
        .await?;
    let client = lambda::Client::new(&aws_config);

    let response = client
        .list_aliases()
        .function_name(function_name)
        .send()
        .await?;

    // Convert to JSON...
}
```

**Step 4: Add normalizer**

Create normalizer in `normalizers/lambda_alias.rs` following standard patterns.

**Key Points:**
- Child resources inherit account/region from parent
- `parent_id` field links child to parent for tree nesting
- Children are queried recursively (grandchildren supported up to depth 3)
- Errors in child queries don't fail parent query (graceful degradation)

**Architectural Decisions:**
- **Lazy Loading**: Services created only when needed to minimize resource usage
- **Consistent Interface**: All services follow same method signature patterns
- **JSON Serialization**: Consistent data format across all AWS service responses
- **Error Context**: Rich error information for debugging and user feedback
- **Credential Isolation**: Each service call uses appropriate account credentials

## Common Troubleshooting

**Compilation Errors: SDK Field Mismatches**

⚠️ **CRITICAL**: When adding a new AWS service, compilation errors in the service file do NOT mean you should disable the entire service integration!

**Symptoms:**
```
error[E0609]: no field `resource_status` on type `&ResourceSummary`
error[E0308]: mismatched types - expected `DateTime`, found `Option<_>`
```

**Root Cause:**
- SDK field names differ from documentation/assumptions
- New AWS services may have evolving SDK structures
- Field optionality doesn't match expectations

**WRONG Approach (DO NOT DO THIS):**
```rust
// ❌ DO NOT comment out the entire service!
// pub mod bedrockagentcore_control;
// fn get_bedrock_agentcore_control_service(&self) -> BedrockAgentCoreControlService { ... }
```

**CORRECT Approach:**
1. **Normalizers are independent** - They work with JSON using safe `.get()` patterns and don't depend on the service file compiling
2. **Fix the service file** - Use docs.rs to verify actual SDK field structures
3. **Keep everything enabled** - Only the service file needs fixing, not the entire integration

**Example Fix Process:**
```bash
# 1. Check docs.rs for actual SDK types
# Visit: https://docs.rs/aws-sdk-servicename/latest/

# 2. Find the actual field names
# Instead of: resource.resource_status()
# SDK has: resource.status()

# 3. Fix field access patterns
let status = resource.status()  // Not resource_status()
    .map(|s| s.as_str().to_string());

# 4. Handle optionality correctly
let created_at = resource.created_at()
    .map(|dt| dt.fmt(aws_smithy_types::date_time::Format::DateTime)
        .unwrap_or_default());
```

**Key Lesson:**
- Service file compilation errors are **isolated to that file only**
- Normalizers, routing, and UI integration are **independent** and work fine
- Fix the service file by checking actual SDK types - don't disable everything!

**References:**
- [Resource Explorer System](resource-explorer-system.md) - Service integration usage
- [Credential Management](credential-management.md) - Account credential handling
- [Resource Normalizers](resource-normalizers.md) - Data transformation integration