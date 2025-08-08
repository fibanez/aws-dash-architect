//! Dynamic Branch Detection System
//!
//! This module provides comprehensive branch detection capabilities for git repositories,
//! designed to replace hardcoded branch assumptions with dynamic detection of the
//! default branch and fallback mechanisms for common branch naming conventions.

use anyhow::{anyhow, Result};
use git2::{Repository, Direction};
use tracing::{error, info, warn};

use crate::app::git_error_handling::{
    GitOperation, GitOperationError, GitOperationLogger
};

/// Information about a git branch
#[derive(Debug, Clone, PartialEq)]
pub struct BranchInfo {
    /// Branch name (without refs/heads/ prefix)
    pub name: String,
    /// Whether this is the default branch
    pub is_default: bool,
    /// Commit hash this branch points to
    pub commit_hash: String,
    /// Whether this is a remote branch
    pub is_remote: bool,
}

/// Dynamic branch detector for git repositories
pub struct BranchDetector {
    /// Repository URL to query
    repo_url: String,
    /// Common branch names to try as fallbacks
    fallback_branches: Vec<String>,
}

impl BranchDetector {
    /// Create a new branch detector for the given repository URL
    pub fn new(repo_url: String) -> Self {
        Self {
            repo_url,
            fallback_branches: vec![
                "main".to_string(),
                "master".to_string(),
                "develop".to_string(),
                "dev".to_string(),
                "trunk".to_string(),
            ],
        }
    }

    /// Create a new branch detector with custom fallback branches
    pub fn with_fallbacks(repo_url: String, fallback_branches: Vec<String>) -> Self {
        Self {
            repo_url,
            fallback_branches,
        }
    }

    /// Detect the default branch for the repository
    pub fn detect_default_branch(&self) -> Result<String> {
        // First, try to detect the default branch using remote HEAD reference
        match self.detect_default_branch_from_remote() {
            Ok(branch_name) => {
                info!("Successfully detected default branch: {}", branch_name);
                return Ok(branch_name);
            }
            Err(e) => {
                warn!("Failed to detect default branch from remote HEAD: {}", e);
            }
        }

        // If remote HEAD detection fails, try fallback branches
        match self.detect_branch_from_fallbacks() {
            Ok(branch_name) => {
                info!("Successfully detected branch using fallback: {}", branch_name);
                Ok(branch_name)
            }
            Err(e) => {
                error!("Failed to detect any valid branch: {}", e);
                Err(anyhow!("Failed to detect any valid branch: {}", e))
            }
        }
    }

