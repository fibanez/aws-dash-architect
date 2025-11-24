# End-to-End Human Integration Test Scenarios

This document describes manual test scenarios for verifying the task-agent system works correctly in the real application.

## Prerequisites

- AWS credentials configured (for AWS API calls)
- Application running: `cargo run`
- Agent Manager window open

---

## Test Scenario 1: Single Task Worker Creation

**Goal**: Verify a task-manager can spawn a single worker and the worker executes successfully.

### Steps:

1. **Create TaskManager Agent**
   - Click "+ New Agent" in Agent Manager window
   - Verify new agent appears as "Agent 1" in left pane
   - Verify agent type is TaskManager
   - Click on the agent to select it

2. **Send Task Creation Request**
   - In the chat input, type:
     ```
     Please use the start-task tool to list all EC2 instances in the current region
     ```
   - Press Send

3. **Verify Task Spawning**
   - Watch the conversation for the agent to call `start_task` tool
   - Verify tool response shows:
     - `status: "task_agent_created"`
     - `agent_id: "..."` (UUID)
     - `message: "TaskWorker agent created..."`

4. **Verify Worker Created**
   - Check left pane - should see new agent: "Task Worker 1"
   - UI should automatically switch to show Task Worker 1 (SwitchToAgent event)

5. **Verify Worker Execution**
   - Worker agent should show conversation with initial task message
   - Worker should start executing JavaScript to query EC2 instances
   - Verify worker uses `execute_javascript` tool
   - Verify worker returns EC2 instance data

6. **Verify Parent-Child Relationship**
   - In logs, search for: `"Created TaskWorker agent"`
   - Verify log shows: `parent_id` matching the TaskManager's ID

### Expected Results:
- ✅ TaskManager successfully calls start-task tool
- ✅ TaskWorker agent appears in agent list
- ✅ UI switches to TaskWorker automatically
- ✅ TaskWorker executes the task
- ✅ Parent-child relationship is correct

### Failure Modes to Check:
- ❌ Tool returns error about agent context not set → Agent context not being set before tool execution
- ❌ 5-second timeout error → AgentManagerWindow not processing creation requests
- ❌ "Parent agent not found" error → Parent ID mismatch
- ❌ Worker not appearing in list → Agent not being added to AgentManagerWindow.agents map

---

## Test Scenario 2: Multiple Parallel Task Workers

**Goal**: Verify a task-manager can spawn multiple workers in parallel and coordinate them.

### Steps:

1. **Create TaskManager Agent**
   - Click "+ New Agent"
   - Select the new TaskManager

2. **Request Multiple Parallel Tasks**
   - In chat input, type:
     ```
     I need you to gather information about three different AWS services in parallel:
     1. List all EC2 instances
     2. List all S3 buckets
     3. List all RDS databases

     Use the start-task tool to spawn three separate workers for these tasks.
     ```
   - Press Send

3. **Verify Multiple Workers Spawn**
   - TaskManager should call start-task tool 3 times
   - Watch left pane - should see:
     - "Task Worker 1"
     - "Task Worker 2"
     - "Task Worker 3"
   - Each worker creation should trigger a SwitchToAgent event (UI will switch to last created)

4. **Verify All Workers Execute**
   - Click through each worker in left pane
   - Each should show:
     - Initial task message
     - Worker executing JavaScript
     - Results being gathered

5. **Check Task Coordination**
   - Switch back to TaskManager
   - TaskManager should track which workers are running
   - Verify TaskManager can use todo-write tool to track worker progress

### Expected Results:
- ✅ TaskManager spawns 3 workers
- ✅ All 3 workers appear in agent list
- ✅ Each worker has correct parent_id
- ✅ All workers execute their tasks independently
- ✅ TaskManager can track worker status

### Failure Modes to Check:
- ❌ Only 1 worker created → Channel not handling concurrent requests
- ❌ Workers have wrong parent → Parent ID not being passed correctly
- ❌ Race conditions in agent creation → Mutex/channel issues

---

## Test Scenario 3: Task Planning with Todo Tools

**Goal**: Verify task-manager uses planning tools (think, todo-write, todo-read) before spawning workers.

### Steps:

1. **Create TaskManager Agent**
   - Click "+ New Agent"
   - Select the new TaskManager

2. **Request Complex Multi-Step Task**
   - In chat input, type:
     ```
     I need a comprehensive security audit of my AWS infrastructure.
     Please plan the approach, break it into subtasks, and execute them.
     ```
   - Press Send

