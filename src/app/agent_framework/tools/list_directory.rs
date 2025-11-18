//! List Directory Tool
//!
//! Allows AI agents to list directory contents with security restrictions.
//! Only allows listing trusted skill directories for skill discovery.

use super::file_security::{validate_directory_path, SecurityError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{debug, info, warn};

/// File entry with metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    /// File or directory name
    pub name: String,
    /// Full path to the file/directory
    pub path: String,
    /// File size in bytes (0 for directories)
    pub size_bytes: u64,
    /// Whether this is a directory
    pub is_directory: bool,
}

/// Directory listing result
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryListing {
    /// Directory that was listed
    pub directory: String,
    /// List of files in the directory
    pub files: Vec<FileEntry>,
    /// List of subdirectories
    pub directories: Vec<FileEntry>,
    /// Total number of items
    pub total_items: usize,
}

/// Tool for listing directory contents
#[derive(Clone, Debug, Default)]
pub struct ListDirectoryTool;

impl ListDirectoryTool {
    pub fn new() -> Self {
        Self
    }

    /// Check if a filename matches a glob pattern
    fn matches_pattern(filename: &str, pattern: &str) -> bool {
        if pattern.is_empty() || pattern == "*" {
            return true;
        }

        // Simple wildcard matching
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                // Pattern like "*.md" or "SKILL.*"
                let prefix = parts[0];
                let suffix = parts[1];

                if !prefix.is_empty() && !filename.starts_with(prefix) {
                    return false;
                }
                if !suffix.is_empty() && !filename.ends_with(suffix) {
                    return false;
                }
                return true;
            }
        }

        // Exact match
        filename == pattern
    }
}

#[async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn description(&self) -> &str {
        r#"List files and directories in a given path.

This tool lists directory contents with comprehensive security restrictions.

Security Constraints:
- Only allows listing: ~/.claude/skills/, ~/.awsdash/skills/
- Rejects relative paths (must be absolute)
- Blocks path traversal attempts (../, symlinks)
- Does not traverse into subdirectories (non-recursive by default)

Input Parameters:
- directory_path: Absolute path to directory (e.g., '/home/user/.claude/skills/')
- pattern: Optional glob pattern to filter files (e.g., '*.md', 'SKILL.*', '*')
  - Default: '*' (matches all files)
  - Supports simple wildcards: '*.md' (ends with .md), 'SKILL.*' (starts with SKILL.)

Output:
- directory: Canonical directory path that was listed
- files: Array of file entries with name, path, size, is_directory flag
- directories: Array of subdirectory entries
- total_items: Total count of files and directories

Error Handling:
- Returns error if directory doesn't exist
- Returns error if path is not absolute
- Returns error if path escapes allowed directories
- Returns empty lists if no files match pattern

Examples:
1. List all files in skills directory:
   {"directory_path": "/home/user/.claude/skills/"}

2. List only SKILL.md files:
   {"directory_path": "/home/user/.claude/skills/", "pattern": "SKILL.md"}

3. List all markdown files:
   {"directory_path": "/home/user/.awsdash/skills/aws-ec2-troubleshooting/", "pattern": "*.md"}

