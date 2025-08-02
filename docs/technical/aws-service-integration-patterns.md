# AWS Service Integration Patterns

Standardized integration patterns for AWS services providing consistent client interfaces, error handling, and data transformation across 45+ integrated services with parallel processing capabilities.

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

**Architectural Decisions:**
- **Lazy Loading**: Services created only when needed to minimize resource usage
- **Consistent Interface**: All services follow same method signature patterns
- **JSON Serialization**: Consistent data format across all AWS service responses
- **Error Context**: Rich error information for debugging and user feedback
- **Credential Isolation**: Each service call uses appropriate account credentials

**References:**
- [Resource Explorer System](resource-explorer-system.md) - Service integration usage
- [Credential Management](credential-management.md) - Account credential handling
- [Resource Normalizers](resource-normalizers.md) - Data transformation integration