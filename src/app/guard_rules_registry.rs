//! AWS CloudFormation Guard Rules Registry client and caching system.
//!
//! This module provides functionality to download, cache, and manage CloudFormation Guard
//! rules from the AWS Guard Rules Registry. It supports multiple compliance programs
//! and provides offline capabilities through local caching.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::time::Duration;

use crate::app::cfn_guard::ComplianceProgram;

/// Client for downloading and managing Guard rules from the AWS Rules Registry
#[derive(Debug, Clone)]
pub struct GuardRulesRegistry {
    /// Local cache directory for storing downloaded rules
    cache_dir: PathBuf,
    /// HTTP client for downloading rules
    client: reqwest::Client,
    /// Base URL for the AWS Guard Rules Registry
    base_url: String,
}

/// Metadata about a set of compliance rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMetadata {
    /// Version of the rules
    pub version: String,
    /// When the rules were last updated
    pub last_updated: String,
    /// Number of rules in the compliance program
    pub rules_count: usize,
    /// The compliance program these rules belong to
    pub compliance_program: ComplianceProgram,
    /// Source URL where the rules were downloaded from
    pub source_url: String,
}

/// Version specifications for rules
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleVersion {
    /// Use the latest available version
    Latest,
    /// Use a specific version
    Specific(String),
}

impl GuardRulesRegistry {
    /// Create a new Guard Rules Registry client
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Directory where downloaded rules will be cached
    ///
    /// # Returns
    ///
    /// A Result containing the initialized registry client
    pub async fn new(cache_dir: PathBuf) -> Result<Self> {
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(GuardRulesRegistry {
            cache_dir,
            client,
            base_url:
                "https://raw.githubusercontent.com/aws-cloudformation/aws-guard-rules-registry/main"
                    .to_string(),
        })
    }

    /// Get the cache directory path
    pub fn get_cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Download rules for a specific compliance program
    ///
    /// This method downloads the latest rules from the AWS Guard Rules Registry
    /// and caches them locally for offline use.
    ///
    /// # Arguments
    ///
    /// * `program` - The compliance program to download rules for
    ///
    /// # Returns
    ///
    /// A HashMap mapping rule names to their content
    pub async fn download_compliance_rules(
        &mut self,
        program: ComplianceProgram,
    ) -> Result<HashMap<String, String>> {
        // Try to download from AWS Guard Rules Registry first
        match self.download_from_aws_registry(&program).await {
            Ok(rules) => {
                // Cache the downloaded rules
                self.cache_rules(&program, &rules).await?;
                Ok(rules)
            }
            Err(e) => {
                log::warn!(
                    "Failed to download rules from AWS registry for {:?}: {}",
                    program,
                    e
                );
                // Fall back to placeholder rules for development
                let rules = self.get_placeholder_rules(&program).await?;
                self.cache_rules(&program, &rules).await?;
                Ok(rules)
            }
        }
    }

