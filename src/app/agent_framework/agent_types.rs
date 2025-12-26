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
/// Agents can be either task managers (orchestrators) or task workers (executors).
/// Task workers always have a reference to their parent task manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    /// Task management agent that orchestrates multiple task workers
    ///
    /// Capabilities:
    /// - Break down complex goals into tasks
    /// - Spawn task-worker agents
    /// - Track task progress
    /// - Aggregate results
    TaskManager,

    /// Task execution agent that performs specific AWS operations
    ///
    /// Capabilities:
    /// - Execute JavaScript code with AWS APIs
    /// - Report results back to parent
    /// - Ask user for clarifications
    ///
    /// Always has a parent task-manager agent
    TaskWorker {
        /// ID of the parent task-manager agent
        parent_id: AgentId,
    },
}

impl AgentType {
    /// Check if this is a task-manager agent
    pub fn is_task_manager(&self) -> bool {
        matches!(self, AgentType::TaskManager)
    }

    /// Get the parent agent ID if this is a task-worker
    ///
    /// Returns `None` for task-manager agents
    pub fn parent_id(&self) -> Option<AgentId> {
        match self {
            AgentType::TaskManager => None,
            AgentType::TaskWorker { parent_id } => Some(*parent_id),
        }
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentType::TaskManager => write!(f, "Task Manager"),
            AgentType::TaskWorker { .. } => write!(f, "Task Worker"),
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
    fn test_agent_type_copy() {
        let agent_type = AgentType::TaskManager;
        let copied = agent_type; // Should compile (Copy trait)
        assert_eq!(agent_type, copied);
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
