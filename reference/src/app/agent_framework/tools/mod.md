# Tools Module - AWS Agent Framework Tools

## Component Overview

Module exports for Agent Framework tool implementations. Each tool provides
specific capabilities to agents through the stood::tools::Tool trait.

**Pattern**: Module organization with re-exports
**External**: stood::tools::Tool, V8 JavaScript engine
**Purpose**: Tool implementations for agent-driven AWS operations and orchestration

---

## Module Structure

### Tool Implementations
- `mod.rs` - This file: tool module exports
- `execute_javascript.rs` - ExecuteJavaScriptTool (V8 engine with AWS API bindings)
- `start_task.rs` - StartTaskTool (worker agent spawning)
- `think.rs` - ThinkTool (reasoning/planning)
- `todo_read.rs` - TodoReadTool
- `todo_write.rs` - TodoWriteTool
- `todo_types.rs` - TodoItem, TodoStatus
- `file_security.rs` - File security validation utilities

---

## Public API Exports

```rust
pub use execute_javascript::ExecuteJavaScriptTool;
pub use start_task::StartTaskTool;
pub use think::ThinkTool;
pub use todo_read::TodoReadTool;
pub use todo_types::{TodoItem, TodoStatus};
pub use todo_write::TodoWriteTool;
```

---

## Tool Categories

### JavaScript Execution (Primary Worker Tool)
Execute JavaScript code with AWS API bindings:
- **ExecuteJavaScriptTool** - V8 sandbox with AWS API access
  - listAccounts(), listRegions(), queryResources()
  - queryCloudWatchLogEvents(), getCloudTrailEvents()
  - Console output capture, timeout/memory limits

### Agent Coordination Tools
Multi-agent orchestration:
- **StartTaskTool** - Spawn TaskWorker agents, wait for results
- **ThinkTool** - Reasoning/planning (logs thoughts, no operation)

### Task Tracking Tools
Shared TODO list management:
- **TodoWriteTool** - Create/update shared TODO items
- **TodoReadTool** - Query shared TODO items

---

## Tool Assignment by Agent Type

### TaskManager Agent
Uses these tools for orchestration:
- **think** - Planning and analysis
- **start_task** - Spawn workers

### TaskWorker Agent
Uses this tool for execution:
- **execute_javascript** - JavaScript with AWS API bindings

---

## Tool Registration

Tools are registered with agents during stood::Agent builder configuration.

### TaskManager Tools
```rust
Agent::builder()
    .add_tool(Box::new(ThinkTool::new()))
    .add_tool(Box::new(StartTaskTool::new()))
    .build()
```

### TaskWorker Tools
```rust
Agent::builder()
    .add_tool(Box::new(ExecuteJavaScriptTool::new()))
    .build()
```

---

## Implementation Notes

### JavaScript Execution Model
ExecuteJavaScriptTool is the primary tool for TaskWorker agents:
- V8 sandbox with memory limits (256MB default)
- Timeout enforcement (30s default)
- AWS API bindings for resource queries
- Console output capture (stdout/stderr)
- JSON serialization of results

### Global State Access
Tools access global state from `tools_registry.rs`:
- GLOBAL_AWS_CLIENT: For AWS SDK operations
- GLOBAL_AWS_CREDENTIALS: For standalone agent AWS access
- GLOBAL_CANCELLATION_MANAGER: For agent stop signals
- GLOBAL_MODEL_CONFIG: For agent model configuration

### Error Handling
Tools return ToolError with user-friendly messages:
- Explain what went wrong
- Suggest remediation steps
- Never expose credentials or internal implementation

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects tools/mod.rs structure
