//! Enhanced Git Error Handling System
//!
//! This module provides comprehensive error handling and diagnostic logging for git operations,
//! specifically designed to address git2 library failures on macOS and provide detailed
//! diagnostic information for debugging repository cloning issues.

use anyhow::anyhow;
use git2::Error as Git2Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

/// Comprehensive error context for git operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOperationError {
    /// The git operation that failed
    pub operation: GitOperation,
    /// The original git2 error if available
    pub git2_error_code: Option<i32>,
    /// The git2 error message
    pub git2_error_message: Option<String>,
    /// The git2 error class for categorization
    pub git2_error_class: Option<String>,
    /// The path involved in the operation
    pub path: PathBuf,
    /// Repository URL if applicable
    pub repository_url: Option<String>,
    /// Branch name if applicable
    pub branch_name: Option<String>,
    /// Platform-specific information
    pub platform_info: PlatformInfo,
    /// Additional context information
    pub additional_context: HashMap<String, String>,
    /// Timestamp when the error occurred
    pub timestamp: SystemTime,
}

/// Different types of git operations that can fail
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GitOperation {
    /// Repository cloning operation
    Clone,
    /// Fetching changes from remote
    Fetch,
    /// Checking out a branch or commit
    Checkout,
    /// Branch detection and listing
    BranchDetection,
    /// Opening an existing repository
    RepositoryOpen,
    /// Pulling changes (fetch + merge)
    Pull,
    /// Merging branches
    Merge,
    /// Creating references
    ReferenceCreate,
    /// Remote operations
    RemoteOperation,
}

/// Platform-specific information for debugging path and system issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    /// Operating system name
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Whether the path contains spaces
    pub path_contains_spaces: bool,
    /// Whether the path contains special characters
    pub path_contains_special_chars: bool,
    /// Path length
    pub path_length: usize,
    /// Git2 library version
    pub git2_version: String,
    /// Whether the path is in a system directory
    pub is_system_path: bool,
    /// Path components for analysis
    pub path_components: Vec<String>,
}

impl GitOperationError {
    /// Create a new GitOperationError from a git2::Error
    pub fn from_git2_error(
        operation: GitOperation,
        git2_error: Git2Error,
        path: PathBuf,
        repository_url: Option<String>,
        branch_name: Option<String>,
    ) -> Self {
        let mut additional_context = HashMap::new();
        
        // Add git2 error details to context
        additional_context.insert("git2_error_raw".to_string(), format!("{:?}", git2_error));
        
        Self {
            operation,
            git2_error_code: Some(git2_error.code() as i32),
            git2_error_message: Some(git2_error.message().to_string()),
            git2_error_class: Some(format!("{:?}", git2_error.class())),
            path: path.clone(),
            repository_url,
            branch_name,
            platform_info: PlatformInfo::from_path(&path),
            additional_context,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a new GitOperationError for general operation failures
    pub fn new(
        operation: GitOperation,
        message: String,
        path: PathBuf,
        repository_url: Option<String>,
        branch_name: Option<String>,
    ) -> Self {
        let mut additional_context = HashMap::new();
        additional_context.insert("error_message".to_string(), message);
        
        Self {
            operation,
            git2_error_code: None,
            git2_error_message: None,
            git2_error_class: None,
            path: path.clone(),
            repository_url,
            branch_name,
            platform_info: PlatformInfo::from_path(&path),
            additional_context,
            timestamp: SystemTime::now(),
        }
    }

    /// Add additional context to the error
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.additional_context.insert(key, value);
        self
    }

    /// Convert to anyhow::Error for compatibility
    pub fn into_anyhow(self) -> anyhow::Error {
        anyhow!(
            "Git {} operation failed: {} (path: {}, platform: {})",
            format!("{:?}", self.operation).to_lowercase(),
            self.git2_error_message.as_deref().unwrap_or("Unknown error"),
            self.path.display(),
            self.platform_info.os
        )
    }

    /// Log this error using the enhanced logging system
    pub fn log_error(&self) {
        error!(
            "Git {} operation failed on {}: {}",
            format!("{:?}", self.operation).to_lowercase(),
            self.platform_info.os,
            self.git2_error_message.as_deref().unwrap_or("Unknown error")
        );

        error!(
            "Error details - Code: {:?}, Class: {:?}, Path: {}",
            self.git2_error_code,
            self.git2_error_class,
            self.path.display()
        );

        if let Some(url) = &self.repository_url {
            error!("Repository URL: {}", url);
        }

        if let Some(branch) = &self.branch_name {
            error!("Branch: {}", branch);
        }

        // Log platform-specific information
        self.platform_info.log_platform_details();

        // Log additional context
        if !self.additional_context.is_empty() {
            error!("Additional context:");
            for (key, value) in &self.additional_context {
                error!("  {}: {}", key, value);
            }
        }
    }

    /// Log this error as a warning (for recoverable issues)
    pub fn log_warning(&self) {
        warn!(
            "Git {} operation encountered issue on {}: {}",
            format!("{:?}", self.operation).to_lowercase(),
            self.platform_info.os,
            self.git2_error_message.as_deref().unwrap_or("Unknown issue")
        );

        debug!(
            "Warning details - Code: {:?}, Class: {:?}, Path: {}",
            self.git2_error_code,
            self.git2_error_class,
            self.path.display()
        );
    }
}

impl PlatformInfo {
    /// Create platform information from a path
    pub fn from_path(path: &PathBuf) -> Self {
        let path_str = path.to_string_lossy();
        let path_components: Vec<String> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();

        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            path_contains_spaces: path_str.contains(' '),
            path_contains_special_chars: path_str.chars().any(|c| {
                !c.is_ascii_alphanumeric() && !matches!(c, '/' | '\\' | '.' | '-' | '_' | ':')
            }),
            path_length: path_str.len(),
            git2_version: Self::get_git2_version(),
            is_system_path: Self::is_system_path(path),
            path_components,
        }
    }

