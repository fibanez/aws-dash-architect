# AWS Explorer - Implementation Plan

## Overview

The AWS Explorer is a new egui window feature that provides a comprehensive view of AWS resources across multiple accounts and regions. It replaces the need to navigate the AWS Console by providing a unified, searchable tree view of all resources with advanced filtering and grouping capabilities. Designed to handle hundreds of accounts across dozens of regions efficiently.

## Core Features

### 1. Advanced Resource Tree View
- Grouping**: Dropdown menu for hierarchical 
- **Color-Coded Display**: Consistent color coding for accounts and regions
- **Context Labels**: Show account/region labels next to resources when not grouped by those fields
- **Smart Resource Display**: Show meaningful names (e.g., instance names) instead of just IDs
- **Resource Status**: Display important status information (e.g., EC2 instance state, enabled/disabled)
- **Resource Relationships**: Show connections between related resources (EC2→Security Groups→VPCs)

### 2. Smart Query Management
- **Fuzzy Search Selection**: All add buttons open fuzzy search dialogs:
  - **Add Account**: Fuzzy search through available AWS accounts
  - **Add Region**: Fuzzy search through AWS regions
  - **Add Resource**: Fuzzy search through supported resource types
- **Tag Display**: Show active selections as colored, removable tags
- **Default Population**: Auto-populate with current project's accounts and regions
- **Session Caching**: Query each account/region/resource combination once per session

### 3. Advanced Search and Filtering
- **Global Fuzzy Search**: Search across all resource properties and metadata
- **Real-time Filtering**: Instant filtering of tree view based on search
- **Highlight Matches**: Yellow highlighting of matching text in results
- **Context Preservation**: Maintain parent hierarchy for matched resources
- **Search Performance**: Optimized for large datasets (hundreds of accounts)

### 4. Parallel Data Collection & Refresh
- **Background Threading**: Query AWS APIs in parallel for all account/region/resource combinations
- **Session-Based Caching**: Cache results until session ends or manual refresh
- **Selective Refresh**: Refresh button opens dialog to choose specific combinations to re-query
- **Progress Indicators**: Show loading state with progress bars for active queries
- **Error Handling**: Graceful handling of API failures with retry options

## Technical Architecture

### Core Data Structures

```rust
#[derive(Debug, Clone)]
pub struct ResourceEntry {
    pub resource_type: String,           // AWS::EC2::Instance
    pub account_id: String,
    pub region: String,
    pub resource_id: String,
    pub display_name: String,            // Human readable name (instance name, role name, etc.)
    pub status: Option<String>,          // Instance state, enabled/disabled, etc.
    pub properties: serde_json::Value,   // Normalized AWS API response
    pub raw_properties: serde_json::Value, // Original AWS API response
    pub tags: Vec<ResourceTag>,
    pub relationships: Vec<ResourceRelationship>, // Connections to other resources
    pub account_color: egui::Color32,    // Consistent color for account
    pub region_color: egui::Color32,     // Consistent color for region
}

#[derive(Debug, Clone)]
pub struct ResourceTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct ResourceRelationship {
    pub relationship_type: RelationshipType,
    pub target_resource_id: String,
    pub target_resource_type: String,
}

#[derive(Debug, Clone)]
pub enum RelationshipType {
    Uses,        // EC2 uses Security Group
    Contains,    // VPC contains Subnet
    AttachedTo,  // EBS attached to EC2
    MemberOf,    // User member of Group
}

#[derive(Debug, Clone)]
pub struct QueryScope {
    pub accounts: Vec<AccountSelection>,
    pub regions: Vec<RegionSelection>,
    pub resource_types: Vec<ResourceTypeSelection>,
}

#[derive(Debug, Clone)]
pub struct AccountSelection {
    pub account_id: String,
    pub display_name: String,
    pub color: egui::Color32,
}

#[derive(Debug, Clone)]
pub struct RegionSelection {
    pub region_code: String,
    pub display_name: String,
    pub color: egui::Color32,
}

#[derive(Debug, Clone)]
pub struct ResourceTypeSelection {
    pub resource_type: String,
    pub display_name: String,
    pub service_name: String,  // EC2, IAM, Bedrock, etc.
}

#[derive(Debug)]
pub struct ResourceExplorerState {
    pub resources: Vec<ResourceEntry>,
    pub query_scope: QueryScope,
    pub search_filter: String,
    pub primary_grouping: GroupingMode,
    pub secondary_grouping: GroupingMode,
    pub loading_tasks: HashSet<String>,     // Track active queries
    pub cached_queries: HashMap<String, Vec<ResourceEntry>>, // Session cache
    pub show_refresh_dialog: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GroupingMode {
    ByAccount,
    ByRegion,
    ByResourceType,
}
```

