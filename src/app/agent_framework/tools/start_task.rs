#![warn(clippy::all, rust_2018_idioms)]

//! Start-Task Tool - Agent Spawning for Task-Manager Agents
//!
//! This tool allows task-manager agents to spawn worker agents for parallel task execution.
//!
//! ## Implementation
//!
//! Uses the agent creation request/response channel to spawn TaskWorker agents.
//! The parent agent ID is retrieved from thread-local storage.

use crate::app::agent_framework::{
    get_current_agent_id, request_agent_creation, wait_for_worker_completion,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use stood::tools::{Tool, ToolError, ToolResult};

/// Start-task tool for spawning worker agents
#[derive(Clone, Debug)]
pub struct StartTaskTool;

/// Input schema for start-task tool
#[derive(Debug, Deserialize, Serialize)]
struct StartTaskInput {
    /// High-level description of WHAT to accomplish
    task_description: String,

    /// Optional description of expected output format
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_output_format: Option<String>,
}

impl StartTaskTool {
    /// Create a new start-task tool instance
    pub fn new() -> Self {
        Self
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        "start_task"
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        "Spawn a worker agent to execute an AWS task using JavaScript APIs.\n\n\
         **CRITICAL**: Include comprehensive context in your task description:\n\
         - Original user request for context\n\
         - Specific task details (WHAT to accomplish, not HOW)\n\
         - Context information from previous completed tasks\n\
         - Objective and how this contributes to the overall goal\n\
         - Expected output format\n\n\
         Workers have access to: listAccounts(), listRegions(), queryResources(), \
         queryCloudWatchLogEvents(), getCloudTrailEvents()\n\n\
         **Good task example**:\n\
         'User asked: \"Find all production EC2 instances with high CPU usage\"\n\
         Task: List all EC2 instances in accounts with \"prod\" in the name.\n\
         Context: This is step 1 of analyzing production infrastructure.\n\
         Expected output: JSON array with instance ID, type, state, launch time'\n\n\
         **Bad task example**:\n\
         'Use queryResources() API to call EC2' (too implementation-focused, lacks context)"
    }

    /// Get the parameters schema
    pub fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["task_description"],
            "properties": {
                "task_description": {
                    "type": "string",
                    "description": "High-level description of WHAT to accomplish (verb + subject + constraints). Do NOT specify implementation details.",
                    "examples": [
                        "List all EC2 instances in the production account",
                        "Find S3 buckets larger than 100GB",
                        "Analyze RDS databases for unused instances in us-east-1"
                    ]
                },
                "expected_output_format": {
                    "type": "string",
                    "description": "Optional description of the expected output format",
                    "examples": [
                        "JSON array of instance objects with id, type, state, tags",
                        "Table with columns: bucket name, size, region",
                        "Summary statistics with total count and breakdown by type"
                    ]
                }
            }
        })
    }
}

impl Default for StartTaskTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for StartTaskTool {
    fn name(&self) -> &str {
        StartTaskTool::name(self)
    }

    fn description(&self) -> &str {
        StartTaskTool::description(self)
    }

