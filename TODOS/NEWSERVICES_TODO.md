# AWS Service Integration Cheatsheet

This document provides step-by-step instructions for adding new AWS services to the resource explorer application, based on lessons learned from implementing S3, CloudFormation, RDS, and additional EC2 resources.

## Prerequisites

1. **Add AWS SDK Dependency**: Update `Cargo.toml` with the service SDK
   ```toml
   aws-sdk-servicename = "1.67"  # Use same version as other SDKs
   ```

2. **Check AWS SDK Documentation**: Verify the service has list/describe APIs
   - Most services follow `list_*` and `describe_*` patterns
   - Check for pagination support (`into_paginator()`)
   - Note which fields require manual JSON conversion vs automatic serde

## Step-by-Step Integration Process

### 1. Create Service Implementation (`aws_services/servicename.rs`)

**Template Structure:**
```rust
use anyhow::{Result, Context};
use aws_sdk_servicename as servicename;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct ServiceNameService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ServiceNameService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List primary resources (basic list data)
    pub async fn list_resources(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = servicename::Client::new(&aws_config);
        // Use appropriate list method (list_*, describe_* with empty filters)
        let response = client.list_resources().send().await?;

        let mut resources = Vec::new();
        if let Some(resource_list) = response.resources {
            for resource in resource_list {
                let resource_json = self.resource_to_json(&resource);
                resources.push(resource_json);
            }
        }

        Ok(resources)
    }

    /// Get detailed information for specific resource (for describe functionality)
    pub async fn describe_resource(
        &self,
        account_id: &str,
        region: &str,
        resource_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = servicename::Client::new(&aws_config);
        let response = client
            .describe_resource()
            .resource_id(resource_id)
            .send()
            .await?;

        if let Some(resource_details) = response.resource {
            Ok(self.resource_details_to_json(&resource_details))
        } else {
            Err(anyhow::anyhow!("Resource {} not found", resource_id))
        }
    }

    // JSON conversion methods - CRITICAL: Avoid serde_json::to_value for AWS SDK types
    fn resource_to_json(&self, resource: &servicename::types::Resource) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        // Always include the primary identifier
        if let Some(id) = &resource.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            // Also map to common field names for normalizer
            json.insert("ResourceId".to_string(), serde_json::Value::String(id.clone()));
        }
        
        // Add name/display fields
        if let Some(name) = &resource.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        // Add status fields
        if let Some(status) = &resource.status {
            json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
        }

        // Handle Option<Vec<T>> fields safely
        if let Some(tags) = &resource.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                        }
                        if let Some(value) = &tag.value {
                            tag_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                        }
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        // For complex types that don't implement Serialize:
        // EITHER: Convert manually field by field (recommended)
        // OR: Comment out with TODO for manual conversion later

        serde_json::Value::Object(json)
    }
}
```

**⚠️ Common Pitfalls:**
- **AWS SDK Serialization**: AWS SDK types often don't implement `Serialize`. Use manual JSON conversion
- **Option Handling**: Always check `if !list.is_empty()` before processing `Option<Vec<T>>`
- **Enum Conversion**: Use `.as_str()` for AWS SDK enums, then `.to_string()`
- **Pagination**: Use `.into_paginator().send()` for large result sets

### 2. Update Service Module Exports (`aws_services/mod.rs`)

```rust
pub mod servicename;
pub use servicename::ServiceNameService;
```

### 3. Add Service to Client (`aws_client.rs`)

**Add field to struct:**
```rust
pub struct AWSResourceClient {
    // ... existing services
    servicename_service: ServiceNameService,
}
```

**Update constructor:**
```rust
impl AWSResourceClient {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            // ... existing services
            servicename_service: ServiceNameService::new(Arc::clone(&credential_coordinator)),
            // ... rest
        }
    }
}
```

**Add to query routing:**
```rust
async fn query_resource_type(
    &self,
    account: &str,
    region: &str,
    resource_type: &str,
) -> Result<Vec<ResourceEntry>> {
    let raw_resources = match resource_type {
        // ... existing types
        "AWS::ServiceName::Resource" => self.servicename_service.list_resources(account, region).await?,
        _ => {
            warn!("Unsupported resource type: {}", resource_type);
            return Ok(Vec::new());
        }
    };
    // ...
}
```

