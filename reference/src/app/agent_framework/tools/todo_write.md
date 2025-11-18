# TODO Write Tool

## Component Overview

Creates and updates shared TODO items for task tracking. Supports status
transitions (pending → in_progress → completed) and priority levels.

**Pattern**: Tool trait with global TODO storage
**Algorithm**: HashMap update with atomic Mutex operations
**External**: stood::tools::Tool, GLOBAL_TODO_STORAGE

---

## Major Methods

- `new()` - Create with isolated storage
- `with_shared_storage()` - Create with global shared storage
- `execute()` - Create/update TODO items

---

## Implementation Patterns

### Pattern: Shared TODO Storage Write

**Algorithm**: Write to global Arc&lt;Mutex&lt;HashMap&gt;&gt;
**External**: GLOBAL_TODO_STORAGE from tools_registry

Pseudocode:
  1. Parse input: todos (Vec<TodoItem>)
  2. Lock GLOBAL_TODO_STORAGE Mutex
  3. Get or create Vec for current agent_id
  4. For each TodoItem:
     - If new: append to Vec
     - If update: find by content, update status/priority
  5. Store updated Vec back to HashMap
  6. Unlock Mutex

### Pattern: TODO Item Structure

**Algorithm**: Struct with status state machine
**External**: TodoStatus, TodoPriority enums

Pseudocode:
  1. TodoItem fields:
     - content: String (task description)
     - status: pending | in_progress | completed
     - priority: low | medium | high
     - created_at: timestamp
     - updated_at: timestamp
  2. Status transitions:
     pending → in_progress → completed
  3. UI displays with emoji: 📝 pending, 🔧 in_progress, ✅ completed

### Pattern: Cross-Agent TODO Synchronization

**Algorithm**: Agents share TODO storage via GLOBAL_TODO_STORAGE
**External**: tools_registry initialization

Pseudocode:
  1. Orchestration agent creates TODOs for plan
  2. Task agents update TODOs as they complete work
  3. Orchestration agent queries TODOs to track progress
  4. Atomic updates via Mutex prevent race conditions
  5. HashMap keyed by agent_id enables per-agent TODO lists

---

## Tool Parameters

- todos: Vec&lt;TodoItem&gt; (items to create/update)

TodoItem structure:
- content: String (required)
- status: String (pending/in_progress/completed)
- priority: String (low/medium/high)

---

## External Dependencies

- **stood::tools::Tool**: Tool trait
- **GLOBAL_TODO_STORAGE**: Shared HashMap from tools_registry
- **TodoItem, TodoStatus, TodoPriority**: Data structures

---

**Last Updated**: 2025-01-28
