# Agent Feedback Engines - Complete Implementation Plan

## Overview

Three complementary systems for the AWS Dash agent framework:

1. **Message Injection Engine** - Programmatic message injection at app level + middleware hooks
2. **Processing Status Display Engine** - Animated UI with whimsical + informative status messages
3. **Stood Library Enhancements** - Cancellation support + ToolMiddleware for deep interception

---

## Architecture Comparison: Where Things Live

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           APP LAYER (aws-dash)                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  Message Injection Engine                                        │   │
│  │  - Programmatic execute() calls (no user input needed)           │   │
│  │  - Pre/post processing via ConversationLayer middleware          │   │
│  │  - StatusUpdate channel for UI feedback                          │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                  │                                      │
│                                  ▼                                      │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  agent.execute(message)  ←── Can inject messages programmatically│   │
│  │  tokio::select! { ... }  ←── Can race against CancellationToken  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         STOOD LIBRARY LAYER                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  CancellationToken Integration (NEW)                             │   │
│  │  - Check token.is_cancelled() between phases                     │   │
│  │  - Abort cleanly with Err(Cancelled)                             │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                  │                                      │
│                                  ▼                                      │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │  EventLoop (5-phase execution)                                   │   │
│  │  Phase 1: Reasoning    ──┐                                       │   │
│  │  Phase 2: Model Call     │  StreamCallback (observe-only)        │   │
│  │  Phase 3: Tool Execute ──┼─ ToolMiddleware (NEW - can intercept) │   │
│  │  Phase 4: Reflection     │                                       │   │
│  │  Phase 5: Evaluation   ──┘                                       │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Part 1: App-Level Message Injection (No Stood Changes)

### Key Insight

We don't need user input to send messages to an agent. We can call `agent.execute()` programmatically:

```rust
// Current: Only triggered by user input in UI
pub fn send_message(&mut self, message: String) {
    // ... spawns thread, calls agent.execute(message)
}

// NEW: Programmatic injection
pub fn inject_message(&mut self, message: String, injection_type: InjectionType) {
    // Same flow, but triggered by code, not user
    // Can be: SystemContext, ToolFollowUp, MemorySummary, etc.
}
```

### Message Injection Types

```rust
pub enum InjectionType {
    /// System context to prepend to next user message
    SystemContext(String),

    /// Follow-up after tool execution (e.g., "Now analyze these results")
    ToolFollowUp { tool_name: String, context: String },

    /// Memory/summary injection for long conversations
    MemorySummary(String),

    /// Redirect/correction (e.g., "Actually, focus on X instead")
    Correction(String),
}
```

### When to Inject (Examples)

| Trigger | Injection | Use Case |
|---------|-----------|----------|
| After JavaScript tool returns | "Analyze these {n} resources and summarize" | Auto-analysis |
| Token count > threshold | "[Previous context summarized: ...]" | Context management |
| Tool returns error | "The tool failed with: {error}. Try alternative approach." | Error recovery |
| Worker agent completes | "[Worker result: ...] Now continue with next step." | Multi-agent coordination |

### Implementation

**New file: `src/app/agent_framework/message_injection.rs`**

```rust
/// Message injection coordinator
pub struct MessageInjector {
    pending_injections: VecDeque<PendingInjection>,
}

pub struct PendingInjection {
    injection_type: InjectionType,
    message: String,
    trigger: InjectionTrigger,
}

pub enum InjectionTrigger {
    Immediate,                           // Inject now
    AfterToolComplete(String),           // After specific tool
    AfterResponse,                       // After next LLM response
    OnTokenThreshold(usize),             // When tokens exceed N
}

impl MessageInjector {
    /// Queue an injection for later
    pub fn queue(&mut self, injection: PendingInjection);

    /// Check if any injections should fire, return message to inject
    pub fn check_triggers(&mut self, context: &InjectionContext) -> Option<String>;
}
```

**Integration with AgentInstance:**