### 4. Create Normalizer (`normalizers/servicename.rs`)

**Template:**
```rust
use super::*;
use super::utils::*;  // CRITICAL: Import utils functions
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for ServiceName Resources
pub struct ServiceNameResourceNormalizer;

impl ResourceNormalizer for ServiceNameResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")  // Use your service's ID field
            .or_else(|| raw_response.get("Id"))  // Fallback options
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-resource")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::ServiceName::Resource".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            account_color: assign_account_color(account),    // CRITICAL: Use these exact functions
            region_color: assign_region_color(region),       // from state.rs
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Implement based on your service's relationships
        // Example: VPC relationships, attached resources, etc.
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ServiceName::Resource"
    }
}
```

**⚠️ Common Pitfalls:**
- **Missing Utils Import**: Must include `use super::utils::*;`
- **Wrong Field Order**: Follow the exact `ResourceEntry` field order from existing services
- **Color Functions**: Use `assign_account_color(account)` and `assign_region_color(region)` exactly
- **Resource Type**: Use consistent AWS CloudFormation naming convention

### 5. Update Normalizer Module (`normalizers/mod.rs`)

**Add exports:**
```rust
pub mod servicename;
pub use servicename::*;
```

**Update factory:**
```rust
impl NormalizerFactory {
    pub fn create_normalizer(resource_type: &str) -> Option<Box<dyn ResourceNormalizer + Send + Sync>> {
        match resource_type {
            // ... existing types
            "AWS::ServiceName::Resource" => Some(Box::new(ServiceNameResourceNormalizer)),
            _ => None,
        }
    }

    pub fn get_supported_resource_types() -> Vec<&'static str> {
        vec![
            // ... existing types
            "AWS::ServiceName::Resource",
        ]
    }
}
```

### 6. **CRITICAL: Enhanced Resource Integration**

**⚠️ MANDATORY INTEGRATION STEP** - This step is essential for UI data flow and was discovered to be missing across multiple services:

**For ALL services with enhanced describe methods, you MUST integrate them into the describe_resource routing method in `aws_client.rs`:**

```rust
// In src/app/resource_explorer/aws_client.rs, add to describe_resource() method
pub async fn describe_resource(
    &self,
    resource: &ResourceEntry,
) -> Result<serde_json::Value> {
    match resource.resource_type.as_str() {
        "AWS::S3::Bucket" => {
            self.s3_service.describe_bucket(&resource.account_id, &resource.region, &resource.resource_id).await
        }
        "AWS::RDS::DBInstance" => {
            self.rds_service.describe_db_instance(&resource.account_id, &resource.region, &resource.resource_id).await
        }
        "AWS::Lambda::Function" => {
            self.lambda_service.describe_function(&resource.account_id, &resource.region, &resource.resource_id).await
        }
        "AWS::DynamoDB::Table" => {
            self.dynamodb_service.describe_table(&resource.account_id, &resource.region, &resource.resource_id).await
        }
        "AWS::CloudFormation::Stack" => {
            self.cloudformation_service.describe_stack(&resource.account_id, &resource.region, &resource.resource_id).await
        }
        "AWS::ECS::Cluster" => {
            self.ecs_service.describe_cluster(&resource.account_id, &resource.region, &resource.resource_id).await
        }
        "AWS::EKS::Cluster" => {
            self.eks_service.describe_cluster(&resource.account_id, &resource.region, &resource.resource_id).await
        }
        // Add your new service with describe method here
        "AWS::ServiceName::Resource" => {
            self.servicename_service.describe_resource(&resource.account_id, &resource.region, &resource.resource_id).await
        }
        _ => {
            Err(anyhow::anyhow!("Describe not implemented for resource type: {}", resource.resource_type))
        }
    }
}
```

