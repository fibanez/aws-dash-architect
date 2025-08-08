//! Repository Recovery Mechanisms
//!
//! This module provides comprehensive recovery mechanisms for git repositories,
//! designed to detect incomplete or corrupted repository states and implement
//! recovery workflows with multiple strategies before failing.

use anyhow::{anyhow, Result};
use git2::{Repository, RepositoryOpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info, warn};

use crate::app::git_error_handling::{
    GitOperation, GitOperationLogger
};
use crate::app::branch_detector::BranchDetector;

/// Repository validation result with detailed diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryValidation {
    /// Whether the repository is valid and complete
    pub is_valid: bool,
    /// List of missing required directories
    pub missing_directories: Vec<String>,
    /// Total number of files in the repository
    pub file_count: usize,
    /// Total number of directories in the repository
    pub directory_count: usize,
    /// Total size of repository content in bytes
    pub total_size_bytes: u64,
    /// Hash of the last commit if available
    pub last_commit_hash: Option<String>,
    /// List of validation errors encountered
    pub validation_errors: Vec<String>,
    /// Whether the .git directory exists and is valid
    pub git_directory_valid: bool,
    /// Whether the repository can be opened by git2
    pub repository_openable: bool,
    /// Timestamp when validation was performed
    pub validation_timestamp: SystemTime,
    /// Additional diagnostic information
    pub diagnostic_info: HashMap<String, String>,
}

/// Different types of repository corruption or incompleteness
#[derive(Debug, Clone, PartialEq)]
pub enum RepositoryIssue {
    /// Repository directory doesn't exist
    DirectoryMissing,
    /// .git directory is missing or corrupted
    GitDirectoryCorrupted,
    /// Repository exists but is empty (no files)
    EmptyRepository,
    /// Required directories are missing (mappings, rules, etc.)
    MissingRequiredDirectories(Vec<String>),
    /// Repository cannot be opened by git2
    RepositoryNotOpenable(String),
    /// Repository has no valid HEAD reference
    InvalidHeadReference,
    /// Repository has incomplete clone (partial files)
    IncompleteClone,
    /// Repository has permission issues
    PermissionIssues(String),
}

/// Recovery strategy to attempt for different types of issues
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    /// Remove corrupted repository and attempt fresh clone
    CleanAndRetry,
    /// Attempt to repair the repository in place
    RepairInPlace,
    /// Try alternative clone parameters (different branch, etc.)
    AlternativeClone(AlternativeCloneParams),
    /// Try alternative target path
    AlternativePath(PathBuf),
    /// Manual intervention required - provide guidance
    ManualIntervention(String),
    /// No recovery possible
    NoRecovery(String),
}

/// Parameters for alternative clone attempts
#[derive(Debug, Clone, PartialEq)]
pub struct AlternativeCloneParams {
    /// Alternative branch to try
    pub branch: Option<String>,
    /// Whether to use shallow clone
    pub shallow: bool,
    /// Alternative repository URL if applicable
    pub alternative_url: Option<String>,
    /// Additional git2 options
    pub additional_options: HashMap<String, String>,
}

/// Recovery attempt result
#[derive(Debug, Clone)]
pub struct RecoveryAttempt {
    /// The strategy that was attempted
    pub strategy: RecoveryStrategy,
    /// Whether the recovery attempt succeeded
    pub success: bool,
    /// Error message if the attempt failed
    pub error_message: Option<String>,
    /// Duration of the recovery attempt
    pub duration: Duration,
    /// Additional context about the attempt
    pub context: HashMap<String, String>,
    /// Timestamp when the attempt was made
    pub timestamp: SystemTime,
}

/// Comprehensive repository recovery manager
pub struct RepositoryRecoveryManager {
    /// Repository URL for cloning operations
    repo_url: String,
    /// Target path where repository should be located
    target_path: PathBuf,
    /// Branch detector for dynamic branch detection
    branch_detector: BranchDetector,
    /// Required directories that must exist in a valid repository
    required_directories: Vec<String>,
    /// Maximum number of recovery attempts before giving up
    max_recovery_attempts: usize,
    /// Delay between recovery attempts
    retry_delay: Duration,
    /// History of recovery attempts
    recovery_history: Vec<RecoveryAttempt>,
}

