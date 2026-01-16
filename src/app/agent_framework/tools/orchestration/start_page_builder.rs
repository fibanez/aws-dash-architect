#![warn(clippy::all, rust_2018_idioms)]

//! Start-Page-Builder Tool - Spawning Page Builder Workers
//!
//! This tool allows task-manager agents to spawn page builder workers
//! for creating interactive Dash Pages (HTML/CSS/JS applications).
//!
//! ## Implementation
//!
//! Uses the agent creation request/response channel to spawn PageBuilderWorker agents.
//! The parent agent ID is retrieved from thread-local storage.
//! Workspace names are sanitized and collision detection ensures unique directories.

use crate::app::agent_framework::{
    get_current_agent_id, get_current_vfs_id, request_page_builder_creation, wait_for_worker_completion,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use stood::tools::{Tool, ToolError, ToolResult};

/// Start-tool-builder tool for spawning page builder workers
#[derive(Clone, Debug)]
pub struct StartPageBuilderTool;

/// Input schema for start-page-builder tool
#[derive(Debug, Deserialize, Serialize)]
struct StartPageBuilderInput {
    /// Suggested workspace name (will be sanitized)
    /// Example: "Lambda Function Dashboard" → "lambda-function-dashboard"
    workspace_name: String,

    /// Concise description (4-5 words) for inline progress display
    /// Example: "Building Lambda dashboard" or "Creating S3 explorer"
    concise_description: String,

    /// High-level description of the page to build
    task_description: String,

    /// Optional description of what data/resources the page should show
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_context: Option<String>,

    /// Whether to save the page permanently to disk (default: false)
    /// - false: Temporary VFS page (uses session data, disappears after session)
    /// - true: Persistent disk page (saved to Pages Manager, standalone)
    #[serde(default)]
    persistent: bool,
}

impl StartPageBuilderTool {
    /// Create a new start-page-builder tool instance
    pub fn new() -> Self {
        Self
    }

    /// Get the page name
    pub fn name(&self) -> &str {
        "start_page_builder"
    }

    /// Get the page description
    pub fn description(&self) -> &str {
        "Spawn a page builder worker to create an interactive Dash Page (HTML/CSS/JS app).\n\n\
         **Page Types (controlled by `persistent` parameter):**\n\n\
         `persistent: false` (default) - **Temporary VFS page**\n\
         - Uses session VFS data (can read files from /workspace/, /results/)\n\
         - Disappears when session ends\n\
         - Use for: 'Show me results', 'Display these findings', 'Visualize this data'\n\
         - Pass `data_file` in resource_context to tell builder which VFS file to read\n\n\
         `persistent: true` - **Permanent disk page**\n\
         - Saved to Pages Manager, appears in page list\n\
         - Standalone - doesn't depend on session data\n\
         - Use for: 'Create a dashboard', 'Build a tool', 'Make a reusable viewer'\n\n\
         **Examples:**\n\n\
         User: 'Show me the security groups with port 22 open'\n\
         → persistent: false, resource_context: 'Data file: /workspace/ssh-audit/findings.json'\n\n\
         User: 'Create a Lambda dashboard I can use later'\n\
         → persistent: true, builds standalone page with live data queries\n\n\
         User: 'Build a VPC explorer tool'\n\
         → persistent: true, reusable tool saved to disk\n\n\
         **IMPORTANT**: On success, returns `workspace_name` which you MUST pass EXACTLY to `open_page`.\n\
         For VFS pages, workspace_name includes 'vfs:' prefix - use the full string."
    }

    /// Get the parameters schema
    pub fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["workspace_name", "concise_description", "task_description"],
            "properties": {
                "workspace_name": {
                    "type": "string",
                    "description": "Suggested workspace name (will be sanitized to kebab-case). Choose a descriptive name based on what the page does.",
                    "examples": [
                        "Lambda Function Dashboard",
                        "S3 Bucket Explorer",
                        "ssh-findings-view"
                    ]
                },
                "concise_description": {
                    "type": "string",
                    "description": "Concise progress description in 4-5 words. Use present continuous tense (-ing). This appears in inline worker display.",
                    "examples": [
                        "Building Lambda dashboard",
                        "Creating S3 explorer",
                        "Displaying SSH findings"
                    ]
                },
                "task_description": {
                    "type": "string",
                    "description": "High-level description of WHAT to build and WHY. Include user's original request for context.",
                    "examples": [
                        "User requested: 'Create a dashboard for my Lambda functions'. Build an interactive dashboard showing Lambda functions with filters for runtime and memory.",
                        "Display the SSH audit findings from /workspace/ssh-audit/findings.json in a table with sorting and filtering."
                    ]
                },
                "resource_context": {
                    "type": "string",
                    "description": "Context about data source. For temporary pages, specify the VFS data file path.",
                    "examples": [
                        "Data file: /workspace/ssh-audit/findings.json (47 security groups with public SSH)",
                        "Show Lambda functions from all accounts/regions with runtime, memory, timeout"
                    ]
                },
                "persistent": {
                    "type": "boolean",
                    "default": false,
                    "description": "Whether to save page permanently. false=temporary VFS page (session data), true=permanent disk page (standalone).",
                    "examples": [
                        false,
                        true
                    ]
                }
            }
        })
    }
}

