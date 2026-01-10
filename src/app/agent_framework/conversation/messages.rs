//! Simplified message types for agent conversations
//!
//! This module defines minimal message types for the standalone agent implementation.
//! Unlike the legacy system, we don't use nested message trees, tool call hierarchies, or
//! complex content blocks. Just simple User/Assistant messages with text content.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Message role in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationRole {
    /// User message (input from user)
    User,
    /// Assistant message (response from LLM)
    Assistant,
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Role of the message sender
    pub role: ConversationRole,
    /// Text content of the message
    pub content: String,
    /// Timestamp when the message was created
    pub timestamp: DateTime<Utc>,
}

impl ConversationMessage {
    /// Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ConversationRole::User,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ConversationRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
        }
    }
}

/// Response types from the agent background thread
///
/// These responses flow from the agent background thread to the UI through
/// a channel, enabling non-blocking agent execution while providing real-time
/// feedback to the user interface.
#[derive(Debug, Clone)]
pub enum ConversationResponse {
    /// Agent completed successfully with final response text
    ///
    /// This indicates the agent has finished processing and the provided
    /// string contains the complete response to display to the user.
    Success(String),

    /// Agent encountered an error
    ///
    /// The provided string contains error details that should be displayed
    /// to the user. The agent status will be marked as Failed.
    Error(String),

    /// Status update (for UI display)
    ///
    /// Provides intermediate status messages during long-running operations.
    /// Examples: "Starting Task", "Task Completed", "Processing Results"
    ///
    /// Unlike Success/Error, this does not mark the agent as complete and
    /// allows the agent to continue processing while updating the UI status.
    ///
    /// **Note**: Infrastructure is in place but StatusUpdate messages are not
    /// actively sent by tools yet. This requires deeper integration with the
    /// stood library's response channel mechanism.
    StatusUpdate(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_message() {
        let msg = ConversationMessage::user("Hello");
        assert_eq!(msg.role, ConversationRole::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_create_assistant_message() {
        let msg = ConversationMessage::assistant("World");
        assert_eq!(msg.role, ConversationRole::Assistant);
        assert_eq!(msg.content, "World");
    }

    #[test]
    fn test_message_timestamp() {
        let msg = ConversationMessage::user("test");
        let now = Utc::now();
        // Timestamp should be very close to now (within 1 second)
        assert!((now - msg.timestamp).num_seconds().abs() < 1);
    }
}
