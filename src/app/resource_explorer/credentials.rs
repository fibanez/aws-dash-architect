use crate::app::aws_identity::AwsIdentityCenter;
use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_types::region::Region;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Account-specific credentials from AWS Identity Center
#[derive(Debug, Clone)]
pub struct AccountCredentials {
    pub account_id: String,
    pub role_name: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: DateTime<Utc>,
}

impl AccountCredentials {
    /// Check if credentials are expired or will expire within the next 5 minutes
    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        let buffer = chrono::Duration::minutes(5);
        now + buffer >= self.expiration
    }

    /// Create AWS SDK Credentials from this account's credentials
    pub fn to_aws_credentials(&self) -> Credentials {
        Credentials::from_keys(
            &self.access_key_id,
            &self.secret_access_key,
            Some(self.session_token.clone()),
        )
    }
}

/// Coordinator for managing credentials across hundreds of AWS accounts
#[derive(Debug)]
pub struct CredentialCoordinator {
    /// Cache of credentials per account ID
    credential_cache: Arc<RwLock<HashMap<String, AccountCredentials>>>,
    /// AWS Identity Center reference for credential requests (live reference)
    identity_center: Arc<std::sync::Mutex<AwsIdentityCenter>>,
    /// Default role name from login (usually "awsdash")
    default_role_name: String,
}

impl CredentialCoordinator {
    /// Create a new credential coordinator
    pub fn new(
        identity_center: Arc<std::sync::Mutex<AwsIdentityCenter>>,
        default_role_name: String,
    ) -> Self {
        Self {
            credential_cache: Arc::new(RwLock::new(HashMap::new())),
            identity_center,
            default_role_name,
        }
    }

    /// Create a mock credential coordinator for testing (when no real AWS Identity Center is available)
    #[cfg(test)]
    pub fn new_mock() -> Self {
        use crate::app::aws_identity::AwsIdentityCenter;
        let mock_identity = AwsIdentityCenter::new(
            "https://test.awsapps.com/start".to_string(),
            "TestRole".to_string(),
            "us-east-1".to_string(),
        );
        Self {
            credential_cache: Arc::new(RwLock::new(HashMap::new())),
            identity_center: Arc::new(std::sync::Mutex::new(mock_identity)),
            default_role_name: "TestRole".to_string(),
        }
    }

    /// Get or request credentials for a specific account
    pub async fn get_credentials_for_account(
        &self,
        account_id: &str,
    ) -> Result<AccountCredentials> {
        debug!(
            "🔑 CREDS: get_credentials_for_account ENTRY for account: {}",
            account_id
        );
        debug!("Getting credentials for account: {}", account_id);

        // Check cache first
        debug!("🔑 CREDS: Checking cache for account: {}", account_id);
        if let Some(cached_creds) = self.get_cached_credentials(account_id).await {
            debug!(
                "🔑 CREDS: Found cached credentials for account: {}",
                account_id
            );
            if !cached_creds.is_expired() {
                debug!(
                    "🔑 CREDS: Using cached credentials for account: {}",
                    account_id
                );
                debug!("Using cached credentials for account: {}", account_id);
                return Ok(cached_creds);
            } else {
                debug!("🔑 CREDS: Cached credentials for account {} using role {} are expired, requesting fresh credentials", account_id, cached_creds.role_name);
            }
        } else {
            debug!(
                "🔑 CREDS: No cached credentials found for account: {}",
                account_id
            );
        }

        // Request new credentials from AWS Identity Center
        debug!(
            "🔑 CREDS: Requesting fresh credentials for account: {}",
            account_id
        );
        let fresh_creds = self
            .request_fresh_credentials(account_id)
            .await
            .with_context(|| {
                format!("Failed to get fresh credentials for account {}", account_id)
            })?;

        debug!(
            "🔑 CREDS: Got fresh credentials, now caching for account: {}",
            account_id
        );
        // Cache the credentials
        self.cache_credentials(account_id, &fresh_creds).await;

        debug!("🔑 CREDS: get_credentials_for_account EXIT successfully for account: {} using role: {}", account_id, fresh_creds.role_name);
        Ok(fresh_creds)
    }

    /// Get cached credentials for an account if available
    async fn get_cached_credentials(&self, account_id: &str) -> Option<AccountCredentials> {
        let cache = self.credential_cache.read().await;
        cache.get(account_id).cloned()
    }