### Core Functions

#### 1. Main Query Orchestration
```rust
pub async fn query_aws_resources(
    scope: QueryScope,
    progress_sender: mpsc::Sender<QueryProgress>,
    cache: &mut HashMap<String, Vec<ResourceEntry>>
) -> Result<Vec<ResourceEntry>, anyhow::Error>

// Uses AWS SDK with dash default profile credentials only
pub async fn parallel_query_all_combinations(
    accounts: &[AccountSelection],
    regions: &[RegionSelection], 
    resource_types: &[ResourceTypeSelection],
    cache: &HashMap<String, Vec<ResourceEntry>>
) -> Result<Vec<ResourceEntry>, anyhow::Error>
```

#### 2. Resource Normalization Layer
```rust
// Trait for normalizing different AWS service responses
pub trait ResourceNormalizer {
    fn normalize(&self, raw_response: serde_json::Value, account: &str, region: &str) -> Result<ResourceEntry, anyhow::Error>;
    fn extract_relationships(&self, entry: &ResourceEntry, all_resources: &[ResourceEntry]) -> Vec<ResourceRelationship>;
}

// Individual service normalizers
pub struct EC2Normalizer;
pub struct IAMNormalizer;
pub struct BedrockNormalizer;
```

#### 3. Individual Resource Queries (AWS SDK Based)
```rust
pub async fn query_ec2_instances(
    account: &str, 
    region: &str
) -> Result<Vec<serde_json::Value>, anyhow::Error>

pub async fn query_iam_roles(
    account: &str
) -> Result<Vec<serde_json::Value>, anyhow::Error>

pub async fn query_bedrock_models(
    account: &str, 
    region: &str
) -> Result<Vec<serde_json::Value>, anyhow::Error>

// Add security groups, VPCs, etc.
pub async fn query_ec2_security_groups(
    account: &str, 
    region: &str
) -> Result<Vec<serde_json::Value>, anyhow::Error>
```

#### 4. Search and Filtering
```rust
pub fn fuzzy_filter_resources(
    resources: &[ResourceEntry],
    search_term: &str
) -> Vec<ResourceEntry>

pub fn build_hierarchical_tree(
    resources: &[ResourceEntry],
    primary_grouping: GroupingMode,
    secondary_grouping: GroupingMode
) -> TreeNode

// Color assignment for consistent UI display
pub fn assign_account_color(account_id: &str) -> egui::Color32
pub fn assign_region_color(region: &str) -> egui::Color32
```

#### 5. Fuzzy Search Dialogs
```rust
pub fn show_account_fuzzy_search(
    ui: &mut egui::Ui,
    available_accounts: &[String]
) -> Option<AccountSelection>

pub fn show_region_fuzzy_search(
    ui: &mut egui::Ui,
    available_regions: &[String]
) -> Option<RegionSelection>

pub fn show_resource_type_fuzzy_search(
    ui: &mut egui::Ui,
    available_types: &[String]
) -> Option<ResourceTypeSelection>
```

## Initial Resource Types

### Phase 1 - Core Services
1. **EC2**
   - `AWS::EC2::Instance` - EC2 instances
   - `AWS::EC2::SecurityGroup` - Security groups
   - `AWS::EC2::VPC` - Virtual Private Clouds

2. **IAM**
   - `AWS::IAM::Role` - IAM roles
   - `AWS::IAM::User` - IAM users  
   - `AWS::IAM::Policy` - IAM policies

3. **Bedrock**
   - `AWS::Bedrock::Model` - Foundation models
   - `AWS::Bedrock::KnowledgeBase` - Knowledge bases

### Phase 2 - Extended Services (Future)
- S3 buckets, Lambda functions, RDS instances, etc.

## UI Components

## Implementation Steps

### Phase 1: Core Infrastructure ✅ COMPLETED
1. ✅ **Create base data structures** (`ResourceEntry`, `QueryScope`, etc.)
2. ✅ **Implement basic UI window** with tree view and buttons
3. ✅ **Add account/region/resource management** (tags with close buttons)
4. ✅ **Implement basic tree rendering** with grouping modes

### Phase 2: AWS Integration ✅ COMPLETED  
1. ✅ **Set up AWS SDK dependencies** for EC2, IAM, Bedrock
2. ✅ **Implement individual resource query functions**
3. ✅ **Create async query orchestration** with parallel execution
4. ✅ **Add progress tracking and error handling**

### Phase 3: Search and Filtering ✅ COMPLETED
1. ✅ **Implement fuzzy search algorithm**
2. ✅ **Add real-time filtering** of tree view
3. ✅ **Implement search result highlighting**
4. ✅ **Add search performance optimizations**