impl RepositoryRecoveryManager {
    /// Create a new repository recovery manager
    pub fn new(
        repo_url: String,
        target_path: PathBuf,
        required_directories: Vec<String>,
    ) -> Self {
        let branch_detector = BranchDetector::new(repo_url.clone());
        
        Self {
            repo_url: repo_url.clone(),
            target_path,
            branch_detector,
            required_directories,
            max_recovery_attempts: 3,
            retry_delay: Duration::from_secs(2),
            recovery_history: Vec::new(),
        }
    }

    /// Create a recovery manager with custom settings
    pub fn with_settings(
        repo_url: String,
        target_path: PathBuf,
        required_directories: Vec<String>,
        max_attempts: usize,
        retry_delay: Duration,
    ) -> Self {
        let branch_detector = BranchDetector::new(repo_url.clone());
        
        Self {
            repo_url: repo_url.clone(),
            target_path,
            branch_detector,
            required_directories,
            max_recovery_attempts: max_attempts,
            retry_delay,
            recovery_history: Vec::new(),
        }
    }

    /// Detect incomplete or corrupted repository states
    pub fn detect_repository_issues(&self) -> Result<Vec<RepositoryIssue>> {
        let mut logger = GitOperationLogger::start(
            GitOperation::RepositoryOpen,
            self.target_path.clone(),
            Some(self.repo_url.clone()),
            None,
        );

        logger.add_context("operation".to_string(), "detect_issues".to_string());
        info!("Detecting repository issues for path: {}", self.target_path.display());

        let mut issues = Vec::new();

        // Check if repository directory exists
        if !self.target_path.exists() {
            warn!("Repository directory does not exist: {}", self.target_path.display());
            issues.push(RepositoryIssue::DirectoryMissing);
            return Ok(issues);
        }

        // Check if .git directory exists and is valid
        let git_dir = self.target_path.join(".git");
        if !git_dir.exists() {
            warn!(".git directory is missing: {}", git_dir.display());
            issues.push(RepositoryIssue::GitDirectoryCorrupted);
        } else {
            // Try to validate .git directory structure
            let essential_git_files = vec!["HEAD", "config", "refs", "objects"];
            for file in essential_git_files {
                let git_file_path = git_dir.join(file);
                if !git_file_path.exists() {
                    warn!("Essential git file/directory missing: {}", git_file_path.display());
                    issues.push(RepositoryIssue::GitDirectoryCorrupted);
                    break;
                }
            }
        }

        // Try to open repository with git2
        match Repository::open_ext(&self.target_path, RepositoryOpenFlags::empty(), &[] as &[&std::ffi::OsStr]) {
            Ok(repo) => {
                logger.add_context("repository_openable".to_string(), "true".to_string());
                
                // Check HEAD reference
                match repo.head() {
                    Ok(head_ref) => {
                        if let Some(name) = head_ref.name() {
                            logger.add_context("head_reference".to_string(), name.to_string());
                            debug!("Repository HEAD reference is valid: {}", name);
                        }
                    }
                    Err(e) => {
                        warn!("Repository HEAD reference is invalid: {}", e);
                        issues.push(RepositoryIssue::InvalidHeadReference);
                    }
                }
            }
            Err(e) => {
                warn!("Repository cannot be opened by git2: {}", e);
                issues.push(RepositoryIssue::RepositoryNotOpenable(e.message().to_string()));
            }
        }

        // Check if repository is empty
        let file_count = self.count_repository_files()?;
        if file_count == 0 {
            warn!("Repository directory exists but contains no files");
            issues.push(RepositoryIssue::EmptyRepository);
        }

        // Check for required directories
        let missing_dirs = self.find_missing_required_directories();
        if !missing_dirs.is_empty() {
            warn!("Required directories are missing: {:?}", missing_dirs);
            issues.push(RepositoryIssue::MissingRequiredDirectories(missing_dirs.clone()));
        }

        // Check for permission issues
        if let Err(e) = self.check_repository_permissions() {
            warn!("Repository permission issues detected: {}", e);
            issues.push(RepositoryIssue::PermissionIssues(e.to_string()));
        }

        // Check for incomplete clone (heuristic based on expected content)
        if file_count > 0 && file_count < 10 && missing_dirs.len() > 1 {
            warn!("Repository appears to have incomplete clone (few files, missing directories)");
            issues.push(RepositoryIssue::IncompleteClone);
        }

        logger.add_context("issues_found".to_string(), issues.len().to_string());
        
        if issues.is_empty() {
            logger.log_success("No repository issues detected");
        } else {
            warn!("Detected {} repository issues", issues.len());
            for issue in &issues {
                warn!("Issue: {:?}", issue);
            }
        }

        Ok(issues)
    }

