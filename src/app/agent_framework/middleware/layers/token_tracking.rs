//! Token Tracking Layer
//!
//! Middleware layer that tracks token usage and can trigger
//! context summarization when thresholds are exceeded.

#![warn(clippy::all, rust_2018_idioms)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::app::agent_framework::middleware::{
    ConversationLayer, LayerContext, LayerResult, PostResponseAction,
};

/// Configuration for token tracking behavior
#[derive(Debug, Clone)]
pub struct TokenTrackingConfig {
    /// Token threshold for triggering context summary injection
    pub summary_threshold: usize,
    /// Prefix to add to summary injections
    pub summary_prefix: String,
    /// Whether to log token counts
    pub log_tokens: bool,
}

impl Default for TokenTrackingConfig {
    fn default() -> Self {
        Self {
            summary_threshold: 100_000, // 100k tokens
            summary_prefix: "[Context Summary]\n".to_string(),
            log_tokens: true,
        }
    }
}

impl TokenTrackingConfig {
    /// Create a new configuration with a custom threshold
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.summary_threshold = threshold;
        self
    }

    /// Set the summary prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.summary_prefix = prefix.into();
        self
    }
}

/// Token tracking middleware layer
///
/// Tracks estimated token usage and can:
/// - Log token counts for monitoring
/// - Inject context summaries when thresholds are exceeded
/// - Provide token statistics
///
/// ## Example
///
/// ```ignore
/// let layer = TokenTrackingLayer::new(TokenTrackingConfig::default());
/// stack.add(layer);
/// ```
pub struct TokenTrackingLayer {
    /// Configuration
    config: TokenTrackingConfig,
    /// Running total of tokens sent
    tokens_sent: Arc<AtomicUsize>,
    /// Running total of tokens received
    tokens_received: Arc<AtomicUsize>,
    /// Whether summary was recently triggered (to avoid spam)
    summary_triggered: Arc<AtomicUsize>, // Using usize as bool for atomic
}

impl TokenTrackingLayer {
    /// Create a new token tracking layer
    pub fn new(config: TokenTrackingConfig) -> Self {
        Self {
            config,
            tokens_sent: Arc::new(AtomicUsize::new(0)),
            tokens_received: Arc::new(AtomicUsize::new(0)),
            summary_triggered: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(TokenTrackingConfig::default())
    }

    /// Get total tokens sent
    pub fn tokens_sent(&self) -> usize {
        self.tokens_sent.load(Ordering::Relaxed)
    }

    /// Get total tokens received
    pub fn tokens_received(&self) -> usize {
        self.tokens_received.load(Ordering::Relaxed)
    }

    /// Get total tokens (sent + received)
    pub fn total_tokens(&self) -> usize {
        self.tokens_sent() + self.tokens_received()
    }

    /// Reset token counters
    pub fn reset(&self) {
        self.tokens_sent.store(0, Ordering::Relaxed);
        self.tokens_received.store(0, Ordering::Relaxed);
        self.summary_triggered.store(0, Ordering::Relaxed);
    }

    /// Estimate tokens for a given text
    fn estimate_tokens(text: &str) -> usize {
        // Rough approximation: ~4 characters per token for English
        // This is a simplification - real tokenization is more complex
        text.len() / 4
    }

    /// Check if summary should be triggered
    fn should_trigger_summary(&self, ctx: &LayerContext) -> bool {
        // Don't trigger if already triggered recently
        if self.summary_triggered.load(Ordering::Relaxed) > 0 {
            return false;
        }

        // Check if token threshold exceeded
        let total = ctx.token_count + self.total_tokens();
        total > self.config.summary_threshold
    }
}

impl ConversationLayer for TokenTrackingLayer {
    fn name(&self) -> &str {
        "TokenTracking"
    }

    fn on_pre_send(&self, message: &str, ctx: &LayerContext) -> LayerResult<String> {
        let tokens = Self::estimate_tokens(message);
        self.tokens_sent.fetch_add(tokens, Ordering::Relaxed);

        if self.config.log_tokens {
            log::debug!(
                "TokenTracking: Sending ~{} tokens (total sent: {}, ctx: {})",
                tokens,
                self.tokens_sent(),
                ctx.token_count
            );
        }

        // Check if we should add a context summary prefix
        if self.should_trigger_summary(ctx) {
            self.summary_triggered.store(1, Ordering::Relaxed);
            log::info!(
                "TokenTracking: Threshold {} exceeded, would inject summary",
                self.config.summary_threshold
            );
            // Note: In a real implementation, we would generate a summary
            // of older messages here. For now, we just log the event.
        }

        Ok(message.to_string())
    }

