//! Delete File Tool - Delete files from Page Builder workspace
//!
//! This tool allows Page Builder agents to delete files from their workspace.
//! All file paths are validated to prevent directory traversal attacks.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use stood::tools::{Tool, ToolError, ToolResult};

/// Tool for deleting files from the Page Builder workspace
#[derive(Debug, Clone)]
pub struct DeleteFileTool {
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
    pub fn new(page_name: &str) -> Result<Self> {
        let workspace_root = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("awsdash/pages")
            .join(page_name);

        std::fs::create_dir_all(&workspace_root)
            .with_context(|| format!("Failed to create workspace directory: {:?}", workspace_root))?;

        Ok(Self { workspace_root })
    }

    fn validate_path(&self, relative_path: &str) -> Result<PathBuf> {
        if relative_path.contains("..") || relative_path.starts_with('/') {
            anyhow::bail!("Invalid path: directory traversal not allowed");
        }

        let full_path = self.workspace_root.join(relative_path);

        if !full_path.starts_with(&self.workspace_root) {
            anyhow::bail!("Path outside workspace");
        }

        Ok(full_path)
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

        let full_path = match self.validate_path(&params.path) {
            Ok(path) => path,
            Err(e) => {
                return Ok(ToolResult::error(format!("Invalid path: {}", e)));
            }
        };

        if !full_path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                params.path
            )));
        }

        if !full_path.is_file() {
            return Ok(ToolResult::error(format!(
                "Path is not a file: {}",
                params.path
            )));
        }

        match std::fs::remove_file(&full_path) {
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