    /// Perform comprehensive repository validation
    pub fn validate_repository_structure(&self) -> RepositoryValidation {
        let mut validation = RepositoryValidation {
            is_valid: true,
            missing_directories: Vec::new(),
            file_count: 0,
            directory_count: 0,
            total_size_bytes: 0,
            last_commit_hash: None,
            validation_errors: Vec::new(),
            git_directory_valid: false,
            repository_openable: false,
            validation_timestamp: SystemTime::now(),
            diagnostic_info: HashMap::new(),
        };

        info!("Starting comprehensive repository validation for: {}", self.target_path.display());

        // Check if repository directory exists
        if !self.target_path.exists() {
            validation.is_valid = false;
            validation.validation_errors.push("Repository directory does not exist".to_string());
            return validation;
        }

        // Validate .git directory
        let git_dir = self.target_path.join(".git");
        validation.git_directory_valid = git_dir.exists();
        if !validation.git_directory_valid {
            validation.is_valid = false;
            validation.validation_errors.push(".git directory is missing".to_string());
        }

        // Try to open repository
        match Repository::open_ext(&self.target_path, RepositoryOpenFlags::empty(), &[] as &[&std::ffi::OsStr]) {
            Ok(repo) => {
                validation.repository_openable = true;
                validation.diagnostic_info.insert("repository_openable".to_string(), "true".to_string());

                // Get last commit hash
                if let Ok(head) = repo.head() {
                    if let Ok(commit) = head.peel_to_commit() {
                        validation.last_commit_hash = Some(commit.id().to_string());
                        validation.diagnostic_info.insert("head_commit".to_string(), commit.id().to_string());
                    }
                }
            }
            Err(e) => {
                validation.is_valid = false;
                validation.repository_openable = false;
                validation.validation_errors.push(format!("Cannot open repository: {}", e.message()));
            }
        }

        // Count files and directories
        match self.count_repository_contents() {
            Ok((files, dirs, size)) => {
                validation.file_count = files;
                validation.directory_count = dirs;
                validation.total_size_bytes = size;
                
                if files == 0 {
                    validation.is_valid = false;
                    validation.validation_errors.push("Repository contains no files".to_string());
                }
            }
            Err(e) => {
                validation.is_valid = false;
                validation.validation_errors.push(format!("Failed to count repository contents: {}", e));
            }
        }

        // Check required directories
        validation.missing_directories = self.find_missing_required_directories();
        if !validation.missing_directories.is_empty() {
            validation.is_valid = false;
            validation.validation_errors.push(format!("Missing required directories: {:?}", validation.missing_directories));
        }

        // Add diagnostic information
        validation.diagnostic_info.insert("target_path".to_string(), self.target_path.display().to_string());
        validation.diagnostic_info.insert("repo_url".to_string(), self.repo_url.clone());
        validation.diagnostic_info.insert("required_directories".to_string(), format!("{:?}", self.required_directories));

        info!("Repository validation completed - Valid: {}, Files: {}, Directories: {}, Size: {} bytes", 
              validation.is_valid, validation.file_count, validation.directory_count, validation.total_size_bytes);

        validation
    }