    /// Get all available branches from the remote repository
    pub fn get_available_branches(&self) -> Result<Vec<BranchInfo>> {
        let mut logger = GitOperationLogger::start(
            GitOperation::BranchDetection,
            std::env::temp_dir(),
            Some(self.repo_url.clone()),
            None,
        );

        logger.add_context("operation".to_string(), "list_remote_branches".to_string());
        info!("Listing available branches for repository: {}", self.repo_url);

        // Create a temporary repository to list remote branches
        let temp_dir = tempfile::tempdir()
            .map_err(|e| anyhow!("Failed to create temporary directory: {}", e))?;
        
        let temp_repo_path = temp_dir.path().join("temp_repo");
        logger.add_context("temp_repo_path".to_string(), temp_repo_path.display().to_string());

        // Initialize a bare repository for remote operations
        let repo = Repository::init_bare(&temp_repo_path)
            .map_err(|e| {
                let git_error = GitOperationError::from_git2_error(
                    GitOperation::RepositoryOpen,
                    e,
                    temp_repo_path.clone(),
                    Some(self.repo_url.clone()),
                    None,
                );
                git_error.log_error();
                git_error.into_anyhow()
            })?;

        // Add remote
        let mut remote = repo.remote("origin", &self.repo_url)
            .map_err(|e| {
                let git_error = GitOperationError::from_git2_error(
                    GitOperation::RemoteOperation,
                    e,
                    temp_repo_path.clone(),
                    Some(self.repo_url.clone()),
                    None,
                );
                git_error.log_error();
                git_error.into_anyhow()
            })?;

        // Set up callbacks for listing references
        let mut branches = Vec::new();
        let mut default_branch: Option<String> = None;

        {
            let branches_ref = &mut branches;
            let default_branch_ref = &mut default_branch;
            
            // Use TLS-configured callbacks from git_error_handling module
            use crate::app::git_error_handling::tls_config;
            let mut callbacks = tls_config::configure_tls_callbacks();
            
            // Add branch detection logic on top of TLS configuration
            callbacks.update_tips(|refname, _old_oid, new_oid| {
                // Parse branch references
                if let Some(branch_name) = refname.strip_prefix("refs/heads/") {
                    branches_ref.push(BranchInfo {
                        name: branch_name.to_string(),
                        is_default: false, // Will be updated later
                        commit_hash: new_oid.to_string(),
                        is_remote: true,
                    });
                }
                
                true
            });

            // Connect to remote and list references
            remote.connect_auth(Direction::Fetch, Some(callbacks), None)
                .map_err(|e| {
                    let git_error = GitOperationError::from_git2_error(
                        GitOperation::RemoteOperation,
                        e,
                        temp_repo_path.clone(),
                        Some(self.repo_url.clone()),
                        None,
                    );
                    git_error.log_error();
                    git_error.into_anyhow()
                })?;

            // List remote references
            let refs = remote.list()
                .map_err(|e| {
                    let git_error = GitOperationError::from_git2_error(
                        GitOperation::BranchDetection,
                        e,
                        temp_repo_path.clone(),
                        Some(self.repo_url.clone()),
                        None,
                    );
                    git_error.log_error();
                    git_error.into_anyhow()
                })?;

            // Process references to find branches and default branch
            for remote_head in refs {
                let refname = remote_head.name();
                let oid = remote_head.oid();
                
                if let Some(branch_name) = refname.strip_prefix("refs/heads/") {
                    // Check if this branch already exists in our list
                    if let Some(existing_branch) = branches_ref.iter_mut().find(|b| b.name == branch_name) {
                        existing_branch.commit_hash = oid.to_string();
                    } else {
                        branches_ref.push(BranchInfo {
                            name: branch_name.to_string(),
                            is_default: false,
                            commit_hash: oid.to_string(),
                            is_remote: true,
                        });
                    }
                }
                
                // Check if this is the symbolic HEAD reference
                if refname == "HEAD" {
                    // Try to find which branch HEAD points to
                    for branch in branches_ref.iter_mut() {
                        if branch.commit_hash == oid.to_string() {
                            branch.is_default = true;
                            *default_branch_ref = Some(branch.name.clone());
                            break;
                        }
                    }
                }
            }

            let _ = remote.disconnect();
        }

        // If we couldn't determine the default branch from HEAD, use heuristics
        if default_branch.is_none() && !branches.is_empty() {
            // Try to find a branch matching our fallback list
            for fallback in &self.fallback_branches {
                if let Some(branch) = branches.iter_mut().find(|b| b.name == *fallback) {
                    branch.is_default = true;
                    default_branch = Some(branch.name.clone());
                    info!("Using fallback branch as default: {}", branch.name);
                    break;
                }
            }
            
            // If still no default, mark the first branch as default
            if default_branch.is_none() && !branches.is_empty() {
                branches[0].is_default = true;
                default_branch = Some(branches[0].name.clone());
            }
        }

        if branches.is_empty() {
            return Err(anyhow!("No branches found in remote repository"));
        }

        Ok(branches)
    }

    /// Validate that a specific branch exists in the repository
    pub fn validate_branch_exists(&self, branch_name: &str) -> Result<bool> {
        match self.get_available_branches() {
            Ok(branches) => {
                let exists = branches.iter().any(|b| b.name == branch_name);
                if !exists {
                    warn!("Branch '{}' does not exist in repository", branch_name);
                }
                Ok(exists)
            }
            Err(e) => {
                error!("Failed to validate branch existence: {}", e);
                Err(e)
            }
        }
    }