    /// Cache credentials for an account
    async fn cache_credentials(&self, account_id: &str, credentials: &AccountCredentials) {
        let mut cache = self.credential_cache.write().await;
        cache.insert(account_id.to_string(), credentials.clone());
        debug!(
            "Cached credentials for account: {} using role: {}",
            account_id, credentials.role_name
        );
    }

    /// Request fresh credentials from AWS Identity Center for specific account
    async fn request_fresh_credentials(&self, account_id: &str) -> Result<AccountCredentials> {
        debug!(
            "🔑 CREDS: request_fresh_credentials ENTRY for account: {}",
            account_id
        );
        debug!(
            "Requesting fresh credentials from AWS Identity Center for account: {}",
            account_id
        );

        // Get role credentials from AWS Identity Center (via live reference)
        // Clone the identity center to avoid holding the lock across await
        debug!(
            "🔑 CREDS: Acquiring Identity Center lock for account: {}",
            account_id
        );
        let identity_center_clone = {
            let identity_center = self
                .identity_center
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to acquire lock on Identity Center: {}", e))?;
            debug!(
                "🔑 CREDS: Successfully acquired Identity Center lock, cloning for account: {}",
                account_id
            );
            identity_center.clone()
        };
        debug!(
            "🔑 CREDS: Released Identity Center lock for account: {}",
            account_id
        );

        debug!(
            "🔑 CREDS: Calling Identity Center get_role_credentials for account {} with role '{}'",
            account_id, self.default_role_name
        );
        debug!(
            "Requesting credentials for account {} with role '{}'",
            account_id, self.default_role_name
        );
        let role_credentials = identity_center_clone
            .get_role_credentials(account_id, &self.default_role_name)
            .await
            .with_context(|| {
                format!(
                    "Failed to get role credentials for account {} with role {}",
                    account_id, self.default_role_name
                )
            })?;

        debug!(
            "🔑 CREDS: Identity Center returned credentials for account: {}",
            account_id
        );
        debug!("Successfully received credentials from AWS Identity Center for account: {} using role: {}", account_id, role_credentials.role_name);
        debug!(
            "Credential details - Account ID in response: {}, Role in response: {}",
            role_credentials.account_id, role_credentials.role_name
        );
        debug!(
            "🔑 CREDS: request_fresh_credentials EXIT for account: {}",
            account_id
        );
        Ok(role_credentials)
    }

    /// Create AWS SDK config with account-specific credentials
    pub async fn create_aws_config_for_account(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<aws_config::SdkConfig> {
        debug!(
            "🔑 CREDS: create_aws_config_for_account ENTRY for account: {} in region: {}",
            account_id, region
        );

        debug!(
            "🔑 CREDS: Calling get_credentials_for_account for {}",
            account_id
        );
        let creds = self.get_credentials_for_account(account_id).await?;
        debug!("🔑 CREDS: Got credentials for account {}", account_id);

        let aws_credentials = creds.to_aws_credentials();

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .credentials_provider(aws_credentials)
            .load()
            .await;

        debug!(
            "Successfully created AWS config for account: {} in region: {}",
            account_id, region
        );
        Ok(config)
    }

    /// Clear expired credentials from cache
    pub async fn cleanup_expired_credentials(&self) -> usize {
        let mut cache = self.credential_cache.write().await;
        let initial_count = cache.len();

        cache.retain(|account_id, creds| {
            let expired = creds.is_expired();
            if expired {
                debug!(
                    "Removing expired credentials for account: {} using role: {}",
                    account_id, creds.role_name
                );
            }
            !expired
        });

        let removed_count = initial_count - cache.len();
        if removed_count > 0 {
            info!("Cleaned up {} expired credential entries", removed_count);
        }

        removed_count
    }

    /// Get cache statistics for monitoring
    pub async fn get_cache_stats(&self) -> CredentialCacheStats {
        let cache = self.credential_cache.read().await;
        let total_entries = cache.len();
        let mut expired_entries = 0;
        let mut valid_entries = 0;

        for creds in cache.values() {
            if creds.is_expired() {
                expired_entries += 1;
            } else {
                valid_entries += 1;
            }
        }

        CredentialCacheStats {
            total_entries,
            valid_entries,
            expired_entries,
        }
    }

    /// Preload credentials for multiple accounts in parallel
    pub async fn preload_credentials(&self, account_ids: &[String]) -> Result<usize> {
        info!("Preloading credentials for {} accounts", account_ids.len());

        let mut handles = Vec::new();

        for account_id in account_ids {
            let coordinator = self.clone();
            let account_id = account_id.clone();

            let handle =
                tokio::spawn(
                    async move { coordinator.get_credentials_for_account(&account_id).await },
                );

            handles.push(handle);
        }

        let mut successful_preloads = 0;

        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => {
                    successful_preloads += 1;
                }
                Ok(Err(e)) => {
                    warn!("Failed to preload credentials: {}", e);
                }
                Err(e) => {
                    error!("Preload task panicked: {}", e);
                }
            }
        }