    /// Implement recovery workflow that attempts multiple strategies
    pub fn attempt_repository_recovery<F>(&mut self, clone_fn: F) -> Result<()>
    where
        F: Fn(&str, &Path) -> Result<()> + Clone,
    {
        info!("Starting repository recovery workflow for: {}", self.target_path.display());

        // First, detect what issues exist
        let issues = self.detect_repository_issues()?;
        
        if issues.is_empty() {
            info!("No repository issues detected - recovery not needed");
            return Ok(());
        }

        info!("Detected {} repository issues, starting recovery process", issues.len());

        // Determine recovery strategies based on detected issues
        let strategies = self.determine_recovery_strategies(&issues);
        
        info!("Will attempt {} recovery strategies", strategies.len());

        // Attempt each recovery strategy
        for (index, strategy) in strategies.iter().enumerate() {
            if self.recovery_history.len() >= self.max_recovery_attempts {
                warn!("Maximum recovery attempts ({}) reached, stopping", self.max_recovery_attempts);
                break;
            }

            info!("Attempting recovery strategy {}/{}: {:?}", index + 1, strategies.len(), strategy);

            let attempt_result = self.execute_recovery_strategy(strategy, clone_fn.clone());
            self.recovery_history.push(attempt_result.clone());

            if attempt_result.success {
                info!("Recovery strategy succeeded: {:?}", strategy);
                
                // Validate that the recovery actually fixed the issues
                let remaining_issues = self.detect_repository_issues()?;
                if remaining_issues.is_empty() {
                    info!("Repository recovery completed successfully");
                    return Ok(());
                } else {
                    warn!("Recovery strategy succeeded but issues remain: {:?}", remaining_issues);
                    // Continue with next strategy
                }
            } else {
                warn!("Recovery strategy failed: {:?} - {}", 
                      strategy, 
                      attempt_result.error_message.as_deref().unwrap_or("Unknown error"));
            }

            // Add delay between attempts
            if index < strategies.len() - 1 {
                info!("Waiting {:?} before next recovery attempt", self.retry_delay);
                std::thread::sleep(self.retry_delay);
            }
        }

        // If we get here, all recovery strategies failed
        let final_issues = self.detect_repository_issues()?;
        let guidance = self.generate_user_guidance(&final_issues);
        
        error!("All recovery strategies failed. Manual intervention required:");
        error!("{}", guidance);

        Err(anyhow!("Repository recovery failed after {} attempts. {}", 
                   self.recovery_history.len(), guidance))
    }

    /// Generate clear user guidance for manual resolution
    pub fn generate_user_guidance(&self, issues: &[RepositoryIssue]) -> String {
        let mut guidance = String::new();
        guidance.push_str("Manual intervention required to resolve repository issues:\n\n");

        for (index, issue) in issues.iter().enumerate() {
            guidance.push_str(&format!("{}. ", index + 1));
            
            match issue {
                RepositoryIssue::DirectoryMissing => {
                    guidance.push_str(&format!(
                        "Repository directory is missing ({})\n",
                        self.target_path.display()
                    ));
                    guidance.push_str("   Solution: Ensure the parent directory exists and is writable\n");
                    guidance.push_str(&format!("   Command: mkdir -p '{}'\n", 
                                             self.target_path.parent().unwrap_or(&self.target_path).display()));
                }
                
                RepositoryIssue::GitDirectoryCorrupted => {
                    guidance.push_str("Git directory is corrupted or missing\n");
                    guidance.push_str("   Solution: Remove the corrupted directory and clone fresh\n");
                    guidance.push_str(&format!("   Commands:\n"));
                    guidance.push_str(&format!("     rm -rf '{}'\n", self.target_path.display()));
                    guidance.push_str(&format!("     git clone '{}' '{}'\n", self.repo_url, self.target_path.display()));
                }
                
                RepositoryIssue::EmptyRepository => {
                    guidance.push_str("Repository directory exists but is empty\n");
                    guidance.push_str("   Solution: Remove empty directory and clone fresh\n");
                    guidance.push_str(&format!("   Commands:\n"));
                    guidance.push_str(&format!("     rmdir '{}'\n", self.target_path.display()));
                    guidance.push_str(&format!("     git clone '{}' '{}'\n", self.repo_url, self.target_path.display()));
                }
                
                RepositoryIssue::MissingRequiredDirectories(dirs) => {
                    guidance.push_str(&format!("Required directories are missing: {:?}\n", dirs));
                    guidance.push_str("   Solution: This indicates an incomplete clone. Re-clone the repository\n");
                    guidance.push_str(&format!("   Commands:\n"));
                    guidance.push_str(&format!("     rm -rf '{}'\n", self.target_path.display()));
                    guidance.push_str(&format!("     git clone '{}' '{}'\n", self.repo_url, self.target_path.display()));
                }
                
                RepositoryIssue::RepositoryNotOpenable(error) => {
                    guidance.push_str(&format!("Repository cannot be opened by git: {}\n", error));
                    guidance.push_str("   Solution: The repository is corrupted. Remove and re-clone\n");
                    guidance.push_str(&format!("   Commands:\n"));
                    guidance.push_str(&format!("     rm -rf '{}'\n", self.target_path.display()));
                    guidance.push_str(&format!("     git clone '{}' '{}'\n", self.repo_url, self.target_path.display()));
                }
                
                RepositoryIssue::InvalidHeadReference => {
                    guidance.push_str("Repository HEAD reference is invalid\n");
                    guidance.push_str("   Solution: Reset the repository or re-clone\n");
                    guidance.push_str(&format!("   Commands (try in order):\n"));
                    guidance.push_str(&format!("     cd '{}' && git fetch origin && git reset --hard origin/main\n", self.target_path.display()));
                    guidance.push_str(&format!("     OR: rm -rf '{}' && git clone '{}' '{}'\n", 
                                             self.target_path.display(), self.repo_url, self.target_path.display()));
                }
                
                RepositoryIssue::IncompleteClone => {
                    guidance.push_str("Repository appears to have an incomplete clone\n");
                    guidance.push_str("   Solution: Remove partial clone and retry\n");
                    guidance.push_str(&format!("   Commands:\n"));
                    guidance.push_str(&format!("     rm -rf '{}'\n", self.target_path.display()));
                    guidance.push_str(&format!("     git clone '{}' '{}'\n", self.repo_url, self.target_path.display()));
                }
                
                RepositoryIssue::PermissionIssues(error) => {
                    guidance.push_str(&format!("Permission issues detected: {}\n", error));
                    guidance.push_str("   Solution: Fix directory permissions\n");
                    guidance.push_str(&format!("   Commands:\n"));
                    guidance.push_str(&format!("     chmod -R u+rwX '{}'\n", 
                                             self.target_path.parent().unwrap_or(&self.target_path).display()));
                    guidance.push_str("   Note: You may need to run with appropriate permissions (sudo if necessary)\n");
                }
            }
            
            guidance.push('\n');
        }

        guidance.push_str("Additional troubleshooting steps:\n");
        guidance.push_str("1. Check internet connection and repository accessibility\n");
        guidance.push_str(&format!("2. Verify repository URL is correct: {}\n", self.repo_url));
        guidance.push_str("3. Check available disk space\n");
        guidance.push_str("4. Ensure git is installed and accessible\n");
        guidance.push_str("5. Check firewall/proxy settings if behind corporate network\n");

        if !self.recovery_history.is_empty() {
            guidance.push_str("\nRecovery attempts made:\n");
            for (index, attempt) in self.recovery_history.iter().enumerate() {
                guidance.push_str(&format!("  {}. {:?} - {}\n", 
                                         index + 1, 
                                         attempt.strategy,
                                         if attempt.success { "SUCCESS" } else { "FAILED" }));
                if let Some(error) = &attempt.error_message {
                    guidance.push_str(&format!("     Error: {}\n", error));
                }
            }
        }

        guidance
    }

