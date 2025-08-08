//! # AWS CloudFormation Guard Rules Repository Manager
//!
//! This module provides git-based management of the AWS CloudFormation Guard Rules Repository,
//! replacing the previous HTTP-based download system with efficient git operations.
//!
//! ## Core Functionality
//!
//! * **Git Repository Management**: Clone and pull operations for the guard rules repository
//! * **Local Caching**: Maintains local copy in ~/.local/share/awsdash/guard-rules-registry
//! * **Progress Reporting**: Shows progress during clone/pull operations like schema downloads
//! * **Compliance Discovery**: Parses compliance programs from /mappings directory
//! * **Rules Access**: Provides access to guard rules from /rules directory

use anyhow::{anyhow, Result};
use git2::{Repository, RepositoryOpenFlags};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

use crate::app::git_error_handling::{
    GitOperation, GitOperationError, GitOperationLogger
};
use crate::app::branch_detector::BranchDetector;
use crate::app::repository_recovery::{
    RepositoryRecoveryManager, RepositoryValidation
};

/// GitHub repository URL for AWS CloudFormation Guard Rules
const GUARD_RULES_REPO_URL: &str = "https://github.com/aws-cloudformation/aws-guard-rules-registry.git";

/// Local directory name for the cloned repository
const REPO_DIR_NAME: &str = "guard-rules-registry";

/// Status of repository sync operations
#[derive(Debug, Clone, PartialEq)]
pub struct RepositorySyncStatus {
    /// Current operation phase
    pub phase: SyncPhase,
    /// Whether the operation completed successfully
    pub completed: bool,
    /// Optional error message if operation failed
    pub error: Option<String>,
    /// Progress percentage (0-100)
    pub progress: u8,
}

/// Different phases of repository synchronization
#[derive(Debug, Clone, PartialEq)]
pub enum SyncPhase {
    /// Checking if repository exists locally
    CheckingLocal,
    /// Cloning repository for the first time
    Cloning,
    /// Pulling latest changes
    Pulling,
    /// Parsing compliance programs
    ParsingPrograms,
    /// Indexing guard rules
    IndexingRules,
    /// Sync operation completed
    Complete,
}

impl RepositorySyncStatus {
    fn new(phase: SyncPhase, progress: u8) -> Self {
        Self {
            phase,
            completed: false,
            error: None,
            progress,
        }
    }

    fn completed() -> Self {
        Self {
            phase: SyncPhase::Complete,
            completed: true,
            error: None,
            progress: 100,
        }
    }

    fn with_error(phase: SyncPhase, error: String) -> Self {
        Self {
            phase,
            completed: false,
            error: Some(error),
            progress: 0,
        }
    }
}

/// Repository manager for AWS CloudFormation Guard Rules
#[derive(Debug, Clone)]
pub struct GuardRepositoryManager {
    /// Local data directory where repository is cloned
    data_dir: PathBuf,
    /// Path to the cloned repository
    repo_path: PathBuf,
}

impl GuardRepositoryManager {
    /// Create a new repository manager
    pub fn new() -> Result<Self> {
        let data_dir = Self::get_data_directory()?;
        let repo_path = data_dir.join(REPO_DIR_NAME);

        Ok(Self { data_dir, repo_path })
    }

    /// Get the data directory for storing the repository
    fn get_data_directory() -> Result<PathBuf> {
        dirs::data_dir()
            .map(|dir| dir.join("awsdash"))
            .ok_or_else(|| anyhow!("Could not determine data directory"))
    }

    /// Check if repository is already cloned locally
    pub fn is_repository_cloned(&self) -> bool {
        self.repo_path.exists() && self.repo_path.join(".git").exists()
    }

    /// Get the path to the cloned repository
    pub fn get_repository_path(&self) -> &Path {
        &self.repo_path
    }

    /// Get the path to the mappings directory
    pub fn get_mappings_path(&self) -> PathBuf {
        self.repo_path.join("mappings")
    }

