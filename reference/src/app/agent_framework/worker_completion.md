# Worker Completion - Async Tool Result Delivery

## Component Overview

Provides channel-based mechanism for delivering worker agent results back to
the start_task tool as proper tool results, not user messages. Uses Condvar
for efficient blocking/notification pattern.

**Pattern**: Producer-consumer with condition variables
**Algorithm**: Registry with Condvar notification, timeout support
**External**: std::sync::Condvar, once_cell::sync::Lazy, HashMap

---

## Major Types

### WorkerCompletion Struct
- `worker_id: AgentId` - ID of completed worker
- `result: Result<String, String>` - Ok(response) or Err(error)
- `execution_time: Duration` - How long worker took

### CompletionRegistry
- Type: `Arc<Mutex<HashMap<AgentId, (Option<WorkerCompletion>, Arc<Condvar>)>>>`
- Maps worker_id to (result slot, notification condvar)

---

## Major Functions

- `send_worker_completion()` - UI calls when worker finishes
- `wait_for_worker_completion()` - Tool blocks until result or timeout
- `register_pending_worker()` - Internal: register worker, get condvar

---

## Implementation Patterns

### Pattern: Blocking Wait with Condvar

**Algorithm**: Register, wait on Condvar, extract result
**External**: std::sync::Condvar::wait_timeout_while()

Pseudocode:
  1. wait_for_worker_completion(worker_id, timeout):
     - register_pending_worker(): insert (None, Condvar) in registry
     - Lock registry
     - condvar.wait_timeout_while(guard, timeout, |reg| result is None)
     - If timeout: remove entry, return Err("timeout")
     - Extract result, remove entry, return result

### Pattern: Result Delivery via Notification

**Algorithm**: Store result, notify waiting thread
**External**: Condvar::notify_one()

Pseudocode:
  1. send_worker_completion(completion):
     - Lock registry
     - Find entry by worker_id
     - Store completion in result slot
     - condvar.notify_one() to wake waiting thread
     - Log success or warn if no waiter

### Pattern: Tool Result Flow

**Algorithm**: Ensures LLM sees worker output as tool result
**External**: stood Tool trait, ToolResult

Pseudocode:
  1. start_task tool calls request_agent_creation()
  2. Tool then calls wait_for_worker_completion(worker_id, 5 min)
  3. AgentManagerWindow runs worker in background
  4. When worker responds, UI calls send_worker_completion()
  5. Tool unblocks, returns ToolResult with worker response
  6. LLM conversation shows: start_task returned: {worker result}

---

## External Dependencies

- **std::sync::Condvar** - Efficient blocking/notification
- **std::sync::Mutex** - Registry protection
- **std::collections::HashMap** - Worker ID to completion mapping
- **once_cell::sync::Lazy** - Global static initialization
- **std::time::Duration** - Timeout handling
- **tracing** - Debug logging

---

## Key Algorithms

### Timeout Handling
Uses wait_timeout_while() which returns (guard, WaitTimeoutResult)
WaitTimeoutResult.timed_out() indicates if timeout occurred
Cleanup on timeout: remove registry entry to prevent leak

### Condvar Wait Pattern
wait_timeout_while takes closure that returns true while should wait
Closure checks if result slot is still None
Returns when: result stored OR timeout expires

---

**Last Updated**: 2025-11-25
**Status**: New file for multi-agent task orchestration system
