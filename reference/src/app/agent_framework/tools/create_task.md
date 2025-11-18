# Create_Task Tool - Dynamic Sub-Agent Creation

## Component Overview

Enables orchestration agent to create specialized TaskAgents on-demand for
specific AWS operations. Each task agent runs independently with scoped
account/region access and dedicated logging.

**Pattern**: Tool-based agent spawning with scope enforcement
**External**: TaskAgent builder, global cancellation manager
**Purpose**: Multi-agent task decomposition and parallel execution

---

## Tool Parameters

- `task_description: String` - Detailed prompt for the sub-task
- `account_ids: Vec<String>` - AWS account IDs to operate on
- `regions: Vec<String>` - AWS regions to target
- Optional: `model_id` - Override default model for this task

---

## Implementation Patterns

### Pattern: Async Task Agent Spawning

**Algorithm**: Non-blocking agent creation with cancellation support
**External**: tokio::spawn, CancellationToken, AgentLogger

Pseudocode:
  1. Validate parameters:
     - task_description is non-empty
     - account_ids and regions are valid
  2. Generate unique task_id (UUID)
  3. Log CreateTaskStart debug event with task details
  4. Create AgentLogger for this task agent
  5. Build TaskAgent with:
     - task_description as system prompt
     - account_ids and regions in scope
     - All AWS operation tools registered
     - Per-agent logger attached
  6. Get global cancellation manager
  7. Create cancellation token for this task
  8. Register token with manager using task_id
  9. Spawn tokio task to run agent
 10. Return success immediately (non-blocking)
 11. Task agent executes independently
 12. Results logged to ~/.local/share/awsdash/logs/agents/{id}.log

### Pattern: Scope Enforcement via System Prompt

**Algorithm**: Inject constraints into agent's context
**External**: Claude model following instructions

Pseudocode:
  1. Build system prompt with task description
  2. Append scope constraints:
     "You are working with these AWS accounts: [X, Y, Z]"
     "You are working with these AWS regions: [us-east-1, ...]"
  3. Agent learns to pass these to tools automatically
  4. Tools validate account/region parameters
  5. Prevents unauthorized cross-account access
  6. Enables safe multi-account operations

### Pattern: Performance Tracking

**Algorithm**: Phase-based timing with PerformanceTimer
**External**: Instant timestamps, microsecond precision

Pseudocode:
  1. Start timer at tool invocation
  2. Track phases:
     - Parameter validation time
     - Agent configuration time
     - Agent builder construction time
     - Task spawning time
  3. Log AgentCreationMetrics to debug logger
  4. Used to identify bottlenecks in agent creation
  5. Optimize slow phases (e.g., model loading)

---

## External Dependencies

- **TaskAgent**: Specialized agent builder
- **AgentLogger**: Per-agent execution logging
- **AgentCancellationManager**: Token-based cancellation
- **tokio**: Async task spawning
- **uuid**: Unique task ID generation

---

## Key Algorithms

### Task Agent Lifecycle
- Creation: Spawned by orchestration agent via tool call
- Execution: Independent async task with own event loop
- Logging: Dedicated log file per task agent
- Cancellation: Via global manager using task_id
- Completion: Agent self-terminates, logs results

### Cancellation Coordination
- Single source of truth: global cancellation manager
- UI can cancel individual tasks or all tasks
- Graceful shutdown: agents check token periodically
- Cleanup: Task removed from active set on completion

---

**Last Updated**: 2025-10-28