    fn on_post_response(
        &self,
        response: &str,
        ctx: &LayerContext,
    ) -> LayerResult<PostResponseAction> {
        let tokens = Self::estimate_tokens(response);
        self.tokens_received.fetch_add(tokens, Ordering::Relaxed);

        if self.config.log_tokens {
            log::debug!(
                "TokenTracking: Received ~{} tokens (total received: {}, ctx: {})",
                tokens,
                self.tokens_received(),
                ctx.token_count
            );
        }

        Ok(PostResponseAction::PassThrough)
    }

    fn on_tool_complete(&self, tool_name: &str, success: bool, _ctx: &LayerContext) {
        if self.config.log_tokens {
            log::trace!(
                "TokenTracking: Tool '{}' completed (success: {})",
                tool_name,
                success
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::AgentType;

    #[test]
    fn test_config_default() {
        let config = TokenTrackingConfig::default();
        assert_eq!(config.summary_threshold, 100_000);
        assert!(config.log_tokens);
    }

    #[test]
    fn test_config_builder() {
        let config = TokenTrackingConfig::default()
            .with_threshold(50_000)
            .with_prefix("[Summary] ");

        assert_eq!(config.summary_threshold, 50_000);
        assert_eq!(config.summary_prefix, "[Summary] ");
    }

    #[test]
    fn test_layer_creation() {
        let layer = TokenTrackingLayer::with_defaults();
        assert_eq!(layer.tokens_sent(), 0);
        assert_eq!(layer.tokens_received(), 0);
        assert_eq!(layer.total_tokens(), 0);
    }

    #[test]
    fn test_token_estimation() {
        // ~4 chars per token
        let estimate = TokenTrackingLayer::estimate_tokens("Hello, world!"); // 13 chars
        assert!(estimate > 0);
        assert!(estimate <= 5); // Should be around 3
    }

    #[test]
    fn test_pre_send_tracking() {
        let layer = TokenTrackingLayer::with_defaults();
        let ctx = LayerContext::new("test", AgentType::TaskManager);

        // Send a message
        let result = layer.on_pre_send("Hello, this is a test message", &ctx);
        assert!(result.is_ok());

        // Tokens should be counted
        assert!(layer.tokens_sent() > 0);
        assert_eq!(layer.tokens_received(), 0);
    }

    #[test]
    fn test_post_response_tracking() {
        let layer = TokenTrackingLayer::with_defaults();
        let ctx = LayerContext::new("test", AgentType::TaskManager);

        // Receive a response
        let result = layer.on_post_response("This is a response from the agent", &ctx);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PostResponseAction::PassThrough));

        // Tokens should be counted
        assert_eq!(layer.tokens_sent(), 0);
        assert!(layer.tokens_received() > 0);
    }

    #[test]
    fn test_total_tokens() {
        let layer = TokenTrackingLayer::with_defaults();
        let ctx = LayerContext::new("test", AgentType::TaskManager);

        layer.on_pre_send("Hello", &ctx).unwrap();
        layer.on_post_response("Response", &ctx).unwrap();

        assert!(layer.total_tokens() > 0);
        assert_eq!(
            layer.total_tokens(),
            layer.tokens_sent() + layer.tokens_received()
        );
    }

    #[test]
    fn test_reset() {
        let layer = TokenTrackingLayer::with_defaults();
        let ctx = LayerContext::new("test", AgentType::TaskManager);

        layer.on_pre_send("Hello", &ctx).unwrap();
        layer.on_post_response("Response", &ctx).unwrap();

        assert!(layer.total_tokens() > 0);

        layer.reset();

        assert_eq!(layer.tokens_sent(), 0);
        assert_eq!(layer.tokens_received(), 0);
        assert_eq!(layer.total_tokens(), 0);
    }

    #[test]
    fn test_layer_name() {
        let layer = TokenTrackingLayer::with_defaults();
        assert_eq!(layer.name(), "TokenTracking");
    }
}