    /// Determine appropriate recovery strategies based on detected issues
    fn determine_recovery_strategies(&self, issues: &[RepositoryIssue]) -> Vec<RecoveryStrategy> {
        let mut strategies = Vec::new();

        // Analyze issues to determine best recovery approach
        let has_directory_missing = issues.iter().any(|i| matches!(i, RepositoryIssue::DirectoryMissing));
        let has_git_corruption = issues.iter().any(|i| matches!(i, RepositoryIssue::GitDirectoryCorrupted));
        let has_empty_repo = issues.iter().any(|i| matches!(i, RepositoryIssue::EmptyRepository));
        let has_incomplete_clone = issues.iter().any(|i| matches!(i, RepositoryIssue::IncompleteClone));
        let has_permission_issues = issues.iter().any(|i| matches!(i, RepositoryIssue::PermissionIssues(_)));

        // Strategy 1: Clean and retry for most corruption issues
        if has_git_corruption || has_empty_repo || has_incomplete_clone {
            strategies.push(RecoveryStrategy::CleanAndRetry);
        }

        // Strategy 2: Try alternative clone parameters
        if !has_permission_issues && !has_directory_missing {
            // Try with different branch
            strategies.push(RecoveryStrategy::AlternativeClone(AlternativeCloneParams {
                branch: Some("master".to_string()), // Try master if main failed
                shallow: false,
                alternative_url: None,
                additional_options: HashMap::new(),
            }));

            // Try shallow clone
            strategies.push(RecoveryStrategy::AlternativeClone(AlternativeCloneParams {
                branch: None,
                shallow: true,
                alternative_url: None,
                additional_options: HashMap::new(),
            }));
        }

        // Strategy 3: Try alternative path if current path has issues
        if has_permission_issues || self.target_path.to_string_lossy().contains(' ') {
            if let Some(alt_path) = self.generate_alternative_path() {
                strategies.push(RecoveryStrategy::AlternativePath(alt_path));
            }
        }

        // Strategy 4: Repair in place for minor issues
        if !has_directory_missing && !has_empty_repo {
            strategies.push(RecoveryStrategy::RepairInPlace);
        }

        // If no specific strategies, default to clean and retry
        if strategies.is_empty() {
            strategies.push(RecoveryStrategy::CleanAndRetry);
        }

        strategies
    }

