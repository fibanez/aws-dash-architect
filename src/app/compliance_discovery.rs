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
use tokio::time::Duration;

/// Main client for discovering and managing available compliance programs
#[derive(Debug)]
pub struct ComplianceDiscovery {
    /// Local cache directory for storing discovered programs
    cache_dir: PathBuf,
    /// GitHub API client for fetching repository information
    github_client: GitHubApiClient,
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

/// GitHub API client for accessing AWS Guard Rules Registry
#[derive(Debug, Clone)]
pub struct GitHubApiClient {
    /// HTTP client for API requests
    client: reqwest::Client,
    /// Base URL for GitHub API
    api_base_url: String,
    /// Repository owner/name
    repository: String,
}

/// GitHub API response for repository contents
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubContent {
    pub name: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub path: String,
    pub size: Option<u64>,
    pub download_url: Option<String>,
    pub git_url: Option<String>,
}

/// GitHub API response for repository tree
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubTree {
    pub sha: String,
    pub url: String,
    pub tree: Vec<GitHubTreeItem>,
    pub truncated: bool,
}

/// Individual item in GitHub tree response
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubTreeItem {
    pub path: String,
    pub mode: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub sha: String,
    pub size: Option<u64>,
    pub url: Option<String>,
}

impl ComplianceDiscovery {
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

        let github_client = GitHubApiClient::new().await?;

        Ok(ComplianceDiscovery {
            cache_dir,
            github_client,
            cache_refresh_hours: 24, // Refresh cache daily
        })
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
        // Get repository structure from GitHub
        let repo_structure = self.github_client.get_repository_structure().await?;
        
        // Parse structure to extract compliance programs
        let programs = self.parse_repository_structure(repo_structure).await?;
        
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
                    || program.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
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
    pub async fn filter_by_category(&self, category: &str) -> Result<Vec<AvailableComplianceProgram>> {
        let programs = self.get_cached_programs().await?;
        
        let filtered: Vec<AvailableComplianceProgram> = programs
            .into_iter()
            .filter(|program| program.category == category)
            .collect();
        
        Ok(filtered)
    }

