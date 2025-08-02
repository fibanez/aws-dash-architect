# Credential Management

Multi-account AWS credential coordination system providing secure session management, automatic refresh, and account color assignment through AWS Identity Center integration for hundreds of accounts.

## Core Functionality

**Credential Coordination:**
- Session-based credential caching with automatic expiration handling (5-minute buffer)
- Multi-account credential management for hundreds of AWS accounts
- Account-specific role assumption through AWS Identity Center integration
- Secure credential storage with proper session token handling
- Automatic credential refresh when sessions expire

**Key Features:**
- Credential cache with RwLock-based thread-safe access for concurrent operations
- Account color assignment using deterministic hashing for visual consistency
- Expiration management with proactive refresh before credential expiry
- Role-based access through Identity Center (typically "awsdash" role)
- Mock credential support for testing environments

**Main Components:**
- **CredentialCoordinator**: Central credential management with caching and Identity Center integration
- **AccountCredentials**: Individual account credential structure with expiration tracking
- **AccountColors**: Deterministic color assignment for visual account organization
- **Identity Center Integration**: Live credential requests through AWS SSO

**Integration Points:**
- Resource Explorer System for multi-account resource queries
- AWS Identity Center for credential acquisition and session management
- AWS SDK integration for service client creation
- Visual systems for account color consistency across UI

## Implementation Details

**Key Files:**
- `src/app/resource_explorer/credentials.rs` - Credential coordination and account management
- `src/app/aws_identity.rs` - AWS Identity Center integration (referenced system)

**AccountCredentials Structure:**
```rust
pub struct AccountCredentials {
    pub account_id: String,
    pub role_name: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: DateTime<Utc>,
}
```

**Credential Lifecycle:**
1. **Request**: Account credentials requested for AWS API calls
2. **Cache Check**: Search existing credential cache for valid sessions
3. **Expiration Check**: Verify credentials not expired (5-minute buffer)
4. **Identity Center Request**: Request new credentials if expired/missing
5. **Cache Update**: Store new credentials with expiration timestamp
6. **AWS SDK Integration**: Convert to AWS SDK credential format

**Thread-Safe Caching:**
```rust
credential_cache: Arc<RwLock<HashMap<String, AccountCredentials>>>,
```

**Expiration Management:**
- 5-minute expiration buffer prevents credential failures during operations
- Proactive refresh triggered before actual expiration
- Automatic cleanup of expired credentials from cache

**Account Color Assignment:**
- Deterministic hashing ensures consistent colors across application sessions
- Visual distinction between accounts in resource explorer and visualizations
- Color assignments persist across application restarts

**Security Considerations:**
- Credentials stored in memory only (no persistent storage)
- Session tokens used for temporary access
- Automatic cleanup on application exit
- Role-based access through Identity Center

## Developer Notes

**Extension Points for Enhanced Security:**

1. **Add Credential Encryption**:
   ```rust
   // Encrypt credentials before caching
   pub fn encrypt_credentials(&self, creds: &AccountCredentials) -> Result<Vec<u8>> {
       // Implement encryption logic
   }
   
   pub fn decrypt_credentials(&self, encrypted: &[u8]) -> Result<AccountCredentials> {
       // Implement decryption logic
   }
   ```

2. **Implement Audit Logging**:
   ```rust
   // Track credential usage for security monitoring
   pub fn log_credential_access(&self, account_id: &str, operation: &str) {
       info!("AUDIT: Account {} accessed for {}", account_id, operation);
   }
   ```

3. **Add Permission Validation**:
   ```rust
   // Validate account access permissions
   pub async fn validate_account_access(&self, account_id: &str) -> Result<bool> {
       // Check if user has permission to access account
   }
   ```

**Integration Pattern for New AWS Services:**
```rust
// Get credentials for service client creation
let credentials = credential_coordinator.get_credentials_for_account(account_id).await?;
let aws_creds = credentials.to_aws_credentials();

// Create AWS SDK config
let config = aws_config::defaults(BehaviorVersion::latest())
    .region(Region::new(region.clone()))
    .credentials_provider(aws_creds)
    .load()
    .await;

// Create service client
let client = aws_sdk_s3::Client::new(&config);
```

**Account Color System:**
```rust
// Deterministic color assignment
pub fn assign_account_color(account_id: &str) -> Color32 {
    let mut hasher = DefaultHasher::new();
    account_id.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Generate RGB from hash
    let r = ((hash >> 16) & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = (hash & 0xFF) as u8;
    
    Color32::from_rgb(r, g, b)
}
```

**Mock Testing Support:**
```rust
#[cfg(test)]
impl CredentialCoordinator {
    pub fn new_mock() -> Self {
        // Create mock coordinator for testing
    }
}
```

**Performance Optimizations:**
- RwLock allows concurrent read access to credential cache
- Credential requests batched when possible to reduce Identity Center API calls
- Local caching reduces repeated credential acquisition for same accounts
- Expired credential cleanup prevents memory accumulation

**Security Best Practices:**
- **No Persistent Storage**: Credentials never written to disk
- **Session-Based**: Uses temporary session tokens with expiration
- **Proactive Expiry**: Refreshes credentials before expiration to prevent failures
- **Role-Based Access**: Leverages Identity Center role assumptions
- **Memory Cleanup**: Credentials cleared on application shutdown

**Error Handling Strategy:**
- Network failures handled with retry logic for credential requests
- Expired credential detection with automatic refresh attempts
- Identity Center authentication errors with user guidance
- Account access permission errors with clear messaging

**Architectural Decisions:**
- **Centralized Coordination**: Single point for all credential management
- **Cache-First**: Prioritizes cached credentials to minimize API calls
- **Thread-Safe**: Supports concurrent access from multiple resource queries
- **Identity Center Integration**: Leverages existing AWS authentication system
- **Visual Consistency**: Color assignments provide consistent account identification

**References:**
- [Resource Explorer System](resource-explorer-system.md) - Multi-account resource query integration
- [Identity Center Setup](identity-center-setup.md) - AWS SSO configuration
- [AWS Service Integration Patterns](aws-service-integration-patterns.md) - Service client creation patterns