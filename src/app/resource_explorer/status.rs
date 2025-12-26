//! Status messaging system for reporting operation progress from parallel tasks.
//!
//! This module provides a thread-safe way for async operations (like AWS API calls)
//! to report their current status to the UI for display in a status bar.

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Maximum number of status messages to retain
const MAX_MESSAGES: usize = 50;

/// Maximum age of messages to display (in seconds)
const MESSAGE_DISPLAY_DURATION_SECS: f32 = 3.0;

/// A status message from an async operation
#[derive(Clone, Debug)]
pub struct StatusMessage {
    /// When the message was created
    pub timestamp: Instant,
    /// The operation category (e.g., "IAM", "S3", "EC2")
    pub category: String,
    /// The specific operation (e.g., "list_roles", "get_bucket_policy")
    pub operation: String,
    /// Human-readable detail (e.g., "role: MyRole", "bucket: my-bucket")
    pub detail: Option<String>,
    /// Whether this is a completion message
    pub is_complete: bool,
}

impl StatusMessage {
    /// Create a new status message for an operation starting
    pub fn starting(category: &str, operation: &str, detail: Option<&str>) -> Self {
        Self {
            timestamp: Instant::now(),
            category: category.to_string(),
            operation: operation.to_string(),
            detail: detail.map(|s| s.to_string()),
            is_complete: false,
        }
    }

    /// Create a completion message
    pub fn completed(category: &str, operation: &str, detail: Option<&str>) -> Self {
        Self {
            timestamp: Instant::now(),
            category: category.to_string(),
            operation: operation.to_string(),
            detail: detail.map(|s| s.to_string()),
            is_complete: true,
        }
    }

    /// Format the message for display
    pub fn display_text(&self) -> String {
        let action = if self.is_complete { "Done" } else { "Getting" };
        match &self.detail {
            Some(detail) => format!(
                "{} {} {} ({})",
                action, self.category, self.operation, detail
            ),
            None => format!("{} {} {}", action, self.category, self.operation),
        }
    }

    /// Check if this message is still fresh enough to display
    pub fn is_fresh(&self) -> bool {
        self.timestamp.elapsed().as_secs_f32() < MESSAGE_DISPLAY_DURATION_SECS
    }
}

/// Thread-safe status channel for collecting messages from async tasks
#[derive(Clone)]
pub struct StatusChannel {
    messages: Arc<RwLock<VecDeque<StatusMessage>>>,
}

impl Default for StatusChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusChannel {
    /// Create a new status channel
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_MESSAGES))),
        }
    }

    /// Send a status message
    pub fn send(&self, message: StatusMessage) {
        if let Ok(mut messages) = self.messages.write() {
            // Remove old messages if at capacity
            while messages.len() >= MAX_MESSAGES {
                messages.pop_front();
            }
            messages.push_back(message);
        }
    }

    /// Send a "starting" status message
    pub fn report_starting(&self, category: &str, operation: &str, detail: Option<&str>) {
        self.send(StatusMessage::starting(category, operation, detail));
    }

    /// Send a "completed" status message
    pub fn report_completed(&self, category: &str, operation: &str, detail: Option<&str>) {
        self.send(StatusMessage::completed(category, operation, detail));
    }

    /// Get all fresh (recent) messages for display
    pub fn get_fresh_messages(&self) -> Vec<StatusMessage> {
        if let Ok(messages) = self.messages.read() {
            messages.iter().filter(|m| m.is_fresh()).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Get the most recent active (non-complete) messages for status bar display
    pub fn get_active_operations(&self) -> Vec<String> {
        if let Ok(messages) = self.messages.read() {
            // Get unique active operations from recent messages
            let mut active: Vec<String> = Vec::new();
            let mut seen_ops: std::collections::HashSet<String> = std::collections::HashSet::new();

            // Iterate in reverse to get most recent first
            for msg in messages.iter().rev() {
                if !msg.is_fresh() {
                    continue;
                }

                let op_key = format!("{}:{}", msg.category, msg.operation);

                if msg.is_complete {
                    // Mark this operation as completed
                    seen_ops.insert(op_key);
                } else if !seen_ops.contains(&op_key) {
                    // This operation is still in progress
                    active.push(msg.display_text());
                    seen_ops.insert(op_key);
                }
            }

            active.reverse(); // Show oldest first
            active
        } else {
            Vec::new()
        }
    }

    /// Get a single-line status summary for the status bar
    pub fn get_status_line(&self) -> String {
        let active = self.get_active_operations();
        if active.is_empty() {
            "Ready".to_string()
        } else if active.len() == 1 {
            active[0].clone()
        } else if active.len() <= 3 {
            active.join(" | ")
        } else {
            // Show first 2 and count of remaining
            format!(
                "{} | {} | +{} more...",
                active[0],
                active[1],
                active.len() - 2
            )
        }
    }

    /// Clear all messages
    pub fn clear(&self) {
        if let Ok(mut messages) = self.messages.write() {
            messages.clear();
        }
    }
}

/// Global status channel instance
static GLOBAL_STATUS: std::sync::OnceLock<StatusChannel> = std::sync::OnceLock::new();

/// Get the global status channel
pub fn global_status() -> &'static StatusChannel {
    GLOBAL_STATUS.get_or_init(StatusChannel::new)
}

/// Convenience function to report an operation starting
pub fn report_status(category: &str, operation: &str, detail: Option<&str>) {
    global_status().report_starting(category, operation, detail);
}

/// Convenience function to report an operation completed
pub fn report_status_done(category: &str, operation: &str, detail: Option<&str>) {
    global_status().report_completed(category, operation, detail);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_status_message_display() {
        let msg = StatusMessage::starting("IAM", "list_roles", None);
        assert_eq!(msg.display_text(), "Getting IAM list_roles");

        let msg = StatusMessage::starting("IAM", "list_access_keys", Some("user: admin"));
        assert_eq!(
            msg.display_text(),
            "Getting IAM list_access_keys (user: admin)"
        );

        let msg = StatusMessage::completed("IAM", "list_roles", None);
        assert_eq!(msg.display_text(), "Done IAM list_roles");
    }

    #[test]
    fn test_status_channel_thread_safety() {
        let channel = StatusChannel::new();
        let channel_clone = channel.clone();

        let handle = thread::spawn(move || {
            for i in 0..10 {
                channel_clone.report_starting("Test", &format!("op_{}", i), None);
            }
        });

        for i in 10..20 {
            channel.report_starting("Test", &format!("op_{}", i), None);
        }

        handle.join().unwrap();

        let messages = channel.get_fresh_messages();
        assert_eq!(messages.len(), 20);
    }

    #[test]
    fn test_message_freshness() {
        let msg = StatusMessage::starting("Test", "op", None);
        assert!(msg.is_fresh());

        // Note: In real tests, we'd mock time, but for now we just verify it works
    }

    #[test]
    fn test_status_line_formatting() {
        let channel = StatusChannel::new();

        // Empty - should show Ready
        assert_eq!(channel.get_status_line(), "Ready");

        // Single operation
        channel.report_starting("IAM", "list_roles", None);
        assert!(channel.get_status_line().contains("IAM"));

        // Multiple operations
        channel.report_starting("S3", "list_buckets", None);
        channel.report_starting("EC2", "list_instances", None);
        let status = channel.get_status_line();
        assert!(status.contains("|"));
    }
}
