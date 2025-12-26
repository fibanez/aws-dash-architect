# Implementation Plan: Enhanced Agent Logging with Per-Agent Stood Traces

## Overview

This plan addresses the need to capture Stood library debug information directly in per-agent log files instead of the global awsdash.log, add a logging level dropdown (replacing the checkbox), and fix model ID handling to use proper enums.

## Current State Analysis

### Problems Identified

1. **Stood traces go to global log**: All `stood::*` traces currently go to `~/.local/share/awsdash/logs/awsdash.log` instead of per-agent logs
2. **Checkbox is binary**: Only ON/OFF (trace vs off), no Info/Debug/Trace levels
3. **Model ID hardcoded**: In `agent_instance.rs:339`, model is hardcoded as `Bedrock::Claude35Sonnet` instead of using `self.metadata.model_id`
4. **No agent reset on log level change**: Changing the debug checkbox doesn't reset the agent
5. **Post-hoc trace extraction**: `agent_debug_logger.rs` tries to extract traces after execution, which is inefficient

### Current Files Involved

| File | Purpose |
|------|---------|
| `src/lib.rs:77-105` | Global tracing reload handle and `toggle_stood_traces()` |
| `src/app/dashui/agent_manager_window.rs:228-238` | "Stood Debug" checkbox UI |
| `src/app/agent_framework/agent_instance.rs:291-354` | Agent creation with hardcoded model |
| `src/app/agent_framework/agent_logger.rs` | Per-agent logging (doesn't capture stood traces) |
| `src/app/agent_framework/agent_debug_logger.rs` | Post-hoc trace extraction (to be replaced) |
| `src/app/agent_framework/model_config.rs` | Model configs and `create_agent_with_model!` macro |

---

## Implementation Plan

### Task 1: Create StoodLogLevel Enum and Per-Agent Trace Capture

**Files to modify:**
- `src/app/agent_framework/agent_types.rs` (add enum)
- `src/app/agent_framework/agent_instance.rs` (add log level field)

**Changes:**

1. Add `StoodLogLevel` enum to `agent_types.rs`:
```rust
/// Logging level for Stood library traces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoodLogLevel {
    /// No stood traces
    Off,
    /// Info level - high-level agent events
    Info,
    /// Debug level - detailed agent operations (default)
    #[default]
    Debug,
    /// Trace level - all internal operations
    Trace,
}

impl StoodLogLevel {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            StoodLogLevel::Off => "Off",
            StoodLogLevel::Info => "Info",
            StoodLogLevel::Debug => "Debug",
            StoodLogLevel::Trace => "Trace",
        }
    }

    /// Get all levels for dropdown
    pub fn all() -> &'static [StoodLogLevel] {
        &[StoodLogLevel::Off, StoodLogLevel::Info,
          StoodLogLevel::Debug, StoodLogLevel::Trace]
    }
}
```

2. Add `stood_log_level: StoodLogLevel` field to `AgentInstance`

---

### Task 2: Create Custom Tracing Subscriber for Per-Agent Logs

**Files to create/modify:**
- `src/app/agent_framework/agent_tracing.rs` (NEW)
- `src/app/agent_framework/mod.rs` (add module)

**Approach:**

Create a custom `tracing::Subscriber` layer that:
1. Captures `stood::*` events during agent execution
2. Routes them to the current agent's log file via thread-local storage
3. Respects the per-agent log level setting

```rust
// Pseudocode for agent_tracing.rs
pub struct AgentTracingLayer {
    // Uses thread-local to find current agent's logger
}

impl<S: Subscriber> Layer<S> for AgentTracingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Check if event target starts with "stood::"
        if event.metadata().target().starts_with("stood::") {
            // Get current agent's logger from thread-local
            if let Some(logger) = get_current_agent_logger() {
                // Check if event level meets agent's log level threshold
                if should_log(event.metadata().level(), get_current_log_level()) {
                    // Format and write to agent log
                    logger.log_stood_trace(format_event(event));
                }
            }
        }
    }
}
```

**Thread-local storage additions to `tool_context.rs`:**
```rust
thread_local! {
    static CURRENT_AGENT_LOGGER: RefCell<Option<Arc<AgentLogger>>> = RefCell::new(None);
    static CURRENT_LOG_LEVEL: Cell<StoodLogLevel> = Cell::new(StoodLogLevel::Debug);
}

pub fn set_current_agent_logger(logger: Arc<AgentLogger>) { ... }
pub fn get_current_agent_logger() -> Option<Arc<AgentLogger>> { ... }
pub fn set_current_log_level(level: StoodLogLevel) { ... }
pub fn get_current_log_level() -> StoodLogLevel { ... }
```

---

### Task 3: Update AgentLogger to Handle Stood Traces

**File to modify:**
- `src/app/agent_framework/agent_logger.rs`

**Add new method:**
```rust
impl AgentLogger {
    /// Log a stood library trace message
    pub fn log_stood_trace(&self, level: &str, target: &str, message: &str) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let formatted = format!(
            "[{}] [STOOD] [{}] {}: {}",
            timestamp, level, target, message
        );
        self.write_line(&formatted);
    }
}
```

---

### Task 4: Replace Checkbox with Dropdown in UI

**File to modify:**
- `src/app/dashui/agent_manager_window.rs`

**Changes:**

1. Replace `stood_traces_enabled: bool` field with `stood_log_level: StoodLogLevel`

2. Replace checkbox UI (lines 228-238) with ComboBox:
```rust
// Stood log level dropdown
ui.horizontal(|ui| {
    ui.label("Stood Logging:");
    let current_display = self.stood_log_level.display_name();
    egui::ComboBox::from_id_salt("stood_log_level")
        .selected_text(current_display)
        .show_ui(ui, |ui| {
            for level in StoodLogLevel::all() {
                let is_selected = *level == self.stood_log_level;
                if ui.selectable_label(is_selected, level.display_name()).clicked() {
                    let old_level = self.stood_log_level;
                    self.stood_log_level = *level;

                    // Update global tracing filter
                    update_stood_tracing_level(*level);

                    // Reset current agent if level changed
                    if old_level != *level {
                        if let Some(agent) = self.get_current_agent_mut() {
                            agent.set_log_level(*level);
                            // Clear agent to force re-initialization
                            agent.reset_stood_agent();
                        }
                    }

                    tracing::info!("Stood log level changed to: {:?}", level);
                }
            }
        });
});
```

---

### Task 5: Update Global Tracing Filter Function

**File to modify:**
- `src/lib.rs`

**Replace `toggle_stood_traces()` with `set_stood_log_level()`:**
```rust
/// Set stood tracing level dynamically
pub fn set_stood_log_level(level: StoodLogLevel) {
    if let Some(handle) = TRACING_RELOAD_HANDLE.lock().unwrap().as_ref() {
        let stood_level = match level {
            StoodLogLevel::Off => "off",
            StoodLogLevel::Info => "info",
            StoodLogLevel::Debug => "debug",
            StoodLogLevel::Trace => "trace",
        };

        let new_filter = format!(
            "awsdash=trace,stood={},aws_sdk_cloudformation=trace,...",
            stood_level
        );

        if let Ok(filter) = tracing_subscriber::EnvFilter::builder().parse(&new_filter) {
            if let Err(e) = handle.reload(filter) {
                eprintln!("Failed to reload tracing filter: {}", e);
            }
        }
    }
}
```

---

### Task 6: Fix Model ID Handling (Not Hardcoded)

**File to modify:**
- `src/app/agent_framework/agent_instance.rs`

**Change in `create_stood_agent()` (around line 338-343):**

Replace:
```rust
let agent_builder = Agent::builder()
    .model(Bedrock::Claude35Sonnet)  // HARDCODED - BUG
    .system_prompt(&system_prompt)
    ...
```

With:
```rust
use crate::create_agent_with_model;

let base_builder = Agent::builder()
    .system_prompt(&system_prompt)
    .with_streaming(false)
    .with_credentials(access_key, secret_key, session_token, region)
    .tools(self.get_tools_for_type());

// Use macro to set correct model based on model_id
let agent_builder = create_agent_with_model!(base_builder, self.metadata.model_id);
```

---

### Task 7: Add Agent Reset on Log Level Change

**File to modify:**
- `src/app/agent_framework/agent_instance.rs`

**Add methods:**
```rust
impl AgentInstance {
    /// Set the stood log level for this agent
    pub fn set_log_level(&mut self, level: StoodLogLevel) {
        self.stood_log_level = level;
        self.logger.log_system_message(
            &self.agent_type,
            &format!("Stood log level changed to: {:?}", level)
        );
    }

    /// Reset the stood agent (will be recreated on next message)
    pub fn reset_stood_agent(&mut self) {
        *self.stood_agent.lock().unwrap() = None;
        self.logger.log_system_message(
            &self.agent_type,
            "Agent reset - will reinitialize on next message"
        );
    }
}
```

---

### Task 8: Add Model Change Logging

**File to modify:**
- `src/app/agent_framework/agent_instance.rs`

**Enhance `change_model()` method (already exists at line 587):**
```rust
pub fn change_model(&mut self, new_model_id: String) {
    let old_model = self.metadata.model_id.clone();

    // Validate model ID exists in config
    let models = ModelConfig::default_models();
    let new_model_display = ModelConfig::get_display_name(&models, &new_model_id);
    let old_model_display = ModelConfig::get_display_name(&models, &old_model);

    self.metadata.model_id = new_model_id.clone();
    self.metadata.updated_at = chrono::Utc::now();

    // Log model change with display names
    self.logger.log_model_changed(&self.agent_type, &old_model, &new_model_id);
    self.logger.log_system_message(
        &self.agent_type,
        &format!("Model changed: {} -> {}", old_model_display, new_model_display)
    );

    // Clear stood agent - will be re-created with new model on next message
    *self.stood_agent.lock().unwrap() = None;
}
```

---

### Task 9: Update Thread-Local Context in send_message()

**File to modify:**
- `src/app/agent_framework/agent_instance.rs`

**In `send_message()` background thread (around line 400):**
```rust
std::thread::spawn(move || {
    // Set thread-local context for this agent
    crate::app::agent_framework::set_current_agent_id(agent_id);
    crate::app::agent_framework::set_current_agent_type(agent_type);

    // NEW: Set logger and log level for stood trace capture
    crate::app::agent_framework::set_current_agent_logger(Arc::clone(&logger));
    crate::app::agent_framework::set_current_log_level(stood_log_level);

    // ... rest of execution
});
```

---

### Task 10: Remove/Deprecate agent_debug_logger.rs

**File to modify:**
- `src/app/agent_framework/agent_debug_logger.rs`
- `src/app/agent_framework/mod.rs`

The post-hoc extraction is no longer needed since traces now go directly to agent logs.

Option 1: Remove the file entirely
Option 2: Keep for backwards compatibility but mark as deprecated

---

## Implementation Order

1. **Task 1**: Create `StoodLogLevel` enum (foundation)
2. **Task 6**: Fix hardcoded model ID (bug fix, independent)
3. **Task 8**: Add model change logging (enhancement)
4. **Task 3**: Update AgentLogger with `log_stood_trace()` method
5. **Task 2**: Create custom tracing layer for per-agent capture
6. **Task 5**: Update global tracing filter function
7. **Task 7**: Add agent reset methods
8. **Task 9**: Update thread-local context setup
9. **Task 4**: Replace UI checkbox with dropdown
10. **Task 10**: Deprecate agent_debug_logger.rs

---

## Testing Strategy

1. **Unit tests for StoodLogLevel**:
   - Enum serialization/deserialization
   - Display names
   - Level comparison

2. **Integration tests**:
   - Change log level, verify traces appear in agent log at correct level
   - Change model, verify new model is used
   - Reset agent, verify reinitialization

3. **Manual testing**:
   - Start agent, send message, check agent log for stood traces
   - Change log level mid-session, verify level change takes effect
   - Change model, verify model change logged and new model used

---

## Files Summary

| Action | File |
|--------|------|
| CREATE | `src/app/agent_framework/agent_tracing.rs` |
| MODIFY | `src/app/agent_framework/agent_types.rs` |
| MODIFY | `src/app/agent_framework/agent_instance.rs` |
| MODIFY | `src/app/agent_framework/agent_logger.rs` |
| MODIFY | `src/app/agent_framework/tool_context.rs` |
| MODIFY | `src/app/agent_framework/mod.rs` |
| MODIFY | `src/app/dashui/agent_manager_window.rs` |
| MODIFY | `src/lib.rs` |
| DEPRECATE | `src/app/agent_framework/agent_debug_logger.rs` |

---

## Risks and Mitigations

1. **Risk**: Custom tracing layer may impact performance
   - **Mitigation**: Only capture `stood::*` events, use efficient string matching

2. **Risk**: Thread-local storage may not work correctly across async boundaries
   - **Mitigation**: Set context at start of background thread, before any async code

3. **Risk**: Model ID mismatch between UI and stood library
   - **Mitigation**: Use `create_agent_with_model!` macro with validation

---

## Definition of Done

- [ ] Dropdown replaces checkbox in Agent Manager UI
- [ ] Stood traces appear in per-agent log files
- [ ] Log level filtering works (Off/Info/Debug/Trace)
- [ ] Changing log level resets agent
- [ ] Model ID uses correct enum (not hardcoded)
- [ ] Model changes are logged
- [ ] All existing tests pass
- [ ] New tests added for StoodLogLevel enum
