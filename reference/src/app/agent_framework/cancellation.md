# Agent Cancellation Management

## Component Overview

Manages cancellation tokens for active agents created via create_task tool.
Provides centralized cancellation control for stopping running agents from UI.

**Pattern**: Cancellation token manager with HashMap tracking
**Algorithm**: tokio_util::CancellationToken for cooperative cancellation
**External**: tokio_util::sync::CancellationToken, Arc/Mutex

---

## Major Methods

- `new()` - Create empty cancellation manager
- `create_token()` - Generate new token for agent, store in HashMap
- `cancel_agent()` - Trigger token for specific agent by ID
- `cancel_all()` - Trigger all active tokens, drain HashMap
- `remove_token()` - Clean up token after agent completes normally
- `active_count()` - Get number of agents with active tokens
- `has_active_token()` - Check if specific agent has token
- `get_active_agent_ids()` - List all agent IDs with tokens

---

## Implementation Patterns

### Pattern: Cancellation Token Creation

**Algorithm**: HashMap storage with agent_id key
**External**: tokio_util::sync::CancellationToken

Pseudocode:
  1. create_token(agent_id):
     - Create new CancellationToken
     - Lock active_tokens HashMap
     - Insert agent_id → token mapping
     - Return token clone to caller
  2. Agent stores token, checks periodically via token.is_cancelled()
  3. On cancellation: agent terminates gracefully

### Pattern: Cooperative Cancellation

**Algorithm**: Polling-based cancellation checking
**External**: tokio::select! macro

Pseudocode:
  1. Agent receives CancellationToken during creation
  2. Agent work loop:
     tokio::select! {
       result = do_work() => handle_result(result),
       _ = token.cancelled() => break  // Exit on cancellation
     }
  3. Agent cleans up resources before exit
  4. Cancellation is cooperative, not forced

### Pattern: Centralized Cancellation

**Algorithm**: cancel_agent() or cancel_all() from UI
**External**: Global GLOBAL_CANCELLATION_MANAGER

Pseudocode:
  1. UI Stop button calls get_global_cancellation_manager()
  2. Call manager.cancel_agent(agent_id)
  3. Manager finds token in HashMap, calls token.cancel()
  4. Token cancellation propagates to agent's select! loop
  5. Agent terminates, calls remove_token() during cleanup

---

## External Dependencies

- **tokio_util**: CancellationToken for cooperative cancellation
- **std::sync**: Arc/Mutex for thread-safe HashMap
- **tracing**: Debug logging for cancellation events

---

## Key Algorithms

### Token Lifecycle
Create → Store → Check (agent loop) → Cancel OR Remove (on complete)
Tokens removed from HashMap after cancellation or completion
No memory leaks: completed agents cleaned up

### Parallel Cancellation
cancel_all() drains HashMap, triggers all tokens
Used on app shutdown or "stop all agents" command
Returns count of cancelled agents

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects cancellation.rs implementation
