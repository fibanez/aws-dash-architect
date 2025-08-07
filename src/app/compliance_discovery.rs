//! Compliance program discovery and management system.
//!
//! This module provides functionality to discover available compliance programs
//! from the AWS Guard Rules Registry on GitHub, cache them locally, and provide
//! search and filtering capabilities for UI selection.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::app::guard_repository_manager::GuardRepositoryManager;

/// Specific errors that can occur during compliance discovery
#[derive(Debug, Clone, PartialEq)]
pub enum ComplianceDiscoveryError {
    /// GitHub API is unreachable or returned an error
    GitHubApiFailure { status_code: u16, message: String },
    /// Repository structure is empty or invalid
    InvalidRepositoryStructure { message: String },
    /// Failed to download one or more mapping files
    MappingFileDownloadFailure { failed_files: Vec<String>, errors: Vec<String> },
    /// Failed to parse a mapping file
    MappingFileParseFailure { file: String, error: String },
    /// Network connectivity issues
    NetworkFailure { error: String },
    /// General failure with retry possibility
    RetryableFailure { message: String },
}

/// Main client for discovering and managing available compliance programs
#[derive(Debug, Clone)]
pub struct ComplianceDiscovery {
    /// Local cache directory for storing discovered programs
    cache_dir: PathBuf,
    /// Git repository manager for accessing cloned guard rules repository
    repository_manager: GuardRepositoryManager,
    /// Cache refresh interval (in hours)
    cache_refresh_hours: u64,
}

/// Represents an available compliance program discovered from GitHub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableComplianceProgram {
    /// Internal name (e.g., "nist_800_53_rev_5")
    pub name: String,
    /// Display name for UI (e.g., "NIST 800-53 Revision 5")
    pub display_name: String,
    /// Description of the compliance program
    pub description: String,
    /// GitHub path where rules are located
    pub github_path: String,
    /// Estimated number of rules in this program
    pub estimated_rule_count: usize,
    /// Category (Government, Industry, Framework, etc.)
    pub category: String,
    /// Tags for search and filtering
    pub tags: Vec<String>,
}

impl AvailableComplianceProgram {
    /// Convert to the new ComplianceProgram structure
    pub fn to_compliance_program(&self) -> crate::app::cfn_guard::ComplianceProgram {
        use crate::app::cfn_guard::{ComplianceProgram, ComplianceCategory};
        
        let category = match self.category.as_str() {
            "Government" => ComplianceCategory::Government,
            "Industry" => ComplianceCategory::Industry,
            "International" => ComplianceCategory::International,
            "Framework" => ComplianceCategory::Framework,
            _ => ComplianceCategory::Custom(self.category.clone()),
        };
        
        ComplianceProgram::new(
            self.name.clone(),
            self.display_name.clone(),
            self.description.clone(),
            category,
            self.estimated_rule_count,
            self.github_path.clone(),
        )
    }
}

/// Metadata about a specific compliance program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceProgramMetadata {
    /// Internal name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Description
    pub description: String,
    /// GitHub path
    pub github_path: String,
    /// Number of rules
    pub estimated_rule_count: usize,
    /// Last updated date
    pub last_updated: String,
    /// Category
    pub category: String,
}

/// Cache structure for storing discovered compliance programs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceProgramCache {
    /// List of discovered programs
    pub programs: Vec<AvailableComplianceProgram>,
    /// When the cache was last updated
    pub last_updated: DateTime<Utc>,
    /// Cache format version
    pub cache_version: String,
}

// GitHub API client removed - now using git-based local repository access

impl ComplianceDiscovery {
    /// Get the mappings directory path from the repository manager
    pub fn get_mappings_path(&self) -> PathBuf {
        self.repository_manager.get_mappings_path()
    }

