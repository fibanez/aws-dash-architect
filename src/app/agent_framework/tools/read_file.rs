//! Read File Tool
//!
//! Allows AI agents to read file contents from the filesystem with security restrictions.
//! Only allows reading from trusted skill directories to prevent unauthorized file access.

use super::file_security::{validate_file_path, validate_file_size, SecurityError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{debug, info, warn};

/// Result of reading a file
#[derive(Debug, Serialize, Deserialize)]
pub struct FileContent {
    /// File path that was read
    pub path: String,
    /// File content as UTF-8 string
    pub content: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// Whether the file exists
    pub exists: bool,
}

/// Tool for reading file contents
#[derive(Clone, Debug, Default)]
pub struct ReadFileTool;

impl ReadFileTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        r#"Read the contents of a file from the filesystem.

This tool reads files with comprehensive security restrictions to prevent unauthorized access.

Security Constraints:
- Only allows reading from: ~/.claude/skills/, ~/.awsdash/skills/
- Rejects relative paths (must be absolute)
- Blocks path traversal attempts (../, symlinks)
- Refuses access to sensitive files (/etc/passwd, ~/.aws/credentials, ~/.ssh/, etc.)
- Enforces file size limit: 10MB maximum

Input Parameters:
- file_path: Absolute path to file (e.g., '/home/user/.claude/skills/aws-ec2/SKILL.md')

Output:
- path: Canonical file path
- content: Full file contents as UTF-8 string
- size_bytes: File size in bytes
- exists: Whether the file was found

Error Handling:
- Returns error if file doesn't exist
- Returns error if file size > 10MB
- Returns error if path is not absolute
- Returns error if path escapes allowed directories
- Returns error if file is not valid UTF-8

Examples:
1. Read a skill file:
   {"file_path": "/home/user/.claude/skills/aws-ec2-troubleshooting/SKILL.md"}

2. Read additional skill resource:
   {"file_path": "/home/user/.awsdash/skills/aws-s3-security/checklist.md"}

Common Use Cases:
- Load SKILL.md files for specialized knowledge
- Read additional skill resources (forms.md, reference.md, checklist.md)
- Access skill documentation and procedures"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file to read. Must be within allowed skill directories.",
                    "examples": [
                        "/home/user/.claude/skills/aws-ec2-troubleshooting/SKILL.md",
                        "/home/user/.awsdash/skills/aws-lambda-optimization/SKILL.md",
                        "/home/user/.claude/skills/aws-s3-security/checklist.md"
                    ]
                }
            },
            "required": ["file_path"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        info!("📖 read_file executing with parameters: {:?}", parameters);

        // Parse parameters
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for read_file".to_string(),
        })?;

        let file_path = params
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters {
                message: "Missing or invalid 'file_path' parameter".to_string(),
            })?;

        debug!("Reading file: {}", file_path);

        // Security validation
        let validated_path = match validate_file_path(file_path) {
            Ok(path) => path,
            Err(SecurityError::RelativePath(msg)) => {
                warn!("Security violation - relative path: {}", msg);
                return Err(ToolError::ExecutionFailed {
                    message: format!(
                        "Security error: Path must be absolute, not relative. Provided: {}",
                        file_path
                    ),
                });
            }
            Err(SecurityError::OutsideAllowedDirectory(msg)) => {
                warn!("Security violation - outside allowed directory: {}", msg);
                return Err(ToolError::ExecutionFailed {
                    message: format!(
                        "Security error: Path is outside allowed skill directories. Only ~/.claude/skills/ and ~/.awsdash/skills/ are allowed. Provided: {}",
                        file_path
                    ),
                });
            }
            Err(SecurityError::SensitiveFile(msg)) => {
                warn!("Security violation - sensitive file: {}", msg);
                return Err(ToolError::ExecutionFailed {
                    message: format!(
                        "Security error: Access to sensitive files is denied. Attempted: {}",
                        file_path
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
                // This shouldn't happen in validation, but handle it
                return Err(ToolError::ExecutionFailed {
                    message: "Unexpected file size error during validation".to_string(),
                });
            }
        };

        // Check if file exists
        if !validated_path.exists() {
            warn!("File not found: {:?}", validated_path);
            return Err(ToolError::ExecutionFailed {
                message: format!("File not found: {}", validated_path.display()),
            });
        }

        // Check file size
        let metadata = match fs::metadata(&validated_path) {
            Ok(m) => m,
            Err(e) => {
                return Err(ToolError::ExecutionFailed {
                    message: format!("Failed to read file metadata: {}", e),
                });
            }
        };

        let size_bytes = metadata.len();
        if let Err(SecurityError::FileTooLarge { current, max }) = validate_file_size(size_bytes) {
            warn!(
                "File too large: {} bytes (max: {} bytes)",
                current, max
            );
            return Err(ToolError::ExecutionFailed {
                message: format!(
                    "File size {} bytes exceeds maximum allowed {} bytes (10MB)",
                    current, max
                ),
            });
        }

        // Read file content
        let content = match fs::read_to_string(&validated_path) {
            Ok(c) => c,
            Err(e) => {
                return Err(ToolError::ExecutionFailed {
                    message: format!("Failed to read file content: {}", e),
                });
            }
        };

        let elapsed = start_time.elapsed();
        info!(
            "✅ read_file completed in {:?}: {} bytes from {}",
            elapsed,
            size_bytes,
            validated_path.display()
        );

        // Return result
        let result = FileContent {
            path: validated_path.display().to_string(),
            content,
            size_bytes,
            exists: true,
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
    async fn test_read_file_rejects_relative_paths() {
        let tool = ReadFileTool::new();

        let params = serde_json::json!({
            "file_path": "../secrets/data.txt"
        });

        let result = tool.execute(Some(params), None).await;
        assert!(result.is_err());

        if let Err(ToolError::ExecutionFailed { message: msg }) = result {
            assert!(msg.contains("absolute"));
        }
    }

    #[tokio::test]
    async fn test_read_file_rejects_outside_allowed_dirs() {
        let tool = ReadFileTool::new();

        let params = serde_json::json!({
            "file_path": "/tmp/test.txt"
        });

        let result = tool.execute(Some(params), None).await;
        assert!(result.is_err());

        if let Err(ToolError::ExecutionFailed { message: msg }) = result {
            assert!(msg.contains("outside allowed"));
        }
    }

    #[tokio::test]
    async fn test_read_file_in_allowed_directory() {
        // Create test directory and file
        let home = dirs::home_dir().unwrap();
        let test_dir = home.join(".awsdash/skills/test-read-tool");
        fs::create_dir_all(&test_dir).ok();

        let test_file = test_dir.join("test.md");
        fs::write(&test_file, "# Test Content\nThis is a test.").ok();

        let tool = ReadFileTool::new();

        let params = serde_json::json!({
            "file_path": test_file.to_string_lossy().to_string()
        });

        let result = tool.execute(Some(params), None).await;

        // Clean up
        fs::remove_dir_all(&test_dir).ok();

        assert!(result.is_ok(), "Should successfully read file in allowed directory");
    }
}
