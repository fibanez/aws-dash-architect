//! AWS Resource Tools for AI Agent
//!
//! This module provides a registry and common utilities for AWS bridge tools.
//! Individual tool implementations are in the tools/ subfolder.

use crate::app::dashui::control_bridge_window::AgentResponse;
use crate::app::resource_explorer::aws_client::AWSResourceClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex, RwLock};
use stood::tools::Tool;
use tracing::{error, info, warn};

// Import TODO storage types
use super::tools::todo_write::TodoItem;

// Import individual tools for registry functions
use super::tools::*;

/// Global tool context for accessing AWS client at runtime
static GLOBAL_AWS_CLIENT: RwLock<Option<Arc<AWSResourceClient>>> = RwLock::new(None);

/// Global AWS credentials for standalone agents
static GLOBAL_AWS_CREDENTIALS: RwLock<Option<(String, String, Option<String>, String)>> = RwLock::new(None);

/// Global Bridge response channel for log analysis event bubbling
static GLOBAL_BRIDGE_SENDER: RwLock<Option<mpsc::Sender<AgentResponse>>> = RwLock::new(None);

/// Global shared TODO storage for all agents
static GLOBAL_TODO_STORAGE: RwLock<Option<Arc<Mutex<HashMap<String, Vec<TodoItem>>>>>> = RwLock::new(None);

/// Set the global AWS client for all tools to use
pub fn set_global_aws_client(client: Option<Arc<AWSResourceClient>>) {
    match GLOBAL_AWS_CLIENT.write() {
        Ok(mut guard) => {
            let client_status = if client.is_some() { "✅ Set" } else { "❌ Cleared" };
            info!("🔧 Global AWS client updated for bridge tools: {}", client_status);
            *guard = client;
        }
        Err(e) => {
            warn!("Failed to update global AWS client: {}", e);
        }
    }
}

/// Get the global AWS client for tool execution
pub fn get_global_aws_client() -> Option<Arc<AWSResourceClient>> {
    match GLOBAL_AWS_CLIENT.read() {
        Ok(guard) => {
            let client = guard.clone();
            info!("🔍 Global AWS client access: {}", if client.is_some() { "✅ Available" } else { "❌ Not set" });
            client
        }
        Err(e) => {
            warn!("Failed to read global AWS client: {}", e);
            None
        }
    }
}

/// Simplified resource summary for tool responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceSummary {
    /// AWS resource type
    pub resource_type: String,
    /// AWS account ID
    pub account_id: String,
    /// AWS region
    pub region: String,
    /// Resource identifier
    pub resource_id: String,
    /// Human-readable display name
    pub display_name: String,
    /// Resource status if available
    pub status: Option<String>,
    /// Key properties as JSON
    pub properties: serde_json::Value,
    /// Resource tags
    pub tags: Vec<String>,
}


/// Individual tool constructors for explicit tool selection
/// Creates AWS List Resources tool
pub fn aws_list_resources_tool(aws_client: Option<Arc<AWSResourceClient>>) -> Box<dyn Tool> {
    Box::new(AwsListResourcesTool::new(aws_client))
}

/// Creates AWS Describe Resource tool
pub fn aws_describe_resource_tool(aws_client: Option<Arc<AWSResourceClient>>) -> Box<dyn Tool> {
    Box::new(AwsDescribeResourceTool::new(aws_client))
}

/// Creates AWS Find Account tool
pub fn aws_find_account_tool() -> Box<dyn Tool> {
    Box::new(AwsFindAccountTool::new_uninitialized())
}

/// Creates AWS Find Region tool
pub fn aws_find_region_tool() -> Box<dyn Tool> {
    Box::new(AwsFindRegionTool::new_uninitialized())
}

/// Creates AWS Describe Log Groups tool
pub fn aws_describe_log_groups_tool(aws_client: Option<Arc<AWSResourceClient>>) -> Box<dyn Tool> {
    Box::new(AwsDescribeLogGroupsTool::new(aws_client))
}

/// Creates AWS Get Log Events tool
pub fn aws_get_log_events_tool(aws_client: Option<Arc<AWSResourceClient>>) -> Box<dyn Tool> {
    Box::new(AwsGetLogEventsTool::new(aws_client))
}

/// Creates AWS Get Log Entries tool (high-level tool with standalone agent)
pub fn aws_get_log_entries_tool(aws_client: Option<Arc<AWSResourceClient>>) -> Box<dyn Tool> {
    Box::new(AwsGetLogEntriesTool::new(aws_client))
}

/// Creates TodoWrite tool for task management with shared storage
pub fn todo_write_tool() -> Box<dyn Tool> {
    if let Some(storage) = get_global_todo_storage() {
        Box::new(TodoWriteTool::with_shared_storage(storage))
    } else {
        warn!("❌ Failed to get global TODO storage, creating isolated TodoWrite tool");
        Box::new(TodoWriteTool::new())
    }
}