### Phase 4: Polish and Integration ✅ COMPLETED
1. ✅ **Integrate with command palette**
2. ✅ **Add keyboard navigation support**
3. ✅ **Implement data caching** for performance
4. ✅ **Add comprehensive error handling and logging**

### Phase 5: Dialog Implementation and Testing ✅ COMPLETED
1. ✅ **Implement proper fuzzy search dialogs** - Complete with keyboard navigation, double-click selection, and proper filtering
2. ✅ **Add actual AWS API querying functionality** - Integrated real AWS SDK calls with dash default profile credentials
3. ✅ **Implement tree view with proper hierarchical display** - Full tree rendering with color coding and expandable nodes
4. ⏸️ **Add resource relationship extraction** - Framework exists, basic implementation ready for future enhancement
5. ✅ **Implement refresh dialog with selective query combinations** - Complete with checkboxes, Select All/Clear All, and cache clearing
6. ✅ **Test basic functionality once compilation is working** - All components compile and integrate successfully

### Phase 6: Performance and UI Improvements ✅ COMPLETED
1. ✅ **Use stable node IDs based on accountid:regionid:resourceid** for tree nodes to prevent CollapsingHeader state loss
2. ✅ **Only start search filtering after 3 characters have been typed** to reduce tree rebuilds
3. ✅ **Cache tree structure and only rebuild when data actually changes** to prevent flickering
4. ✅ **Add cleanup logic when tags are removed** - filter displayed resources without clearing cache
5. ✅ **Track query execution timestamps** and mark stale items in UI for user awareness of data age
6. ✅ **Add Resource Explorer to continuous repaint logic** in app.rs
7. ✅ **Fixed resource type grouping** to create collapsible nodes for individual resource types
8. ✅ **Fixed tree cache key generation** to properly detect resource changes beyond first 10 resources
9. ✅ **Removed email from account tag display** in Active Selection - show only account name and account ID
10. ✅ **Added theme-aware color adjustment** for tree node headers - use dark colors for readability on light background
11. ✅ **Changed tree indentation guide lines** to very light grey color for improved visual hierarchy

### Phase 6.1: Additional UI Improvements ✅ COMPLETED
1. ✅ **Fix spinning wheel not showing** when AWS client is active - Added continuous UI repaints and improved loading state management
2. ✅ **Add dropdown option to view resource JSON properties** - Added "</>" button with popup showing formatted JSON and copy functionality

### Phase 7: Global Resource Optimization 🔮 PLANNED
1. 🔮 **Implement global resource handling for IAM/CloudFront** - query once per account instead of per region to avoid duplicate API calls and bandwidth waste

### Phase 8: Future Enhancements 🔮 FUTURE
1. **Resource relationship extraction** - Implement EC2→Security Group→VPC connections
2. **Add dropdown option to view resource JSON properties**
3. **Advanced filtering** - Add tag-based filtering and property search
4. **Export functionality** - CSV/JSON export of resource lists
5. **AI Agent Integration** - Implement find_resources_by_type_and_tags and verify_cloudformation_deployment APIs

## Multi-Account Credential Architecture

### Critical Requirement: Coordinate Hundreds of Credential Sets

The AWS Explorer must handle potentially hundreds of AWS accounts, each requiring separate credentials from AWS Identity Center using the same role name. This architecture ensures proper credential coordination without environment variables.

#### Core Architecture Principles

1. **Single Role, Multiple Accounts**: Use one role name (typically "awsdash") across hundreds of different AWS accounts
2. **Per-Account Credentials**: Each account requires its own credential set from AWS Identity Center
3. **Thread-Safe Coordination**: Each resource collection thread must receive the correct credentials for its target account
4. **Internal Management**: No environment variables - all credential coordination handled internally
5. **Credential Caching**: Cache credentials per account to avoid redundant Identity Center requests



### Security Considerations

1. **Memory Security**: Clear sensitive credential data when no longer needed
2. **Thread Safety**: All credential operations must be thread-safe
3. **Expiration Handling**: Automatic credential refresh before expiration
4. **Error Isolation**: Credential failures for one account shouldn't affect others
5. **Logging Security**: Never log sensitive credential data

### Performance Optimizations

1. **Parallel Credential Requests**: Request credentials for multiple accounts in parallel
2. **Credential Preloading**: Proactively request credentials for known accounts
3. **Smart Caching**: Cache credentials with appropriate TTL based on expiration
4. **Connection Pooling**: Reuse AWS clients where possible with same credentials

This architecture ensures that each resource collection operation gets the correct credentials for its target account while maintaining efficient caching and coordination across potentially hundreds of accounts.