```rust
// In agent_instance.rs
impl AgentInstance {
    /// Inject a message programmatically (not from user)
    pub fn inject_message(&mut self, message: String) {
        // Reuse send_message infrastructure but mark as system-injected
        self.send_message_internal(message, MessageSource::Injected);
    }

    /// Check for pending injections after each response
    fn check_pending_injections(&mut self) {
        if let Some(injection) = self.injector.check_triggers(&self.context()) {
            self.inject_message(injection);
        }
    }
}
```

---

## Part 2: App-Level Middleware (ConversationLayer)

### Purpose

Process messages before/after Stood execution without modifying Stood:

```rust
pub trait ConversationLayer: Send + Sync {
    /// Modify message before sending to Stood
    fn on_pre_send(&self, message: &str, ctx: &LayerContext) -> LayerResult<String>;

    /// Process response after Stood returns (before displaying to user)
    fn on_post_response(&self, response: &str, ctx: &LayerContext) -> LayerResult<PostResponseAction>;
}

pub enum PostResponseAction {
    PassThrough,                    // Show response as-is
    Modify(String),                 // Change the response text
    InjectFollowUp(String),         // Queue another message to send
    SuppressAndInject(String),      // Don't show this response, inject new message
}
```

### Example Layers

**TokenTrackingLayer:**
```rust
impl ConversationLayer for TokenTrackingLayer {
    fn on_pre_send(&self, message: &str, ctx: &LayerContext) -> LayerResult<String> {
        let tokens = estimate_tokens(message);
        ctx.send_status_update(format!("Sending ~{} tokens", tokens));

        // If conversation too long, inject summary
        if ctx.total_tokens > 100_000 {
            let summary = self.summarize_old_messages(ctx);
            return Ok(format!("[Context: {}]\n\n{}", summary, message));
        }
        Ok(message.to_string())
    }
}
```

**AutoAnalysisLayer:**
```rust
impl ConversationLayer for AutoAnalysisLayer {
    fn on_post_response(&self, response: &str, ctx: &LayerContext) -> LayerResult<PostResponseAction> {
        // If response contains raw data, queue analysis follow-up
        if response.contains("resources found") && !response.contains("Summary:") {
            return Ok(PostResponseAction::InjectFollowUp(
                "Now provide a brief summary of these resources.".to_string()
            ));
        }
        Ok(PostResponseAction::PassThrough)
    }
}
```

### New Files

```
src/app/agent_framework/middleware/
  mod.rs              - ConversationLayer trait, LayerContext
  stack.rs            - LayerStack (ordered processing)
  context.rs          - Thread-local layer stack storage
  layers/
    mod.rs            - Built-in layer exports
    token_tracking.rs - Token counting and context management
    auto_analysis.rs  - Automatic follow-up injection
```

---

## Part 3: Stood Library Enhancements

**NOTE: Before implementing Phase D and E, notify user to pull Stood changes from other workstream.**

### 3A: CancellationToken Support

**Problem:** Current cancellation infrastructure exists in aws-dash but isn't wired to Stood.

**Solution:** Add cancellation token support to Stood's Agent and EventLoop.

```rust
// stood/src/agent/mod.rs
impl Agent {
    /// Set a cancellation token for cooperative cancellation
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }
}

// stood/src/agent/event_loop.rs
impl EventLoop {
    async fn execute(&mut self, prompt: String) -> Result<AgentResult> {
        loop {
            // Check cancellation at start of each cycle
            self.check_cancelled()?;

            // Phase 1: Reasoning
            self.check_cancelled()?;

            // Phase 2: Model call
            let response = self.call_model().await?;
            self.check_cancelled()?;

            // Phase 3: Tool execution
            for tool_call in response.tool_calls {
                self.check_cancelled()?;  // Check before each tool
                self.execute_tool(tool_call).await?;
            }

            // Phase 4-5: Reflection & Evaluation
            self.check_cancelled()?;
            if self.should_stop() { break; }
        }
        Ok(result)
    }

    fn check_cancelled(&self) -> Result<()> {
        if let Some(token) = &self.cancellation_token {
            if token.is_cancelled() {
                return Err(StoodError::Cancelled);
            }
        }
        Ok(())
    }
}
```