    /// Parse GitHub repository structure to extract compliance programs
    pub async fn parse_repository_structure(
        &self,
        structure: HashMap<String, Vec<String>>,
    ) -> Result<Vec<AvailableComplianceProgram>> {
        let mut programs = Vec::new();
        
        for (path, files) in structure {
            // Look for Guard rule directories
            if path.contains("rules/") && path.contains("cfn-guard") {
                // Extract program name from path
                let path_parts: Vec<&str> = path.split('/').collect();
                if let Some(program_name) = path_parts.last() {
                    // Count .guard files
                    let rule_count = files.iter().filter(|f| f.ends_with(".guard")).count();
                    
                    if rule_count > 0 {
                        let program = self.create_compliance_program_from_path(
                            program_name,
                            &path,
                            rule_count,
                        );
                        programs.push(program);
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

    /// Create a compliance program from GitHub path information
    fn create_compliance_program_from_path(
        &self,
        program_name: &str,
        github_path: &str,
        rule_count: usize,
    ) -> AvailableComplianceProgram {
        // Generate display name and metadata from program name
        let (display_name, description, category, tags) = self.generate_program_metadata(program_name);
        
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
    fn generate_program_metadata(&self, program_name: &str) -> (String, String, String, Vec<String>) {
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
            tags.extend(vec!["government".to_string(), "cybersecurity".to_string(), "federal".to_string()]);
        }
        if name_lower.contains("pci") {
            tags.extend(vec!["payment".to_string(), "financial".to_string(), "industry".to_string()]);
        }
        if name_lower.contains("hipaa") {
            tags.extend(vec!["healthcare".to_string(), "privacy".to_string(), "medical".to_string()]);
        }
        if name_lower.contains("sox") {
            tags.extend(vec!["financial".to_string(), "audit".to_string(), "public-company".to_string()]);
        }
        if name_lower.contains("fedramp") {
            tags.extend(vec!["government".to_string(), "cloud".to_string(), "federal".to_string()]);
        }

        // Add common security tags
        tags.extend(vec!["security".to_string(), "compliance".to_string()]);

        (display_name, description, category, tags)
    }
}

impl GitHubApiClient {
    /// Create a new GitHub API client
    pub async fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("aws-dash-guard-integration/1.0")
            .build()?;

        Ok(GitHubApiClient {
            client,
            api_base_url: "https://api.github.com".to_string(),
            repository: "aws-cloudformation/aws-guard-rules-registry".to_string(),
        })
    }

    /// Get the repository structure from GitHub
    ///
    /// # Returns
    ///
    /// HashMap mapping directory paths to lists of files
    pub async fn get_repository_structure(&self) -> Result<HashMap<String, Vec<String>>> {
        // Get the repository tree using GitHub API
        let tree_url = format!(
            "{}/repos/{}/git/trees/main?recursive=1",
            self.api_base_url, self.repository
        );

        let response = self.client
            .get(&tree_url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        if !response.status().is_success() {
            // Fall back to placeholder structure if API fails
            tracing::warn!("GitHub API request failed with status: {}, falling back to placeholder data", response.status());
            return self.get_placeholder_structure().await;
        }

        let tree: GitHubTree = response.json().await?;
        let mut structure = HashMap::new();

        // Group files by their directory paths
        for item in tree.tree {
            if item.item_type == "blob" && item.path.contains("rules/") && item.path.ends_with(".guard") {
                // Extract directory path
                if let Some(dir_path) = item.path.rfind('/') {
                    let directory = item.path[..dir_path].to_string();
                    let filename = item.path[dir_path + 1..].to_string();
                    
                    structure
                        .entry(directory)
                        .or_insert_with(Vec::new)
                        .push(filename);
                }
            }
        }

        // If we got empty results, fall back to placeholder
        if structure.is_empty() {
            tracing::warn!("Got empty repository structure from GitHub API, falling back to placeholder data");
            return self.get_placeholder_structure().await;
        }

        Ok(structure)
    }

    /// Get placeholder repository structure as fallback
    async fn get_placeholder_structure(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut structure = HashMap::new();
        
        // Add some realistic placeholder paths based on the actual AWS Guard Rules Registry
        structure.insert(
            "rules/aws-control-tower/cfn-guard/nist_800_53_rev_5".to_string(),
            vec![
                "s3_bucket_ssl_requests_only.guard".to_string(),
                "iam_password_policy.guard".to_string(),
                "ec2_security_group_attached_to_eni.guard".to_string(),
                "rds_instance_encryption_enabled.guard".to_string(),
                "cloudtrail_enabled_in_all_regions.guard".to_string(),
            ]
        );
        
        structure.insert(
            "rules/aws-control-tower/cfn-guard/pci_dss".to_string(),
            vec![
                "rds_storage_encrypted.guard".to_string(),
                "s3_bucket_public_access_prohibited.guard".to_string(),
                "cloudtrail_enabled.guard".to_string(),
            ]
        );
        
        structure.insert(
            "rules/aws-control-tower/cfn-guard/hipaa".to_string(),
            vec![
                "s3_bucket_server_side_encryption_enabled.guard".to_string(),
                "rds_instance_encryption_enabled.guard".to_string(),
            ]
        );

        structure.insert(
            "rules/aws-control-tower/cfn-guard/nist_800_171".to_string(),
            vec![
                "s3_bucket_encryption_enabled.guard".to_string(),
                "cloudwatch_log_group_encrypted.guard".to_string(),
            ]
        );

        structure.insert(
            "rules/aws-control-tower/cfn-guard/sox".to_string(),
            vec![
                "cloudtrail_log_file_validation_enabled.guard".to_string(),
                "s3_bucket_versioning_enabled.guard".to_string(),
            ]
        );
        
        Ok(structure)
    }

    /// Download the content of a specific file from GitHub
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file in the repository
    ///
    /// # Returns
    ///
    /// File content as string
    pub async fn download_file_content(&self, file_path: &str) -> Result<String> {
        let contents_url = format!(
            "{}/repos/{}/contents/{}",
            self.api_base_url, self.repository, file_path
        );

        let response = self.client
            .get(&contents_url)
            .header("Accept", "application/vnd.github.v3.raw")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download file {}: HTTP {}",
                file_path,
                response.status()
            ));
        }

        let content = response.text().await?;
        Ok(content)
    }
}