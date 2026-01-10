//! Layer Stack
//!
//! Manages an ordered collection of middleware layers and coordinates
//! their execution for pre-send and post-response processing.

#![warn(clippy::all, rust_2018_idioms)]

use std::sync::Arc;

use super::{ConversationLayer, LayerContext, LayerError, LayerResult, PostResponseAction};

/// Stack of middleware layers
///
/// Layers are processed in order for pre-send operations and in
/// reverse order for post-response operations. This allows outer
/// layers to wrap inner layers' behavior.
#[derive(Default)]
pub struct LayerStack {
    /// Ordered list of layers
    layers: Vec<Arc<dyn ConversationLayer>>,
    /// Whether the stack is enabled
    enabled: bool,
}

impl LayerStack {
    /// Create a new empty layer stack
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            enabled: true,
        }
    }

    /// Add a layer to the stack
    ///
    /// Layers are processed in the order they are added for pre-send,
    /// and in reverse order for post-response.
    pub fn add_layer(&mut self, layer: Arc<dyn ConversationLayer>) {
        log::debug!("Adding middleware layer: {}", layer.name());
        self.layers.push(layer);
    }

    /// Add a layer using a concrete type
    pub fn add<L: ConversationLayer + 'static>(&mut self, layer: L) {
        self.add_layer(Arc::new(layer));
    }

    /// Remove all layers
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// Get the number of layers
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Enable or disable the stack
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if the stack is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get layer names for debugging
    pub fn layer_names(&self) -> Vec<&str> {
        self.layers.iter().map(|l| l.name()).collect()
    }

    /// Process a message through all layers before sending
    ///
    /// Layers are processed in order (first to last). Each layer
    /// can modify the message or abort the chain.
    ///
    /// # Arguments
    /// * `message` - The original message to send
    /// * `ctx` - Context about the conversation state
    ///
    /// # Returns
    /// * `Ok(String)` - The processed message (possibly modified)
    /// * `Err(LayerError)` - A layer aborted the chain
    pub fn process_pre_send(&self, message: &str, ctx: &LayerContext) -> LayerResult<String> {
        if !self.enabled || self.layers.is_empty() {
            return Ok(message.to_string());
        }

        let mut current_message = message.to_string();

        for layer in &self.layers {
            match layer.on_pre_send(&current_message, ctx) {
                Ok(modified) => {
                    if modified != current_message {
                        log::trace!(
                            "Layer '{}' modified message ({} -> {} chars)",
                            layer.name(),
                            current_message.len(),
                            modified.len()
                        );
                    }
                    current_message = modified;
                }
                Err(LayerError::Skipped(reason)) => {
                    log::trace!("Layer '{}' skipped: {}", layer.name(), reason);
                    // Continue with next layer
                }
                Err(LayerError::Abort(reason)) => {
                    log::warn!("Layer '{}' aborted chain: {}", layer.name(), reason);
                    return Err(LayerError::Abort(reason));
                }
                Err(e) => {
                    log::error!("Layer '{}' error: {}", layer.name(), e);
                    // Continue with next layer on non-fatal errors
                }
            }
        }

        Ok(current_message)
    }

    /// Process a response through all layers after receiving
    ///
    /// Layers are processed in reverse order (last to first). Each layer
    /// can modify the response, trigger injections, or pass through.
    ///
    /// # Arguments
    /// * `response` - The response received from the agent
    /// * `ctx` - Context about the conversation state
    ///
    /// # Returns
    /// The combined action to take (modifications and injections are accumulated)
    pub fn process_post_response(
        &self,
        response: &str,
        ctx: &LayerContext,
    ) -> LayerResult<PostResponseResult> {
        if !self.enabled || self.layers.is_empty() {
            return Ok(PostResponseResult::default());
        }

        let mut result = PostResponseResult::default();
        let mut current_response = response.to_string();

        // Process in reverse order
        for layer in self.layers.iter().rev() {
            match layer.on_post_response(&current_response, ctx) {
                Ok(action) => {
                    match action {
                        PostResponseAction::PassThrough => {
                            // Continue with current response
                        }
                        PostResponseAction::Modify(modified) => {
                            log::trace!(
                                "Layer '{}' modified response ({} -> {} chars)",
                                layer.name(),
                                current_response.len(),
                                modified.len()
                            );
                            current_response = modified;
                            result.was_modified = true;
                        }
                        PostResponseAction::InjectFollowUp(injection) => {
                            log::debug!("Layer '{}' queued follow-up injection", layer.name());
                            result.injections.push(injection);
                        }
                        PostResponseAction::SuppressAndInject(injection) => {
                            log::debug!(
                                "Layer '{}' suppressed response and queued injection",
                                layer.name()
                            );
                            result.suppress = true;
                            result.injections.push(injection);
                        }
                    }
                }
                Err(LayerError::Skipped(reason)) => {
                    log::trace!("Layer '{}' skipped: {}", layer.name(), reason);
                }
                Err(e) => {
                    log::error!("Layer '{}' error: {}", layer.name(), e);
                    // Continue with next layer on errors
                }
            }
        }

        result.final_response = current_response;
        Ok(result)
    }

    /// Notify all layers that a tool execution started
    pub fn notify_tool_start(&self, tool_name: &str, ctx: &LayerContext) {
        if !self.enabled {
            return;
        }

        for layer in &self.layers {
            layer.on_tool_start(tool_name, ctx);
        }
    }

    /// Notify all layers that a tool execution completed
    pub fn notify_tool_complete(&self, tool_name: &str, success: bool, ctx: &LayerContext) {
        if !self.enabled {
            return;
        }

        for layer in &self.layers {
            layer.on_tool_complete(tool_name, success, ctx);
        }
    }
}

