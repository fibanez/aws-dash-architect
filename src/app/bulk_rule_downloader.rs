//! Bulk rule download system for CloudFormation Guard compliance programs.
//!
//! This module provides functionality to download, store, and manage large sets
//! of Guard rules from the AWS Guard Rules Registry with progress tracking,
//! retry logic, and efficient storage organization.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;

use crate::app::compliance_discovery::AvailableComplianceProgram;

/// Main client for bulk downloading compliance program rules
#[derive(Debug)]
pub struct BulkRuleDownloader {
    /// Storage directory for downloaded rules
    storage_dir: PathBuf,
    /// HTTP client for downloading files
    #[allow(dead_code)]
    client: reqwest::Client,
    /// Rule storage manager
    storage: RuleStorage,
    /// Rule indexing system
    rule_index: RuleIndex,
    /// Maximum retry attempts for failed downloads
    max_retries: usize,
    /// Delay between retry attempts (milliseconds)
    retry_delay_ms: u64,
    /// Whether to enable rule deduplication
    deduplication_enabled: bool,
}

/// Manager for tracking and coordinating multiple downloads
#[derive(Debug)]
pub struct RuleDownloadManager {
    /// Active downloads in progress
    #[allow(dead_code)]
    active_downloads: HashMap<String, Arc<Mutex<DownloadProgress>>>,
    /// Download queue
    #[allow(dead_code)]
    download_queue: Vec<AvailableComplianceProgram>,
    /// Maximum concurrent downloads
    #[allow(dead_code)]
    max_concurrent: usize,
}

/// Progress tracking for rule downloads
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProgress {
    /// Current download status
    pub status: DownloadStatus,
    /// Number of files downloaded
    pub files_downloaded: usize,
    /// Total number of files to download
    pub total_files: usize,
    /// Currently downloading file
    pub current_file: Option<String>,
    /// Download start time
    pub started_at: Option<DateTime<Utc>>,
    /// Download completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if download failed
    pub error_message: Option<String>,
}

/// Download status enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Storage system for organizing downloaded rules
#[derive(Debug)]
pub struct RuleStorage {
    /// Base storage directory
    storage_dir: PathBuf,
}

/// Indexing system for fast rule lookup
#[derive(Debug)]
pub struct RuleIndex {
    /// Storage directory for index files
    #[allow(dead_code)]
    index_dir: PathBuf,
    /// In-memory index cache
    resource_type_index: HashMap<String, Vec<String>>,
    /// Program to rules mapping
    program_index: HashMap<String, Vec<String>>,
    /// Rule name to content mapping
    rule_content_cache: HashMap<String, String>,
}

/// Complete set of rules for a compliance program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRuleSet {
    /// Name of the compliance program
    pub program_name: String,
    /// Display name of the program
    pub display_name: String,
    /// Version of the rule set
    pub version: String,
    /// Source URL where rules were downloaded from
    pub source_url: String,
    /// When the rules were downloaded
    pub download_date: DateTime<Utc>,
    /// Map of rule name to rule file
    pub rules: HashMap<String, GuardRuleFile>,
}

/// Individual Guard rule file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardRuleFile {
    /// Rule content (Guard DSL)
    pub content: String,
    /// Original file path in the repository
    pub file_path: String,
    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,
}

impl BulkRuleDownloader {
    /// Create a new bulk rule downloader
    ///
    /// # Arguments
    ///
    /// * `storage_dir` - Directory where downloaded rules will be stored
    pub async fn new(storage_dir: PathBuf) -> Result<Self> {
        // Create storage directory if it doesn't exist
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()?;

        let storage = RuleStorage::new(storage_dir.clone()).await?;
        let rule_index = RuleIndex::new(storage_dir.clone()).await?;

        Ok(BulkRuleDownloader {
            storage_dir,
            client,
            storage,
            rule_index,
            max_retries: 3,
            retry_delay_ms: 1000,
            deduplication_enabled: false,
        })
    }

    /// Get the storage directory path
    pub fn get_storage_dir(&self) -> &PathBuf {
        &self.storage_dir
    }

    /// Set maximum retry attempts
    pub fn set_retry_attempts(&mut self, retries: usize) {
        self.max_retries = retries;
    }

