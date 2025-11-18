//! Skill Discovery Service
//!
//! Scans skill directories and extracts metadata from SKILL.md files.
//! Implements progressive disclosure pattern: loads metadata at startup,
//! full content on-demand.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// Skill metadata extracted from YAML frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Unique skill name (from YAML frontmatter)
    pub name: String,
    /// Brief description of what the skill does
    pub description: String,
    /// Directory containing the skill files
    pub directory_path: PathBuf,
    /// Path to SKILL.md file
    pub skill_md_path: PathBuf,
    /// Additional resource files (forms.md, reference.md, etc.)
    pub additional_files: Vec<String>,
}

/// Errors during skill discovery
#[derive(Debug, Clone)]
pub enum SkillError {
    NoFrontmatter,
    InvalidFrontmatter(String),
    MissingName,
    InvalidName,
    MissingDescription,
    InvalidDescription,
    IoError(String),
}

impl std::fmt::Display for SkillError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillError::NoFrontmatter => write!(f, "No YAML frontmatter found in SKILL.md"),
            SkillError::InvalidFrontmatter(msg) => write!(f, "Invalid YAML frontmatter: {}", msg),
            SkillError::MissingName => write!(f, "Missing 'name' field in frontmatter"),
            SkillError::InvalidName => write!(f, "Invalid 'name' field in frontmatter"),
            SkillError::MissingDescription => {
                write!(f, "Missing 'description' field in frontmatter")
            }
            SkillError::InvalidDescription => {
                write!(f, "Invalid 'description' field in frontmatter")
            }
            SkillError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for SkillError {}

/// Service for discovering skills in configured directories
pub struct SkillDiscoveryService {
    /// Directories to scan for skills
    skill_directories: Vec<PathBuf>,
    /// Cached discovered skills
    discovered_skills: RwLock<Vec<SkillMetadata>>,
}

impl SkillDiscoveryService {
    /// Create a new skill discovery service with default directories
    pub fn new() -> Self {
        let mut skill_directories = Vec::new();

        if let Some(home) = dirs::home_dir() {
            // Anthropic Claude skills directory
            skill_directories.push(home.join(".claude/skills"));

            // AWS Dash skills directory
            skill_directories.push(home.join(".awsdash/skills"));

            info!(
                "🔍 Skill discovery service initialized with directories: {:?}",
                skill_directories
            );
        } else {
            warn!("Could not determine home directory for skill discovery");
        }

        Self {
            skill_directories,
            discovered_skills: RwLock::new(Vec::new()),
        }
    }

    /// Create with custom skill directories (for testing)
    pub fn with_directories(directories: Vec<PathBuf>) -> Self {
        Self {
            skill_directories: directories,
            discovered_skills: RwLock::new(Vec::new()),
        }
    }

    /// Scan all configured directories and discover skills
    ///
    /// Returns the number of skills discovered
    pub fn discover_skills(&self) -> Result<usize, SkillError> {
        let mut skills = Vec::new();

        for skill_dir in &self.skill_directories {
            if !skill_dir.exists() {
                debug!(
                    "Skill directory does not exist, skipping: {:?}",
                    skill_dir
                );
                continue;
            }

            info!("📂 Scanning skill directory: {:?}", skill_dir);

            // Find all SKILL.md files (max depth 2: skill_dir/skill_name/SKILL.md)
            for entry in WalkDir::new(skill_dir)
                .max_depth(2)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                // Check if this is a SKILL.md file
                if path.is_file() && path.file_name() == Some(std::ffi::OsStr::new("SKILL.md")) {
                    debug!("Found SKILL.md: {:?}", path);

                    match self.extract_metadata(path) {
                        Ok(metadata) => {
                            info!(
                                "✅ Discovered skill: {} ({})",
                                metadata.name, metadata.description
                            );
                            skills.push(metadata);
                        }
                        Err(e) => {
                            warn!("Failed to extract metadata from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        let count = skills.len();

        // Cache discovered skills
        match self.discovered_skills.write() {
            Ok(mut guard) => {
                *guard = skills;
                info!("🎯 Skill discovery complete: {} skills found", count);
            }
            Err(e) => {
                error!("Failed to cache discovered skills: {}", e);
                return Err(SkillError::IoError(format!(
                    "Failed to write discovered skills: {}",
                    e
                )));
            }
        }

        Ok(count)
    }

    /// Extract metadata from a SKILL.md file
    fn extract_metadata(&self, skill_md_path: &Path) -> Result<SkillMetadata, SkillError> {
        // Read file content
        let content = fs::read_to_string(skill_md_path)
            .map_err(|e| SkillError::IoError(format!("Failed to read {}: {}", skill_md_path.display(), e)))?;

        // Parse YAML frontmatter
        let frontmatter = extract_yaml_frontmatter(&content)?;

        // Extract name
        let name = frontmatter
            .get(&serde_yaml::Value::String("name".to_string()))
            .ok_or(SkillError::MissingName)?
            .as_str()
            .ok_or(SkillError::InvalidName)?
            .to_string();

        // Extract description
        let description = frontmatter
            .get(&serde_yaml::Value::String("description".to_string()))
            .ok_or(SkillError::MissingDescription)?
            .as_str()
            .ok_or(SkillError::InvalidDescription)?
            .to_string();

        // Get directory path
        let directory_path = skill_md_path
            .parent()
            .ok_or_else(|| SkillError::IoError("SKILL.md has no parent directory".to_string()))?
            .to_path_buf();

        // Find additional files in skill directory
        let additional_files = match fs::read_dir(&directory_path) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_file()
                        && e.path().extension() == Some(std::ffi::OsStr::new("md"))
                        && e.file_name() != std::ffi::OsStr::new("SKILL.md")
                })
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect(),
            Err(e) => {
                warn!(
                    "Failed to list additional files in {:?}: {}",
                    directory_path, e
                );
                Vec::new()
            }
        };

        debug!(
            "Skill {} has {} additional files: {:?}",
            name,
            additional_files.len(),
            additional_files
        );

        Ok(SkillMetadata {
            name,
            description,
            directory_path,
            skill_md_path: skill_md_path.to_path_buf(),
            additional_files,
        })
    }

    /// Get all discovered skills
    pub fn get_discovered_skills(&self) -> Vec<SkillMetadata> {
        match self.discovered_skills.read() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to read discovered skills: {}", e);
                Vec::new()
            }
        }
    }