    /// Create a new compliance discovery client
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Directory where discovered programs will be cached
    pub async fn new(cache_dir: PathBuf) -> Result<Self> {
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let repository_manager = GuardRepositoryManager::new()?;

        Ok(ComplianceDiscovery {
            cache_dir,
            repository_manager,
            cache_refresh_hours: 24, // Refresh cache daily
        })
    }

    /// Create a new compliance discovery client with default cache directory (non-async)
    /// This creates an uninitialized client that will require async initialization on first use
    pub fn new_with_default_cache() -> Self {
        use std::env;
        
        // Use a default cache directory in the user's data directory
        let cache_dir = dirs::data_dir()
            .unwrap_or_else(|| env::current_dir().unwrap_or_default())
            .join("awsdash")
            .join("compliance_cache");

        // Create repository manager
        let repository_manager = GuardRepositoryManager::new()
            .expect("Failed to initialize GuardRepositoryManager");

        ComplianceDiscovery {
            cache_dir,
            repository_manager,
            cache_refresh_hours: 24,
        }
    }

    /// Get the cache directory path
    pub fn get_cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Discover available compliance programs from GitHub
    ///
    /// This method queries the AWS Guard Rules Registry to find all available
    /// compliance programs and their metadata.
    ///
    /// # Returns
    ///
    /// A vector of available compliance programs
    pub async fn discover_available_programs(&mut self) -> Result<Vec<AvailableComplianceProgram>> {
        // Ensure repository is up to date - this should be done during app startup
        // but we check here as a fallback
        if !self.repository_manager.is_repository_cloned() {
            return Err(anyhow!(
                "Guard rules repository not available. Please restart the application to download it."
            ));
        }

        // Parse compliance programs from local git repository
        let programs = self.parse_local_repository().await?;

        // Cache the results
        self.cache_programs(&programs).await?;

        Ok(programs)
    }

    /// Get cached compliance programs
    ///
    /// # Returns
    ///
    /// Cached programs or error if cache doesn't exist or is invalid
    pub async fn get_cached_programs(&self) -> Result<Vec<AvailableComplianceProgram>> {
        let cache_file = self.cache_dir.join("available_programs.json");

        if !cache_file.exists() {
            return Err(anyhow!("No cached compliance programs found"));
        }

        let content = fs::read_to_string(&cache_file)?;
        let cache: ComplianceProgramCache = serde_json::from_str(&content)?;

        Ok(cache.programs)
    }

    /// Check if cache needs refresh
    ///
    /// # Returns
    ///
    /// True if cache is stale and needs refresh
    pub async fn needs_cache_refresh(&self) -> Result<bool> {
        let cache_file = self.cache_dir.join("available_programs.json");

        if !cache_file.exists() {
            return Ok(true);
        }

        let content = fs::read_to_string(&cache_file)?;
        let cache: ComplianceProgramCache = serde_json::from_str(&content)?;

        let now = Utc::now();
        let cache_age = now.signed_duration_since(cache.last_updated);
        let max_age = chrono::Duration::hours(self.cache_refresh_hours as i64);

        Ok(cache_age > max_age)
    }

    /// Invalidate the cache
    pub async fn invalidate_cache(&mut self) -> Result<()> {
        let cache_file = self.cache_dir.join("available_programs.json");

        if cache_file.exists() {
            fs::remove_file(&cache_file)?;
        }

        Ok(())
    }

