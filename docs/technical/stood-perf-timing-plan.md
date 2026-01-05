# Stood Library Performance Timing Implementation Plan

This document provides a detailed implementation plan for adding `perf_timing` instrumentation to the stood library for unified performance visibility across the full agent stack.

## Executive Summary

**Goal**: Add debug-only performance timing to stood that writes to the same log file as awsdash, providing full-stack visibility from UI click through agent creation, model invocation, and tool execution.

**Approach**: Option B - Use tracing with custom subscriber that outputs to perf_timing.log

**Why**: The stood library already uses `tracing` extensively. We'll add a custom tracing layer that outputs performance data in the same format as awsdash's perf_timing module, allowing unified timing analysis.

## Current Timing Coverage (awsdash side)

From `src/app/agent_framework/perf_timing.rs`:
- `create_new_agent` - Full agent creation flow
- `AgentInstance::new` - Instance creation
- `create_stood_agent` - Stood agent builder
- `get_bedrock_credentials` - AWS credential retrieval
- `send_message` - Message sending flow
- `agent.execute()` - Opaque box (no visibility inside)

## Missing Visibility (inside stood)

What happens inside `agent.execute()` and `agent_builder.build()` is currently a black box.

## Implementation Plan

### Phase 1: Create perf_timing Feature in Stood

**File: `stood-source/Cargo.toml`**

```toml
[features]
default = []
perf-timing = []  # Enables performance timing output

[dependencies]
# No new dependencies needed - uses existing tracing
```

### Phase 2: Create Timing Module

**File: `stood-source/src/perf_timing.rs`**

```rust
//! Debug-only performance timing for the stood library.
//!
//! When the `perf-timing` feature is enabled, this module outputs timing
//! data to the same perf_timing.log used by awsdash for unified analysis.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use once_cell::sync::Lazy;

/// Global log file handle
static LOG_FILE: Lazy<Mutex<Option<std::fs::File>>> = Lazy::new(|| {
    let log_path = get_log_path();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok();
    Mutex::new(file)
});

fn get_log_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("awsdash")
        .join("logs")
        .join("perf_timing.log")
}

/// Write a timing entry to the log
pub fn log_timing(name: &str, duration_ms: f64, context: Option<&str>) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let ctx = context.map(|c| format!(" [{}]", c)).unwrap_or_default();
            let _ = writeln!(file, "[{}] {} = {:.3}ms{}", timestamp, name, duration_ms, ctx);
            let _ = file.flush();
        }
    }
}

/// Write a checkpoint (marker) to the log
pub fn log_checkpoint(name: &str) {
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] CHECKPOINT: {}", timestamp, name);
            let _ = file.flush();
        }
    }
}

/// RAII timing guard
pub struct TimingGuard {
    name: String,
    start: Instant,
    context: Option<String>,
}

impl TimingGuard {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            context: None,
        }
    }

    pub fn with_context(name: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: Instant::now(),
            context: Some(context.into()),
        }
    }
}

impl Drop for TimingGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        log_timing(&self.name, duration.as_secs_f64() * 1000.0, self.context.as_deref());
    }
}

/// Macro for timing a block of code
#[macro_export]
#[cfg(feature = "perf-timing")]
macro_rules! perf_timed {
    ($name:expr, $expr:expr) => {{
        let _guard = $crate::perf_timing::TimingGuard::new($name);
        $expr
    }};
}

#[macro_export]
#[cfg(not(feature = "perf-timing"))]
macro_rules! perf_timed {
    ($name:expr, $expr:expr) => {
        $expr
    };
}

/// Macro for checkpoints
#[macro_export]
#[cfg(feature = "perf-timing")]
macro_rules! perf_checkpoint {
    ($name:expr) => {
        $crate::perf_timing::log_checkpoint($name);
    };
}

#[macro_export]
#[cfg(not(feature = "perf-timing"))]
macro_rules! perf_checkpoint {
    ($name:expr) => {};
}

/// Macro for creating a timing guard
#[macro_export]
#[cfg(feature = "perf-timing")]
macro_rules! perf_guard {
    ($name:expr) => {
        $crate::perf_timing::TimingGuard::new($name)
    };
    ($name:expr, $context:expr) => {
        $crate::perf_timing::TimingGuard::with_context($name, $context)
    };
}

#[macro_export]
#[cfg(not(feature = "perf-timing"))]
macro_rules! perf_guard {
    ($name:expr) => { () };
    ($name:expr, $context:expr) => { () };
}
```

### Phase 3: Instrument Key Functions

#### 3.1 Agent Builder (`src/agent/mod.rs`)

**Line ~1893 - `build()` function:**