    /// Get a specific skill by name
    pub fn get_skill_by_name(&self, name: &str) -> Option<SkillMetadata> {
        match self.discovered_skills.read() {
            Ok(guard) => guard.iter().find(|s| s.name == name).cloned(),
            Err(e) => {
                error!("Failed to read discovered skills: {}", e);
                None
            }
        }
    }
}

impl Default for SkillDiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract YAML frontmatter from markdown content
///
/// Expects format:
/// ```markdown
/// ---
/// name: skill-name
/// description: Skill description
/// ---
/// # Skill Content
/// ```
fn extract_yaml_frontmatter(
    content: &str,
) -> Result<HashMap<serde_yaml::Value, serde_yaml::Value>, SkillError> {
    // Check for frontmatter start
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        return Err(SkillError::NoFrontmatter);
    }

    // Find the second --- delimiter
    let content_after_first = if content.starts_with("---\n") {
        &content[4..]
    } else {
        &content[5..]
    };

    let end_pos = content_after_first
        .find("\n---\n")
        .or_else(|| content_after_first.find("\r\n---\r\n"))
        .ok_or(SkillError::InvalidFrontmatter(
            "Missing closing --- delimiter".to_string(),
        ))?;

    let yaml_str = &content_after_first[..end_pos];

    // Parse YAML
    let frontmatter: HashMap<serde_yaml::Value, serde_yaml::Value> =
        serde_yaml::from_str(yaml_str).map_err(|e| {
            SkillError::InvalidFrontmatter(format!("Failed to parse YAML: {}", e))
        })?;

    Ok(frontmatter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_yaml_frontmatter_valid() {
        let content = r#"---
name: test-skill
description: A test skill
---
# Test Skill Content
This is the skill content.
"#;

        let frontmatter = extract_yaml_frontmatter(content).unwrap();
        assert!(frontmatter.contains_key(&serde_yaml::Value::String("name".to_string())));
        assert!(frontmatter.contains_key(&serde_yaml::Value::String("description".to_string())));
    }

    #[test]
    fn test_extract_yaml_frontmatter_missing() {
        let content = r#"# Test Skill
No frontmatter here.
"#;

        let result = extract_yaml_frontmatter(content);
        assert!(matches!(result, Err(SkillError::NoFrontmatter)));
    }

    #[test]
    fn test_extract_yaml_frontmatter_invalid() {
        let content = r#"---
name: test-skill
description: [invalid yaml
---
# Content
"#;

        let result = extract_yaml_frontmatter(content);
        assert!(matches!(result, Err(SkillError::InvalidFrontmatter(_))));
    }

    #[test]
    fn test_skill_discovery_service_creation() {
        let service = SkillDiscoveryService::new();
        assert!(!service.skill_directories.is_empty());
    }

    #[test]
    fn test_skill_discovery_with_test_directory() {
        // Create test directory structure
        let temp_dir = std::env::temp_dir().join("test-skill-discovery");
        fs::create_dir_all(&temp_dir).ok();

        let skill_dir = temp_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).ok();

        // Create SKILL.md
        let skill_md_content = r#"---
name: test-skill
description: A test skill for discovery
---
# Test Skill
This is a test skill.
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_md_content).ok();

        // Create additional file
        fs::write(skill_dir.join("forms.md"), "# Forms").ok();

        // Discover skills
        let service = SkillDiscoveryService::with_directories(vec![temp_dir.clone()]);
        let count = service.discover_skills().unwrap();

        assert_eq!(count, 1);

        let skills = service.get_discovered_skills();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test-skill");
        assert_eq!(skills[0].description, "A test skill for discovery");
        assert_eq!(skills[0].additional_files.len(), 1);
        assert!(skills[0].additional_files.contains(&"forms.md".to_string()));

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_skill_by_name() {
        let temp_dir = std::env::temp_dir().join("test-skill-by-name");
        fs::create_dir_all(&temp_dir).ok();

        let skill_dir = temp_dir.join("aws-ec2");
        fs::create_dir_all(&skill_dir).ok();

        let skill_md_content = r#"---
name: aws-ec2-troubleshooting
description: EC2 troubleshooting procedures
---
# EC2 Troubleshooting
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_md_content).ok();

        let service = SkillDiscoveryService::with_directories(vec![temp_dir.clone()]);
        service.discover_skills().unwrap();

        let skill = service.get_skill_by_name("aws-ec2-troubleshooting");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().name, "aws-ec2-troubleshooting");

        let not_found = service.get_skill_by_name("non-existent");
        assert!(not_found.is_none());

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }
}
