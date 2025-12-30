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

### ⚠️ SDK Drift & Integration Pitfalls (Read First)

1. **Verify SDK Struct Fields**: Before coding, open the Rust SDK type definitions in
   `~/.cargo/registry/src/index.crates.io-*/aws-sdk-{service}-*/src/types/_*.rs`
   to confirm field names and availability. Fields like `last_update_time`, `*_arn`,
   or `filter_arn` may not exist in the SDK version being used.

2. **Paginator Availability Varies**: Not all operations support `into_paginator()`.
   If the method is missing, use a `next_token` loop or a single `send()` call.

3. **Avoid Capturing `self` in Threads**: `std::thread::spawn` requires `'static`.
   Pass only `Arc`/owned data into the closure or use a static helper.

4. **Check SDK Deprecations**: Run `cargo build` to surface deprecated fields,
   and verify in SDK type definitions (`#[deprecated]` markers) before wiring fields.

5. **Add Short Tags for New Resource Types**: Update
   `src/app/resource_explorer/tree.rs` `resource_type_to_short_tag()` so new resource
   types show meaningful badges (avoid falling back to `RESOURCE`).

3. **Determine Tag Fetching Method**: Identify how tags are fetched for this service (see Tag Implementation section below)

## Tag Implementation Guide

### Overview: How AWS Services Handle Tags

AWS services use **three different approaches** for tag fetching:

1. **Service-Specific Tag Methods** (13 services) - Have dedicated tag APIs like `list_tags_for_resource()`
2. **Resource Groups Tagging API** (67 services) - Use generic `get_resources()` with ARNs
3. **Embedded Tags** (rare) - Tags included in describe/list responses

**Reference Document**: See `AWS_SDK_TAG_METHODS_REFERENCE.md` for complete list of all services and their tag methods.

### Step 1: Identify Tag Method by Examining SDK Source

The **ONLY reliable way** to determine the tag method is to examine the actual AWS Rust SDK source code.

**Find the SDK source directory:**
```bash
# SDKs are located in cargo registry
ls ~/.cargo/registry/src/index.crates.io-*/aws-sdk-{service}-*/src/operation/
```

**Look for tag-related operations:**
```bash
# Search for tag operations in the service SDK
ls ~/.cargo/registry/src/index.crates.io-*/aws-sdk-{service}-*/src/operation/ | grep -i tag
```

**Common tag operation names:**
- `list_tags_for_resource` - Most ARN-based services
- `list_tags` - Lambda-style (single operation name)
- `list_tags_of_resource` - DynamoDB-style
- `describe_tags` - EC2-style (with filters)
- `get_bucket_tagging` - S3-style (service-specific)
- `list_queue_tags` - SQS-style (service-specific)
- `list_user_tags`, `list_role_tags`, `list_policy_tags` - IAM-style (resource-specific)

**Example: Examining EC2 tags**
```bash
$ ls ~/.cargo/registry/src/index.crates.io-*/aws-sdk-ec2-*/src/operation/ | grep tag
create_tags
delete_tags
describe_tags  # <-- This is the one we need for fetching
```

### Step 2: Examine Input Parameters

Once you find the tag operation, examine its input file to see what parameters it requires:

```bash
# Read the input file to see parameters
cat ~/.cargo/registry/src/index.crates.io-*/aws-sdk-{service}-*/src/operation/{operation}/_*_input.rs
```

**What to look for:**
- Parameter names (`resource_arn`, `key_id`, `bucket`, `queue_url`, etc.)
- Parameter types (`String` vs `Option<String>`)
- Required vs optional parameters
- Special formats (ARN, URL, ID, name)

**Example: S3 get_bucket_tagging parameters**
```rust
// From aws-sdk-s3/src/operation/get_bucket_tagging/_get_bucket_tagging_input.rs
pub struct GetBucketTaggingInput {
    pub bucket: String,  // <-- Uses bucket NAME, not ARN
    pub expected_bucket_owner: Option<String>,
}
```

**Example: EC2 describe_tags parameters**
```rust
// From aws-sdk-ec2/src/operation/describe_tags/_describe_tags_input.rs
pub struct DescribeTagsInput {
    pub filters: Option<Vec<Filter>>,  // <-- Uses filters with resource-id
    pub max_results: Option<i32>,
    pub next_token: Option<String>,
}
```

