# Resource Explorer System

Comprehensive AWS resource discovery and visualization platform providing multi-account, multi-region resource querying across 156 resource types from 72 AWS services, with hierarchical organization, parent-child resource nesting, and real-time credential management.

## Core Functionality

**Key Features:**
- Multi-account, multi-region AWS resource querying across 156 resource types from 72 services
- Hierarchical tree organization with customizable grouping (by Account, Region, or Resource Type)
- Parent-child resource nesting with automatic recursive querying (6 nested resource types)
- Real-time credential management with session caching and automatic renewal
- Fuzzy search and filtering capabilities for large resource inventories
- Color-coded visual organization for accounts and regions
- Resource relationship mapping and detailed property inspection
- Session-based query caching with staleness detection (15-minute default threshold)

**Main Components:**
- **ResourceExplorer**: Main interface coordinating state management and UI rendering
- **ResourceExplorerState**: Core state container managing resources, queries, and UI state
- **AWSResourceClient**: Orchestrates parallel queries across all supported AWS services
- **CredentialCoordinator**: Manages AWS credentials for hundreds of accounts via Identity Center
- **TreeBuilder/TreeRenderer**: Hierarchical visualization system with stable node IDs
- **NormalizerFactory**: Standardizes AWS API responses into consistent ResourceEntry format

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
- `src/app/resource_explorer/aws_services/` - 72 AWS service modules (EC2, IAM, S3, Lambda, Bedrock, etc.)
- `src/app/resource_explorer/child_resources.rs` - Parent-child resource hierarchy configuration
- `src/app/resource_explorer/normalizers/` - Resource data transformation modules

**Important Patterns:**
- **Service Integration**: Each AWS service follows consistent pattern with service module and normalizer
- **Parallel Queries**: Uses `FuturesUnordered` with semaphore-based concurrency control (20 concurrent requests)
- **Credential Management**: Session-based caching with 5-minute expiration buffer and automatic refresh
- **State Management**: Arc<RwLock<ResourceExplorerState>> for thread-safe state sharing

**Configuration Requirements:**
- AWS Identity Center must be configured for multi-account access
- Default role name (typically "awsdash") for credential assumption
- Memory limits require `-j 7` flag for testing due to large concurrent operations

## Developer Notes

**Extension Points for Adding New Services:**

1. **Create Service Module** in `aws_services/newservice.rs`:
   ```rust
   pub struct NewService {
       credential_coordinator: Arc<CredentialCoordinator>,
   }
   
   impl NewService {
       pub async fn list_resources(&self, account_id: &str, region: &str) -> Result<Vec<serde_json::Value>> {
           // Implement AWS API calls
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
   - Add lazy service getter method
   - Add resource type routing in `query_resources_by_type()`
   - Add detail fetching in `fetch_detailed_properties()` 
   - Add normalizer mapping in `NormalizerFactory::create_normalizer()`

4. **Add Resource Types**: Extend `get_default_resource_types()` in `dialogs.rs`

**Architectural Decisions:**
- **Concurrent Processing**: Balances API rate limits with performance using semaphore control
- **Query Caching**: 15-minute staleness threshold balances data freshness vs. performance  
- **Color Assignment**: Deterministic hashing ensures consistent visual organization
- **Credential Strategy**: Session-based approach minimizes Identity Center API calls

**References:**
- [Window Focus System](window-focus-system.md) - Integration with application focus management
- [Credential Management](credential-management.md) - AWS authentication and multi-account access
- [Trait Patterns](trait-patterns.md) - Common patterns used throughout the system