impl Default for StartPageBuilderTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for StartPageBuilderTool {
    fn name(&self) -> &str {
        StartPageBuilderTool::name(self)
    }

    fn description(&self) -> &str {
        StartPageBuilderTool::description(self)
    }

    fn parameters_schema(&self) -> Value {
        StartPageBuilderTool::parameters_schema(self)
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        stood::perf_checkpoint!("awsdash.start_page_builder.execute.start");
        let _tool_guard = stood::perf_guard!("awsdash.start_page_builder.execute");

        // Parse input
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "start_page_builder requires parameters".to_string(),
        })?;

        let input: StartPageBuilderInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParameters {
                message: format!("Failed to parse start_page_builder input: {}", e),
            })?;

        // Validate workspace_name not empty
        if input.workspace_name.trim().is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "workspace_name cannot be empty".to_string(),
            });
        }

        // Validate task_description not empty
        if input.task_description.trim().is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "task_description cannot be empty".to_string(),
            });
        }

        // Get parent agent ID from thread-local
        let parent_id = get_current_agent_id().ok_or_else(|| ToolError::InvalidParameters {
            message: "Cannot determine parent agent ID - agent context not set".to_string(),
        })?;

        tracing::info!(
            target: "agent::start_page_builder",
            parent_id = %parent_id,
            "start_page_builder TOOL CALL:\n  Workspace Name: {}\n  Task Description: {}\n  Resource Context: {:?}\n  Persistent: {}",
            input.workspace_name,
            input.task_description,
            input.resource_context,
            input.persistent
        );

        // Decide VFS usage based on persistent flag:
        // - persistent=false (default): Use VFS for temporary session-based page
        // - persistent=true: Don't use VFS, save to disk for permanent page
        let vfs_id = if input.persistent {
            tracing::info!(
                target: "agent::start_page_builder",
                "Creating PERSISTENT page (saved to disk, no VFS)"
            );
            None
        } else {
            let id = get_current_vfs_id();
            if id.is_some() {
                tracing::info!(
                    target: "agent::start_page_builder",
                    "Creating TEMPORARY page (VFS-backed, session data)"
                );
            } else {
                tracing::warn!(
                    target: "agent::start_page_builder",
                    "No VFS available - temporary page will use disk storage"
                );
            }
            id
        };

        // Request page builder creation via channel with reuse_existing=false
        // This enables collision detection to create a unique folder name
        stood::perf_checkpoint!(
            "awsdash.start_page_builder.request_creation.start",
            &format!("parent_id={}, workspace={}", parent_id, &input.workspace_name)
        );
        let (agent_id, sanitized_workspace) = stood::perf_timed!(
            "awsdash.start_page_builder.request_page_builder_creation",
            {
                request_page_builder_creation(
                    input.workspace_name.clone(),
                    input.concise_description.clone(),
                    input.task_description.clone(),
                    input.resource_context.clone(),
                    parent_id,
                    false,  // reuse_existing: false means use collision detection for new pages
                    vfs_id,
                    input.persistent,  // Pass through the persistent flag
                )
            }
        )
        .map_err(|e| ToolError::InvalidParameters {
            message: format!("Failed to create tool-builder worker: {}", e),
        })?;
        stood::perf_checkpoint!(
            "awsdash.start_page_builder.request_creation.end",
            &format!("worker_id={}, workspace={}", agent_id, sanitized_workspace)
        );

        tracing::info!(
            target: "agent::start_page_builder",
            parent_id = %parent_id,
            agent_id = %agent_id,
            workspace_name = %sanitized_workspace,
            "PageBuilderWorker created, waiting for completion"
        );

        // Wait for worker to complete (10 minute timeout - tool building takes longer)
        let start_time = std::time::Instant::now();
        tracing::info!(
            target: "agent::start_page_builder",
            parent_id = %parent_id,
            agent_id = %agent_id,
            "Waiting for page builder to complete (status: Building Tool)"
        );

        stood::perf_checkpoint!(
            "awsdash.start_page_builder.wait_completion.start",
            &format!("worker_id={}", agent_id)
        );
        match stood::perf_timed!("awsdash.start_page_builder.wait_for_worker_completion", {
            wait_for_worker_completion(agent_id, Duration::from_secs(600))
        }) {
            Ok(result) => {
                let execution_time_ms = start_time.elapsed().as_millis();
                stood::perf_checkpoint!(
                    "awsdash.start_page_builder.wait_completion.success",
                    &format!(
                        "worker_id={}, workspace={}, execution_time_ms={}",
                        agent_id, sanitized_workspace, execution_time_ms
                    )
                );

                tracing::info!(
                    target: "agent::start_page_builder",
                    parent_id = %parent_id,
                    agent_id = %agent_id,
                    workspace_name = %sanitized_workspace,
                    execution_time_ms = execution_time_ms,
                    "PageBuilderWorker completed successfully"
                );

                // Return workspace name and result
                // IMPORTANT: workspace_name must be passed exactly to open_page
                Ok(ToolResult::success(json!({
                    "workspace_name": sanitized_workspace,
                    "result": result,
                    "execution_time_ms": execution_time_ms,
                    "next_step": format!("To open this page, call: open_page({{\"page_name\": \"{}\"}})", sanitized_workspace),
                })))
            }
            Err(error) => {
                stood::perf_checkpoint!(
                    "awsdash.start_page_builder.wait_completion.error",
                    &format!("worker_id={}, error={}", agent_id, error)
                );
                tracing::error!(
                    target: "agent::start_page_builder",
                    parent_id = %parent_id,
                    agent_id = %agent_id,
                    error = %error,
                    "PageBuilderWorker failed"
                );

                Ok(ToolResult::error(&error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_page_builder_tool_creation() {
        let tool = StartPageBuilderTool::new();
        assert_eq!(tool.name(), "start_page_builder");
        assert!(tool.description().contains("Spawn"));
        assert!(tool.description().contains("dashboard"));
    }

    #[test]
    fn test_start_page_builder_tool_schema() {
        let tool = StartPageBuilderTool::new();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("workspace_name")));
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("task_description")));

        // Verify workspace_name property exists
        assert!(schema["properties"]["workspace_name"].is_object());
        assert!(schema["properties"]["workspace_name"]["description"]
            .as_str()
            .unwrap()
            .contains("sanitized"));

        // Verify task_description property exists
        assert!(schema["properties"]["task_description"].is_object());

        // Verify resource_context is optional
        assert!(!schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("resource_context")));
    }

    #[test]
    fn test_start_page_builder_tool_clone() {
        let tool1 = StartPageBuilderTool::new();
        let tool2 = tool1.clone();
        assert_eq!(tool1.name(), tool2.name());
    }
}