    /// Get the path to the rules directory
    pub fn get_rules_path(&self) -> PathBuf {
        self.repo_path.join("rules")
    }

    /// Create a branch detector for the guard rules repository
    pub fn create_branch_detector(&self) -> BranchDetector {
        BranchDetector::new(GUARD_RULES_REPO_URL.to_string())
    }

    /// Synchronize repository (clone if not exists, pull if exists) - async version
    pub fn sync_repository_async() -> mpsc::Receiver<RepositorySyncStatus> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let manager = match Self::new() {
                Ok(manager) => manager,
                Err(e) => {
                    let _ = tx.send(RepositorySyncStatus::with_error(
                        SyncPhase::CheckingLocal,
                        format!("Failed to initialize repository manager: {}", e),
                    ));
                    return;
                }
            };

            let result = manager.sync_repository_with_progress(&tx);

            // Send final status
            match result {
                Ok(()) => {
                    let _ = tx.send(RepositorySyncStatus::completed());
                }
                Err(e) => {
                    let _ = tx.send(RepositorySyncStatus::with_error(
                        SyncPhase::Complete,
                        format!("Repository sync failed: {}", e),
                    ));
                }
            }
        });

        rx
    }

    /// Synchronize repository with progress reporting
    fn sync_repository_with_progress(&self, tx: &mpsc::Sender<RepositorySyncStatus>) -> Result<()> {
        // Phase 1: Check local repository
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::CheckingLocal, 10));

        // Create data directory if it doesn't exist
        if !self.data_dir.exists() {
            std::fs::create_dir_all(&self.data_dir)?;
        }

        if self.is_repository_cloned() {
            // Repository exists, pull latest changes
            self.pull_repository(tx)?;
        } else {
            // Repository doesn't exist, clone it
            self.clone_repository(tx)?;
        }

        // Phase 4: Parse compliance programs
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::ParsingPrograms, 80));
        self.verify_repository_structure()?;

        // Phase 5: Index rules (placeholder for now)
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::IndexingRules, 90));
        self.index_rules()?;

        Ok(())
    }

    /// Clone the repository from GitHub
    fn clone_repository(&self, tx: &mpsc::Sender<RepositorySyncStatus>) -> Result<()> {
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Cloning, 20));
        
        // Detect the default branch before starting clone operation
        let branch_detector = self.create_branch_detector();
        let default_branch = match branch_detector.detect_default_branch() {
            Ok(branch) => {
                info!("Successfully detected default branch: {}", branch);
                branch
            }
            Err(e) => {
                warn!("Failed to detect default branch, falling back to 'main': {}", e);
                "main".to_string()
            }
        };
        
        // Start logging for the clone operation
        let logger = GitOperationLogger::start(
            GitOperation::Clone,
            self.repo_path.clone(),
            Some(GUARD_RULES_REPO_URL.to_string()),
            Some(default_branch.clone()),
        );
        
        // Check if target directory already exists and handle it
        if self.repo_path.exists() {
            // Check if it's a git repository
            if self.repo_path.join(".git").exists() {
                info!("Removing existing repository for fresh clone");
                if let Err(e) = std::fs::remove_dir_all(&self.repo_path) {
                    let error_msg = format!("Failed to remove existing repository: {}", e);
                    error!("{}", error_msg);
                    let git_error = logger.log_failure_with_message(error_msg);
                    return Err(git_error.into_anyhow());
                }
            } else {
                // Directory exists but is not a git repo - check if it's empty
                match std::fs::read_dir(&self.repo_path) {
                    Ok(entries) => {
                        let count = entries.count();
                        if count > 0 {
                            let error_msg = format!("Target directory exists and contains {} items - cannot clone", count);
                            error!("{}", error_msg);
                            let git_error = logger.log_failure_with_message(error_msg);
                            return Err(git_error.into_anyhow());
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Cannot read target directory: {}", e);
                        error!("{}", error_msg);
                        let git_error = logger.log_failure_with_message(error_msg);
                        return Err(git_error.into_anyhow());
                    }
                }
            }
        }
        
        // Ensure parent directory exists
        let parent_dir = self.repo_path.parent().unwrap_or(&self.data_dir);
        if !parent_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(parent_dir) {
                let error_msg = format!("Failed to create parent directory {}: {}", parent_dir.display(), e);
                error!("{}", error_msg);
                let git_error = logger.log_failure_with_message(error_msg);
                return Err(git_error.into_anyhow());
            }
        }
        
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Cloning, 30));
        
        // Set up git2 clone operation with TLS configuration
        use crate::app::git_error_handling::tls_config;
        let mut cb = tls_config::configure_tls_callbacks();
        
        // Add minimal progress callbacks
        cb.update_tips(|_refname, _a, _b| true);

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(cb);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);
        
        // Add branch specification using dynamic branch detection
        builder.branch(&default_branch);
        
        info!("Starting git clone operation");
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Cloning, 40));

        match builder.clone(GUARD_RULES_REPO_URL, &self.repo_path) {
            Ok(_repo) => {
                info!("Git clone operation completed successfully");
                let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Cloning, 60));
                
                // Basic validation - check that essential directories exist
                let mappings_dir = self.repo_path.join("mappings");
                let rules_dir = self.repo_path.join("rules");
                
                if !mappings_dir.exists() {
                    warn!("Mappings directory not found - this may indicate an incomplete clone");
                }
                
                if !rules_dir.exists() {
                    warn!("Rules directory not found - this may indicate an incomplete clone");
                }
                
                logger.log_success("Repository successfully cloned");
                let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Cloning, 70));
                Ok(())
            }
            Err(e) => {
                error!("Git clone operation failed: {}", e.message());
                let git_error = logger.log_failure(e);
                Err(git_error.into_anyhow())
            }
        }
    }

    /// Pull latest changes from the repository
    fn pull_repository(&self, tx: &mpsc::Sender<RepositorySyncStatus>) -> Result<()> {
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Pulling, 30));
        
        // Detect the default branch for pull operation
        let branch_detector = self.create_branch_detector();
        let default_branch = match branch_detector.detect_default_branch() {
            Ok(branch) => {
                info!("Successfully detected default branch for pull: {}", branch);
                branch
            }
            Err(e) => {
                warn!("Failed to detect default branch for pull, falling back to 'main': {}", e);
                "main".to_string()
            }
        };
        
        // Start logging for the pull operation
        let mut logger = GitOperationLogger::start(
            GitOperation::Pull,
            self.repo_path.clone(),
            Some(GUARD_RULES_REPO_URL.to_string()),
            Some(default_branch.clone()),
        );

        // First, try to open the repository
        let repo = match Repository::open_ext(&self.repo_path, RepositoryOpenFlags::empty(), &[] as &[&std::ffi::OsStr]) {
            Ok(repo) => {
                logger.add_context("repository_opened".to_string(), "success".to_string());
                repo
            }
            Err(e) => {
                let git_error = GitOperationError::from_git2_error(
                    GitOperation::RepositoryOpen,
                    e,
                    self.repo_path.clone(),
                    Some(GUARD_RULES_REPO_URL.to_string()),
                    None,
                );
                git_error.log_error();
                return Err(git_error.into_anyhow());
            }
        };

        // Get the remote with error handling
        let mut remote = match repo.find_remote("origin") {
            Ok(remote) => {
                logger.add_context("remote_found".to_string(), "origin".to_string());
                remote
            }
            Err(e) => {
                let git_error = GitOperationError::from_git2_error(
                    GitOperation::RemoteOperation,
                    e,
                    self.repo_path.clone(),
                    Some(GUARD_RULES_REPO_URL.to_string()),
                    None,
                );
                git_error.log_error();
                return Err(git_error.into_anyhow());
            }
        };

        // Set up callbacks with TLS configuration
        use crate::app::git_error_handling::tls_config;
        let mut cb = tls_config::configure_tls_callbacks();
        
        // Add minimal progress callbacks
        cb.update_tips(|_refname, _a, _b| true);

        // Fetch changes with error handling
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(cb);
        
        let fetch_refspec = format!("refs/heads/{}:refs/remotes/origin/{}", default_branch, default_branch);
        if let Err(e) = remote.fetch(&[&fetch_refspec], Some(&mut fetch_options), None) {
            let git_error = logger.log_failure(e);
            
            // Provide specific guidance based on error type
            if let Some(error_class) = &git_error.git2_error_class {
                match error_class.as_str() {
                    "Net" | "Http" | "Ssl" => {
                        error!("Network error during fetch - check internet connection");
                    }
                    "Reference" => {
                        error!("Branch error during fetch - the '{}' branch may not exist on remote", default_branch);
                        
                        // Try to list available branches for debugging
                        if let Ok(branches) = branch_detector.get_available_branches() {
                            error!("Available branches: {:?}", 
                                   branches.iter().map(|b| &b.name).collect::<Vec<_>>());
                        }
                    }
                    _ => {
                        error!("Fetch failure with error class: {}", error_class);
                    }
                }
            }
            
            return Err(git_error.into_anyhow());
        }

        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Pulling, 50));

        // Get references with error handling
        let fetch_head = match repo.find_reference("FETCH_HEAD") {
            Ok(reference) => reference,
            Err(e) => {
                let git_error = GitOperationError::from_git2_error(
                    GitOperation::ReferenceCreate,
                    e,
                    self.repo_path.clone(),
                    Some(GUARD_RULES_REPO_URL.to_string()),
                    Some("FETCH_HEAD".to_string()),
                );
                git_error.log_error();
                return Err(git_error.into_anyhow());
            }
        };

        let fetch_commit = match repo.reference_to_annotated_commit(&fetch_head) {
            Ok(commit) => commit,
            Err(e) => {
                let git_error = logger.log_failure(e);
                return Err(git_error.into_anyhow());
            }
        };

        // Do the merge analysis with error handling
        let analysis = match repo.merge_analysis(&[&fetch_commit]) {
            Ok(analysis) => analysis,
            Err(e) => {
                let git_error = GitOperationError::from_git2_error(
                    GitOperation::Merge,
                    e,
                    self.repo_path.clone(),
                    Some(GUARD_RULES_REPO_URL.to_string()),
                    Some(default_branch.clone()),
                );
                git_error.log_error();
                return Err(git_error.into_anyhow());
            }
        };
        
        if analysis.0.is_up_to_date() {
            logger.log_success("Repository is already up to date");
        } else if analysis.0.is_fast_forward() {
            info!("Fast-forwarding repository");
            
            let refname = format!("refs/heads/{}", default_branch);
            
            // Update or create reference with error handling
            if let Err(e) = (|| -> Result<(), git2::Error> {
                // Try to find existing reference, create if it doesn't exist
                match repo.find_reference(&refname) {
                    Ok(mut reference) => {
                        // Reference exists, update it
                        reference.set_target(fetch_commit.id(), "Fast-Forward")?;
                    }
                    Err(_) => {
                        // Reference doesn't exist, create it
                        repo.reference(&refname, fetch_commit.id(), false, "Create branch")?;
                    }
                }
                repo.set_head(&refname)?;
                repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
                Ok(())
            })() {
                let git_error = GitOperationError::from_git2_error(
                    GitOperation::Checkout,
                    e,
                    self.repo_path.clone(),
                    Some(GUARD_RULES_REPO_URL.to_string()),
                    Some(default_branch.clone()),
                );
                git_error.log_error();
                return Err(git_error.into_anyhow());
            }
            
            logger.log_success("Successfully fast-forwarded repository");
        } else {
            let error_msg = "Repository requires manual merge - cannot fast-forward".to_string();
            warn!("{}", error_msg);
            let git_error = logger.log_failure_with_message(error_msg);
            return Err(git_error.into_anyhow());
        }

        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Pulling, 70));
        Ok(())
    }

    /// Create a repository recovery manager for this guard repository
    pub fn create_recovery_manager(&self) -> RepositoryRecoveryManager {
        let required_directories = vec![
            "mappings".to_string(),
            "rules".to_string(),
        ];
        
        RepositoryRecoveryManager::new(
            GUARD_RULES_REPO_URL.to_string(),
            self.repo_path.clone(),
            required_directories,
        )
    }

    /// Perform comprehensive repository validation using recovery manager
    pub fn validate_repository_comprehensive(&self) -> RepositoryValidation {
        let recovery_manager = self.create_recovery_manager();
        recovery_manager.validate_repository_structure()
    }

    /// Attempt repository recovery if issues are detected
    pub fn attempt_repository_recovery(&self, tx: &mpsc::Sender<RepositorySyncStatus>) -> Result<()> {
        info!("Starting repository recovery process");
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::CheckingLocal, 5));

        let mut recovery_manager = self.create_recovery_manager();
        
        // Detect issues first
        let issues = recovery_manager.detect_repository_issues()
            .map_err(|e| anyhow!("Failed to detect repository issues: {}", e))?;

        if issues.is_empty() {
            info!("No repository issues detected - recovery not needed");
            return Ok(());
        }

        info!("Detected {} repository issues, attempting recovery", issues.len());
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::CheckingLocal, 10));

        // Create a clone function that integrates with our existing clone logic
        let data_dir = self.data_dir.clone();
        let tx_clone = tx.clone();
        
        let clone_fn = move |_repo_url: &str, target_path: &Path| -> Result<()> {
            // Create a temporary manager for the recovery clone
            let temp_manager = GuardRepositoryManager {
                data_dir: data_dir.clone(),
                repo_path: target_path.to_path_buf(),
            };
            
            // Use our existing clone logic
            temp_manager.clone_repository(&tx_clone)
        };

        // Attempt recovery
        recovery_manager.attempt_repository_recovery(clone_fn)
            .map_err(|e| {
                error!("Repository recovery failed: {}", e);
                
                // Generate user guidance for manual resolution
                let final_issues = recovery_manager.detect_repository_issues()
                    .unwrap_or_default();
                let guidance = recovery_manager.generate_user_guidance(&final_issues);
                
                error!("Manual intervention required:");
                for line in guidance.lines() {
                    error!("{}", line);
                }
                
                anyhow!("Repository recovery failed: {}", e)
            })?;

        info!("Repository recovery completed successfully");
        Ok(())
    }

    /// Enhanced sync repository that includes recovery mechanisms
    pub fn sync_repository_with_recovery() -> mpsc::Receiver<RepositorySyncStatus> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let manager = match Self::new() {
                Ok(manager) => manager,
                Err(e) => {
                    let _ = tx.send(RepositorySyncStatus::with_error(
                        SyncPhase::CheckingLocal,
                        format!("Failed to initialize repository manager: {}", e),
                    ));
                    return;
                }
            };

            // First, try normal sync
            let normal_sync_result = manager.sync_repository_with_progress(&tx);
            
            match normal_sync_result {
                Ok(()) => {
                    // Normal sync succeeded, validate the result
                    let validation = manager.validate_repository_comprehensive();
                    if validation.is_valid {
                        let _ = tx.send(RepositorySyncStatus::completed());
                        return;
                    } else {
                        warn!("Repository sync completed but validation failed: {:?}", validation.validation_errors);
                        // Continue to recovery attempt
                    }
                }
                Err(e) => {
                    warn!("Normal repository sync failed: {}", e);
                    // Continue to recovery attempt
                }
            }

            // Normal sync failed or validation failed, attempt recovery
            info!("Attempting repository recovery due to sync issues");
            let recovery_result = manager.attempt_repository_recovery(&tx);

            match recovery_result {
                Ok(()) => {
                    // Recovery succeeded, validate again
                    let validation = manager.validate_repository_comprehensive();
                    if validation.is_valid {
                        let _ = tx.send(RepositorySyncStatus::completed());
                    } else {
                        let _ = tx.send(RepositorySyncStatus::with_error(
                            SyncPhase::Complete,
                            format!("Recovery completed but repository is still invalid: {:?}", validation.validation_errors),
                        ));
                    }
                }
                Err(e) => {
                    let _ = tx.send(RepositorySyncStatus::with_error(
                        SyncPhase::Complete,
                        format!("Repository recovery failed: {}", e),
                    ));
                }
            }
        });

        rx
    }

    /// Verify that the cloned repository has the expected structure with enhanced logging
    fn verify_repository_structure(&self) -> Result<()> {
        info!("Verifying repository structure at {}", self.repo_path.display());
        
        let mappings_dir = self.get_mappings_path();
        let rules_dir = self.get_rules_path();

        // Check mappings directory
        if !mappings_dir.exists() {
            error!("Repository missing /mappings directory at {}", mappings_dir.display());
            error!("Repository structure verification failed - incomplete clone detected");
            
            // Log what directories do exist for debugging
            if let Ok(entries) = std::fs::read_dir(&self.repo_path) {
                let existing_dirs: Vec<String> = entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                    .map(|entry| entry.file_name().to_string_lossy().to_string())
                    .collect();
                error!("Existing directories in repository: {:?}", existing_dirs);
            }
            
            return Err(anyhow!("Repository missing /mappings directory"));
        } else {
            info!("Found mappings directory at {}", mappings_dir.display());
        }

        // Check rules directory
        if !rules_dir.exists() {
            error!("Repository missing /rules directory at {}", rules_dir.display());
            error!("Repository structure verification failed - incomplete clone detected");
            return Err(anyhow!("Repository missing /rules directory"));
        } else {
            info!("Found rules directory at {}", rules_dir.display());
        }

        // Count files in each directory for additional verification
        let mappings_count = std::fs::read_dir(&mappings_dir)
            .map(|entries| entries.count())
            .unwrap_or(0);
        
        let rules_count = std::fs::read_dir(&rules_dir)
            .map(|entries| entries.count())
            .unwrap_or(0);

        info!("Repository structure verified successfully:");
        info!("  - Mappings directory: {} items", mappings_count);
        info!("  - Rules directory: {} items", rules_count);

        if mappings_count == 0 {
            warn!("Mappings directory is empty - this may indicate an incomplete clone");
        }

        if rules_count == 0 {
            warn!("Rules directory is empty - this may indicate an incomplete clone");
        }

        Ok(())
    }

    /// Index available rules (placeholder for future implementation)
    fn index_rules(&self) -> Result<()> {
        // This could be expanded to pre-index rules for faster access
        // For now, just verify we can read the directories
        
        let rules_count = std::fs::read_dir(self.get_rules_path())?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type()
                    .map(|ft| ft.is_dir())
                    .unwrap_or(false)
            })
            .count();

        let mappings_count = std::fs::read_dir(self.get_mappings_path())?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "guard" || ext == "json")
                    .unwrap_or(false)
            })
            .count();

        info!("Indexed {} rule categories and {} compliance mappings", rules_count, mappings_count);
        Ok(())
    }

    /// Get the timestamp of the last repository update
    pub fn get_last_update_timestamp(&self) -> Option<u64> {
        if !self.is_repository_cloned() {
            return None;
        }

        Repository::open_ext(&self.repo_path, RepositoryOpenFlags::empty(), &[] as &[&std::ffi::OsStr])
            .ok()?
            .head()
            .ok()?
            .peel_to_commit()
            .ok()?
            .time()
            .seconds()
            .try_into()
            .ok()
    }

    /// Check if repository needs updating (older than 24 hours)
    pub fn needs_update(&self) -> bool {
        match self.get_last_update_timestamp() {
            Some(last_update) => {
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                
                // Update if older than 24 hours (86400 seconds)
                current_time.saturating_sub(last_update) > 86400
            }
            None => true, // Repository doesn't exist, needs initial clone
        }
    }
}

impl Default for GuardRepositoryManager {
    fn default() -> Self {
        Self::new().expect("Failed to create GuardRepositoryManager")
    }
}