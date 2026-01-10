//! Read File Tool - Read files from Page Builder workspace
//!
//! This tool allows Page Builder agents to read file contents from their workspace.
//! All file paths are validated to prevent directory traversal attacks.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use stood::tools::{Tool, ToolError, ToolResult};

/// Tool for reading files from the Page Builder workspace
#[derive(Debug, Clone)]
pub struct ReadFileTool {
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
    /// * `page_name` - Name of the page (used as workspace folder name)
    ///
    /// # Example
    /// ```ignore
    /// let tool = ReadFileTool::new("my-s3-explorer")?;
    /// // Workspace: ~/.local/share/awsdash/pages/my-s3-explorer/
    /// ```
    pub fn new(page_name: &str) -> Result<Self> {
        let workspace_root = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("awsdash/pages")
            .join(page_name);

        // Ensure workspace exists
        std::fs::create_dir_all(&workspace_root)
            .with_context(|| format!("Failed to create workspace directory: {:?}", workspace_root))?;

        Ok(Self { workspace_root })
    }

    /// Validate that a relative path is safe and within the workspace
    ///
    /// Returns the absolute path if valid, error otherwise
    fn validate_path(&self, relative_path: &str) -> Result<PathBuf> {
        // Prevent directory traversal
        if relative_path.contains("..") || relative_path.starts_with('/') {
            anyhow::bail!("Invalid path: directory traversal not allowed");
        }

        let full_path = self.workspace_root.join(relative_path);

        // Verify path is within workspace
        if !full_path.starts_with(&self.workspace_root) {
            anyhow::bail!("Path outside workspace");
        }

        Ok(full_path)
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

        // Validate and get full path
        let full_path = match self.validate_path(&params.path) {
            Ok(path) => path,
            Err(e) => {
                return Ok(ToolResult::error(format!("Invalid path: {}", e)));
            }
        };

        // Check if file exists
        if !full_path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                params.path
            )));
        }

        // Check if it's a file (not a directory)
        if !full_path.is_file() {
            return Ok(ToolResult::error(format!(
                "Path is not a file: {}",
                params.path
            )));
        }

        // Read file contents
        let content = match std::fs::read_to_string(&full_path) {
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

    fn create_test_tool(tool_name: &str, temp_dir: &TempDir) -> ReadFileTool {
        let workspace_root = temp_dir.path().join("awsdash/tools").join(tool_name);
        fs::create_dir_all(&workspace_root).unwrap();

        ReadFileTool { workspace_root }
    }

    #[test]
    fn test_validate_path_success() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let result = tool.validate_path("index.html");
        assert!(result.is_ok());

        let result = tool.validate_path("assets/logo.png");
        assert!(result.is_ok());
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
