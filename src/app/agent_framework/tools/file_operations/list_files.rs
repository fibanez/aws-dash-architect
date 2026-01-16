//! List Files Tool - List files and directories in Page Builder workspace
//!
//! This tool allows Page Builder agents to list files and directories in their workspace.
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

/// Tool for listing files in the Page Builder workspace
#[derive(Debug, Clone)]
pub struct ListFilesTool {
    workspace: WorkspaceType,
    /// Keep for backwards compatibility with tests
    #[allow(dead_code)]
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct ListFilesParams {
    /// Relative path to directory (optional, defaults to root)
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_directory: bool,
    size_bytes: u64,
}

#[derive(Debug, Serialize)]
struct ListFilesResult {
    files: Vec<FileEntry>,
    total_count: usize,
}

impl ListFilesTool {
    /// Create a new ListFilesTool for the specified page workspace
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
impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files and directories in the page workspace"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to directory (optional, defaults to root)"
                }
            }
        })
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let params: ListFilesParams = match parameters {
            Some(p) => {
                serde_json::from_value(p).map_err(|e| ToolError::InvalidParameters {
                    message: format!("Invalid parameters: {}", e),
                })?
            }
            None => ListFilesParams { path: None },
        };

        // Validate path if provided
        if let Some(ref path) = params.path {
            if let Err(e) = self.workspace.validate_path(path) {
                return Ok(ToolResult::error(format!("Invalid path: {}", e)));
            }
        }

        // List directory using workspace abstraction
        let entries = match self.workspace.list_dir(params.path.as_deref()) {
            Ok(entries) => entries,
            Err(e) => {
                return Ok(ToolResult::error(format!("Failed to list directory: {}", e)));
            }
        };

        // Convert to FileEntry format
        let files: Vec<FileEntry> = entries
            .into_iter()
            .map(|e| FileEntry {
                name: e.name,
                path: e.path,
                is_directory: e.is_directory,
                size_bytes: e.size_bytes,
            })
            .collect();

        let result = ListFilesResult {
            total_count: files.len(),
            files,
        };

        match serde_json::to_value(result) {
            Ok(json) => Ok(ToolResult::success(json)),
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to serialize result: {}",
                e
            ))),
        }
    }
}