    /// Search compliance programs by name or description
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string
    ///
    /// # Returns
    ///
    /// Programs matching the search query
    pub async fn search_programs(&self, query: &str) -> Result<Vec<AvailableComplianceProgram>> {
        let programs = self.get_cached_programs().await?;
        let query_lower = query.to_lowercase();

        let filtered: Vec<AvailableComplianceProgram> = programs
            .into_iter()
            .filter(|program| {
                program.name.to_lowercase().contains(&query_lower)
                    || program.display_name.to_lowercase().contains(&query_lower)
                    || program.description.to_lowercase().contains(&query_lower)
                    || program
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(filtered)
    }

    /// Filter compliance programs by category
    ///
    /// # Arguments
    ///
    /// * `category` - Category to filter by
    ///
    /// # Returns
    ///
    /// Programs in the specified category
    pub async fn filter_by_category(
        &self,
        category: &str,
    ) -> Result<Vec<AvailableComplianceProgram>> {
        let programs = self.get_cached_programs().await?;

        let filtered: Vec<AvailableComplianceProgram> = programs
            .into_iter()
            .filter(|program| program.category == category)
            .collect();

        Ok(filtered)
    }

    /// Parse GitHub repository structure to extract compliance programs
    /// Parse compliance programs from local git repository
    pub async fn parse_local_repository(&self) -> Result<Vec<AvailableComplianceProgram>> {
        let mappings_dir = self.repository_manager.get_mappings_path();
        
        tracing::info!("Looking for mappings in: {:?}", mappings_dir);

        if !mappings_dir.exists() {
            return Err(anyhow!("Mappings directory not found in cloned repository: {:?}", mappings_dir));
        }
        
        // Debug: List directory contents
        match std::fs::read_dir(&mappings_dir) {
            Ok(entries) => {
                let file_list: Vec<String> = entries
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .collect();
                tracing::info!("Found {} files in mappings directory: {:?}", file_list.len(), file_list.iter().take(10).collect::<Vec<_>>());
            }
            Err(e) => {
                tracing::error!("Failed to read mappings directory: {}", e);
            }
        }

        let mut programs = Vec::new();

        // Read all files in the mappings directory
        let mut processed_count = 0;
        let mut error_count = 0;
        
        for entry in fs::read_dir(&mappings_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                tracing::debug!("Checking file: {}", file_name);
                
                if file_name.ends_with(".guard") || file_name.ends_with(".json") {
                    processed_count += 1;
                    tracing::info!("Processing mapping file: {}", file_name);
                    
                    // Parse mapping file to create compliance program
                    match self.create_compliance_program_from_local_file(&path).await {
                        Ok(program) => {
                            tracing::info!("Successfully parsed program: {} ({})", program.display_name, program.name);
                            programs.push(program);
                        }
                        Err(e) => {
                            error_count += 1;
                            tracing::warn!("Failed to process mapping file {:?}: {}. Skipping.", path, e);
                            // Continue processing other files instead of failing completely
                        }
                    }
                } else {
                    tracing::debug!("Skipping non-mapping file: {}", file_name);
                }
            }
        }
        
        tracing::info!("Discovery summary: processed {} files, {} errors, {} programs found", 
            processed_count, error_count, programs.len());

        if programs.is_empty() {
            return Err(anyhow!("No compliance programs found in repository mappings directory"));
        }

        tracing::info!("Discovered {} compliance programs from local repository", programs.len());
        Ok(programs)
    }

    /// Parse repository structure (deprecated - use parse_local_repository)
    #[allow(dead_code)]
    pub async fn parse_repository_structure(
        &self,
        structure: HashMap<String, Vec<String>>,
    ) -> Result<Vec<AvailableComplianceProgram>> {
        let mut programs = Vec::new();

        for (path, files) in structure {
            // Look for mapping files in the mappings directory
            if path == "mappings" {
                // Process .guard mapping files
                for file in files {
                    if file.ends_with(".guard") {
                        // Use the deprecated method but marked as dead code
                        match self.create_compliance_program_from_mapping_file(&file).await {
                            Ok(program) => programs.push(program),
                            Err(e) => {
                                tracing::error!("Failed to process mapping file {}: {}. Compliance discovery incomplete.", file, e);
                                return Err(anyhow!("Failed to process compliance program '{}': {}. Cannot proceed with incomplete compliance data.", file, e));
                            }
                        }
                    }
                }
            }
        }

        Ok(programs)
    }

    /// Cache discovered programs to local filesystem
    async fn cache_programs(&self, programs: &[AvailableComplianceProgram]) -> Result<()> {
        let cache = ComplianceProgramCache {
            programs: programs.to_vec(),
            last_updated: Utc::now(),
            cache_version: "1.0".to_string(),
        };

        let cache_file = self.cache_dir.join("available_programs.json");
        let json = serde_json::to_string_pretty(&cache)?;
        fs::write(&cache_file, json)?;

        Ok(())
    }

    /// Count rules in a local mapping file
    async fn count_rules_in_local_file(&self, file_path: &std::path::Path) -> Result<usize> {
        let content = fs::read_to_string(file_path)?;
        
        // Try parsing as JSON first
        if let Ok(json_content) = serde_json::from_str::<serde_json::Value>(&content) {
            // Extract rule count from JSON structure
            if let Some(rules) = json_content.get("rules") {
                if let Some(rules_array) = rules.as_array() {
                    return Ok(rules_array.len());
                }
            }
            // Fallback: count rule references in JSON
            return Ok(content.matches("\"rule\"").count().max(content.matches("\"Rule\"").count()));
        }
        
        // Parse as Guard DSL
        if file_path.extension().and_then(|ext| ext.to_str()) == Some("guard") {
            // Count rule blocks in Guard DSL format
            let rule_count = content.matches("rule ").count()
                .max(content.matches("Rule ").count())
                .max(content.split('\n')
                    .filter(|line| line.trim_start().starts_with("rule ") || line.trim_start().starts_with("Rule "))
                    .count());
            
            return Ok(rule_count);
        }
        
        // Fallback: estimate based on file size (very rough estimate)
        Ok((content.len() / 500).max(1))
    }

    /// Create a compliance program from a local mapping file
    async fn create_compliance_program_from_local_file(
        &self,
        file_path: &std::path::Path,
    ) -> Result<AvailableComplianceProgram> {
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid file path: {:?}", file_path))?;

        // First try to extract metadata from JSON content if it's a JSON file
        if file_name.ends_with(".json") {
            match self.parse_json_mapping_file(file_path).await {
                Ok(program) => return Ok(program),
                Err(e) => {
                    tracing::warn!("Failed to parse JSON mapping file {}: {}", file_name, e);
                    // Fall through to filename parsing as backup
                }
            }
        }

        // Fallback: Parse the mapping file name to extract program information  
        let (program_id, display_name, description, category) = self.parse_mapping_filename(file_name)
            .ok_or_else(|| anyhow!("Unable to parse mapping filename: {}", file_name))?;
        
        // Read and parse the local mapping file to get accurate rule count
        let rule_count = self.count_rules_in_local_file(file_path).await?;
        
        let tags = self.generate_tags_from_program_info(&program_id, &display_name, &category);
        
        Ok(AvailableComplianceProgram {
            name: program_id,
            display_name,
            description,
            github_path: format!("mappings/{}", file_name),
            estimated_rule_count: rule_count,
            category,
            tags,
        })
    }

    /// Parse a JSON mapping file to extract compliance program metadata
    async fn parse_json_mapping_file(&self, file_path: &std::path::Path) -> Result<AvailableComplianceProgram> {
        let content = fs::read_to_string(file_path)?;
        let json_data: serde_json::Value = serde_json::from_str(&content)?;
        
        // Extract metadata from JSON structure
        let rule_set_name = json_data.get("ruleSetName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing ruleSetName in JSON mapping file"))?;
            
        let description = json_data.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("No description available");
            
        let version = json_data.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0");
            
        // Count the number of mappings (rules)
        let rule_count = json_data.get("mappings")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);
        