    fn parameters_schema(&self) -> Value {
        StartTaskTool::parameters_schema(self)
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Parse input
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "start_task tool requires 'task_description' parameter".to_string(),
        })?;

        let input: StartTaskInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParameters {
                message: format!("Failed to parse start_task input: {}", e),
            })?;

        // Validate task description not empty
        if input.task_description.trim().is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "task_description cannot be empty".to_string(),
            });
        }

        // Get parent agent ID from thread-local context
        let parent_id = get_current_agent_id().ok_or_else(|| ToolError::InvalidParameters {
            message: "Cannot determine parent agent ID - agent context not set".to_string(),
        })?;

        // Log the complete tool call with all parameters for debugging
        tracing::info!(
            target: "agent::start_task",
            parent_id = %parent_id,
            "start_task TOOL CALL:\n  Task Description: {}\n  Expected Output Format: {:?}",
            input.task_description,
            input.expected_output_format
        );

        // Request agent creation via channel
        let agent_id = request_agent_creation(
            input.task_description.clone(),
            input.expected_output_format.clone(),
            parent_id,
        )
        .map_err(|e| ToolError::InvalidParameters {
            message: format!("Failed to create task-agent: {}", e),
        })?;

        tracing::info!(
            target: "agent::start_task",
            parent_id = %parent_id,
            agent_id = %agent_id,
            "TaskWorker agent created, waiting for completion"
        );

        // NOTE: We do NOT send SwitchToAgent event - worker tabs are created
        // but not focused. User can manually switch to worker tab if desired.

        // Note: StatusUpdate messages could be sent here in the future to show
        // "Starting Task" -> "Task Completed" -> "Processing" status changes,
        // but this requires deeper integration with the stood library's response channel.

        // Wait for worker to complete (5 minute timeout)
        let start_time = std::time::Instant::now();
        tracing::info!(
            target: "agent::start_task",
            parent_id = %parent_id,
            agent_id = %agent_id,
            "Waiting for worker to complete (status: Starting Task)"
        );

        match wait_for_worker_completion(agent_id, Duration::from_secs(300)) {
            Ok(result) => {
                let execution_time_ms = start_time.elapsed().as_millis();
                tracing::info!(
                    target: "agent::start_task",
                    parent_id = %parent_id,
                    agent_id = %agent_id,
                    execution_time_ms = execution_time_ms,
                    "TaskWorker completed successfully"
                );

                // Return raw result directly (no wrapper text)
                Ok(ToolResult::success(json!({
                    "result": result,
                    "execution_time_ms": execution_time_ms,
                })))
            }
            Err(error) => {
                tracing::error!(
                    target: "agent::start_task",
                    parent_id = %parent_id,
                    agent_id = %agent_id,
                    error = %error,
                    "TaskWorker failed"
                );

                // Return raw error directly (no wrapper text)
                Ok(ToolResult::error(&error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_task_tool_creation() {
        let tool = StartTaskTool::new();
        assert_eq!(tool.name(), "start_task");
        assert!(tool.description().contains("Spawn"));
        assert!(tool.description().contains("context"));
    }

    #[test]
    fn test_start_task_tool_schema() {
        let tool = StartTaskTool::new();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("task_description")));
        assert!(schema["properties"]["task_description"].is_object());
        assert!(schema["properties"]["expected_output_format"].is_object());
    }

    #[tokio::test]
    async fn test_start_task_with_task_only() {
        use crate::app::agent_framework::{init_agent_creation_channel, init_ui_event_channel};
        use crate::app::agent_framework::{set_current_agent_id, AgentId};

        // Initialize channels
        init_agent_creation_channel();
        init_ui_event_channel();

        // Set up agent context
        let parent_id = AgentId::new();
        set_current_agent_id(parent_id);

        let tool = StartTaskTool::new();
        let input = json!({
            "task_description": "List all EC2 instances in production"
        });

        // Note: This will fail because AgentManagerWindow isn't running to process the request
        // For unit tests, we just verify the tool can be called without panicking
        let result = tool.execute(Some(input), None).await;

        // Expect a timeout error since no one is processing the channel
        assert!(result.is_err());
        if let Err(ToolError::InvalidParameters { message }) = result {
            assert!(message.contains("Failed to create task-agent"));
        }
    }

    #[tokio::test]
    async fn test_start_task_with_format() {
        use crate::app::agent_framework::{init_agent_creation_channel, init_ui_event_channel};
        use crate::app::agent_framework::{set_current_agent_id, AgentId};

        // Initialize channels
        init_agent_creation_channel();
        init_ui_event_channel();

        // Set up agent context
        let parent_id = AgentId::new();
        set_current_agent_id(parent_id);

        let tool = StartTaskTool::new();
        let input = json!({
            "task_description": "Find large S3 buckets",
            "expected_output_format": "JSON array with bucket name and size"
        });

        // Expect timeout error since no one is processing the channel
        let result = tool.execute(Some(input), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_task_empty_description_rejected() {
        let tool = StartTaskTool::new();
        let input = json!({ "task_description": "" });

        // Empty description is caught before agent context is checked
        let result = tool.execute(Some(input), None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ToolError::InvalidParameters { .. }
        ));
    }

    #[tokio::test]
    async fn test_start_task_missing_parameter() {
        let tool = StartTaskTool::new();
        let input = json!({});

        // Missing parameter is caught before agent context is checked
        let result = tool.execute(Some(input), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_task_no_agent_context() {
        use crate::app::agent_framework::clear_current_agent_id;

        // Ensure no agent context is set
        clear_current_agent_id();

        let tool = StartTaskTool::new();
        let input = json!({
            "task_description": "Test task"
        });

        // Should fail with invalid parameters error about missing agent context
        let result = tool.execute(Some(input), None).await;
        assert!(result.is_err());
        if let Err(ToolError::InvalidParameters { message }) = result {
            assert!(message.contains("agent context not set"));
        }
    }
}