    /// Download rules from the official AWS Guard Rules Registry
    async fn download_from_aws_registry(
        &self,
        program: &ComplianceProgram,
    ) -> Result<HashMap<String, String>> {
        let mut rules = HashMap::new();

        // Use the GitHub path from the compliance program
        // If no path is specified, try to infer from the program ID
        let program_path = if !program.github_path.is_empty() {
            // Extract the path from the mapping file path (remove "mappings/" prefix and ".guard" suffix)
            if program.github_path.starts_with("mappings/") && program.github_path.ends_with(".guard") {
                let path_without_prefix = &program.github_path[9..]; // Remove "mappings/"
                let path_without_suffix = &path_without_prefix[..path_without_prefix.len()-6]; // Remove ".guard"
                
                // Convert mapping file name to rules directory path
                match path_without_suffix {
                    "rule_set_nist800_53rev5" => "compliance/cis-aws-foundations-benchmark/nist-800-53-rev5",
                    "rule_set_nist800_53rev4" => "compliance/cis-aws-foundations-benchmark/nist-800-53-rev4", 
                    "rule_set_pci_dss" => "compliance/pci-dss-3.2.1",
                    "rule_set_hipaa" => "compliance/hipaa-security-rule-2003",
                    "rule_set_soc_2" => "compliance/soc-2-type-ii",
                    "rule_set_fedramp" => "compliance/fedramp-moderate-baseline",
                    "rule_set_nist_800_171" => "compliance/nist-800-171",
                    _ => {
                        log::warn!("Unknown mapping file path: {}, skipping rule download", program.github_path);
                        return Ok(rules);
                    }
                }
            } else {
                log::warn!("Invalid GitHub path format: {}, skipping rule download", program.github_path);
                return Ok(rules);
            }
        } else {
            log::warn!("No GitHub path specified for program: {}, skipping rule download", program.id);
            return Ok(rules);
        };

        // Construct the URL to download the compliance program's rules
        let rules_url = format!("{}/rules/{}", self.base_url, program_path);

        // Download the directory listing to find rule files
        match self.download_directory_listing(&rules_url).await {
            Ok(rule_files) => {
                // Download each rule file
                for rule_file in rule_files {
                    if rule_file.ends_with(".guard") {
                        let rule_url = format!("{}/{}", rules_url, rule_file);
                        match self.download_rule_file(&rule_url).await {
                            Ok(content) => {
                                let rule_name = rule_file.trim_end_matches(".guard").to_string();
                                rules.insert(rule_name, content);
                            }
                            Err(e) => {
                                log::warn!("Failed to download rule file {}: {}", rule_file, e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(anyhow!(
                    "Failed to download directory listing for {}: {}",
                    program_path,
                    e
                ));
            }
        }

        if rules.is_empty() {
            return Err(anyhow!(
                "No rules found for compliance program {:?}",
                program
            ));
        }

        Ok(rules)
    }

    /// Download directory listing from GitHub API to find rule files
    async fn download_directory_listing(&self, url: &str) -> Result<Vec<String>> {
        // Convert raw GitHub URL to API URL
        let api_url = url
            .replace("raw.githubusercontent.com", "api.github.com/repos")
            .replace("/main/", "/contents/")
            .replace("https://api.github.com/repos/aws-cloudformation/aws-guard-rules-registry/contents/", 
                     "https://api.github.com/repos/aws-cloudformation/aws-guard-rules-registry/contents/");

        let response = self
            .client
            .get(&api_url)
            .header("User-Agent", "aws-dash-cfn-guard")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch directory listing: {}",
                response.status()
            ));
        }

        let files: Vec<serde_json::Value> = response.json().await?;
        let mut rule_files = Vec::new();

        for file in files {
            if let Some(name) = file.get("name").and_then(|n| n.as_str()) {
                if name.ends_with(".guard") {
                    rule_files.push(name.to_string());
                }
            }
        }

        Ok(rule_files)
    }

    /// Download a single rule file content
    async fn download_rule_file(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .header("User-Agent", "aws-dash-cfn-guard")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download rule file: {}",
                response.status()
            ));
        }