        info!(
            "Successfully preloaded credentials for {}/{} accounts",
            successful_preloads,
            account_ids.len()
        );
        Ok(successful_preloads)
    }

    /// Get the CloudFormation deployment role name if available
    pub fn get_cloudformation_deployment_role_name(&self) -> Option<String> {
        self.identity_center
            .lock()
            .ok()
            .and_then(|identity| identity.cloudformation_deployment_role_name.clone())
    }

    /// Create AWS SDK config specifically for CloudFormation deployment operations
    ///
    /// This method uses Identity Center credentials (AWSReservedSSO_awsdash_*) for authentication.
    /// The CloudFormation service role is passed separately to the CloudFormation API calls.
    pub async fn create_deployment_aws_config(
        &self,
        target_account_id: &str,
        region: &str,
    ) -> Result<aws_config::SdkConfig> {
        info!(
            "Creating deployment AWS config for account: {} in region: {}",
            target_account_id, region
        );

        // Use Identity Center credentials (AWSReservedSSO_awsdash_*) for CloudFormation API authentication
        let base_creds = self
            .get_credentials_for_account(target_account_id)
            .await
            .with_context(|| {
                format!(
                    "Failed to get Identity Center credentials for account {}",
                    target_account_id
                )
            })?;

        info!(
            "Using Identity Center role '{}' for CloudFormation API authentication",
            base_creds.role_name
        );
        info!(
            "Credentials obtained for account: {} (matches target: {})",
            base_creds.account_id,
            base_creds.account_id == target_account_id
        );

        let aws_credentials = base_creds.to_aws_credentials();

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region.to_string()))
            .credentials_provider(aws_credentials)
            .load()
            .await;

        info!(
            "Successfully created deployment AWS config for account: {} in region: {}",
            target_account_id, region
        );
        Ok(config)
    }

    /// Construct CloudFormation service role ARN for deployment
    ///
    /// This constructs the ARN for the CloudFormation service role that will be
    /// passed to CloudFormation API calls. The role exists in the target account
    /// but is not discoverable through Identity Center role enumeration.
    pub fn get_cloudformation_service_role_arn(&self, account_id: &str) -> Option<String> {
        self.get_cloudformation_deployment_role_name()
            .map(|role_name| format!("arn:aws:iam::{}:role/{}", account_id, role_name))
    }

    /// Resolve the actual IAM role name from Identity Center discovery
    ///
    /// This method maps logical role names (like "awsdash") to actual IAM role names
    /// (like "AWSReservedSSO_awsdash_37838b6bb020f9ca") using the discovery data
    /// from Identity Center login without any hardcoding.
    pub fn resolve_actual_role_name(
        &self,
        account_id: &str,
        logical_role_name: &str,
    ) -> Option<String> {
        // Get available roles for the account from Identity Center discovery (via live reference)
        let identity = self.identity_center.lock().ok()?;
        if let Some(available_roles) = identity.available_roles.get(account_id) {
            // Look for exact match first
            if available_roles.contains(&logical_role_name.to_string()) {
                return Some(logical_role_name.to_string());
            }

            // Look for SSO-generated role names that contain the logical name
            for role in available_roles {
                if role.contains(logical_role_name) && role.starts_with("AWSReservedSSO_") {
                    debug!(
                        "Resolved logical role '{}' to actual role '{}' in account {}",
                        logical_role_name, role, account_id
                    );
                    return Some(role.clone());
                }
            }

            // Look for role names that end with the logical name (another common pattern)
            for role in available_roles {
                if role.ends_with(logical_role_name) {
                    debug!(
                        "Resolved logical role '{}' to actual role '{}' in account {}",
                        logical_role_name, role, account_id
                    );
                    return Some(role.clone());
                }
            }
        }

        debug!(
            "Could not resolve logical role '{}' to actual role in account {}",
            logical_role_name, account_id
        );
        None
    }

    /// Get discovered accounts from Identity Center (no hardcoding)
    pub fn get_discovered_accounts(&self) -> Vec<String> {
        match self.identity_center.lock() {
            Ok(identity) => {
                let accounts: Vec<String> = identity
                    .accounts
                    .iter()
                    .map(|account| account.account_id.clone())
                    .collect();
                debug!(
                    "Successfully accessed Identity Center, found {} accounts: {:?}",
                    accounts.len(),
                    accounts
                );
                accounts
            }
            Err(e) => {
                debug!(
                    "Failed to lock Identity Center for account discovery: {}",
                    e
                );
                Vec::new()
            }
        }
    }

    /// Get discovered roles for an account from Identity Center (no hardcoding)
    pub fn get_discovered_roles(&self, account_id: &str) -> Vec<String> {
        self.identity_center
            .lock()
            .ok()
            .and_then(|identity| identity.available_roles.get(account_id).cloned())
            .unwrap_or_default()
    }

    /// Validate deployment prerequisites for cross-account CloudFormation deployment
    ///
    /// IMPORTANT: This method checks that all necessary components are available for deployment:
    /// - Identity Center authentication is active
    /// - Target account is discoverable and accessible
    /// - Required roles exist and are assumable
    /// - CloudFormation deployment role is configured
    ///
    /// NOTE FOR DEBUGGING: If this returns empty accounts `[]`, it does NOT mean authentication
    /// has failed. The issue is likely that this CredentialCoordinator instance has a stale
    /// reference to the Identity Center. The shared Identity Center reference works correctly,
    /// but different parts of the application may have CredentialCoordinators created at
    /// different times with different Identity Center references.
    ///
    /// SOLUTION: Ensure CloudFormation Manager gets fresh AWS client before deployment,
    /// which will have a fresh CredentialCoordinator with current Identity Center reference.
    pub async fn validate_deployment_prerequisites(
        &self,
        target_account_id: &str,
        region: &str,
    ) -> Result<DeploymentValidationResult> {
        debug!(
            "Validating deployment prerequisites for account {} in region {}",
            target_account_id, region
        );

        let mut validation = DeploymentValidationResult {
            is_valid: true,
            account_accessible: false,
            role_assumable: false,
            cloudformation_role_configured: false,
            discovered_roles: Vec::new(),
            cloudformation_deployment_role: None,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Check 1: Account is in discovered accounts
        let discovered_accounts = self.get_discovered_accounts();
        debug!(
            "Credential coordinator discovered {} accounts: {:?}",
            discovered_accounts.len(),
            discovered_accounts
        );

        if !discovered_accounts.contains(&target_account_id.to_string()) {
            validation.is_valid = false;
            validation.errors.push(format!(
                "Account {} is not accessible through Identity Center. Available accounts: {:?}",
                target_account_id, discovered_accounts
            ));
            debug!(
                "Account {} NOT found in discovered accounts",
                target_account_id
            );
        } else {
            validation.account_accessible = true;
            debug!(
                "Account {} is accessible through Identity Center",
                target_account_id
            );
        }

        // Check 2: Get available roles for this account
        validation.discovered_roles = self.get_discovered_roles(target_account_id);
        if validation.discovered_roles.is_empty() {
            validation.is_valid = false;
            validation.errors.push(format!(
                "No roles discovered for account {}",
                target_account_id
            ));
        } else {
            debug!(
                "Discovered {} roles for account {}: {:?}",
                validation.discovered_roles.len(),
                target_account_id,
                validation.discovered_roles
            );
        }

        // Check 3: Try to get credentials for default role
        match self.get_credentials_for_account(target_account_id).await {
            Ok(creds) => {
                validation.role_assumable = true;
                debug!(
                    "Successfully validated role assumption for account {} using role {}",
                    target_account_id, creds.role_name
                );
            }
            Err(e) => {
                validation.is_valid = false;
                validation.errors.push(format!(
                    "Cannot assume role in account {}: {}",
                    target_account_id, e
                ));
            }
        }

        // Check 4: CloudFormation deployment role configuration
        if let Some(cf_role) = self.get_cloudformation_deployment_role_name() {
            validation.cloudformation_role_configured = true;
            validation.cloudformation_deployment_role = Some(cf_role.clone());

            debug!("CloudFormation service role '{}' will be passed to CloudFormation API for account {}", cf_role, target_account_id);
            debug!("Note: CloudFormation service roles are not discoverable through Identity Center role enumeration");
        } else {
            validation.warnings.push("No CloudFormation deployment role configured. CloudFormation will use caller's permissions for resource operations.".to_string());
        }

        // Additional validation: Region support
        if region.is_empty() {
            validation.is_valid = false;
            validation.errors.push("Region cannot be empty".to_string());
        }

        info!("Deployment validation for {}/{}: valid={}, accessible={}, role_assumable={}, cf_role_configured={}",
              target_account_id, region, validation.is_valid, validation.account_accessible,
              validation.role_assumable, validation.cloudformation_role_configured);

        Ok(validation)
    }

    /// Get management account ID from Identity Center discovery (no hardcoding)
    pub fn get_management_account_id(&self) -> Option<String> {
        self.identity_center
            .lock()
            .ok()
            .and_then(|identity| identity.sso_management_account_id.clone())
    }

    /// Check if an account is the management account
    pub fn is_management_account(&self, account_id: &str) -> bool {
        self.get_management_account_id()
            .map(|mgmt_id| mgmt_id == account_id)
            .unwrap_or(false)
    }

    /// Get cross-account credential strategy summary
    pub fn get_credential_strategy_info(&self, target_account_id: &str) -> CredentialStrategyInfo {
        let base_role = self.default_role_name.clone();
        let actual_role = self.resolve_actual_role_name(target_account_id, &base_role);
        let cf_role = self.get_cloudformation_deployment_role_name();
        let is_mgmt = self.is_management_account(target_account_id);

        CredentialStrategyInfo {
            target_account_id: target_account_id.to_string(),
            base_role_name: base_role,
            actual_role_name: actual_role.unwrap_or("Unknown".to_string()),
            cloudformation_deployment_role: cf_role.clone(),
            is_management_account: is_mgmt,
            requires_role_assumption: false, // We don't assume the CloudFormation service role
            discovered_roles: self.get_discovered_roles(target_account_id),
        }
    }
}