### Resource Normalization Strategy
- **Inspiration**: Review GitHub project `awsets` (Go) for normalization patterns
- **Consistent Format**: All resources normalized to common structure regardless of AWS service
- **Relationship Mapping**: Extract common resource relationships during normalization:
  - EC2 instances → Security Groups → VPCs
  - IAM Users → Groups → Policies
  - EBS Volumes → EC2 Instances

### Session Caching Strategy
- **Cache Duration**: Session-based caching (until application restart)
- **Cache Key**: `{account_id}:{region}:{resource_type}` combination
- **Manual Refresh**: Selective refresh dialog for specific combinations
- **No Auto-Refresh**: Manual refresh only, no automatic intervals

## Performance Optimization

### Large Scale Support
- **Target Scale**: Hundreds of accounts across dozens of regions
- **Parallel Queries**: Concurrent AWS API calls for each account/region/resource combination
- **UI Responsiveness**: Background threading with progress indicators
- **Memory Management**: Efficient storage of large resource datasets

### Smart Display Features
- **Meaningful Names**: Show resource names instead of IDs when available
- **Status Indicators**: Display important status (running/stopped, enabled/disabled)
- **Color Coding**: Consistent colors for accounts and regions throughout UI
- **Context Labels**: Show account/region when not part of current grouping

## Integration Points

- **Command Palette**: Add "Open AWS Explorer" command
- **Project System**: Auto-populate accounts/regions from current project
- **AWS Login**: Integrate with existing dash default profile credentials
- **Window Management**: Implement `WindowFocus` trait for keyboard navigation
- **egui Framework**: Follow existing egui patterns and conventions
- **Logging**: Use existing logging framework for debugging and monitoring


## AI Agent Integration Milestone

### Overview
Create internal functions that can be exposed as tools for AI agents to query AWS resources. This enables AI-powered CloudFormation template creation and deployment verification.

### Primary Use Cases
1. **Template Parameter Discovery**: Find existing resources (VPCs, subnets, security groups) to use as parameters in CloudFormation templates
2. **Deployment Verification**: Verify deployments by checking resources with specific CloudFormation stack tags

### Proposed AI LLM Tools

#### 1. `find_resources_by_type_and_tags`
```rust
/// Find AWS resources of specific types with matching tags
/// Perfect for finding existing infrastructure to reference in CF templates
pub async fn find_resources_by_type_and_tags(
    resource_types: Vec<String>,    // ["AWS::EC2::VPC", "AWS::EC2::Subnet"]
    tags: HashMap<String, String>,  // {"Environment": "prod", "Team": "backend"}
    regions: Option<Vec<String>>,   // None = all regions, Some(vec) = specific regions
    accounts: Option<Vec<String>>   // None = current account, Some(vec) = specific accounts
) -> Result<Vec<ResourceEntry>, anyhow::Error>
```

**AI Tool Use Cases**:
- "Find all VPCs tagged with Environment=prod in us-east-1"
- "List all subnets in the backend team's infrastructure"
- "Show me security groups that are tagged for the web tier"

#### 2. `verify_cloudformation_deployment`
```rust
/// Verify CloudFormation deployment by checking resources with stack tags
/// Essential for deployment verification and troubleshooting
pub async fn verify_cloudformation_deployment(
    stack_name: String,
    expected_resources: Vec<String>, // Resource types that should exist
    region: String,
    account: Option<String>
) -> Result<DeploymentVerification, anyhow::Error>

#[derive(Debug)]
pub struct DeploymentVerification {
    pub stack_exists: bool,
    pub expected_resources_found: Vec<ResourceEntry>,
    pub missing_resources: Vec<String>,
    pub unexpected_resources: Vec<ResourceEntry>,
    pub resource_health: HashMap<String, ResourceHealth>
}
```

**AI Tool Use Cases**:
- "Verify that my CloudFormation stack 'my-web-app' deployed all expected EC2 and RDS resources"
- "Check if all resources in stack 'infrastructure-prod' are healthy and properly tagged"
- "Find any orphaned resources that might be left over from a failed deployment"

### Additional Tool Candidates

#### 3. `find_resources_by_property`
```rust
/// Search resources by any property value (name, state, configuration)
/// Useful for finding resources matching specific criteria
pub async fn find_resources_by_property(
    property_path: String,      // "state" or "tags.Name" or "instanceType"
    property_value: String,     // "running" or "web-server" or "t3.micro"
    resource_types: Option<Vec<String>>
) -> Result<Vec<ResourceEntry>, anyhow::Error>
```

#### 4. `get_resource_relationships`
```rust
/// Get all related resources for a given resource
/// Helps understand resource dependencies and connections
pub async fn get_resource_relationships(
    resource_id: String,
    resource_type: String,
    relationship_depth: u32  // 1 = direct relationships, 2+ = nested
) -> Result<ResourceRelationshipMap, anyhow::Error>
```
