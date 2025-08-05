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

    /// Create TodoWrite tool with shared storage for multi-agent TODO sharing
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

        let todos = storage
            .entry(session_id.to_string())
            .or_insert_with(Vec::new);

        if let Some(todo) = todos.iter_mut().find(|t| t.id == todo_id) {
            // Update fields if provided
            if let Some(content) = updates.get("content").and_then(|v| v.as_str()) {
                todo.content = content.to_string();
            }

            if let Some(status_str) = updates.get("status").and_then(|v| v.as_str()) {
                if let Ok(status) =
                    serde_json::from_str::<TodoStatus>(&format!("\"{}\"", status_str))
                {
                    todo.status = status.clone();
                    if status.clone() == TodoStatus::Completed {
                        todo.completed_at = Some(Utc::now());
                    }
                }
            }

            if let Some(priority_str) = updates.get("priority").and_then(|v| v.as_str()) {
                if let Ok(priority) =
                    serde_json::from_str::<TodoPriority>(&format!("\"{}\"", priority_str))
                {
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
        "

Use this tool to create and manage a structured task list for your current coding session. This helps you track progress, organize complex tasks, and demonstrate thoroughness to the user.
It also helps the user understand the progress of the task and overall progress of their requests.

## When to Use This Tool
Use this tool proactively in these scenarios:

1. Complex multi-step tasks - When a task requires 3 or more distinct steps or actions
2. Non-trivial and complex tasks - Tasks that require careful planning or multiple operations
3. User explicitly requests todo list - When the user directly asks you to use the todo list
4. User provides multiple tasks - When users provide a list of things to be done (numbered or comma-separated)
5. After receiving new instructions - Immediately capture user requirements as todos
6. When you start working on a task - Mark it as in_progress BEFORE beginning work. Ideally you should only have one todo as in_progress at a time
7. After completing a task - Mark it as completed and add any new follow-up tasks discovered during implementation

## When NOT to Use This Tool

Skip using this tool when:
1. There is only a single, straightforward task
2. The task is trivial and tracking it provides no organizational benefit
3. The task can be completed in less than 3 trivial steps
4. The task is purely conversational or informational

NOTE that you should not use this tool if there is only one trivial task to do. In this case you are better off just doing the task directly.

## Examples of When to Use the Todo List

<example>
User: I want to add a dark mode toggle to the application settings. Make sure you run the tests and build when you're done!
Assistant: I'll help add a dark mode toggle to your application settings. Let me create a todo list to track this implementation.
*Creates todo list with the following items:*
1. Create dark mode toggle component in Settings page
2. Add dark mode state management (context/store)
3. Implement CSS-in-JS styles for dark theme
4. Update existing components to support theme switching
5. Run tests and build process, addressing any failures or errors that occur
*Begins working on the first task*

<reasoning>
The assistant used the todo list because:
1. Adding dark mode is a multi-step feature requiring UI, state management, and styling changes
2. The user explicitly requested tests and build be run afterward
3. The assistant inferred that tests and build need to pass by adding 'Ensure tests and build succeed' as the final task
</reasoning>
</example>

<example>
User: Help me rename the function getCwd to getCurrentWorkingDirectory across my project
Assistant: Let me first search through your codebase to find all occurrences of 'getCwd'.
*Uses grep or search tools to locate all instances of getCwd in the codebase*
Assistant: I've found 15 instances of 'getCwd' across 8 different files. Let me create a todo list to track these changes.
*Creates todo list with specific items for each file that needs updating*

<reasoning>
The assistant used the todo list because:
1. First, the assistant searched to understand the scope of the task
2. Upon finding multiple occurrences across different files, it determined this was a complex task with multiple steps
3. The todo list helps ensure every instance is tracked and updated systematically
4. This approach prevents missing any occurrences and maintains code consistency
</reasoning>
</example>


<example>
User: I need to implement these features for my e-commerce site: user registration, product catalog, shopping cart, and checkout flow.
Assistant: I'll help implement these features. First, let's add all the features to the todo list.
*Creates a todo list breaking down each feature into specific tasks based on the project architecture*
Assistant: Let's start with user registration. This will involve creating database models, API endpoints, and frontend forms that integrate with your existing authentication system.

<reasoning>
The assistant used the todo list because:
1. The user provided multiple complex features to implement in a comma separated list
2. The todo list helps organize these large features into manageable tasks
3. This approach allows for tracking progress across the entire implementation
</reasoning>
</example>

<example>
User: Can you help optimize my React application? It's rendering slowly and has performance issues.</user>
Assistant: I'll help optimize your React application. First, let me examine your codebase to identify potential performance bottlenecks.
*Reviews component structure, render patterns, state management, and data fetching*
Assistant: After analyzing your codebase, I've identified several performance issues. Let me create a todo list to track our optimization efforts.
*Creates todo list with items like: 1) Implement memoization for expensive calculations in ProductList, 2) Add virtualization for long lists in Dashboard, 3) Optimize image loading in Gallery component, 4) Fix state update loops in ShoppingCart, 5) Review bundle size and implement code splitting*
Let's start by implementing memoization for the expensive calculations in your ProductList component.</assistant>