impl Clone for CredentialCoordinator {
    fn clone(&self) -> Self {
        Self {
            credential_cache: Arc::clone(&self.credential_cache),
            identity_center: Arc::clone(&self.identity_center),
            default_role_name: self.default_role_name.clone(),
        }
    }
}

/// Statistics about the credential cache
#[derive(Debug, Clone)]
pub struct CredentialCacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

/// Validation result for deployment prerequisites
#[derive(Debug, Clone)]
pub struct DeploymentValidationResult {
    pub is_valid: bool,
    pub account_accessible: bool,
    pub role_assumable: bool,
    pub cloudformation_role_configured: bool,
    pub discovered_roles: Vec<String>,
    pub cloudformation_deployment_role: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Information about credential strategy for cross-account deployment
#[derive(Debug, Clone)]
pub struct CredentialStrategyInfo {
    pub target_account_id: String,
    pub base_role_name: String,
    pub actual_role_name: String,
    pub cloudformation_deployment_role: Option<String>,
    pub is_management_account: bool,
    pub requires_role_assumption: bool,
    pub discovered_roles: Vec<String>,
}

impl CredentialCacheStats {
    /// Get cache hit ratio as a percentage
    pub fn hit_ratio(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            (self.valid_entries as f64 / self.total_entries as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_credentials_expiration() {
        let future_time = Utc::now() + chrono::Duration::hours(1);
        let past_time = Utc::now() - chrono::Duration::hours(1);
        let soon_time = Utc::now() + chrono::Duration::minutes(2); // Within 5-minute buffer

        let valid_creds = AccountCredentials {
            account_id: "123456789012".to_string(),
            role_name: "awsdash".to_string(),
            access_key_id: "AKIA...".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: "token".to_string(),
            expiration: future_time,
        };

        let expired_creds = AccountCredentials {
            account_id: "123456789012".to_string(),
            role_name: "awsdash".to_string(),
            access_key_id: "AKIA...".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: "token".to_string(),
            expiration: past_time,
        };

        let soon_expired_creds = AccountCredentials {
            account_id: "123456789012".to_string(),
            role_name: "awsdash".to_string(),
            access_key_id: "AKIA...".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: "token".to_string(),
            expiration: soon_time,
        };

        assert!(!valid_creds.is_expired());
        assert!(expired_creds.is_expired());
        assert!(soon_expired_creds.is_expired()); // Should be expired due to 5-minute buffer
    }

    #[test]
    fn test_cache_stats() {
        let stats = CredentialCacheStats {
            total_entries: 10,
            valid_entries: 8,
            expired_entries: 2,
        };

        assert_eq!(stats.hit_ratio(), 80.0);

        let empty_stats = CredentialCacheStats {
            total_entries: 0,
            valid_entries: 0,
            expired_entries: 0,
        };

        assert_eq!(empty_stats.hit_ratio(), 0.0);
    }
}
