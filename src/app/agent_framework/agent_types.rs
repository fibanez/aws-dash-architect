//! Shared Agent Types
//!
//! Core types used by both V1 (legacy) and V2 agent systems.
//! These types provide common identification, status tracking, and metadata
//! for agents across different implementations.

#![warn(clippy::all, rust_2018_idioms)]

use chrono::{DateTime, Utc};
use std::fmt;
use uuid::Uuid;

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

/// Agent metadata and configuration
#[derive(Debug, Clone)]
pub struct AgentMetadata {
    /// Human-readable agent name
    pub name: String,
    /// Agent description or purpose
    pub description: String,
    /// Model ID being used (e.g., "claude-sonnet-4")
    pub model_id: String,
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
}
