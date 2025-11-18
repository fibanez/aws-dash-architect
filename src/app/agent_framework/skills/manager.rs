//! Global Skill Manager
//!
//! Provides a global singleton for accessing the skill system.
//! Initialized at application startup, accessible throughout the application.

use super::discovery::SkillDiscoveryService;
use super::loader::SkillLoader;
use super::{LoadedSkill, SkillError, SkillMetadata};
use std::sync::{Arc, RwLock};
use tracing::{error, info, warn};

/// Global skill manager singleton
static GLOBAL_SKILL_MANAGER: RwLock<Option<Arc<SkillManager>>> = RwLock::new(None);

/// Central manager for the skill system
pub struct SkillManager {
    /// Skill discovery service
    pub discovery: Arc<SkillDiscoveryService>,
    /// Skill loading service
    pub loader: Arc<SkillLoader>,
}

impl SkillManager {
    /// Create a new skill manager
    pub fn new() -> Self {
        let discovery = Arc::new(SkillDiscoveryService::new());
        let loader = Arc::new(SkillLoader::new(discovery.clone()));

        info!("🎯 Skill manager created");

        Self { discovery, loader }
    }

    /// Create with custom discovery service (for testing)
    pub fn with_discovery(discovery: Arc<SkillDiscoveryService>) -> Self {
        let loader = Arc::new(SkillLoader::new(discovery.clone()));

        Self { discovery, loader }
    }

    /// Discover all skills
    ///
    /// Scans configured directories and extracts metadata.
    /// Should be called at initialization.
    pub fn discover_skills(&self) -> Result<usize, SkillError> {
        self.discovery.discover_skills()
    }

    /// Get all discovered skill metadata (cheap operation)
    pub fn get_all_skill_metadata(&self) -> Vec<SkillMetadata> {
        self.discovery.get_discovered_skills()
    }

    /// Load a skill by name (expensive operation, loads full content)
    pub fn load_skill(&self, skill_name: &str) -> Result<LoadedSkill, SkillError> {
        self.loader.load_skill(skill_name)
    }

    /// Load an additional skill file
    pub fn load_skill_file(&self, skill_name: &str, filename: &str) -> Result<String, SkillError> {
        self.loader.load_skill_file(skill_name, filename)
    }

    /// Get skill metadata without loading content
    pub fn get_skill_metadata(&self, skill_name: &str) -> Option<SkillMetadata> {
        self.loader.get_skill_metadata(skill_name)
    }

    /// Refresh skills (rediscover and clear cache)
    pub fn refresh(&self) -> Result<usize, SkillError> {
        info!("🔄 Refreshing skill system");

        // Clear loader cache
        self.loader.clear_cache();

        // Rediscover skills
        let count = self.discovery.discover_skills()?;

        info!("✅ Skill refresh complete: {} skills discovered", count);
        Ok(count)
    }

    /// Get cache statistics (cached count, total bytes)
    pub fn get_cache_stats(&self) -> (usize, usize) {
        self.loader.get_cache_stats()
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the global skill system
///
/// Should be called once at application startup.
/// Discovers skills and sets up the global manager.
pub fn initialize_skill_system() -> Result<usize, SkillError> {
    info!("🚀 Initializing skill system");

    let manager = Arc::new(SkillManager::new());

    // Discover skills
    let count = manager.discover_skills()?;

    // Set global
    match GLOBAL_SKILL_MANAGER.write() {
        Ok(mut guard) => {
            *guard = Some(manager);
            info!(
                "✅ Skill system initialized: {} skills discovered",
                count
            );
        }
        Err(e) => {
            error!("Failed to set global skill manager: {}", e);
            return Err(SkillError::IoError(format!(
                "Failed to initialize skill system: {}",
                e
            )));
        }
    }

    Ok(count)
}

/// Get the global skill manager
///
/// Returns None if the skill system hasn't been initialized.
pub fn get_global_skill_manager() -> Option<Arc<SkillManager>> {
    match GLOBAL_SKILL_MANAGER.read() {
        Ok(guard) => {
            let manager = guard.clone();
            if manager.is_none() {
                warn!("Skill system not initialized, call initialize_skill_system() first");
            }
            manager
        }
        Err(e) => {
            error!("Failed to read global skill manager: {}", e);
            None
        }
    }
}

/// Clear the global skill manager (for testing)
#[cfg(test)]
pub fn clear_global_skill_manager() {
    if let Ok(mut guard) = GLOBAL_SKILL_MANAGER.write() {
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_skill_manager_creation() {
        let manager = SkillManager::new();
        let metadata = manager.get_all_skill_metadata();
        // Should be empty initially
        assert_eq!(metadata.len(), 0);
    }

    #[test]
    fn test_skill_manager_discovery() {
        let temp_dir = std::env::temp_dir().join("test-skill-manager");
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
        let manager = SkillManager::with_discovery(discovery);

        let count = manager.discover_skills().unwrap();
        assert_eq!(count, 1);

        let metadata = manager.get_all_skill_metadata();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].name, "test-skill");

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_skill_manager_load() {
        let temp_dir = std::env::temp_dir().join("test-skill-manager-load");
        fs::create_dir_all(&temp_dir).ok();

        let skill_dir = temp_dir.join("test-skill");
        fs::create_dir_all(&skill_dir).ok();

        let skill_md_content = r#"---
name: test-skill
description: A test skill
---
# Test Skill
Full content here.
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_md_content).ok();

        let discovery = Arc::new(SkillDiscoveryService::with_directories(vec![temp_dir.clone()]));
        let manager = SkillManager::with_discovery(discovery);
        manager.discover_skills().ok();

        // Load skill
        let loaded = manager.load_skill("test-skill");
        assert!(loaded.is_ok());

        let skill = loaded.unwrap();
        assert!(skill.content.contains("Full content"));

        // Check cache
        let (cached, size) = manager.get_cache_stats();
        assert_eq!(cached, 1);
        assert!(size > 0);

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_skill_manager_refresh() {
        let temp_dir = std::env::temp_dir().join("test-skill-manager-refresh");
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
        let manager = SkillManager::with_discovery(discovery);
        manager.discover_skills().ok();

        // Load skill
        manager.load_skill("test-skill").ok();
        let (cached_before, _) = manager.get_cache_stats();
        assert_eq!(cached_before, 1);

        // Refresh
        let count = manager.refresh().unwrap();
        assert_eq!(count, 1);

        // Cache should be cleared
        let (cached_after, _) = manager.get_cache_stats();
        assert_eq!(cached_after, 0);

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_global_skill_manager() {
        // Clear any existing global manager
        clear_global_skill_manager();

        // Should be None initially
        assert!(get_global_skill_manager().is_none());

        // Initialize (will discover real skills if they exist)
        let _ = initialize_skill_system();

        // Should be Some now
        assert!(get_global_skill_manager().is_some());

        // Clean up
        clear_global_skill_manager();
    }
}
