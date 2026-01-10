//! Conversation Middleware System
//!
//! Provides a layered middleware architecture for processing messages
//! before and after they are sent to the Stood agent. This allows for:
//!
//! - Pre-processing messages (adding context, token management)
//! - Post-processing responses (triggering follow-ups, modifications)
//! - Observing conversation flow without modifying Stood library
//!
//! ## Architecture
//!
//! ```text
//! User Message
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  LayerStack     │
//! │  ┌───────────┐  │
//! │  │ Layer 1   │──┼── on_pre_send()
//! │  │ Layer 2   │  │
//! │  │ Layer 3   │  │
//! │  └───────────┘  │
//! └─────────────────┘
//!      │
//!      ▼
//!   Stood Agent
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  LayerStack     │
//! │  ┌───────────┐  │
//! │  │ Layer 3   │──┼── on_post_response()
//! │  │ Layer 2   │  │
//! │  │ Layer 1   │  │
//! │  └───────────┘  │
//! └─────────────────┘
//!      │
//!      ▼
//!   UI Display
//! ```

#![warn(clippy::all, rust_2018_idioms)]

mod context;
pub mod layers;
mod stack;
pub mod workspace_locking;
pub mod page_validation;

pub use context::LayerContext;
pub use stack::LayerStack;
pub use workspace_locking::WorkspaceLockingMiddleware;
pub use page_validation::PageValidationMiddleware;

use std::fmt;

/// Result type for layer operations
pub type LayerResult<T> = Result<T, LayerError>;

/// Errors that can occur during layer processing
#[derive(Debug, Clone)]
pub enum LayerError {
    /// Layer processing failed with a message
    ProcessingFailed(String),
    /// Layer was skipped (not an error, just informational)
    Skipped(String),
    /// Chain should be aborted
    Abort(String),
}

impl fmt::Display for LayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayerError::ProcessingFailed(msg) => write!(f, "Layer processing failed: {}", msg),
            LayerError::Skipped(reason) => write!(f, "Layer skipped: {}", reason),
            LayerError::Abort(reason) => write!(f, "Layer chain aborted: {}", reason),
        }
    }
}

impl std::error::Error for LayerError {}

/// Action to take after processing a response
#[derive(Debug, Clone, Default)]
pub enum PostResponseAction {
    /// Pass the response through unchanged
    #[default]
    PassThrough,
    /// Modify the response text before displaying
    Modify(String),
    /// Queue a follow-up message to be injected
    InjectFollowUp(String),
    /// Suppress this response and inject a new message instead
    SuppressAndInject(String),
}

impl PostResponseAction {
    /// Check if this action modifies the response
    pub fn modifies_response(&self) -> bool {
        matches!(
            self,
            PostResponseAction::Modify(_) | PostResponseAction::SuppressAndInject(_)
        )
    }

    /// Check if this action triggers an injection
    pub fn triggers_injection(&self) -> bool {
        matches!(
            self,
            PostResponseAction::InjectFollowUp(_) | PostResponseAction::SuppressAndInject(_)
        )
    }

    /// Get the injection message if any
    pub fn injection_message(&self) -> Option<&str> {
        match self {
            PostResponseAction::InjectFollowUp(msg)
            | PostResponseAction::SuppressAndInject(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get the modified response if any
    pub fn modified_response(&self) -> Option<&str> {
        match self {
            PostResponseAction::Modify(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Trait for conversation middleware layers
///
/// Layers can intercept messages before they are sent to the agent
/// and responses before they are displayed to the user.
///
/// Layers are processed in order for pre-send (first to last) and
/// reverse order for post-response (last to first).
pub trait ConversationLayer: Send + Sync {
    /// Get the name of this layer for logging
    fn name(&self) -> &str;

    /// Process a message before sending to the agent
    ///
    /// Called for each outgoing message. Can modify the message,
    /// add context, or abort the send.
    ///
    /// # Arguments
    /// * `message` - The message about to be sent
    /// * `ctx` - Context about the conversation state
    ///
    /// # Returns
    /// * `Ok(String)` - The (possibly modified) message to send
    /// * `Err(LayerError)` - Error occurred, chain may be aborted
    fn on_pre_send(&self, message: &str, _ctx: &LayerContext) -> LayerResult<String> {
        // Default: pass through unchanged
        Ok(message.to_string())
    }

    /// Process a response after receiving from the agent
    ///
    /// Called for each incoming response. Can modify the response,
    /// trigger follow-up injections, or suppress the response.
    ///
    /// # Arguments
    /// * `response` - The response received from the agent
    /// * `ctx` - Context about the conversation state
    ///
    /// # Returns
    /// * `Ok(PostResponseAction)` - Action to take with this response
    /// * `Err(LayerError)` - Error occurred during processing
    fn on_post_response(
        &self,
        _response: &str,
        _ctx: &LayerContext,
    ) -> LayerResult<PostResponseAction> {
        // Default: pass through unchanged
        Ok(PostResponseAction::PassThrough)
    }

    /// Called when a tool execution starts
    ///
    /// Useful for tracking tool usage, timing, etc.
    fn on_tool_start(&self, _tool_name: &str, _ctx: &LayerContext) {
        // Default: no-op
    }

    /// Called when a tool execution completes
    ///
    /// Useful for post-tool analysis, caching, etc.
    fn on_tool_complete(&self, _tool_name: &str, _success: bool, _ctx: &LayerContext) {
        // Default: no-op
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_error_display() {
        let err = LayerError::ProcessingFailed("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = LayerError::Skipped("not applicable".to_string());
        assert!(err.to_string().contains("skipped"));

        let err = LayerError::Abort("critical failure".to_string());
        assert!(err.to_string().contains("aborted"));
    }

    #[test]
    fn test_post_response_action_default() {
        let action = PostResponseAction::default();
        assert!(matches!(action, PostResponseAction::PassThrough));
    }

    #[test]
    fn test_post_response_action_modifies() {
        assert!(!PostResponseAction::PassThrough.modifies_response());
        assert!(PostResponseAction::Modify("test".into()).modifies_response());
        assert!(!PostResponseAction::InjectFollowUp("test".into()).modifies_response());
        assert!(PostResponseAction::SuppressAndInject("test".into()).modifies_response());
    }

    #[test]
    fn test_post_response_action_triggers_injection() {
        assert!(!PostResponseAction::PassThrough.triggers_injection());
        assert!(!PostResponseAction::Modify("test".into()).triggers_injection());
        assert!(PostResponseAction::InjectFollowUp("test".into()).triggers_injection());
        assert!(PostResponseAction::SuppressAndInject("test".into()).triggers_injection());
    }

    #[test]
    fn test_post_response_action_injection_message() {
        assert!(PostResponseAction::PassThrough
            .injection_message()
            .is_none());
        assert!(PostResponseAction::Modify("test".into())
            .injection_message()
            .is_none());
        assert_eq!(
            PostResponseAction::InjectFollowUp("follow".into()).injection_message(),
            Some("follow")
        );
        assert_eq!(
            PostResponseAction::SuppressAndInject("suppress".into()).injection_message(),
            Some("suppress")
        );
    }

    #[test]
    fn test_post_response_action_modified_response() {
        assert!(PostResponseAction::PassThrough
            .modified_response()
            .is_none());
        assert_eq!(
            PostResponseAction::Modify("modified".into()).modified_response(),
            Some("modified")
        );
        assert!(PostResponseAction::InjectFollowUp("test".into())
            .modified_response()
            .is_none());
        assert!(PostResponseAction::SuppressAndInject("test".into())
            .modified_response()
            .is_none());
    }
}