**Why This Integration Is Critical:**
- Enhanced describe methods provide detailed configuration data (encryption, policies, lifecycle rules, etc.)
- Without this integration, enhanced data never reaches the UI through the detailed_properties field
- The describe_resource method is the central routing point for detailed resource inspection
- Missing this integration means enhanced describe methods exist but are never used

**Architecture Flow for Enhanced Data:**
1. **Basic Listing**: Service list methods populate ResourceEntry with basic data
2. **User Inspection**: User selects resource for detailed view → describe_resource() called
3. **Enhanced Routing**: describe_resource() routes to service-specific describe method  
4. **Data Integration**: Enhanced data returned and populated into detailed_properties field
5. **UI Display**: UI displays enhanced configuration data to user

**Example: S3 Bucket Enhanced Data Flow**
```rust
// 1. Basic listing: list_buckets() → bucket_to_json() → ResourceEntry with basic bucket info
// 2. User clicks bucket → describe_resource() called
// 3. Routes to s3_service.describe_bucket() → gets policy, encryption, versioning, lifecycle
// 4. Enhanced data flows to detailed_properties field
// 5. UI shows bucket configuration details
```

**Integration Checklist for Every Service:**
- ✅ Implement enhanced describe methods in service (describe_*, get_*_configuration, etc.)
- ✅ Add service case to describe_resource method routing in aws_client.rs
- ✅ Verify enhanced data flows through detailed_properties to UI
- ✅ Test timeout handling and error handling for configuration API calls
- ✅ Validate JSON structure is compatible with UI display

### 7. Update UI Resource Type Selection (`dialogs.rs`)

**⚠️ CRITICAL STEP - Often Forgotten!**

After implementing backend support, you MUST update the UI to expose the new resource types to users.

**Update `get_default_resource_types()` function:**
```rust
pub fn get_default_resource_types() -> Vec<ResourceTypeSelection> {
    vec![
        // ... existing resource types
        ResourceTypeSelection::new(
            "AWS::ServiceName::Resource".to_string(),
            "Service Display Name".to_string(),    // User-friendly name
            "ServiceName".to_string(),             // Service category
        ),
    ]
}
```

**Location**: `src/app/resource_explorer/dialogs.rs` around line 564

**Guidelines for display names:**
- Use clear, descriptive names (e.g., "RDS DB Instance", "EBS Volume")
- Follow existing naming patterns in the list
- Group by service using comments for organization
- Use service abbreviations consistently (EC2, RDS, S3, etc.)

**Common Mistake**: Implementing full backend support but forgetting to update the UI, leaving users unable to select the new resource types in the AWS Explorer.

## Testing & Validation Checklist

### Compilation Fixes
1. **Run `cargo build`** after each major step
2. **Fix imports**: Most errors are missing `use super::utils::*;`
3. **Fix field order**: ResourceEntry fields must match existing pattern exactly
4. **Fix color functions**: Use exact function names from state.rs

### Common Compilation Errors & Solutions

| Error | Solution |
|-------|----------|
| `cannot find function 'extract_display_name'` | Add `use super::utils::*;` to normalizer |
| `the trait bound 'AwsSdkType: Serialize' is not satisfied` | Manual JSON conversion instead of `serde_json::to_value()` |
| `expected struct 'Color32', found enum 'Option<_>'` | Use `assign_account_color(account)` not `None` |
| `mismatched types` in ResourceEntry | Check field order matches existing pattern |

### Runtime Testing
1. **Verify service appears** in resource type selection
2. **Test basic listing** works without errors
3. **Check resource details** expand properly
4. **Verify relationships** display correctly

## Performance Considerations

1. **Pagination**: Always use `.into_paginator()` for services that support it
2. **Filtering**: Use service-specific filters to reduce API calls (e.g., owner filters)
3. **Caching**: Rely on existing cache infrastructure in `aws_client.rs`
4. **Rate Limiting**: AWS SDK handles this automatically

## Service-Specific Notes

### S3 Buckets
- **Global Resource**: S3 buckets are global but tracked per-region for UI
- **Permissions**: Bucket operations may require specific IAM permissions
- **Complex Properties**: Bucket policies, encryption, etc. need manual conversion

