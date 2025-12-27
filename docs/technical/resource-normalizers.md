# Resource Normalizers

Service-specific data normalization system transforming AWS API responses into consistent ResourceEntry format for unified display and processing across 174 resource types.

## Core Functionality

**Normalization System:**
- Transforms diverse AWS service API responses into standardized ResourceEntry objects
- Consistent display name extraction from various AWS naming patterns (Name, InstanceName, RoleName, etc.)
- Status normalization across different AWS service status formats (State, Status, InstanceState)
- Tag extraction and processing with key-value pair standardization
- Resource relationship mapping for dependency visualization

**Key Features:**
- Factory pattern for creating appropriate normalizers by resource type
- 174 supported AWS resource types with specific normalization logic
- Utility functions for common normalization tasks (name/status/tag extraction)
- Property normalization for consistent field naming across services
- Account color assignment for visual resource organization

**Main Components:**
- **ResourceNormalizer Trait**: Standard interface for all service-specific normalizers
- **NormalizerFactory**: Creates appropriate normalizer instances by resource type
- **Service Normalizers**: Specialized normalizers for each AWS service (EC2, S3, RDS, etc.)
- **Utility Functions**: Common extraction logic for names, statuses, tags, and properties

**Integration Points:**
- Resource Explorer System for consistent resource display
- AWS Service Integration for API response processing
- Resource visualization system for hierarchical organization
- Search and filtering system for normalized property access

## Implementation Details

**Key Files:**
- `src/app/resource_explorer/normalizers/mod.rs` - Normalizer trait, factory, and utility functions
- `src/app/resource_explorer/normalizers/{service}.rs` - Individual service normalizers (s3.rs, ec2.rs, etc.)
- `src/app/resource_explorer/normalizers/json_expansion.rs` - Embedded JSON string detection and expansion

**ResourceNormalizer Trait:**
```rust
pub trait ResourceNormalizer {
    fn normalize(&self, raw_response: serde_json::Value, account: &str, region: &str, query_timestamp: DateTime<Utc>) -> Result<ResourceEntry>;
    fn extract_relationships(&self, entry: &ResourceEntry, all_resources: &[ResourceEntry]) -> Vec<ResourceRelationship>;
    fn resource_type(&self) -> &'static str;
}
```

**Factory Pattern Usage:**
```rust
impl NormalizerFactory {
    pub fn create_normalizer(resource_type: &str) -> Option<Box<dyn ResourceNormalizer + Send + Sync>> {
        match resource_type {
            "AWS::S3::Bucket" => Some(Box::new(S3BucketNormalizer)),
            "AWS::EC2::Instance" => Some(Box::new(EC2InstanceNormalizer)),
            // 80+ additional mappings...
        }
    }
}
```

**Common Extraction Patterns:**
- **Display Names**: Try Name → InstanceName → RoleName → Tags[Name] → fallback to resource ID
- **Status Values**: Try State → InstanceState.Name → Status → None
- **Tag Processing**: Extract Tags array with Key/Value pairs into ResourceTag structs
- **Property Normalization**: Map service-specific fields to common property names
- **JSON Expansion**: Automatically expand embedded JSON strings in policy documents

**JSON Expansion:**

AWS APIs often return policy documents as URL-encoded or stringified JSON. The `json_expansion` module detects and expands these automatically:

```rust
use crate::app::resource_explorer::normalizers::json_expansion::expand_embedded_json;

// In normalizer, after getting raw response:
let expanded = expand_embedded_json(raw_response);
```

This transforms stringified policies like `"{\"Version\":\"2012-10-17\"}"` into proper JSON objects for improved readability in the UI.

**Supported Resource Types:**
- **EC2**: 14 resource types (Instance, VPC, SecurityGroup, Volume, etc.)
- **RDS**: 5 resource types (DBInstance, DBCluster, DBSnapshot, etc.)
- **Lambda**: 3 resource types (Function, LayerVersion, EventSourceMapping)
- **IAM**: 3 resource types (Role, User, Policy)
- **Storage**: S3 Bucket, EFS FileSystem
- **Additional**: 50+ more resource types across 25+ AWS services

## Developer Notes

**Extension Points for Adding New AWS Services:**

1. **Create Service Normalizer**:
   ```rust
   // In normalizers/newservice.rs
   use super::*;
   
   pub struct NewServiceResourceNormalizer;
   
   impl ResourceNormalizer for NewServiceResourceNormalizer {
       fn normalize(&self, raw_response: serde_json::Value, account: &str, region: &str, query_timestamp: DateTime<Utc>) -> Result<ResourceEntry> {
           let resource_id = raw_response.get("ResourceId").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
           let display_name = extract_display_name(&raw_response, &resource_id);
           let status = extract_status(&raw_response);
           let tags = extract_tags(&raw_response);
           let properties = create_normalized_properties(&raw_response);
           
           Ok(ResourceEntry {
               resource_type: "AWS::NewService::Resource".to_string(),
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
               account_color: assign_account_color(account),
               query_timestamp,
           })
       }
       
       fn resource_type(&self) -> &'static str { "AWS::NewService::Resource" }
   }
   ```

2. **Register in Factory**:
   ```rust
   // In NormalizerFactory::create_normalizer()
   "AWS::NewService::Resource" => Some(Box::new(NewServiceResourceNormalizer)),
   
   // In get_supported_resource_types()
   "AWS::NewService::Resource",
   ```

3. **Add Module Export**:
   ```rust
   // In normalizers/mod.rs
   pub mod newservice;
   pub use newservice::*;
   ```

**Common Normalization Patterns:**
- Use utility functions for standard field extraction
- Handle missing or null fields gracefully with fallbacks
- Preserve original raw response for detailed property access
- Apply consistent status mapping across similar service states
- Extract relationships for resources that reference other resources

**Property Normalization Strategy:**
```rust
// Standard fields to normalize across all resources
normalized.insert("id".to_string(), resource_id);
normalized.insert("arn".to_string(), arn);
normalized.insert("created_date".to_string(), creation_timestamp);
```

**Relationship Extraction:**
- Parse ARNs and resource references in properties
- Map subnet/VPC relationships for networking resources
- Track role/policy attachments for IAM resources
- Connect load balancer/target group associations

**Performance Considerations:**
- Normalizers called once per resource during query processing
- Factory pattern provides efficient normalizer selection
- Utility functions reduce code duplication across normalizers
- Normalized properties cached for search and filtering operations

**Architectural Decisions:**
- **Trait-Based**: Enables consistent interface across all AWS services
- **Factory Pattern**: Centralizes normalizer creation and type mapping
- **Utility Functions**: Reduce code duplication for common extraction tasks
- **Property Preservation**: Maintains both normalized and raw properties
- **Flexible Relationships**: Supports complex resource interdependencies

**References:**
- [Resource Explorer System](resource-explorer-system.md) - Integration with resource discovery
- [AWS Service Integration Patterns](aws-service-integration-patterns.md) - Service integration templates