    /// Set retry delay in milliseconds
    pub fn set_retry_delay_ms(&mut self, delay: u64) {
        self.retry_delay_ms = delay;
    }

    /// Enable or disable rule deduplication
    pub fn enable_deduplication(&mut self, enabled: bool) {
        self.deduplication_enabled = enabled;
    }

    /// Download rules for a single compliance program
    ///
    /// # Arguments
    ///
    /// * `program` - The compliance program to download rules for
    ///
    /// # Returns
    ///
    /// The downloaded rule set
    pub async fn download_compliance_program_rules(
        &mut self,
        program: &AvailableComplianceProgram,
    ) -> Result<ComplianceRuleSet> {
        // Use GitHub API to download actual rules
        let rule_set = self.download_from_github(program).await?;

        // Store the rule set
        self.storage.store_rule_set(&rule_set).await?;

        // Index the rules
        self.rule_index.index_rule_set(&rule_set).await?;

        Ok(rule_set)
    }

    /// Download rules for multiple compliance programs
    ///
    /// # Arguments
    ///
    /// * `programs` - List of compliance programs to download
    ///
    /// # Returns
    ///
    /// Vector of downloaded rule sets
    pub async fn download_multiple_programs(
        &mut self,
        programs: &[AvailableComplianceProgram],
    ) -> Result<Vec<ComplianceRuleSet>> {
        let mut rule_sets = Vec::new();

        for program in programs {
            let rule_set = self.download_compliance_program_rules(program).await?;
            rule_sets.push(rule_set);
        }

        Ok(rule_sets)
    }

    /// Download rules with progress tracking
    ///
    /// # Arguments
    ///
    /// * `program` - The compliance program to download
    /// * `progress` - Shared progress tracker
    ///
    /// # Returns
    ///
    /// The downloaded rule set
    pub async fn download_with_progress(
        &mut self,
        program: &AvailableComplianceProgram,
        progress: Arc<Mutex<DownloadProgress>>,
    ) -> Result<ComplianceRuleSet> {
        // Start progress tracking
        {
            let mut prog = progress.lock().await;
            prog.start_download(program.estimated_rule_count);
        }

        // Download the rules
        let result = self.download_compliance_program_rules(program).await;

        // Update progress based on result
        {
            let mut prog = progress.lock().await;
            match &result {
                Ok(rule_set) => {
                    prog.files_downloaded = rule_set.rules.len();
                    prog.complete_download();
                }
                Err(err) => {
                    prog.fail_download(err.to_string());
                }
            }
        }

        result
    }

    /// Download rule set from GitHub using the API client
    async fn download_from_github(
        &self,
        _program: &AvailableComplianceProgram,
    ) -> Result<ComplianceRuleSet> {
        // NOTE: This method is deprecated - git-based repository access replaces bulk downloading
        // For now, return an error indicating the new approach should be used
        Err(anyhow!("Bulk rule downloader is deprecated. Use GuardRepositoryManager for git-based rule access."))
    }

}

impl RuleStorage {
    /// Create a new rule storage system
    pub async fn new(storage_dir: PathBuf) -> Result<Self> {
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)?;
        }

        Ok(RuleStorage { storage_dir })
    }

    /// Get the storage directory
    pub fn get_storage_dir(&self) -> &PathBuf {
        &self.storage_dir
    }

    /// Store a rule set to the filesystem
    ///
    /// # Arguments
    ///
    /// * `rule_set` - The rule set to store
    pub async fn store_rule_set(&mut self, rule_set: &ComplianceRuleSet) -> Result<()> {
        let program_dir = self.storage_dir.join(&rule_set.program_name);

        // Create program directory
        if !program_dir.exists() {
            fs::create_dir_all(&program_dir)?;
        }

        // Store individual rule files  
        for rule_file in rule_set.rules.values() {
            let rule_path = program_dir.join(&rule_file.file_path);
            fs::write(&rule_path, &rule_file.content)?;
        }

        // Store metadata
        let metadata_path = program_dir.join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(rule_set)?;
        fs::write(&metadata_path, metadata_json)?;

        Ok(())
    }
}

impl RuleIndex {
    /// Create a new rule index
    pub async fn new(storage_dir: PathBuf) -> Result<Self> {
        let index_dir = storage_dir.join("index");
        if !index_dir.exists() {
            fs::create_dir_all(&index_dir)?;
        }

        Ok(RuleIndex {
            index_dir,
            resource_type_index: HashMap::new(),
            program_index: HashMap::new(),
            rule_content_cache: HashMap::new(),
        })
    }

