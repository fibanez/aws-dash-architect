# Prompts Module - Agent System Prompts

## Component Overview

Contains system prompts for different agent types. Prompts define agent
behavior, capabilities, and interaction patterns.

**Pattern**: Constant string prompts with structured instructions
**External**: None
**Purpose**: Define agent personalities and capabilities

---

## Module Structure

- `mod.rs` - Module exports (TASK_MANAGER_PROMPT, TASK_WORKER_PROMPT)
- `task_manager.rs` - Orchestration agent prompt
- `task_worker.rs` - Worker agent prompt

---

## Prompt Types

### TASK_MANAGER_PROMPT
System prompt for TaskManager agents:
- Orchestrates complex multi-step tasks
- Spawns TaskWorker sub-agents via start_task tool
- Uses think tool for planning
- Coordinates results from workers

### TASK_WORKER_PROMPT
System prompt for TaskWorker agents:
- Executes specific focused tasks
- Uses execute_javascript tool with AWS API bindings
- Returns structured results to parent
- Single-purpose execution

---

## Usage

```rust
use crate::app::agent_framework::prompts::{TASK_MANAGER_PROMPT, TASK_WORKER_PROMPT};

let prompt = match agent_type {
    AgentType::TaskManager { .. } => TASK_MANAGER_PROMPT,
    AgentType::TaskWorker { .. } => TASK_WORKER_PROMPT,
};
```

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects prompts/mod.rs