### Step 3: Categorize Your Service

Based on what you found in the SDK, categorize the service:

#### Category A: Service-Specific ARN-Based (Most Common)

**Characteristics:**
- Has `list_tags_for_resource()` or similar operation
- Takes `resource_arn` parameter
- Returns tags directly

**Example Services**: ECS, EKS, SNS, KMS, CloudFront, Lambda, RDS, DynamoDB

**Implementation Pattern:**
```rust
// In resource_tagging.rs
pub async fn get_{service}_tags(
    &self,
    account_id: &str,
    region: &str,
    resource_arn: &str,
) -> Result<Vec<ResourceTag>> {
    let aws_config = self.credential_coordinator
        .create_aws_config_for_account(account_id, region)
        .await?;

    let client = {service}::Client::new(&aws_config);

    let response = client
        .list_tags_for_resource()  // Check SDK for actual method name
        .resource_arn(resource_arn)
        .send()
        .await
        .context("Failed to fetch {service} tags")?;

    let tags: Vec<ResourceTag> = response
        .tags
        .unwrap_or_default()
        .into_iter()
        .map(|tag| ResourceTag {
            key: tag.key,
            value: tag.value,
        })
        .collect();

    Ok(tags)
}
```

**Then add to aws_client.rs:**
```rust
match resource_type {
    "AWS::Service::Resource" => {
        tagging_service.get_{service}_tags(account, region, resource_id).await?
    }
    // ...
}
```

#### Category B: Special Identifier Format (Non-ARN)

**Characteristics:**
- Uses identifiers OTHER than ARNs (bucket names, queue URLs, key IDs, resource names)
- Requires construction or extraction of identifier

**Example Services:**
- **S3**: Uses bucket name (string)
- **SQS**: Uses queue URL (constructed)
- **IAM**: Uses resource names (user name, role name) except Policy (ARN)
- **EC2**: Uses resource IDs with filters

**S3 Pattern (bucket name):**
```rust
pub async fn get_s3_bucket_tags(
    &self,
    account_id: &str,
    region: &str,
    bucket_name: &str,  // <-- Just the name, not ARN
) -> Result<Vec<ResourceTag>> {
    let client = s3::Client::new(&aws_config);

    let response = client
        .get_bucket_tagging()  // <-- Service-specific method name
        .bucket(bucket_name)
        .send()
        .await?;

    // Handle response...
}
```

**SQS Pattern (queue URL construction):**
```rust
pub async fn get_sqs_queue_tags(
    &self,
    account_id: &str,
    region: &str,
    queue_name: &str,
) -> Result<Vec<ResourceTag>> {
    let client = sqs::Client::new(&aws_config);

    // Construct queue URL from components
    let queue_url = format!(
        "https://sqs.{}.amazonaws.com/{}/{}",
        region, account_id, queue_name
    );

    let response = client
        .list_queue_tags()
        .queue_url(&queue_url)  // <-- Uses URL, not ARN
        .send()
        .await?;

    // Handle response...
}
```

**EC2 Pattern (resource ID with filters):**
```rust
pub async fn get_ec2_tags(
    &self,
    account_id: &str,
    region: &str,
    resource_id: &str,  // <-- EC2 resource ID (i-*, vol-*, vpc-*, etc.)
) -> Result<Vec<ResourceTag>> {
    let client = ec2::Client::new(&aws_config);

    let response = client
        .describe_tags()
        .filters(
            ec2::types::Filter::builder()
                .name("resource-id")
                .values(resource_id)
                .build(),
        )
        .send()
        .await?;

    // Handle response...
}
```

**IAM Pattern (resource-specific methods):**
```rust
// Users and Roles use names
pub async fn get_iam_user_tags(&self, account_id: &str, _region: &str, user_name: &str) -> Result<Vec<ResourceTag>> {
    let client = iam::Client::new(&aws_config);
    client.list_user_tags().user_name(user_name).send().await?
}

// Policies use ARN (exception!)
pub async fn get_iam_policy_tags(&self, account_id: &str, _region: &str, policy_arn: &str) -> Result<Vec<ResourceTag>> {
    let client = iam::Client::new(&aws_config);
    client.list_policy_tags().policy_arn(policy_arn).send().await?
}
```

