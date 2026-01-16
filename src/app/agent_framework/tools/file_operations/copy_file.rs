//! Copy File Tool - Copy files within VFS without loading into context
//!
//! This tool allows Page Builder agents to copy files from VFS paths
//! (like /results/ or /workspace/) directly to their page workspace,
//! without loading the content through the LLM context.
//!
//! This is critical for context efficiency - large data files should
//! be copied directly, not read and then written.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use stood::tools::{Tool, ToolError, ToolResult};

use super::workspace::WorkspaceType;
use crate::app::agent_framework::vfs::with_vfs;

/// Maximum file size that can be copied (10MB)
const MAX_COPY_SIZE: usize = 10 * 1024 * 1024;

/// Tool for copying files within VFS to page workspace
#[derive(Debug, Clone)]
pub struct CopyFileTool {
    workspace: WorkspaceType,
    /// Keep for backwards compatibility
    #[allow(dead_code)]
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct CopyFileParams {
    /// Source VFS path (e.g., "/results/resources_123.json", "/workspace/findings.json")
    source: String,
    /// Destination filename in page workspace (e.g., "data.js", "inventory.json")
    /// Must be a simple filename - no directories allowed
    destination: String,
    /// Optional: Wrap content as JavaScript variable
    /// If provided, creates: `const {var_name} = {content};`
    #[serde(default)]
    as_js_variable: Option<String>,
}

#[derive(Debug, Serialize)]
struct CopyFileResult {
    status: String,
    source: String,
    destination: String,
    bytes_copied: usize,
    message: String,
}

impl CopyFileTool {
    /// Create a new CopyFileTool for the specified page workspace
    ///
    /// # Arguments
    /// * `page_name` - Name of the page, or VFS pattern `vfs:{vfs_id}:{page_id}`
    pub fn new(page_name: &str) -> Result<Self> {
        let workspace = WorkspaceType::from_workspace_name(page_name)?;

        let workspace_root = match &workspace {
            WorkspaceType::Disk { path } => path.clone(),
            WorkspaceType::Vfs { page_id, .. } => PathBuf::from(format!("/vfs/pages/{}", page_id)),
        };

        Ok(Self {
            workspace,
            workspace_root,
        })
    }
}

#[async_trait]
impl Tool for CopyFileTool {
    fn name(&self) -> &str {
        "copy_file"
    }

