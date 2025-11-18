# Agent Manager - Agent Instance Registry and Lifecycle Tracking

## Component Overview

Simplified agent registry that maintains a HashMap of AgentInstance objects,
provides agent creation/deletion, and window management for agent UIs.
Does NOT handle async execution or message passing - that's in AgentInstance.

**Pattern**: Registry/Container with simple CRUD operations
**External**: None (uses AgentInstance for complex operations)
**State**: HashMap<AgentId, AgentInstance> + Arc<Mutex<AwsIdentityCenter>>

---

## Major Methods

- `new()` - Initialize with AWS Identity Center reference
- `launch_agent()` - Create new AgentInstance with metadata, return AgentId
- `get_agent()` - Retrieve immutable agent reference by ID
- `get_agent_mut()` - Retrieve mutable agent reference by ID
- `list_agents()` - Get all agent instances as Vec<&AgentInstance>
- `terminate_agent()` - Set agent status to Cancelled, close all windows
- `delete_agent()` - Remove agent from registry completely

---

## Implementation Patterns

### Pattern: Agent Registration

**Algorithm**: HashMap insertion with UUID-based AgentId
**External**: None

Pseudocode:
  1. Generate unique AgentId (Uuid::new_v4())
  2. Create AgentMetadata with name, description, model_id, timestamps
  3. Create new AgentInstance with metadata
  4. Insert into agents HashMap
  5. Return AgentId to caller
  6. Agent execution handled by AgentInstance, not AgentManager

### Pattern: Agent Termination

**Algorithm**: Status update and window cleanup
**External**: None

Pseudocode:
  1. Look up agent by AgentId in HashMap
  2. Call agent.set_status(AgentStatus::Cancelled)
  3. Call agent.close_all_windows() - returns Vec<String> of window IDs
  4. Return window IDs for UI cleanup
  5. Agent remains in registry (use delete_agent to remove)

### Pattern: Agent Deletion

**Algorithm**: Simple HashMap removal
**External**: None

Pseudocode:
  1. Call HashMap.remove(agent_id)
  2. Return error if not found
  3. AgentInstance dropped, cleanup automatic

---

## External Dependencies

- **std::collections::HashMap** - Agent storage
- **chrono::Utc** - Timestamp generation for metadata
- **AgentInstance** - Complex agent operations (execution, messaging)
- **AwsIdentityCenter** - AWS credential management (stored but not actively used in manager)

---

## Key Algorithms

### Agent Lifecycle States
States: Created (Running) → Paused | Completed | Failed | Cancelled
AgentManager only sets Cancelled status on termination
Other states managed by AgentInstance during execution

### Window Management
Track which UI windows are viewing an agent
On termination: close all associated windows
Window registry maintained in AgentInstance, not AgentManager

---

## Simplifications from Original Design

This implementation is significantly simpler than originally documented:
- No async task spawning (moved to AgentInstance)
- No message channels (in AgentInstance)
- No callback handlers (in AgentInstance)
- No event processing loop (in AgentInstance)
- No cancellation token management (in AgentCancellationManager global)

AgentManager is a thin registry layer.
AgentInstance handles all complex agent operations.

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects simplified implementation in agent_manager.rs
