//! TodoWrite Tool for Bridge Agent Task Management
//!
//! This tool allows the Bridge Agent to proactively track multi-step AWS tasks,
//! providing visibility into complex operations and ensuring nothing is forgotten.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Priority levels for todo items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TodoPriority {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}

impl Default for TodoPriority {
    fn default() -> Self {
        TodoPriority::Medium
    }
}

/// Status of todo items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TodoStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "completed")]
    Completed,
}

impl Default for TodoStatus {
    fn default() -> Self {
        TodoStatus::Pending
    }
}

/// Individual todo item with comprehensive tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl TodoItem {
    pub fn new(content: String, priority: TodoPriority) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            status: TodoStatus::Pending,
            priority,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    pub fn with_status(mut self, status: TodoStatus) -> Self {
        self.status = status.clone();
        self.updated_at = Utc::now();
        if status == TodoStatus::Completed {
            self.completed_at = Some(Utc::now());
        }
        self
    }
}

/// TodoWrite tool for proactive task management
#[derive(Clone)]
pub struct TodoWriteTool {
    /// In-memory task storage - keyed by session/agent ID
    task_storage: Arc<Mutex<HashMap<String, Vec<TodoItem>>>>,
}

impl std::fmt::Debug for TodoWriteTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TodoWriteTool")
            .field("task_storage", &"<HashMap<String, Vec<TodoItem>>>")
            .finish()
    }
}

impl TodoWriteTool {
    pub fn new() -> Self {
        Self {
            task_storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get session ID from context or generate default
    fn get_session_id(&self, agent_context: Option<&stood::agent::AgentContext>) -> String {
        agent_context
            .map(|ctx| ctx.agent_id.clone())
            .unwrap_or_else(|| "default-session".to_string())
    }

    /// Update existing todo item
    fn update_todo_item(
        &self,
        session_id: &str,
        todo_id: &str,
        updates: &serde_json::Value,
    ) -> Result<bool, ToolError> {
        let mut storage = self.task_storage.lock().map_err(|e| {
            warn!("Failed to lock task storage: {}", e);
            ToolError::ExecutionFailed {
                message: "Failed to access task storage".to_string(),
            }
        })?;

        let todos = storage.entry(session_id.to_string()).or_insert_with(Vec::new);
        
        if let Some(todo) = todos.iter_mut().find(|t| t.id == todo_id) {
            // Update fields if provided
            if let Some(content) = updates.get("content").and_then(|v| v.as_str()) {
                todo.content = content.to_string();
            }
            
            if let Some(status_str) = updates.get("status").and_then(|v| v.as_str()) {
                if let Ok(status) = serde_json::from_str::<TodoStatus>(&format!("\"{}\"", status_str)) {
                    todo.status = status.clone();
                    if status.clone() == TodoStatus::Completed {
                        todo.completed_at = Some(Utc::now());
                    }
                }
            }
            
            if let Some(priority_str) = updates.get("priority").and_then(|v| v.as_str()) {
                if let Ok(priority) = serde_json::from_str::<TodoPriority>(&format!("\"{}\"", priority_str)) {
                    todo.priority = priority;
                }
            }
            
            todo.updated_at = Utc::now();
            return Ok(true);
        }
        
        Ok(false)
    }
}

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }

    fn description(&self) -> &str {
        "Write and manage todo items for tracking multi-step AWS tasks. \
         IMPORTANT: Use this tool proactively to break down complex operations \
         and provide visibility into task progress. Essential for user experience."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "Array of todo items to create or update",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Todo ID for updates (omit for new todos)"
                            },
                            "content": {
                                "type": "string",
                                "description": "Todo item description"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed"],
                                "description": "Todo status"
                            },
                            "priority": {
                                "type": "string", 
                                "enum": ["high", "medium", "low"],
                                "description": "Todo priority level"
                            }
                        },
                        "required": ["content"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        info!("📝 Executing TodoWrite tool");

        let params = parameters.unwrap_or_default();
        let session_id = self.get_session_id(agent_context);

        let todos_array = params
            .get("todos")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                warn!("❌ todos parameter is required and must be an array");
                ToolError::InvalidParameters {
                    message: "todos parameter is required and must be an array of todo items".to_string(),
                }
            })?;

        if todos_array.is_empty() {
            warn!("❌ Empty todos array provided");
            return Err(ToolError::InvalidParameters {
                message: "At least one todo item must be provided".to_string(),
            });
        }

        let mut created_count = 0;
        let mut updated_count = 0;
        let mut todo_results = Vec::new();

        // Process each todo item
        for todo_value in todos_array {
            let todo_obj = todo_value.as_object().ok_or_else(|| {
                ToolError::InvalidParameters {
                    message: "Each todo item must be an object".to_string(),
                }
            })?;

            let content = todo_obj
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolError::InvalidParameters {
                        message: "Each todo item must have a 'content' field".to_string(),
                    }
                })?;

            // Check if this is an update (has ID) or new todo
            if let Some(todo_id) = todo_obj.get("id").and_then(|v| v.as_str()) {
                // Update existing todo
                if self.update_todo_item(&session_id, todo_id, todo_value)? {
                    updated_count += 1;
                    todo_results.push(serde_json::json!({
                        "id": todo_id,
                        "action": "updated",
                        "content": content
                    }));
                    debug!("✅ Updated todo item: {}", content);
                } else {
                    warn!("❌ Todo item not found for update: {}", todo_id);
                    return Err(ToolError::InvalidParameters {
                        message: format!("Todo item with ID {} not found", todo_id),
                    });
                }
            } else {
                // Create new todo
                let priority = todo_obj
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .and_then(|s| serde_json::from_str::<TodoPriority>(&format!("\"{}\"", s)).ok())
                    .unwrap_or(TodoPriority::Medium);

                let status = todo_obj
                    .get("status")
                    .and_then(|v| v.as_str())
                    .and_then(|s| serde_json::from_str::<TodoStatus>(&format!("\"{}\"", s)).ok())
                    .unwrap_or(TodoStatus::Pending);

                let mut todo_item = TodoItem::new(content.to_string(), priority);
                todo_item.status = status;

                // Store the todo
                {
                    let mut storage = self.task_storage.lock().map_err(|e| {
                        warn!("Failed to lock task storage: {}", e);
                        ToolError::ExecutionFailed {
                            message: "Failed to access task storage".to_string(),
                        }
                    })?;

                    let todos = storage.entry(session_id.clone()).or_insert_with(Vec::new);
                    todos.push(todo_item.clone());
                }

                created_count += 1;
                todo_results.push(serde_json::json!({
                    "id": todo_item.id,
                    "action": "created",
                    "content": content,
                    "status": todo_item.status,
                    "priority": todo_item.priority
                }));
                debug!("✅ Created todo item: {}", content);
            }
        }

        info!(
            "✅ TodoWrite completed: {} created, {} updated",
            created_count, updated_count
        );

        let response = serde_json::json!({
            "success": true,
            "session_id": session_id,
            "created_count": created_count,
            "updated_count": updated_count,
            "total_processed": created_count + updated_count,
            "todos": todo_results
        });

        Ok(ToolResult::success(response))
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_todo_write_creation() {
        let tool = TodoWriteTool::new();
        
        let params = serde_json::json!({
            "todos": [
                {
                    "content": "Test AWS account discovery",
                    "priority": "high"
                },
                {
                    "content": "Create log analyzer agent",
                    "priority": "medium", 
                    "status": "in_progress"
                }
            ]
        });

        let result = tool.execute(Some(params), None).await.unwrap();
        
        assert!(result.success);
        let response = result.data.unwrap();
        assert_eq!(response["created_count"], 2);
        assert_eq!(response["updated_count"], 0);
    }

    #[tokio::test]
    async fn test_todo_write_update() {
        let tool = TodoWriteTool::new();
        
        // First create a todo
        let create_params = serde_json::json!({
            "todos": [
                {
                    "content": "Initial task",
                    "priority": "high"
                }
            ]
        });

        let create_result = tool.execute(Some(create_params), None).await.unwrap();
        let created_todo_id = create_result.data.unwrap()["todos"][0]["id"].as_str().unwrap();

        // Then update it
        let update_params = serde_json::json!({
            "todos": [
                {
                    "id": created_todo_id,
                    "content": "Updated task content",
                    "status": "completed"
                }
            ]
        });

        let update_result = tool.execute(Some(update_params), None).await.unwrap();
        
        assert!(update_result.success);
        let response = update_result.data.unwrap();
        assert_eq!(response["created_count"], 0);
        assert_eq!(response["updated_count"], 1);
    }

    #[test]
    fn test_todo_item_creation() {
        let todo = TodoItem::new("Test task".to_string(), TodoPriority::High);
        
        assert!(!todo.id.is_empty());
        assert_eq!(todo.content, "Test task");
        assert_eq!(todo.status, TodoStatus::Pending);
        assert_eq!(todo.priority, TodoPriority::High);
        assert!(todo.completed_at.is_none());
    }

    #[test]
    fn test_todo_item_completion() {
        let todo = TodoItem::new("Test task".to_string(), TodoPriority::Medium)
            .with_status(TodoStatus::Completed);
        
        assert_eq!(todo.status, TodoStatus::Completed);
        assert!(todo.completed_at.is_some());
    }
}