    /// Detect default branch using remote HEAD reference
    fn detect_default_branch_from_remote(&self) -> Result<String> {
        // Get all branches and find the default one
        let branches = self.get_available_branches()?;
        
        // Look for the branch marked as default
        if let Some(default_branch) = branches.iter().find(|b| b.is_default) {
            return Ok(default_branch.name.clone());
        }
        
        Err(anyhow!("Could not determine default branch from remote HEAD"))
    }

    /// Detect branch using fallback branch names
    fn detect_branch_from_fallbacks(&self) -> Result<String> {
        // Get all available branches
        let branches = self.get_available_branches()?;
        let branch_names: Vec<String> = branches.iter().map(|b| b.name.clone()).collect();
        
        // Try each fallback branch in order
        for fallback in &self.fallback_branches {
            if branch_names.contains(fallback) {
                return Ok(fallback.clone());
            }
        }
        
        // If no fallback matches, return the first available branch
        if let Some(first_branch) = branches.first() {
            return Ok(first_branch.name.clone());
        }
        
        Err(anyhow!("No valid branches found in repository"))
    }
}

/// Utility functions for branch detection
pub mod utils {

    /// Check if a branch name is likely to be a default branch
    pub fn is_likely_default_branch(branch_name: &str) -> bool {
        matches!(branch_name, "main" | "master" | "trunk" | "default")
    }

    /// Get a list of common default branch names
    pub fn get_common_default_branches() -> Vec<String> {
        vec![
            "main".to_string(),
            "master".to_string(),
            "trunk".to_string(),
            "default".to_string(),
        ]
    }

    /// Get a list of common development branch names
    pub fn get_common_dev_branches() -> Vec<String> {
        vec![
            "develop".to_string(),
            "dev".to_string(),
            "development".to_string(),
            "devel".to_string(),
        ]
    }

    /// Create a comprehensive list of fallback branches
    pub fn get_comprehensive_fallback_branches() -> Vec<String> {
        let mut branches = get_common_default_branches();
        branches.extend(get_common_dev_branches());
        branches
    }
}

#[cfg(test)]
mod tests {

    use super::{BranchDetector, BranchInfo, utils};

    #[test]
    fn test_branch_detector_creation() {
        let detector = BranchDetector::new("https://github.com/test/repo.git".to_string());
        assert_eq!(detector.repo_url, "https://github.com/test/repo.git");
        assert!(!detector.fallback_branches.is_empty());
        assert!(detector.fallback_branches.contains(&"main".to_string()));
        assert!(detector.fallback_branches.contains(&"master".to_string()));
    }

    #[test]
    fn test_branch_detector_with_custom_fallbacks() {
        let custom_fallbacks = vec!["custom".to_string(), "branch".to_string()];
        let detector = BranchDetector::with_fallbacks(
            "https://github.com/test/repo.git".to_string(),
            custom_fallbacks.clone()
        );
        assert_eq!(detector.fallback_branches, custom_fallbacks);
    }

    #[test]
    fn test_branch_info_creation() {
        let branch = BranchInfo {
            name: "main".to_string(),
            is_default: true,
            commit_hash: "abc123".to_string(),
            is_remote: true,
        };
        
        assert_eq!(branch.name, "main");
        assert!(branch.is_default);
        assert_eq!(branch.commit_hash, "abc123");
        assert!(branch.is_remote);
    }

    #[test]
    fn test_utils_functions() {
        assert!(utils::is_likely_default_branch("main"));
        assert!(utils::is_likely_default_branch("master"));
        assert!(!utils::is_likely_default_branch("feature-branch"));
        
        let default_branches = utils::get_common_default_branches();
        assert!(default_branches.contains(&"main".to_string()));
        assert!(default_branches.contains(&"master".to_string()));
        
        let dev_branches = utils::get_common_dev_branches();
        assert!(dev_branches.contains(&"develop".to_string()));
        assert!(dev_branches.contains(&"dev".to_string()));
        
        let comprehensive = utils::get_comprehensive_fallback_branches();
        assert!(comprehensive.len() > default_branches.len());
    }
}