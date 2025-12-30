# Agent Middleware Developer Guide

Process messages before sending to agents and responses after receiving them.

## Quick Start

```rust
use crate::app::agent_framework::{AgentInstance, AgentType, AgentMetadata};
use crate::app::agent_framework::middleware::layers::LoggingLayer;

// Create agent with middleware
let agent = AgentInstance::new(metadata, AgentType::TaskManager)
    .with_logging_layer();

// Or add layers after creation
let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);
agent.add_layer(LoggingLayer::with_defaults());
```

## How It Works

```
User Message
     |
     v
[LayerStack.process_pre_send()]  <-- Layers can modify or abort
     |
     v
   Agent Execution (Stood)
     |
     v
[LayerStack.process_post_response()]  <-- Layers can modify, suppress, or inject
     |
     v
Display to User
```

**Pre-send**: Layers process in order (first added = first called)
**Post-response**: Layers process in reverse order (last added = first called)

## Creating a Custom Layer

Implement the `ConversationLayer` trait:

```rust
use crate::app::agent_framework::middleware::{
    ConversationLayer, LayerContext, LayerResult, PostResponseAction,
};

pub struct MyCustomLayer {
    // your fields
}

impl ConversationLayer for MyCustomLayer {
    fn name(&self) -> &str {
        "MyCustom"
    }

    fn on_pre_send(&self, message: &str, ctx: &LayerContext) -> LayerResult<String> {
        // Modify message before sending
        Ok(format!("[PREFIX] {}", message))
    }

    fn on_post_response(
        &self,
        response: &str,
        ctx: &LayerContext,
    ) -> LayerResult<PostResponseAction> {
        // React to responses
        if response.contains("error") {
            Ok(PostResponseAction::InjectFollowUp(
                "Please try again with a different approach.".to_string()
            ))
        } else {
            Ok(PostResponseAction::PassThrough)
        }
    }
}
```

## LayerContext - Available Information

```rust
ctx.agent_id        // String - unique agent identifier
ctx.agent_type      // AgentType - TaskManager or TaskWorker
ctx.token_count     // usize - estimated total tokens
ctx.turn_count      // usize - conversation turns
ctx.message_count   // usize - total messages
ctx.last_tool       // Option<String> - last tool executed
ctx.last_tool_success  // bool - whether it succeeded
```

## PostResponseAction Options

| Action | Effect |
|--------|--------|
| `PassThrough` | Show response unchanged |
| `Modify(String)` | Replace response text |
| `InjectFollowUp(String)` | Queue a follow-up message |
| `SuppressAndInject(String)` | Hide response, send new message |

## Built-in Layers

### LoggingLayer
Logs all message flow for debugging.

```rust
agent.add_layer(LoggingLayer::with_defaults());

// Or configure log level
agent.add_layer(LoggingLayer::new(LogLevel::Full));
```

### TokenTrackingLayer
Monitors token usage across the conversation.

```rust
agent.add_layer(TokenTrackingLayer::with_defaults()); // 100k threshold

// Or configure threshold
use crate::app::agent_framework::middleware::layers::TokenTrackingConfig;
agent.add_layer(TokenTrackingLayer::new(
    TokenTrackingConfig::default().with_threshold(50_000)
));
```

### AutoAnalysisLayer
Automatically triggers analysis follow-ups.

```rust
agent.add_layer(AutoAnalysisLayer::new());
```

## Use Cases

### 1. Add Context to Every Message

```rust
impl ConversationLayer for ContextLayer {
    fn on_pre_send(&self, message: &str, _ctx: &LayerContext) -> LayerResult<String> {
        Ok(format!(
            "[Current time: {}]\n{}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M"),
            message
        ))
    }
}
```

### 2. Auto-Summarize Long Responses

```rust
fn on_post_response(&self, response: &str, ctx: &LayerContext)
    -> LayerResult<PostResponseAction>
{
    if response.len() > 5000 {
        Ok(PostResponseAction::InjectFollowUp(
            "Please provide a brief summary of the above.".to_string()
        ))
    } else {
        Ok(PostResponseAction::PassThrough)
    }
}
```

### 3. Filter Sensitive Information

```rust
fn on_post_response(&self, response: &str, _ctx: &LayerContext)
    -> LayerResult<PostResponseAction>
{
    let filtered = response
        .replace("secret", "[REDACTED]")
        .replace("password", "[REDACTED]");

    if filtered != response {
        Ok(PostResponseAction::Modify(filtered))
    } else {
        Ok(PostResponseAction::PassThrough)
    }
}
```

### 4. Abort on Specific Conditions

```rust
fn on_pre_send(&self, message: &str, ctx: &LayerContext) -> LayerResult<String> {
    if ctx.token_count > 150_000 {
        Err(LayerError::Abort("Token limit exceeded".to_string()))
    } else {
        Ok(message.to_string())
    }
}
```

## Convenience Methods

```rust
// Single logging layer
let agent = AgentInstance::new(metadata, agent_type)
    .with_logging_layer();

// Recommended production layers (Logging + TokenTracking)
let agent = AgentInstance::new(metadata, agent_type)
    .with_recommended_layers();
```

## Key Files

| File | Purpose |
|------|---------|
| `middleware/mod.rs` | `ConversationLayer` trait, `PostResponseAction` |
| `middleware/context.rs` | `LayerContext` definition |
| `middleware/stack.rs` | `LayerStack` - manages layer ordering |
| `middleware/layers/` | Built-in layer implementations |
| `agent_instance.rs` | Integration point (`add_layer`, etc.) |

## Testing Your Layer

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_layer_modifies_message() {
        let layer = MyCustomLayer::new();
        let ctx = LayerContext::default();

        let result = layer.on_pre_send("hello", &ctx).unwrap();
        assert_eq!(result, "[PREFIX] hello");
    }
}
```

## Middleware vs Stood Evaluation Strategies

The stood library provides **evaluation strategies** that control agent continuation. Our middleware operates at a different layer:

| Aspect | Our Middleware | Stood Evaluation |
|--------|----------------|------------------|
| **When** | Before/after each message | Between execution cycles |
| **Controls** | Message content | Whether agent continues |
| **Scope** | Per-message processing | Multi-cycle orchestration |
| **Location** | App layer (aws-dash) | Library layer (stood) |

### Stood Evaluation Strategies

```rust
// Model-Driven (default): Agent decides naturally when done
Agent::builder().model(model).build()

// Task Evaluation: Multi-cycle until user intent satisfied
Agent::builder()
    .with_task_evaluation("Is the user's request fully addressed?")
    .build()

// Agent-Based: Separate evaluator agent assesses completion
Agent::builder()
    .with_agent_based_evaluation(evaluator_agent)
    .build()
```

### When to Use Each

| Use Case | Solution |
|----------|----------|
| Add context/metadata to messages | Middleware (pre-send) |
| Filter/modify responses | Middleware (post-response) |
| Auto-inject follow-up questions | Middleware (InjectFollowUp) |
| Control multi-step task completion | Stood Task Evaluation |
| Quality gate before finishing | Stood Agent-Based Evaluation |
| Track tokens/conversation metrics | Middleware (TokenTrackingLayer) |

### Using Both Together

```rust
// Middleware handles message processing
let agent = AgentInstance::new(metadata, AgentType::TaskManager)
    .with_logging_layer();  // Logs all messages

// Stood evaluation handles continuation (configured in create_agent)
// Agent will keep executing until task evaluation passes
```

Middleware and evaluation are complementary: middleware shapes individual messages, evaluation orchestrates the overall agent loop.