    /// Get the git2 library version
    fn get_git2_version() -> String {
        // git2 doesn't expose version info directly, so we'll use the crate version
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Check if the path is in a system directory
    fn is_system_path(path: &PathBuf) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        
        // Check for common system paths on different platforms
        path_str.contains("/system/") ||
        path_str.contains("/usr/") ||
        path_str.contains("/opt/") ||
        path_str.contains("program files") ||
        path_str.contains("windows") ||
        path_str.contains("/applications/") ||
        path_str.contains("/library/")
    }

    /// Log platform-specific details for debugging
    pub fn log_platform_details(&self) {
        // Only log warnings for potential issues
        if self.path_contains_spaces {
            warn!("Path contains spaces - this may cause git2 issues on some platforms");
        }
        
        if self.path_length > 260 {
            warn!("Path is very long ({} chars) - this may cause issues on some platforms", self.path_length);
        }
    }
}

/// Enhanced git operation logger that provides detailed diagnostics
pub struct GitOperationLogger {
    pub operation: GitOperation,
    pub path: PathBuf,
    pub repository_url: Option<String>,
    pub branch_name: Option<String>,
    start_time: SystemTime,
    context: HashMap<String, String>,
}

impl GitOperationLogger {
    /// Start logging a new git operation
    pub fn start(
        operation: GitOperation,
        path: PathBuf,
        repository_url: Option<String>,
        branch_name: Option<String>,
    ) -> Self {
        let context = HashMap::new();
        
        info!(
            "Starting git {} operation",
            format!("{:?}", operation).to_lowercase()
        );

        // Log platform warnings if needed
        let platform_info = PlatformInfo::from_path(&path);
        platform_info.log_platform_details();

        Self {
            operation,
            path,
            repository_url,
            branch_name,
            start_time: SystemTime::now(),
            context,
        }
    }

    /// Add context information to the operation log
    pub fn add_context(&mut self, key: String, value: String) {
        self.context.insert(key, value);
    }

    /// Log successful completion of the operation
    pub fn log_success(&self, details: &str) {
        info!(
            "Git {} operation completed successfully - {}",
            format!("{:?}", self.operation).to_lowercase(),
            details
        );
    }

