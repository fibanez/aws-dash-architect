//! Write File Tool - Write/create files in Page Builder workspace
//!
//! This tool allows Page Builder agents to create or overwrite files in their workspace.
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

/// Tool for writing files to the Page Builder workspace
#[derive(Debug, Clone)]
pub struct WriteFileTool {
    workspace: WorkspaceType,
    /// Keep for backwards compatibility with tests
    #[allow(dead_code)]
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct WriteFileParams {
    /// Relative path within page workspace
    path: String,
    /// File contents to write
    content: String,
}

#[derive(Debug, Serialize)]
struct WriteFileResult {
    path: String,
    bytes_written: usize,
}

impl WriteFileTool {
    /// Create a new WriteFileTool for the specified page workspace
    ///
    /// # Arguments
    /// * `page_name` - Name of the page, or VFS pattern `vfs:{vfs_id}:{page_id}`
    ///
    /// # Examples
    /// ```ignore
    /// // Disk-based workspace
    /// let tool = WriteFileTool::new("my-s3-explorer")?;
    /// // Workspace: ~/.local/share/awsdash/pages/my-s3-explorer/
    ///
    /// // VFS-based workspace
    /// let tool = WriteFileTool::new("vfs:abc123:my-dashboard")?;
    /// // Files stored in VFS at /pages/my-dashboard/
    /// ```
    pub fn new(page_name: &str) -> Result<Self> {
        let workspace = WorkspaceType::from_workspace_name(page_name)?;

        // Extract path for backwards compatibility
        let workspace_root = match &workspace {
            WorkspaceType::Disk { path } => path.clone(),
            WorkspaceType::Vfs { page_id, .. } => {
                // For VFS, create a placeholder path (not actually used)
                PathBuf::from(format!("/vfs/pages/{}", page_id))
            }
        };

        Ok(Self {
            workspace,
            workspace_root,
        })
    }

    /// Validate that a relative path is safe and within the workspace
    ///
    /// Returns the path string if valid, error otherwise
    fn validate_path(&self, relative_path: &str) -> Result<String> {
        // Prevent directory traversal
        if relative_path.contains("..") || relative_path.starts_with('/') {
            anyhow::bail!("Invalid path: directory traversal not allowed");
        }

        // Prevent subfolders - all files must be directly in the workspace folder
        if relative_path.contains('/') || relative_path.contains('\\') {
            anyhow::bail!(
                "Subfolders not allowed! All files must be directly in the workspace folder.\n\
                Use simple filenames like 'app.js', not 'js/app.js' or 'assets/logo.png'.\n\
                Subfolders cannot be deleted and will cause permanent clutter.\n\
                Your path: '{}'",
                relative_path
            );
        }

        // Validate with workspace
        self.workspace.validate_path(relative_path)?;

        Ok(relative_path.to_string())
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file in the page workspace.

Uses RELATIVE paths - files are created directly in the page workspace folder.

CRITICAL: All files MUST be directly in the workspace folder - NO subfolders!

Path format:
- Simple filename only (e.g., 'index.html', 'app.js', 'styles.css')
- NO slashes (/) or backslashes (\\)
- NO subfolders (subfolders cannot be deleted later!)

Correct examples:
  write_file('index.html', html)  ← Creates: pages/{workspace}/index.html
  write_file('app.js', code)      ← Creates: pages/{workspace}/app.js
  write_file('styles.css', css)   ← Creates: pages/{workspace}/styles.css
  write_file('logo.png', data)    ← Creates: pages/{workspace}/logo.png

WRONG examples:
  write_file('js/app.js', code)        ← NO! Creates subfolder (cannot delete!)
  write_file('assets/logo.png', img)   ← NO! Creates subfolder (cannot delete!)
  write_file('2005/app.js', code)      ← NO! Don't include workspace name
  write_file('/app.js', code)          ← NO! Absolute paths blocked

In HTML, reference files with full wry:// URLs:
  <script src='wry://localhost/pages/{workspace}/app.js'></script>"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path within page workspace (e.g., 'index.html', 'app.js', 'assets/logo.png')"
                },
                "content": {
                    "type": "string",
                    "description": "File contents to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Parse parameters
        let params_value = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for write_file".to_string(),
        })?;

