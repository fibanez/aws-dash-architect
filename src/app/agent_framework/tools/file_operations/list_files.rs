//! List Files Tool - List files and directories in Page Builder workspace
//!
//! This tool allows Page Builder agents to list files and directories in their workspace.
//! All file paths are validated to prevent directory traversal attacks.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use stood::tools::{Tool, ToolError, ToolResult};

/// Tool for listing files in the Page Builder workspace
#[derive(Debug, Clone)]
pub struct ListFilesTool {
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
    pub fn new(page_name: &str) -> Result<Self> {
        let workspace_root = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("awsdash/pages")
            .join(page_name);

        std::fs::create_dir_all(&workspace_root)
            .with_context(|| format!("Failed to create workspace directory: {:?}", workspace_root))?;

        Ok(Self { workspace_root })
    }

    fn validate_path(&self, relative_path: Option<&str>) -> Result<PathBuf> {
        let path = match relative_path {
            Some(p) => {
                if p.contains("..") || p.starts_with('/') {
                    anyhow::bail!("Invalid path: directory traversal not allowed");
                }
                self.workspace_root.join(p)
            }
            None => self.workspace_root.clone(),
        };

        if !path.starts_with(&self.workspace_root) {
            anyhow::bail!("Path outside workspace");
        }

        Ok(path)
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

        let full_path = match self.validate_path(params.path.as_deref()) {
            Ok(path) => path,
            Err(e) => {
                return Ok(ToolResult::error(format!("Invalid path: {}", e)));
            }
        };

        if !full_path.exists() {
            return Ok(ToolResult::error("Directory not found".to_string()));
        }

        if !full_path.is_dir() {
            return Ok(ToolResult::error("Path is not a directory".to_string()));
        }

        let mut files = Vec::new();

        match std::fs::read_dir(&full_path) {
            Ok(entries) => {
                for entry_result in entries {
                    match entry_result {
                        Ok(entry) => {
                            if let Ok(metadata) = entry.metadata() {
                                let name = entry.file_name().to_string_lossy().to_string();

                                let relative_path = match entry
                                    .path()
                                    .strip_prefix(&self.workspace_root)
                                {
                                    Ok(p) => p.to_string_lossy().to_string(),
                                    Err(_) => continue,
                                };

                                files.push(FileEntry {
                                    name,
                                    path: relative_path,
                                    is_directory: metadata.is_dir(),
                                    size_bytes: metadata.len(),
                                });
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Err(e) => {
                return Ok(ToolResult::error(format!("Failed to read directory: {}", e)));
            }
        }

        // Sort: directories first, then alphabetically
        files.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

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