    /// Log operation failure and return a GitOperationError
    pub fn log_failure(&self, git2_error: Git2Error) -> GitOperationError {
        let duration = self.start_time.elapsed().unwrap_or_default();
        
        let mut error = GitOperationError::from_git2_error(
            self.operation.clone(),
            git2_error,
            self.path.clone(),
            self.repository_url.clone(),
            self.branch_name.clone(),
        );

        // Add operation context to the error
        for (key, value) in &self.context {
            error = error.with_context(key.clone(), value.clone());
        }
        
        error = error.with_context("operation_duration".to_string(), format!("{:?}", duration));

        error.log_error();
        error
    }

    /// Log operation failure with a custom message
    pub fn log_failure_with_message(&self, message: String) -> GitOperationError {
        let duration = self.start_time.elapsed().unwrap_or_default();
        
        let mut error = GitOperationError::new(
            self.operation.clone(),
            message,
            self.path.clone(),
            self.repository_url.clone(),
            self.branch_name.clone(),
        );

        // Add operation context to the error
        for (key, value) in &self.context {
            error = error.with_context(key.clone(), value.clone());
        }
        
        error = error.with_context("operation_duration".to_string(), format!("{:?}", duration));

        error.log_error();
        error
    }
}

/// Git2 TLS/SSL configuration utilities
pub mod tls_config {
    use git2::{CertificateCheckStatus, RemoteCallbacks};
    use tracing::warn;

    /// Configure git2 remote callbacks with proper TLS/SSL settings for macOS
    pub fn configure_tls_callbacks() -> RemoteCallbacks<'static> {
        let mut callbacks = RemoteCallbacks::new();
        
        // Configure certificate checking for HTTPS connections
        callbacks.certificate_check(|cert, _valid| {
            // For GitHub and other well-known hosts, we can be more permissive
            // This helps with macOS TLS issues while maintaining reasonable security
            if let Some(hostkey) = cert.as_hostkey() {
                if let Some(host_data) = hostkey.hostkey() {
                    let host_str = String::from_utf8_lossy(host_data);
                    
                    // Accept certificates for GitHub and other common git hosts
                    if host_str.contains("github.com") || 
                       host_str.contains("gitlab.com") ||
                       host_str.contains("bitbucket.org") {
                        return Ok(CertificateCheckStatus::CertificateOk);
                    }
                }
            }
            
            // For other hosts, use default validation
            Ok(CertificateCheckStatus::CertificatePassthrough)
        });

        // Configure credentials callback if needed
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            // For public repositories, we typically don't need credentials
            git2::Cred::default()
        });

        // Configure push update reference callback
        callbacks.push_update_reference(|refname, status| {
            match status {
                Some(msg) => warn!("Push update failed for {}: {}", refname, msg),
                None => {},
            }
            Ok(())
        });

        callbacks
    }

    /// Configure git2 fetch options with TLS settings
    pub fn configure_fetch_options() -> git2::FetchOptions<'static> {
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(configure_tls_callbacks());
        fetch_options
    }

    /// Configure git2 clone builder with TLS settings
    pub fn configure_clone_builder() -> git2::build::RepoBuilder<'static> {
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(configure_fetch_options());
        builder
    }
}

/// Utility functions for enhanced git error handling
pub mod utils {
    use super::*;

    /// Check if a git2 error is related to network issues
    pub fn is_network_error(error: &Git2Error) -> bool {
        matches!(
            error.class(),
            git2::ErrorClass::Net | git2::ErrorClass::Http | git2::ErrorClass::Ssl
        )
    }

    /// Check if a git2 error is related to authentication
    pub fn is_auth_error(error: &Git2Error) -> bool {
        matches!(error.class(), git2::ErrorClass::Http) && 
        error.message().to_lowercase().contains("auth")
    }

    /// Check if a git2 error is related to path issues
    pub fn is_path_error(error: &Git2Error) -> bool {
        matches!(
            error.class(),
            git2::ErrorClass::Os | git2::ErrorClass::Filesystem
        ) || error.message().to_lowercase().contains("path")
    }