#### Category C: Resource Groups Tagging API (Default)

**Characteristics:**
- No service-specific tag operation found in SDK
- Service resources use standard ARN format
- Falls back to generic tagging API

**Example Services**: 67 services including ACM, Amplify, AppRunner, Backup, CodeBuild, etc.

**Implementation:**
Already handled by default in `aws_client.rs`:
```rust
// Default case in fetch_tags_for_resource()
_ => {
    if resource_id.starts_with("arn:") {
        tagging_service.get_tags_for_arn(account, region, resource_id).await?
    } else {
        tracing::warn!(
            "Cannot fetch tags for {}: {} - not an ARN and no service-specific implementation",
            resource_type, resource_id
        );
        Vec::new()
    }
}
```

**No additional code needed** - it just works if your resource uses standard ARN format!

#### Category D: Embedded Tags (Rare)

**Characteristics:**
- Tags are included in the list/describe response
- No separate tag API call needed
- More efficient (one less API call)

**Example Services**: CloudFormation Stacks

**Implementation:**
Tags are already in the JSON from list/describe operations:
```rust
// In service implementation
fn stack_to_json(&self, stack: &cloudformation::types::Stack) -> serde_json::Value {
    let mut json = serde_json::Map::new();

    // Tags are directly in the Stack object
    if let Some(tags) = &stack.tags {
        let tags_json: Vec<serde_json::Value> = tags
            .iter()
            .map(|tag| {
                let mut tag_json = serde_json::Map::new();
                tag_json.insert("Key".to_string(), json!(tag.key));
                tag_json.insert("Value".to_string(), json!(tag.value));
                serde_json::Value::Object(tag_json)
            })
            .collect();
        json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
    }

    serde_json::Value::Object(json)
}
```

**No additional tag fetching code needed** - normalizer uses `extract_tags(&raw_response)` utility.

### Step 4: Implementation Checklist

**For Service-Specific Methods (Categories A, B, D):**

1. **Add SDK import** to `resource_tagging.rs`:
   ```rust
   use aws_sdk_{service} as {service};
   ```

2. **Implement tag method** in `resource_tagging.rs`:
   ```rust
   pub async fn get_{service}_tags(...) -> Result<Vec<ResourceTag>> { ... }
   ```

3. **Add routing case** in `aws_client.rs` > `fetch_tags_for_resource()`:
   ```rust
   "AWS::Service::Resource" => {
       tagging_service.get_{service}_tags(account, region, resource_id).await?
   }
   ```

4. **Test compilation**:
   ```bash
   cargo build
   ```

**For Resource Groups Tagging API (Category C):**

1. **Verify resource uses ARN format** in normalizer
2. **No code needed** - default implementation handles it
3. **Test with real resources** to confirm tags appear

### Common Pitfalls

1. **Wrong Parameter Order**:
   - ❌ `get_tags(resource_id, account, region)`
   - ✅ `get_tags(account, region, resource_id)`
   - All service methods follow: account → region → resource identifier

2. **Assuming ARN When Not**:
   - Check SDK input parameters
   - S3 uses bucket name, not ARN
   - SQS uses queue URL, not ARN
   - IAM users/roles use names, not ARNs

3. **Not Checking Response Structure**:
   - Read SDK response struct to see tag field names
   - Some use `tags`, others `tag_set`, `tag_list`, etc.
   - Some wrap in extra objects (CloudFront uses `tags.items`)

4. **Forgetting Global Services**:
   - IAM, CloudFront, Organizations are global
   - Use `"us-east-1"` for region parameter
   - Mark with comment `// Service is global`

### Testing Your Implementation

**Manual SDK Verification:**
```bash
# 1. Find the service SDK directory
find ~/.cargo/registry/src -name "aws-sdk-{service}-*" -type d

# 2. Look for tag operations
ls {sdk-dir}/src/operation/ | grep -i tag

# 3. Examine the input parameters
cat {sdk-dir}/src/operation/{tag-operation}/_*_input.rs | grep "pub "

# 4. Examine the output structure
cat {sdk-dir}/src/operation/{tag-operation}/_*_output.rs | grep "pub tags"
```

**Compilation Test:**
```bash
cargo build 2>&1 | grep -i "tag\|error"
```

