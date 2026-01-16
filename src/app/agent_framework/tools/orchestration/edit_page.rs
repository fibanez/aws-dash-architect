#![warn(clippy::all, rust_2018_idioms)]

//! Edit-Page Tool - Modify Existing Dash Pages
//!
//! This tool allows task-manager agents to spawn page builder workers
//! for editing existing Dash Pages (HTML/CSS/JS applications).
//!
//! ## Implementation
//!
//! Uses the agent creation request/response channel to spawn PageBuilderWorker agents
//! with an existing workspace name. The worker reads existing files and makes changes.

use crate::app::agent_framework::{
    get_current_agent_id, get_current_vfs_id, request_page_builder_creation, wait_for_worker_completion,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;
use stood::tools::{Tool, ToolError, ToolResult};

/// Edit-page tool for modifying existing Dash Pages
#[derive(Clone, Debug)]
pub struct EditPageTool;

/// Input schema for edit-page tool
#[derive(Debug, Deserialize, Serialize)]
struct EditPageInput {
    /// Name of the existing page to edit (folder name in pages directory)
    page_name: String,

    /// Concise description (4-5 words) for inline progress display
    /// Example: "Updating Lambda dashboard" or "Fixing S3 filters"
    concise_description: String,

    /// Description of what changes to make
    task_description: String,
}

/// Get the pages directory path
fn get_pages_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("awsdash/pages"))
}

impl EditPageTool {
    /// Create a new edit-page tool instance
    pub fn new() -> Self {
        Self
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        "edit_page"
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        "Edit an existing Dash Page by spawning a page builder worker.\n\n\
         Use this tool when:\n\
         1. User wants to modify an existing page they created earlier\n\
         2. You need to fix a bug or add features to an existing page\n\
         3. User references a page by name and wants changes\n\n\
         **Good examples**:\n\
         - 'Fix the filter on my Lambda dashboard'\n\
         - 'Add region selector to the S3 explorer'\n\
         - 'Update the VPC viewer to show more details'\n\n\
         **Bad examples**:\n\
         - 'Create a new dashboard' (use start_page_builder instead)\n\
         - 'List my pages' (pages are listed in the Pages Manager)\n\n\
         The worker will read existing files and apply the requested changes.\n\
         Make sure the page exists before calling this tool."
    }

    /// Get the parameters schema
    pub fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["page_name", "concise_description", "task_description"],
            "properties": {
                "page_name": {
                    "type": "string",
                    "description": "Name of the existing page to edit. This is the folder name in the pages directory.",
                    "examples": [
                        "lambda-function-dashboard",
                        "s3-bucket-explorer",
                        "vpc-subnet-viewer"
                    ]
                },
                "concise_description": {
                    "type": "string",
                    "description": "Concise progress description in 4-5 words. Use present continuous tense (-ing). This appears in inline worker display.",
                    "examples": [
                        "Updating Lambda dashboard",
                        "Fixing S3 filters",
                        "Adding VPC details"
                    ]
                },
                "task_description": {
                    "type": "string",
                    "description": "Description of what changes to make. Be specific about what to add, modify, or fix.",
                    "examples": [
                        "User requested: 'Add a search box to filter functions by name'. Add a text input that filters the function list as the user types.",
                        "User found a bug: 'The refresh button doesn't work'. Fix the refresh button to properly reload data."
                    ]
                }
            }
        })
    }
}

impl Default for EditPageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for EditPageTool {
    fn name(&self) -> &str {
        EditPageTool::name(self)
    }

    fn description(&self) -> &str {
        EditPageTool::description(self)
    }