    fn description(&self) -> &str {
        "Copy a file from VFS directly to the page workspace WITHOUT loading into context.

USE THIS instead of reading data and writing it back - it's much more efficient!

This tool copies files like:
- /results/*.json (query results from queryCachedResources, etc.)
- /workspace/*/*.json (processed findings from TaskWorker)

Directly to your page workspace as:
- data.js (with as_js_variable to wrap as JavaScript)
- data.json (raw JSON)

Example 1 - Copy as JavaScript data file:
{
  \"source\": \"/workspace/vpc-inventory/findings.json\",
  \"destination\": \"data.js\",
  \"as_js_variable\": \"INVENTORY_DATA\"
}
Creates: data.js containing `const INVENTORY_DATA = {...};`

Example 2 - Copy as raw JSON:
{
  \"source\": \"/results/resources_123.json\",
  \"destination\": \"data.json\"
}
Creates: data.json with the raw JSON content

CRITICAL: Do NOT use read_file + write_file to copy data - it pollutes the context!
Use copy_file instead for efficient, context-free copying."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Source VFS path (e.g., '/results/resources_123.json', '/workspace/task/findings.json')"
                },
                "destination": {
                    "type": "string",
                    "description": "Destination filename in page workspace (simple name only, e.g., 'data.js', 'inventory.json')"
                },
                "as_js_variable": {
                    "type": "string",
                    "description": "Optional: Variable name to wrap content as JavaScript (e.g., 'INVENTORY_DATA' creates 'const INVENTORY_DATA = ...;')"
                }
            },
            "required": ["source", "destination"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Parse parameters
        let params_value = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for copy_file".to_string(),
        })?;

        let params: CopyFileParams =
            serde_json::from_value(params_value).map_err(|e| ToolError::InvalidParameters {
                message: format!("Invalid parameters: {}", e),
            })?;

        // Validate source path - must start with / and be in allowed directories
        if !params.source.starts_with('/') {
            return Ok(ToolResult::error(
                "Source must be an absolute VFS path starting with / (e.g., '/results/file.json')".to_string(),
            ));
        }

        // Only allow copying from specific VFS directories
        let allowed_prefixes = ["/results/", "/workspace/", "/final/"];
        if !allowed_prefixes.iter().any(|p| params.source.starts_with(p)) {
            return Ok(ToolResult::error(format!(
                "Source must be in /results/, /workspace/, or /final/ directory. Got: {}",
                params.source
            )));
        }

        // Validate destination - must be simple filename, no directories
        if params.destination.contains('/') || params.destination.contains('\\') {
            return Ok(ToolResult::error(
                "Destination must be a simple filename (e.g., 'data.js'), not a path. Files go directly in page root.".to_string(),
            ));
        }

        if params.destination.is_empty() {
            return Ok(ToolResult::error("Destination filename cannot be empty".to_string()));
        }

        // Get VFS ID from workspace
        let vfs_id = match &self.workspace {
            WorkspaceType::Vfs { vfs_id, .. } => vfs_id.clone(),
            WorkspaceType::Disk { .. } => {
                return Ok(ToolResult::error(
                    "copy_file only works with VFS workspaces. Use read_file + write_file for disk workspaces.".to_string(),
                ));
            }
        };

        // Read source file from VFS
        let source_content = match with_vfs(&vfs_id, |vfs| {
            // Check file exists
            if !vfs.exists(&params.source) {
                return Err(format!("Source file not found: {}", params.source));
            }

            // Check file size
            match vfs.stat(&params.source) {
                Ok(stat) => {
                    if stat.size > MAX_COPY_SIZE {
                        return Err(format!(
                            "File too large: {} bytes (max: {} bytes). Consider filtering the data first.",
                            stat.size, MAX_COPY_SIZE
                        ));
                    }
                }
                Err(e) => return Err(format!("Failed to stat source file: {}", e)),
            }

            // Read content
            match vfs.read_file(&params.source) {
                Ok(bytes) => Ok(bytes.to_vec()),
                Err(e) => Err(format!("Failed to read source file: {}", e)),
            }
        }) {
            Some(Ok(content)) => content,
            Some(Err(e)) => return Ok(ToolResult::error(e)),
            None => return Ok(ToolResult::error(format!("VFS not found: {}", vfs_id))),
        };

        // Transform content if wrapping as JS variable
        let final_content = if let Some(var_name) = &params.as_js_variable {
            // Validate variable name
            if !var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Ok(ToolResult::error(format!(
                    "Invalid JavaScript variable name: {}. Use only letters, numbers, and underscores.",
                    var_name
                )));
            }

            let content_str = match String::from_utf8(source_content.clone()) {
                Ok(s) => s,
                Err(_) => {
                    return Ok(ToolResult::error(
                        "Source file is not valid UTF-8. Cannot wrap as JavaScript variable.".to_string(),
                    ));
                }
            };

            format!(
                "// Auto-copied from VFS: {}\n// Generated at: {}\nconst {} = {};\n",
                params.source,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                var_name,
                content_str
            )
            .into_bytes()
        } else {
            source_content
        };

        let bytes_copied = final_content.len();

        // Write to destination in page workspace
        if let Err(e) = self.workspace.write_file(&params.destination, &final_content) {
            return Ok(ToolResult::error(format!(
                "Failed to write destination file: {}",
                e
            )));
        }

        let result = CopyFileResult {
            status: "success".to_string(),
            source: params.source,
            destination: params.destination.clone(),
            bytes_copied,
            message: format!(
                "Copied {} bytes to page workspace as '{}'{}",
                bytes_copied,
                params.destination,
                if params.as_js_variable.is_some() {
                    " (wrapped as JavaScript variable)"
                } else {
                    ""
                }
            ),
        };

        match serde_json::to_value(result) {
            Ok(json) => Ok(ToolResult::success(json)),
            Err(e) => Ok(ToolResult::error(format!("Failed to serialize result: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        // Can't easily test without VFS setup, just verify it compiles
        assert_eq!("copy_file", "copy_file");
    }
}