**Runtime Test:**
1. Create test resource with tags in AWS account
2. Query resource through Explorer
3. Verify tags appear in UI
4. Check logs for tag fetch success/errors

### Quick Reference: Service Categories

**Category A (Service-Specific ARN-Based):**
CloudFront, DynamoDB, ECS, EKS, KMS, Lambda, RDS, SNS

**Category B (Special Identifiers):**
EC2 (resource IDs), S3 (bucket name), SQS (queue URL), IAM (names/ARN)

**Category C (Resource Groups Tagging API):**
ACM, Amplify, AppRunner, AppSync, Athena, Backup, Batch, Bedrock, CodeBuild, CodeCommit, CodePipeline, Cognito, Config, Connect, DataBrew, DataSync, Detective, DocumentDB, ECR, EFS, ElastiCache, ELB, ELBv2, EMR, EventBridge, FSx, GlobalAccelerator, Glue, Greengrass, GuardDuty, Inspector, IoT, Kinesis, KinesisFirehose, LakeFormation, Lex, Logs, Macie, MQ, MSK, Neptune, OpenSearch, Organizations, Polly, QuickSight, Redshift, Rekognition, Route53, SageMaker, SecretsManager, SecurityHub, Shield, SSM, StepFunctions, Timestream, Transfer, WAFv2, WorkSpaces, XRay

**Category D (Embedded Tags):**
CloudFormation

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
        let mut relationships = Vec::new();
        
        // Example: Find resources this service relates to
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::EC2::VPC" => {
                    // Example: Service deployed in VPC
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::IAM::Role" => {
                    // Example: Service assumes IAM role
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                _ => {}
            }
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ServiceName::Resource"
    }
}
```

### **ResourceRelationship System (Updated Structure)**

**⚠️ IMPORTANT: ResourceRelationship Structure Changed**

The relationship system uses the following **current structure**:

```rust
pub struct ResourceRelationship {
    pub relationship_type: RelationshipType,     // Enum: Uses, Contains, AttachedTo
    pub target_resource_id: String,             // ID of target resource
    pub target_resource_type: String,           // Type like "AWS::EC2::VPC"
}

pub enum RelationshipType {
    Uses,            // Resource uses/depends on target (EC2 uses SecurityGroup)
    Contains,        // Resource contains target (VPC contains Subnet)
    AttachedTo,      // Resource attached to target (EBS attached to EC2)
}
```

**Examples of proper relationship creation:**
```rust
// Lambda function uses IAM role
relationships.push(ResourceRelationship {
    relationship_type: RelationshipType::Uses,
    target_resource_id: role_resource.resource_id.clone(),
    target_resource_type: "AWS::IAM::Role".to_string(),
});

// VPC contains subnet  
relationships.push(ResourceRelationship {
    relationship_type: RelationshipType::Contains,
    target_resource_id: subnet_resource.resource_id.clone(),
    target_resource_type: "AWS::EC2::Subnet".to_string(),
});