/// Result of processing a response through the layer stack
#[derive(Debug, Default)]
pub struct PostResponseResult {
    /// The final response text (possibly modified)
    pub final_response: String,
    /// Whether the response was modified by any layer
    pub was_modified: bool,
    /// Whether to suppress displaying the response
    pub suppress: bool,
    /// Messages to inject as follow-ups
    pub injections: Vec<String>,
}

impl PostResponseResult {
    /// Check if there are any injections queued
    pub fn has_injections(&self) -> bool {
        !self.injections.is_empty()
    }

    /// Get the first injection (if any)
    pub fn first_injection(&self) -> Option<&str> {
        self.injections.first().map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test layer that passes through unchanged
    struct PassThroughLayer;

    impl ConversationLayer for PassThroughLayer {
        fn name(&self) -> &str {
            "PassThrough"
        }
    }

    /// Test layer that modifies messages
    struct ModifyLayer {
        prefix: String,
    }

    impl ConversationLayer for ModifyLayer {
        fn name(&self) -> &str {
            "Modify"
        }

        fn on_pre_send(&self, message: &str, _ctx: &LayerContext) -> LayerResult<String> {
            Ok(format!("{}{}", self.prefix, message))
        }

        fn on_post_response(
            &self,
            response: &str,
            _ctx: &LayerContext,
        ) -> LayerResult<PostResponseAction> {
            Ok(PostResponseAction::Modify(format!(
                "{}{}",
                self.prefix, response
            )))
        }
    }

    /// Test layer that injects follow-ups
    struct InjectLayer {
        injection: String,
    }

    impl ConversationLayer for InjectLayer {
        fn name(&self) -> &str {
            "Inject"
        }

        fn on_post_response(
            &self,
            _response: &str,
            _ctx: &LayerContext,
        ) -> LayerResult<PostResponseAction> {
            Ok(PostResponseAction::InjectFollowUp(self.injection.clone()))
        }
    }

    #[test]
    fn test_empty_stack() {
        let stack = LayerStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
        assert!(stack.is_enabled());
    }

    #[test]
    fn test_add_layers() {
        let mut stack = LayerStack::new();
        stack.add(PassThroughLayer);
        stack.add(ModifyLayer {
            prefix: "[MOD] ".to_string(),
        });

        assert_eq!(stack.len(), 2);
        assert!(!stack.is_empty());
        assert_eq!(stack.layer_names(), vec!["PassThrough", "Modify"]);
    }

    #[test]
    fn test_disabled_stack() {
        let mut stack = LayerStack::new();
        stack.add(ModifyLayer {
            prefix: "[MOD] ".to_string(),
        });
        stack.set_enabled(false);

        let ctx = LayerContext::default();
        let result = stack.process_pre_send("test", &ctx).unwrap();

        // Should pass through unchanged when disabled
        assert_eq!(result, "test");
    }

    #[test]
    fn test_pre_send_passthrough() {
        let mut stack = LayerStack::new();
        stack.add(PassThroughLayer);

        let ctx = LayerContext::default();
        let result = stack.process_pre_send("Hello", &ctx).unwrap();

        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_pre_send_modify() {
        let mut stack = LayerStack::new();
        stack.add(ModifyLayer {
            prefix: "[PREFIX] ".to_string(),
        });

        let ctx = LayerContext::default();
        let result = stack.process_pre_send("Hello", &ctx).unwrap();

        assert_eq!(result, "[PREFIX] Hello");
    }

    #[test]
    fn test_pre_send_chain() {
        let mut stack = LayerStack::new();
        stack.add(ModifyLayer {
            prefix: "[A] ".to_string(),
        });
        stack.add(ModifyLayer {
            prefix: "[B] ".to_string(),
        });

        let ctx = LayerContext::default();
        let result = stack.process_pre_send("Hello", &ctx).unwrap();

        // Should apply A first, then B
        assert_eq!(result, "[B] [A] Hello");
    }

    #[test]
    fn test_post_response_passthrough() {
        let mut stack = LayerStack::new();
        stack.add(PassThroughLayer);

        let ctx = LayerContext::default();
        let result = stack.process_post_response("Response", &ctx).unwrap();

        assert_eq!(result.final_response, "Response");
        assert!(!result.was_modified);
        assert!(!result.suppress);
        assert!(result.injections.is_empty());
    }

    #[test]
    fn test_post_response_modify() {
        let mut stack = LayerStack::new();
        stack.add(ModifyLayer {
            prefix: "[MOD] ".to_string(),
        });

        let ctx = LayerContext::default();
        let result = stack.process_post_response("Response", &ctx).unwrap();

        assert_eq!(result.final_response, "[MOD] Response");
        assert!(result.was_modified);
    }

    #[test]
    fn test_post_response_inject() {
        let mut stack = LayerStack::new();
        stack.add(InjectLayer {
            injection: "Follow up message".to_string(),
        });

        let ctx = LayerContext::default();
        let result = stack.process_post_response("Response", &ctx).unwrap();

        assert!(result.has_injections());
        assert_eq!(result.first_injection(), Some("Follow up message"));
    }

    #[test]
    fn test_clear_layers() {
        let mut stack = LayerStack::new();
        stack.add(PassThroughLayer);
        stack.add(PassThroughLayer);
        assert_eq!(stack.len(), 2);

        stack.clear();
        assert!(stack.is_empty());
    }
}
