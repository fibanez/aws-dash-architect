# TODO Read Tool

## Component Overview

Queries shared TODO items across agents. Supports filtering by agent, status,
and priority for task tracking and coordination.

**Pattern**: Tool trait with global TODO storage
**Algorithm**: HashMap query with filtering
**External**: stood::tools::Tool, GLOBAL_TODO_STORAGE

---

## Major Methods

- `new()` - Create with isolated storage
- `with_shared_storage()` - Create with global shared storage
- `execute()` - Query TODOs with filters

---

## Implementation Patterns

### Pattern: Shared TODO Storage Access

**Algorithm**: Read from global Arc&lt;Mutex&lt;HashMap&gt;&gt;
**External**: GLOBAL_TODO_STORAGE from tools_registry

Pseudocode:
  1. Parse input: agent_id (optional), status_filter, priority_filter
  2. Lock GLOBAL_TODO_STORAGE Mutex
  3. If agent_id specified: get Vec<TodoItem> for that agent
     Else: aggregate TODOs from all agents
  4. Filter by status (pending/in_progress/completed)
  5. Filter by priority (low/medium/high)
  6. Return filtered Vec<TodoItem>

### Pattern: Cross-Agent TODO Visibility

**Algorithm**: Agents can query other agents' TODOs
**External**: Shared HashMap keyed by agent_id

Pseudocode:
  1. Orchestration agent: query all TODOs (no agent_id filter)
  2. Task agent: query own TODOs (filter by agent_id)
  3. Enables coordination: orchestration sees sub-agent progress
  4. Atomic reads via Mutex lock

---

## Tool Parameters

- agent_id: Optional&lt;String&gt; (filter by agent, None = all)
- status: Optional&lt;String&gt; (pending/in_progress/completed)
- priority: Optional&lt;String&gt; (low/medium/high)

---

## External Dependencies

- **stood::tools::Tool**: Tool trait
- **GLOBAL_TODO_STORAGE**: Shared HashMap from tools_registry
- **TodoItem**: Struct with content, status, priority, timestamps

---

**Last Updated**: 2025-01-28
