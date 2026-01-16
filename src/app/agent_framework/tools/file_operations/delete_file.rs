//! Delete File Tool - Delete files from Page Builder workspace
//!
//! This tool allows Page Builder agents to delete files from their workspace.
//! All file paths are validated to prevent directory traversal attacks.
//! Supports both disk-based and VFS-based workspaces.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use stood::tools::{Tool, ToolError, ToolResult};

use super::workspace::WorkspaceType;

/// Tool for deleting files from the Page Builder workspace
#[derive(Debug, Clone)]
pub struct DeleteFileTool {
    workspace: WorkspaceType,
    /// Keep for backwards compatibility with tests
    #[allow(dead_code)]
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct DeleteFileParams {
    /// Relative path within page workspace
    path: String,
}

#[derive(Debug, Serialize)]
struct DeleteFileResult {
    path: String,
    deleted: bool,
}

impl DeleteFileTool {
    /// Create a new DeleteFileTool for the specified page workspace
    ///
    /// # Arguments
    /// * `page_name` - Name of the page, or VFS pattern `vfs:{vfs_id}:{page_id}`
    pub fn new(page_name: &str) -> Result<Self> {
        let workspace = WorkspaceType::from_workspace_name(page_name)?;

        // Extract path for backwards compatibility
        let workspace_root = match &workspace {
            WorkspaceType::Disk { path } => path.clone(),
            WorkspaceType::Vfs { page_id, .. } => {
                PathBuf::from(format!("/vfs/pages/{}", page_id))
            }
        };

        Ok(Self {
            workspace,
            workspace_root,
        })
    }
}

#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> &str {
        "Delete a file from the tool workspace"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path within tool workspace (e.g., 'old-file.txt', 'temp/data.json')"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let params_value = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for delete_file".to_string(),
        })?;

        let params: DeleteFileParams =
            serde_json::from_value(params_value).map_err(|e| ToolError::InvalidParameters {
                message: format!("Invalid parameters: {}", e),
            })?;

        // Validate path
        if let Err(e) = self.workspace.validate_path(&params.path) {
            return Ok(ToolResult::error(format!("Invalid path: {}", e)));
        }

        // Check if file exists
        match self.workspace.exists(&params.path) {
            Ok(false) => {
                return Ok(ToolResult::error(format!(
                    "File not found: {}",
                    params.path
                )));
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to check file existence: {}",
                    e
                )));
            }
            Ok(true) => {}
        }

        // Check if it's a file (not a directory)
        match self.workspace.is_file(&params.path) {
            Ok(false) => {
                return Ok(ToolResult::error(format!(
                    "Path is not a file: {}",
                    params.path
                )));
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to check file type: {}",
                    e
                )));
            }
            Ok(true) => {}
        }

        // Delete file using workspace abstraction
        match self.workspace.delete_file(&params.path) {
            Ok(_) => {
                let result = DeleteFileResult {
                    path: params.path,
                    deleted: true,
                };

                match serde_json::to_value(result) {
                    Ok(json) => Ok(ToolResult::success(json)),
                    Err(e) => Ok(ToolResult::error(format!(
                        "Failed to serialize result: {}",
                        e
                    ))),
                }
            }
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to delete file {}: {}",
                params.path, e
            ))),
        }
    }
}