3. **Verify Planning Phase**
   - TaskManager should use `think` tool to analyze the request
   - Verify think tool output shows reasoning about approach
   - TaskManager should use `todo-write` tool to create task list
   - Verify todo list shows:
     - Multiple tasks (security groups, IAM, S3 policies, etc.)
     - One task marked as "in_progress"
     - Others marked as "pending"

4. **Verify Task Execution**
   - For the in_progress task, TaskManager should call `start-task`
   - Worker should be created for first subtask
   - After worker completes, TaskManager should:
     - Use `todo-write` to mark first task as "completed"
     - Use `todo-write` to mark next task as "in_progress"
     - Call `start-task` for next subtask

5. **Verify Todo-Read Usage**
   - At any point, type: "What's the current status?"
   - TaskManager should use `todo-read` tool
   - Verify it shows current progress

### Expected Results:
- ✅ TaskManager uses think tool for planning
- ✅ TaskManager creates structured todo list
- ✅ Only one task is "in_progress" at a time
- ✅ TaskManager spawns workers for each subtask sequentially
- ✅ Todo list is updated as tasks complete

### Failure Modes to Check:
- ❌ Multiple tasks marked "in_progress" → Validation not working
- ❌ TaskManager doesn't plan → Prompt not guiding behavior
- ❌ Workers not spawned for subtasks → start-task not being called

---

## Test Scenario 4: UI Event Integration

**Goal**: Verify UI events work correctly (SwitchToAgent, SwitchToParent, AgentCompleted).

### Steps:

1. **Setup: Create Manager and Worker**
   - Create TaskManager
   - Have it spawn a worker via start-task
   - Note the manager ID and worker ID

2. **Test SwitchToAgent Event**
   - Spawn a second worker
   - Verify UI automatically switches to show the newly created worker
   - Check log: `"UI event: Switch to agent"`

3. **Test Manual Agent Switching**
   - Click on TaskManager in left pane
   - Verify UI switches back to manager
   - Click on worker in left pane
   - Verify UI switches to worker

4. **Test AgentCompleted Event** (Future)
   - When worker finishes task, it should send AgentCompleted event
   - Verify event is logged: `"UI event: Agent completed"`
   - Note: Full auto-switch implementation is deferred to future milestone

### Expected Results:
- ✅ Newly created agents trigger SwitchToAgent
- ✅ UI switches focus to new agent
- ✅ Manual switching works
- ✅ Events are logged correctly

### Failure Modes to Check:
- ❌ UI doesn't switch → Event not being sent or processed
- ❌ Event channel full → Too many events, not being consumed

---

## Test Scenario 5: Error Handling

**Goal**: Verify system handles errors gracefully.

### Steps:

1. **Test Invalid Task Description**
   - Create TaskManager
   - Try to manually call start-task with empty description
   - Verify error: "task_description cannot be empty"

2. **Test Agent Creation Timeout** (Simulated)
   - This is hard to test manually
   - Would require stopping AgentManagerWindow from processing requests
   - Expected: 5-second timeout with error message

3. **Test Parent Not Found** (Simulated)
   - This shouldn't happen in normal flow
   - Would require sending creation request with non-existent parent ID

4. **Test Tool Execution Without Context**
   - This shouldn't happen if agents are created correctly
   - All tools should have agent context set

### Expected Results:
- ✅ Validation errors are returned to agent
- ✅ Timeout errors don't crash the system
- ✅ Error messages are clear and actionable

---

## Test Scenario 6: Concurrent Agent Operations

**Goal**: Verify multiple agents can work simultaneously without interference.

### Steps:

1. **Create Two TaskManagers**
   - Click "+ New Agent" twice
   - Should have "Agent 1" and "Agent 2"
   - Both are TaskManagers

2. **Have Each Spawn Workers**
   - In Agent 1, request: "List EC2 instances"
   - Switch to Agent 2, request: "List S3 buckets"
   - Each should spawn its own worker

3. **Verify Independent Operation**
   - Should have 4 agents total:
     - Agent 1 (TaskManager)
     - Task Worker 1 (parent: Agent 1)
     - Agent 2 (TaskManager)
     - Task Worker 2 (parent: Agent 2)
   - Each worker should have correct parent_id
   - Workers should not interfere with each other

4. **Verify Concurrent Execution**
   - Both workers should execute simultaneously
   - Check logs for interleaved execution traces

### Expected Results:
- ✅ Multiple managers can coexist
- ✅ Each spawns independent workers
- ✅ No cross-contamination of parent IDs
- ✅ Concurrent execution works