**Integration in aws-dash:**

```rust
// agent_instance.rs - Wire up the token
let cancellation_token = cancellation_manager.create_token(agent_id.clone());

let agent = Agent::builder()
    .model(Bedrock::ClaudeSonnet45)
    .with_cancellation_token(cancellation_token.clone())  // NEW
    .tools(tools)
    .build()
    .await?;

// Use select! to race execution against cancellation
tokio::select! {
    result = agent.execute(&message) => {
        // Normal completion
    }
    _ = cancellation_token.cancelled() => {
        // User pressed Stop - Stood also stopped internally
        return Err("Cancelled by user".into());
    }
}
```

### 3B: ToolMiddleware Trait

**Problem:** StreamCallback is observe-only. Cannot intercept/modify/abort tool calls.

**Solution:** Add ToolMiddleware trait for tool-level interception.

```rust
// stood/src/tools/middleware.rs (NEW)

/// Middleware for intercepting tool execution
pub trait ToolMiddleware: Send + Sync {
    /// Called before tool execution - can modify params or abort
    async fn before_tool(
        &self,
        tool_name: &str,
        params: &Value,
        ctx: &ToolContext,
    ) -> ToolMiddlewareAction;

    /// Called after tool execution - can modify result or inject messages
    async fn after_tool(
        &self,
        tool_name: &str,
        result: &ToolResult,
        ctx: &ToolContext,
    ) -> AfterToolAction;
}

pub enum ToolMiddlewareAction {
    /// Continue with original parameters
    Continue,
    /// Continue with modified parameters
    ModifyParams(Value),
    /// Abort this tool call with synthetic result
    Abort { reason: String, synthetic_result: Option<ToolResult> },
    /// Skip this tool entirely (no result added to conversation)
    Skip,
}

pub enum AfterToolAction {
    /// Pass result through unchanged
    PassThrough,
    /// Modify the result before adding to conversation
    ModifyResult(ToolResult),
    /// Inject additional context after the result
    InjectContext(String),
}
```

---

## Part 4: Processing Status Display Engine

### Architecture

```
AgentInstance → StatusUpdate channel → poll_response() → ProcessingStatusWidget → UI
                     ↑
              Phase changes sent from:
              - App middleware (token counts)
              - Stood ToolMiddleware (tool timing)
              - Agent execution phases
```

### Animation Types (egui 0.32.3 Painter API)

#### Orbital Dots (Thinking phase)
```rust
for i in 0..3 {
    let angle = phase + (i as f32 * TAU / 3.0);
    let pos = center + Vec2::angled(angle) * radius;
    painter.circle_filled(pos, dot_radius, color);
}
```

#### Wave Bars (Tool Execution phase)
```rust
for i in 0..5 {
    let height = base_height + (phase + i as f32 * 0.3).sin() * amplitude;
    let rect = Rect::from_min_size(bar_pos, Vec2::new(bar_width, height));
    painter.rect_filled(rect, 0.0, color);
}
```

### Phase-Based Animation Switching

| Phase | Animation | Rationale |
|-------|-----------|-----------|
| Thinking | Orbital Dots | Contemplative feel |
| ExecutingTool | Wave Bars | Active processing |
| AnalyzingResults | Orbital Dots (slower) | Consolidating |

### Whimsical Messages + Details

```rust
let messages = match phase {
    Thinking => vec![
        "Pondering possibilities",
        "Cogitating carefully",
        "Musing methodically",
    ],
    ExecutingTool(name) => vec![
        format!("Consulting the oracle ({})", name),
        format!("Summoning data via {}", name),
    ],
    AnalyzingResults => vec![
        "Distilling wisdom",
        "Synthesizing insights",
    ],
};

// Combine with details
format!("{}... ({})", random_message, detail)
// e.g., "Pondering possibilities... (2,500 tokens)"
```