Common Use Cases:
- Discover available skills by listing ~/.claude/skills/
- Find SKILL.md files in skill directories
- List additional skill resources (forms.md, reference.md, etc.)
- Verify skill directory structure"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "directory_path": {
                    "type": "string",
                    "description": "Absolute path to the directory to list. Must be within allowed skill directories.",
                    "examples": [
                        "/home/user/.claude/skills/",
                        "/home/user/.awsdash/skills/",
                        "/home/user/.claude/skills/aws-ec2-troubleshooting/"
                    ]
                },
                "pattern": {
                    "type": "string",
                    "description": "Optional glob pattern to filter files (e.g., '*.md', 'SKILL.*'). Defaults to '*' (all files).",
                    "examples": ["*.md", "SKILL.*", "SKILL.md", "*"],
                    "default": "*"
                }
            },
            "required": ["directory_path"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        info!("📂 list_directory executing with parameters: {:?}", parameters);

        // Parse parameters
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for list_directory".to_string(),
        })?;

        let directory_path = params
            .get("directory_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters {
                message: "Missing or invalid 'directory_path' parameter".to_string(),
            })?;

        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("*");

        debug!("Listing directory: {} with pattern: {}", directory_path, pattern);

        // Security validation
        let validated_path = match validate_directory_path(directory_path) {
            Ok(path) => path,
            Err(SecurityError::RelativePath(msg)) => {
                warn!("Security violation - relative path: {}", msg);
                return Err(ToolError::ExecutionFailed {
                    message: format!(
                        "Security error: Path must be absolute, not relative. Provided: {}",
                        directory_path
                    ),
                });
            }
            Err(SecurityError::OutsideAllowedDirectory(msg)) => {
                warn!("Security violation - outside allowed directory: {}", msg);
                return Err(ToolError::ExecutionFailed {
                    message: format!(
                        "Security error: Path is outside allowed skill directories. Only ~/.claude/skills/ and ~/.awsdash/skills/ are allowed. Provided: {}",
                        directory_path
                    ),
                });
            }
            Err(SecurityError::SensitiveFile(msg)) => {
                warn!("Security violation - sensitive directory: {}", msg);
                return Err(ToolError::ExecutionFailed {
                    message: format!(
                        "Security error: Access to sensitive directories is denied. Attempted: {}",
                        directory_path
                    ),
                });
            }
            Err(SecurityError::InvalidPath(msg)) => {
                return Err(ToolError::ExecutionFailed {
                    message: format!("Invalid path: {}", msg),
                });
            }
            Err(SecurityError::InvalidComponents(msg)) => {
                warn!("Security violation - invalid path components: {}", msg);
                return Err(ToolError::ExecutionFailed {
                    message: format!("Security error: {}", msg),
                });
            }
            Err(SecurityError::FileTooLarge { .. }) => {
                // Not applicable for directories
                return Err(ToolError::ExecutionFailed {
                    message: "Unexpected error during validation".to_string(),
                });
            }
        };

        // Check if directory exists
        if !validated_path.exists() {
            warn!("Directory not found: {:?}", validated_path);
            return Err(ToolError::ExecutionFailed {
                message: format!("Directory not found: {}", validated_path.display()),
            });
        }

        if !validated_path.is_dir() {
            return Err(ToolError::ExecutionFailed {
                message: format!("Path is not a directory: {}", validated_path.display()),
            });
        }

        // Read directory contents
        let entries = match fs::read_dir(&validated_path) {
            Ok(e) => e,
            Err(e) => {
                return Err(ToolError::ExecutionFailed {
                    message: format!("Failed to read directory: {}", e),
                });
            }
        };

        let mut files = Vec::new();
        let mut directories = Vec::new();

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip hidden files/directories (start with .)
            if filename.starts_with('.') {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to read metadata for {:?}: {}", path, e);
                    continue;
                }
            };

            let is_directory = metadata.is_dir();
            let size_bytes = if is_directory { 0 } else { metadata.len() };

            let file_entry = FileEntry {
                name: filename.clone(),
                path: path.display().to_string(),
                size_bytes,
                is_directory,
            };

            if is_directory {
                directories.push(file_entry);
            } else {
                // Apply pattern filter for files only
                if Self::matches_pattern(&filename, pattern) {
                    files.push(file_entry);
                }
            }
        }

        // Sort for consistent output
        files.sort_by(|a, b| a.name.cmp(&b.name));
        directories.sort_by(|a, b| a.name.cmp(&b.name));

        let total_items = files.len() + directories.len();

        let elapsed = start_time.elapsed();
        info!(
            "✅ list_directory completed in {:?}: {} items from {}",
            elapsed,
            total_items,
            validated_path.display()
        );

        // Return result
        let result = DirectoryListing {
            directory: validated_path.display().to_string(),
            files,
            directories,
            total_items,
        };

        let result_json = serde_json::to_value(result).map_err(|e| ToolError::ExecutionFailed {
            message: format!("Failed to serialize result: {}", e),
        })?;

        Ok(ToolResult::success(result_json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_list_directory_rejects_relative_paths() {
        let tool = ListDirectoryTool::new();

        let params = serde_json::json!({
            "directory_path": "../secrets/"
        });

        let result = tool.execute(Some(params), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_directory_rejects_outside_allowed_dirs() {
        let tool = ListDirectoryTool::new();

        let params = serde_json::json!({
            "directory_path": "/tmp/"
        });

        let result = tool.execute(Some(params), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pattern_matching() {
        assert!(ListDirectoryTool::matches_pattern("SKILL.md", "*.md"));
        assert!(ListDirectoryTool::matches_pattern("SKILL.md", "SKILL.*"));
        assert!(ListDirectoryTool::matches_pattern("test.md", "*.md"));
        assert!(!ListDirectoryTool::matches_pattern("test.txt", "*.md"));
        assert!(ListDirectoryTool::matches_pattern("anything", "*"));
        assert!(ListDirectoryTool::matches_pattern("SKILL.md", "SKILL.md"));
    }

    #[tokio::test]
    async fn test_list_directory_in_allowed_directory() {
        // Create test directory
        let home = dirs::home_dir().unwrap();
        let test_dir = home.join(".awsdash/skills/test-list-tool");
        fs::create_dir_all(&test_dir).ok();

        // Create test files
        fs::write(test_dir.join("SKILL.md"), "# Test Skill").ok();
        fs::write(test_dir.join("forms.md"), "# Forms").ok();
        fs::write(test_dir.join("test.txt"), "text file").ok();

        let tool = ListDirectoryTool::new();

        // List all files
        let params = serde_json::json!({
            "directory_path": test_dir.to_string_lossy().to_string()
        });

        let result = tool.execute(Some(params), None).await;

        // Clean up
        fs::remove_dir_all(&test_dir).ok();

        assert!(result.is_ok(), "Should successfully list directory");
    }

    #[tokio::test]
    async fn test_list_directory_with_pattern() {
        // Create test directory
        let home = dirs::home_dir().unwrap();
        let test_dir = home.join(".awsdash/skills/test-pattern-tool");
        fs::create_dir_all(&test_dir).ok();

        // Create test files
        fs::write(test_dir.join("SKILL.md"), "# Test Skill").ok();
        fs::write(test_dir.join("forms.md"), "# Forms").ok();
        fs::write(test_dir.join("test.txt"), "text file").ok();

        let tool = ListDirectoryTool::new();

        // List only .md files
        let params = serde_json::json!({
            "directory_path": test_dir.to_string_lossy().to_string(),
            "pattern": "*.md"
        });

        let result = tool.execute(Some(params), None).await;

        // Clean up
        fs::remove_dir_all(&test_dir).ok();

        assert!(result.is_ok(), "Should successfully list directory with pattern");
    }
}
