# Multi-Agent System: Design Decisions and Q&A

**Document Purpose**: Captures key architectural decisions, Q&A discussions, and rationale from the multi-agent task management system implementation.

**Implementation Date**: November 2025
**Status**: Complete and merged

---

## Table of Contents

1. [Core Architectural Decisions](#core-architectural-decisions)
2. [Agent Type System](#agent-type-system)
3. [Tool Design Decisions](#tool-design-decisions)
4. [Communication Architecture](#communication-architecture)
5. [UI State Management](#ui-state-management)
6. [Prompt Engineering](#prompt-engineering)
7. [Storage and Persistence](#storage-and-persistence)
8. [Key Implementation Patterns](#key-implementation-patterns)

---

## Core Architectural Decisions

### Decision 1: Single AgentInstance with AgentType Enum

**Question**: Should we create separate structs for TaskManager and TaskWorker, or use a single struct with an enum?

**Options Considered**:
- **Option A**: Single `AgentInstance` struct with `AgentType` enum
- **Option B**: Separate `TaskManagerAgent` and `TaskWorkerAgent` structs
- **Option C**: Trait-based polymorphism with `dyn Agent`

**Decision**: Option A - Single struct with enum

**Rationale**:
- Type safety with explicit behavior differentiation
- Compile-time guarantees for parent-child relationships
- Simpler state management in AgentManagerWindow
- Follows Rust stdlib pattern (e.g., `std::net::IpAddr`)
- Easier to add new agent types in the future
- Less code duplication

**Trade-offs**:
- Some agent-type-specific fields may be `Option<T>` (acceptable)
- Match statements required in some methods (good for exhaustiveness)

**Implementation Location**: `src/app/agent_framework/agent_types.rs`

```rust
pub enum AgentType {
    TaskManager,
    TaskWorker { parent_id: AgentId },
}
```

---

### Decision 2: Constitutional Law - "Let the LLM Decide"

**Principle**: This applies to **runtime agent behavior**, NOT architectural design.

**Agent Runtime Decisions (LLM Decides)**:
- How to handle errors (retry, skip, ask user, different approach)
- Whether to retry failed tasks and with what modifications
- How to aggregate results (narrative, JSON, table, mixed)
- When to use think tool for reasoning
- How to break down complex requests into tasks
- What to do when encountering ambiguous requests

**Implementation Design Decisions (Humans Decide)**:
- System architecture (AgentType enum, UI state machine)
- Tool schemas and APIs
- File organization and module structure
- Data structures and types
- Build/test strategies

**Runtime Behavior Guidelines**:
- Don't over-engineer with rigid formats - let agents adapt
- Trust the LLM's intelligence for decision-making
- Provide flexibility over constraints
- No hardcoded retry logic - agents analyze and decide
- No prescribed error handling flows - agents reason through problems

**Task Philosophy**: WHAT, not HOW
- Task descriptions specify the outcome (business goal)
- Do NOT specify implementation (AWS SDK calls, API methods)
- Task-agents figure out HOW using available JavaScript APIs

---

### Decision 3: Build from Scratch Philosophy

**Question**: Should we reuse existing tools or build new ones?

**Decision**: Build new tools from scratch (except `execute_javascript`)

**Rationale**:
- Clean separation between old and new tool systems
- Tools specifically designed for orchestration agent architecture
- Avoid legacy patterns that don't fit multi-agent model
- Clearer codebase with explicit intent

**Tools Built**:
- `think` - Structured reasoning space (Anthropic pattern)
- `todo-write` / `todo-read` - Task tracking (Claude Code schema)
- `start-task` - Worker spawning with result passing

---

## Agent Type System

### Agent Type Comparison

| Aspect | Task Manager | Task Worker |
|--------|-------------|-------------|
| **Tools** | think, start-task | execute_javascript |
| **Prompt** | Orchestration-focused | Task execution-focused |
| **Lifecycle** | Lives until user closes | Terminates after task completion |
| **Parent** | None (top-level) | Has parent_id (task-manager) |
| **UI Behavior** | Shows task progress, aggregates results | Shows work-in-progress |
| **Stop Behavior** | Gathers state, injects summary | Sends partial results to parent |

### Key Methods

```rust
impl AgentType {
    pub fn is_task_manager(&self) -> bool;
    pub fn parent_id(&self) -> Option<AgentId>;
}
```

---

## Tool Design Decisions

### Decision 4: Todo Storage Strategy

**Question**: How should task lists be stored?

**Options Considered**:
- **Option A**: Add `todo_list: Vec<TodoItem>` field to AgentInstance struct
- **Option B**: Use thread-local storage with agent ID key
- **Option C**: Use Arc<Mutex<HashMap<AgentId, Vec<TodoItem>>>> shared state

**Decision**: Option A (originally selected, later removed)

**Rationale**:
- Simplest, most explicit
- Follows Rust patterns
- No concurrency concerns
- Easy to serialize/deserialize

**Note**: Todo tools were later commented out in favor of simpler workflow. Decision documented for historical reference.

---

### Decision 5: Think Tool - No-op with Logging

**Question**: Should the think tool have side effects?

**Decision**: No-op that logs thoughts only

**Rationale**:
- Follows Anthropic's research (54% improvement in complex scenarios)
- Provides debugging visibility
- No state changes = safer
- Encourages agent to pause and reason
- Logs preserved in agent log files

**Implementation**: `src/app/agent_framework/tools/think.rs`

---

### Decision 6: Start-Task Tool Architecture Challenge

**Problem**: start-task tool needs to create agents, but tools are instantiated in AgentInstance which has no access to AgentManagerWindow.

**Solution**: Channel-based communication

**Architecture**:
```
[StartTaskTool] --request--> [Global Channel] <--poll-- [AgentManagerWindow]
                                                        (creates agent)
[StartTaskTool] <-response-- [Response Channel] <-----  [AgentManagerWindow]
```

**Benefits**:
- Decouples tool execution from UI management
- No circular dependencies
- Thread-safe communication
- Tools remain pure functions

**Implementation Files**:
- `src/app/agent_framework/agent_creation.rs` - Request/response channels
- `src/app/agent_framework/tools/start_task.rs` - Tool using channels

---

## Communication Architecture

### Decision 7: Worker Completion Delivery

**Question**: How should worker results get back to the parent agent?

**Options Considered**:
1. **User messages**: Worker result appears as new user message to parent
2. **Tool results**: Worker result appears as tool result from start_task
3. **Callback functions**: Pass closure to worker
4. **Database/file**: Write results to shared storage

**Decision**: Option 2 - Tool results via completion channel

**Rationale**:
- Preserves LLM conversation flow (tool call → tool result)
- Worker results appear in proper context
- No "unexpected user message" confusion
- Blocking wait pattern: start_task blocks until worker completes

**Implementation**:
```rust
// start_task tool blocks waiting for completion
pub fn wait_for_worker_completion(worker_id: AgentId) -> Result<String, String>

// Worker sends completion when done
pub fn send_worker_completion(completion: WorkerCompletion)
```

**Location**: `src/app/agent_framework/worker_completion.rs`

---

### Decision 8: UI Event Channel

**Question**: How should tools trigger UI changes without window access?

**Decision**: Global UI event channel

**Events**:
- `SwitchToAgent(AgentId)` - Show a specific agent
- `SwitchToParent(AgentId)` - Switch back to parent
- `AgentCompleted(AgentId)` - Worker finished

**Pattern**:
```rust
// Tools send events
let sender = get_ui_event_sender();
sender.send(AgentUIEvent::SwitchToAgent(worker_id))?;

// UI polls and processes
let receiver = get_ui_event_receiver();
while let Ok(event) = receiver.try_recv() {
    match event {
        AgentUIEvent::SwitchToAgent(id) => { /* switch */ }
        // ...
    }
}
```

**Location**: `src/app/agent_framework/ui_events.rs`

---

## UI State Management

### Decision 9: Worker Tab Auto-Close Timers

**Question**: When should worker tabs be closed?

**Decision**: 30-second auto-close after completion with user interaction reset

**Behavior**:
- Worker completes → timer starts (30 seconds)
- User clicks worker tab → timer resets
- Timer expires → worker removed, switch to parent
- Multiple workers have independent timers

**Implementation**:
```rust
struct WorkerTabMetadata {
    completed_at: Option<Instant>,
    last_interaction: Instant,
    auto_close_seconds: u32,
}
```

**UI Feedback**: Tab label shows "Worker 1 - autoclose 25" (countdown)

**Location**: `src/app/dashui/agent_manager_window.rs`

---

### Decision 10: Manager Tab Always Visible

**Question**: Should manager tab be hidden when no workers exist?

**Decision**: Manager tab always visible for TaskManager agents

**Rationale**:
- Consistent UI experience
- No visual "popping" as workers spawn/terminate
- User always knows where to find the main conversation
- Clear distinction between manager and worker tabs

---

## Prompt Engineering

### Decision 11: JavaScript Power Maximization

**Philosophy**: Workers can do complex multi-step operations in ONE task

**Key Insight**: JavaScript enables:
- Filtering: `resources.filter(r => r.rawProperties.InstanceType === 't3.micro')`
- Sorting: `resources.sort((a, b) => new Date(a.created) - new Date(b.created))`
- Mapping: `instances.map(i => ({ id: i.resourceId, type: i.rawProperties.InstanceType }))`
- Aggregation: `resources.reduce((acc, r) => { acc[r.region] = (acc[r.region] || 0) + 1; return acc; }, {})`

**Task Design Guidance**:

✅ **GOOD - One Task** (combines query + filter + sort + count):
```
Task: "Find all production Lambda functions created in last 30 days, sorted by creation date"

JavaScript (one execution):
- Find production accounts
- Query Lambda functions
- Filter by date (30 days)
- Sort by creation time
- Count by region
```

❌ **BAD - Three Tasks** (unnecessarily split):
- Task 1: "Get all Lambda functions"
- Task 2: "Filter functions by date"
- Task 3: "Sort by creation date"

**SPLIT tasks only when**:
1. Truly independent operations (no dependencies)
2. Error isolation needed (one might fail)
3. Results inform next steps (manager needs to decide)

---

### Decision 12: Autonomous Operation Model

**Key Changes to Prompts**:

**Before**:
- Agents talked to humans: "I will query the resources for you"
- No awareness of autonomous loop
- Unclear about parent-worker relationship

**After**:
- Self-talk: "I need to query resources with filter"
- Explicit autonomous operation explanation
- Clear parent-worker communication model
- XML structured outputs: `<thinking>`, `<summary>`, `<result>`

**Task Manager Prompt**:
- ~250 lines (was 35 lines)
- Complete JavaScript API documentation
- "JavaScript Secret Weapon" section
- Task design philosophy (combine vs. split)
- Workflow examples

**Task Worker Prompt**:
- ~180 lines (was 95 lines)
- XML output structure
- Self-talk examples
- Complete data requirement (not summaries)
- Multi-step JavaScript examples

**Location**: `src/app/agent_framework/prompts/`

---

### Decision 13: XML Tags for Structured Outputs

**Question**: How should agents format responses?

**Decision**: Use XML tags based on Anthropic best practices

**Tags**:
- `<thinking>` - Agent reasoning (for manager)
- `<summary>` - High-level summary (2-3 sentences)
- `<result>` - Complete data (JSON, tables, formatted)
- `<error>` - Error messages with context

**Example Worker Response**:
```xml
<summary>
Found 112 t3.micro EC2 instances in 3 production accounts across 5 regions
</summary>

<result>
[
  { "resourceId": "i-123...", "region": "us-east-1", ... },
  ... (all 112 instances)
]
</result>
```

**Benefits**:
- Claude models trained to recognize XML
- Easy parsing
- Clear structure
- Separates reasoning from data

---

### Decision 14: Instruction Template Removal

**Question**: Should we prepend instructions to every user message?

**Decision**: No - all instructions in system prompt only

**Before**:
```rust
let instruction_template = "<critical_instructions>...</critical_instructions>";
let full_message = format!("{}{}", instruction_template, user_message);
```

**After**:
```rust
// System prompt contains all instructions
match agent.execute(&user_message).await {
```

**Rationale**:
- Reduces token usage (~400 tokens per message)
- Avoids confusion in autonomous loop
- System prompt is the right place for standing instructions
- Cleaner conversation history

---

## Storage and Persistence

### Decision 15: Agent Logger Sharing

**Question**: Should workers log to their own file or parent's file?

**Decision**: Workers share parent's log file (not yet implemented)

**Rationale**:
- Complete conversation flow in one place
- Easier debugging
- Parent context visible in worker operations
- No need to follow multiple files

**Expected Log Format**:
```
[Parent TaskManager Log: agent-xxx.log]

[10:00:00] User: List all EC2 instances
[10:00:01] Assistant: Spawning worker...
[10:00:02] Tool: start_task → Worker agent-yyy created

====== Worker Agent: Task Worker 1 (agent-yyy) ======
[10:00:03] Initial Task: List all EC2 instances
[10:00:04] Tool: execute_javascript
[10:00:06] Result: Found 5 instances
====== End Worker Agent ======

[10:00:08] Parent: Received results from worker
```

**Implementation**: `src/app/agent_framework/agent_logger.rs`

**Status**: Pending implementation (tracked in LOGGING_IMPROVEMENTS.md)

---

## Key Implementation Patterns

### Pattern 1: Tool Factory

**Question**: How to provide different tools to different agent types?

**Pattern**: Tool factory function per agent type

```rust
fn get_tools_for_type(agent_type: &AgentType) -> Vec<Box<dyn Tool>> {
    match agent_type {
        AgentType::TaskManager => vec![
            Box::new(ThinkTool::new()),
            Box::new(StartTaskTool::new()),
        ],
        AgentType::TaskWorker { .. } => vec![
            Box::new(ExecuteJavaScriptTool::new()),
        ],
    }
}
```

---

### Pattern 2: Channel-Based Decoupling

**Pattern**: Use global channels to decouple components

**Examples**:
1. **UI Events**: Tools → AgentManagerWindow
2. **Agent Creation**: StartTaskTool ↔ AgentManagerWindow
3. **Worker Completion**: Worker → Parent (via tool result)

**Benefits**:
- No circular dependencies
- Thread-safe
- Clean separation of concerns
- Easy to test

---

### Pattern 3: Test-Driven Development (TDD)

**Approach Used Throughout**:
1. Write failing test
2. Implement minimal code to pass
3. Verify test passes
4. Refactor if needed
5. Move to next test

**Compilation Checkpoints**:
- After each phase: `cargo check`
- After each milestone: `cargo test`
- Before merge: `./scripts/test-chunks.sh fast`

---

### Pattern 4: Condvar for Blocking Operations

**Problem**: start_task needs to block until worker completes

**Solution**: Condvar-based completion registry

```rust
// Register worker as pending
let condvar = register_pending_worker(worker_id);

// Wait for completion (with timeout)
let result = condvar.wait_timeout(lock, Duration::from_secs(300));

// Worker sends completion
send_worker_completion(WorkerCompletion { worker_id, result, ... });
// ^ This notifies the condvar
```

**Benefits**:
- Blocking wait pattern (tool doesn't return until worker done)
- Timeout protection (5 minutes)
- No busy polling
- Standard Rust concurrency pattern

**Location**: `src/app/agent_framework/worker_completion.rs`

---

## Research References

All prompt engineering decisions based on official Anthropic research:

1. **Building Effective AI Agents**
   https://www.anthropic.com/research/building-effective-agents
   - Agent patterns and best practices
   - Think tool 54% improvement
   - Start simple, iterate

2. **Multi-Agent Research System**
   https://www.anthropic.com/engineering/multi-agent-research-system
   - Orchestrator-worker pattern
   - 90.2% performance improvement
   - Lightweight references between agents

3. **XML Tags for Prompts**
   https://docs.claude.com/en/docs/build-with-claude/prompt-engineering/use-xml-tags
   - Claude models trained on XML
   - Benefits: clarity, accuracy, parseability
   - Recommended tags for outputs

4. **Effective Context Engineering**
   https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
   - Context window management
   - Few-shot prompting patterns
   - High-signal token selection

---

## Migration Notes

### Breaking Changes Introduced

1. **AgentInstance Constructor**:
   - Added `agent_type: AgentType` parameter
   - Call sites: 2 locations updated in agent_manager_window.rs

2. **Tool Registry**:
   - Tools now type-specific via `get_tools_for_type()`
   - Old tools no longer mixed with new tools

3. **System Prompts**:
   - Moved to centralized prompts module
   - Type-specific prompts loaded dynamically

### Backward Compatibility

**Preserved**:
- Existing `execute_javascript` tool unchanged
- AgentManagerWindow API surface unchanged
- Agent log file format extended (not changed)

**Not Preserved**:
- Generic "agent" concept replaced with typed agents
- Some old tools deprecated (todo-related)

---

## Future Considerations

### Not Yet Implemented

1. **Worker Auto-Close Logic**:
   - `WorkerTabMetadata` exists
   - `process_worker_auto_close()` not yet implemented
   - Tabs don't actually auto-close after 30 seconds

2. **Shared Logger**:
   - Design complete
   - Implementation pending
   - Workers currently log to separate files

3. **Tab Keyboard Navigation**:
   - Tab key cycling mentioned in specs
   - Not implemented in initial version

### Potential Future Enhancements

1. **Agent Type Registry**:
   - Currently: Hardcoded enum
   - Future: Plugin-based agent type registration

2. **Tool Marketplace**:
   - Currently: Tools hardcoded per agent type
   - Future: Dynamic tool loading, user-defined tools

3. **Result Streaming**:
   - Currently: Workers return complete result
   - Future: Stream partial results as they compute

4. **Worker Pooling**:
   - Currently: New agent per task
   - Future: Reuse idle workers for new tasks

---

## Conclusion

The multi-agent task management system represents a fundamental shift from single-agent to orchestrated multi-agent architecture. Key success factors:

1. **Clear type system** (AgentType enum)
2. **Channel-based decoupling** (no circular dependencies)
3. **Anthropic best practices** (autonomous operation, XML outputs, JavaScript power)
4. **Test-driven development** (comprehensive test coverage)
5. **Flexibility over rigidity** ("let the LLM decide" at runtime)

This document captures the "why" behind the implementation, preserving design rationale for future maintainers and contributors.

---

**Document Metadata**:
- Created: 2025-11-24
- Based on: MILESTONE_1-5_IMPLEMENTATION.md, multi-agent-improvements.md, TASK-AGENT.md
- Status: Historical record post-merge
- Location: Saved before worktree deletion
