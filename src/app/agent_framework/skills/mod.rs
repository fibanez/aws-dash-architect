//! Agent Skills System
//!
//! Implements Anthropic-style Agent Skills with progressive disclosure:
//! - Discovery: Scan directories and extract metadata
//! - Loading: Load skill content on-demand
//! - Management: Global singleton for skill access

pub mod discovery;
pub mod loader;
pub mod manager;

pub use discovery::{SkillDiscoveryService, SkillError, SkillMetadata};
pub use loader::{LoadedSkill, SkillLoader};
pub use manager::{get_global_skill_manager, initialize_skill_system, SkillManager};
