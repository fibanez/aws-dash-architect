# Task Manager Agent - Comprehensive Specification

**Document Version:** 1.0
**Date:** 2025-11-20
**Status:** Planning Phase

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Current System vs New System](#current-system-vs-new-system)
4. [Component Specifications](#component-specifications)
5. [UI State Machine](#ui-state-machine)
6. [Error Handling](#error-handling)
7. [Implementation Milestones](#implementation-milestones)
8. [File Organization](#file-organization)
9. [Testing Strategy](#testing-strategy)
10. [Integration Points](#integration-points)

---

## Executive Summary

### Goal
Transform the current single-agent system into a **task orchestration system** where a task-manager-agent (orchestrator) decomposes user requests into parallel AWS tasks and delegates them to task-agents (workers).

### Key Principles

**Constitutional Law (Agent Runtime Behavior):** Let the LLM decide if in doubt

**IMPORTANT:** This principle applies to how **agents behave at runtime** when making decisions, NOT to architectural design decisions during implementation. Specifically:

**Agent Runtime Decisions (LLM Decides):**
- How to handle errors (retry, skip, ask user, different approach)
- Whether to retry failed tasks and with what modifications
- How to aggregate results (narrative, JSON, table, mixed)
- When to use think tool for reasoning
- How to break down complex requests into tasks
- What to do when encountering ambiguous requests

**Implementation Design Decisions (Humans Decide):**
- System architecture (AgentType enum, UI state machine)
- Tool schemas and APIs
- File organization and module structure
- Data structures and types
- Build/test strategies

**Runtime Behavior Guidelines:**
- Don't over-engineer with rigid formats - let agents adapt
- Trust the LLM's intelligence for decision-making
- Provide flexibility over constraints
- No hardcoded retry logic - agents analyze and decide
- No prescribed error handling flows - agents reason through problems

**Task Philosophy:** WHAT, not HOW
- Task descriptions specify the outcome (business goal)
- Do NOT specify implementation (AWS SDK calls, API methods)
- Task-agents figure out HOW using available JavaScript APIs

### High-Level Flow

```
User: "Analyze my AWS infrastructure"
    ↓
Task-Manager-Agent:
    1. Uses think tool → "Need to gather EC2, S3, RDS data"
    2. Uses todo-write → Creates 3 tasks
    3. Uses start-task → Spawns 3 task-agents in parallel
    ↓
Task-Agent-1: Lists EC2 instances (using execute_javascript)
Task-Agent-2: Lists S3 buckets (using execute_javascript)
Task-Agent-3: Lists RDS databases (using execute_javascript)
    ↓
Task-Manager-Agent:
    - Receives results from all 3 task-agents
    - Aggregates and presents unified analysis
    - Responds to user
```

---

## Architecture Overview

### Agent Type Hierarchy

```rust
pub enum AgentType {
    TaskManager,                          // Orchestrator
    TaskWorker { parent_id: AgentId },   // Worker with parent reference
}

pub struct AgentInstance {
    id: AgentId,
    agent_type: AgentType,  // NEW: Determines behavior
    // ... existing fields
}
```

**Design Decision:** Single `AgentInstance` struct with `AgentType` enum (Option A)
- **Rationale:** Type safety, explicit behavior differentiation, compile-time guarantees
- **Benefit:** Different lifecycle/UI/communication behavior per agent type

### Agent Type Comparison

| Aspect | Task-Manager-Agent | Task-Agent (Worker) |
|--------|-------------------|---------------------|
| **Tools** | think, todo-write, todo-read, start-task | execute_javascript |
| **Prompt** | Orchestration-focused | Task execution-focused |
| **Lifecycle** | Lives until user closes | Terminates after task completion |
| **Parent** | None (top-level) | Has parent_id (task-manager) |
| **UI Behavior** | Shows task progress, aggregates results | Shows work-in-progress |
| **Stop Behavior** | Gathers state, injects summary | Sends partial results to parent |

---

## Current System vs New System

### Current System

**Architecture:**
```
User → AgentInstance (execute_javascript) → AWS APIs → Response
```

**Characteristics:**
- Single agent per conversation
- Direct JavaScript execution
- Simple 1:1 user-agent relationship
- No task decomposition

**File:** `src/app/agent_framework/agent_instance.rs` (lines 1-500)

### New System

**Architecture:**
```
User → Task-Manager-Agent (orchestrator)
           ↓ start-task
       ┌───┴────┬────────┐
       ↓        ↓        ↓
   Task-Agent-1  Task-Agent-2  Task-Agent-3
       │        │        │
       └────┬───┴───┬────┘
            ↓       ↓
      Results → Task-Manager → Aggregated Response
```

**Characteristics:**
- Hierarchical agent system (parent-child)
- Task decomposition and parallel execution
- Dynamic UI switching between agents
- Intelligent error recovery

**Changes Required:**
1. Add `AgentType` enum to `AgentInstance`
2. Create 4 new tools (think, todo-write, todo-read, start-task)
3. Implement UI state machine for agent switching
4. Add parent-child communication mechanism
5. Implement stop/resume with context injection

---

## Component Specifications

### 1. AgentType Enum

**File:** `src/app/agent_framework/agent_types.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    /// Orchestrator agent that decomposes tasks and manages workers
    TaskManager,

    /// Worker agent that executes individual tasks
    /// Contains reference to parent task-manager
    TaskWorker {
        parent_id: AgentId
    },
}

impl AgentType {
    /// Check if this is a task-manager agent
    pub fn is_task_manager(&self) -> bool {
        matches!(self, AgentType::TaskManager)
    }

    /// Get parent agent ID if this is a worker
    pub fn parent_id(&self) -> Option<AgentId> {
        match self {
            AgentType::TaskManager => None,
            AgentType::TaskWorker { parent_id } => Some(*parent_id),
        }
    }
}
```

**Integration:** Update `AgentInstance` struct to include `agent_type: AgentType` field

---

### 2. Task-Manager Tools

#### Tool 1: think (Anthropic's Pattern)

**File:** `src/app/agent_framework/tools/think.rs`

**Purpose:** No-op tool that gives agent structured space for reasoning

**Schema:**
```json
{
  "name": "think",
  "description": "Use this tool when you need to pause and think through complex reasoning or planning. It will not obtain new information or change anything, but will log your thought process. Use it when complex reasoning, planning, or reviewing previous tool results is needed.",
  "input_schema": {
    "type": "object",
    "properties": {
      "thought": {
        "type": "string",
        "description": "Your reasoning, analysis, or planning thoughts"
      }
    },
    "required": ["thought"]
  }
}
```

**Implementation:**
```rust
#[async_trait]
impl Tool for ThinkTool {
    async fn execute(&self, parameters: Option<Value>, _ctx: Option<&AgentContext>) -> Result<ToolResult, ToolError> {
        let input: ThinkInput = serde_json::from_value(parameters.unwrap())?;

        // Log to agent logger
        if let Some(logger) = get_current_agent_logger() {
            logger.log_system_message(&format!("THINKING: {}", input.thought));
        }

        // Return no-op result
        Ok(ToolResult::success(json!({
            "status": "thought_recorded",
            "message": "Thought logged successfully"
        })))
    }
}
```

**When to Use:**
- Before creating tasks (analyze user request)
- After receiving task results (decide next steps)
- When errors occur (analyze recovery strategy)
- Before aggregating results (plan presentation)

**Performance Impact:** 54% improvement in complex multi-step scenarios (Anthropic research)

---

#### Tool 2: todo-write (Claude Code's Proven Schema)

**File:** `src/app/agent_framework/tools/todo_write.rs`

**Purpose:** Create and manage task list for tracking work

**Schema:**
```json
{
  "name": "todo_write",
  "description": "Create and manage your task list. Use this to track tasks you plan to execute. Always update the entire todo list (not individual items). Limit ONE task to 'in_progress' at a time.",
  "input_schema": {
    "type": "object",
    "required": ["todos"],
    "properties": {
      "todos": {
        "type": "array",
        "description": "The complete updated todo list",
        "items": {
          "type": "object",
          "required": ["content", "status", "activeForm"],
          "properties": {
            "content": {
              "type": "string",
              "minLength": 1,
              "description": "Imperative form: what needs to be done (e.g., 'List EC2 instances')"
            },
            "activeForm": {
              "type": "string",
              "minLength": 1,
              "description": "Present continuous form: what's being done (e.g., 'Listing EC2 instances')"
            },
            "status": {
              "type": "string",
              "enum": ["pending", "in_progress", "completed"],
              "description": "Current task status"
            }
          }
        }
      }
    }
  }
}
```

**Implementation:**
```rust
pub struct TodoWriteTool;

#[derive(Debug, Deserialize)]
struct TodoWriteInput {
    todos: Vec<TodoItem>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TodoItem {
    content: String,
    #[serde(rename = "activeForm")]
    active_form: String,
    status: TodoStatus,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[async_trait]
impl Tool for TodoWriteTool {
    async fn execute(&self, parameters: Option<Value>, _ctx: Option<&AgentContext>) -> Result<ToolResult, ToolError> {
        let input: TodoWriteInput = serde_json::from_value(parameters.unwrap())?;

        // Validate: only ONE task should be in_progress
        let in_progress_count = input.todos.iter()
            .filter(|t| matches!(t.status, TodoStatus::InProgress))
            .count();

        if in_progress_count > 1 {
            return Err(ToolError::validation_error(
                "Only one task should be 'in_progress' at a time"
            ));
        }

        // Store in agent context (thread-local or agent state)
        set_current_todo_list(input.todos.clone());

        // Log to agent logger
        if let Some(logger) = get_current_agent_logger() {
            logger.log_system_message(&format!("TODO LIST UPDATED: {} tasks", input.todos.len()));
        }

        Ok(ToolResult::success(json!({
            "status": "updated",
            "task_count": input.todos.len()
        })))
    }
}
```

**Rules (from Claude Code):**
1. **One in-progress**: Only ONE task with status "in_progress" at a time
2. **Immediate completion**: Mark task "completed" as soon as finished
3. **Full completion only**: Don't mark complete if errors/blockers exist
4. **Two forms required**: Both imperative (content) and continuous (activeForm)

**When to Use:**
- After analyzing user request (create initial task list)
- After spawning task-agent (mark task as "in_progress")
- After task-agent completes (mark task as "completed")
- When task-agent fails (decide whether to mark "completed" with error or retry)

---

#### Tool 3: todo-read (Claude Code's Proven Schema)

**File:** `src/app/agent_framework/tools/todo_read.rs`

**Purpose:** Retrieve current task list status

**Schema:**
```json
{
  "name": "todo_read",
  "description": "Retrieve the current todo list. Takes no parameters - leave input blank or use empty object {}.",
  "input_schema": {
    "type": "object",
    "properties": {}
  }
}
```

**Implementation:**
```rust
pub struct TodoReadTool;

#[async_trait]
impl Tool for TodoReadTool {
    async fn execute(&self, _parameters: Option<Value>, _ctx: Option<&AgentContext>) -> Result<ToolResult, ToolError> {
        // Retrieve from agent context (thread-local or agent state)
        let todos = get_current_todo_list();

        // Format for display
        let result = json!({
            "todos": todos,
            "summary": {
                "total": todos.len(),
                "pending": todos.iter().filter(|t| matches!(t.status, TodoStatus::Pending)).count(),
                "in_progress": todos.iter().filter(|t| matches!(t.status, TodoStatus::InProgress)).count(),
                "completed": todos.iter().filter(|t| matches!(t.status, TodoStatus::Completed)).count(),
            }
        });

        Ok(ToolResult::success(result))
    }
}
```

**When to Use:**
- At conversation start (check if tasks exist from previous session)
- Before creating new tasks (understand current state)
- After task completion (verify progress)
- When user asks "what's the status?"

---

#### Tool 4: start-task (NEW - Core Orchestration Tool)

**File:** `src/app/agent_framework/tools/start_task.rs`

**Purpose:** Spawn a new task-agent to execute a specific task

**Schema:**
```json
{
  "name": "start_task",
  "description": "Spawn a new task-agent to execute an AWS-related task. Describe WHAT to accomplish, NOT HOW to do it. The task-agent has built-in JavaScript APIs and manages all technical implementation details.\n\nGood task: 'List all EC2 instances in production account'\nBad task: 'Use queryResources() API to call EC2...'\n\nYou can specify the expected output format to ensure the task-agent returns data in a useful structure.",
  "input_schema": {
    "type": "object",
    "required": ["task_description"],
    "properties": {
      "task_description": {
        "type": "string",
        "description": "High-level description of WHAT to accomplish (verb + subject + constraints). Do NOT specify implementation details.",
        "examples": [
          "List all EC2 instances in the production account",
          "Find S3 buckets larger than 100GB",
          "Analyze RDS databases for unused instances in us-east-1"
        ]
      },
      "expected_output_format": {
        "type": "string",
        "description": "Optional description of the expected output format (e.g., 'JSON array of instance objects with id, type, state, tags')",
        "examples": [
          "JSON array of instance objects with id, type, state, tags",
          "Table with columns: bucket name, size, region",
          "Summary statistics with total count and breakdown by type"
        ]
      }
    }
  }
}
```

**Implementation:**
```rust
pub struct StartTaskTool {
    // Reference to AgentManagerWindow for creating new agents
    agent_manager: Arc<Mutex<AgentManagerWindow>>,
}

#[derive(Debug, Deserialize)]
struct StartTaskInput {
    task_description: String,
    expected_output_format: Option<String>,
}

#[async_trait]
impl Tool for StartTaskTool {
    async fn execute(&self, parameters: Option<Value>, ctx: Option<&AgentContext>) -> Result<ToolResult, ToolError> {
        let input: StartTaskInput = serde_json::from_value(parameters.unwrap())?;

        // Get current agent ID (the task-manager)
        let current_agent_id = get_current_agent_id()?;

        // Create task-agent metadata
        let task_name = format!("Task: {}", truncate(&input.task_description, 50));
        let mut metadata = AgentMetadata::new(task_name, "Task execution agent");

        // Build task-agent prompt
        let mut task_prompt = TASK_WORKER_PROMPT.to_string();

        // Append task-specific instructions
        task_prompt.push_str(&format!("\n\n<task>\n{}\n</task>\n", input.task_description));

        // Append expected output format if provided
        if let Some(expected_format) = &input.expected_output_format {
            task_prompt.push_str(&format!(
                "\n<expected_output_format>\n{}\n</expected_output_format>\n",
                expected_format
            ));
        }

        // Create task-agent
        let mut agent_manager = self.agent_manager.lock().unwrap();
        let task_agent_id = agent_manager.create_agent(
            metadata,
            AgentType::TaskWorker { parent_id: current_agent_id },
            task_prompt,
        )?;

        // Send initial message to task-agent (starts execution)
        agent_manager.send_message_to_agent(
            task_agent_id,
            input.task_description.clone(),
        )?;

        // Signal UI to switch to task-agent
        send_ui_event(AgentUIEvent::SwitchToAgent(task_agent_id))?;

        // Log
        if let Some(logger) = get_current_agent_logger() {
            logger.log_system_message(&format!(
                "TASK STARTED: {} (agent_id: {})",
                input.task_description,
                task_agent_id
            ));
        }

        // Return task ID to task-manager
        Ok(ToolResult::success(json!({
            "task_id": task_agent_id.to_string(),
            "status": "started",
            "task_description": input.task_description,
        })))
    }
}
```

**Key Behaviors:**

1. **Agent Creation:**
   - Creates new `AgentInstance` with `AgentType::TaskWorker { parent_id }`
   - Uses task-worker prompt + task-specific instructions
   - Injects expected output format into prompt

2. **UI Switching:**
   - Sends `AgentUIEvent::SwitchToAgent(task_agent_id)` to UI
   - User sees task-agent's work in real-time

3. **Task Execution:**
   - Immediately sends task description as first message
   - Task-agent begins execution in background thread

4. **Parallel Execution:**
   - Uses stood's parallel execution (max 5 concurrent)
   - If 5 tasks running, 6th waits in queue

**Return Value:**
- `task_id`: Unique identifier for tracking
- `status`: Always "started" (asynchronous execution)
- `task_description`: Echo of what was requested

---

### 3. Task-Worker Tool

#### Tool: execute_javascript (Existing)

**File:** `src/app/agent_framework/tools/execute_javascript.rs` (no changes)

**Purpose:** Execute JavaScript code in V8 sandbox with AWS API bindings

**Current Implementation:** Lines 1-500 (fully functional)

**Available JavaScript APIs:**
- `listAccounts()` - List configured AWS accounts
- `listRegions()` - List available AWS regions
- `queryResources(options)` - Query AWS resources (93 services, 183 types)
- `queryCloudWatchLogEvents(params)` - Query CloudWatch Logs
- `getCloudTrailEvents(params)` - Get CloudTrail events

**No Changes Required** - This tool works perfectly for task-agents

---

### 4. Agent Prompts

#### Task-Manager System Prompt

**File:** `src/app/agent_framework/prompts/task_manager_prompt.rs`

```rust
pub const TASK_MANAGER_PROMPT: &str = r#"You are a task orchestration agent for AWS infrastructure analysis and management. Your role is to:

1. **Understand user requests** - Analyze what the user wants to accomplish
2. **Decompose into tasks** - Break complex requests into independent AWS tasks
3. **Orchestrate execution** - Spawn task-agents to execute tasks in parallel
4. **Aggregate results** - Combine task results into a unified response

# Available Tools

## Planning Tools

**think** - Use when you need to reason through complex planning or analysis
- Use before creating tasks to analyze the user request
- Use after receiving task results to decide next steps
- Use when errors occur to determine recovery strategy

**todo_write** - Manage your task list
- Create task list after analyzing user request
- Mark tasks as "in_progress" when spawning task-agent
- Mark tasks as "completed" when task-agent succeeds
- Only ONE task should be "in_progress" at a time

**todo_read** - Check current task list status
- Use at conversation start to check existing tasks
- Use before creating new tasks to understand current state

## Task Execution

**start_task** - Spawn a new task-agent to execute an AWS task
- Describe WHAT to accomplish (business goal), NOT HOW to implement
- Task-agents have built-in JavaScript APIs for AWS operations
- Do NOT specify AWS SDK methods, API calls, or authentication details
- Optionally specify expected output format for structured results

### Task Description Guidelines

✅ **GOOD Tasks (WHAT to accomplish):**
- "List all EC2 instances in the production account"
- "Find S3 buckets larger than 100GB in us-east-1"
- "Analyze RDS databases for unused instances"
- "Get CloudWatch logs for Lambda function errors in the last hour"

❌ **BAD Tasks (Specifying HOW):**
- "Use AWS SDK ec2.describeInstances() to list instances"
- "Call queryResources() with service='ec2' parameter"
- "Authenticate with IAM role and then query S3"
- "Execute JavaScript: queryResources({service: 's3'})"

**Remember:** Task-agents manage all technical implementation. You focus on the outcome.

# Workflow Pattern

1. **Analyze Request**
   - Use `think` tool to understand user's goal
   - Identify independent sub-tasks

2. **Create Task List**
   - Use `todo_write` to create task list
   - Each task should be independently executable
   - Tasks that can run in parallel should be separate items

3. **Execute Tasks**
   - Use `start_task` for each task
   - Spawn multiple tasks in parallel when possible (max 5 concurrent)
   - Mark task as "in_progress" in todo list

4. **Monitor Progress**
   - Task results will be sent back to you automatically
   - Use `think` tool to analyze results
   - Update todo list as tasks complete

5. **Handle Errors**
   - If task-agent reports error, use `think` to analyze
   - Decide: retry with different approach, skip task, or ask user
   - Update todo list based on decision

6. **Aggregate and Respond**
   - Once all tasks complete, use `think` to plan response format
   - Combine results (narrative, JSON, table - you decide)
   - Respond to user with unified analysis

# Important Rules

1. **Task Descriptions:**
   - Describe WHAT (business goal), not HOW (implementation)
   - Be specific about constraints (account, region, filters)
   - Let task-agent figure out API calls and authentication

2. **Parallel Execution:**
   - Spawn independent tasks in parallel (up to 5 concurrent)
   - Sequential tasks only when one depends on another's output

3. **Error Handling:**
   - Don't auto-retry - analyze error and decide strategy
   - Use `think` tool to understand what went wrong
   - Options: retry with refinement, try different approach, ask user

4. **Result Aggregation:**
   - You decide the best presentation format
   - Could be narrative, structured JSON, tables, or mixed
   - Consider what's most useful for the user's request

5. **AWS Scope:**
   - You are specialized for AWS operations only
   - For non-AWS questions, politely explain your scope

# Example Workflow

User: "Analyze my EC2 infrastructure in production"

1. think("User wants EC2 analysis. Need: list instances, check utilization, identify issues")

2. todo_write({
     todos: [
       {content: "List all EC2 instances", activeForm: "Listing EC2 instances", status: "pending"},
       {content: "Get CloudWatch metrics", activeForm: "Getting metrics", status: "pending"},
       {content: "Analyze instance utilization", activeForm: "Analyzing utilization", status: "pending"}
     ]
   })

3. start_task({
     task_description: "List all EC2 instances in production account",
     expected_output_format: "JSON array with id, type, state, tags"
   })

   start_task({
     task_description: "Get CPU utilization metrics for production EC2 instances",
     expected_output_format: "JSON array with instance_id, avg_cpu, max_cpu"
   })

4. [Wait for results...]

5. think("Received EC2 list (15 instances) and metrics. 3 instances have <10% utilization - suggest downsizing")

6. [Respond to user with aggregated analysis]
"#;
```

**Key Features:**
- Clear role definition
- Tool-by-tool documentation
- Workflow pattern guidance
- Example scenarios
- WHAT vs HOW emphasis

---

#### Task-Worker System Prompt

**File:** `src/app/agent_framework/prompts/task_worker_prompt.rs`

```rust
pub const TASK_WORKER_PROMPT: &str = r#"You are a task execution agent for AWS operations. You receive a specific task and must execute it using the available JavaScript APIs.

# Your Mission

You will receive a task description in <task> tags. Your job is to:
1. Understand the task requirements
2. Write JavaScript code using available APIs
3. Execute the code using the execute_javascript tool
4. Return the complete results (not just a summary)

# Available Tool

**execute_javascript** - Execute JavaScript code in a V8 sandbox with AWS API bindings

Available JavaScript APIs:
- `listAccounts()` - List configured AWS accounts
- `listRegions()` - List AWS regions
- `queryResources(options)` - Query AWS resources (93 services, 183 resource types)
- `queryCloudWatchLogEvents(params)` - Query CloudWatch Logs
- `getCloudTrailEvents(params)` - Get CloudTrail events
- `console.log(...)` - Log messages for debugging

See tool description for complete API documentation and examples.

# Critical Rules

## 1. Include Complete Data in Final Response

**CRITICAL:** Your final response MUST include the complete data from tool results, not just summaries.

❌ **BAD Examples:**
- "I found 5 EC2 instances." (no data)
- "The instances are listed above." (refers to hidden tool result)
- "See the query results." (data not shown)

✅ **GOOD Examples:**
- "I found 5 EC2 instances:\n```json\n[{full data here}]\n```"
- "Query results (15 S3 buckets):\n[complete bucket list]"

**Why:** The task-manager needs the actual data to aggregate results. Tool results are hidden from the parent agent.

## 2. Respect Expected Output Format

If you received <expected_output_format> instructions, format your final response accordingly:
- JSON array → return formatted JSON
- Table → return markdown table
- Summary → return statistics with counts

## 3. Ask User for Clarification When Needed

If the task is ambiguous or missing required information:
- Ask the user directly (your input goes to the UI)
- Examples: "Which AWS account?", "Which region?", "What time range?"
- The user will respond, and you'll receive their answer

## 4. Handle Errors Gracefully

If JavaScript execution fails:
- Report detailed error information
- Include context: what you were trying to do
- Suggest what went wrong and potential fixes
- Don't just say "error occurred" - be specific

## 5. Default Assumptions

- **No account specified?** Use `listAccounts()` and pick the first one, or ask user
- **No region specified?** Default to us-east-1 unless task implies otherwise
- **Ambiguous filters?** Ask user for clarification

# Example Task Execution

<task>List all EC2 instances in the production account</task>

<expected_output_format>JSON array of instance objects with id, type, state, tags</expected_output_format>

**Your Execution:**

1. Write JavaScript to query EC2 instances:
```javascript
const accounts = listAccounts();
const prodAccount = accounts.find(a => a.name.includes('prod') || a.alias === 'production');

if (!prodAccount) {
  console.log('No production account found. Available accounts:', accounts.map(a => a.name));
  throw new Error('Production account not found');
}

const instances = queryResources({
  service: 'ec2',
  resourceType: 'instance',
  accounts: [prodAccount.id],
  regions: ['us-east-1']
});

instances;
```

2. Execute using execute_javascript tool

3. Return complete results:

"I found 5 EC2 instances in the production account:

```json
[
  {
    "id": "i-1234567890abcdef0",
    "type": "t3.medium",
    "state": "running",
    "tags": {"Name": "web-server-1", "Environment": "production"}
  },
  {
    "id": "i-0987654321fedcba0",
    "type": "t3.small",
    "state": "running",
    "tags": {"Name": "api-server-1", "Environment": "production"}
  },
  ...
]
```

Task complete."

# AWS Scope

You are specialized for AWS operations only. If asked to perform non-AWS tasks, politely explain you can only help with AWS-related operations.

# Ready to Execute

Your task will be provided below in <task> tags. Execute it using the execute_javascript tool and return complete results.
"#;
```

**Key Features:**
- Clear mission statement
- Critical rules for data inclusion
- Error handling guidance
- Example execution flow
- Expected output format respect

---

## UI State Machine

### Overview

The UI implements an **automatic agent-switching state machine** that allows users to see task-agent work in real-time and cycle through multiple parallel tasks.

### Design: Option C - Queue with Tab Cycling

**Features:**
1. Auto-switch to first task when spawned
2. Show indicator: "Task 1 of 3 - Press Tab to cycle"
3. **Input waiting indicator:** "⚠️ Task 2 waiting for input - Press Tab"
4. Tab key cycles through active tasks
5. Auto-switch back to task-manager when all tasks complete

### State Machine Diagram

```
┌─────────────────┐
│  Task-Manager   │
│    (viewing)    │
└────────┬────────┘
         │ start_task called
         ↓
┌─────────────────┐
│   Task-Agent-1  │◄─────┐
│    (viewing)    │      │ Tab key cycles
└────────┬────────┘      │ through active
         │               │ task-agents
    completes            │
         │               │
    ┌────▼────┐          │
    │All tasks│──────────┘
    │complete?│
    └────┬────┘
         │ yes
         ↓
┌─────────────────┐
│  Task-Manager   │
│    (viewing)    │
└─────────────────┘
```

### UI Components

#### 1. Active Task Indicator

**Location:** Top of right pane in AgentManagerWindow

```rust
// In render_agent_chat()
if let Some(task_context) = get_task_context(agent) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!(
            "Task {} of {} - Press Tab to cycle",
            task_context.current_index + 1,
            task_context.total_tasks
        )).strong());

        // Show warning if any task is waiting for input
        if let Some(waiting_task) = task_context.find_waiting_for_input() {
            ui.label(
                RichText::new(format!("⚠️ Task {} waiting for input", waiting_task.index + 1))
                    .color(Color32::YELLOW)
            );
        }
    });
    ui.separator();
}
```

**Visual Example:**
```
┌─────────────────────────────────────────────────────┐
│ Task 1 of 3 - Press Tab to cycle                   │
│ ⚠️ Task 2 waiting for input                         │
├─────────────────────────────────────────────────────┤
│ [Task-Agent conversation appears here]              │
│                                                      │
└─────────────────────────────────────────────────────┘
```

#### 2. Tab Key Handler

**Location:** `src/app/dashui/agent_manager_window.rs`

```rust
impl AgentManagerWindow {
    pub fn handle_input(&mut self, ctx: &Context) {
        // Tab key: cycle through active task-agents
        if ctx.input(|i| i.key_pressed(Key::Tab)) {
            self.cycle_to_next_task_agent();
        }

        // Escape key: stop all agents
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.stop_all_agents_with_context();
        }
    }

    fn cycle_to_next_task_agent(&mut self) {
        let active_tasks = self.get_active_task_agents();
        if active_tasks.is_empty() {
            return;
        }

        // Find current task index
        let current_index = active_tasks.iter()
            .position(|id| Some(*id) == self.selected_agent_id)
            .unwrap_or(0);

        // Cycle to next (wrap around)
        let next_index = (current_index + 1) % active_tasks.len();
        let next_agent_id = active_tasks[next_index];

        self.select_agent(next_agent_id);
    }
}
```

#### 3. Auto-Switch on Task Start

**Location:** `src/app/agent_framework/tools/start_task.rs`

```rust
// In StartTaskTool::execute()
// After creating task-agent...

// Signal UI to switch to task-agent
send_ui_event(AgentUIEvent::SwitchToAgent(task_agent_id))?;
```

**UI Event Channel:**
```rust
pub enum AgentUIEvent {
    SwitchToAgent(AgentId),           // Switch to specific agent
    SwitchToParent(AgentId),           // Switch back to parent
    AgentCompleted(AgentId),           // Agent finished
}

// In AgentManagerWindow
pub fn process_ui_events(&mut self) {
    while let Ok(event) = self.ui_event_receiver.try_recv() {
        match event {
            AgentUIEvent::SwitchToAgent(agent_id) => {
                self.select_agent(agent_id);
            }
            AgentUIEvent::SwitchToParent(parent_id) => {
                self.select_agent(parent_id);
            }
            AgentUIEvent::AgentCompleted(agent_id) => {
                self.handle_agent_completion(agent_id);
            }
        }
    }
}
```

#### 4. Auto-Switch Back on Completion

**Location:** `src/app/agent_framework/agent_instance.rs`

```rust
impl AgentInstance {
    pub fn poll_response(&mut self) -> bool {
        match self.response_channel.1.try_recv() {
            Ok(ConversationResponse::Success(text)) => {
                self.messages.push_back(ConversationMessage::assistant(text));
                self.processing = false;

                // NEW: If this is a task-worker, notify parent
                if let AgentType::TaskWorker { parent_id } = self.agent_type {
                    // Send result to parent task-manager
                    self.send_result_to_parent(parent_id, &text);

                    // Signal UI to switch back to parent
                    send_ui_event(AgentUIEvent::SwitchToParent(parent_id))
                        .expect("Failed to send UI event");

                    // Mark self for termination
                    self.status = AgentStatus::Completed;
                }

                true
            }
            // ... error handling
        }
    }
}
```

#### 5. Detecting "Waiting for Input"

**Logic:**
```rust
impl AgentInstance {
    /// Check if agent is waiting for user input
    pub fn is_waiting_for_input(&self) -> bool {
        // Agent is NOT processing AND last message is from assistant
        !self.processing &&
        self.messages.back()
            .map(|m| matches!(m.role, ConversationRole::Assistant))
            .unwrap_or(false)
    }
}
```

**UI Indicator Update:**
```rust
// In AgentManagerWindow
fn find_tasks_waiting_for_input(&self) -> Vec<(usize, AgentId)> {
    self.get_active_task_agents()
        .iter()
        .enumerate()
        .filter_map(|(idx, agent_id)| {
            self.agents.get(agent_id)
                .filter(|agent| agent.is_waiting_for_input())
                .map(|_| (idx, *agent_id))
        })
        .collect()
}
```

### User Experience Flow

**Scenario:** User asks to analyze EC2, S3, and RDS

1. **Task-manager visible**
   - User: "Analyze my AWS infrastructure"
   - Task-manager uses think → Creates 3 tasks → Spawns task-agent-1

2. **Auto-switch to Task-Agent-1**
   - UI shows: "Task 1 of 3 - Press Tab to cycle"
   - User sees: Task-agent-1 writing JavaScript, querying EC2

3. **Task-Agent-1 needs input**
   - Task-agent-1: "Which account should I use?"
   - UI shows: Agent is waiting (not processing)
   - User types answer → goes to task-agent-1

4. **User presses Tab**
   - Switches to Task-Agent-2 (S3 query in progress)
   - UI shows: "Task 2 of 3 - Press Tab to cycle"

5. **Task-Agent-1 completes**
   - Auto-switches back to Task-Agent-1 briefly (shows completion)
   - Then switches to Task-Agent-2 (next active task)
   - Or back to task-manager if all complete

6. **All tasks complete**
   - Auto-switch back to task-manager
   - Task-manager aggregates results and responds

---

## Error Handling

### Philosophy

**Constitutional Law:** Let the task-manager LLM decide what to do with errors
- Don't auto-mark tasks as failed
- Don't auto-retry without analysis
- Provide comprehensive error context for intelligent decision-making

### Error Report Format

When task-agent encounters error, send detailed report to task-manager:

```json
{
  "task_id": "uuid-string",
  "task_description": "List all EC2 instances in production",
  "status": "error",
  "error_summary": "JavaScript execution timeout after 30 seconds",
  "error_details": {
    "type": "timeout",
    "message": "V8 execution exceeded 30 second limit",
    "context": {
      "last_successful_step": "Retrieved account list (3 accounts)",
      "failed_at": "Querying EC2 instances in 'production' account (account ID: 123456789012)",
      "partial_results": {
        "accounts_found": 3,
        "regions_checked": ["us-east-1"],
        "instances_retrieved": 0
      }
    },
    "suggestion": "The production account may have too many instances. Try querying one region at a time, or add pagination to the query."
  },
  "stack_trace": "Optional JavaScript stack trace if available"
}
```

### Error Types

#### 1. JavaScript Execution Errors

**Timeout:**
```json
{
  "type": "timeout",
  "message": "V8 execution exceeded 30 second limit",
  "suggestion": "Query returned too much data. Try narrowing scope (single region, specific account, time range limit)."
}
```

**Syntax Error:**
```json
{
  "type": "syntax_error",
  "message": "Unexpected token '}' at line 15",
  "suggestion": "JavaScript code has syntax error. Review code structure."
}
```

**Runtime Error:**
```json
{
  "type": "runtime_error",
  "message": "TypeError: Cannot read property 'id' of undefined",
  "suggestion": "Check for null/undefined values before accessing properties."
}
```

#### 2. AWS API Errors

**Permission Denied:**
```json
{
  "type": "permission_denied",
  "message": "Access Denied when querying EC2 instances",
  "suggestion": "AWS credentials may lack ec2:DescribeInstances permission. Try a different account or contact admin."
}
```

**Resource Not Found:**
```json
{
  "type": "not_found",
  "message": "Account 'production' not found in configured accounts",
  "suggestion": "Available accounts: dev, staging, test. Did you mean one of these?"
}
```

### Task-Manager Error Handling Flow

**Implementation in task-manager prompt:**

```
When you receive an error report from a task-agent:

1. Use `think` tool to analyze the error:
   - What was the task trying to accomplish?
   - Why did it fail?
   - Can it be retried with modifications?
   - Should we try a different approach?
   - Do we need to ask the user?

2. Decide on strategy:
   - **Retry with refinement:** Modify task description and spawn new task-agent
   - **Different approach:** Change the task entirely
   - **Ask user:** Request clarification or additional info
   - **Skip task:** Mark as completed with error note
   - **Abort all:** If error is fundamental (no credentials, wrong account)

3. Update todo list based on decision

4. Execute chosen strategy

Example:
- Task fails with timeout → think("Too much data") → Retry with region filter
- Task fails with permission error → think("No access") → Ask user for different account
- Task fails with syntax error → think("Bug in my task description") → Refine and retry
```

### Error Handling Example

**Scenario:** EC2 query times out

1. **Task-agent reports error:**
```json
{
  "status": "error",
  "error_summary": "JavaScript timeout after 30s",
  "suggestion": "Query returned too much data. Try one region at a time."
}
```

2. **Task-manager receives error and uses think:**
```
think("EC2 query timed out in production account. Suggestion says too much data.
Instead of querying all regions, I should spawn separate tasks per region: us-east-1, us-west-2, eu-west-1")
```

3. **Task-manager updates approach:**
```
todo_write({
  todos: [
    {content: "List EC2 in production/us-east-1", status: "pending", activeForm: "..."},
    {content: "List EC2 in production/us-west-2", status: "pending", activeForm: "..."},
    {content: "List EC2 in production/eu-west-1", status: "pending", activeForm: "..."},
  ]
})
```

4. **Spawns new task-agents with refined scope:**
```
start_task("List all EC2 instances in production account, us-east-1 region only")
start_task("List all EC2 instances in production account, us-west-2 region only")
start_task("List all EC2 instances in production account, eu-west-1 region only")
```

---

## Stop/ESC Key Behavior

### Design: Option D - Cancel All with Context Injection

**User Action:** Presses Stop button or ESC key

**Behavior:**
1. Cancel task-manager + all active task-agents
2. Gather state from all agents (what they were doing)
3. Inject comprehensive summary into task-manager's conversation
4. Switch UI back to task-manager
5. User can resume, cancel remaining tasks, or give new instructions

### Implementation

#### 1. Stop Handler

**Location:** `src/app/dashui/agent_manager_window.rs`

```rust
impl AgentManagerWindow {
    fn stop_all_agents_with_context(&mut self) {
        // 1. Gather state from all active agents
        let state_summary = self.gather_agent_state();

        // 2. Cancel all task-agents
        let task_manager_id = self.get_task_manager_id();
        for agent_id in self.get_active_task_agents() {
            if let Some(agent) = self.agents.get_mut(&agent_id) {
                agent.cancel();
            }
        }

        // 3. Inject summary into task-manager conversation
        if let Some(task_manager) = self.agents.get_mut(&task_manager_id) {
            let summary_message = self.format_stop_summary(&state_summary);
            task_manager.inject_system_message(summary_message);
        }

        // 4. Validate and repair conversation context
        if let Some(task_manager) = self.agents.get_mut(&task_manager_id) {
            self.validate_and_repair_context(task_manager);
        }

        // 5. Switch UI to task-manager
        self.select_agent(task_manager_id);
    }
}
```

#### 2. State Gathering

```rust
struct AgentState {
    agent_id: AgentId,
    agent_type: AgentType,
    task_description: Option<String>,
    status: TaskExecutionStatus,
    last_message: Option<String>,
    processing: bool,
}

enum TaskExecutionStatus {
    NotStarted,
    ExecutingJavaScript(String),  // What JavaScript code
    WaitingForInput,
    Completed(String),             // Result
    Failed(String),                // Error
}

impl AgentManagerWindow {
    fn gather_agent_state(&self) -> Vec<AgentState> {
        let mut states = Vec::new();

        // Gather task-manager state
        if let Some(task_manager_id) = self.get_task_manager_id() {
            if let Some(agent) = self.agents.get(&task_manager_id) {
                states.push(AgentState {
                    agent_id: task_manager_id,
                    agent_type: agent.agent_type(),
                    task_description: None,
                    status: self.infer_task_manager_status(agent),
                    last_message: agent.last_message_preview(),
                    processing: agent.is_processing(),
                });
            }
        }

        // Gather task-agent states
        for agent_id in self.get_active_task_agents() {
            if let Some(agent) = self.agents.get(&agent_id) {
                states.push(AgentState {
                    agent_id,
                    agent_type: agent.agent_type(),
                    task_description: agent.get_task_description(),
                    status: self.infer_task_status(agent),
                    last_message: agent.last_message_preview(),
                    processing: agent.is_processing(),
                });
            }
        }

        states
    }

    fn infer_task_status(&self, agent: &AgentInstance) -> TaskExecutionStatus {
        if !agent.is_processing() {
            if let Some(last_msg) = agent.last_assistant_message() {
                if last_msg.contains("error") || last_msg.contains("failed") {
                    return TaskExecutionStatus::Failed(last_msg.clone());
                }
                return TaskExecutionStatus::WaitingForInput;
            }
        }

        // Check if executing JavaScript (look at last tool call)
        if let Some(tool_call) = agent.last_tool_call() {
            if tool_call.tool_name == "execute_javascript" {
                if let Some(code) = tool_call.parameters.get("code") {
                    return TaskExecutionStatus::ExecutingJavaScript(
                        code.as_str().unwrap_or("").to_string()
                    );
                }
            }
        }

        TaskExecutionStatus::NotStarted
    }
}
```

#### 3. Summary Message Format

```rust
impl AgentManagerWindow {
    fn format_stop_summary(&self, states: &[AgentState]) -> String {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        let todo_list = self.get_current_todo_list();

        let mut summary = format!(
            "[SYSTEM INTERRUPTION]\n\
            User pressed Stop at {}\n\n",
            timestamp
        );

        // Active tasks when stopped
        summary.push_str("Active state when stopped:\n");
        for (idx, state) in states.iter().enumerate() {
            match state.agent_type {
                AgentType::TaskManager => {
                    summary.push_str(&format!(
                        "- Task Manager: {}\n",
                        if state.processing { "processing" } else { "idle" }
                    ));
                }
                AgentType::TaskWorker { .. } => {
                    let status_str = match &state.status {
                        TaskExecutionStatus::ExecutingJavaScript(code) => {
                            format!("executing JavaScript ({}...)", truncate(code, 50))
                        }
                        TaskExecutionStatus::WaitingForInput => "waiting for user input".to_string(),
                        TaskExecutionStatus::Failed(err) => format!("failed: {}", truncate(err, 50)),
                        TaskExecutionStatus::Completed(result) => format!("completed: {}", truncate(result, 50)),
                        TaskExecutionStatus::NotStarted => "not started".to_string(),
                    };

                    summary.push_str(&format!(
                        "- Task {}: \"{}\" - {}\n",
                        idx,
                        state.task_description.as_deref().unwrap_or("Unknown"),
                        status_str
                    ));
                }
            }
        }

        // Todo list state
        summary.push_str(&format!(
            "\nTodo list: {} tasks ({} pending, {} in_progress, {} completed)\n",
            todo_list.len(),
            todo_list.iter().filter(|t| matches!(t.status, TodoStatus::Pending)).count(),
            todo_list.iter().filter(|t| matches!(t.status, TodoStatus::InProgress)).count(),
            todo_list.iter().filter(|t| matches!(t.status, TodoStatus::Completed)).count(),
        ));

        summary.push_str("\nWhat would you like to do next?\n");

        summary
    }
}
```

**Example Output:**
```
[SYSTEM INTERRUPTION]
User pressed Stop at 14:32:15

Active state when stopped:
- Task Manager: processing
- Task 0: "List all EC2 instances in production" - executing JavaScript (const accounts = listAccounts(); const prodAcco...)
- Task 1: "List all S3 buckets larger than 100GB" - waiting for user input
- Task 2: "Analyze RDS databases for unused instances" - not started

Todo list: 3 tasks (1 pending, 2 in_progress, 0 completed)

What would you like to do next?
```

#### 4. Context Window Validation and Repair

**Purpose:** Ensure conversation is in valid state for agent to resume

```rust
impl AgentManagerWindow {
    fn validate_and_repair_context(&mut self, agent: &mut AgentInstance) {
        let messages = agent.messages_mut();

        // 1. Validate: Check for proper user/assistant alternation
        let mut has_issues = false;
        for i in 0..messages.len() - 1 {
            let current_role = messages[i].role;
            let next_role = messages[i + 1].role;

            if current_role == next_role {
                log::warn!("Invalid message sequence: two {} messages in a row", current_role);
                has_issues = true;
            }
        }

        // 2. Check last message type
        let last_is_user = messages.back()
            .map(|m| matches!(m.role, ConversationRole::User))
            .unwrap_or(false);

        // 3. Repair: If last message is assistant, inject user message
        if !last_is_user {
            log::info!("Repairing context: injecting user message after stop");
            messages.push_back(ConversationMessage::user(
                "[System: User stopped execution. See summary above.]".to_string()
            ));
        }

        // 4. Check for unclosed tool calls
        if let Some(last_tool_call) = agent.last_unclosed_tool_call() {
            log::warn!("Found unclosed tool call: {}", last_tool_call.tool_name);

            // Inject tool response: cancelled
            agent.inject_tool_response(ToolResponse {
                tool_call_id: last_tool_call.id,
                status: "cancelled",
                reason: "user_interruption",
            });
        }

        // 5. Log repairs
        if has_issues {
            agent.log_system_message("Context validation: repaired message sequence");
        }
    }
}
```

**Validation Checks:**
1. ✅ Messages alternate user/assistant
2. ✅ Last message is from user (ready for agent)
3. ✅ No unclosed tool calls

**Repairs:**
1. If last message is assistant → inject user message
2. If tool call pending → inject cancelled response
3. Log what was fixed for debugging

---

## Implementation Milestones

### Milestone 1: Core Infrastructure

**Goal:** Set up `AgentType` enum and basic structure

**Tasks:**
1. Add `AgentType` enum to `agent_types.rs`
   - Two variants: `TaskManager`, `TaskWorker { parent_id }`
   - Helper methods: `is_task_manager()`, `parent_id()`

2. Update `AgentInstance` struct
   - Add `agent_type: AgentType` field
   - Update constructor to accept agent type
   - Add `new_task_manager()` and `new_task_worker()` factory methods

3. Create prompts module
   - Create `src/app/agent_framework/prompts/mod.rs`
   - Create `task_manager_prompt.rs` with TASK_MANAGER_PROMPT constant
   - Create `task_worker_prompt.rs` with TASK_WORKER_PROMPT constant
   - Export prompts from mod.rs

4. Update `AgentManagerWindow`
   - Modify "New Agent" button to create TaskManager type
   - Add `get_task_manager_id()` helper method
   - Add `get_active_task_agents()` helper method

**Testing:**
- Create task-manager agent via UI
- Verify agent_type is TaskManager
- Verify prompt is task-manager prompt
- No functional changes yet (tools not implemented)

**Completion Criteria:**
- [x] AgentType enum compiles
- [x] AgentInstance has agent_type field
- [x] Prompts module exists with both prompts
- [x] "New Agent" creates TaskManager type
- [x] All existing tests pass

**Status:** ✅ COMPLETE (2025-11-21)

---

### Milestone 2: Task-Manager Tools (Part 1 - Planning)

**Goal:** Implement think, todo-write, todo-read tools

**IMPORTANT NOTE:** All tools for the task-manager/task-worker system are being built **from scratch**.
Do NOT reuse existing tools from the codebase (except execute_javascript which task-workers use).
The new tools are specifically designed for the orchestration agent architecture.

**Tasks:**

1. Implement `think` tool
   - Create `src/app/agent_framework/tools/think.rs`
   - Implement `ThinkTool` struct
   - No-op execution (just logs thought)
   - Register in tools registry

2. Implement `todo-write` tool
   - Create `src/app/agent_framework/tools/todo_write.rs`
   - Implement `TodoWriteTool` struct
   - JSON schema matching Claude Code's format
   - Validation: only one "in_progress" task
   - Store todo list in agent context (thread-local or agent state)

3. Implement `todo-read` tool
   - Create `src/app/agent_framework/tools/todo_read.rs`
   - Implement `TodoReadTool` struct
   - Retrieve todo list from agent context
   - Return formatted JSON with summary statistics

4. Create todo list storage mechanism
   - Add `todo_list: Vec<TodoItem>` to AgentInstance
   - Or use thread-local storage for current agent's todos
   - Implement `set_current_todo_list()` and `get_current_todo_list()` helpers

5. Update tool registry
   - Add factory functions for new tools
   - Create `task_manager_tools()` function returning all 3 tools
   - Update `AgentInstance::create_stood_agent()` to use correct tools based on agent_type

**Testing:**
- Unit tests for each tool
- Test think tool logs correctly
- Test todo-write validation (reject multiple in_progress)
- Test todo-read returns correct summary
- Integration test: create task-manager, use tools in sequence

**Completion Criteria:**
- [x] think, todo-write, todo-read tools compile and pass tests
- [x] Task-manager agent has access to these tools
- [x] Task-worker agent does NOT have these tools
- [x] Todo list persists across tool calls in same agent

**Status:** ✅ COMPLETE (2025-11-21)

**Before Milestone 3:**
- [x] Create `TODOS/MILESTONE_2_IMPLEMENTATION.md` following the same detailed approach as MILESTONE_1_IMPLEMENTATION.md
  - Read current code and identify integration points
  - Write tests first (TDD approach)
  - Provide exact code changes with file paths and line numbers
  - Include compilation checkpoints and verification steps

---

### Milestone 3: start-task Tool and Agent Spawning

**Goal:** Implement start-task tool (placeholder approach for Milestone 3)

**Tasks:**

1. ✅ Implement `start-task` tool (placeholder)
   - Created `src/app/agent_framework/tools/start_task.rs`
   - Implemented `StartTaskTool` struct with proper schema
   - Accepts `task_description` (required) and `expected_output_format` (optional) parameters
   - Validates parameters and logs requests (actual spawning deferred)

2. ✅ Add to tool registry
   - Updated `tools/mod.rs` to export StartTaskTool
   - Updated `agent_instance.rs` get_tools_for_type() to include start-task for TaskManager
   - TaskManager agents now have 4 tools: think, todo-write, todo-read, start-task

3. ✅ Update prompts
   - Updated TASK_MANAGER_PROMPT to mention start-task availability
   - Clearly documented that spawning is placeholder in Milestone 3

4. ✅ Testing
   - 6 new tests for start-task tool (all passing)
   - Updated test_task_manager_has_planning_tools to check for 4 tools

**Deferred to Future Milestones:**
- Agent creation mechanism (requires architectural changes)
- Result passing mechanism
- AgentInstance lifecycle updates
- Parallel execution configuration

**Architectural Challenge:** start-task tool needs access to AgentManagerWindow to spawn agents, but tools are created in AgentInstance which doesn't have window access. Future solutions include: passing window reference, using global agent registry, or moving agent creation to accessible service.

**Status:** ✅ COMPLETE (2025-11-21) - Placeholder implementation

**Test Results:**
- start_task tool: 6 tests passing
- agent_instance: 1 updated test passing
- Total: 7 new/updated tests passing

**Before Milestone 4:**
- [x] Create `TODOS/MILESTONE_3_IMPLEMENTATION.md` following the same detailed approach as MILESTONE_1_IMPLEMENTATION.md

---

### Milestone 4: UI State Machine and Event Channel

**Goal:** Implement UI event channel system for agent communication

**Tasks:**

1. ✅ Create UI event channel infrastructure
   - Created `src/app/agent_framework/ui_events.rs` with AgentUIEvent enum
   - Used `std::sync::mpsc` (standard library, no external deps)
   - Implemented `send_ui_event()` global function for tools
   - 4 tests passing

2. ✅ Integrate event channel into AgentManagerWindow
   - Added `ui_event_receiver` field to AgentManagerWindow
   - Implemented `process_ui_events()` method
   - Called in `show_with_focus()` update loop
   - Fixed borrow checker issues with event collection pattern

3. ✅ Add Tab key cycling
   - Implemented `get_active_task_agents()` method
   - Implemented `cycle_to_next_task_agent()` with wrap-around
   - Implemented `handle_keyboard_navigation()` method
   - Called in update loop

4. ✅ Implement task indicator UI
   - Created `TaskContext` struct
   - Implemented `get_task_context()` method
   - Implemented `render_task_indicator()` with blue highlight
   - Shows "Task X of Y - Press Tab to cycle" for task-workers

**Deferred to Milestone 5:**
- Auto-switch on task start (requires actual agent spawning)
- Auto-switch back on completion (requires completion detection)
- Waiting-for-input indicator

**Status:** ✅ COMPLETE (2025-11-21)

**What Works:**
- UI event channel system operational
- AgentManagerWindow processes events
- Tab key cycles through task-agents
- Task indicator renders correctly
- Tools can trigger UI changes without window access

**Test Results:**
```
ui_events tests: 4/4 passing
- test_ui_event_creation
- test_channel_initialization
- test_send_ui_event_helper
- test_multiple_events_in_order

Compilation: ✅ Success
```

**Architectural Achievement:**
The Observer Pattern with global message bus successfully decouples tools from UI. Tools can now send events via `send_ui_event()` without needing window references, solving the architectural challenge from Milestone 3.

**Before Milestone 5:**
- [x] Create `TODOS/MILESTONE_4_IMPLEMENTATION.md` following the same detailed approach as MILESTONE_1_IMPLEMENTATION.md

---

### Milestone 5: Error Handling and Recovery

**Goal:** Implement intelligent error reporting and recovery

**Tasks:**

1. Update task-worker error handling
   - Modify `execute_javascript` error handling to capture detailed context
   - Create `ErrorReport` struct with comprehensive fields
   - Include: error type, message, context, partial results, suggestions

2. Implement error reporting to parent
   - When task-worker fails: send error report to parent
   - Format as JSON with structured error details
   - Include stack trace if available

3. Update task-manager prompt with error handling guidance
   - Add section on error analysis using `think` tool
   - Add examples of different recovery strategies
   - Emphasize LLM decision-making (no auto-retry)

4. Test error scenarios
   - Test timeout error (query too large)
   - Test permission error (access denied)
   - Test syntax error (invalid JavaScript)
   - Test resource not found error
   - Verify task-manager receives detailed error reports

5. Test recovery strategies
   - Test retry with refinement (narrow scope)
   - Test different approach (alternative API)
   - Test asking user for clarification
   - Test skipping failed task and continuing

**Testing:**
- Unit test: error report format
- Integration test: task fails with timeout, task-manager analyzes and retries
- Integration test: task fails with permission error, task-manager asks user
- Integration test: task fails with syntax error, task-manager refines and retries

**Completion Criteria:**
- [ ] Task-worker errors generate detailed error reports
- [ ] Error reports sent to task-manager with full context
- [ ] Task-manager can analyze errors using think tool
- [ ] Task-manager can retry with refinements
- [ ] Task-manager can ask user for help on errors

**Before Milestone 6:**
- [ ] Create `TODOS/MILESTONE_5_IMPLEMENTATION.md` following the same detailed approach as MILESTONE_1_IMPLEMENTATION.md

---

### Milestone 6: Stop/Resume with Context Injection

**Goal:** Implement stop/ESC behavior with intelligent context preservation

**Tasks:**

1. Implement state gathering
   - Create `gather_agent_state()` method in AgentManagerWindow
   - Collect state from task-manager and all active task-workers
   - Capture: task descriptions, execution status, last messages, partial results

2. Implement stop handler
   - Create `stop_all_agents_with_context()` method
   - Cancel all active agents (task-manager + workers)
   - Gather state before cancellation

3. Implement context injection
   - Create `format_stop_summary()` method
   - Generate comprehensive summary message
   - Inject into task-manager conversation as system/user message
   - Include: timestamp, active tasks, todo list state, "what next?" prompt

4. Implement context validation and repair
   - Create `validate_and_repair_context()` method
   - Check message alternation (user/assistant)
   - Check for unclosed tool calls
   - Repair: inject user message if needed, cancel pending tool calls
   - Log repairs for debugging

5. Connect ESC key to stop handler
   - In `AgentManagerWindow::handle_input()`: detect ESC key
   - Call `stop_all_agents_with_context()` on ESC press

6. Test stop/resume flow
   - Test stop during task execution (JavaScript running)
   - Test stop during wait for input
   - Test resume with "continue" command
   - Test resume with new instructions
   - Verify context is clean (valid message pairs)

**Testing:**
- Manual test: start 3 tasks, press ESC mid-execution
- Verify summary message injected with correct state
- Manual test: after stop, say "continue" and verify agent resumes correctly
- Manual test: after stop, give new instructions and verify agent complies
- Unit test: context validation finds and repairs invalid sequences

**Completion Criteria:**
- [ ] ESC key stops all agents (task-manager + workers)
- [ ] Comprehensive summary injected into task-manager conversation
- [ ] Summary includes task states, todo list, timestamp
- [ ] Context validated and repaired (message pairs, tool calls)
- [ ] Agent can resume after stop with full context

**Before Milestone 7:**
- [ ] Create `TODOS/MILESTONE_6_IMPLEMENTATION.md` following the same detailed approach as MILESTONE_1_IMPLEMENTATION.md

---

### Milestone 7: Result Format Enforcement

**Goal:** Ensure task-workers include complete data in final responses

**Tasks:**

1. Update task-worker prompt
   - Add "Critical Rules" section emphasizing data inclusion
   - Add examples of good vs bad final responses
   - Add guidance on respecting expected_output_format

2. Implement expected output format injection
   - In `start-task` tool: append `<expected_output_format>` to task prompt
   - Task-worker receives format instructions in context

3. Test result formatting
   - Test task with expected format "JSON array" → verify JSON in response
   - Test task with expected format "table" → verify table in response
   - Test task with no format specified → verify complete data still included

4. Add result validation (optional)
   - Create optional validator that checks if result matches expected format
   - Log warning if result doesn't match expected format
   - Don't block result passing (just log for evaluation)

**Testing:**
- Integration test: specify JSON format, verify JSON in result
- Integration test: specify table format, verify table in result
- Integration test: no format specified, verify data still included (not just summary)

**Completion Criteria:**
- [ ] Task-worker prompt emphasizes including complete data
- [ ] Expected format instructions injected into task-worker context
- [ ] Task-workers return complete data (not just summaries)
- [ ] Results match expected format when specified

**Before Milestone 8:**
- [ ] Create `TODOS/MILESTONE_7_IMPLEMENTATION.md` following the same detailed approach as MILESTONE_1_IMPLEMENTATION.md

---

### Milestone 8: Integration and Polish

**Goal:** End-to-end integration and user experience polish

**Tasks:**

1. Test complete user workflows
   - Simple request: "List my EC2 instances"
   - Complex request: "Analyze my entire AWS infrastructure"
   - Error scenario: "List instances in non-existent account"
   - Interruption: Stop mid-execution and resume

2. Performance optimization
   - Measure task-agent spawn time
   - Optimize agent creation (lazy initialization)
   - Test with 10+ parallel tasks
   - Profile memory usage

3. Logging improvements
   - Ensure all tool calls logged to agent logger
   - Log task-agent lifecycle (spawn, execute, complete, terminate)
   - Log UI events (switches, Tab presses)
   - Add debug logs for troubleshooting

4. Documentation updates
   - Update `CLAUDE.md` with task-manager-agent usage
   - Add examples to technical docs
   - Document keyboard shortcuts (Tab, ESC)
   - Add troubleshooting guide

5. UI polish
   - Improve task indicator styling
   - Add visual distinction between task-manager and task-worker conversations
   - Add icons/colors for task states (pending, in-progress, completed, failed)
   - Improve "waiting for input" indicator visibility

**Testing:**
- Full end-to-end workflows
- Performance benchmarks
- Memory leak testing
- Concurrency stress testing (spawn 20 tasks)

**Completion Criteria:**
- [ ] All user workflows work end-to-end
- [ ] Performance meets expectations (task spawn < 100ms)
- [ ] No memory leaks
- [ ] Documentation complete
- [ ] UI polished and user-friendly

---

## File Organization

Following **Option A: Extend Existing Structure** (stdlib pattern)

```
src/app/agent_framework/
├── agent_instance.rs          # UPDATE: Add agent_type field, lifecycle methods
├── agent_types.rs             # UPDATE: Add AgentType enum
├── agent_logger.rs            # No changes
├── conversation.rs            # No changes
├── model_config.rs            # No changes
├── tools/                     # EXTEND: Add new tools
│   ├── mod.rs                 # UPDATE: Export new tools, add task_manager_tools()
│   ├── execute_javascript.rs  # No changes (existing, for task-worker)
│   ├── think.rs               # NEW: Think tool implementation
│   ├── todo_write.rs          # NEW: TodoWrite tool implementation
│   ├── todo_read.rs           # NEW: TodoRead tool implementation
│   └── start_task.rs          # NEW: StartTask tool implementation
├── prompts/                   # NEW: Prompt management module
│   ├── mod.rs                 # NEW: Export prompts
│   ├── task_manager_prompt.rs # NEW: Task-manager system prompt
│   └── task_worker_prompt.rs  # NEW: Task-worker system prompt
└── tools_registry.rs          # UPDATE: Add new tools to registry

src/app/dashui/
└── agent_manager_window.rs    # UPDATE: UI state machine, Tab cycling, stop handler
```

### Module Dependencies

```rust
// tools/mod.rs
pub mod execute_javascript;
pub mod think;
pub mod todo_write;
pub mod todo_read;
pub mod start_task;

pub use execute_javascript::execute_javascript_tool;
pub use think::think_tool;
pub use todo_write::todo_write_tool;
pub use todo_read::todo_read_tool;
pub use start_task::start_task_tool;

/// Get tools for task-manager agent
pub fn task_manager_tools(agent_manager: Arc<Mutex<AgentManagerWindow>>) -> Vec<Box<dyn Tool>> {
    vec![
        think_tool(),
        todo_write_tool(),
        todo_read_tool(),
        start_task_tool(agent_manager),
    ]
}

/// Get tools for task-worker agent
pub fn task_worker_tools() -> Vec<Box<dyn Tool>> {
    vec![execute_javascript_tool()]
}
```

```rust
// prompts/mod.rs
pub mod task_manager_prompt;
pub mod task_worker_prompt;

pub use task_manager_prompt::TASK_MANAGER_PROMPT;
pub use task_worker_prompt::TASK_WORKER_PROMPT;

/// Load prompt from file (for evaluations)
pub fn load_prompt_from_file(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to load prompt from {}: {}", path, e))
}
```

---

## Testing Strategy

### Unit Tests

**Location:** `tests/agent_framework/`

1. **AgentType Tests** (`test_agent_types.rs`)
   - Test `is_task_manager()`
   - Test `parent_id()` returns correct value
   - Test enum matching

2. **Tool Tests** (`test_tools.rs`)
   - Test `think_tool()` logs correctly
   - Test `todo_write_tool()` validation (reject multiple in_progress)
   - Test `todo_read_tool()` returns correct summary
   - Test `start_task_tool()` creates agent (mocked)

3. **Context Validation Tests** (`test_context_validation.rs`)
   - Test detecting invalid message sequences
   - Test repair: inject user message after assistant
   - Test repair: cancel unclosed tool calls

### Integration Tests

**Location:** `tests/integration/`

1. **Task Orchestration Test** (`test_task_orchestration.rs`)
   ```rust
   #[test]
   fn test_parallel_task_execution() {
       // Create task-manager
       // Spawn 3 tasks in parallel
       // Verify all complete
       // Verify results passed to parent
       // Verify workers terminated
   }
   ```

2. **Error Handling Test** (`test_error_handling.rs`)
   ```rust
   #[test]
   fn test_task_failure_and_retry() {
       // Create task-manager
       // Spawn task that will fail (timeout)
       // Verify error report sent to parent
       // Task-manager analyzes error
       // Task-manager retries with refinement
       // Verify success on retry
   }
   ```

3. **Stop/Resume Test** (`test_stop_resume.rs`)
   ```rust
   #[test]
   fn test_stop_and_resume() {
       // Create task-manager
       // Spawn 3 tasks
       // Simulate ESC key press
       // Verify summary injected
       // Verify context repaired
       // Resume with "continue"
       // Verify agent continues correctly
   }
   ```

### Manual UI Tests

**Location:** Manual test checklist

1. **Basic Workflow**
   - [ ] Create task-manager agent
   - [ ] Ask "List my EC2 instances"
   - [ ] Verify task spawned
   - [ ] Verify UI switches to task-agent
   - [ ] Verify result returned to task-manager
   - [ ] Verify final response includes data

2. **Parallel Tasks**
   - [ ] Ask "Analyze EC2, S3, and RDS"
   - [ ] Verify 3 tasks spawned
   - [ ] Verify UI shows "Task 1 of 3"
   - [ ] Press Tab, verify cycling through tasks
   - [ ] Verify all tasks complete
   - [ ] Verify UI switches back to task-manager

3. **Error Handling**
   - [ ] Spawn task with invalid account
   - [ ] Verify error reported to task-manager
   - [ ] Verify task-manager uses think tool
   - [ ] Verify task-manager asks user or retries

4. **Stop/Resume**
   - [ ] Spawn 3 tasks
   - [ ] Press ESC mid-execution
   - [ ] Verify summary message injected
   - [ ] Say "continue"
   - [ ] Verify agent resumes

5. **Waiting for Input**
   - [ ] Spawn task that needs user input
   - [ ] Switch to different task with Tab
   - [ ] Verify "⚠️ Task X waiting for input" indicator
   - [ ] Tab back to waiting task
   - [ ] Provide input
   - [ ] Verify task continues

---

## Integration Points

### 1. AgentInstance Modifications

**File:** `src/app/agent_framework/agent_instance.rs`

**Changes:**
- Add `agent_type: AgentType` field
- Add `parent_id()` method
- Update `create_stood_agent()` to use correct tools based on agent_type
- Update `poll_response()` to detect worker completion and send result to parent
- Add `send_result_to_parent()` method
- Add `terminate()` method for cleanup
- Add `inject_system_message()` for stop/resume summaries
- Add `is_waiting_for_input()` detection

### 2. AgentManagerWindow Modifications

**File:** `src/app/dashui/agent_manager_window.rs`

**Changes:**
- Add UI event channel (sender/receiver)
- Add `process_ui_events()` called every frame
- Add `cycle_to_next_task_agent()` for Tab key
- Add `stop_all_agents_with_context()` for ESC key
- Add `gather_agent_state()` for stop summaries
- Add `format_stop_summary()` for context injection
- Add `validate_and_repair_context()` for context cleanup
- Add `get_task_manager_id()` helper
- Add `get_active_task_agents()` helper
- Update `handle_input()` to detect Tab and ESC keys
- Add task indicator UI to conversation header

### 3. Tools Registry Modifications

**File:** `src/app/agent_framework/tools_registry.rs`

**Changes:**
- Add `task_manager_tools()` function
- Add `task_worker_tools()` function
- Export new tools (think, todo_write, todo_read, start_task)

### 4. Stood Configuration

**Location:** Agent initialization

**Changes:**
- Set max concurrent agents to 5
- Configure parallel execution mode
- Set up agent communication channels

---

## Non-Goals

**What we are NOT implementing in this spec:**

1. **Persistence:** Task state does not persist across app restarts (in-memory only)
2. **Historical task tracking:** No database of past tasks/results
3. **User authentication:** AWS credentials managed same as current system
4. **Custom agent types:** Only TaskManager and TaskWorker in this version
5. **Web UI:** Desktop egui only
6. **Real-time collaboration:** Single user only
7. **Task scheduling:** No cron-like scheduled tasks
8. **Cost tracking:** No AWS cost analysis for executed queries
9. **Rate limiting:** Beyond stood's parallel execution limit

---

## Success Criteria

**This implementation is successful when:**

1. ✅ User can create task-manager agent via "New Agent" button
2. ✅ Task-manager decomposes complex requests into parallel tasks
3. ✅ Task-agents execute independently and return results
4. ✅ UI auto-switches to show task-agent work in real-time
5. ✅ User can cycle through active tasks with Tab key
6. ✅ UI indicates when tasks are waiting for input
7. ✅ Task results include complete data (not just summaries)
8. ✅ Errors are intelligently reported and handled
9. ✅ ESC key stops all agents with context preservation
10. ✅ User can resume after stop with full context
11. ✅ Up to 5 tasks run in parallel (stood manages queuing)
12. ✅ All existing tests pass
13. ✅ New tests cover core functionality
14. ✅ Documentation is complete and accurate

---

## Open Questions

**To be resolved during implementation:**

1. **Todo List Storage:** Thread-local vs agent instance field?
   - Thread-local: Simpler, matches current pattern
   - Instance field: More explicit, easier to serialize

2. **Result Passing Mechanism:** Channel vs callback vs direct method call?
   - Channel: Async, decoupled
   - Direct call: Simpler, synchronous

3. **UI Event Channel:** Global singleton vs AgentManagerWindow field?
   - Global: Tools can access easily
   - Window field: Cleaner architecture

4. **Task-Agent Naming:** Auto-generate vs use task description?
   - Auto: "Task: List EC2..." (current spec)
   - Description: Just use first 50 chars of description

5. **Prompt Swapping:** File-based vs in-memory constants?
   - File: Easy to swap for evaluations
   - Constants: Simpler, faster, typechecked

**Decision needed before implementation starts.**

---

## References

### External Documentation
- **Claude Code TodoWrite/TodoRead:** https://gist.github.com/wong2/e0f34aac66caf890a332f7b6f9e2ba8f
- **Anthropic Think Tool:** https://www.anthropic.com/engineering/claude-think-tool
- **Anthropic Agent Best Practices:** https://www.anthropic.com/research/building-effective-agents
- **Google ADK Multi-Agent:** https://google.github.io/adk-docs/agents/multi-agents/

### Internal Documentation
- **Current Agent Implementation:** `src/app/agent_framework/agent_instance.rs`
- **Current UI:** `src/app/dashui/agent_manager_window.rs`
- **Execute JavaScript Tool:** `src/app/agent_framework/tools/execute_javascript.rs`
- **Technical Docs:** `docs/technical/README.md`

---

**END OF SPECIFICATION**
