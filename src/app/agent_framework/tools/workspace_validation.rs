//! Workspace validation middleware for Page Builder agents
//!
//! Prevents agents from writing files with incorrect workspace references in URLs.

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use stood::tools::middleware::{
    AfterToolAction, ToolContext, ToolMiddleware, ToolMiddlewareAction,
};
use stood::tools::ToolResult;

/// Middleware that validates workspace paths in write_file operations
///
/// For Page Builder agents, this middleware intercepts `write_file` calls
/// and validates that file content doesn't reference incorrect workspace
/// names in `wry://localhost/pages/{workspace}/` URLs.
///
/// # Example Error
///
/// If workspace is "2028" but file contains "wry://localhost/pages/2005/app.js":
/// ```text
/// WORKSPACE VALIDATION ERROR
///
/// File content contains INCORRECT workspace name: 2005
/// Your workspace is: '2028'
///
/// YOU MUST USE: wry://localhost/pages/2028/
///
/// Example:
/// <script src="wry://localhost/pages/2028/app.js"></script>
/// ```
#[derive(Debug)]
pub struct WorkspaceValidationMiddleware {
    /// Expected workspace name for this agent
    workspace_name: String,
    /// Regex pattern to match wry:// URLs
    wry_url_pattern: Regex,
}

impl WorkspaceValidationMiddleware {
    /// Create new validation middleware for the given workspace
    pub fn new(workspace_name: impl Into<String>) -> Self {
        Self {
            workspace_name: workspace_name.into(),
            wry_url_pattern: Regex::new(r#"wry://localhost/pages/([^/\s"']+)/"#)
                .expect("Invalid regex pattern"),
        }
    }

    /// Validate write_file parameters for correct workspace references
    fn validate_write_file_content(&self, content: &str) -> Result<(), String> {
        let mut wrong_workspaces = Vec::new();

        // Scan for wry:// URLs with wrong workspace names
        for cap in self.wry_url_pattern.captures_iter(content) {
            if let Some(workspace_match) = cap.get(1) {
                let found_workspace = workspace_match.as_str();

                // Check if it matches the expected workspace
                if found_workspace != self.workspace_name {
                    wrong_workspaces.push(found_workspace.to_string());
                }
            }
        }

        if !wrong_workspaces.is_empty() {
            // Remove duplicates
            wrong_workspaces.sort();
            wrong_workspaces.dedup();

            return Err(format!(
                "WORKSPACE VALIDATION ERROR\n\
                 \n\
                 File content contains INCORRECT workspace name(s): {}\n\
                 \n\
                 Your workspace is: '{}'\n\
                 \n\
                 YOU MUST USE:\n\
                 wry://localhost/pages/{}/\n\
                 \n\
                 WRONG patterns detected:\n\
                 {}\n\
                 \n\
                 Fix all wry:// URLs to use the correct workspace name.\n\
                 \n\
                 Example:\n\
                 <link href=\"wry://localhost/pages/{}/styles.css\">\n\
                 <script src=\"wry://localhost/pages/{}/app.js\"></script>",
                wrong_workspaces.join(", "),
                self.workspace_name,
                self.workspace_name,
                wrong_workspaces
                    .iter()
                    .map(|w| format!("  * wry://localhost/pages/{}/  <- WRONG!", w))
                    .collect::<Vec<_>>()
                    .join("\n"),
                self.workspace_name,
                self.workspace_name
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl ToolMiddleware for WorkspaceValidationMiddleware {
    async fn before_tool(
        &self,
        tool_name: &str,
        params: &Value,
        _ctx: &ToolContext,
    ) -> ToolMiddlewareAction {
        // Only validate write_file calls
        if tool_name != "write_file" {
            return ToolMiddlewareAction::Continue;
        }

        // Extract content parameter
        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => {
                // No content parameter - let the tool handle this error
                return ToolMiddlewareAction::Continue;
            }
        };

        // Validate workspace references
        match self.validate_write_file_content(content) {
            Ok(()) => {
                // Validation passed - continue with execution
                ToolMiddlewareAction::Continue
            }
            Err(error_message) => {
                // Validation failed - abort with error
                tracing::warn!("Workspace validation failed: {}", error_message);

                ToolMiddlewareAction::Abort {
                    reason: "Workspace validation failed".to_string(),
                    synthetic_result: Some(ToolResult::error(error_message)),
                }
            }
        }
    }

    async fn after_tool(
        &self,
        _tool_name: &str,
        _result: &ToolResult,
        _ctx: &ToolContext,
    ) -> AfterToolAction {
        // No post-processing needed
        AfterToolAction::PassThrough
    }

    fn name(&self) -> &str {
        "WorkspaceValidation"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_correct_workspace() {
        let middleware = WorkspaceValidationMiddleware::new("2028");
        let params = json!({
            "path": "index.html",
            "content": r#"<script src="wry://localhost/pages/2028/app.js"></script>"#
        });

        let ctx = ToolContext::new("test-agent".to_string());
        let action = middleware.before_tool("write_file", &params, &ctx).await;
        assert!(matches!(action, ToolMiddlewareAction::Continue));
    }

    #[tokio::test]
    async fn test_wrong_workspace() {
        let middleware = WorkspaceValidationMiddleware::new("2028");
        let params = json!({
            "path": "index.html",
            "content": r#"<script src="wry://localhost/pages/2005/app.js"></script>"#
        });

        let ctx = ToolContext::new("test-agent".to_string());
        let action = middleware.before_tool("write_file", &params, &ctx).await;
        assert!(matches!(action, ToolMiddlewareAction::Abort { .. }));
    }

    #[tokio::test]
    async fn test_multiple_wrong_workspaces() {
        let middleware = WorkspaceValidationMiddleware::new("2028");
        let params = json!({
            "path": "index.html",
            "content": r#"
                <link href="wry://localhost/pages/2005/styles.css">
                <script src="wry://localhost/pages/page3/app.js"></script>
            "#
        });

        let ctx = ToolContext::new("test-agent".to_string());
        let action = middleware.before_tool("write_file", &params, &ctx).await;

        if let ToolMiddlewareAction::Abort {
            ref synthetic_result,
            ..
        } = action
        {
            if let Some(result) = synthetic_result {
                let error_msg = result.content.to_string();
                assert!(error_msg.contains("2005"));
                assert!(error_msg.contains("page3"));
            }
        } else {
            panic!("Expected Abort action");
        }
    }

    #[tokio::test]
    async fn test_no_wry_urls() {
        let middleware = WorkspaceValidationMiddleware::new("2028");
        let params = json!({
            "path": "index.html",
            "content": "<h1>Hello World</h1>"
        });

        let ctx = ToolContext::new("test-agent".to_string());
        let action = middleware.before_tool("write_file", &params, &ctx).await;
        assert!(matches!(action, ToolMiddlewareAction::Continue));
    }

    #[tokio::test]
    async fn test_non_write_file_tool() {
        let middleware = WorkspaceValidationMiddleware::new("2028");
        let params = json!({
            "path": "index.html"
        });

        let ctx = ToolContext::new("test-agent".to_string());
        // Should not validate read_file calls
        let action = middleware
            .before_tool("read_file", &params, &ctx)
            .await;
        assert!(matches!(action, ToolMiddlewareAction::Continue));
    }
}