/// Creates TodoRead tool for task querying with shared storage
pub fn todo_read_tool() -> Box<dyn Tool> {
    if let Some(storage) = get_global_todo_storage() {
        Box::new(TodoReadTool::with_shared_storage(storage))
    } else {
        warn!("❌ Failed to get global TODO storage, creating isolated TodoRead tool");
        Box::new(TodoReadTool::new())
    }
}

/// Creates Create_Task tool for flexible task-based agent orchestration
pub fn create_task_tool() -> Box<dyn Tool> {
    Box::new(CreateTaskTool::new())
}

/// Set global AWS credentials for standalone agents
pub fn set_global_aws_credentials(
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
    region: String,
) {
    match GLOBAL_AWS_CREDENTIALS.write() {
        Ok(mut guard) => {
            info!("🔐 Global AWS credentials updated for standalone agents");
            *guard = Some((access_key, secret_key, session_token, region));
        }
        Err(e) => {
            error!("❌ Failed to set global AWS credentials: {}", e);
        }
    }
}

/// Get global AWS credentials for standalone agents
pub fn get_global_aws_credentials() -> Option<(String, String, Option<String>, String)> {
    match GLOBAL_AWS_CREDENTIALS.read() {
        Ok(guard) => {
            let has_creds = guard.is_some();
            info!("🔍 Global AWS credentials access: {}", if has_creds { "✅ Available" } else { "❌ Not set" });
            guard.clone()
        }
        Err(e) => {
            error!("❌ Failed to read global AWS credentials: {}", e);
            None
        }
    }
}

/// Clear global AWS credentials
pub fn clear_global_aws_credentials() {
    match GLOBAL_AWS_CREDENTIALS.write() {
        Ok(mut guard) => {
            info!("🔐 Global AWS credentials cleared");
            *guard = None;
        }
        Err(e) => {
            error!("❌ Failed to clear global AWS credentials: {}", e);
        }
    }
}

/// Set global Bridge response channel for log analysis event bubbling
pub fn set_global_bridge_sender(sender: mpsc::Sender<AgentResponse>) {
    match GLOBAL_BRIDGE_SENDER.write() {
        Ok(mut guard) => {
            info!("📡 Global Bridge response channel set for log analysis event bubbling");
            *guard = Some(sender);
        }
        Err(e) => {
            error!("❌ Failed to set global Bridge response channel: {}", e);
        }
    }
}

/// Get global Bridge response channel for log analysis event bubbling
pub fn get_global_bridge_sender() -> Option<mpsc::Sender<AgentResponse>> {
    match GLOBAL_BRIDGE_SENDER.read() {
        Ok(guard) => {
            let has_sender = guard.is_some();
            info!("📡 Global Bridge response channel access: {}", if has_sender { "✅ Available" } else { "❌ Not set" });
            guard.clone()
        }
        Err(e) => {
            error!("❌ Failed to read global Bridge response channel: {}", e);
            None
        }
    }
}

/// Clear global Bridge response channel
pub fn clear_global_bridge_sender() {
    match GLOBAL_BRIDGE_SENDER.write() {
        Ok(mut guard) => {
            info!("📡 Global Bridge response channel cleared");
            *guard = None;
        }
        Err(e) => {
            error!("❌ Failed to clear global Bridge response channel: {}", e);
        }
    }
}

/// Initialize global shared TODO storage (call once at startup)
pub fn initialize_global_todo_storage() {
    match GLOBAL_TODO_STORAGE.write() {
        Ok(mut guard) => {
            if guard.is_none() {
                info!("📝 Initializing global shared TODO storage for all agents");
                *guard = Some(Arc::new(Mutex::new(HashMap::new())));
            } else {
                info!("📝 Global TODO storage already initialized");
            }
        }
        Err(e) => {
            error!("❌ Failed to initialize global TODO storage: {}", e);
        }
    }
}

/// Get global shared TODO storage for tools
pub fn get_global_todo_storage() -> Option<Arc<Mutex<HashMap<String, Vec<TodoItem>>>>> {
    match GLOBAL_TODO_STORAGE.read() {
        Ok(guard) => {
            let storage = guard.clone();
            if storage.is_some() {
                info!("📝 Global TODO storage access: ✅ Available");
                storage
            } else {
                info!("📝 Global TODO storage access: ❌ Not initialized - initializing now");
                // Auto-initialize if not already done
                drop(guard); // Release read lock
                initialize_global_todo_storage();
                // Try again
                match GLOBAL_TODO_STORAGE.read() {
                    Ok(guard) => guard.clone(),
                    Err(e) => {
                        error!("❌ Failed to read global TODO storage after initialization: {}", e);
                        None
                    }
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to read global TODO storage: {}", e);
            None
        }
    }
}