# Resource Explorer System

Comprehensive AWS resource discovery and visualization platform providing multi-account, multi-region resource querying across 177 resource types from 82 AWS services, with hierarchical organization, parent-child resource nesting, and real-time credential management.

## Core Functionality

**Key Features:**
- Multi-account, multi-region AWS resource querying across 177 resource types from 82 services
- Hierarchical tree organization with customizable grouping (by Account, Region, or Resource Type)
- Parent-child resource nesting with automatic recursive querying (5 child resource types)
- Real-time credential management with session caching and automatic renewal
- Fuzzy search and filtering capabilities for large resource inventories
- Tag-based filtering (show only tagged, show only untagged resources)
- Color-coded visual organization for accounts and regions with reduced brightness
- Resource relationship mapping and detailed property inspection
- Session-based query caching with staleness detection (15-minute default threshold)

**Main Components:**
- **ResourceExplorer**: Main interface coordinating state management and UI rendering
- **ResourceExplorerState**: Core state container managing resources, queries, and UI state
- **AWSResourceClient**: Orchestrates parallel queries across all supported AWS services with two-phase loading
- **CredentialCoordinator**: Manages AWS credentials for hundreds of accounts via Identity Center
- **TreeBuilder/TreeRenderer**: Hierarchical visualization system with stable node IDs
- **NormalizerFactory**: Standardizes AWS API responses into consistent ResourceEntry format

## Two-Phase Loading Architecture

Resource Explorer uses a two-phase loading pattern to provide fast initial results while fetching detailed security and compliance information in the background.

**Phase 1 (Fast Discovery):**
- Executes list/describe operations to discover resources quickly
- Returns basic resource metadata (name, ARN, type, region, account)
- Populates the resource tree immediately for user interaction
- Typically completes in seconds

**Phase 2 (Background Enrichment):**
- Runs automatically after Phase 1 completes
- Fetches detailed security-relevant properties per resource
- Updates resources in-place via cache without blocking the UI
- Progress displayed in status bar

**Supported Resource Types for Phase 2 Enrichment:**

| Category | Resource Types |
|----------|----------------|
| Compute | Lambda::Function, ECS::Cluster, ECS::Service, EMR::Cluster |
| Security | IAM::Role, IAM::User, IAM::Policy, KMS::Key |
| Storage | S3::Bucket, DynamoDB::Table, Backup::BackupPlan, Backup::BackupVault |
| Messaging | SQS::Queue, SNS::Topic, Events::EventBus |
| Identity | Cognito::UserPool, Cognito::IdentityPool |
| Infrastructure | CloudFormation::Stack, ElasticLoadBalancingV2::LoadBalancer |
| Data/Analytics | Glue::Job, OpenSearchService::Domain, Redshift::Cluster |
| Developer Tools | CodeCommit::Repository |
| Orchestration | StepFunctions::StateMachine |

**Implementation Pattern:**

```rust
// Phase 1: Fast listing with basic info
pub async fn list_resources(
    &self,
    account_id: &str,
    region: &str,
    include_details: bool,  // false for Phase 1
) -> Result<Vec<serde_json::Value>> {
    // Returns quickly with basic resource data
}

// Phase 2: Detailed security information
pub async fn get_security_details(
    &self,
    account_id: &str,
    region: &str,
    resource_id: &str,
) -> Result<serde_json::Value> {
    // Fetches policies, configurations, encryption settings, etc.
}
```

**Progress Tracking:**

Phase 2 enrichment reports progress via `QueryProgress` with dedicated status values:
- `EnrichmentStarted`: Background enrichment has begun
- `EnrichmentInProgress`: Updates every 10 resources with count
- `EnrichmentCompleted`: All resources enriched

The UI automatically refreshes when `phase2_enrichment_completed` flag is set in state.

## Global Services

Some AWS services are global and return the same resources regardless of which region you query. The Resource Explorer handles these automatically.