    fn parameters_schema(&self) -> Value {
        EditPageTool::parameters_schema(self)
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        stood::perf_checkpoint!("awsdash.edit_page.execute.start");
        let _tool_guard = stood::perf_guard!("awsdash.edit_page.execute");

        // Parse input
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "edit_page requires parameters".to_string(),
        })?;

        let input: EditPageInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParameters {
                message: format!("Failed to parse edit_page input: {}", e),
            })?;

        // Validate page_name not empty
        if input.page_name.trim().is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "page_name cannot be empty".to_string(),
            });
        }

        // Validate task_description not empty
        if input.task_description.trim().is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "task_description cannot be empty".to_string(),
            });
        }

        // Verify the page exists
        let pages_dir = get_pages_dir().ok_or_else(|| ToolError::InvalidParameters {
            message: "Could not determine pages directory".to_string(),
        })?;

        let page_path = pages_dir.join(&input.page_name);

        // Security: Prevent path traversal
        if !page_path.starts_with(&pages_dir) {
            return Err(ToolError::InvalidParameters {
                message: "Invalid page name: path traversal not allowed".to_string(),
            });
        }

        if !page_path.exists() {
            return Err(ToolError::InvalidParameters {
                message: format!("Page '{}' not found. Check the page name and try again.", input.page_name),
            });
        }

        if !page_path.is_dir() {
            return Err(ToolError::InvalidParameters {
                message: format!("'{}' is not a valid page directory", input.page_name),
            });
        }

        // Get parent agent ID from thread-local
        let parent_id = get_current_agent_id().ok_or_else(|| ToolError::InvalidParameters {
            message: "Cannot determine parent agent ID - agent context not set".to_string(),
        })?;

        tracing::info!(
            target: "agent::edit_page",
            parent_id = %parent_id,
            "edit_page TOOL CALL:\n  Page Name: {}\n  Task Description: {}",
            input.page_name,
            input.task_description
        );

        // Build the task description with edit context
        let edit_task_description = format!(
            "You are EDITING an existing page named '{}'. \
            The page files already exist in the workspace. \
            Read the existing files first to understand the current implementation, \
            then make the following changes:\n\n{}",
            input.page_name,
            input.task_description
        );

        // Get VFS ID from parent (if TaskManager)
        let vfs_id = get_current_vfs_id();

        // Request page builder creation via channel (using existing workspace)
        stood::perf_checkpoint!(
            "awsdash.edit_page.request_creation.start",
            &format!("parent_id={}, page_name={}", parent_id, &input.page_name)
        );

        // Pass the existing page name as the workspace name with reuse_existing=true
        // This tells the creation function to skip collision detection and reuse the folder
        // Editing existing disk pages uses persistent=true (tool building prompt)
        let (agent_id, workspace_name) = stood::perf_timed!(
            "awsdash.edit_page.request_page_builder_creation",
            {
                request_page_builder_creation(
                    input.page_name.clone(),  // Use existing page name
                    input.concise_description.clone(),
                    edit_task_description,
                    None,       // No additional resource context
                    parent_id,
                    true,       // reuse_existing: reuse the existing folder
                    vfs_id,
                    true,       // is_persistent: editing existing pages uses tool building prompt
                )
            }
        )
        .map_err(|e| ToolError::InvalidParameters {
            message: format!("Failed to create page-builder worker: {}", e),
        })?;

        stood::perf_checkpoint!(
            "awsdash.edit_page.request_creation.end",
            &format!("worker_id={}, workspace={}", agent_id, workspace_name)
        );

        tracing::info!(
            target: "agent::edit_page",
            parent_id = %parent_id,
            agent_id = %agent_id,
            page_name = %input.page_name,
            "PageBuilderWorker created for editing, waiting for completion"
        );

        // Wait for worker to complete (10 minute timeout)
        let start_time = std::time::Instant::now();

        stood::perf_checkpoint!(
            "awsdash.edit_page.wait_completion.start",
            &format!("worker_id={}", agent_id)
        );

        match stood::perf_timed!("awsdash.edit_page.wait_for_worker_completion", {
            wait_for_worker_completion(agent_id, Duration::from_secs(600))
        }) {
            Ok(result) => {
                let execution_time_ms = start_time.elapsed().as_millis();
                stood::perf_checkpoint!(
                    "awsdash.edit_page.wait_completion.success",
                    &format!(
                        "worker_id={}, page_name={}, execution_time_ms={}",
                        agent_id, input.page_name, execution_time_ms
                    )
                );

                tracing::info!(
                    target: "agent::edit_page",
                    parent_id = %parent_id,
                    agent_id = %agent_id,
                    page_name = %input.page_name,
                    execution_time_ms = execution_time_ms,
                    "Page edit completed successfully"
                );

                // Return result
                Ok(ToolResult::success(json!({
                    "page_name": input.page_name,
                    "result": result,
                    "execution_time_ms": execution_time_ms,
                })))
            }
            Err(error) => {
                stood::perf_checkpoint!(
                    "awsdash.edit_page.wait_completion.error",
                    &format!("worker_id={}, error={}", agent_id, error)
                );

                tracing::error!(
                    target: "agent::edit_page",
                    parent_id = %parent_id,
                    agent_id = %agent_id,
                    error = %error,
                    "Page edit failed"
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
    fn test_edit_page_tool_creation() {
        let tool = EditPageTool::new();
        assert_eq!(tool.name(), "edit_page");
        assert!(tool.description().contains("Edit"));
        assert!(tool.description().contains("existing"));
    }

    #[test]
    fn test_edit_page_tool_schema() {
        let tool = EditPageTool::new();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("page_name")));
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("task_description")));
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("concise_description")));

        // Verify page_name property exists
        assert!(schema["properties"]["page_name"].is_object());
    }

    #[test]
    fn test_edit_page_tool_clone() {
        let tool1 = EditPageTool::new();
        let tool2 = tool1.clone();
        assert_eq!(tool1.name(), tool2.name());
    }
}
