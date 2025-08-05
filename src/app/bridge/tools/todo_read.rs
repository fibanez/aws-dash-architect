//! TodoRead Tool for Bridge Agent Task Querying
//!
//! This tool allows the Bridge Agent to query current task status and progress,
//! enabling intelligent decision-making about next steps in complex workflows.

use async_trait::async_trait;
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{debug, info, warn};

use super::todo_write::{TodoItem, TodoPriority, TodoStatus};

/// TodoRead tool for querying task status and progress
#[derive(Clone)]
pub struct TodoReadTool {
    /// Shared reference to the same task storage used by TodoWrite
    task_storage: Arc<Mutex<HashMap<String, Vec<TodoItem>>>>,
}

impl std::fmt::Debug for TodoReadTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TodoReadTool")
            .field("task_storage", &"<HashMap<String, Vec<TodoItem>>>")
            .finish()
    }
}

impl TodoReadTool {
    pub fn new() -> Self {
        Self {
            task_storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create TodoRead tool with shared storage from TodoWrite
    pub fn with_shared_storage(storage: Arc<Mutex<HashMap<String, Vec<TodoItem>>>>) -> Self {
        Self {
            task_storage: storage,
        }
    }

    /// Get session ID from context or generate default
    fn get_session_id(&self, agent_context: Option<&stood::agent::AgentContext>) -> String {
        agent_context
            .map(|ctx| ctx.agent_id.clone())
            .unwrap_or_else(|| "default-session".to_string())
    }

    /// Filter todos based on criteria
    fn apply_filters(&self, todos: &[TodoItem], filters: &serde_json::Value) -> Vec<TodoItem> {
        let mut filtered = todos.to_vec();

        // Filter by status
        if let Some(status_str) = filters.get("status").and_then(|v| v.as_str()) {
            if let Ok(status) = serde_json::from_str::<TodoStatus>(&format!("\"{}\"", status_str)) {
                filtered.retain(|todo| todo.status == status);
            }
        }

        // Filter by priority
        if let Some(priority_str) = filters.get("priority").and_then(|v| v.as_str()) {
            if let Ok(priority) =
                serde_json::from_str::<TodoPriority>(&format!("\"{}\"", priority_str))
            {
                filtered.retain(|todo| todo.priority == priority);
            }
        }

        // Filter by content (case-insensitive substring match)
        if let Some(search_term) = filters.get("content_contains").and_then(|v| v.as_str()) {
            let search_lower = search_term.to_lowercase();
            filtered.retain(|todo| todo.content.to_lowercase().contains(&search_lower));
        }

        // Filter by ID (exact match)
        if let Some(todo_id) = filters.get("id").and_then(|v| v.as_str()) {
            filtered.retain(|todo| todo.id == todo_id);
        }

        filtered
    }

    /// Generate summary statistics for todos
    fn generate_summary(&self, todos: &[TodoItem]) -> serde_json::Value {
        let total = todos.len();
        let pending = todos
            .iter()
            .filter(|t| t.status == TodoStatus::Pending)
            .count();
        let in_progress = todos
            .iter()
            .filter(|t| t.status == TodoStatus::InProgress)
            .count();
        let completed = todos
            .iter()
            .filter(|t| t.status == TodoStatus::Completed)
            .count();

        let high_priority = todos
            .iter()
            .filter(|t| t.priority == TodoPriority::High)
            .count();
        let medium_priority = todos
            .iter()
            .filter(|t| t.priority == TodoPriority::Medium)
            .count();
        let low_priority = todos
            .iter()
            .filter(|t| t.priority == TodoPriority::Low)
            .count();

        let completion_rate = if total > 0 {
            (completed as f64 / total as f64 * 100.0).round() as u32
        } else {
            0
        };

        serde_json::json!({
            "total_todos": total,
            "by_status": {
                "pending": pending,
                "in_progress": in_progress,
                "completed": completed
            },
            "by_priority": {
                "high": high_priority,
                "medium": medium_priority,
                "low": low_priority
            },
            "completion_rate_percent": completion_rate,
            "has_active_tasks": pending > 0 || in_progress > 0
        })
    }
}

#[async_trait]
impl Tool for TodoReadTool {
    fn name(&self) -> &str {
        "todo_read"
    }

    fn description(&self) -> &str {
        "Use this tool to read the current to-do list for the session. This tool should be used proactively and frequently to ensure that you are aware of the status of the current task list. You should make use of this tool as often as possible, especially in the following situations:\n- At the beginning of conversations to see what's pending\n- Before starting new tasks to prioritize work\n- When the user asks about previous tasks or plans\n- Whenever you're uncertain about what to do next\n- After completing tasks to update your understanding of remaining work\n- After every few messages to ensure you're on track\n\nUsage:\n- This tool takes in no parameters. So leave the input blank or empty. DO NOT include a dummy object, placeholder string or a key like 'input' or 'empty'. LEAVE IT BLANK.\n- Returns a list of todo items with their status, priority, and content\n- Use this information to track progress and plan next steps\n- If no todos exist yet, an empty list will be returned"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
            },
            "additionalProperties": false
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        info!("📖 Executing TodoRead tool");

        let params = parameters.unwrap_or_default();
        let session_id = self.get_session_id(agent_context);

        // Get parameters
        let filters = params.get("filters").cloned().unwrap_or_default();
        let include_summary = params
            .get("include_summary")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(50)
            .min(100) as usize;

        // Access task storage
        let storage = self.task_storage.lock().map_err(|e| {
            warn!("Failed to lock task storage: {}", e);
            ToolError::ExecutionFailed {
                message: "Failed to access task storage".to_string(),
            }
        })?;

        let todos = storage.get(&session_id).cloned().unwrap_or_default();
        debug!("📖 Found {} todos for session: {}", todos.len(), session_id);

        // Apply filters
        let filtered_todos = self.apply_filters(&todos, &filters);
        debug!("📖 After filtering: {} todos", filtered_todos.len());

        // Apply limit
        let limited_todos: Vec<TodoItem> = filtered_todos.into_iter().take(limit).collect();

        // Convert todos to JSON representation
        let todos_json: Vec<serde_json::Value> = limited_todos
            .iter()
            .map(|todo| {
                serde_json::json!({
                    "id": todo.id,
                    "content": todo.content,
                    "status": todo.status,
                    "priority": todo.priority,
                    "created_at": todo.created_at.to_rfc3339(),
                    "updated_at": todo.updated_at.to_rfc3339(),
                    "completed_at": todo.completed_at.map(|dt| dt.to_rfc3339())
                })
            })
            .collect();

        // Build response
        let mut response = serde_json::json!({
            "success": true,
            "session_id": session_id,
            "todos_count": limited_todos.len(),
            "todos": todos_json
        });

        // Add summary if requested
        if include_summary {
            let summary = self.generate_summary(&todos); // Use all todos for summary, not just filtered
            response["summary"] = summary;
        }

        // Add filter info if filters were applied
        if !filters.is_null() && filters.as_object().is_some_and(|obj| !obj.is_empty()) {
            response["applied_filters"] = filters;
            response["total_before_filtering"] = serde_json::Value::Number(todos.len().into());
        }

        info!(
            "✅ TodoRead completed: {} todos returned (filtered from {})",
            limited_todos.len(),
            todos.len()
        );

        Ok(ToolResult::success(response))
    }
}

impl Default for TodoReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::bridge::tools::todo_write::{TodoItem, TodoPriority, TodoStatus};