**Global Resource Types:**
- `AWS::S3::Bucket` - The `list-buckets` API returns all buckets in the account, regardless of region
- `AWS::IAM::Role`, `AWS::IAM::User`, `AWS::IAM::Policy` - IAM is a global service
- `AWS::Route53::HostedZone` - Route53 DNS is global
- `AWS::CloudFront::Distribution` - CloudFront CDN is global
- `AWS::Organizations::*` - Organizations is global

**How It Works:**

The `GlobalServiceRegistry` in `global_services.rs` tracks which resource types are global. When querying:
1. The system checks if a resource type is in the global registry
2. For global services, it queries only once per account (not per region)
3. Results are cached with a `Global` region indicator
4. The region parameter in queries has no filtering effect for global services

**Implementation:**

```rust
// Check if a resource type is global
let registry = GlobalServiceRegistry::new();
if registry.is_global("AWS::S3::Bucket") {
    // Query once per account, not per region
}

// Get the default query region for global services
let query_region = registry.get_query_region(); // Returns "us-east-1"
```

**Key Files:**
- `src/app/resource_explorer/global_services.rs` - Global service registry

**Integration Points:**
- AWS Identity Center for live credential management and multi-account access
- Window Focus System for keyboard navigation integration
- Color System for consistent visual coding across the application
- Agent for AI-powered resource operations

## Implementation Details

**Key Files:**
- `src/app/resource_explorer/mod.rs` - Main ResourceExplorer struct and module interface
- `src/app/resource_explorer/state.rs` - State management with ResourceEntry and caching
- `src/app/resource_explorer/aws_client.rs` - Service coordination and parallel query execution
- `src/app/resource_explorer/window.rs` - UI rendering and user interaction handling
- `src/app/resource_explorer/credentials.rs` - Multi-account credential management
- `src/app/resource_explorer/tree.rs` - Hierarchical resource organization
- `src/app/resource_explorer/status.rs` - Thread-safe status messaging for async operation progress
- `src/app/resource_explorer/aws_services/` - 89 AWS service modules (EC2, IAM, S3, Lambda, Bedrock, etc.)
- `src/app/resource_explorer/child_resources.rs` - Parent-child resource hierarchy configuration
- `src/app/resource_explorer/normalizers/` - Resource data transformation modules
- `src/app/resource_explorer/normalizers/json_expansion.rs` - Embedded JSON detection and expansion

**Important Patterns:**
- **Service Integration**: Each AWS service follows consistent pattern with service module and normalizer
- **Parallel Queries**: Uses `FuturesUnordered` with semaphore-based concurrency control (20 concurrent requests)
- **Credential Management**: Session-based caching with 5-minute expiration buffer and automatic refresh
- **State Management**: Arc<RwLock<ResourceExplorerState>> for thread-safe state sharing

**Configuration Requirements:**
- AWS Identity Center must be configured for multi-account access
- Default role name (typically "awsdash") for credential assumption
- Memory limits require `-j 7` flag for testing due to large concurrent operations

**Filtering System:**

The Explorer provides multiple filtering mechanisms in the sidebar:

- **Tag Presence Filters**: Mutually exclusive checkboxes for tag-based filtering
  - "Show only tagged": Displays only resources that have at least one tag
  - "Show only untagged": Displays only resources with no tags
  - Both unchecked: Shows all resources regardless of tag presence

- **Search Filter**: Fuzzy text matching across resource names and identifiers

- **Account/Region Selection**: Checkboxes to filter by specific accounts or regions

- **Resource Type Selection**: Checkboxes to filter by specific AWS resource types

Tag filters are mutually exclusive (selecting one unchecks the other) and work in combination with other filter types to provide precise resource discovery.

**Color System:**

Resource colors use reduced brightness to improve readability:

- **Lambda Functions**: Light luminosity (Luminosity::Light) instead of alternating Bright/Light
- **Account Colors**: Deterministic hashing ensures consistent colors across sessions
- **Region Colors**: Visual distinction in hierarchical tree views

