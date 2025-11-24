# Logging Improvements for Task-Agent System

## Issue

Currently, worker agents create their own separate log files. We need them to log to their parent's log file so you can see the complete conversation flow in one place.

## Current State

### Agent Log Files Location
- **Parent TaskManager**: `~/.local/share/awsdash/logs/agents/agent-{parent-uuid}.log`
- **Worker**: `~/.local/share/awsdash/logs/agents/agent-{worker-uuid}.log` (separate file)

### Current Logging Infrastructure

**AgentLogger** (`src/app/agent_framework/agent_logger.rs`):
- Already has `log_path()` method to get the log file path
- Logs all agent interactions, tool calls, model responses

**AgentInstance** (`src/app/agent_framework/agent_instance.rs`):
- Has `logger()` method to access the logger
- Creates AgentLogger in `new()` method
- Sets current logger for thread-local access

## Required Changes

### Option 1: Share Parent's Logger (Recommended)

Modify `AgentInstance` to accept an optional parent logger during creation:

```rust
// In AgentInstance
pub fn new(metadata: AgentMetadata, agent_type: AgentType) -> Self {
    // Current implementation - creates new logger
}

pub fn new_with_parent_logger(
    metadata: AgentMetadata,
    agent_type: AgentType,
    parent_logger: Arc<AgentLogger>
) -> Self {
    // New implementation - reuses parent's logger
}
```

**Changes needed**:

1. **`src/app/agent_framework/agent_instance.rs`**:
   - Add `new_with_parent_logger()` constructor
   - Accept parent logger and use it instead of creating new one
   - Log worker creation to parent's log

2. **`src/app/dashui/agent_manager_window.rs`** (handle_agent_creation_request):
   ```rust
   fn handle_agent_creation_request(&mut self, request: &AgentCreationRequest)
       -> Result<AgentId, String>
   {
       // ... existing code ...

       // Get parent agent's logger
       let parent_agent = self.agents.get(&request.parent_id)
           .ok_or("Parent agent not found")?;
       let parent_logger = parent_agent.logger().clone();

       // Create worker with parent's logger
       let mut agent = AgentInstance::new_with_parent_logger(
           metadata,
           agent_type,
           parent_logger  // <-- Pass parent's logger
       );

       // ... rest of code ...
   }
   ```

3. **Expected Log Format**:
   ```
   [Parent TaskManager Log: agent-xxx.log]

   [2025-11-21 10:00:00] User: Please list all EC2 instances
   [2025-11-21 10:00:01] Assistant: I'll spawn a worker to list EC2 instances
   [2025-11-21 10:00:02] Tool: start_task
   [2025-11-21 10:00:02]   Result: Worker agent-yyy created

   ====== Worker Agent: Task Worker 1 (agent-yyy) ======
   [2025-11-21 10:00:03] Initial Task: List all EC2 instances
   [2025-11-21 10:00:04] Tool: execute_javascript
   [2025-11-21 10:00:05]   JavaScript Code: const accounts = await listAccounts()...
   [2025-11-21 10:00:06]   Result: Found 5 EC2 instances
   [2025-11-21 10:00:07] Worker Response: Here are the 5 EC2 instances: [...]
   ====== End Worker Agent ======

   [2025-11-21 10:00:08] Parent: Received results from worker
   ```

### Option 2: Cross-Reference Logs

Alternative approach - keep separate logs but add cross-references:

```rust
// In parent's log
[2025-11-21 10:00:02] Tool: start_task
[2025-11-21 10:00:02]   Result: Worker agent-yyy created
[2025-11-21 10:00:02]   Worker Log: ~/.local/share/awsdash/logs/agents/agent-yyy.log

// In worker's log
[2025-11-21 10:00:03] Worker created by parent: agent-xxx
[2025-11-21 10:00:03] Parent Log: ~/.local/share/awsdash/logs/agents/agent-xxx.log
```

**Pros**: Simpler to implement, logs stay independent
**Cons**: Need to follow multiple files to see full picture

## Implementation Priority

**High Priority**:
1. ⏳ Add `new_with_parent_logger()` to AgentInstance
2. ⏳ Update `handle_agent_creation_request()` to pass parent logger
3. ⏳ Add worker section headers in log

**Medium Priority**:
4. ⏳ Add log separators for better readability
5. ⏳ Include parent context in worker log entries

**Low Priority**:
6. ⏳ UI indicator showing which log file contains the conversation
7. ⏳ Log viewer that can follow parent→worker chains

## Completed Tasks

**Log Flooding Fix** (commit cc9c285):
- ✅ Implemented debouncing for UI render logs
- ✅ Added last_logged_message_count HashMap to track state per agent
- ✅ Only log when message count changes (not every frame)
- ✅ Reduced log noise from 3600 messages/minute to only when new messages arrive

## Testing

After implementation, verify:

1. **Single Worker**:
   - Create TaskManager
   - Spawn 1 worker
   - Check parent's log file
   - Should see both parent and worker interactions

2. **Multiple Workers**:
   - Create TaskManager
   - Spawn 3 workers
   - Check parent's log file
   - Should see all 3 workers' interactions interleaved

3. **Concurrent Managers**:
   - Create 2 TaskManagers
   - Each spawns workers
   - Each parent's log should only show its own workers

## Additional Context

### Existing Logging Points

**AgentLogger logs**:
- Agent creation
- User messages
- Assistant responses
- Tool calls (name, parameters)
- Tool results
- Model API calls (requests, responses, token usage)
- Errors and warnings

**JavaScript execution logging** (execute_javascript tool):
- Already logs to agent logger via `set_current_agent_logger()`
- Shows JavaScript code, execution time, output, errors
- This should automatically work with shared logger

### Current Agent Execution Flow

```
1. User sends message to TaskManager
2. TaskManager processes message (logged to parent log)
3. TaskManager calls start-task tool (logged to parent log)
4. Worker agent created
5. Worker processes task (currently logged to separate file)
6. Worker calls execute_javascript (currently logged to separate file)
7. Worker returns result (currently logged to separate file)
8. Parent receives result (logged to parent log) - FUTURE
```

With shared logger, steps 5-7 would also log to parent's file.
