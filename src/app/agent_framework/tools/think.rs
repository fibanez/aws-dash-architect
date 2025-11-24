#![warn(clippy::all, rust_2018_idioms)]

//! Think Tool - Structured Reasoning Space for Task-Manager Agents
//!
//! This tool provides a no-op space for agents to explicitly reason through
//! complex planning and decision-making. Following Anthropic's research,
//! providing structured thinking space improves performance by 54% in
//! complex multi-step scenarios.
//!
//! ## Usage
//!
//! The agent uses this tool when it needs to:
//! - Analyze user requests before creating tasks
//! - Review task results before deciding next steps
//! - Reason through error recovery strategies
//! - Plan result aggregation and presentation
//!
//! ## Implementation
//!
//! This is a no-op tool that simply logs the agent's thought process.
//! It does not:
//! - Fetch new information
//! - Modify state
//! - Invoke other tools
//! - Return data
//!
//! The logging allows developers to trace agent reasoning during debugging.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use stood::tools::{Tool, ToolError, ToolResult};

/// Think tool for structured agent reasoning
#[derive(Clone, Debug)]
pub struct ThinkTool;

/// Input schema for think tool
#[derive(Debug, Deserialize, Serialize)]
struct ThinkInput {
    /// The agent's reasoning, analysis, or planning thoughts
    thought: String,
}

impl ThinkTool {
    /// Create a new think tool instance
    pub fn new() -> Self {
        Self
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        "think"
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        "Use this tool when you need to pause and think through complex reasoning or planning. \
         It will not obtain new information or change anything, but will log your thought process. \
         Use it when complex reasoning, planning, or reviewing previous tool results is needed."
    }

    /// Get the parameters schema
    pub fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Your reasoning, analysis, or planning thoughts"
                }
            },
            "required": ["thought"]
        })
    }
}

impl Default for ThinkTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for ThinkTool {
    fn name(&self) -> &str {
        ThinkTool::name(self)
    }

    fn description(&self) -> &str {
        ThinkTool::description(self)
    }

    fn parameters_schema(&self) -> Value {
        ThinkTool::parameters_schema(self)
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Parse input
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "think tool requires 'thought' parameter".to_string(),
        })?;

        let input: ThinkInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParameters {
                message: format!("Failed to parse think input: {}", e),
            })?;

        // Log thought (will appear in agent logs)
        tracing::info!(target: "agent::think", thought = %input.thought, "Agent thinking");

        // Return success with acknowledgment
        Ok(ToolResult::success(json!({
            "status": "thought_recorded",
            "message": "Thought logged successfully"
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_think_tool_creation() {
        let tool = ThinkTool::new();
        assert_eq!(tool.name(), "think");
        assert!(tool.description().contains("complex reasoning"));
    }

    #[test]
    fn test_think_tool_schema() {
        let tool = ThinkTool::new();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["thought"].is_object());
        assert_eq!(schema["required"][0], "thought");
    }

    #[tokio::test]
    async fn test_think_tool_execution() {
        let tool = ThinkTool::new();
        let input = json!({
            "thought": "I need to analyze the user's request for EC2 instances across multiple regions"
        });

        let result = tool.execute(Some(input), None).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content["status"], "thought_recorded");
    }

    #[tokio::test]
    async fn test_think_tool_missing_parameter() {
        let tool = ThinkTool::new();
        let input = json!({});

        let result = tool.execute(Some(input), None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ToolError::InvalidParameters { .. }
        ));
    }

    #[tokio::test]
    async fn test_think_tool_no_parameters() {
        let tool = ThinkTool::new();
        let result = tool.execute(None, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_think_tool_long_thought() {
        let tool = ThinkTool::new();
        let long_thought = "A".repeat(10000); // 10KB thought
        let input = json!({ "thought": long_thought });

        let result = tool.execute(Some(input), None).await.unwrap();
        assert!(result.success);
    }
}
