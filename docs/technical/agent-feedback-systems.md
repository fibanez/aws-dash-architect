# Agent Feedback Systems

Interactive feedback systems that enhance agent communication through animated status displays, programmatic message injection, and conversation middleware.

## Overview

The Agent Feedback Systems provide three complementary capabilities:

1. **Processing Status Display**: Animated UI with whimsical status messages during agent processing
2. **Message Injection Engine**: Programmatic injection of messages without user input
3. **Conversation Middleware**: Layered processing of messages before/after agent execution

These systems work together to create a more responsive and informative agent interaction experience.

## Architecture

```
User Input
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│                    AgentInstance                             │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │ MessageInjector │    │      LayerStack                  │ │
│  │  - Queued msgs  │    │  ┌─────────────────────────────┐│ │
│  │  - Triggers     │    │  │ TokenTrackingLayer          ││ │
│  └────────┬────────┘    │  │ AutoAnalysisLayer           ││ │
│           │             │  │ LoggingLayer                ││ │
│           ▼             │  └─────────────────────────────┘│ │
│  ┌─────────────────┐    └─────────────────────────────────┘ │
│  │ ProcessingPhase │                                        │
│  │  - Thinking     │                                        │
│  │  - ExecutingTool│──────▶ StatusUpdate channel            │
│  │  - Analyzing    │                                        │
│  └─────────────────┘                                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│               ProcessingStatusWidget                         │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │ Animation       │    │ StatusMessageGenerator          │ │
│  │  - OrbitalDots  │    │  - 61 thinking messages         │ │
│  │  - WaveBars     │    │  - 58 tool messages             │ │
│  │  - Slower Orbit │    │  - 55 analysis messages         │ │
│  └─────────────────┘    └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Processing Status Display

The status display system provides animated visual feedback during agent processing phases.

### Processing Phases

```rust
pub enum ProcessingPhase {
    Thinking,                    // Model is reasoning
    ExecutingTool(String),       // Tool execution in progress
    AnalyzingResults,            // Processing tool output
    Idle,                        // Not processing
}
```

### Animation Types

| Phase | Animation | Description |
|-------|-----------|-------------|
| Thinking | Orbital Dots | Three dots rotating in a circle |
| ExecutingTool | Wave Bars | Five vertical bars with wave motion |
| AnalyzingResults | Slow Orbital | Orbital dots at reduced speed |
| Idle | None | No animation displayed |

### Whimsical Messages

The system includes 174 rotating status messages inspired by Claude Code:

**Thinking Phase (61 messages)**:
- "Pondering possibilities", "Cogitating carefully", "Reticulating splines"
- "Noodling", "Marinating thoughts", "Brewing ideas"

**Tool Execution Phase (58 messages)**:
- "Consulting the oracle", "Channeling the cloud", "Invoking AWS powers"
- "Wrangling data", "Traversing clouds", "Harvesting data"

**Analysis Phase (55 messages)**:
- "Distilling wisdom", "Synthesizing insights", "Connecting the dots"
- "Assembling the puzzle", "Mapping patterns", "Fusing insights"

Messages rotate every 4 seconds to maintain visual interest during longer operations.

### Usage

The status widget is automatically integrated into the agent UI:

```rust
// In agent_ui.rs
pub fn render_agent_chat(
    ui: &mut Ui,
    agent: &mut AgentInstance,
    status_widget: &mut ProcessingStatusWidget,
) {
    // Widget updates from agent's processing phase
    let phase = agent.processing_phase().clone();
    status_widget.set_phase(phase);

    // Widget handles its own animation and message rotation
    status_widget.show(ui);
}
```

## Message Injection Engine

Programmatic message injection allows the system to send messages to agents without user input.

### Injection Types

```rust
pub enum InjectionType {
    /// System context to prepend
    SystemContext(String),

    /// Follow-up after tool execution
    ToolFollowUp { tool_name: String, context: String },

    /// Memory/summary for long conversations
    MemorySummary(String),