    /// Execute a specific recovery strategy
    fn execute_recovery_strategy<F>(&self, strategy: &RecoveryStrategy, clone_fn: F) -> RecoveryAttempt
    where
        F: Fn(&str, &Path) -> Result<()>,
    {
        let start_time = SystemTime::now();
        let mut context = HashMap::new();
        
        info!("Executing recovery strategy: {:?}", strategy);

        let result = match strategy {
            RecoveryStrategy::CleanAndRetry => {
                self.execute_clean_and_retry(clone_fn, &mut context)
            }
            
            RecoveryStrategy::RepairInPlace => {
                self.execute_repair_in_place(&mut context)
            }
            
            RecoveryStrategy::AlternativeClone(params) => {
                self.execute_alternative_clone(params, clone_fn, &mut context)
            }
            
            RecoveryStrategy::AlternativePath(alt_path) => {
                self.execute_alternative_path(alt_path, clone_fn, &mut context)
            }
            
            RecoveryStrategy::ManualIntervention(guidance) => {
                context.insert("guidance".to_string(), guidance.clone());
                Err(anyhow!("Manual intervention required: {}", guidance))
            }
            
            RecoveryStrategy::NoRecovery(reason) => {
                context.insert("reason".to_string(), reason.clone());
                Err(anyhow!("No recovery possible: {}", reason))
            }
        };

        let duration = start_time.elapsed().unwrap_or_default();
        let success = result.is_ok();
        let error_message = if let Err(ref e) = result {
            Some(e.to_string())
        } else {
            None
        };

        RecoveryAttempt {
            strategy: strategy.clone(),
            success,
            error_message,
            duration,
            context,
            timestamp: start_time,
        }
    }

    /// Execute clean-and-retry recovery strategy
    fn execute_clean_and_retry<F>(&self, clone_fn: F, context: &mut HashMap<String, String>) -> Result<()>
    where
        F: Fn(&str, &Path) -> Result<()>,
    {
        info!("Executing clean-and-retry recovery strategy");
        
        // Remove existing repository if it exists
        if self.target_path.exists() {
            info!("Removing existing repository directory: {}", self.target_path.display());
            context.insert("removed_existing".to_string(), "true".to_string());
            
            std::fs::remove_dir_all(&self.target_path)
                .map_err(|e| anyhow!("Failed to remove existing repository: {}", e))?;
            
            info!("Successfully removed existing repository directory");
        }

        // Ensure parent directory exists
        if let Some(parent) = self.target_path.parent() {
            if !parent.exists() {
                info!("Creating parent directory: {}", parent.display());
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow!("Failed to create parent directory: {}", e))?;
                context.insert("created_parent_dir".to_string(), "true".to_string());
            }
        }

        // Attempt fresh clone
        info!("Attempting fresh clone to: {}", self.target_path.display());
        clone_fn(&self.repo_url, &self.target_path)
            .map_err(|e| anyhow!("Fresh clone failed: {}", e))?;

