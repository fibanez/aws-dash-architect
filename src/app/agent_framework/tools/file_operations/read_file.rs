//! Read File Tool - Read files from Page Builder workspace
//!
//! This tool allows Page Builder agents to read file contents from their workspace.
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

/// Maximum file size that can be read into context (10KB)
/// Files larger than this should use copy_file or execute_javascript
const MAX_READ_SIZE: u64 = 10 * 1024;

/// Tool for reading files from the Page Builder workspace
#[derive(Debug, Clone)]
pub struct ReadFileTool {
    workspace: WorkspaceType,
    /// Keep for backwards compatibility with tests
    #[allow(dead_code)]
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReadFileParams {
    /// Relative path within page workspace (e.g., "index.html", "app.js")
    path: String,
}

#[derive(Debug, Serialize)]
struct ReadFileResult {
    content: String,
    path: String,
}

impl ReadFileTool {
    /// Create a new ReadFileTool for the specified page workspace
    ///
    /// # Arguments
    /// * `page_name` - Name of the page, or VFS pattern `vfs:{vfs_id}:{page_id}`
    ///
    /// # Examples
    /// ```ignore
    /// // Disk-based workspace
    /// let tool = ReadFileTool::new("my-s3-explorer")?;
    /// // Workspace: ~/.local/share/awsdash/pages/my-s3-explorer/
    ///
    /// // VFS-based workspace
    /// let tool = ReadFileTool::new("vfs:abc123:my-dashboard")?;
    /// // Files read from VFS at /pages/my-dashboard/
    /// ```
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
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file in the page workspace"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path within page workspace (e.g., 'index.html', 'app.js', 'styles.css')"
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
        // Parse parameters
        let params_value = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for read_file".to_string(),
        })?;

        let params: ReadFileParams =
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

        // Check file size - block large files to prevent context pollution
        match self.workspace.file_size(&params.path) {
            Ok(size) if size > MAX_READ_SIZE => {
                return Ok(ToolResult::error(format!(
                    "File too large to read into context: {} bytes (limit: {} bytes).\n\n\
                    CONTEXT POLLUTION WARNING: Reading large files wastes LLM tokens!\n\n\
                    Use one of these instead:\n\
                    1. copy_file - Copy VFS data directly to page workspace:\n\
                       copy_file({{ source: \"/results/file.json\", destination: \"data.js\", as_js_variable: \"DATA\" }})\n\n\
                    2. execute_javascript - Process data in V8 sandbox:\n\
                       const data = JSON.parse(vfs.readFile('/results/file.json'));\n\
                       const filtered = data.filter(...);\n\
                       // Return only summary, not full data\n\n\
                    Do NOT read large files into context - it's wasteful and slow!",
                    size, MAX_READ_SIZE
                )));
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to get file size: {}",
                    e
                )));
            }
            Ok(_) => {}
        }

        // Read file contents using workspace abstraction
        let content = match self.workspace.read_file_string(&params.path) {
            Ok(content) => content,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to read file {}: {}",
                    params.path, e
                )));
            }
        };

        // Create result
        let result = ReadFileResult {
            content,
            path: params.path,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_tool(_tool_name: &str, temp_dir: &TempDir) -> ReadFileTool {
        let workspace_root = temp_dir.path().to_path_buf();
        fs::create_dir_all(&workspace_root).unwrap();

        ReadFileTool {
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

        let result = tool.workspace.validate_path("index.html");
        assert!(result.is_ok());

        let result = tool.workspace.validate_path("assets/logo.png");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_prevents_directory_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let result = tool.workspace.validate_path("../other-tool/file.js");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("directory traversal"));

        let result = tool.workspace.validate_path("../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_prevents_absolute_paths() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let result = tool.workspace.validate_path("/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("directory traversal"));
    }

    #[tokio::test]
    async fn test_read_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        // Create a test file
        let test_file = tool.workspace_root.join("test.txt");
        fs::write(&test_file, "Hello, World!").unwrap();

        // Read the file
        let params = Some(serde_json::json!({
            "path": "test.txt"
        }));

        let tool_result = tool.execute(params, None).await.unwrap();
        assert!(tool_result.success);

        let result: ReadFileResult = serde_json::from_value(tool_result.content).unwrap();
        assert_eq!(result.content, "Hello, World!");
        assert_eq!(result.path, "test.txt");
    }

    #[tokio::test]
    async fn test_read_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let params = Some(serde_json::json!({
            "path": "nonexistent.txt"
        }));

        let result = tool.execute(params, None).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("File not found"));
    }

    #[tokio::test]
    async fn test_read_file_directory_traversal_blocked() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let params = Some(serde_json::json!({
            "path": "../other-tool/secret.txt"
        }));

        let result = tool.execute(params, None).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Invalid path"));
    }

    #[tokio::test]
    async fn test_read_file_in_subdirectory() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        // Create subdirectory and file
        let assets_dir = tool.workspace_root.join("assets");
        fs::create_dir_all(&assets_dir).unwrap();
        let test_file = assets_dir.join("config.json");
        fs::write(&test_file, r#"{"key": "value"}"#).unwrap();

        // Read the file
        let params = Some(serde_json::json!({
            "path": "assets/config.json"
        }));

        let tool_result = tool.execute(params, None).await.unwrap();
        assert!(tool_result.success);

        let result: ReadFileResult = serde_json::from_value(tool_result.content).unwrap();
        assert_eq!(result.content, r#"{"key": "value"}"#);
        assert_eq!(result.path, "assets/config.json");
    }

    #[test]
    fn test_tool_name() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        assert_eq!(tool.name(), "read_file");
    }

    #[test]
    fn test_tool_description() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let desc = tool.description();
        assert!(!desc.is_empty());
        assert!(desc.contains("Read") || desc.contains("read"));
    }

    #[test]
    fn test_parameters_schema() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["path"].is_object());
        assert_eq!(schema["required"][0], "path");
    }
}
