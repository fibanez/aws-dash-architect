//! Workspace Locking Middleware
//!
//! This middleware ensures TaskManager agents stay locked to a single tool workspace
//! per session. Once a workspace is set (via the first start_tool_builder call),
//! all subsequent calls must use the same workspace. Attempts to switch workspaces
//! are rejected with a clear error message.

#![warn(clippy::all, rust_2018_idioms)]

use async_trait::async_trait;
use serde_json::Value;
use stood::tools::middleware::{
    AfterToolAction, ToolContext, ToolMiddleware, ToolMiddlewareAction,
};
use stood::tools::ToolResult;

/// Workspace locking middleware for TaskManager agents
///
/// Enforces a single-workspace-per-session policy by:
/// 1. Tracking the first workspace used by each agent
/// 2. Allowing subsequent calls to the same workspace
/// 3. Rejecting attempts to switch to different workspaces
/// 4. Injecting context reminders after tool completion
#[derive(Debug)]
pub struct WorkspaceLockingMiddleware;

impl WorkspaceLockingMiddleware {
    /// Create a new workspace locking middleware instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorkspaceLockingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolMiddleware for WorkspaceLockingMiddleware {
    async fn before_tool(
        &self,
        tool_name: &str,
        params: &Value,
        ctx: &ToolContext,
    ) -> ToolMiddlewareAction {
        // Only intercept start_tool_builder
        if tool_name != "start_tool_builder" {
            return ToolMiddlewareAction::Continue;
        }

        // Extract agent_id from context (it's already a string)
        let agent_id = &ctx.agent_id;

        // Extract requested workspace_name from parameters
        let Some(requested_workspace) = params["workspace_name"].as_str() else {
            log::warn!("[WorkspaceLocking] start_tool_builder called without workspace_name parameter");
            return ToolMiddlewareAction::Continue;
        };

        // Check current workspace using global tracking
        let current_workspace =
            crate::app::agent_framework::get_current_workspace_for_agent_str(agent_id);

        match current_workspace {
            None => {
                // First tool creation - store workspace and continue
                log::info!(
                    "[WorkspaceLocking] Agent {} locked to workspace: {}",
                    agent_id,
                    requested_workspace
                );
                crate::app::agent_framework::set_current_workspace_for_agent_str(
                    agent_id,
                    requested_workspace,
                );
                ToolMiddlewareAction::Continue
            }
            Some(ref current) if current == requested_workspace => {
                // Same workspace - continue
                log::debug!(
                    "[WorkspaceLocking] Agent {} continuing work on workspace: {}",
                    agent_id,
                    requested_workspace
                );
                ToolMiddlewareAction::Continue
            }
            Some(ref current) => {
                // Different workspace - reject with clear error message
                let error_msg = format!(
                    "Workspace locked to: {}\n\n\
                     You are currently working on this tool workspace and can only make changes to it during this session.\n\
                     To work on a different tool, please create a new agent.\n\n\
                     Requested workspace: {} (rejected)",
                    current, requested_workspace
                );

                log::warn!(
                    "[WorkspaceLocking] Agent {} attempted to switch workspace from {} to {}",
                    agent_id,
                    current,
                    requested_workspace
                );

                // Return abort action with error result
                ToolMiddlewareAction::Abort {
                    reason: format!(
                        "Workspace switch rejected: {} -> {}",
                        current, requested_workspace
                    ),
                    synthetic_result: Some(ToolResult::error(error_msg)),
                }
            }
        }
    }

    async fn after_tool(
        &self,
        tool_name: &str,
        result: &ToolResult,
        _ctx: &ToolContext,
    ) -> AfterToolAction {
        // Only process start_tool_builder completions
        if tool_name != "start_tool_builder" || !result.success {
            return AfterToolAction::PassThrough;
        }

        // Inject context reminder about active workspace
        if let Some(workspace) = result.content.get("workspace_name").and_then(|v| v.as_str()) {
            let context = format!(
                "Tool workspace: {}. Reference this workspace name for future edits.",
                workspace
            );

            log::debug!(
                "[WorkspaceLocking] Injecting workspace context: {}",
                workspace
            );

            return AfterToolAction::InjectContext(context);
        }

        AfterToolAction::PassThrough
    }

    fn name(&self) -> &str {
        "WorkspaceLocking"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_workspace_locking_first_call() {
        let middleware = WorkspaceLockingMiddleware::new();
        let ctx = ToolContext::new("test-agent".to_string());
        let params = json!({
            "workspace_name": "test-workspace",
            "task_description": "Build a test tool"
        });

        let action = middleware
            .before_tool("start_tool_builder", &params, &ctx)
            .await;

        assert!(matches!(action, ToolMiddlewareAction::Continue));
    }

    #[tokio::test]
    async fn test_workspace_locking_ignores_other_tools() {
        let middleware = WorkspaceLockingMiddleware::new();
        let ctx = ToolContext::new("test-agent".to_string());
        let params = json!({
            "some_param": "value"
        });

        let action = middleware
            .before_tool("some_other_tool", &params, &ctx)
            .await;

        assert!(matches!(action, ToolMiddlewareAction::Continue));
    }

    #[tokio::test]
    async fn test_workspace_locking_after_tool_inject_context() {
        let middleware = WorkspaceLockingMiddleware::new();
        let ctx = ToolContext::new("test-agent".to_string());
        let result = ToolResult::success(json!({
            "workspace_name": "test-workspace",
            "status": "created"
        }));

        let action = middleware
            .after_tool("start_tool_builder", &result, &ctx)
            .await;

        match action {
            AfterToolAction::InjectContext(msg) => {
                assert!(msg.contains("test-workspace"));
                assert!(msg.contains("Tool workspace:"));
            }
            _ => panic!("Expected InjectContext action"),
        }
    }

    #[tokio::test]
    async fn test_workspace_locking_after_tool_passthrough_on_failure() {
        let middleware = WorkspaceLockingMiddleware::new();
        let ctx = ToolContext::new("test-agent".to_string());
        let result = ToolResult::error("Creation failed");

        let action = middleware
            .after_tool("start_tool_builder", &result, &ctx)
            .await;

        assert!(matches!(action, AfterToolAction::PassThrough));
    }
}