The Light luminosity setting prevents overly bright colors (particularly pink for Lambda functions) while maintaining visual distinction between resources.

## Developer Notes

**Extension Points for Adding New Services:**

1. **Create Service Module** in `aws_services/newservice.rs`:
   ```rust
   pub struct NewService {
       credential_coordinator: Arc<CredentialCoordinator>,
   }

   impl NewService {
       /// Phase 1: Fast listing with basic metadata
       pub async fn list_resources(
           &self,
           account_id: &str,
           region: &str,
           include_details: bool,  // false for Phase 1, true for inline details
       ) -> Result<Vec<serde_json::Value>> {
           // Implement list/describe AWS API calls
       }

       /// Phase 2: Detailed security/compliance information
       pub async fn get_security_details(
           &self,
           account_id: &str,
           region: &str,
           resource_id: &str,
       ) -> Result<serde_json::Value> {
           // Fetch policies, configurations, encryption settings, etc.
       }
   }
   ```

2. **Create Normalizer** in `normalizers/newservice.rs`:
   ```rust
   pub struct NewServiceNormalizer;
   
   impl ResourceNormalizer for NewServiceNormalizer {
       fn normalize(&self, raw_data: &serde_json::Value, account_id: &str, region: &str) -> Result<ResourceEntry> {
           // Transform to ResourceEntry
       }
   }
   ```

3. **Register in AWS Client**:
   - Add lazy service getter method (e.g., `get_newservice_service()`)
   - Add resource type routing in `query_resources_by_type()`
   - Add Phase 2 detail fetching in `fetch_resource_details()` match arm
   - Add resource type to `enrichable_types` array in `start_phase2_enrichment()`
   - Add normalizer mapping in `NormalizerFactory::create_normalizer()`

4. **Add Resource Types**: Extend `get_default_resource_types()` in `dialogs.rs`

5. **Update Documentation**: Add new API calls to [AWS API Calls Inventory](aws-api-calls-inventory.md)

**Architectural Decisions:**
- **Concurrent Processing**: Balances API rate limits with performance using semaphore control
- **Query Caching**: 15-minute staleness threshold balances data freshness vs. performance
- **Color Assignment**: Deterministic hashing ensures consistent visual organization
- **Credential Strategy**: Session-based approach minimizes Identity Center API calls

## Status Channel System

The StatusChannel provides thread-safe progress reporting from async operations to the UI.

**Components:**
- `StatusMessage`: Individual status update with category, operation, detail, and completion flag
- `StatusChannel`: Arc<RwLock<VecDeque<StatusMessage>>> for thread-safe message collection
- Automatic message expiration (3-second display duration)
- Maximum 50 messages retained in buffer

**Usage Pattern:**
```rust
// From an async AWS operation
status_channel.send(StatusMessage::starting("IAM", "list_roles", None));
// ... perform operation ...
status_channel.send(StatusMessage::completed("IAM", "list_roles", Some("42 roles")));
```

**UI Integration:**
The status bar displays the most recent fresh message, providing real-time feedback during resource discovery and enrichment operations.

## JSON Expansion Utility

AWS APIs often return policy documents and configurations as URL-encoded or stringified JSON. The json_expansion module automatically detects and expands these embedded JSON strings for improved readability.

**Capabilities:**
- Detects URL-encoded JSON strings and decodes them
- Parses stringified JSON into proper JSON objects
- Recursively processes all values in the JSON tree
- Field hints for common embedded JSON fields (PolicyDocument, AssumeRolePolicyDocument, etc.)

**Example Transformation:**
```json
// Before
{ "PolicyDocument": "%7B%22Version%22%3A%222012-10-17%22%7D" }

// After
{ "PolicyDocument": { "Version": "2012-10-17" } }
```

**Integration:**
Applied automatically during resource normalization for IAM policies, Lambda configurations, and other resources with embedded JSON content.

**References:**
- [Credential Management](credential-management.md) - AWS authentication and multi-account access
- [Resource Normalizers](resource-normalizers.md) - Data transformation patterns