// EBS volume attached to EC2 instance
relationships.push(ResourceRelationship {
    relationship_type: RelationshipType::AttachedTo,
    target_resource_id: instance_resource.resource_id.clone(),
    target_resource_type: "AWS::EC2::Instance".to_string(),
});
```

**⚠️ Old Structure (DO NOT USE):**
```rust
// ❌ OUTDATED - Will cause compilation errors
ResourceRelationship {
    source_id: entry.resource_id.clone(),        // Field removed
    target_id: resource.resource_id.clone(),     // Field renamed
    relationship_type: "uses".to_string(),       // Now enum, not string
    description: "description".to_string(),      // Field removed
}
```

**Migration Pattern:**
- Remove `source_id` and `description` fields
- Change `target_id` → `target_resource_id`
- Add `target_resource_type` field
- Change `relationship_type` from String to `RelationshipType` enum
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

## Parent-Child (Nested) Resource Hierarchies

**⚠️ NEW FEATURE**: The Resource Explorer now supports hierarchical parent-child relationships for resources that have natural nesting (e.g., KnowledgeBase → DataSource → IngestionJob).

### When to Use Nested Resources

Use parent-child relationships when:
- Resources have a natural containment hierarchy (parent contains children)
- Child resources cannot exist without a parent
- You want children to appear nested in the tree view under their parent
- You want to automatically query children when querying parents

**Examples:**
- AWS::Bedrock::KnowledgeBase → AWS::Bedrock::DataSource → AWS::Bedrock::IngestionJob
- AWS::Bedrock::Agent → AWS::Bedrock::AgentAlias + AWS::Bedrock::AgentActionGroup
- AWS::Bedrock::Flow → AWS::Bedrock::FlowAlias
- AWS::ECS::Cluster → AWS::ECS::Service → AWS::ECS::Task (potential future implementation)
- AWS::RDS::DBCluster → AWS::RDS::DBInstance (potential future implementation)

### Implementation Steps

#### 1. Define Parent-Child Configuration

Add your hierarchy to `src/app/resource_explorer/child_resources.rs`:

```rust
impl ChildResourceConfig {
    pub fn new() -> Self {
        let mut parent_to_children = HashMap::new();

        // Define your parent-child relationship
        parent_to_children.insert(
            "AWS::Service::ParentResource".to_string(),
            vec![
                ChildResourceDef {
                    child_type: "AWS::Service::ChildResource".to_string(),
                    query_method: ChildQueryMethod::SingleParent {
                        param_name: "parent_id",  // Parameter name to pass to child query
                    },
                },
            ],
        );

        // For children that need multiple parent parameters
        parent_to_children.insert(
            "AWS::Service::ChildResource".to_string(),
            vec![
                ChildResourceDef {
                    child_type: "AWS::Service::GrandchildResource".to_string(),
                    query_method: ChildQueryMethod::MultiParent {
                        params: vec!["parent_id", "child_id"],  // Multiple parameters
                    },
                },
            ],
        );

        Self { parent_to_children }
    }
}
```

**Query Method Types:**
- `SingleParent`: Child needs only the parent's ID (e.g., list_data_sources(knowledge_base_id))
- `MultiParent`: Child needs multiple parameters from parent hierarchy (e.g., list_ingestion_jobs(knowledge_base_id, data_source_id))

#### 2. Implement Child Query Methods in Service

In your service implementation (e.g., `aws_services/yourservice.rs`), add methods to query child resources:

```rust
impl YourService {
    /// List child resources that belong to a parent
    pub async fn list_child_resources(
        &self,
        account_id: &str,
        region: &str,
        parent_id: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = yourservice::Client::new(&aws_config);

        // Call AWS API with parent ID
        let response = client
            .list_child_resources()
            .parent_id(parent_id)
            .send()
            .await?;

        let mut resources = Vec::new();
        if let Some(child_list) = response.children {
            for child in child_list {
                let child_json = self.child_to_json(&child);
                resources.push(child_json);
            }
        }

        Ok(resources)
    }
}
```

#### 3. Add Child Query Routing in AWSResourceClient

In `src/app/resource_explorer/aws_client.rs`, add your child resource types to the query routing methods.

The system provides these helper methods (already implemented):
- `query_children_recursive()` - Automatically queries children recursively up to max depth
- `query_child_with_single_parent()` - For children needing only parent_id
- `query_child_with_multi_parent()` - For children needing multiple parent parameters

**Add routing for single-parent children:**
```rust
async fn query_child_with_single_parent(
    &self,
    account: &str,
    region: &str,
    child_type: &str,
    _param_name: &str,
    parent_id: &str,
    parent: &ResourceEntry,
) -> Result<Vec<ResourceEntry>> {
    let raw_children = match child_type {
        // Add your child resource type here
        "AWS::Service::ChildResource" => {
            self.get_yourservice_service()
                .list_child_resources(account, region, parent_id)
                .await?
        }
        _ => return Ok(vec![]),
    };

    self.normalize_child_resources(
        raw_children,
        child_type,
        account,
        region,
        Some(parent.resource_id.clone()),
        Some(parent.resource_type.clone()),
    )
}
```

**Add routing for multi-parent children:**
```rust
async fn query_child_with_multi_parent(
    &self,
    account: &str,
    region: &str,
    child_type: &str,
    parent_params: &HashMap<String, String>,
    parent: &ResourceEntry,
) -> Result<Vec<ResourceEntry>> {
    let raw_children = match child_type {
        "AWS::Service::GrandchildResource" => {
            let parent_id = parent_params.get("parent_id")
                .context("Missing parent_id")?;
            let child_id = parent_params.get("child_id")
                .context("Missing child_id")?;

            self.get_yourservice_service()
                .list_grandchild_resources(account, region, parent_id, child_id)
                .await?
        }
        _ => return Ok(vec![]),
    };

    self.normalize_child_resources(
        raw_children,
        child_type,
        account,
        region,
        Some(parent.resource_id.clone()),
        Some(parent.resource_type.clone()),
    )
}
```

**Add parameter extraction logic** (for multi-parent children):
```rust
fn extract_parent_params(
    &self,
    parent: &ResourceEntry,
    _param_names: &[&str],
) -> Result<HashMap<String, String>> {
    let mut params = HashMap::new();

    // Example: Child resource needs both grandparent and parent IDs
    if parent.resource_type == "AWS::Service::ChildResource" {
        // Get grandparent ID from parent's parent_resource_id
        if let Some(grandparent_id) = &parent.parent_resource_id {
            params.insert("parent_id".to_string(), grandparent_id.clone());
        }
        // Get direct parent ID
        params.insert("child_id".to_string(), parent.resource_id.clone());
    }

    Ok(params)
}
```

#### 4. Automatic Child Resource Features

Once configured, the system automatically:
- ✅ Queries child resources recursively when parent is queried (up to depth 3)
- ✅ Marks children with `is_child_resource = true`
- ✅ Tracks parent with `parent_resource_id` and `parent_resource_type` fields
- ✅ Hides children from top-level resource list (they appear nested under parents)
- ✅ Creates bidirectional relationships (`ChildOf` / `ParentOf`)
- ✅ Displays children in tree view grouped by type under their parent
- ✅ Handles errors gracefully (child query failures don't break parent queries)

#### 5. Testing Child Hierarchies

Add unit tests to `child_resources.rs`:
```rust
#[test]
fn test_your_service_has_children() {
    let config = ChildResourceConfig::new();
    assert!(config.has_children("AWS::Service::ParentResource"));

    let children = config.get_children("AWS::Service::ParentResource").unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].child_type, "AWS::Service::ChildResource");
}
```

### Architecture Details

**Data Model Fields** (automatically handled):
- `parent_resource_id: Option<String>` - ID of parent resource
- `parent_resource_type: Option<String>` - Type of parent resource
- `is_child_resource: bool` - Flag to identify child resources

**Relationship Types**:
- `RelationshipType::ChildOf` - Added to child pointing to parent
- `RelationshipType::ParentOf` - Can be added to parent pointing to children

**Tree Display**:
- Parent resources appear in normal tree groupings
- Children appear as nested nodes under their parents
- Children are grouped by resource type if multiple types
- Grandchildren nest recursively under children

**Depth Limiting**:
- Maximum recursion depth: 3 levels
- Prevents infinite loops in misconfigured hierarchies
- Logs warning when depth limit reached

### Best Practices

1. **Define Clear Hierarchies**: Only use for resources with true containment relationships
2. **Keep Hierarchies Shallow**: 2-3 levels max for optimal UI performance
3. **Handle Missing Parents**: Children should gracefully handle missing parent data
4. **Test Recursion**: Verify depth limiting works correctly
5. **Document Relationships**: Add comments explaining why resources are hierarchical

### Reference Implementation

See `src/app/resource_explorer/child_resources.rs` for complete examples of:
- Bedrock KnowledgeBase → DataSource → IngestionJob (3-level hierarchy)
- Bedrock Agent → AgentAlias + AgentActionGroup (2-level with multiple children)
- Bedrock Flow → FlowAlias (2-level simple hierarchy)

---

## Future Enhancements

1. **Detailed Properties**: Implement describe functionality for rich resource details
2. **Cross-Service Relationships**: Link resources across different AWS services (non-hierarchical)
3. **Real-time Updates**: Add CloudWatch Events integration for live updates
4. **Cost Data**: Integrate AWS Cost Explorer for resource cost information
5. **More Hierarchies**: Extend nested resources to ECS, RDS, Organizations, etc.

---

**Pro Tip**: Always start with the Bedrock service implementation as a reference - it follows all the correct patterns including hierarchical child resources, and has been thoroughly tested.
