//! Edit File Tool - Apply search/replace edits to files
//!
//! This tool allows Page Builder agents to edit existing files using SEARCH/REPLACE blocks.
//! Inspired by Aider's approach to avoid hitting max_tokens limits with large files.
//! Supports both disk-based and VFS-based workspaces.
//!
//! The LLM provides one or more edit blocks with:
//! - The exact text to search for (SEARCH)
//! - The replacement text (REPLACE)
//!
//! Benefits over write_file:
//! - Only returns changed portions (not entire file)
//! - Avoids hitting 4096 token output limits
//! - Multiple small edits can be applied in one call
//! - Safer - verifies SEARCH text exists before replacing

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use stood::tools::{Tool, ToolError, ToolResult};

use super::workspace::WorkspaceType;

/// Tool for editing files in the Page Builder workspace using SEARCH/REPLACE blocks
#[derive(Debug, Clone)]
pub struct EditFileTool {
    workspace: WorkspaceType,
    /// Keep for backwards compatibility with tests
    #[allow(dead_code)]
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct EditFileParams {
    /// Relative path to file to edit
    path: String,
    /// Array of search/replace edit blocks
    edits: Vec<EditBlock>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct EditBlock {
    /// Exact text to search for (must match exactly including whitespace)
    search: String,
    /// Replacement text
    replace: String,
}

#[derive(Debug, Serialize)]
struct EditFileResult {
    path: String,
    edits_applied: usize,
    edits_failed: usize,
    errors: Vec<String>,
}

impl EditFileTool {
    /// Create a new EditFileTool for the specified page workspace
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

    /// Validate that a relative path is safe
    fn validate_path(&self, relative_path: &str) -> Result<String> {
        // Prevent directory traversal
        if relative_path.contains("..") || relative_path.starts_with('/') {
            anyhow::bail!("Invalid path: directory traversal not allowed");
        }

        // Prevent subfolders
        if relative_path.contains('/') || relative_path.contains('\\') {
            anyhow::bail!(
                "Subfolders not allowed! All files must be directly in the workspace folder.\n\
                Your path: '{}'",
                relative_path
            );
        }

        // Validate with workspace
        self.workspace.validate_path(relative_path)?;

        Ok(relative_path.to_string())
    }

    /// Apply a single search/replace edit to content
    ///
    /// Returns (success, new_content, error_message)
    fn apply_edit(
        content: &str,
        edit: &EditBlock,
    ) -> (bool, String, Option<String>) {
        // Try exact match first
        if content.contains(&edit.search) {
            // Replace only the first occurrence (like aider)
            let new_content = content.replacen(&edit.search, &edit.replace, 1);
            return (true, new_content, None);
        }

        // Try with normalized whitespace (handles indentation differences)
        let normalized_content = normalize_whitespace(content);
        let normalized_search = normalize_whitespace(&edit.search);

        if normalized_content.contains(&normalized_search) {
            // Find the original text position in the un-normalized content
            // This is complex, so for now we'll report it as a fuzzy match failure
            let error = format!(
                "SEARCH text not found exactly. Found similar text with different whitespace.\n\
                Try adjusting indentation to match exactly.\n\
                SEARCH was:\n{}\n",
                &edit.search
            );
            return (false, content.to_string(), Some(error));
        }

        // No match found
        let error = format!(
            "SEARCH text not found in file.\n\
            Make sure the SEARCH block matches the file content exactly, character-for-character.\n\
            SEARCH was:\n{}\n",
            &edit.search
        );
        (false, content.to_string(), Some(error))
    }
}

/// Normalize whitespace for fuzzy matching
///
/// Converts all whitespace sequences to single spaces and trims
fn normalize_whitespace(text: &str) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(text.trim(), " ").to_string()
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit an existing file using SEARCH/REPLACE blocks.

This tool is MORE EFFICIENT than write_file for large files because:
- You only specify the parts that change (not the entire file)
- Avoids hitting the 4096 token output limit
- Multiple edits can be applied in one call

**How to use:**

Each edit block contains:
- `search`: Exact text to find (must match character-for-character)
- `replace`: New text to put in place of the search text

**Example - Add import at top of file:**
{
  \"path\": \"app.js\",
  \"edits\": [{
    \"search\": \"import React from 'react';\",
    \"replace\": \"import React from 'react';\\nimport axios from 'axios';\"
  }]
}