    /// Index a rule set for fast lookup
    ///
    /// # Arguments
    ///
    /// * `rule_set` - The rule set to index
    pub async fn index_rule_set(&mut self, rule_set: &ComplianceRuleSet) -> Result<()> {
        // Index rules by resource type
        for (rule_name, rule_file) in &rule_set.rules {
            // Extract resource types from rule content
            let resource_types = self.extract_resource_types(&rule_file.content);

            for resource_type in resource_types {
                self.resource_type_index
                    .entry(resource_type)
                    .or_default()
                    .push(rule_name.clone());
            }

            // Index by program
            self.program_index
                .entry(rule_set.program_name.clone())
                .or_default()
                .push(rule_name.clone());

            // Cache rule content
            self.rule_content_cache
                .insert(rule_name.clone(), rule_file.content.clone());
        }

        Ok(())
    }

    /// Find rules that apply to a specific AWS resource type
    ///
    /// # Arguments
    ///
    /// * `resource_type` - AWS resource type (e.g., "AWS::S3::Bucket")
    ///
    /// # Returns
    ///
    /// Vector of rule names that apply to the resource type
    pub async fn find_rules_by_resource_type(&self, resource_type: &str) -> Result<Vec<String>> {
        Ok(self
            .resource_type_index
            .get(resource_type)
            .cloned()
            .unwrap_or_default())
    }

    /// Find all rules in a compliance program
    ///
    /// # Arguments
    ///
    /// * `program_name` - Name of the compliance program
    ///
    /// # Returns
    ///
    /// Vector of rule names in the program
    pub async fn find_rules_by_program(&self, program_name: &str) -> Result<Vec<String>> {
        Ok(self
            .program_index
            .get(program_name)
            .cloned()
            .unwrap_or_default())
    }

    /// Get the content of a specific rule
    ///
    /// # Arguments
    ///
    /// * `rule_name` - Name of the rule
    ///
    /// # Returns
    ///
    /// Rule content or error if not found
    pub async fn get_rule_content(&self, rule_name: &str) -> Result<String> {
        self.rule_content_cache
            .get(rule_name)
            .cloned()
            .ok_or_else(|| anyhow!("Rule '{}' not found in index", rule_name))
    }

    /// Extract AWS resource types from Guard rule content
    fn extract_resource_types(&self, content: &str) -> Vec<String> {
        let mut resource_types = Vec::new();

        // Simple regex-like extraction (in a real implementation, you'd use proper parsing)
        for line in content.lines() {
            if line.trim().contains("AWS::") {
                // Extract AWS resource types
                let parts: Vec<&str> = line.split_whitespace().collect();
                for part in parts {
                    if part.starts_with("AWS::") && part.contains("::") {
                        let resource_type = part.trim_end_matches('{').trim();
                        if !resource_types.contains(&resource_type.to_string()) {
                            resource_types.push(resource_type.to_string());
                        }
                    }
                }
            }
        }

        resource_types
    }
}

impl DownloadProgress {
    /// Create a new download progress tracker
    pub fn new() -> Self {
        DownloadProgress {
            status: DownloadStatus::NotStarted,
            files_downloaded: 0,
            total_files: 0,
            current_file: None,
            started_at: None,
            completed_at: None,
            error_message: None,
        }
    }

    /// Start a download
    pub fn start_download(&mut self, total_files: usize) {
        self.status = DownloadStatus::InProgress;
        self.total_files = total_files;
        self.started_at = Some(Utc::now());
    }

    /// Update the currently downloading file
    pub fn update_current_file(&mut self, file_name: String) {
        self.current_file = Some(file_name);
    }

    /// Increment the downloaded file count
    pub fn increment_downloaded(&mut self) {
        self.files_downloaded += 1;
    }

    /// Mark download as completed
    pub fn complete_download(&mut self) {
        self.status = DownloadStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.current_file = None;
    }

    /// Mark download as failed
    pub fn fail_download(&mut self, error: String) {
        self.status = DownloadStatus::Failed;
        self.error_message = Some(error);
        self.completed_at = Some(Utc::now());
    }
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self::new()
    }
}