### New Files

```
src/app/agent_framework/status_display/
  mod.rs              - Module exports
  messages.rs         - StatusMessageGenerator
  animation.rs        - ProcessingAnimation (OrbitalDots, WaveBars)
  widget.rs           - ProcessingStatusWidget
```

---

## Implementation Phases

### Phase A: Status Display (Quick Win) - NO STOOD CHANGES
1. Create `status_display/` module
2. Implement animations with egui Painter
3. Wire StatusUpdate channel
4. Replace "Processing..." in agent_ui.rs

### Phase B: App-Level Message Injection - NO STOOD CHANGES
1. Create `message_injection.rs`
2. Add `inject_message()` to AgentInstance
3. Create injection triggers system
4. Test with auto-analysis use case

### Phase C: App-Level Middleware - NO STOOD CHANGES
1. Create `middleware/` module
2. Implement ConversationLayer trait
3. Add TokenTrackingLayer
4. Integrate with agent execution

### Phase D: Stood Cancellation (Library Change) - NOTIFY USER FIRST
1. Add `cancellation_token` to Stood Agent
2. Add `check_cancelled()` to EventLoop
3. Wire aws-dash cancellation manager
4. Test Stop button functionality

### Phase E: Stood ToolMiddleware (Library Change) - NOTIFY USER FIRST
1. Create `ToolMiddleware` trait in Stood
2. Add middleware hooks to EventLoop
3. Create PerformanceMiddleware example
4. Test tool interception

---

## Files Summary

### New Files (aws-dash)
```
src/app/agent_framework/
  message_injection.rs     - Programmatic message injection
  middleware/
    mod.rs                 - ConversationLayer trait
    stack.rs               - Layer ordering
    context.rs             - Thread-local context
    layers/
      mod.rs
      token_tracking.rs
      auto_analysis.rs
  status_display/
    mod.rs
    messages.rs
    animation.rs
    widget.rs
```

### Modified Files (aws-dash)
```
src/app/agent_framework/
  mod.rs                   - Add new module exports
  agent_instance.rs        - Add inject_message(), middleware integration
  agent_ui.rs              - Replace Processing... with widget
src/app/dashui/
  agent_manager_window.rs  - Add widget state
```

### New/Modified Files (Stood Library)
```
stood/src/
  agent/
    mod.rs                 - Add with_cancellation_token()
    event_loop.rs          - Add check_cancelled(), tool middleware hooks
  tools/
    middleware.rs          - NEW: ToolMiddleware trait
  error.rs                 - Add Cancelled variant
```

---

## Capability Matrix

| Capability | App Injection | App Middleware | Stood Cancel | Stood ToolMW |
|------------|---------------|----------------|--------------|--------------|
| Inject follow-up messages | Yes | Yes | No | No |
| Modify outgoing message | No | Yes | No | No |
| Stop execution | No | No | Yes | No |
| Intercept tool calls | No | No | No | Yes |
| Modify tool params | No | No | No | Yes |
| Cache tool results | No | No | No | Yes |
| Track token usage | No | Yes | No | No |
| Auto-analysis | Yes | Yes | No | No |

---

## Testing Strategy

### Unit Tests
- StatusMessageGenerator: All phases generate valid messages
- ProcessingAnimation: Phase calculation, color pulsing
- LayerStack: Layer ordering, result accumulation
- MessageInjector: Trigger conditions, queue management

### Integration Tests
- Cancellation: Stop button actually stops Stood execution
- ToolMiddleware: Tool calls are intercepted correctly
- Message injection: Follow-ups are sent automatically
- StatusUpdate: UI receives and displays updates

---

## Migration Notes

### Stood Library Changes
- CancellationToken is additive (backward compatible)
- ToolMiddleware is optional (None = no interception)
- Existing StreamCallback unchanged (observe-only)

### aws-dash Changes
- All new functionality in new files
- Existing agent execution path unchanged if middleware = None
- Feature flag ready: `#[cfg(feature = "agent_feedback")]`