**Example - Update function:**
{
  \"path\": \"utils.js\",
  \"edits\": [{
    \"search\": \"function old() {\\n  return 'old';\\n}\",
    \"replace\": \"function new() {\\n  return 'new';\\n}\"
  }]
}

**Example - Multiple edits:**
{
  \"path\": \"index.html\",
  \"edits\": [
    {
      \"search\": \"<title>Old Title</title>\",
      \"replace\": \"<title>New Title</title>\"
    },
    {
      \"search\": \"<!-- Content here -->\",
      \"replace\": \"<div>Actual content</div>\"
    }
  ]
}

**CRITICAL RULES:**

1. SEARCH text must match EXACTLY - including:
   - All whitespace (spaces, tabs, newlines)
   - All punctuation
   - All comments
   - Letter case

2. Keep SEARCH blocks small and unique:
   - Include just enough context to uniquely identify the location
   - Don't include the entire file
   - 5-20 lines is usually sufficient

3. For large changes:
   - Break into multiple small edit blocks
   - Each block should change one logical section

4. File must exist - use write_file to create new files

**When to use edit_file vs write_file:**
- Use edit_file: Modifying existing files, especially large ones (>1000 chars)
- Use write_file: Creating new files or completely rewriting small files (<1000 chars)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path to file to edit (e.g., 'app.js', 'index.html')"
                },
                "edits": {
                    "type": "array",
                    "description": "Array of search/replace edit blocks to apply",
                    "items": {
                        "type": "object",
                        "properties": {
                            "search": {
                                "type": "string",
                                "description": "Exact text to search for (must match exactly)"
                            },
                            "replace": {
                                "type": "string",
                                "description": "Replacement text"
                            }
                        },
                        "required": ["search", "replace"]
                    }
                }
            },
            "required": ["path", "edits"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Parse parameters
        let params_value = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for edit_file".to_string(),
        })?;

        let params: EditFileParams =
            serde_json::from_value(params_value).map_err(|e| ToolError::InvalidParameters {
                message: format!("Invalid parameters: {}", e),
            })?;

        // Validate path
        if let Err(e) = self.validate_path(&params.path) {
            return Ok(ToolResult::error(format!("Invalid path: {}", e)));
        }

        // Check file exists
        match self.workspace.exists(&params.path) {
            Ok(false) => {
                return Ok(ToolResult::error(format!(
                    "File not found: {}. Use write_file to create new files.",
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

        // Read current content using workspace abstraction
        let mut content = match self.workspace.read_file_string(&params.path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to read file {}: {}",
                    params.path, e
                )));
            }
        };

        // Apply edits sequentially
        let mut edits_applied = 0;
        let mut edits_failed = 0;
        let mut errors = Vec::new();

        for (idx, edit) in params.edits.iter().enumerate() {
            let (success, new_content, error) = Self::apply_edit(&content, edit);

            if success {
                content = new_content;
                edits_applied += 1;
            } else {
                edits_failed += 1;
                if let Some(err_msg) = error {
                    errors.push(format!("Edit #{}: {}", idx + 1, err_msg));
                }
            }
        }

        // Write updated content if any edits succeeded using workspace abstraction
        if edits_applied > 0 {
            if let Err(e) = self.workspace.write_file(&params.path, content.as_bytes()) {
                return Ok(ToolResult::error(format!(
                    "Failed to write updated file {}: {}",
                    params.path, e
                )));
            }
        }

        // Return result
        let result = EditFileResult {
            path: params.path,
            edits_applied,
            edits_failed,
            errors,
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

        if edits_failed > 0 {
            // Return failed result with content showing details
            Ok(ToolResult {
                success: false,
                content: result_json,
                error: Some(format!(
                    "{} edits applied, {} failed. See content for details.",
                    edits_applied, edits_failed
                )),
            })
        } else {
            Ok(ToolResult::success(result_json))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_tool(_tool_name: &str, temp_dir: &TempDir) -> EditFileTool {
        let workspace_root = temp_dir.path().to_path_buf();
        fs::create_dir_all(&workspace_root).unwrap();

        EditFileTool {
            workspace: WorkspaceType::Disk {
                path: workspace_root.clone(),
            },
            workspace_root,
        }
    }

    fn create_test_file(tool: &EditFileTool, filename: &str, content: &str) {
        let path = tool.workspace_root.join(filename);
        fs::write(path, content).unwrap();
    }

    #[tokio::test]
    async fn test_edit_file_simple_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        // Create test file
        let original = "function hello() {\n  return 'world';\n}";
        create_test_file(&tool, "test.js", original);

        // Apply edit
        let params = Some(serde_json::json!({
            "path": "test.js",
            "edits": [{
                "search": "return 'world';",
                "replace": "return 'universe';"
            }]
        }));

        let result = tool.execute(params, None).await.unwrap();
        assert!(result.success);

        // Verify file was updated
        let updated = fs::read_to_string(tool.workspace_root.join("test.js")).unwrap();
        assert!(updated.contains("return 'universe';"));
        assert!(!updated.contains("return 'world';"));
    }

    #[tokio::test]
    async fn test_edit_file_multiple_edits() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let original = "<html>\n<title>Old</title>\n<body>Content</body>\n</html>";
        create_test_file(&tool, "index.html", original);

        let params = Some(serde_json::json!({
            "path": "index.html",
            "edits": [
                {
                    "search": "<title>Old</title>",
                    "replace": "<title>New</title>"
                },
                {
                    "search": "<body>Content</body>",
                    "replace": "<body>Updated Content</body>"
                }
            ]
        }));

        let result = tool.execute(params, None).await.unwrap();
        assert!(result.success);

        let updated = fs::read_to_string(tool.workspace_root.join("index.html")).unwrap();
        assert!(updated.contains("<title>New</title>"));
        assert!(updated.contains("<body>Updated Content</body>"));
    }

    #[tokio::test]
    async fn test_edit_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        let params = Some(serde_json::json!({
            "path": "nonexistent.js",
            "edits": [{
                "search": "foo",
                "replace": "bar"
            }]
        }));

        let result = tool.execute(params, None).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("File not found"));
    }

    #[tokio::test]
    async fn test_edit_file_search_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        create_test_file(&tool, "test.js", "const x = 1;");

        let params = Some(serde_json::json!({
            "path": "test.js",
            "edits": [{
                "search": "const y = 2;",  // This doesn't exist
                "replace": "const y = 3;"
            }]
        }));

        let result = tool.execute(params, None).await.unwrap();
        assert!(!result.success);

        let res: EditFileResult = serde_json::from_value(result.content).unwrap();
        assert_eq!(res.edits_failed, 1);
        assert_eq!(res.edits_applied, 0);
    }

    #[tokio::test]
    async fn test_edit_file_partial_success() {
        let temp_dir = TempDir::new().unwrap();
        let tool = create_test_tool("test-tool", &temp_dir);

        create_test_file(&tool, "test.js", "const x = 1;\nconst y = 2;");

        let params = Some(serde_json::json!({
            "path": "test.js",
            "edits": [
                {
                    "search": "const x = 1;",
                    "replace": "const x = 10;"  // This will succeed
                },
                {
                    "search": "const z = 3;",   // This will fail (doesn't exist)
                    "replace": "const z = 30;"
                }
            ]
        }));

        let result = tool.execute(params, None).await.unwrap();
        assert!(!result.success);  // Overall failed because one edit failed

        let res: EditFileResult = serde_json::from_value(result.content).unwrap();
        assert_eq!(res.edits_applied, 1);
        assert_eq!(res.edits_failed, 1);

        // Verify the successful edit was applied
        let updated = fs::read_to_string(tool.workspace_root.join("test.js")).unwrap();
        assert!(updated.contains("const x = 10;"));
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("  hello  world  "), "hello world");
        assert_eq!(normalize_whitespace("hello\n\nworld"), "hello world");
        assert_eq!(normalize_whitespace("  \t  hello  \t  "), "hello");
    }
}