### CloudFormation Stacks
- **Rich Metadata**: Stacks have extensive metadata (parameters, outputs, drift)
- **Nested Resources**: Can extract resource relationships from stack resources
- **Status Handling**: Multiple status types (stack status, drift status)

### RDS Resources
- **Multiple Types**: DB Instances, Clusters, Snapshots each need separate normalizers
- **Security Groups**: Complex VPC security group relationships
- **Multi-AZ**: Special handling for multi-availability-zone configurations

### EC2 Extensions
- **Relationships**: Rich relationship data (volume attachments, VPC memberships)
- **State Handling**: Multiple state fields per resource type
- **Filtering**: Use owner filters to avoid listing public resources

## Troubleshooting Guide

### "Resource not found" errors
- Check IAM permissions for the service
- Verify region support for the service
- Check if service requires special opt-in

### "Serialization failed" errors  
- Use manual JSON conversion for AWS SDK types
- Check for circular references in complex types
- Consider commenting out problematic fields with TODO

### "Widget ID conflicts"
- Ensure resource IDs are unique across services
- Use account:region:resource_id format for uniqueness

## Critical Lessons Learned from Implementation

### AWS SDK Field Access Patterns (MUST FOLLOW)

**Based on extensive debugging, these are the correct patterns for AWS SDK field access:**

#### Option<T> Fields - Check Before Access
```rust
// CORRECT - Always check Option fields
if let Some(field_value) = &aws_object.optional_field {
    json.insert("Field".to_string(), serde_json::Value::String(field_value.clone()));
}

// WRONG - Will cause compilation errors
json.insert("Field".to_string(), serde_json::Value::String(aws_object.optional_field.clone()));
```

#### Boolean Fields - Use unwrap_or()
```rust
// CORRECT - AWS SDK boolean fields are Option<bool>
json.insert("Enabled".to_string(), serde_json::Value::Bool(response.enabled.unwrap_or(false)));

// WRONG - Treating as direct bool
json.insert("Enabled".to_string(), serde_json::Value::Bool(response.enabled));
```

#### DateTime Fields - Check Option First
```rust
// CORRECT - AWS SDK DateTime fields are Option<DateTime>
if let Some(creation_time) = response.creation_time {
    json.insert("CreationTime".to_string(), serde_json::Value::String(creation_time.to_string()));
}

// WRONG - Direct access without Option check
json.insert("CreationTime".to_string(), serde_json::Value::String(response.creation_time.to_string()));
```

#### Enum Fields - Use as_str() then to_string()
```rust
// CORRECT - AWS SDK enums need as_str() conversion
if let Some(status) = &response.status {
    json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
}

// WRONG - Direct enum to string
json.insert("Status".to_string(), serde_json::Value::String(response.status.to_string()));
```

#### Float/Number Conversions - Use serde_json::Number::from_f64()
```rust
// CORRECT - Safe float to JSON number conversion
if let Some(weight) = variant.current_weight {
    if let Some(weight_num) = serde_json::Number::from_f64(weight as f64) {
        json.insert("CurrentWeight".to_string(), serde_json::Value::Number(weight_num));
    }
}

// WRONG - Direct f32/f64 to Number (will fail)
json.insert("Weight".to_string(), serde_json::Value::Number(weight.into()));
```

#### Vec<T> Fields - Always Check is_empty()
```rust
// CORRECT - Check if vector has items before processing
if let Some(items) = &response.items {
    if !items.is_empty() {
        let items_json: Vec<serde_json::Value> = items.iter().map(|item| {
            // Convert each item
        }).collect();
        json.insert("Items".to_string(), serde_json::Value::Array(items_json));
    }
}

// WRONG - Processing empty vectors or not checking Option
let items_json: Vec<serde_json::Value> = response.items.iter().map(|item| {
    // This will fail if items is None
}).collect();
```

### Pagination Patterns

#### Standard Paginator Pattern
```rust
// CORRECT - Most AWS services support this pattern
let mut paginator = client
    .list_resources()
    .into_paginator()
    .send();

let mut resources = Vec::new();
while let Some(page) = paginator.next().await {
    let page = page?;
    if let Some(resource_list) = page.resources {
        for resource in resource_list {
            resources.push(self.resource_to_json(&resource));
        }
    }
}
```

