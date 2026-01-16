//! Agent Framework Module - AI Agent Tools for AWS Infrastructure Management
//!
//! This module provides the Agent Framework, which enables AI agents to interact
//! with AWS resource operations through natural language requests.

// Core modules
pub mod core;
pub mod conversation;
pub mod logging;
pub mod vfs;
pub mod workers;

// Feature modules
pub mod middleware;
pub mod prompts;
pub mod skills;
pub mod tools;
pub mod ui;
pub mod utils;
pub mod v8_bindings;

// Re-export commonly used items from core
pub use core::*;

// Re-export conversation items
pub use conversation::*;

// Re-export logging items
pub use logging::*;

// Re-export worker items
pub use workers::*;

// Re-export middleware items
pub use middleware::{
    ConversationLayer, LayerContext, LayerError, LayerResult, LayerStack, PostResponseAction,
};

// Re-export prompts
pub use prompts::{
    TASK_MANAGER_PROMPT, TASK_WORKER_PROMPT, PAGE_BUILDER_PROMPT, PAGE_BUILDER_WORKER_PROMPT,
    PAGE_BUILDER_COMMON, PAGE_BUILDER_RESULTS_PROMPT, PAGE_BUILDER_TOOL_PROMPT,
};

// Re-export skills
pub use skills::*;

// Re-export tools
pub use tools::*;

// Re-export UI items
pub use ui::*;

// Re-export utils
pub use utils::*;

// Re-export V8 bindings (explicit exports to avoid ambiguous glob re-exports with core::types)
pub use v8_bindings::{
    get_api_documentation, register_bindings, set_global_aws_identity,
    register_console, ConsoleBuffers,
    initialize_v8_platform, is_v8_initialized,
    ExecutionResult, RuntimeConfig, V8Runtime,
    from_v8_value, to_v8_value,
};

// Global workspace tracking for TaskManager agents
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

static AGENT_WORKSPACE_MAP: OnceLock<Arc<Mutex<HashMap<String, Option<String>>>>> =
    OnceLock::new();

/// Get the current workspace being worked on by an agent
///
/// Returns `None` if the agent has no workspace set or if the agent is not tracked.
pub fn get_current_workspace_for_agent(agent_id: crate::app::agent_framework::core::types::AgentId) -> Option<String> {
    let agent_id_str = agent_id.to_string();
    AGENT_WORKSPACE_MAP
        .get()?
        .lock()
        .unwrap()
        .get(&agent_id_str)
        .cloned()
        .flatten()
}

/// Get the current workspace by agent ID string
///
/// This variant is used by middleware that receives agent_id as a string.
pub fn get_current_workspace_for_agent_str(agent_id: &str) -> Option<String> {
    AGENT_WORKSPACE_MAP
        .get()?
        .lock()
        .unwrap()
        .get(agent_id)
        .cloned()
        .flatten()
}

/// Set the current workspace for an agent
///
/// This locks the agent to a specific workspace for the session.
pub fn set_current_workspace_for_agent(agent_id: crate::app::agent_framework::core::types::AgentId, workspace: &str) {
    let agent_id_str = agent_id.to_string();
    AGENT_WORKSPACE_MAP
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .lock()
        .unwrap()
        .insert(agent_id_str, Some(workspace.to_string()));
}

/// Set the current workspace by agent ID string
///
/// This variant is used by middleware that receives agent_id as a string.
pub fn set_current_workspace_for_agent_str(agent_id: &str, workspace: &str) {
    AGENT_WORKSPACE_MAP
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .lock()
        .unwrap()
        .insert(agent_id.to_string(), Some(workspace.to_string()));
}

/// Clear workspace tracking for an agent
///
/// This should be called when an agent terminates.
pub fn clear_workspace_for_agent(agent_id: crate::app::agent_framework::core::types::AgentId) {
    let agent_id_str = agent_id.to_string();
    if let Some(map) = AGENT_WORKSPACE_MAP.get() {
        map.lock().unwrap().remove(&agent_id_str);
    }
}
