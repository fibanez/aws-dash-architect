#![warn(clippy::all, rust_2018_idioms)]

//! Todo-Write Tool - Task List Management for Task-Manager Agents
//!
//! This tool allows task-manager agents to create and update their task lists.
//! It follows Claude Code's proven schema and validation rules exactly.
//!
//! ## Rules
//!
//! 1. **One in-progress**: Only ONE task with status "in_progress" at a time
//! 2. **Immediate completion**: Mark task "completed" as soon as finished
//! 3. **Full completion only**: Don't mark complete if errors/blockers exist
//! 4. **Two forms required**: Both imperative (content) and continuous (activeForm)
//!
//! ## Usage
//!
//! The agent uses this tool to:
//! - Create initial task list after analyzing user request
//! - Update task status when spawning task-agents (mark "in_progress")
//! - Mark tasks as "completed" when task-agents finish
//! - Decide on retry strategy when task-agents fail

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use stood::tools::{Tool, ToolError, ToolResult};

use super::types::TodoItem;

/// Todo-write tool for managing task lists
#[derive(Clone)]
pub struct TodoWriteTool {
    /// Callback to update agent's todo list
    /// This will be provided by AgentInstance at runtime
    update_callback: Option<std::sync::Arc<dyn Fn(Vec<TodoItem>) + Send + Sync>>,
}

impl std::fmt::Debug for TodoWriteTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TodoWriteTool")
            .field("update_callback", &self.update_callback.is_some())
            .finish()
    }
}

/// Input schema for todo-write tool
#[derive(Debug, Deserialize, Serialize)]
struct TodoWriteInput {
    /// The complete updated todo list
    todos: Vec<TodoItem>,
}

impl TodoWriteTool {
    /// Create a new todo-write tool instance
    pub fn new() -> Self {
        Self {
            update_callback: None,
        }
    }

    /// Set the update callback for storing todos
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(Vec<TodoItem>) + Send + Sync + 'static,
    {
        self.update_callback = Some(std::sync::Arc::new(callback));
        self
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        "todo_write"
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        "Create and manage your task list. Use this to track tasks you plan to execute. \
         Always update the entire todo list (not individual items). \
         Limit ONE task to 'in_progress' at a time."
    }

    /// Get the parameters schema
    pub fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["todos"],
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "The complete updated todo list",
                    "items": {
                        "type": "object",
                        "required": ["content", "status", "activeForm"],
                        "properties": {
                            "content": {
                                "type": "string",
                                "minLength": 1,
                                "description": "Imperative form: what needs to be done (e.g., 'List EC2 instances')"
                            },
                            "activeForm": {
                                "type": "string",
                                "minLength": 1,
                                "description": "Present continuous form: what's being done (e.g., 'Listing EC2 instances')"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed"],
                                "description": "Current task status"
                            }
                        }
                    }
                }
            }
        })
    }

    /// Validate that only one task is in_progress
    fn validate_single_in_progress(todos: &[TodoItem]) -> Result<(), String> {
        let in_progress_count = todos.iter().filter(|t| t.is_in_progress()).count();

        if in_progress_count > 1 {
            return Err(format!(
                "Only one task should be 'in_progress' at a time, found {}",
                in_progress_count
            ));
        }

        Ok(())
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        TodoWriteTool::name(self)
    }

    fn description(&self) -> &str {
        TodoWriteTool::description(self)
    }

    fn parameters_schema(&self) -> Value {
        TodoWriteTool::parameters_schema(self)
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Parse input
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "todo_write tool requires 'todos' parameter".to_string(),
        })?;

        let input: TodoWriteInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidParameters {
                message: format!("Failed to parse todo_write input: {}", e),
            })?;

        // Validate: only one in_progress
        Self::validate_single_in_progress(&input.todos)
            .map_err(|e| ToolError::InvalidParameters { message: e })?;

        // Update via callback if available
        if let Some(ref callback) = self.update_callback {
            callback(input.todos.clone());
        }

        // Log update
        tracing::info!(
            target: "agent::todo_write",
            task_count = input.todos.len(),
            "Todo list updated"
        );

        // Return success
        Ok(ToolResult::success(json!({
            "status": "updated",
            "task_count": input.todos.len()
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_write_tool_creation() {
        let tool = TodoWriteTool::new();
        assert_eq!(tool.name(), "todo_write");
        assert!(tool.description().contains("task list"));
    }

    #[test]
    fn test_todo_write_tool_schema() {
        let tool = TodoWriteTool::new();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("todos")));
        assert_eq!(schema["properties"]["todos"]["type"], "array");
    }

    #[tokio::test]
    async fn test_todo_write_valid_list() {
        let tool = TodoWriteTool::new();
        let input = json!({
            "todos": [
                {
                    "content": "Task 1",
                    "activeForm": "Doing task 1",
                    "status": "completed"
                },
                {
                    "content": "Task 2",
                    "activeForm": "Doing task 2",
                    "status": "in_progress"
                },
                {
                    "content": "Task 3",
                    "activeForm": "Doing task 3",
                    "status": "pending"
                }
            ]
        });

        let result = tool.execute(Some(input), None).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content["task_count"], 3);
    }

    #[tokio::test]
    async fn test_todo_write_multiple_in_progress_rejected() {
        let tool = TodoWriteTool::new();
        let input = json!({
            "todos": [
                {
                    "content": "Task 1",
                    "activeForm": "Doing task 1",
                    "status": "in_progress"
                },
                {
                    "content": "Task 2",
                    "activeForm": "Doing task 2",
                    "status": "in_progress"
                }
            ]
        });

        let result = tool.execute(Some(input), None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::InvalidParameters { .. }));
    }

    #[tokio::test]
    async fn test_todo_write_empty_list() {
        let tool = TodoWriteTool::new();
        let input = json!({ "todos": [] });

        let result = tool.execute(Some(input), None).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content["task_count"], 0);
    }

    #[tokio::test]
    async fn test_todo_write_callback_invoked() {
        use std::sync::{Arc, Mutex};

        let captured_todos = Arc::new(Mutex::new(Vec::new()));
        let captured_todos_clone = Arc::clone(&captured_todos);

        let tool = TodoWriteTool::new().with_callback(move |todos| {
            *captured_todos_clone.lock().unwrap() = todos;
        });

        let input = json!({
            "todos": [{
                "content": "Test task",
                "activeForm": "Testing task",
                "status": "pending"
            }]
        });

        let result = tool.execute(Some(input), None).await.unwrap();
        assert!(result.success);

        let todos = captured_todos.lock().unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].content, "Test task");
    }
}