#### Manual Token Pagination (for services without paginator)
```rust
// CORRECT - For services like Kinesis that don't have into_paginator()
let mut next_token: Option<String> = None;

loop {
    let mut request = client.list_streams();
    if let Some(token) = &next_token {
        request = request.next_token(token);
    }
    
    let response = request.send().await?;
    
    // Process response.items...
    
    if response.has_more_items && response.next_token.is_some() {
        next_token = response.next_token;
    } else {
        break;
    }
}
```

### Describe API Patterns

#### Internal Helper Method Pattern
```rust
// CORRECT - Use internal helper for both list detail enrichment and direct describe
async fn describe_resource_internal(
    &self,
    client: &servicename::Client,
    resource_id: &str,
) -> Result<serde_json::Value> {
    let response = client
        .describe_resource()
        .resource_id(resource_id)
        .send()
        .await?;

    if let Some(resource) = response.resource {
        Ok(self.resource_details_to_json(&resource))
    } else {
        Err(anyhow::anyhow!("Resource {} not found", resource_id))
    }
}

// Then use in both list and describe public methods
pub async fn describe_resource(&self, account_id: &str, region: &str, resource_id: &str) -> Result<serde_json::Value> {
    let aws_config = self.credential_coordinator
        .create_aws_config_for_account(account_id, region)
        .await?;
    let client = servicename::Client::new(&aws_config);
    self.describe_resource_internal(&client, resource_id).await
}
```

### Common Clippy Warnings to Avoid

#### Collapsible If Statements
```rust
// WRONG - Clippy warning: collapsible_if
if condition1 {
    if condition2 {
        do_something();
    }
}

// CORRECT - Combine conditions
if condition1 && condition2 {
    do_something();
}
```

#### Needless Borrows
```rust
// WRONG - Unnecessary &
some_function(&variable)

// CORRECT - Direct reference if function takes &T
some_function(variable)
```

#### Using last() instead of next_back()
```rust
// WRONG - Using last() on DoubleEndedIterator
let last_item = vector.iter().last();

// CORRECT - Use next_back() for DoubleEndedIterator
let last_item = vector.iter().next_back();
```

#### Length Comparison vs is_empty()
```rust
// WRONG - Clippy warning about length comparison
if vector.len() == 0 {
    // handle empty
}

// CORRECT - Use is_empty()
if vector.is_empty() {
    // handle empty
}
```

### Essential Implementation Checklist

**Before Starting a New Service:**
1. ✅ Check AWS SDK documentation for field types (Option<T> vs T)
2. ✅ Verify pagination support (into_paginator() vs manual tokens)
3. ✅ Check if describe APIs exist and their response structure
4. ✅ Plan JSON conversion for complex/nested types

**During Implementation:**
1. ✅ Always use manual JSON conversion, never serde_json::to_value() on AWS types
2. ✅ Handle all Option fields with proper checking
3. ✅ Use unwrap_or() for boolean fields
4. ✅ Convert enums with as_str().to_string()
5. ✅ Check Vec fields with is_empty() before processing

**After Implementation:**
1. ✅ Run cargo build after each service
2. ✅ Fix compilation errors immediately before moving to next service
3. ✅ Run clippy and fix warnings
4. ✅ Update UI dialogs.rs to expose new resource types

**Testing Pattern:**
- Test with minimal permissions first
- Verify empty responses don't crash
- Check edge cases (no tags, missing optional fields)
- Validate JSON structure matches normalizer expectations

## Future Enhancements

1. **Detailed Properties**: Implement describe functionality for rich resource details
2. **Cross-Service Relationships**: Link resources across different AWS services
3. **Real-time Updates**: Add CloudWatch Events integration for live updates
4. **Cost Data**: Integrate AWS Cost Explorer for resource cost information

---

**Pro Tip**: Always start with the Bedrock service implementation as a reference - it follows all the correct patterns and has been thoroughly tested.