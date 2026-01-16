//! Shared Agent Types
//!
//! Core types used by both V1 (legacy) and V2 agent systems.
//! These types provide common identification, status tracking, and metadata
//! for agents across different implementations.

#![warn(clippy::all, rust_2018_idioms)]

use chrono::{DateTime, Utc};
use std::fmt;
use uuid::Uuid;

use super::model_selection::AgentModel;

/// Unique identifier for an agent instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Create a new unique agent ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is currently running
    Running,
    /// Agent execution is paused
    Paused,
    /// Agent completed successfully
    Completed,
    /// Agent failed with error message
    Failed(String),
    /// Agent was cancelled by user
    Cancelled,
}

/// Type of agent and its specialized capabilities
///
/// Agents can be parent agents (TaskManager) or child agents (TaskWorker, PageBuilderWorker).
/// Child agents always have a reference to their parent agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentType {
    /// Task management agent that orchestrates multiple task workers
    ///
    /// Capabilities:
    /// - Break down complex goals into tasks
    /// - Spawn task-worker agents via start_task tool
    /// - Spawn page-builder-worker agents via start_page_builder tool
    /// - Open tools via open_tool tool
    /// - Full conversation context with the user
    TaskManager,

    /// Task execution agent that performs specific AWS operations
    ///
    /// Capabilities:
    /// - Execute JavaScript code with AWS APIs
    /// - Report results back to parent
    /// - Ask user for clarifications
    ///
    /// Always has a parent TaskManager agent
    TaskWorker {
        /// ID of the parent agent
        parent_id: AgentId,
    },

    /// Tool builder worker agent spawned by TaskManager
    ///
    /// Capabilities:
    /// - Build HTML/CSS/JS applications for a specific task
    /// - Access file operations in dedicated workspace
    /// - Execute JavaScript to explore AWS data
    /// - Isolated context (doesn't pollute parent)
    ///
    /// Always has a parent TaskManager agent
    PageBuilderWorker {
        /// ID of the parent agent
        parent_id: AgentId,
        /// Workspace name (sanitized, kebab-case)
        workspace_name: String,
        /// Whether this is a persistent page (saved to disk) or temporary (VFS-backed)
        /// - false: Results display page (focus on VFS data, temporary)
        /// - true: Reusable tool page (queries AWS live, persistent)
        is_persistent: bool,
    },
}

impl AgentType {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::TaskManager => "General Purpose Agent",
            AgentType::TaskWorker { .. } => "Worker Agent",
            AgentType::PageBuilderWorker { .. } => "Page Builder Worker",
        }
    }

    /// Get description for UI
    pub fn description(&self) -> &'static str {
        match self {
            AgentType::TaskManager => "Multi-purpose agent with access to all tools",
            AgentType::TaskWorker { .. } => "Specialized worker for focused tasks",
            AgentType::PageBuilderWorker { .. } => "Specialized worker for building Dash Pages in isolated context",
        }
    }

    /// Check if this is a parent agent (can be created from UI)
    pub fn is_parent_agent(&self) -> bool {
        matches!(self, AgentType::TaskManager)
    }

    /// Check if this agent type can be created from UI
    pub fn is_user_creatable(&self) -> bool {
        self.is_parent_agent()
    }

    /// Check if this is a task-manager agent
    pub fn is_task_manager(&self) -> bool {
        matches!(self, AgentType::TaskManager)
    }

    /// Get the parent agent ID if this is a worker agent
    ///
    /// Returns `None` for parent agents (TaskManager)
    pub fn parent_id(&self) -> Option<AgentId> {
        match self {
            AgentType::TaskManager => None,
            AgentType::TaskWorker { parent_id } => Some(*parent_id),
            AgentType::PageBuilderWorker { parent_id, .. } => Some(*parent_id),
        }
    }

    /// Get the workspace name if this is a page-builder-worker
    ///
    /// Returns `None` for other agent types
    pub fn workspace_name(&self) -> Option<&str> {
        match self {
            AgentType::PageBuilderWorker { workspace_name, .. } => Some(workspace_name),
            _ => None,
        }
    }

    /// Get whether this is a persistent page (if page-builder-worker)
    ///
    /// Returns `None` for other agent types
    pub fn is_persistent(&self) -> Option<bool> {
        match self {
            AgentType::PageBuilderWorker { is_persistent, .. } => Some(*is_persistent),
            _ => None,
        }
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentType::TaskManager => write!(f, "Task Manager"),
            AgentType::TaskWorker { .. } => write!(f, "Task Worker"),
            AgentType::PageBuilderWorker { .. } => write!(f, "Page Builder Worker"),
        }
    }
}