```rust
pub async fn build(mut self) -> Result<Agent> {
    perf_checkpoint!("stood.agent_builder.build.start");

    // Model selection
    let model = perf_timed!("stood.agent_builder.model_selection", {
        self.model.unwrap_or_else(|| Box::new(crate::llm::models::Bedrock::ClaudeHaiku45))
    });

    // ... config updates ...

    // Custom credentials configuration
    if provider_type == ProviderType::Bedrock && self.aws_credentials.is_some() {
        perf_timed!("stood.agent_builder.configure_bedrock_creds", {
            // ... existing credential config code ...
        });
    }

    // Provider configuration check (with timeout)
    let is_configured = perf_timed!("stood.agent_builder.is_configured_check", {
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            PROVIDER_REGISTRY.is_configured(provider_type),
        ).await.unwrap_or(false)
    });

    if !is_configured {
        perf_timed!("stood.agent_builder.auto_configure", {
            // ... existing auto-configure code ...
        });
    }

    // Get provider (THIS IS THE MAIN BOTTLENECK)
    let provider = perf_timed!("stood.agent_builder.get_provider", {
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            PROVIDER_REGISTRY.get_provider(provider_type),
        ).await
        // ... error handling ...
    });

    // Build internal agent
    let agent = perf_timed!("stood.agent_builder.build_internal", {
        Agent::build_internal(/* ... */).await
    });

    perf_checkpoint!("stood.agent_builder.build.end");
    agent
}
```

#### 3.2 Provider Registry (`src/llm/registry.rs`)

**Line ~190 - `get_provider()` function:**

```rust
pub async fn get_provider(&self, provider_type: ProviderType) -> Result<Arc<dyn LlmProvider>, LlmError> {
    perf_checkpoint!("stood.registry.get_provider.start");

    // Cache check
    {
        let _guard = perf_guard!("stood.registry.cache_read");
        let providers = self.providers.read().await;
        if let Some(provider) = providers.get(&provider_type) {
            perf_checkpoint!("stood.registry.cache_hit");
            return Ok(Arc::clone(provider));
        }
    }
    perf_checkpoint!("stood.registry.cache_miss");

    // Provider creation
    let provider: Arc<dyn LlmProvider> = match (provider_type, config) {
        (ProviderType::Bedrock, ProviderConfig::Bedrock { region, credentials }) => {
            perf_timed!("stood.registry.create_bedrock_provider", {
                // ... existing bedrock creation code ...
            })
        }
        // ... other providers ...
    };

    // Cache write
    perf_timed!("stood.registry.cache_write", {
        let mut providers = self.providers.write().await;
        providers.insert(provider_type, Arc::clone(&provider));
    });

    perf_checkpoint!("stood.registry.get_provider.end");
    Ok(provider)
}
```

#### 3.3 Bedrock Provider (`src/llm/providers/bedrock.rs`)

**Line ~112 - `with_credentials()` function:**

```rust
pub async fn with_credentials(
    region: Option<String>,
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
) -> Result<Self, LlmError> {
    perf_checkpoint!("stood.bedrock.with_credentials.start");

    // Credentials creation
    let creds = perf_timed!("stood.bedrock.create_credentials", {
        aws_sdk_bedrockruntime::config::Credentials::new(/* ... */)
    });

    // Config loader setup
    perf_checkpoint!("stood.bedrock.config_loader_setup");
    let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .credentials_provider(creds);

    if let Some(region) = region {
        config_loader = config_loader.region(aws_config::Region::new(region));
    }

    // AWS config load (MAIN BOTTLENECK - ~950ms on first call)
    let aws_config = perf_timed!("stood.bedrock.aws_config_load", {
        config_loader.load().await
    });

    // Client creation
    let client = perf_timed!("stood.bedrock.client_new", {
        BedrockRuntimeClient::new(&aws_config)
    });

    perf_checkpoint!("stood.bedrock.with_credentials.end");
    Ok(Self { client, aws_config, /* ... */ })
}
```

**Line ~1024 - `chat_with_tools()` function:**

```rust
async fn chat_with_tools(
    &self,
    model_id: &str,
    messages: &Messages,
    tools: &[Tool],
    config: &ChatConfig,
) -> Result<ChatResponse, LlmError> {
    let _guard = perf_guard!("stood.bedrock.chat_with_tools", model_id);

    // Build request body
    let request_body = perf_timed!("stood.bedrock.build_request_body", {
        self.build_request_body(messages, model_id, tools, config)?
    });

    // API call (network latency)
    let response = perf_timed!("stood.bedrock.invoke_model", {
        self.client
            .invoke_model()
            .model_id(model_id)
            .body(/* ... */)
            .send()
            .await
    });

    // Response parsing
    let chat_response = perf_timed!("stood.bedrock.parse_response", {
        self.parse_bedrock_response(/* ... */)
    });

    chat_response
}
```

