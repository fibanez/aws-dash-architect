# Multi-Agent Task Management System

Agent orchestration system enabling task managers to spawn and coordinate worker agents for parallel AWS operations.

## Overview

The multi-agent system consists of two agent types working together: **Task Manager** agents that break down complex requests and orchestrate work, and **Task Worker** agents that execute specific AWS operations using JavaScript APIs. This architecture enables parallel task execution and intelligent work delegation for complex AWS infrastructure analysis.

## How to Use

### Starting a Task Manager Agent

Task managers serve as the entry point for user requests:

```rust
// Task managers are created through the Agent Manager Window
// They automatically spawn workers as needed
let agent = AgentInstance::new(
    AgentId::new(),
    AgentType::TaskManager,
    metadata,
    logger,
);
```

### Spawning Worker Agents

Task managers use the `start_task` tool to spawn workers:

```rust
// Inside a task manager's conversation
"Use the start_task tool with these parameters:
- task_description: 'List all EC2 instances in production accounts'
- expected_output_format: 'JSON array with instance ID, type, state, launch time'
"
```

### Worker Agent Responses

Workers return raw data with execution timing:

```json
{
  "result": "[...array of EC2 instances...]",
  "execution_time_ms": 1234
}
```

## How it Works

### Agent Types

**Task Manager (`AgentType::TaskManager`)**
- Receives user requests and breaks them into tasks
- Spawns worker agents using the `start_task` tool
- Aggregates results from multiple workers
- Generates final reports for users
- Access to: `think` tool, `start_task` tool

**Task Worker (`AgentType::TaskWorker { parent_id }`)**
- Executes specific AWS operations using JavaScript
- Returns raw data to parent manager
- Terminates after completing task
- Access to: `execute_javascript` tool

### Communication Flow

```
User Request
    ↓
Task Manager Agent
    ├─→ start_task(task_1) → Worker Agent 1
    │                           ↓
    │                        Result 1
    ├─→ start_task(task_2) → Worker Agent 2
    │                           ↓
    │                        Result 2
    └─→ Aggregate & Report
            ↓
        User Response
```

### Worker Lifecycle

1. **Creation**: Manager calls `start_task` tool
2. **Initialization**: Worker agent created with parent_id reference
3. **Execution**: Worker runs JavaScript to query AWS resources
4. **Completion**: Result sent through completion channel
5. **Cleanup**: Worker tab auto-closes after 30 seconds

### Synchronization Mechanism

Workers communicate completion through a channel-based system:

```rust
// Worker completion sent to channel
let completion = WorkerCompletion {
    worker_id,
    result,  // Ok(String) or Err(String)
    execution_time,
};
send_worker_completion(completion);

// Manager waits for completion with timeout
match wait_for_worker_completion(agent_id, Duration::from_secs(300)) {
    Ok(result) => /* process result */,
    Err(error) => /* handle timeout or error */,
}
```

## Agent Prompts and Behavior

### Task Manager Prompt Features

**Context-Aware Task Design**:
- Determines request category (POINT_QUERY, ENVIRONMENT_SURVEY, etc.)
- Plans task breakdown considering AWS service interdependencies
- Maximizes single-task power using JavaScript capabilities
- Provides comprehensive context when spawning workers

**Task Context Requirements**:
When spawning workers, managers must include:
- Original user request for context
- Specific task details (WHAT to accomplish, not HOW)
- Context from previous completed tasks
- Expected output format

### Task Worker Prompt Features

**Resource Discovery Pattern**:
Workers follow a three-step process when searching for resources:

1. Query without filters to inspect data structure
2. Examine which fields contain the target information
3. Apply filters using the discovered field names

**Example - Finding CloudFormation Stacks**:
```javascript
// Step 1: Query without filters
const allStacks = queryResources({
  accounts: null,
  regions: null,
  resourceTypes: ['AWS::CloudFormation::Stack']
});
console.log('Sample stack:', JSON.stringify(allStacks[0], null, 2));

// Step 2: Inspect structure
// Discover that stack name is in resourceId, not displayName

// Step 3: Filter correctly
const filtered = allStacks.filter(s =>
  s.resourceId.includes('PVRE') ||
  s.rawProperties.StackName?.includes('PVRE')
);
```

**Property Access Pattern**:
- Use `rawProperties` for AWS-specific fields (not `properties`)
- Use optional chaining (`?.`) for nullable fields
- Check `resourceId` for primary identifiers

### Date/Time Context

Both manager and worker prompts receive current date/time via placeholder replacement:

```rust
// Template in prompt
"The current date and time are {{CURRENT_DATETIME}}"

// Replaced at runtime
"The current date and time are 2025-11-24 15:30:00 UTC"
```

This enables agents to calculate relative time windows ("last 7 days", "this month").

## Tool Reference

### start_task Tool

**Purpose**: Spawn worker agent to execute AWS task

**Parameters**:
- `task_description` (required): High-level description of what to accomplish
- `expected_output_format` (optional): Description of expected output format

**Returns**:
```json
{
  "result": "Raw data from worker execution",
  "execution_time_ms": 1234
}
```

**Usage Guidelines**:
- Include original user request for context
- Describe WHAT to accomplish, not HOW
- Provide context from previous tasks
- Specify expected output format

**Good Example**:
```
User asked: "Find all production EC2 instances with high CPU usage"
Task: List all EC2 instances in accounts with "prod" in the name.
Context: This is step 1 of analyzing production infrastructure.
Expected output: JSON array with instance ID, type, state, launch time
```