        // Generate program details
        let program_id = rule_set_name.to_string(); // Use ruleSetName as unique ID
        let display_name = self.format_description_for_display(description);
        let category = self.categorize_compliance_program(rule_set_name);
        
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
            
        let tags = self.generate_tags_from_program_info(&program_id, &display_name, &category);
        
        Ok(AvailableComplianceProgram {
            name: program_id,
            display_name,
            description: format!("{} (v{})", description, version),
            github_path: format!("mappings/{}", file_name),
            estimated_rule_count: rule_count,
            category,
            tags,
        })
    }

    /// Normalize program ID for internal use
    fn normalize_program_id(&self, rule_set_name: &str) -> String {
        rule_set_name.to_lowercase().replace("-", "_").replace(" ", "_")
    }
    
    /// Format display name for UI presentation
    fn format_display_name(&self, rule_set_name: &str) -> String {
        // Remove "AWS Guard rule set for" prefix first
        let cleaned_name = rule_set_name
            .strip_prefix("AWS Guard rule set for ")
            .unwrap_or(rule_set_name);
        
        // Return the cleaned name as-is since it should already be the proper full name
        // like "Center for Cybersecurity Maturity Model Certification (CMMC) Level 1"
        cleaned_name.to_string()
    }
    
    /// Format description field for display as checkbox label
    fn format_description_for_display(&self, description: &str) -> String {
        // Remove "AWS Guard rule set for" prefix from description if present
        let cleaned_description = description
            .strip_prefix("AWS Guard rule set for ")
            .unwrap_or(description);
        
        // Return the cleaned description as the display name
        cleaned_description.to_string()
    }
    
    fn get_best_display_name(&self, rule_set_name: &str, description: &str) -> String {
        // First try to use the description if it's meaningful
        let cleaned_description = description
            .strip_prefix("AWS Guard rule set for ")
            .unwrap_or(description);
        
        // Check if the description is useful (not empty, not the default fallback)
        if !cleaned_description.is_empty() 
            && cleaned_description != "No description available"
            && cleaned_description.trim().len() > 3 {
            return cleaned_description.to_string();
        }
        
        // Fall back to using the rule set name (cleaned)
        self.format_display_name(rule_set_name)
    }
    
    /// Categorize compliance program by type
    fn categorize_compliance_program(&self, rule_set_name: &str) -> String {
        let name_lower = rule_set_name.to_lowercase();
        
        if name_lower.contains("fedramp") || name_lower.contains("nist") || name_lower.contains("cisa") {
            "Government".to_string()
        } else if name_lower.contains("pci") || name_lower.contains("hipaa") || name_lower.contains("sox") {
            "Industry".to_string()
        } else if name_lower.contains("iso") || name_lower.contains("cis") {
            "International".to_string()
        } else if name_lower.contains("cmmc") || name_lower.contains("framework") {
            "Framework".to_string()
        } else {
            "Custom".to_string()
        }
    }

    /// Create a compliance program from a mapping file (deprecated - use create_compliance_program_from_local_file)
    #[allow(dead_code)]
    async fn create_compliance_program_from_mapping_file(
        &self,
        mapping_file: &str,
    ) -> Result<AvailableComplianceProgram> {
        // This method is deprecated in favor of local file parsing
        // Parse the mapping file name to extract program information
        let (program_id, display_name, description, category) = self.parse_mapping_filename(mapping_file)
            .ok_or_else(|| anyhow!("Unable to parse mapping filename: {}", mapping_file))?;
        
        // Use placeholder rule count since we can't download without HTTP client
        let rule_count = 10; // Placeholder
        
        let tags = self.generate_tags_from_program_info(&program_id, &display_name, &category);
        
        Ok(AvailableComplianceProgram {
            name: program_id,
            display_name,
            description,
            github_path: format!("mappings/{}", mapping_file),
            estimated_rule_count: rule_count,
            category,
            tags,
        })
    }

    /// Parse mapping filename to extract program information
    fn parse_mapping_filename(&self, filename: &str) -> Option<(String, String, String, String)> {
        // Remove "rule_set_" prefix and file extension suffix
        let name_part = filename.strip_prefix("rule_set_")?;
        let name_part = name_part.strip_suffix(".guard")
            .or_else(|| name_part.strip_suffix(".json"))?;
        
        let (program_id, display_name, description, category) = match name_part {
            "nist800_53rev5" => (
                "nist_800_53_rev_5".to_string(),
                "NIST 800-53 Revision 5".to_string(),
                "NIST Cybersecurity Framework controls revision 5".to_string(),
                "Government".to_string(),
            ),
            "nist800_53rev4" => (
                "nist_800_53_rev_4".to_string(),
                "NIST 800-53 Revision 4".to_string(),
                "NIST Cybersecurity Framework controls revision 4".to_string(),
                "Government".to_string(),
            ),
            "pci_dss" => (
                "pci_dss".to_string(),
                "PCI DSS".to_string(),
                "Payment Card Industry Data Security Standard".to_string(),
                "Industry".to_string(),
            ),
            "hipaa" => (
                "hipaa".to_string(),
                "HIPAA".to_string(),
                "Health Insurance Portability and Accountability Act".to_string(),
                "Industry".to_string(),
            ),
            "soc_2" => (
                "soc_2".to_string(),
                "SOC 2".to_string(),
                "Service Organization Control 2 audit framework".to_string(),
                "Industry".to_string(),
            ),
            "fedramp_moderate" => (
                "fedramp_moderate".to_string(),
                "FedRAMP Moderate".to_string(),
                "Federal Risk and Authorization Management Program - Moderate".to_string(),
                "Government".to_string(),
            ),
            "fedramp_low" => (
                "fedramp_low".to_string(),
                "FedRAMP Low".to_string(),
                "Federal Risk and Authorization Management Program - Low".to_string(),
                "Government".to_string(),
            ),
            "nist_800_171" => (
                "nist_800_171".to_string(),
                "NIST 800-171".to_string(),
                "NIST guidelines for protecting CUI in nonfederal systems".to_string(),
                "Government".to_string(),
            ),
            "cis_aws_level_1" => (
                "cis_aws_level_1".to_string(),
                "CIS AWS Foundations Benchmark Level 1".to_string(),
                "Center for Internet Security AWS Foundations Level 1".to_string(),
                "Framework".to_string(),
            ),
            "cis_aws_level_2" => (
                "cis_aws_level_2".to_string(),
                "CIS AWS Foundations Benchmark Level 2".to_string(),
                "Center for Internet Security AWS Foundations Level 2".to_string(),
                "Framework".to_string(),
            ),
            _ => {
                // Generic parsing for unknown programs
                let clean_name = name_part.replace('_', " ");
                let display_name = clean_name
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                
                (
                    name_part.to_string(),
                    display_name.clone(),
                    format!("Compliance program: {}", display_name),
                    "Unknown".to_string(),
                )
            }
        };
        
        Some((program_id, display_name, description, category))
    }


    /// Generate tags from program information
    fn generate_tags_from_program_info(&self, program_id: &str, display_name: &str, category: &str) -> Vec<String> {
        let mut tags = Vec::new();
        
        // Add category tag
        tags.push(category.to_lowercase());
        
        // Add keywords from display name
        for word in display_name.split_whitespace() {
            if word.len() > 2 {
                tags.push(word.to_lowercase());
            }
        }
        
        // Add specific tags based on program ID
        if program_id.contains("nist") {
            tags.extend(["nist".to_string(), "cybersecurity".to_string(), "federal".to_string()]);
        }
        if program_id.contains("pci") {
            tags.extend(["pci".to_string(), "payment".to_string(), "card".to_string()]);
        }
        if program_id.contains("hipaa") {
            tags.extend(["hipaa".to_string(), "healthcare".to_string(), "medical".to_string()]);
        }
        if program_id.contains("cis") {
            tags.extend(["cis".to_string(), "benchmark".to_string(), "security".to_string()]);
        }
        if program_id.contains("fedramp") {
            tags.extend(["fedramp".to_string(), "federal".to_string(), "cloud".to_string()]);
        }
        
        // Remove duplicates and sort
        tags.sort();
        tags.dedup();
        tags
    }

    /// Create a compliance program from GitHub path information
    #[allow(dead_code)]
    fn create_compliance_program_from_path(
        &self,
        program_name: &str,
        github_path: &str,
        rule_count: usize,
    ) -> AvailableComplianceProgram {
        // Generate display name and metadata from program name
        let (display_name, description, category, tags) =
            self.generate_program_metadata(program_name);

        AvailableComplianceProgram {
            name: program_name.to_string(),
            display_name,
            description,
            github_path: github_path.to_string(),
            estimated_rule_count: rule_count,
            category,
            tags,
        }
    }

    /// Generate metadata for a compliance program based on its name
    #[allow(dead_code)]
    fn generate_program_metadata(
        &self,
        program_name: &str,
    ) -> (String, String, String, Vec<String>) {
        let name_lower = program_name.to_lowercase();

        // Generate display name
        let display_name = if name_lower.contains("nist_800_53") {
            if name_lower.contains("rev_5") {
                "NIST 800-53 Revision 5".to_string()
            } else if name_lower.contains("rev_4") {
                "NIST 800-53 Revision 4".to_string()
            } else {
                "NIST 800-53".to_string()
            }
        } else if name_lower.contains("nist_800_171") {
            "NIST 800-171".to_string()
        } else if name_lower.contains("pci") || name_lower.contains("dss") {
            "PCI DSS".to_string()
        } else if name_lower.contains("hipaa") {
            "HIPAA".to_string()
        } else if name_lower.contains("sox") {
            "SOX (Sarbanes-Oxley)".to_string()
        } else if name_lower.contains("fedramp") {
            "FedRAMP".to_string()
        } else {
            // Capitalize and clean up the name
            program_name
                .replace('_', " ")
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().chain(chars).collect(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        };

        // Generate description
        let description = if name_lower.contains("nist") {
            "NIST cybersecurity framework controls and guidelines".to_string()
        } else if name_lower.contains("pci") {
            "Payment Card Industry Data Security Standard requirements".to_string()
        } else if name_lower.contains("hipaa") {
            "Health Insurance Portability and Accountability Act compliance".to_string()
        } else if name_lower.contains("sox") {
            "Sarbanes-Oxley Act financial compliance requirements".to_string()
        } else if name_lower.contains("fedramp") {
            "Federal Risk and Authorization Management Program requirements".to_string()
        } else {
            format!("Compliance controls for {}", display_name)
        };

        // Determine category
        let category = if name_lower.contains("nist") || name_lower.contains("fedramp") {
            "Government".to_string()
        } else if name_lower.contains("pci") || name_lower.contains("sox") {
            "Industry".to_string()
        } else if name_lower.contains("hipaa") {
            "Healthcare".to_string()
        } else {
            "Other".to_string()
        };

        // Generate tags
        let mut tags = Vec::new();
        if name_lower.contains("nist") {
            tags.extend(vec![
                "government".to_string(),
                "cybersecurity".to_string(),
                "federal".to_string(),
            ]);
        }
        if name_lower.contains("pci") {
            tags.extend(vec![
                "payment".to_string(),
                "financial".to_string(),
                "industry".to_string(),
            ]);
        }
        if name_lower.contains("hipaa") {
            tags.extend(vec![
                "healthcare".to_string(),
                "privacy".to_string(),
                "medical".to_string(),
            ]);
        }
        if name_lower.contains("sox") {
            tags.extend(vec![
                "financial".to_string(),
                "audit".to_string(),
                "public-company".to_string(),
            ]);
        }
        if name_lower.contains("fedramp") {
            tags.extend(vec![
                "government".to_string(),
                "cloud".to_string(),
                "federal".to_string(),
            ]);
        }

        // Add common security tags
        tags.extend(vec!["security".to_string(), "compliance".to_string()]);

        (display_name, description, category, tags)
    }
}

// GitHubApiClient implementation removed - replaced with git-based local repository access

// MappingFileContent struct removed - now using direct file parsing