    /// Redirect/correction message
    Correction(String),
}
```

### Injection Triggers

```rust
pub enum InjectionTrigger {
    /// Inject immediately
    Immediate,

    /// Inject after specific tool completes
    AfterToolComplete(String),

    /// Inject after next LLM response
    AfterResponse,

    /// Inject when token count exceeds threshold
    OnTokenThreshold(usize),
}
```

### Usage Examples

**Queue an injection for after the next response:**

```rust
agent.queue_injection(
    InjectionType::ToolFollowUp {
        tool_name: "execute_javascript".into(),
        context: "Summarize these results".into(),
    },
    InjectionTrigger::AfterResponse,
);
```

**Inject immediately:**

```rust
agent.inject_message("Please focus on EC2 instances only.".into());
```

**Check for pending injections:**

```rust
if agent.has_pending_injections() {
    // Handle pending injections
}
```

## Conversation Middleware

The middleware system provides layered processing of messages before and after agent execution.

### ConversationLayer Trait

```rust
pub trait ConversationLayer: Send + Sync {
    /// Layer name for logging
    fn name(&self) -> &str;

    /// Process message before sending to agent
    fn on_pre_send(&self, message: &str, ctx: &LayerContext)
        -> LayerResult<String>;

    /// Process response after receiving from agent
    fn on_post_response(&self, response: &str, ctx: &LayerContext)
        -> LayerResult<PostResponseAction>;

    /// Called when tool execution starts
    fn on_tool_start(&self, tool_name: &str, ctx: &LayerContext);

    /// Called when tool execution completes
    fn on_tool_complete(&self, tool_name: &str, success: bool, ctx: &LayerContext);
}
```

### Layer Context

The `LayerContext` provides information about the current conversation state:

```rust
pub struct LayerContext {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub token_count: usize,
    pub turn_count: usize,
    pub message_count: usize,
    pub last_tool: Option<String>,
    pub last_tool_success: bool,
    pub processing_start: Option<Instant>,
}
```

### Post-Response Actions

```rust
pub enum PostResponseAction {
    /// Pass response through unchanged
    PassThrough,

    /// Modify response before displaying
    Modify(String),

    /// Queue a follow-up message
    InjectFollowUp(String),

    /// Suppress response and inject new message
    SuppressAndInject(String),
}
```

### Built-in Layers

**TokenTrackingLayer**: Monitors token usage and can inject context summaries:

```rust
let layer = TokenTrackingLayer::new()
    .with_warning_threshold(80_000)
    .with_critical_threshold(100_000);
```

**AutoAnalysisLayer**: Automatically triggers analysis follow-ups:

```rust
let layer = AutoAnalysisLayer::new()
    .with_resource_threshold(10)
    .enabled(true);
```

**LoggingLayer**: Logs all conversation flow for debugging:

```rust
let layer = LoggingLayer::new()
    .with_level(LogLevel::Detailed);
```

### Layer Stack

Layers are processed in order for pre-send and reverse order for post-response:

```rust
let mut stack = LayerStack::new();
stack.add(Box::new(LoggingLayer::new()));
stack.add(Box::new(TokenTrackingLayer::with_defaults()));
stack.add(Box::new(AutoAnalysisLayer::with_defaults()));

// Pre-send: Logging → TokenTracking → AutoAnalysis
// Post-response: AutoAnalysis → TokenTracking → Logging
```

## Integration with AgentInstance

The feedback systems are integrated into `AgentInstance`:

```rust
impl AgentInstance {
    // === Processing Status ===
    pub fn processing_phase(&self) -> &ProcessingPhase;

    // === Message Injection ===
    pub fn queue_injection(&mut self, injection_type: InjectionType, trigger: InjectionTrigger);
    pub fn inject_message(&mut self, message: String);
    pub fn has_pending_injections(&self) -> bool;

