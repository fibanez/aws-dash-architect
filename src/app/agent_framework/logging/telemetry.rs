//! Agent Telemetry Initialization
//!
//! This module handles pre-creation of CloudWatch log groups at application
//! startup to avoid the ~1 second timeout per agent during creation.
//!
//! ## Log Group Naming
//!
//! Log groups are named based on the IAM role name from Identity Center:
//! - Manager: `/aws/bedrock-agentcore/runtimes/{role_name}-manager`
//! - Worker: `/aws/bedrock-agentcore/runtimes/{role_name}-worker`
//!
//! This allows telemetry data to be organized by role/application.

#![warn(clippy::all, rust_2018_idioms)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Global flag indicating whether log groups have been initialized
static LOG_GROUPS_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Cached role name from the last initialization
static CACHED_ROLE_NAME: OnceLock<String> = OnceLock::new();

/// Get the agent ID for a manager agent
pub fn manager_agent_id(role_name: &str) -> String {
    format!("{}-manager", role_name)
}

/// Get the agent ID for a worker agent
pub fn worker_agent_id(role_name: &str) -> String {
    format!("{}-worker", role_name)
}

/// Check if log groups have been initialized
pub fn are_log_groups_initialized() -> bool {
    LOG_GROUPS_INITIALIZED.load(Ordering::SeqCst)
}

/// Get the cached role name if available
pub fn get_cached_role_name() -> Option<String> {
    CACHED_ROLE_NAME.get().cloned()
}

/// Initialize agent telemetry log groups
///
/// This function creates the CloudWatch log groups for manager and worker agents.
/// It should be called:
/// - At application startup when agent logging is enabled
/// - When the agent logging toggle is turned on
///
/// # Arguments
///
/// * `role_name` - The IAM role name from Identity Center (e.g., "awsdash")
/// * `region` - AWS region for CloudWatch (e.g., "us-east-1")
/// * `access_key` - AWS access key ID
/// * `secret_key` - AWS secret access key
/// * `session_token` - Optional session token for temporary credentials
///
/// # Returns
///
/// * `Ok(())` - Log groups were created or already exist
/// * `Err(String)` - Failed to create log groups
pub async fn initialize_log_groups(
    role_name: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
    session_token: Option<&str>,
) -> Result<(), String> {
    use aws_credential_types::Credentials;
    use stood::telemetry::{AgentLogGroup, LogGroupManager};

    tracing::info!(
        "Initializing agent telemetry log groups for role: {} in region: {}",
        role_name,
        region
    );

    // Create AWS credentials from provided values
    let credentials = Credentials::new(
        access_key,
        secret_key,
        session_token.map(|s| s.to_string()),
        None, // expiry
        "awsdash-telemetry-init",
    );

    // Build SDK config with explicit credentials
    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_types::region::Region::new(region.to_string()))
        .credentials_provider(credentials)
        .load()
        .await;

    // Create log group manager with our credentials
    let manager = LogGroupManager::from_sdk_config(&sdk_config, region);

    // Create manager log group
    let manager_id = manager_agent_id(role_name);
    let manager_config = AgentLogGroup::new(&manager_id);
    match manager.ensure_exists(&manager_config).await {
        Ok(created) => {
            if created {
                tracing::info!("Created log group: {}", manager_config.log_group_name());
            } else {
                tracing::debug!(
                    "Log group already exists: {}",
                    manager_config.log_group_name()
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to create manager log group: {}", e);
            // Continue - we'll try the worker log group
        }
    }

    // Create worker log group
    let worker_id = worker_agent_id(role_name);
    let worker_config = AgentLogGroup::new(&worker_id);
    match manager.ensure_exists(&worker_config).await {
        Ok(created) => {
            if created {
                tracing::info!("Created log group: {}", worker_config.log_group_name());
            } else {
                tracing::debug!(
                    "Log group already exists: {}",
                    worker_config.log_group_name()
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to create worker log group: {}", e);
        }
    }

    // Cache the role name and mark as initialized
    let _ = CACHED_ROLE_NAME.set(role_name.to_string());
    LOG_GROUPS_INITIALIZED.store(true, Ordering::SeqCst);

    tracing::info!("Agent telemetry log groups initialized successfully");
    Ok(())
}

/// Reset the initialization state
///
/// This should be called when the role name changes or credentials expire.
pub fn reset_initialization() {
    LOG_GROUPS_INITIALIZED.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_generation() {
        assert_eq!(manager_agent_id("awsdash"), "awsdash-manager");
        assert_eq!(worker_agent_id("awsdash"), "awsdash-worker");
        assert_eq!(manager_agent_id("my-app"), "my-app-manager");
        assert_eq!(worker_agent_id("my-app"), "my-app-worker");
    }

    #[test]
    fn test_initialization_state() {
        // Initially not initialized
        reset_initialization();
        assert!(!are_log_groups_initialized());
    }
}