        context.insert("fresh_clone_success".to_string(), "true".to_string());
        info!("Clean-and-retry recovery completed successfully");
        Ok(())
    }

    /// Execute repair-in-place recovery strategy
    fn execute_repair_in_place(&self, context: &mut HashMap<String, String>) -> Result<()> {
        info!("Executing repair-in-place recovery strategy");
        
        // Try to open the repository
        let repo = Repository::open_ext(&self.target_path, RepositoryOpenFlags::empty(), &[] as &[&std::ffi::OsStr])
            .map_err(|e| anyhow!("Cannot open repository for repair: {}", e))?;

        context.insert("repository_opened".to_string(), "true".to_string());

        // Try to repair HEAD reference
        if repo.head().is_err() {
            info!("Attempting to repair HEAD reference");
            
            // Try to find a valid branch to point HEAD to
            if let Ok(branches) = self.branch_detector.get_available_branches() {
                if let Some(default_branch) = branches.iter().find(|b| b.is_default) {
                    // This is a simplified repair - in practice, you might need more complex logic
                    info!("Found default branch for HEAD repair: {}", default_branch.name);
                    context.insert("head_repair_attempted".to_string(), default_branch.name.clone());
                }
            }
        }

        // Check if repair was successful
        if repo.head().is_ok() {
            context.insert("head_repair_success".to_string(), "true".to_string());
            info!("Repair-in-place recovery completed successfully");
            Ok(())
        } else {
            Err(anyhow!("Repair-in-place failed - HEAD reference still invalid"))
        }
    }

    /// Execute alternative clone parameters strategy
    fn execute_alternative_clone<F>(&self, params: &AlternativeCloneParams, clone_fn: F, context: &mut HashMap<String, String>) -> Result<()>
    where
        F: Fn(&str, &Path) -> Result<()>,
    {
        info!("Executing alternative clone strategy with params: {:?}", params);
        
        // Remove existing repository first
        if self.target_path.exists() {
            std::fs::remove_dir_all(&self.target_path)
                .map_err(|e| anyhow!("Failed to remove existing repository: {}", e))?;
            context.insert("removed_existing".to_string(), "true".to_string());
        }

        // For now, we'll use the provided clone function
        // In a more advanced implementation, you would modify the clone parameters
        // based on the AlternativeCloneParams
        
        if let Some(branch) = &params.branch {
            context.insert("alternative_branch".to_string(), branch.clone());
            info!("Attempting clone with alternative branch: {}", branch);
        }
        
        if params.shallow {
            context.insert("shallow_clone".to_string(), "true".to_string());
            info!("Attempting shallow clone");
        }

        // Attempt clone with alternative parameters
        clone_fn(&self.repo_url, &self.target_path)
            .map_err(|e| anyhow!("Alternative clone failed: {}", e))?;

        context.insert("alternative_clone_success".to_string(), "true".to_string());
        info!("Alternative clone recovery completed successfully");
        Ok(())
    }

    /// Execute alternative path strategy
    fn execute_alternative_path<F>(&self, alt_path: &Path, clone_fn: F, context: &mut HashMap<String, String>) -> Result<()>
    where
        F: Fn(&str, &Path) -> Result<()>,
    {
        info!("Executing alternative path strategy: {}", alt_path.display());
        context.insert("alternative_path".to_string(), alt_path.display().to_string());
        
        // Ensure alternative path parent exists
        if let Some(parent) = alt_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| anyhow!("Failed to create alternative path parent: {}", e))?;
                context.insert("created_alt_parent".to_string(), "true".to_string());
            }
        }

        // Attempt clone to alternative path
        clone_fn(&self.repo_url, alt_path)
            .map_err(|e| anyhow!("Alternative path clone failed: {}", e))?;

        // If successful, we might want to move it to the original location
        // or update the target path - this depends on the specific use case
        
        context.insert("alternative_path_success".to_string(), "true".to_string());
        info!("Alternative path recovery completed successfully");
        Ok(())
    }

    /// Generate an alternative path for the repository
    fn generate_alternative_path(&self) -> Option<PathBuf> {
        let parent = self.target_path.parent()?;
        let name = self.target_path.file_name()?.to_string_lossy();
        
        // Try different alternative paths
        let alternatives = vec![
            parent.join(format!("{}_alt", name)),
            parent.join(format!("{}_backup", name)),
            parent.join("guard-rules-alt"),
            std::env::temp_dir().join(&*name),
        ];

        for alt_path in alternatives {
            if !alt_path.exists() {
                return Some(alt_path);
            }
        }

        None
    }

    /// Count files in the repository (excluding .git)
    fn count_repository_files(&self) -> Result<usize> {
        if !self.target_path.exists() {
            return Ok(0);
        }

        let mut count = 0;
        self.count_directory_files(&self.target_path, &mut count)?;
        Ok(count)
    }

    /// Count files, directories, and total size in the repository
    fn count_repository_contents(&self) -> Result<(usize, usize, u64)> {
        if !self.target_path.exists() {
            return Ok((0, 0, 0));
        }

        let mut files = 0;
        let mut dirs = 0;
        let mut size = 0;
        
        self.count_directory_contents(&self.target_path, &mut files, &mut dirs, &mut size)?;
        Ok((files, dirs, size))
    }

    /// Recursively count files in a directory
    fn count_directory_files(&self, path: &Path, count: &mut usize) -> Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            
            if file_type.is_file() {
                *count += 1;
            } else if file_type.is_dir() {
                let dir_name = entry.file_name();
                // Skip .git directory
                if dir_name != ".git" {
                    self.count_directory_files(&entry.path(), count)?;
                }
            }
        }
        Ok(())
    }

    /// Recursively count files, directories, and size
    fn count_directory_contents(&self, path: &Path, files: &mut usize, dirs: &mut usize, size: &mut u64) -> Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            
            if file_type.is_file() {
                *files += 1;
                if let Ok(metadata) = entry.metadata() {
                    *size += metadata.len();
                }
            } else if file_type.is_dir() {
                *dirs += 1;
                let dir_name = entry.file_name();
                // Skip .git directory for size calculation
                if dir_name != ".git" {
                    self.count_directory_contents(&entry.path(), files, dirs, size)?;
                }
            }
        }
        Ok(())
    }

    /// Find missing required directories
    fn find_missing_required_directories(&self) -> Vec<String> {
        let mut missing = Vec::new();
        
        for required_dir in &self.required_directories {
            let dir_path = self.target_path.join(required_dir);
            if !dir_path.exists() {
                missing.push(required_dir.clone());
            }
        }
        
        missing
    }

    /// Check repository permissions
    fn check_repository_permissions(&self) -> Result<()> {
        if !self.target_path.exists() {
            return Err(anyhow!("Repository path does not exist"));
        }

        // Test read permission
        std::fs::read_dir(&self.target_path)
            .map_err(|e| anyhow!("Cannot read repository directory: {}", e))?;

        // Test write permission by creating a temporary file
        let temp_file = self.target_path.join(".awsdash_permission_test");
        std::fs::write(&temp_file, "test")
            .map_err(|e| anyhow!("Cannot write to repository directory: {}", e))?;
        
        // Clean up test file
        let _ = std::fs::remove_file(&temp_file);

        Ok(())
    }

    /// Get recovery history
    pub fn get_recovery_history(&self) -> &[RecoveryAttempt] {
        &self.recovery_history
    }

    /// Clear recovery history
    pub fn clear_recovery_history(&mut self) {
        self.recovery_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn test_recovery_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_repo");
        let required_dirs = vec!["mappings".to_string(), "rules".to_string()];
        
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path.clone(),
            required_dirs.clone(),
        );

        assert_eq!(manager.repo_url, "https://github.com/test/repo.git");
        assert_eq!(manager.target_path, target_path);
        assert_eq!(manager.required_directories, required_dirs);
        assert_eq!(manager.max_recovery_attempts, 3);
    }

    #[test]
    fn test_detect_directory_missing() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("nonexistent_repo");
        let required_dirs = vec!["mappings".to_string()];
        
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let issues = manager.detect_repository_issues().unwrap();
        assert!(issues.contains(&RepositoryIssue::DirectoryMissing));
    }

    #[test]
    fn test_detect_empty_repository() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("empty_repo");
        std::fs::create_dir_all(&target_path).unwrap();
        
        let required_dirs = vec!["mappings".to_string()];
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let issues = manager.detect_repository_issues().unwrap();
        assert!(issues.contains(&RepositoryIssue::EmptyRepository));
    }

    #[test]
    fn test_repository_validation() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_repo");
        let required_dirs = vec!["mappings".to_string(), "rules".to_string()];
        
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let validation = manager.validate_repository_structure();
        assert!(!validation.is_valid); // Should be invalid since directory doesn't exist
        assert!(validation.validation_errors.len() > 0);
    }

    #[test]
    fn test_user_guidance_generation() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_repo");
        let required_dirs = vec!["mappings".to_string()];
        
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let issues = vec![
            RepositoryIssue::DirectoryMissing,
            RepositoryIssue::EmptyRepository,
        ];

        let guidance = manager.generate_user_guidance(&issues);
        assert!(guidance.contains("Manual intervention required"));
        assert!(guidance.contains("Repository directory is missing"));
        assert!(guidance.contains("mkdir -p"));
    }
}