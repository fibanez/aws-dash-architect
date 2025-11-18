//! Skill Loading Service
//!
//! Loads skill content on-demand and caches for performance.
//! Implements progressive disclosure: metadata is cheap, full content is expensive.

use super::discovery::{SkillDiscoveryService, SkillError, SkillMetadata};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, RwLock};
use tracing::{debug, error, info, warn};

/// A loaded skill with full content
#[derive(Debug, Clone)]
pub struct LoadedSkill {
    /// Skill metadata
    pub metadata: SkillMetadata,
    /// Full SKILL.md content
    pub content: String,
    /// When the skill was loaded
    pub loaded_at: DateTime<Utc>,
}

/// Service for loading skills on-demand
pub struct SkillLoader {
    /// Reference to discovery service
    discovery_service: Arc<SkillDiscoveryService>,
    /// Cache of loaded skills (name -> LoadedSkill)
    loaded_skills: RwLock<HashMap<String, LoadedSkill>>,
}

impl SkillLoader {
    /// Create a new skill loader with reference to discovery service
    pub fn new(discovery_service: Arc<SkillDiscoveryService>) -> Self {
        Self {
            discovery_service,
            loaded_skills: RwLock::new(HashMap::new()),
        }
    }

    /// Load a skill by name
    ///
    /// Returns cached version if available, otherwise loads from disk.
    pub fn load_skill(&self, skill_name: &str) -> Result<LoadedSkill, SkillError> {
        // Check cache first
        {
            if let Ok(loaded) = self.loaded_skills.read() {
                if let Some(skill) = loaded.get(skill_name) {
                    debug!("✅ Skill '{}' found in cache", skill_name);
                    return Ok(skill.clone());
                }
            }
        }

        info!("📖 Loading skill '{}' from disk", skill_name);

        // Find skill metadata
        let metadata = self
            .discovery_service
            .get_skill_by_name(skill_name)
            .ok_or_else(|| {
                warn!("Skill '{}' not found in discovered skills", skill_name);
                SkillError::IoError(format!("Skill '{}' not found", skill_name))
            })?;

        // Load SKILL.md content
        let content = fs::read_to_string(&metadata.skill_md_path).map_err(|e| {
            error!(
                "Failed to read skill file {:?}: {}",
                metadata.skill_md_path, e
            );
            SkillError::IoError(format!(
                "Failed to read {}: {}",
                metadata.skill_md_path.display(),
                e
            ))
        })?;

        let loaded = LoadedSkill {
            metadata: metadata.clone(),
            content,
            loaded_at: Utc::now(),
        };

        // Cache the loaded skill
        match self.loaded_skills.write() {
            Ok(mut cache) => {
                cache.insert(skill_name.to_string(), loaded.clone());
                info!(
                    "✅ Skill '{}' loaded and cached ({} bytes)",
                    skill_name,
                    loaded.content.len()
                );
            }
            Err(e) => {
                error!("Failed to cache loaded skill: {}", e);
                // Continue anyway, we have the loaded skill
            }
        }

        Ok(loaded)
    }

    /// Load an additional skill file (forms.md, reference.md, etc.)
    pub fn load_skill_file(&self, skill_name: &str, filename: &str) -> Result<String, SkillError> {
        debug!(
            "📄 Loading additional file '{}' for skill '{}'",
            filename, skill_name
        );

        // Get skill metadata
        let metadata = self
            .discovery_service
            .get_skill_by_name(skill_name)
            .ok_or_else(|| {
                SkillError::IoError(format!("Skill '{}' not found", skill_name))
            })?;

        // Check if file exists in additional files
        if !metadata.additional_files.contains(&filename.to_string()) {
            warn!(
                "File '{}' not found in skill '{}' additional files: {:?}",
                filename, skill_name, metadata.additional_files
            );
            return Err(SkillError::IoError(format!(
                "File '{}' not found in skill '{}'",
                filename, skill_name
            )));
        }

        // Load the file
        let file_path = metadata.directory_path.join(filename);
        let content = fs::read_to_string(&file_path).map_err(|e| {
            error!("Failed to read file {:?}: {}", file_path, e);
            SkillError::IoError(format!("Failed to read {}: {}", file_path.display(), e))
        })?;

        info!(
            "✅ Loaded additional file '{}' for skill '{}' ({} bytes)",
            filename,
            skill_name,
            content.len()
        );

        Ok(content)
    }