<reasoning>
The assistant used the todo list because:
1. First, the assistant examined the codebase to identify specific performance issues
2. Based on this analysis, it identified multiple optimization opportunities
3. Performance optimization is a non-trivial task requiring multiple steps
4. The todo list helps methodically track improvements across different components
5. This systematic approach ensures all performance bottlenecks are addressed
</reasoning>
</example>

## Examples of When NOT to Use the Todo List

<example>
User: Can you add a comment to the calculateTotal function to explain what it does?
Assistant: Sure, let me add a comment to the calculateTotal function to explain what it does.
* Uses the Edit tool to add a comment to the calculateTotal function *

<reasoning>
The assistant did not use the todo list because this is a single, straightforward task confined to one location in the code. Adding a comment doesn't require tracking multiple steps or systematic organization.
</reasoning>
</example>

<example>
User: Run npm install for me and tell me what happens.
Assistant: I'll run the npm install command for you.

*Executes: npm install*

The command completed successfully. Here's the output:
[Output of npm install command]

All dependencies have been installed according to your package.json file.

<reasoning>
The assistant did not use the todo list because this is a single command execution with immediate results. There are no multiple steps to track or organize, making the todo list unnecessary for this straightforward task.
</reasoning>
</example>

## Task States and Management

1. **Task States**: Use these states to track progress:
   - pending: Task not yet started
   - in_progress: Currently working on (limit to ONE task at a time)
   - completed: Task finished successfully

2. **Task Management**:
   - Update task status in real-time as you work
   - Mark tasks complete IMMEDIATELY after finishing (don't batch completions)
   - Only have ONE task in_progress at any time
   - Complete current tasks before starting new ones
   - Remove tasks that are no longer relevant from the list entirely

3. **Task Completion Requirements**:
   - ONLY mark a task as completed when you have FULLY accomplished it
   - If you encounter errors, blockers, or cannot finish, keep the task as in_progress
   - When blocked, create a new task describing what needs to be resolved
   - Never mark a task as completed if:
     - Tests are failing
     - Implementation is partial
     - You encountered unresolved errors
     - You couldn't find necessary files or dependencies

4. **Task Breakdown**:
   - Create specific, actionable items
   - Break complex tasks into smaller, manageable steps
   - Use clear, descriptive task names

When in doubt, use this tool. Being proactive with task management demonstrates attentiveness and ensures you complete all requirements successfully.


{
  // The updated todo list
  todos: {
    content: string;
    status: 'pending' | 'in_progress' | 'completed';
    priority: 'high' | 'medium' | 'low';
    id: string;
  }[];
}

"
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
                    message: "todos parameter is required and must be an array of todo items"
                        .to_string(),
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
            let todo_obj = todo_value
                .as_object()
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "Each todo item must be an object".to_string(),
                })?;

            let content = todo_obj
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "Each todo item must have a 'content' field".to_string(),
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
        let response = result.content;
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
        let created_todo_id = create_result.content["todos"][0]["id"].as_str().unwrap();

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
        let response = update_result.content;
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
