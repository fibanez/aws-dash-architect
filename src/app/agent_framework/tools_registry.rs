//! AWS Resource Tools for AI Agent
//!
//! This module provides a registry and common utilities for AWS agent framework tools.
//! Individual tool implementations are in the tools/ subfolder.

use crate::app::agent_framework::message::AgentResponse;
use crate::app::resource_explorer::aws_client::AWSResourceClient;
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Arc, RwLock};
use stood::tools::Tool;
use tracing::{error, info, warn};

use super::cancellation::AgentCancellationManager;

// Import individual tools for registry functions
use super::tools::*;

/// Global tool context for accessing AWS client at runtime
static GLOBAL_AWS_CLIENT: RwLock<Option<Arc<AWSResourceClient>>> = RwLock::new(None);

/// Global AWS credentials for standalone agents
static GLOBAL_AWS_CREDENTIALS: RwLock<Option<(String, String, Option<String>, String)>> =
    RwLock::new(None);

/// Global Agent Framework response channel for log analysis event bubbling
static GLOBAL_AGENT_SENDER: RwLock<Option<mpsc::Sender<AgentResponse>>> = RwLock::new(None);

/// Global agent cancellation manager for stopping running agents
static GLOBAL_CANCELLATION_MANAGER: RwLock<Option<Arc<AgentCancellationManager>>> =
    RwLock::new(None);

/// Global model configuration for agent creation
static GLOBAL_MODEL_CONFIG: RwLock<Option<String>> = RwLock::new(None);

/// Set the global AWS client for all tools to use
pub fn set_global_aws_client(client: Option<Arc<AWSResourceClient>>) {
    match GLOBAL_AWS_CLIENT.write() {
        Ok(mut guard) => {
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
            info!(
                "🔍 Global AWS client access: {}",
                if client.is_some() {
                    "✅ Available"
                } else {
                    "❌ Not set"
                }
            );
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

/// Creates AWS CloudTrail Lookup Events tool
pub fn aws_cloudtrail_lookup_events_tool(aws_client: Option<Arc<AWSResourceClient>>) -> Box<dyn Tool> {
    Box::new(AwsCloudTrailLookupEventsTool::new(aws_client))
}

/// Creates Read File tool for skill system
pub fn read_file_tool() -> Box<dyn Tool> {
    Box::new(ReadFileTool::new())
}

/// Creates List Directory tool for skill discovery
pub fn list_directory_tool() -> Box<dyn Tool> {
    Box::new(ListDirectoryTool::new())
}

/// Creates Invoke Skill tool for loading skills on-demand
pub fn invoke_skill_tool() -> Box<dyn Tool> {
    Box::new(InvokeSkillTool::new())
}

/// Creates Execute JavaScript tool for code execution
pub fn execute_javascript_tool() -> Box<dyn Tool> {
    Box::new(ExecuteJavaScriptTool::new())
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
            info!(
                "🔍 Global AWS credentials access: {}",
                if has_creds {
                    "✅ Available"
                } else {
                    "❌ Not set"
                }
            );
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

/// Set global Agent response channel for log analysis event bubbling
pub fn set_global_agent_sender(sender: mpsc::Sender<AgentResponse>) {
    match GLOBAL_AGENT_SENDER.write() {
        Ok(mut guard) => {
            info!("📡 Global Agent Framework response channel set for log analysis event bubbling");
            *guard = Some(sender);
        }
        Err(e) => {
            error!("❌ Failed to set global Agent response channel: {}", e);
        }
    }
}

/// Get global Agent response channel for log analysis event bubbling
pub fn get_global_agent_sender() -> Option<mpsc::Sender<AgentResponse>> {
    match GLOBAL_AGENT_SENDER.read() {
        Ok(guard) => {
            let has_sender = guard.is_some();
            info!(
                "📡 Global Agent Framework response channel access: {}",
                if has_sender {
                    "✅ Available"
                } else {
                    "❌ Not set"
                }
            );
            guard.clone()
        }
        Err(e) => {
            error!("❌ Failed to read global Agent response channel: {}", e);
            None
        }
    }
}

/// Clear global Agent response channel
pub fn clear_global_agent_sender() {
    match GLOBAL_AGENT_SENDER.write() {
        Ok(mut guard) => {
            info!("📡 Global Agent Framework response channel cleared");
            *guard = None;
        }
        Err(e) => {
            error!("❌ Failed to clear global Agent response channel: {}", e);
        }
    }
}

/// Set global agent cancellation manager for stopping running agents
pub fn set_global_cancellation_manager(manager: Arc<AgentCancellationManager>) {
    match GLOBAL_CANCELLATION_MANAGER.write() {
        Ok(mut guard) => {
            info!("🛑 Global agent cancellation manager updated");
            *guard = Some(manager);
        }
        Err(e) => {
            error!("❌ Failed to set global cancellation manager: {}", e);
        }
    }
}

/// Get global agent cancellation manager for stopping running agents
pub fn get_global_cancellation_manager() -> Option<Arc<AgentCancellationManager>> {
    match GLOBAL_CANCELLATION_MANAGER.read() {
        Ok(guard) => {
            let manager = guard.clone();
            // Remove excessive logging that floods the log in render loops
            manager
        }
        Err(e) => {
            error!("❌ Failed to read global cancellation manager: {}", e);
            None
        }
    }
}

/// Clear global agent cancellation manager
pub fn clear_global_cancellation_manager() {
    match GLOBAL_CANCELLATION_MANAGER.write() {
        Ok(mut guard) => {
            info!("🛑 Global cancellation manager cleared");
            *guard = None;
        }
        Err(e) => {
            error!("❌ Failed to clear global cancellation manager: {}", e);
        }
    }
}

/// Set global model configuration for agent creation
pub fn set_global_model(model_id: String) {
    match GLOBAL_MODEL_CONFIG.write() {
        Ok(mut guard) => {
            info!("🤖 Global model updated to: {}", model_id);
            *guard = Some(model_id);
        }
        Err(e) => {
            error!("❌ Failed to set global model: {}", e);
        }
    }
}

/// Get global model configuration for agent creation
pub fn get_global_model() -> Option<String> {
    match GLOBAL_MODEL_CONFIG.read() {
        Ok(guard) => {
            let model = guard.clone();
            info!(
                "🤖 Global model access: {}",
                if model.is_some() {
                    "✅ Available"
                } else {
                    "❌ Not set"
                }
            );
            model
        }
        Err(e) => {
            error!("❌ Failed to read global model: {}", e);
            None
        }
    }
}

/// Clear global model configuration
pub fn clear_global_model() {
    match GLOBAL_MODEL_CONFIG.write() {
        Ok(mut guard) => {
            info!("🤖 Global model cleared");
            *guard = None;
        }
        Err(e) => {
            error!("❌ Failed to clear global model: {}", e);
        }
    }
}

/// Set the global current project for all tools to use (stubbed - project management removed)
pub fn set_global_current_project(_project: Option<()>) {
    // Project management removed from Agent Framework
}

/// Get the global current project for tool execution (stubbed - project management removed)
pub fn get_global_current_project() -> Option<()> {
    None
}
