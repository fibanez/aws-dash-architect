//! AWS Bridge Task Agents
//!
//! This module contains the task-based agent implementation for the agent-on-demand
//! architecture. The TaskAgent handles any AWS task based on natural language descriptions.

pub mod task_agent;

// Re-export agent for easy access
pub use task_agent::TaskAgent;