### Failure Modes to Check:
- ❌ Wrong parent_id assignments → Thread-local context issues
- ❌ Workers interfering → Shared state problems

---

## Test Scenario 7: Agent Context Lifecycle

**Goal**: Verify thread-local agent context is properly managed.

### Steps:

1. **Create TaskManager**
   - Click "+ New Agent"

2. **Monitor Agent Context in Logs**
   - Set `RUST_LOG=awsdash=trace`
   - Look for agent context being set/cleared
   - Should see context set before tool execution
   - Should see context cleared after

3. **Spawn Worker**
   - Request start-task
   - Verify parent_id is correctly extracted from context
   - Verify worker is created with correct parent

4. **Check for Context Leaks**
   - Create multiple agents in sequence
   - Verify each gets correct context
   - Verify no cross-contamination

### Expected Results:
- ✅ Context is set before tool execution
- ✅ Context is cleared after execution
- ✅ No context leaks between agents

---

## Test Scenario 8: Real AWS Task End-to-End

**Goal**: Verify the complete system works with real AWS API calls.

### Steps:

1. **Prerequisites**
   - Ensure AWS credentials are configured
   - Ensure you have EC2 instances in your account (or other resources)

2. **Create TaskManager**
   - Click "+ New Agent"

3. **Request Real AWS Task**
   - In chat input, type:
     ```
     Please list all EC2 instances in us-east-1 and provide a summary of:
     - Total number of instances
     - Instances by state (running, stopped, etc.)
     - Instance types being used

     Use start-task to spawn a worker for this.
     ```

4. **Verify Complete Flow**
   - TaskManager calls start-task
   - Worker is created
   - Worker uses execute_javascript tool
   - JavaScript calls AWS APIs via queryResources()
   - Worker processes results
   - Worker returns formatted summary
   - TaskManager receives worker results (future: via result channel)

5. **Verify Correct Data**
   - Check worker output matches real AWS console data
   - Verify counts are accurate
   - Verify no duplicate data

### Expected Results:
- ✅ Real AWS API calls succeed
- ✅ Data is retrieved correctly
- ✅ Worker processes and formats data
- ✅ TaskManager can use results (future)

### Failure Modes to Check:
- ❌ AWS API errors → Check credentials, permissions
- ❌ Timeout on large queries → Adjust timeouts
- ❌ Incorrect data → Check JavaScript implementation

---

## Monitoring and Debugging

### Key Log Locations

**Main Application Log**:
```bash
tail -f ~/.local/share/awsdash/logs/awsdash.log
```

**Per-Agent Logs**:
```bash
# List all agent logs
ls -lht ~/.local/share/awsdash/logs/agents/

# Tail most recent agent
tail -f $(ls -t ~/.local/share/awsdash/logs/agents/*.log | head -1)
```

### Key Log Patterns to Search For

**Agent Creation**:
```bash
grep "Requesting agent creation" ~/.local/share/awsdash/logs/awsdash.log
grep "Created TaskWorker agent" ~/.local/share/awsdash/logs/awsdash.log
```

**UI Events**:
```bash
grep "UI event: Switch to agent" ~/.local/share/awsdash/logs/awsdash.log
```

**Tool Executions**:
```bash
grep "start_task" ~/.local/share/awsdash/logs/agents/*.log
```

**Errors**:
```bash
grep -i "error\|failed" ~/.local/share/awsdash/logs/awsdash.log
```

---

## Success Criteria Summary

For the task-agent system to be considered working:

- ✅ **Basic Spawning**: TaskManager can spawn a single worker
- ✅ **Multiple Workers**: TaskManager can spawn multiple workers in parallel
- ✅ **Planning Tools**: TaskManager uses think and todo tools effectively
- ✅ **UI Integration**: UI events trigger correctly, UI switches agents
- ✅ **Parent Tracking**: All workers have correct parent_id
- ✅ **Error Handling**: Errors are handled gracefully with clear messages
- ✅ **Concurrent Operation**: Multiple managers can work independently
- ✅ **Real AWS Tasks**: Workers can execute real AWS API calls successfully

---

## Known Limitations (As of Milestone 5)

1. **Result Passing**: Workers cannot yet pass results back to parent (deferred to future milestone)
2. **Auto-Switch on Completion**: UI doesn't automatically switch back to parent when worker completes
3. **Worker Lifecycle**: No automatic cleanup of completed workers
4. **Progress Tracking**: Parent cannot monitor worker progress in real-time

These will be addressed in future milestones.