    /// Get metadata for a skill without loading full content
    pub fn get_skill_metadata(&self, skill_name: &str) -> Option<SkillMetadata> {
        self.discovery_service.get_skill_by_name(skill_name)
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        match self.loaded_skills.write() {
            Ok(mut cache) => {
                let count = cache.len();
                cache.clear();
                info!("🧹 Cleared skill cache ({} skills removed)", count);
            }
            Err(e) => {
                error!("Failed to clear skill cache: {}", e);
            }
        }
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        match self.loaded_skills.read() {
            Ok(cache) => {
                let cached_count = cache.len();
                let total_size: usize = cache.values().map(|s| s.content.len()).sum();
                (cached_count, total_size)
            }
            Err(_) => (0, 0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_skill_loader_creation() {
        let discovery = Arc::new(SkillDiscoveryService::new());
        let loader = SkillLoader::new(discovery);
        let (cached, _size) = loader.get_cache_stats();
        assert_eq!(cached, 0);
    }

    #[test]
    fn test_load_skill() {
        // Create test directory structure
        let temp_dir = std::env::temp_dir().join("test-skill-loader");
        fs::create_dir_all(&temp_dir).ok();

        let skill_dir = temp_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).ok();

        // Create SKILL.md
        let skill_md_content = r#"---
name: test-skill
description: A test skill
---
# Test Skill
This is the full skill content with detailed procedures.
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_md_content).ok();

        // Discover and load
        let discovery = Arc::new(SkillDiscoveryService::with_directories(vec![temp_dir.clone()]));
        discovery.discover_skills().ok();

        let loader = SkillLoader::new(discovery);

        // Load skill
        let loaded = loader.load_skill("test-skill");
        assert!(loaded.is_ok());

        let skill = loaded.unwrap();
        assert_eq!(skill.metadata.name, "test-skill");
        assert!(skill.content.contains("full skill content"));

        // Check cache
        let (cached, size) = loader.get_cache_stats();
        assert_eq!(cached, 1);
        assert!(size > 0);

        // Load again (should come from cache)
        let loaded_again = loader.load_skill("test-skill");
        assert!(loaded_again.is_ok());

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_load_additional_file() {
        let temp_dir = std::env::temp_dir().join("test-additional-file");
        fs::create_dir_all(&temp_dir).ok();

        let skill_dir = temp_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).ok();

        // Create SKILL.md
        let skill_md_content = r#"---
name: test-skill
description: A test skill
---
# Test Skill
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_md_content).ok();

        // Create additional file
        fs::write(skill_dir.join("forms.md"), "# Forms\nForm content here.").ok();

        // Discover and load
        let discovery = Arc::new(SkillDiscoveryService::with_directories(vec![temp_dir.clone()]));
        discovery.discover_skills().ok();

        let loader = SkillLoader::new(discovery);

        // Load additional file
        let content = loader.load_skill_file("test-skill", "forms.md");
        assert!(content.is_ok());
        assert!(content.unwrap().contains("Form content"));

        // Try non-existent file
        let not_found = loader.load_skill_file("test-skill", "missing.md");
        assert!(not_found.is_err());

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = std::env::temp_dir().join("test-clear-cache");
        fs::create_dir_all(&temp_dir).ok();

        let skill_dir = temp_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).ok();

        let skill_md_content = r#"---
name: test-skill
description: A test skill
---
# Test Skill
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_md_content).ok();

        let discovery = Arc::new(SkillDiscoveryService::with_directories(vec![temp_dir.clone()]));
        discovery.discover_skills().ok();

        let loader = SkillLoader::new(discovery);

        // Load skill
        loader.load_skill("test-skill").ok();
        let (cached_before, _) = loader.get_cache_stats();
        assert_eq!(cached_before, 1);

        // Clear cache
        loader.clear_cache();
        let (cached_after, _) = loader.get_cache_stats();
        assert_eq!(cached_after, 0);

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }
}
