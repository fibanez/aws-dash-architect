//! AWS Bridge Task Agents
//!
//! This module contains agent implementations for the Bridge system:
//! - BridgeAgent: Main orchestration agent for AWS infrastructure management
//! - TaskAgent: Specialized task agents for specific AWS operations

pub mod bridge_agent;
pub mod task_agent;

// Re-export agents for easy access
pub use bridge_agent::BridgeAgent;
pub use task_agent::TaskAgent;