#### 3.4 Event Loop (`src/agent/event_loop.rs`)

**Line ~331 - `execute()` function:**

```rust
pub async fn execute(&mut self, prompt: impl Into<String>) -> Result<EventLoopResult> {
    let prompt = prompt.into();
    perf_checkpoint!("stood.event_loop.execute.start");

    // Telemetry setup
    perf_timed!("stood.event_loop.telemetry_setup", {
        // ... existing telemetry code ...
    });

    // Add user message
    perf_timed!("stood.event_loop.add_user_message", {
        self.agent.add_user_message(&prompt);
    });

    // Main loop
    let mut cycle_count = 0;
    loop {
        cycle_count += 1;
        let cycle_name = format!("stood.event_loop.cycle_{}", cycle_count);
        let _cycle_guard = perf_guard!(cycle_name);

        // Execute cycle
        let cycle_result = perf_timed!(format!("stood.event_loop.cycle_{}.execute", cycle_count), {
            self.execute_cycle_with_prompt_with_context(/* ... */).await?
        });

        if cycle_result.is_complete {
            break;
        }
    }

    perf_checkpoint!("stood.event_loop.execute.end");
    // ... return result ...
}
```

**Line ~695 - `execute_cycle_with_prompt_with_context()` function:**

```rust
async fn execute_cycle_with_prompt_with_context(/* ... */) -> Result<CycleResult> {
    perf_checkpoint!("stood.cycle.start");

    // Chat with tools (model invocation)
    let response = perf_timed!("stood.cycle.chat_with_tools", {
        self.execute_chat_with_tools(tool_config).await?
    });

    // Process tool calls
    if !response.tool_calls.is_empty() {
        for tool_call in &response.tool_calls {
            perf_timed!(format!("stood.cycle.tool.{}", tool_call.name), {
                // ... tool execution ...
            });
        }
    }

    perf_checkpoint!("stood.cycle.end");
    // ... return result ...
}
```

#### 3.5 Tool Executor (`src/tools/executor.rs`)

**`execute_tool()` function:**

```rust
pub async fn execute_tool(
    &self,
    tool: Arc<dyn Tool>,
    tool_use: &ToolUse,
) -> (ToolResult, Option<ToolExecutionMetrics>) {
    let tool_name = tool.name().to_string();
    let _guard = perf_guard!(format!("stood.tool.{}", tool_name));

    // Input validation
    if self.config.validate_inputs {
        perf_timed!(format!("stood.tool.{}.validate", tool_name), {
            // ... validation code ...
        });
    }

    // Tool execution
    let result = perf_timed!(format!("stood.tool.{}.execute", tool_name), {
        tool.execute(Some(tool_use.input.clone())).await
    });

    // ... return result ...
}
```

### Phase 4: Enable in awsdash

**File: `awsdash/Cargo.toml`**

```toml
[dependencies]
stood = { path = "../stood-source", features = ["perf-timing"] }
```

Or for conditional debug-only timing:

```toml
[features]
perf-timing = ["stood/perf-timing"]

[dependencies]
stood = { path = "../stood-source" }
```

### Expected Output

After implementation, the `perf_timing.log` will show the full stack:

```
[2024-01-15 10:30:00.000] ==================== Performance Timing Session ====================
[2024-01-15 10:30:00.001] create_new_agent.start
[2024-01-15 10:30:00.002] AgentInstance.new.start
[2024-01-15 10:30:00.003] create_stood_agent.start
[2024-01-15 10:30:00.004] create_stood_agent.get_bedrock_credentials = 15.234ms
[2024-01-15 10:30:00.020] CHECKPOINT: stood.agent_builder.build.start
[2024-01-15 10:30:00.021] stood.agent_builder.model_selection = 0.012ms
[2024-01-15 10:30:00.021] stood.agent_builder.configure_bedrock_creds = 0.543ms
[2024-01-15 10:30:00.022] stood.agent_builder.is_configured_check = 1.234ms
[2024-01-15 10:30:00.023] CHECKPOINT: stood.registry.get_provider.start
[2024-01-15 10:30:00.024] CHECKPOINT: stood.registry.cache_miss
[2024-01-15 10:30:00.024] CHECKPOINT: stood.bedrock.with_credentials.start
[2024-01-15 10:30:00.025] stood.bedrock.create_credentials = 0.023ms
[2024-01-15 10:30:00.025] CHECKPOINT: stood.bedrock.config_loader_setup
[2024-01-15 10:30:01.010] stood.bedrock.aws_config_load = 985.432ms  <-- BOTTLENECK
[2024-01-15 10:30:01.015] stood.bedrock.client_new = 5.123ms
[2024-01-15 10:30:01.015] CHECKPOINT: stood.bedrock.with_credentials.end
[2024-01-15 10:30:01.016] stood.registry.create_bedrock_provider = 991.234ms
[2024-01-15 10:30:01.017] stood.registry.cache_write = 0.234ms
[2024-01-15 10:30:01.017] CHECKPOINT: stood.registry.get_provider.end
[2024-01-15 10:30:01.018] stood.agent_builder.get_provider = 995.678ms
[2024-01-15 10:30:01.020] stood.agent_builder.build_internal = 2.345ms
[2024-01-15 10:30:01.020] CHECKPOINT: stood.agent_builder.build.end
[2024-01-15 10:30:01.021] create_stood_agent.agent_builder_build = 1001.234ms
[2024-01-15 10:30:01.022] AgentInstance.new = 1020.456ms
[2024-01-15 10:30:01.022] create_new_agent = 1022.789ms

[2024-01-15 10:30:05.000] send_message.start
[2024-01-15 10:30:05.001] CHECKPOINT: stood.event_loop.execute.start
[2024-01-15 10:30:05.002] stood.event_loop.telemetry_setup = 0.456ms
[2024-01-15 10:30:05.003] stood.event_loop.add_user_message = 0.012ms
[2024-01-15 10:30:05.003] CHECKPOINT: stood.cycle.start
[2024-01-15 10:30:05.004] stood.bedrock.build_request_body = 1.234ms
[2024-01-15 10:30:15.500] stood.bedrock.invoke_model = 10496.123ms [model=anthropic.claude-3-5-haiku-20241022-v1:0]
[2024-01-15 10:30:15.505] stood.bedrock.parse_response = 5.234ms
[2024-01-15 10:30:15.506] stood.cycle.chat_with_tools = 10502.567ms
[2024-01-15 10:30:15.507] stood.tool.execute_javascript.validate = 0.123ms
[2024-01-15 10:30:15.607] stood.tool.execute_javascript.execute = 100.456ms
[2024-01-15 10:30:15.608] stood.tool.execute_javascript = 101.234ms
[2024-01-15 10:30:15.608] CHECKPOINT: stood.cycle.end
[2024-01-15 10:30:15.609] stood.event_loop.cycle_1.execute = 10605.789ms
[2024-01-15 10:30:15.609] stood.event_loop.cycle_1 = 10606.123ms
```

## Implementation Steps

1. **Create perf_timing module** in stood-source/src/perf_timing.rs
2. **Add feature flag** to stood-source/Cargo.toml
3. **Export module** in stood-source/src/lib.rs
4. **Instrument agent builder** (src/agent/mod.rs)
5. **Instrument provider registry** (src/llm/registry.rs)
6. **Instrument bedrock provider** (src/llm/providers/bedrock.rs)
7. **Instrument event loop** (src/agent/event_loop.rs)
8. **Instrument tool executor** (src/tools/executor.rs)
9. **Enable feature** in awsdash Cargo.toml
10. **Test** full-stack timing output

## Files to Modify

| File | Changes |
|------|---------|
| `stood-source/Cargo.toml` | Add `perf-timing` feature |
| `stood-source/src/lib.rs` | Export `perf_timing` module |
| `stood-source/src/perf_timing.rs` | **NEW** - Timing module |
| `stood-source/src/agent/mod.rs` | Instrument `build()` |
| `stood-source/src/llm/registry.rs` | Instrument `get_provider()` |
| `stood-source/src/llm/providers/bedrock.rs` | Instrument `with_credentials()`, `chat_with_tools()` |
| `stood-source/src/agent/event_loop.rs` | Instrument `execute()`, `execute_cycle_*()` |
| `stood-source/src/tools/executor.rs` | Instrument `execute_tool()` |
| `awsdash/Cargo.toml` | Enable `perf-timing` feature |

## Estimated Effort

- Phase 1 (Feature flag): 5 minutes
- Phase 2 (Timing module): 30 minutes
- Phase 3 (Instrumentation): 2 hours
- Phase 4 (Integration): 15 minutes
- Testing: 30 minutes

**Total: ~3.5 hours**

## Root Cause Confirmation

The ~1 second delay in agent creation is caused by:
1. **AWS SDK TLS/SSL initialization** on first `aws_config.load()` call (~950ms)
2. HTTP client connection pool setup
3. Credential provider chain resolution

## Recommended Optimizations (after timing instrumentation)

1. **Share BedrockProvider** - Cache and reuse provider instance across agents with same credentials
2. **Pre-warm on startup** - Initialize Bedrock client during app startup (after login)
3. **Connection pooling** - Ensure HTTP connections are reused across requests
4. **Parallel initialization** - Initialize provider while doing other setup work