        let params: WriteFileParams =
            serde_json::from_value(params_value).map_err(|e| ToolError::InvalidParameters {
                message: format!("Invalid parameters: {}", e),
            })?;

        // Validate path
        if let Err(e) = self.validate_path(&params.path) {
            return Ok(ToolResult::error(format!("Invalid path: {}", e)));
        }

        // Write file contents using workspace abstraction
        match self.workspace.write_file(&params.path, params.content.as_bytes()) {
            Ok(_) => {
                let result = WriteFileResult {
                    path: params.path,
                    bytes_written: params.content.len(),
                };

                let result_json = match serde_json::to_value(result) {
                    Ok(json) => json,
                    Err(e) => {
                        return Ok(ToolResult::error(format!(
                            "Failed to serialize result: {}",
                            e
                        )));
                    }
                };

                Ok(ToolResult::success(result_json))
            }
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to write file {}: {}",
                params.path, e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_tool(_tool_name: &str, temp_dir: &TempDir) -> WriteFileTool {
        let workspace_root = temp_dir.path().to_path_buf();
        fs::create_dir_all(&workspace_root).unwrap();

        WriteFileTool {
            workspace: WorkspaceType::Disk {
                path: workspace_root.clone(),
            },
            workspace_root,
        }
    }

    #[test]
    fn test_validate_path_success() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let result = tool.validate_path("index.html");
        assert!(result.is_ok());

        // Subfolders are blocked to prevent clutter
        let result = tool.validate_path("assets/logo.png");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Subfolders not allowed"));
    }

    #[test]
    fn test_validate_path_prevents_directory_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let result = tool.validate_path("../other-tool/file.js");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("directory traversal"));

        let result = tool.validate_path("../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_prevents_absolute_paths() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let result = tool.validate_path("/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("directory traversal"));
    }

    #[tokio::test]
    async fn test_write_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let params = Some(serde_json::json!({
            "path": "test.txt",
            "content": "Hello, World!"
        }));

        let tool_result = tool.execute(params, None).await.unwrap();
        assert!(tool_result.success);

        let result: WriteFileResult = serde_json::from_value(tool_result.content).unwrap();
        assert_eq!(result.path, "test.txt");
        assert_eq!(result.bytes_written, 13);

        // Verify file was actually written
        let file_path = tool.workspace_root.join("test.txt");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file_blocks_subdirectories() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        // Subfolders are blocked to prevent clutter
        let params = Some(serde_json::json!({
            "path": "assets/images/logo.png",
            "content": "PNG data here"
        }));

        let tool_result = tool.execute(params, None).await.unwrap();
        assert!(!tool_result.success);
        assert!(tool_result.error.unwrap().contains("Subfolders not allowed"));
    }

    #[tokio::test]
    async fn test_write_file_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        // Write initial content
        let file_path = tool.workspace_root.join("config.json");
        fs::write(&file_path, "old content").unwrap();

        // Overwrite with new content
        let params = Some(serde_json::json!({
            "path": "config.json",
            "content": "new content"
        }));

        let tool_result = tool.execute(params, None).await.unwrap();
        assert!(tool_result.success);

        // Verify content was overwritten
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }

    #[tokio::test]
    async fn test_write_file_directory_traversal_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let params = Some(serde_json::json!({
            "path": "../other-tool/malicious.txt",
            "content": "malicious content"
        }));

        let result = tool.execute(params, None).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid path"));
    }

    #[tokio::test]
    async fn test_write_file_empty_content() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let params = Some(serde_json::json!({
            "path": "empty.txt",
            "content": ""
        }));

        let tool_result = tool.execute(params, None).await.unwrap();
        assert!(tool_result.success);

        let result: WriteFileResult = serde_json::from_value(tool_result.content).unwrap();
        assert_eq!(result.bytes_written, 0);

        // Verify empty file was created
        let file_path = tool.workspace_root.join("empty.txt");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_tool_name() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        assert_eq!(tool.name(), "write_file");
    }

    #[test]
    fn test_tool_description() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let desc = tool.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("write") || desc.contains("Create"));
    }

    #[test]
    fn test_parameters_schema() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["properties"]["content"].is_object());
        assert_eq!(schema["required"][0], "path");
        assert_eq!(schema["required"][1], "content");
    }
}