        let content = response.text().await?;
        Ok(content)
    }

    /// Get cached rules for a compliance program
    ///
    /// # Arguments
    ///
    /// * `program` - The compliance program to get cached rules for
    ///
    /// # Returns
    ///
    /// A HashMap of cached rules, or an error if no cache exists
    pub async fn get_cached_rules(
        &self,
        program: ComplianceProgram,
    ) -> Result<HashMap<String, String>> {
        let cache_path = self.get_program_cache_path(&program);

        if !cache_path.exists() {
            return Err(anyhow!("No cached rules found for {:?}", program));
        }

        let mut rules = HashMap::new();

        // Read all .guard files from the cache directory
        for entry in fs::read_dir(&cache_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("guard") {
                let rule_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let content = fs::read_to_string(&path)?;
                rules.insert(rule_name, content);
            }
        }

        Ok(rules)
    }

    /// Get metadata about rules for a compliance program
    ///
    /// # Arguments
    ///
    /// * `program` - The compliance program to get metadata for
    ///
    /// # Returns
    ///
    /// Metadata about the rules
    pub async fn get_rule_metadata(&self, program: ComplianceProgram) -> Result<RuleMetadata> {
        // For now, return placeholder metadata
        // TODO: Implement actual metadata retrieval from AWS Registry
        Ok(RuleMetadata {
            version: "1.0.0".to_string(),
            last_updated: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            rules_count: self.get_placeholder_rule_count(&program),
            compliance_program: program.clone(),
            source_url: format!("{}/rules/{:?}", self.base_url, program),
        })
    }

    /// Check if updates are available for a compliance program
    ///
    /// # Arguments
    ///
    /// * `program` - The compliance program to check updates for
    ///
    /// # Returns
    ///
    /// True if updates are available, false otherwise
    pub async fn check_for_updates(&self, program: ComplianceProgram) -> Result<bool> {
        let cache_path = self.get_program_cache_path(&program);

        // If no cache exists, updates are available
        if !cache_path.exists() {
            return Ok(true);
        }

        // For now, always return true to indicate updates might be available
        // TODO: Implement actual version checking against AWS Registry
        Ok(true)
    }

    /// Clear cached rules for a compliance program
    ///
    /// # Arguments
    ///
    /// * `program` - Optional compliance program to clear cache for. If None, clears all cache.
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub async fn clear_cache(&mut self, program: Option<ComplianceProgram>) -> Result<()> {
        match program {
            Some(prog) => {
                let cache_path = self.get_program_cache_path(&prog);
                if cache_path.exists() {
                    fs::remove_dir_all(&cache_path)?;
                }
            }
            None => {
                // Clear entire cache directory
                if self.cache_dir.exists() {
                    fs::remove_dir_all(&self.cache_dir)?;
                    fs::create_dir_all(&self.cache_dir)?;
                }
            }
        }
        Ok(())
    }

    /// Get the cache path for a specific compliance program
    fn get_program_cache_path(&self, program: &ComplianceProgram) -> PathBuf {
        // Use the program ID as the directory name, sanitized for filesystem
        let program_name = program.id.replace(' ', "_").replace('/', "_");
        self.cache_dir.join(program_name)
    }

    /// Cache rules to local filesystem
    async fn cache_rules(
        &self,
        program: &ComplianceProgram,
        rules: &HashMap<String, String>,
    ) -> Result<()> {
        let cache_path = self.get_program_cache_path(program);

        // Create cache directory for this program
        if !cache_path.exists() {
            fs::create_dir_all(&cache_path)?;
        }

        // Write each rule to a separate file
        for (rule_name, rule_content) in rules {
            let rule_file = cache_path.join(format!("{}.guard", rule_name));
            fs::write(&rule_file, rule_content)?;
        }

        // Write metadata file
        let metadata_file = cache_path.join("metadata.json");
        let metadata = RuleMetadata {
            version: "1.0.0".to_string(),
            last_updated: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            rules_count: rules.len(),
            compliance_program: program.clone(),
            source_url: format!("{}/rules/{:?}", self.base_url, program),
        };

        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        fs::write(&metadata_file, metadata_json)?;

        Ok(())
    }

    /// Get placeholder rules for testing and development
    /// TODO: Replace with actual AWS Guard Rules Registry integration
    async fn get_placeholder_rules(
        &self,
        program: &ComplianceProgram,
    ) -> Result<HashMap<String, String>> {
        let mut rules = HashMap::new();

        match program.id.as_str() {
            "nist_800_53_rev_5" => {
                rules.insert(
                    "S3_BUCKET_SSL_REQUESTS_ONLY".to_string(),
                    r#"
# S3 buckets should require SSL requests
rule s3_bucket_ssl_requests_only {
    AWS::S3::Bucket {
        Properties {
            BucketPolicy exists
            BucketPolicy.PolicyDocument.Statement[*] {
                Effect == "Deny"
                Principal == "*"
                Action == "s3:*"
                Condition.Bool."aws:SecureTransport" == "false"
            }
        }
    }
}
                    "#
                    .trim()
                    .to_string(),
                );

                rules.insert(
                    "S3_BUCKET_PUBLIC_READ_PROHIBITED".to_string(),
                    r#"
# S3 buckets should not allow public read access
rule s3_bucket_public_read_prohibited {  
    AWS::S3::Bucket {
        Properties {
            PublicReadPolicy != true
        }
    }
}
                    "#
                    .trim()
                    .to_string(),
                );
            }
            "pci_dss" => {
                rules.insert(
                    "EC2_SECURITY_GROUP_ATTACHED_TO_ENI".to_string(),
                    r#"
# Security groups should be attached to ENIs
rule ec2_security_group_attached_to_eni {
    AWS::EC2::SecurityGroup {
        # Security group rules here
    }
}
                    "#
                    .trim()
                    .to_string(),
                );
            }
            "hipaa" => {
                rules.insert(
                    "RDS_INSTANCE_ENCRYPTION_ENABLED".to_string(),
                    r#"
# RDS instances should have encryption enabled
rule rds_instance_encryption_enabled {
    AWS::RDS::DBInstance {
        Properties {
            StorageEncrypted == true
        }
    }
}
                    "#
                    .trim()
                    .to_string(),
                );
            }
            id if id.starts_with("custom_") => {
                // Return empty rules for custom programs
                return Ok(rules);
            }
            _ => {
                // Add placeholder rules for other programs
                rules.insert(
                    "GENERIC_COMPLIANCE_RULE".to_string(),
                    format!(
                        r#"
# Generic compliance rule for {:?}
rule generic_compliance_rule {{
    # Placeholder rule content
}}
                    "#,
                        program
                    )
                    .trim()
                    .to_string(),
                );
            }
        }

        Ok(rules)
    }

    /// Get placeholder rule count for a compliance program
    fn get_placeholder_rule_count(&self, program: &ComplianceProgram) -> usize {
        // Use the actual rule count from the program metadata
        program.rule_count
    }
}