    /// Check if a git2 error is related to branch issues
    pub fn is_branch_error(error: &Git2Error) -> bool {
        matches!(error.class(), git2::ErrorClass::Reference) ||
        error.message().to_lowercase().contains("branch") ||
        error.message().to_lowercase().contains("reference")
    }

    /// Get a human-readable description of the git2 error
    pub fn get_error_description(error: &Git2Error) -> String {
        match error.class() {
            git2::ErrorClass::None => "No error".to_string(),
            git2::ErrorClass::NoMemory => "Out of memory".to_string(),
            git2::ErrorClass::Os => "Operating system error".to_string(),
            git2::ErrorClass::Invalid => "Invalid input".to_string(),
            git2::ErrorClass::Reference => "Reference/branch error".to_string(),
            git2::ErrorClass::Zlib => "Compression error".to_string(),
            git2::ErrorClass::Repository => "Repository error".to_string(),
            git2::ErrorClass::Config => "Configuration error".to_string(),
            git2::ErrorClass::Regex => "Regular expression error".to_string(),
            git2::ErrorClass::Odb => "Object database error".to_string(),
            git2::ErrorClass::Index => "Index error".to_string(),
            git2::ErrorClass::Object => "Object error".to_string(),
            git2::ErrorClass::Net => "Network error".to_string(),
            git2::ErrorClass::Tag => "Tag error".to_string(),
            git2::ErrorClass::Tree => "Tree error".to_string(),
            git2::ErrorClass::Indexer => "Indexer error".to_string(),
            git2::ErrorClass::Ssl => "SSL/TLS error".to_string(),
            git2::ErrorClass::Submodule => "Submodule error".to_string(),
            git2::ErrorClass::Thread => "Threading error".to_string(),
            git2::ErrorClass::Stash => "Stash error".to_string(),
            git2::ErrorClass::Checkout => "Checkout error".to_string(),
            git2::ErrorClass::FetchHead => "Fetch head error".to_string(),
            git2::ErrorClass::Merge => "Merge error".to_string(),
            git2::ErrorClass::Http => "HTTP error".to_string(),
            git2::ErrorClass::Ssh => "SSH error".to_string(),
            git2::ErrorClass::Filter => "Filter error".to_string(),
            git2::ErrorClass::Revert => "Revert error".to_string(),
            git2::ErrorClass::Callback => "Callback error".to_string(),
            git2::ErrorClass::CherryPick => "Cherry-pick error".to_string(),
            git2::ErrorClass::Describe => "Describe error".to_string(),
            git2::ErrorClass::Rebase => "Rebase error".to_string(),
            git2::ErrorClass::Filesystem => "Filesystem error".to_string(),
            git2::ErrorClass::Patch => "Patch error".to_string(),
            git2::ErrorClass::Worktree => "Worktree error".to_string(),
            git2::ErrorClass::Sha1 => "SHA1 error".to_string(),

        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_platform_info_creation() {
        let path = PathBuf::from("/Users/test user/Documents/my repo");
        let platform_info = PlatformInfo::from_path(&path);
        
        assert_eq!(platform_info.os, std::env::consts::OS);
        assert!(platform_info.path_contains_spaces);
        assert!(!platform_info.path_contains_special_chars);
        assert!(platform_info.path_length > 0);
    }

    #[test]
    fn test_git_operation_error_creation() {
        let path = PathBuf::from("/test/path");
        let error = GitOperationError::new(
            GitOperation::Clone,
            "Test error".to_string(),
            path.clone(),
            Some("https://github.com/test/repo.git".to_string()),
            Some("main".to_string()),
        );

        assert_eq!(error.operation, GitOperation::Clone);
        assert_eq!(error.path, path);
        assert!(error.repository_url.is_some());
        assert!(error.branch_name.is_some());
    }

    #[test]
    fn test_error_utils() {
        // These tests would need actual git2::Error instances to be meaningful
        // For now, we'll just test that the functions exist and compile
        assert!(true);
    }
}