    fn create_test_todos() -> Vec<TodoItem> {
        vec![
            TodoItem::new("High priority pending task".to_string(), TodoPriority::High),
            TodoItem::new(
                "Medium priority completed task".to_string(),
                TodoPriority::Medium,
            )
            .with_status(TodoStatus::Completed),
            TodoItem::new(
                "Low priority in-progress task".to_string(),
                TodoPriority::Low,
            )
            .with_status(TodoStatus::InProgress),
        ]
    }

    #[tokio::test]
    async fn test_todo_read_all() {
        let tool = TodoReadTool::new();

        // Populate storage
        {
            let mut storage = tool.task_storage.lock().unwrap();
            storage.insert("default-session".to_string(), create_test_todos());
        }

        let result = tool.execute(None, None).await.unwrap();

        assert!(result.success);
        let response = result.content;
        assert_eq!(response["todos_count"], 3);
        assert!(response["summary"].is_object());
    }

    #[tokio::test]
    async fn test_todo_read_with_status_filter() {
        let tool = TodoReadTool::new();

        // Populate storage
        {
            let mut storage = tool.task_storage.lock().unwrap();
            storage.insert("default-session".to_string(), create_test_todos());
        }

        let params = serde_json::json!({
            "filters": {
                "status": "completed"
            }
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(result.success);
        let response = result.content;
        assert_eq!(response["todos_count"], 1);
        assert_eq!(response["total_before_filtering"], 3);
    }

    #[tokio::test]
    async fn test_todo_read_with_priority_filter() {
        let tool = TodoReadTool::new();

        // Populate storage
        {
            let mut storage = tool.task_storage.lock().unwrap();
            storage.insert("default-session".to_string(), create_test_todos());
        }

        let params = serde_json::json!({
            "filters": {
                "priority": "high"
            }
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(result.success);
        let response = result.content;
        assert_eq!(response["todos_count"], 1);
    }

    #[tokio::test]
    async fn test_todo_read_with_content_filter() {
        let tool = TodoReadTool::new();

        // Populate storage
        {
            let mut storage = tool.task_storage.lock().unwrap();
            storage.insert("default-session".to_string(), create_test_todos());
        }

        let params = serde_json::json!({
            "filters": {
                "content_contains": "progress"
            }
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(result.success);
        let response = result.content;
        assert_eq!(response["todos_count"], 1);
    }

    #[test]
    fn test_summary_generation() {
        let tool = TodoReadTool::new();
        let todos = create_test_todos();

        let summary = tool.generate_summary(&todos);

        assert_eq!(summary["total_todos"], 3);
        assert_eq!(summary["by_status"]["pending"], 1);
        assert_eq!(summary["by_status"]["completed"], 1);
        assert_eq!(summary["by_status"]["in_progress"], 1);
        assert_eq!(summary["completion_rate_percent"], 33); // 1/3 * 100 rounded
        assert_eq!(summary["has_active_tasks"], true);
    }
}