/// Logging level for Stood library traces
///
/// Controls the verbosity of stood library debug output captured in agent logs.
/// Higher levels include all lower level messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoodLogLevel {
    /// No stood traces captured
    Off,
    /// Info level - high-level agent events only
    Info,
    /// Debug level - detailed agent operations (default)
    #[default]
    Debug,
    /// Trace level - all internal operations (verbose)
    Trace,
}

impl StoodLogLevel {
    /// Get display name for UI dropdown
    pub fn display_name(&self) -> &'static str {
        match self {
            StoodLogLevel::Off => "Off",
            StoodLogLevel::Info => "Info",
            StoodLogLevel::Debug => "Debug",
            StoodLogLevel::Trace => "Trace",
        }
    }

    /// Get all levels for dropdown iteration
    pub fn all() -> &'static [StoodLogLevel] {
        &[
            StoodLogLevel::Off,
            StoodLogLevel::Info,
            StoodLogLevel::Debug,
            StoodLogLevel::Trace,
        ]
    }

    /// Convert to tracing filter string for stood library
    pub fn to_filter_str(&self) -> &'static str {
        match self {
            StoodLogLevel::Off => "off",
            StoodLogLevel::Info => "info",
            StoodLogLevel::Debug => "debug",
            StoodLogLevel::Trace => "trace",
        }
    }

    /// Check if a given tracing level should be logged at this setting
    pub fn should_log(&self, level: tracing::Level) -> bool {
        match self {
            StoodLogLevel::Off => false,
            StoodLogLevel::Info => level <= tracing::Level::INFO,
            StoodLogLevel::Debug => level <= tracing::Level::DEBUG,
            StoodLogLevel::Trace => true,
        }
    }
}

impl fmt::Display for StoodLogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Agent metadata and configuration
#[derive(Debug, Clone)]
pub struct AgentMetadata {
    /// Human-readable agent name
    pub name: String,
    /// Agent description or purpose
    pub description: String,
    /// Selected model for this agent
    pub model: AgentModel,
    /// When the agent was created
    pub created_at: DateTime<Utc>,
    /// Last time the agent was updated
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_task_manager() {
        let agent_type = AgentType::TaskManager;
        assert!(matches!(agent_type, AgentType::TaskManager));
    }

    #[test]
    fn test_agent_type_task_worker() {
        let parent_id = AgentId::new();
        let agent_type = AgentType::TaskWorker { parent_id };

        match agent_type {
            AgentType::TaskWorker { parent_id: id } => {
                assert_eq!(id, parent_id);
            }
            _ => panic!("Expected TaskWorker variant"),
        }
    }

    #[test]
    fn test_agent_type_parent_id() {
        let parent_id = AgentId::new();
        let task_worker = AgentType::TaskWorker { parent_id };
        let task_manager = AgentType::TaskManager;

        assert_eq!(task_worker.parent_id(), Some(parent_id));
        assert_eq!(task_manager.parent_id(), None);
    }

    #[test]
    fn test_agent_type_is_task_manager() {
        let task_manager = AgentType::TaskManager;
        let task_worker = AgentType::TaskWorker {
            parent_id: AgentId::new(),
        };

        assert!(task_manager.is_task_manager());
        assert!(!task_worker.is_task_manager());
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::TaskManager.to_string(), "Task Manager");

