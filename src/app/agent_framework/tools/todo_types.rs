#![warn(clippy::all, rust_2018_idioms)]

//! Todo List Types - Shared between todo-write and todo-read tools
//!
//! These types match Claude Code's proven todo list schema exactly.

use serde::{Deserialize, Serialize};

/// A single todo item in the task list
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TodoItem {
    /// Imperative form: what needs to be done (e.g., "List EC2 instances")
    pub content: String,

    /// Present continuous form: what's being done (e.g., "Listing EC2 instances")
    #[serde(rename = "activeForm")]
    pub active_form: String,

    /// Current status of the task
    pub status: TodoStatus,
}

/// Status of a todo item
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// Task not yet started
    Pending,

    /// Task currently being worked on (only ONE task should have this status)
    InProgress,

    /// Task completed successfully
    Completed,
}

impl TodoItem {
    /// Create a new todo item
    pub fn new(content: String, active_form: String, status: TodoStatus) -> Self {
        Self {
            content,
            active_form,
            status,
        }
    }

    /// Check if this todo item is in progress
    pub fn is_in_progress(&self) -> bool {
        matches!(self.status, TodoStatus::InProgress)
    }

    /// Check if this todo item is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TodoStatus::Completed)
    }

    /// Check if this todo item is pending
    pub fn is_pending(&self) -> bool {
        matches!(self.status, TodoStatus::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_item_creation() {
        let item = TodoItem::new(
            "List EC2 instances".to_string(),
            "Listing EC2 instances".to_string(),
            TodoStatus::Pending,
        );

        assert_eq!(item.content, "List EC2 instances");
        assert_eq!(item.active_form, "Listing EC2 instances");
        assert!(item.is_pending());
    }

    #[test]
    fn test_todo_item_status_checks() {
        let pending = TodoItem::new("Task 1".into(), "Doing task 1".into(), TodoStatus::Pending);
        let in_progress = TodoItem::new(
            "Task 2".into(),
            "Doing task 2".into(),
            TodoStatus::InProgress,
        );
        let completed = TodoItem::new(
            "Task 3".into(),
            "Doing task 3".into(),
            TodoStatus::Completed,
        );

        assert!(pending.is_pending());
        assert!(!pending.is_in_progress());
        assert!(!pending.is_completed());

        assert!(!in_progress.is_pending());
        assert!(in_progress.is_in_progress());
        assert!(!in_progress.is_completed());

        assert!(!completed.is_pending());
        assert!(!completed.is_in_progress());
        assert!(completed.is_completed());
    }

    #[test]
    fn test_todo_item_serialization() {
        let item = TodoItem::new(
            "Test task".to_string(),
            "Testing task".to_string(),
            TodoStatus::InProgress,
        );

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: TodoItem = serde_json::from_str(&json).unwrap();

        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_todo_status_serialization() {
        let pending_json = serde_json::to_string(&TodoStatus::Pending).unwrap();
        assert_eq!(pending_json, r#""pending""#);

        let in_progress_json = serde_json::to_string(&TodoStatus::InProgress).unwrap();
        assert_eq!(in_progress_json, r#""in_progress""#);

        let completed_json = serde_json::to_string(&TodoStatus::Completed).unwrap();
        assert_eq!(completed_json, r#""completed""#);
    }
}
