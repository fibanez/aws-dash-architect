//! Logging Layer
//!
//! Simple middleware layer that logs all message flow for debugging.

#![warn(clippy::all, rust_2018_idioms)]

use crate::app::agent_framework::middleware::{
    ConversationLayer, LayerContext, LayerResult, PostResponseAction,
};

/// Logging level for the layer
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LogLevel {
    /// Log only basic events
    Basic,
    /// Log with message previews
    #[default]
    Detailed,
    /// Log full messages
    Full,
}

/// Logging middleware layer
///
/// Logs all conversation flow for debugging purposes.
/// Does not modify any messages, only observes.
///
/// ## Example
///
/// ```ignore
/// let layer = LoggingLayer::new(LogLevel::Detailed);
/// stack.add(layer);
/// ```
pub struct LoggingLayer {
    level: LogLevel,
    preview_length: usize,
}

impl LoggingLayer {
    /// Create a new logging layer
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            preview_length: 100,
        }
    }

    /// Create with default settings (Detailed level)
    pub fn with_defaults() -> Self {
        Self::new(LogLevel::default())
    }

    /// Set the preview length for detailed logging
    pub fn with_preview_length(mut self, length: usize) -> Self {
        self.preview_length = length;
        self
    }

    /// Format a message preview
    fn preview(&self, text: &str) -> String {
        if text.len() <= self.preview_length {
            text.to_string()
        } else {
            format!("{}...", &text[..self.preview_length])
        }
    }
}

impl ConversationLayer for LoggingLayer {
    fn name(&self) -> &str {
        "Logging"
    }

    fn on_pre_send(&self, message: &str, ctx: &LayerContext) -> LayerResult<String> {
        match self.level {
            LogLevel::Basic => {
                log::info!(
                    "[{}] Sending message ({} chars)",
                    ctx.agent_id,
                    message.len()
                );
            }
            LogLevel::Detailed => {
                log::info!(
                    "[{}] Sending: {} ({} chars, turn {})",
                    ctx.agent_id,
                    self.preview(message),
                    message.len(),
                    ctx.turn_count
                );
            }
            LogLevel::Full => {
                log::info!(
                    "[{}] Sending (turn {}, {} tokens est.):\n{}",
                    ctx.agent_id,
                    ctx.turn_count,
                    ctx.token_count,
                    message
                );
            }
        }

        Ok(message.to_string())
    }

    fn on_post_response(
        &self,
        response: &str,
        ctx: &LayerContext,
    ) -> LayerResult<PostResponseAction> {
        match self.level {
            LogLevel::Basic => {
                log::info!(
                    "[{}] Received response ({} chars)",
                    ctx.agent_id,
                    response.len()
                );
            }
            LogLevel::Detailed => {
                log::info!(
                    "[{}] Received: {} ({} chars)",
                    ctx.agent_id,
                    self.preview(response),
                    response.len()
                );
            }
            LogLevel::Full => {
                log::info!("[{}] Received response:\n{}", ctx.agent_id, response);
            }
        }

        Ok(PostResponseAction::PassThrough)
    }

    fn on_tool_start(&self, tool_name: &str, ctx: &LayerContext) {
        log::debug!("[{}] Tool starting: {}", ctx.agent_id, tool_name);
    }

    fn on_tool_complete(&self, tool_name: &str, success: bool, ctx: &LayerContext) {
        if success {
            log::debug!("[{}] Tool completed: {}", ctx.agent_id, tool_name);
        } else {
            log::warn!("[{}] Tool failed: {}", ctx.agent_id, tool_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::AgentType;

    #[test]
    fn test_layer_creation() {
        let layer = LoggingLayer::with_defaults();
        assert_eq!(layer.name(), "Logging");
        assert_eq!(layer.level, LogLevel::Detailed);
    }

    #[test]
    fn test_preview() {
        let layer = LoggingLayer::new(LogLevel::Detailed).with_preview_length(10);

        assert_eq!(layer.preview("short"), "short");
        assert_eq!(layer.preview("this is a longer message"), "this is a ...");
    }

    #[test]
    fn test_pre_send_passthrough() {
        let layer = LoggingLayer::with_defaults();
        let ctx = LayerContext::new("test", AgentType::TaskManager);

        let result = layer.on_pre_send("Hello", &ctx).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_post_response_passthrough() {
        let layer = LoggingLayer::with_defaults();
        let ctx = LayerContext::new("test", AgentType::TaskManager);

        let result = layer.on_post_response("Response", &ctx).unwrap();
        assert!(matches!(result, PostResponseAction::PassThrough));
    }

    #[test]
    fn test_log_levels() {
        // Just verify they can be created
        let _basic = LoggingLayer::new(LogLevel::Basic);
        let _detailed = LoggingLayer::new(LogLevel::Detailed);
        let _full = LoggingLayer::new(LogLevel::Full);
    }
}
