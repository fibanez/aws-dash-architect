#![warn(clippy::all, rust_2018_idioms)]

//! Todo-Read Tool - Task List Retrieval for Task-Manager Agents
//!
//! This tool allows task-manager agents to retrieve their current task list.
//! It follows Claude Code's proven schema exactly.
//!
//! ## Usage
//!
//! The agent uses this tool to:
//! - Check if tasks exist from previous session (conversation start)
//! - Understand current state before creating new tasks
//! - Verify progress after task completion
//! - Respond to user status queries

use serde_json::{json, Value};
use stood::tools::{Tool, ToolError, ToolResult};

use super::types::TodoItem;

/// Todo-read tool for retrieving task lists
#[derive(Clone)]
pub struct TodoReadTool {
    /// Callback to read agent's todo list
    /// This will be provided by AgentInstance at runtime
    read_callback: Option<std::sync::Arc<dyn Fn() -> Vec<TodoItem> + Send + Sync>>,
}

impl std::fmt::Debug for TodoReadTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TodoReadTool")
            .field("read_callback", &self.read_callback.is_some())
            .finish()
    }
}

impl TodoReadTool {
    /// Create a new todo-read tool instance
    pub fn new() -> Self {
        Self {
            read_callback: None,
        }
    }

    /// Set the read callback for retrieving todos
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Vec<TodoItem> + Send + Sync + 'static,
    {
        self.read_callback = Some(std::sync::Arc::new(callback));
        self
    }

    /// Get the tool name
    pub fn name(&self) -> &str {
        "todo_read"
    }

    /// Get the tool description
    pub fn description(&self) -> &str {
        "Retrieve the current todo list. Takes no parameters - leave input blank or use empty object {}."
    }

    /// Get the parameters schema
    pub fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    /// Calculate summary statistics for todo list
    fn calculate_summary(todos: &[TodoItem]) -> Value {
        let pending = todos.iter().filter(|t| t.is_pending()).count();
        let in_progress = todos.iter().filter(|t| t.is_in_progress()).count();
        let completed = todos.iter().filter(|t| t.is_completed()).count();

        json!({
            "total": todos.len(),
            "pending": pending,
            "in_progress": in_progress,
            "completed": completed
        })
    }
}

impl Default for TodoReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for TodoReadTool {
    fn name(&self) -> &str {
        TodoReadTool::name(self)
    }

    fn description(&self) -> &str {
        TodoReadTool::description(self)
    }

    fn parameters_schema(&self) -> Value {
        TodoReadTool::parameters_schema(self)
    }

    async fn execute(
        &self,
        _parameters: Option<Value>,
        _context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Retrieve todos via callback
        let todos = if let Some(ref callback) = self.read_callback {
            callback()
        } else {
            Vec::new() // Empty list if no callback
        };

        // Calculate summary
        let summary = Self::calculate_summary(&todos);

        // Log read
        tracing::debug!(
            target: "agent::todo_read",
            task_count = todos.len(),
            "Todo list retrieved"
        );

        // Return todos with summary
        Ok(ToolResult::success(json!({
            "todos": todos,
            "summary": summary
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::TodoStatus;

    #[test]
    fn test_todo_read_tool_creation() {
        let tool = TodoReadTool::new();
        assert_eq!(tool.name(), "todo_read");
        assert!(tool.description().contains("todo list"));
    }

    #[test]
    fn test_todo_read_tool_schema() {
        let tool = TodoReadTool::new();
        let schema = tool.parameters_schema();

        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].as_object().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_todo_read_empty_list() {
        let tool = TodoReadTool::new().with_callback(|| Vec::new());

        let result = tool.execute(None, None).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content["todos"].as_array().unwrap().len(), 0);
        assert_eq!(result.content["summary"]["total"], 0);
    }

    #[tokio::test]
    async fn test_todo_read_with_todos() {
        let todos = vec![
            TodoItem::new(
                "Task 1".to_string(),
                "Doing task 1".to_string(),
                TodoStatus::Completed,
            ),
            TodoItem::new(
                "Task 2".to_string(),
                "Doing task 2".to_string(),
                TodoStatus::InProgress,
            ),
            TodoItem::new(
                "Task 3".to_string(),
                "Doing task 3".to_string(),
                TodoStatus::Pending,
            ),
        ];

        let todos_clone = todos.clone();
        let tool = TodoReadTool::new().with_callback(move || todos_clone.clone());

        let result = tool.execute(None, None).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content["todos"].as_array().unwrap().len(), 3);
        assert_eq!(result.content["summary"]["total"], 3);
        assert_eq!(result.content["summary"]["pending"], 1);
        assert_eq!(result.content["summary"]["in_progress"], 1);
        assert_eq!(result.content["summary"]["completed"], 1);
    }

    #[tokio::test]
    async fn test_todo_read_with_parameters_ignored() {
        let tool = TodoReadTool::new().with_callback(|| Vec::new());
        let input = json!({ "ignored": "parameter" });

        let result = tool.execute(Some(input), None).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_todo_read_summary_calculation() {
        let todos = vec![
            TodoItem::new("T1".into(), "D1".into(), TodoStatus::Completed),
            TodoItem::new("T2".into(), "D2".into(), TodoStatus::Completed),
            TodoItem::new("T3".into(), "D3".into(), TodoStatus::InProgress),
            TodoItem::new("T4".into(), "D4".into(), TodoStatus::Pending),
            TodoItem::new("T5".into(), "D5".into(), TodoStatus::Pending),
            TodoItem::new("T6".into(), "D6".into(), TodoStatus::Pending),
        ];

        let summary = TodoReadTool::calculate_summary(&todos);
        assert_eq!(summary["total"], 6);
        assert_eq!(summary["pending"], 3);
        assert_eq!(summary["in_progress"], 1);
        assert_eq!(summary["completed"], 2);
    }
}
