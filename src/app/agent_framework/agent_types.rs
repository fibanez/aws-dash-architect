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
