# Debug Logger - Framework-Wide Debug Event Logging

## Component Overview

Optional framework-wide debug logging system that records all agent framework
events to a single log file. Captures orchestration agent lifecycle, tool
calls, task creation, and performance metrics for debugging and analysis.

**Pattern**: Global singleton logger with lazy initialization
**External**: lazy_static, file I/O, tracing
**Purpose**: Framework-level debugging and performance analysis

---

## Major Components

### AgentDebugEvent Enum
- `OrchestrationAgentStart` - Main agent creation with session metadata
- `OrchestrationPromptSent` - User prompt sent to model
- `OrchestrationResponseReceived` - Model response received
- `OrchestrationToolCall` - Tool invocation by agent
- `CreateTaskStart` - Sub-task agent creation initiated

### AgentDebugLogger
- Singleton instance in AGENT_DEBUG_LOGGER global
- Log file: ~/.local/share/awsdash/logs/agent-framework-debug.log
- Thread-safe write access with Mutex

---

## Implementation Patterns

### Pattern: Lazy Global Initialization

**Algorithm**: Lazy static with optional creation based on env var
**External**: lazy_static macro, std::env

Pseudocode:
  1. Check AGENT_FRAMEWORK_DEBUG environment variable
  2. If set to "1" or "true":
     - Create log directory if not exists
     - Create agent-framework-debug.log file
     - Initialize AgentDebugLogger
     - Store in AGENT_DEBUG_LOGGER static
  3. If not set:
     - Set AGENT_DEBUG_LOGGER to None
     - log_agent_debug_event() becomes no-op
  4. Enables debug logging without code changes

### Pattern: Event Logging with Structured Format

**Algorithm**: Structured event serialization to log file
**External**: serde_json, file::write_all, Mutex

Pseudocode:
  1. Call log_agent_debug_event(AgentDebugEvent)
  2. Get AGENT_DEBUG_LOGGER global
  3. If logger is Some:
     - Lock logger mutex
     - Serialize event to JSON
     - Format with timestamp and event type
     - Write to log file
     - Flush to ensure persistence
  4. If logger is None:
     - Return immediately (no-op)
  5. Preserves all events for post-analysis

### Pattern: Session-Based Organization

**Algorithm**: Session ID tracks related events
**External**: Session ID from agent creation

Pseudocode:
  1. OrchestrationAgentStart event includes session_id
  2. All subsequent events for that agent include same session_id
  3. CreateTaskStart events create new session for sub-agent
  4. Log file can be filtered/grouped by session_id
  5. Enables correlation across multi-agent workflows

---

## External Dependencies

- **lazy_static**: Global logger initialization
- **serde_json**: Event serialization
- **std::fs**: File operations
- **std::sync**: Mutex for thread-safe writes
- **chrono**: Timestamp generation

---

## Key Algorithms

### Event Capture Points
- Agent creation: OrchestrationAgentStart
- Tool invocation: OrchestrationToolCall
- Task spawning: CreateTaskStart
- Model interaction: Prompt/Response events

### Log File Format
```
[timestamp] SESSION_START session_id=... model=...
[timestamp] TOOL_CALL tool=create_task input={...}
[timestamp] TASK_START task_id=... description=...
```

---

**Last Updated**: 2025-10-28