    // === Middleware ===
    pub fn add_layer<L: ConversationLayer + 'static>(&mut self, layer: L);
    pub fn layer_stack(&self) -> &LayerStack;
    pub fn layer_stack_mut(&mut self) -> &mut LayerStack;
    pub fn with_logging_layer(self) -> Self;         // Builder pattern
    pub fn with_recommended_layers(self) -> Self;    // Logging + TokenTracking

    // === Cancellation ===
    pub fn cancel(&mut self) -> bool;               // Stop current execution
    pub fn can_cancel(&self) -> bool;               // Check if cancellation available
    pub fn is_cancelled(&self) -> bool;             // Check if cancelled
}
```

## Cancellation Support

Stop agent execution via the UI Stop button or programmatically.

### How It Works

The stood library's `CancellationToken` is captured during agent initialization:

```rust
// During initialize(), the token is captured from stood
self.cancel_token = agent.cancellation_token();
```

When cancellation is requested, the token signals the stood EventLoop to stop at the next cycle boundary.

### Usage

**Stop button (UI)**: The Stop button in the agent chat UI is enabled when the agent is processing and cancellation is available. Clicking it calls `agent.cancel()`.

**Programmatic cancellation**:

```rust
// Check if cancellation is available
if agent.can_cancel() {
    // Request cancellation
    agent.cancel();
}

// Check if already cancelled
if agent.is_cancelled() {
    // Handle cancelled state
}
```

### Lifecycle

The cancellation token is:
- Created when the agent is initialized via `initialize()`
- Cleared when `reset_stood_agent()` is called
- Cleared when `clear_conversation()` is called
- Automatically cancelled when `terminate()` is called

### Quick Middleware Setup

```rust
// Option 1: Builder pattern with recommended layers
let agent = AgentInstance::new(metadata, AgentType::TaskManager)
    .with_recommended_layers();

// Option 2: Add layers after creation
let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);
agent.add_layer(LoggingLayer::with_defaults());
agent.add_layer(MyCustomLayer::new());
```

See [Agent Middleware Guide](agent-middleware-guide.md) for detailed middleware documentation.

## Testing

Manual testing procedures are documented in [Agent Feedback Testing Guide](../testing/agent-feedback-testing.md).

### Unit Tests

```bash
# Run all feedback system tests
cargo test -p awsdash status_display
cargo test -p awsdash message_injection
cargo test -p awsdash middleware
```

### Key Test Areas

1. **Status Display**: Phase transitions, message rotation, animation timing
2. **Message Injection**: Trigger conditions, priority handling, message formatting
3. **Middleware**: Layer ordering, context propagation, action handling

## Current Limitations

1. **Wave Bars Animation**: The Stood library does not currently send StatusUpdate messages during tool execution, so wave bars animation only appears when explicitly triggered.

2. **Tool Interception**: Deep tool interception (modifying tool parameters, caching results) requires Stood library ToolMiddleware support.

## Key Files

| File | Purpose |
|------|---------|
| `agent_instance.rs` | AgentInstance with middleware and cancellation integration |
| `agent_ui.rs` | Agent chat UI with Stop button |
| `status_display/messages.rs` | ProcessingPhase enum, StatusMessageGenerator |
| `status_display/animation.rs` | ProcessingAnimation with egui Painter |
| `status_display/widget.rs` | ProcessingStatusWidget for UI integration |
| `message_injection.rs` | MessageInjector, InjectionType, triggers |
| `middleware/mod.rs` | ConversationLayer trait, PostResponseAction |
| `middleware/stack.rs` | LayerStack for ordered processing |
| `middleware/context.rs` | LayerContext for layer state |
| `middleware/layers/` | Built-in layer implementations |

## Related Documentation

- [Agent Middleware Guide](agent-middleware-guide.md) - Developer guide for creating custom middleware layers
- [Agent Framework](agent-framework-v2.md) - Core agent architecture
- [Multi-Agent System](multi-agent-system.md) - Task manager and worker agents
- [Code Execution Tool](code-execution-tool.md) - JavaScript execution for agents
