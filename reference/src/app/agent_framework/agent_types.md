# Agent Types - Shared Type Definitions

## Component Overview

Core types used by agent systems. Provides common identification, status
tracking, agent type classification, and metadata for agents.

**Pattern**: Type definitions with derives
**Purpose**: Shared agent identity and classification
**External**: uuid, chrono

---

## Major Types

### AgentId
- Wrapper: `AgentId(Uuid)`
- Methods: `new()` generates UUIDv4
- Traits: Debug, Clone, Copy, PartialEq, Eq, Hash, Display, Default

### AgentStatus
- `Running` - Agent currently executing
- `Paused` - Execution paused
- `Completed` - Finished successfully
- `Failed(String)` - Failed with error message
- `Cancelled` - User cancelled

### AgentType
- `TaskManager` - Orchestrates task-worker agents
- `TaskWorker { parent_id: AgentId }` - Executes specific tasks

### AgentMetadata
- `name: String` - Human-readable name
- `description: String` - Agent purpose
- `model_id: String` - Model ID (e.g., "claude-sonnet-4")
- `created_at: DateTime<Utc>` - Creation timestamp
- `updated_at: DateTime<Utc>` - Last update timestamp

---

## AgentType Methods

- `is_task_manager()` - Returns true for TaskManager variant
- `parent_id()` - Returns Some(parent_id) for TaskWorker, None for TaskManager
- Display trait: "Task Manager" or "Task Worker"

---

## Implementation Patterns

### Pattern: Parent-Child Relationship

**Algorithm**: TaskWorker stores parent reference
**External**: AgentId

Pseudocode:
  1. TaskManager agent receives user request
  2. Spawns TaskWorker with own AgentId as parent_id
  3. TaskWorker stores parent_id in AgentType::TaskWorker
  4. worker.agent_type.parent_id() returns parent's ID
  5. Used for: logging to parent's log file, result routing

### Pattern: Type-Based Tool Selection

**Algorithm**: AgentInstance checks type, returns appropriate tools
**External**: AgentInstance::get_tools_for_type()

Pseudocode:
  1. AgentInstance::new() receives AgentType
  2. get_tools_for_type() matches on agent_type:
     - TaskManager -> think, start_task tools
     - TaskWorker -> execute_javascript tool

### Pattern: Type-Based Prompt Selection

**Algorithm**: AgentInstance checks type, returns appropriate prompt
**External**: AgentInstance::get_system_prompt_for_type()

Pseudocode:
  1. get_system_prompt_for_type() matches on agent_type:
     - TaskManager -> TASK_MANAGER_PROMPT
     - TaskWorker -> TASK_WORKER_PROMPT
  2. Replace {{CURRENT_DATETIME}} placeholder
  3. Return configured prompt

---

## External Dependencies

- **uuid::Uuid** - Unique identifier generation
- **chrono** - DateTime<Utc> for timestamps
- **std::fmt** - Display trait implementations

---

## Key Algorithms

### AgentId Generation
Uses Uuid::new_v4() for random UUID
Copy trait allows cheap stack copies

### Type Equality
TaskManager == TaskManager (always)
TaskWorker == TaskWorker if parent_id matches
Different variants always not equal

---

**Last Updated**: 2025-11-25
**Status**: New file for multi-agent task orchestration system