        let worker = AgentType::TaskWorker {
            parent_id: AgentId::new(),
        };
        assert_eq!(worker.to_string(), "Task Worker");
    }

    #[test]
    fn test_agent_type_equality() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();

        let tm1 = AgentType::TaskManager;
        let tm2 = AgentType::TaskManager;
        let tw1 = AgentType::TaskWorker { parent_id: id1 };
        let tw2 = AgentType::TaskWorker { parent_id: id1 };
        let tw3 = AgentType::TaskWorker { parent_id: id2 };

        assert_eq!(tm1, tm2);
        assert_eq!(tw1, tw2); // Same parent_id
        assert_ne!(tw1, tw3); // Different parent_id
        assert_ne!(tm1, tw1); // Different variants
    }

    #[test]
    fn test_agent_type_clone() {
        let agent_type = AgentType::TaskManager;
        let cloned = agent_type.clone();
        assert_eq!(agent_type, cloned);
    }

    #[test]
    fn test_agent_type_page_builder_worker() {
        let parent_id = AgentId::new();
        let agent_type = AgentType::PageBuilderWorker {
            parent_id,
            workspace_name: "my-tool".to_string(),
            is_persistent: false,
        };

        match agent_type {
            AgentType::PageBuilderWorker { parent_id: id, workspace_name, is_persistent } => {
                assert_eq!(id, parent_id);
                assert_eq!(workspace_name, "my-tool");
                assert!(!is_persistent);
            }
            _ => panic!("Expected PageBuilderWorker variant"),
        }
    }

    #[test]
    fn test_agent_type_page_builder_worker_parent_id() {
        let parent_id = AgentId::new();
        let page_builder_worker = AgentType::PageBuilderWorker {
            parent_id,
            workspace_name: "test-workspace".to_string(),
            is_persistent: false,
        };

        assert_eq!(page_builder_worker.parent_id(), Some(parent_id));
        assert!(!page_builder_worker.is_parent_agent());
    }

    #[test]
    fn test_agent_type_page_builder_worker_workspace_name() {
        let parent_id = AgentId::new();
        let page_builder_worker = AgentType::PageBuilderWorker {
            parent_id,
            workspace_name: "s3-bucket-explorer".to_string(),
            is_persistent: true,
        };

        assert_eq!(page_builder_worker.workspace_name(), Some("s3-bucket-explorer"));
        assert_eq!(page_builder_worker.is_persistent(), Some(true));

        // Other agent types should return None
        assert_eq!(AgentType::TaskManager.workspace_name(), None);
        assert_eq!(AgentType::TaskManager.is_persistent(), None);
        assert_eq!(
            AgentType::TaskWorker { parent_id }.workspace_name(),
            None
        );
    }

    #[test]
    fn test_agent_type_page_builder_worker_display() {
        let parent_id = AgentId::new();
        let worker = AgentType::PageBuilderWorker {
            parent_id,
            workspace_name: "vpc-viewer".to_string(),
            is_persistent: false,
        };
        assert_eq!(worker.to_string(), "Page Builder Worker");
        assert_eq!(worker.display_name(), "Page Builder Worker");
        assert_eq!(
            worker.description(),
            "Specialized worker for building Dash Pages in isolated context"
        );
    }

    #[test]
    fn test_stood_log_level_default() {
        let level = StoodLogLevel::default();
        assert_eq!(level, StoodLogLevel::Debug);
    }

    #[test]
    fn test_stood_log_level_display_names() {
        assert_eq!(StoodLogLevel::Off.display_name(), "Off");
        assert_eq!(StoodLogLevel::Info.display_name(), "Info");
        assert_eq!(StoodLogLevel::Debug.display_name(), "Debug");
        assert_eq!(StoodLogLevel::Trace.display_name(), "Trace");
    }

    #[test]
    fn test_stood_log_level_all() {
        let all = StoodLogLevel::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], StoodLogLevel::Off);
        assert_eq!(all[1], StoodLogLevel::Info);
        assert_eq!(all[2], StoodLogLevel::Debug);
        assert_eq!(all[3], StoodLogLevel::Trace);
    }

    #[test]
    fn test_stood_log_level_filter_str() {
        assert_eq!(StoodLogLevel::Off.to_filter_str(), "off");
        assert_eq!(StoodLogLevel::Info.to_filter_str(), "info");
        assert_eq!(StoodLogLevel::Debug.to_filter_str(), "debug");
        assert_eq!(StoodLogLevel::Trace.to_filter_str(), "trace");
    }

    #[test]
    fn test_stood_log_level_should_log() {
        // Off should log nothing
        assert!(!StoodLogLevel::Off.should_log(tracing::Level::ERROR));
        assert!(!StoodLogLevel::Off.should_log(tracing::Level::INFO));

        // Info should log INFO and above (ERROR, WARN, INFO)
        assert!(StoodLogLevel::Info.should_log(tracing::Level::ERROR));
        assert!(StoodLogLevel::Info.should_log(tracing::Level::INFO));
        assert!(!StoodLogLevel::Info.should_log(tracing::Level::DEBUG));

        // Debug should log DEBUG and above
        assert!(StoodLogLevel::Debug.should_log(tracing::Level::INFO));
        assert!(StoodLogLevel::Debug.should_log(tracing::Level::DEBUG));
        assert!(!StoodLogLevel::Debug.should_log(tracing::Level::TRACE));

        // Trace should log everything
        assert!(StoodLogLevel::Trace.should_log(tracing::Level::TRACE));
    }

    #[test]
    fn test_stood_log_level_display() {
        assert_eq!(format!("{}", StoodLogLevel::Debug), "Debug");
        assert_eq!(format!("{}", StoodLogLevel::Trace), "Trace");
    }

    #[test]
    fn test_stood_log_level_copy() {
        let level = StoodLogLevel::Debug;
        let copied = level; // Should compile (Copy trait)
        assert_eq!(level, copied);
    }
}
