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
        
        info!("Cloning AWS CloudFormation Guard Rules Repository...");
        
        // Use git2 to clone the repository
        let mut cb = git2::RemoteCallbacks::new();
        cb.update_tips(|_refname, a, b| {
            if a.is_zero() {
                info!("Cloning: {}", b);
            } else {
                info!("Updating: {} to {}", a, b);
            }
            true
        });

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(cb);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        match builder.clone(GUARD_RULES_REPO_URL, &self.repo_path) {
            Ok(_repo) => {
                info!("Successfully cloned Guard Rules Repository to {:?}", self.repo_path);
                let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Cloning, 70));
                Ok(())
            }
            Err(e) => {
                error!("Failed to clone repository: {}", e);
                Err(anyhow!("Git clone failed: {}", e))
            }
        }
    }

    /// Pull latest changes from the repository
    fn pull_repository(&self, tx: &mpsc::Sender<RepositorySyncStatus>) -> Result<()> {
        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Pulling, 30));
        
        info!("Pulling latest changes for Guard Rules Repository...");

        let repo = Repository::open_ext(&self.repo_path, RepositoryOpenFlags::empty(), &[] as &[&std::ffi::OsStr])?;

        // Get the remote
        let mut remote = repo.find_remote("origin")?;

        // Set up callbacks
        let mut cb = git2::RemoteCallbacks::new();
        cb.update_tips(|_refname, a, b| {
            if a.is_zero() {
                info!("Updating: {}", b);
            } else {
                info!("Updating: {} to {}", a, b);
            }
            true
        });

        // Fetch changes
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(cb);
        
        remote.fetch(&["refs/heads/main:refs/remotes/origin/main"], Some(&mut fetch_options), None)?;

        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Pulling, 50));

        // Get references
        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;

        // Do the merge
        let analysis = repo.merge_analysis(&[&fetch_commit])?;
        
        if analysis.0.is_up_to_date() {
            info!("Repository is already up to date");
        } else if analysis.0.is_fast_forward() {
            info!("Fast-forwarding repository");
            
            let refname = format!("refs/heads/{}", "main");
            let mut reference = repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-Forward")?;
            repo.set_head(&refname)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
            
            info!("Successfully updated Guard Rules Repository");
        } else {
            warn!("Cannot fast-forward repository, manual intervention may be needed");
            return Err(anyhow!("Repository requires manual merge"));
        }

        let _ = tx.send(RepositorySyncStatus::new(SyncPhase::Pulling, 70));
        Ok(())
    }

    /// Verify that the cloned repository has the expected structure
    fn verify_repository_structure(&self) -> Result<()> {
        let mappings_dir = self.get_mappings_path();
        let rules_dir = self.get_rules_path();

        if !mappings_dir.exists() {
            return Err(anyhow!("Repository missing /mappings directory"));
        }

        if !rules_dir.exists() {
            return Err(anyhow!("Repository missing /rules directory"));
        }

        info!("Repository structure verified successfully");
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