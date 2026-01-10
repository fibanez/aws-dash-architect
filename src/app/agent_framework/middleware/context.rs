//! Layer Context
//!
//! Provides contextual information to middleware layers about the current
//! conversation state, allowing layers to make informed decisions.

#![warn(clippy::all, rust_2018_idioms)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::app::agent_framework::AgentType;

/// Context provided to middleware layers
///
/// Contains information about the current conversation state that layers
/// can use for decision-making. This is passed to layer callbacks.
#[derive(Debug, Clone)]
pub struct LayerContext {
    /// Unique identifier for this agent
    pub agent_id: String,
    /// Type of agent (TaskManager, TaskWorker, etc.)
    pub agent_type: AgentType,
    /// Estimated token count for the conversation
    pub token_count: usize,
    /// Number of conversation turns (user + assistant pairs)
    pub turn_count: usize,
    /// Number of messages in the conversation
    pub message_count: usize,
    /// Name of the last tool executed (if any)
    pub last_tool: Option<String>,
    /// Whether the last tool succeeded
    pub last_tool_success: bool,
    /// Time when processing started
    pub processing_start: Option<Instant>,
    /// Custom metadata that layers can share
    metadata: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for LayerContext {
    fn default() -> Self {
        Self {
            agent_id: String::new(),
            agent_type: AgentType::TaskManager,
            token_count: 0,
            turn_count: 0,
            message_count: 0,
            last_tool: None,
            last_tool_success: true,
            processing_start: None,
            metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl LayerContext {
    /// Create a new layer context
    pub fn new(agent_id: impl Into<String>, agent_type: AgentType) -> Self {
        Self {
            agent_id: agent_id.into(),
            agent_type,
            ..Default::default()
        }
    }

    /// Create a builder for constructing a context
    pub fn builder() -> LayerContextBuilder {
        LayerContextBuilder::new()
    }

    /// Set the token count
    pub fn with_token_count(mut self, count: usize) -> Self {
        self.token_count = count;
        self
    }

    /// Set the turn count
    pub fn with_turn_count(mut self, count: usize) -> Self {
        self.turn_count = count;
        self
    }

    /// Set the message count
    pub fn with_message_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    /// Set the last tool information
    pub fn with_last_tool(mut self, tool_name: impl Into<String>, success: bool) -> Self {
        self.last_tool = Some(tool_name.into());
        self.last_tool_success = success;
        self
    }

    /// Mark processing as started
    pub fn with_processing_start(mut self) -> Self {
        self.processing_start = Some(Instant::now());
        self
    }

    /// Get elapsed time since processing started
    pub fn elapsed_ms(&self) -> Option<u64> {
        self.processing_start
            .map(|start| start.elapsed().as_millis() as u64)
    }

    /// Set a metadata value
    pub fn set_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
        if let Ok(mut metadata) = self.metadata.lock() {
            metadata.insert(key.into(), value.into());
        }
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Option<String> {
        self.metadata.lock().ok().and_then(|m| m.get(key).cloned())
    }

    /// Check if the conversation is long (many tokens)
    pub fn is_long_conversation(&self, threshold: usize) -> bool {
        self.token_count > threshold
    }

    /// Check if many turns have occurred
    pub fn many_turns(&self, threshold: usize) -> bool {
        self.turn_count > threshold
    }

    /// Estimate tokens from a message (rough approximation)
    pub fn estimate_tokens(text: &str) -> usize {
        // Rough approximation: ~4 characters per token for English
        text.len() / 4
    }
}

/// Builder for LayerContext
#[derive(Debug, Default)]
pub struct LayerContextBuilder {
    agent_id: Option<String>,
    agent_type: Option<AgentType>,
    token_count: usize,
    turn_count: usize,
    message_count: usize,
    last_tool: Option<String>,
    last_tool_success: bool,
}

impl LayerContextBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the agent ID
    pub fn agent_id(mut self, id: impl Into<String>) -> Self {
        self.agent_id = Some(id.into());
        self
    }

    /// Set the agent type
    pub fn agent_type(mut self, agent_type: AgentType) -> Self {
        self.agent_type = Some(agent_type);
        self
    }

    /// Set the token count
    pub fn token_count(mut self, count: usize) -> Self {
        self.token_count = count;
        self
    }

    /// Set the turn count
    pub fn turn_count(mut self, count: usize) -> Self {
        self.turn_count = count;
        self
    }

    /// Set the message count
    pub fn message_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    /// Set the last tool information
    pub fn last_tool(mut self, tool_name: impl Into<String>, success: bool) -> Self {
        self.last_tool = Some(tool_name.into());
        self.last_tool_success = success;
        self
    }

    /// Build the context
    pub fn build(self) -> LayerContext {
        LayerContext {
            agent_id: self.agent_id.unwrap_or_default(),
            agent_type: self.agent_type.unwrap_or(AgentType::TaskManager),
            token_count: self.token_count,
            turn_count: self.turn_count,
            message_count: self.message_count,
            last_tool: self.last_tool,
            last_tool_success: self.last_tool_success,
            processing_start: None,
            metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_context_default() {
        let ctx = LayerContext::default();
        assert!(ctx.agent_id.is_empty());
        assert_eq!(ctx.token_count, 0);
        assert_eq!(ctx.turn_count, 0);
        assert!(ctx.last_tool.is_none());
    }

    #[test]
    fn test_layer_context_new() {
        let ctx = LayerContext::new("agent-123", AgentType::TaskManager);
        assert_eq!(ctx.agent_id, "agent-123");
        assert!(matches!(ctx.agent_type, AgentType::TaskManager));
    }

    #[test]
    fn test_layer_context_builder() {
        let ctx = LayerContext::builder()
            .agent_id("test-agent")
            .agent_type(AgentType::TaskManager)
            .token_count(5000)
            .turn_count(10)
            .message_count(20)
            .last_tool("execute_javascript", true)
            .build();

        assert_eq!(ctx.agent_id, "test-agent");
        assert_eq!(ctx.token_count, 5000);
        assert_eq!(ctx.turn_count, 10);
        assert_eq!(ctx.message_count, 20);
        assert_eq!(ctx.last_tool, Some("execute_javascript".to_string()));
        assert!(ctx.last_tool_success);
    }

    #[test]
    fn test_layer_context_fluent() {
        let ctx = LayerContext::new("agent", AgentType::TaskManager)
            .with_token_count(1000)
            .with_turn_count(5)
            .with_message_count(10)
            .with_last_tool("test_tool", false);

        assert_eq!(ctx.token_count, 1000);
        assert_eq!(ctx.turn_count, 5);
        assert_eq!(ctx.message_count, 10);
        assert_eq!(ctx.last_tool, Some("test_tool".to_string()));
        assert!(!ctx.last_tool_success);
    }

    #[test]
    fn test_layer_context_metadata() {
        let ctx = LayerContext::default();
        ctx.set_metadata("key1", "value1");
        ctx.set_metadata("key2", "value2");

        assert_eq!(ctx.get_metadata("key1"), Some("value1".to_string()));
        assert_eq!(ctx.get_metadata("key2"), Some("value2".to_string()));
        assert!(ctx.get_metadata("nonexistent").is_none());
    }

    #[test]
    fn test_is_long_conversation() {
        let ctx = LayerContext::default().with_token_count(50000);
        assert!(ctx.is_long_conversation(40000));
        assert!(!ctx.is_long_conversation(60000));
    }

    #[test]
    fn test_many_turns() {
        let ctx = LayerContext::default().with_turn_count(15);
        assert!(ctx.many_turns(10));
        assert!(!ctx.many_turns(20));
    }

    #[test]
    fn test_estimate_tokens() {
        // Rough approximation: ~4 chars per token
        let estimate = LayerContext::estimate_tokens("Hello, world! This is a test.");
        assert!(estimate > 0);
        assert!(estimate < 20); // Should be roughly 7-8 tokens
    }

    #[test]
    fn test_elapsed_time() {
        let ctx = LayerContext::default().with_processing_start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = ctx.elapsed_ms();
        assert!(elapsed.is_some());
        assert!(elapsed.unwrap() >= 10);
    }
}