**Bad Example**:
```
Use queryResources() API to call EC2
```
(Too implementation-focused, lacks context)

## Worker Progress Display

The system provides real-time progress tracking for worker agents within the manager's conversation flow.

### Inline Progress Updates

Worker tool execution appears inline in the manager's conversation:

```
[Worker executing: execute_javascript]
    Running...
```

After completion:
```
[Worker executing: execute_javascript]
    Completed (1.2s)
```

### Token Usage Tracking

Worker agents track and display token consumption:

```rust
// UI event for token updates
AgentUIEvent::WorkerTokensUpdated {
    worker_id: AgentId,
    parent_id: AgentId,
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
}
```

Token counts appear in the worker progress display, helping users understand LLM resource usage across the task tree.

### Callback Handler Architecture

The `WorkerProgressCallbackHandler` forwards tool events from workers to the UI:

```rust
pub struct WorkerProgressCallbackHandler {
    worker_id: AgentId,
    parent_id: AgentId,
}

#[async_trait]
impl CallbackHandler for WorkerProgressCallbackHandler {
    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        match event {
            ToolEvent::Started { name, .. } => {
                send_ui_event(AgentUIEvent::worker_tool_started(
                    self.worker_id, self.parent_id, name
                ));
            }
            ToolEvent::Completed { name, .. } => {
                send_ui_event(AgentUIEvent::worker_tool_completed(
                    self.worker_id, self.parent_id, name, true
                ));
            }
            ToolEvent::Failed { name, error, .. } => {
                send_ui_event(AgentUIEvent::worker_tool_completed(
                    self.worker_id, self.parent_id, name, false
                ));
            }
        }
        Ok(())
    }
}
```

### Key Files

- `src/app/agent_framework/worker_progress_handler.rs` - Callback handler implementation
- `src/app/agent_framework/ui_events.rs` - UI event definitions and channel management
- `src/app/dashui/agent_manager_window.rs` - Progress rendering in conversation view

## Logging and Debugging

### Worker Completion Logging

Worker completions are logged with parent information:

```rust
log::info!(
    target: "agent::worker_complete",
    "Sent worker {} completion to channel (parent: {}, execution time: {:?})",
    worker_id,
    parent_id,
    execution_time
);
```

### Tool Call Logging

start_task tool calls are logged with full parameters:

```rust
tracing::info!(
    target: "agent::start_task",
    parent_id = %parent_id,
    "start_task TOOL CALL:\n  Task Description: {}\n  Expected Output Format: {:?}",
    input.task_description,
    input.expected_output_format
);
```

### Per-Agent Logs

Each agent maintains a dedicated log file:
- Location: `~/.local/share/awsdash/logs/agents/agent-{uuid}.log`
- Contains: Conversations, tool executions, model interactions, lifecycle events

## Worker Tab Management

### Auto-Close Behavior

Worker tabs implement smart auto-close functionality:

```rust
struct WorkerTabMetadata {
    completed_at: Option<Instant>,      // When worker finished
    last_viewed_at: Option<Instant>,    // Last user interaction
    auto_close_seconds: u32,            // Default: 30 seconds
}
```

**Auto-close logic**:
- Starts 30-second timer when worker completes
- Resets timer when user views the tab
- Tab closes when timer expires
- Timer uses `last_viewed_at` or `completed_at` as reference

### Worker Tab Creation

Workers are created but not focused:

```rust
// Create worker tab metadata (but don't change focus)
self.worker_tabs.insert(agent_id, WorkerTabMetadata::new());
log::info!("Created worker tab for agent {} (not focused)", agent_id);
```

Users can manually switch to worker tabs to view execution details.

## Configuration

### Application Storage

Application state is saved to consistent directory:
- Location: `~/.local/share/awsdash/app.ron`
- Contains: Theme preference, status bar settings
- egui internal UI state: Disabled (cleared on startup)

### UI State Persistence

egui's internal memory data is cleared to prevent bloat:

```rust
// Disable egui's internal UI state persistence
cc.egui_ctx.memory_mut(|mem| {
    mem.data.clear();
});
```

This prevents app.ron from growing to megabytes with collapsible states, scroll positions, and text cursors.

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_start_task_with_context() {
    // Initialize channels
    init_agent_creation_channel();
    init_ui_event_channel();

    // Set up agent context
    let parent_id = AgentId::new();
    set_current_agent_id(parent_id);

    let tool = StartTaskTool::new();
    let input = json!({
        "task_description": "List all EC2 instances in production",
        "expected_output_format": "JSON array with instance details"
    });

    // Test tool execution
    let result = tool.execute(Some(input), None).await;
    // Assertions...
}
```

### Integration Testing

Test the full workflow:
1. Create task manager agent
2. Send user request
3. Verify worker spawning
4. Check result aggregation
5. Validate logging output

## Related Documentation

- [Agent Framework V2](agent-framework-v2.md) - Previous agent implementation
- [Code Execution Tool](code-execution-tool.md) - JavaScript execution details
- [Source Code](../src/app/agent_framework/tools/start_task.rs) - start_task tool implementation
- [Agent Instance](../src/app/agent_framework/agent_instance.rs) - Core agent logic
- [Task Manager Prompt](../src/app/agent_framework/prompts/task_manager.rs) - Manager system prompt
- [Task Worker Prompt](../src/app/agent_framework/prompts/task_worker.rs) - Worker system prompt
