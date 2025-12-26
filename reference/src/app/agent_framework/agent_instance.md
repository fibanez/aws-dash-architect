# Agent Instance - Individual Agent Execution and State Management

## Component Overview

Manages a single agent's complete lifecycle including execution, message handling,
and UI communication. Handles async agent creation, message processing via background
threads, and response channel management. This is where the real agent execution happens.

**Pattern**: Active object pattern with async execution and message passing
**External**: tokio runtime, stood Agent library, mpsc channels
**State**: AgentMetadata, AgentStatus, AgentType, Messages, Agent, mpsc channel, AgentLogger

---

## Major Fields

- `id: AgentId` - Unique identifier (UUID wrapper)
- `metadata: AgentMetadata` - Name, description, model_id, timestamps
- `status: AgentStatus` - Running/Paused/Completed/Failed/Cancelled
- `agent_type: AgentType` - TaskManager or TaskWorker (determines tools/prompts)
- `agent: Arc<Mutex<Option<Agent>>>` - Stood Agent instance (lazy initialized)
- `response_channel: (Sender, Receiver)` - mpsc channel for background thread communication
- `messages: VecDeque<Message>` - Conversation history (max 100 messages)
- `processing: bool` - True when agent is actively executing
- `todo_list_shared: Arc<Mutex<Vec<TodoItem>>>` - Shared todo list for task-managers
- `logger: Arc<AgentLogger>` - Per-agent log file
- `runtime: Arc<Runtime>` - Dedicated tokio runtime per agent

---

## Major Methods

**Lifecycle**:
- `new(metadata, agent_type)` - Create instance with type-specific tools/prompts
- `new_with_parent_logger(metadata, agent_type, parent_logger)` - Worker shares parent log
- `send_message()` - Spawn background thread to execute agent with input
- `check_responses()` - Non-blocking receive from response channel
- `initialize()` - Create and configure stood Agent with credentials

**State Management**:
- `status()` - Get current status
- `agent_type()` - Get agent type (TaskManager/TaskWorker)
- `is_processing()` - Check if actively executing
- `todo_list()` - Get current todo list (for display)
- `set_todo_list()` - Update todo list
- `clear_todo_list()` - Clear all todos

**Type-Specific Configuration**:
- `get_tools_for_type()` - Return tools based on AgentType
- `get_system_prompt_for_type()` - Return prompt based on AgentType

**Logger Access**:
- `logger()` - Get AgentLogger reference for UI display

---

## Implementation Patterns

### Pattern: Lazy Agent Creation with Background Execution

**Algorithm**: Spawn std::thread with tokio runtime, lazy Agent initialization
**External**: std::thread::spawn, tokio::runtime::Runtime, stood::Agent

Pseudocode:
  1. User calls send_message(input)
  2. Add user message to conversation history
  3. Set processing = true
  4. Clone agent Arc, sender, logger, runtime
  5. Spawn std::thread:
     a. Set thread-local agent context (set_current_agent_id, set_current_agent_type)
     b. runtime.block_on(async):
        - Lock agent Arc
        - If None: create agent with type-specific tools/prompt
        - Call agent.execute(&input)
        - Log model request/response with logger
        - Return AgentResult
     c. Convert result to AgentResponse (Success/Error)
     d. Send via mpsc channel
  6. UI polls check_responses() to receive results

### Pattern: Type-Based Tool Configuration

**Algorithm**: Match on AgentType, return appropriate tool set
**External**: ThinkTool, StartTaskTool, ExecuteJavaScriptTool

Pseudocode:
  1. get_tools_for_type() matches self.agent_type:
  2. TaskManager returns:
     - ThinkTool (no-op reasoning space)
     - StartTaskTool (spawn workers)
  3. TaskWorker returns:
     - ExecuteJavaScriptTool (V8 sandbox with AWS APIs)

### Pattern: Type-Based System Prompt

**Algorithm**: Match on AgentType, substitute datetime placeholder
**External**: TASK_MANAGER_PROMPT, TASK_WORKER_PROMPT

Pseudocode:
  1. get_system_prompt_for_type() matches self.agent_type:
  2. TaskManager -> TASK_MANAGER_PROMPT
  3. TaskWorker -> TASK_WORKER_PROMPT
  4. Replace {{CURRENT_DATETIME}} with Utc::now()
  5. Return configured prompt string

### Pattern: Response Processing Loop

**Algorithm**: Non-blocking channel receive with message aggregation
**External**: mpsc::Receiver::try_recv()

Pseudocode:
  1. UI calls check_responses() each frame
  2. try_recv() returns Vec<AgentResponse>
  3. For each response, call handle_response(response):
     - Success: add assistant message, set processing=false
     - Error: add system error message, set status=Failed, processing=false
     - ToolCallStart: add parent message with tool input
     - ToolCallComplete: add child message with tool output
     - JsonDebug: debug logging only
     - ModelChanged: update metadata.model_id
  4. Empty responses show tool summary if tools_called not empty

### Pattern: Per-Agent Logging

**Algorithm**: Structured logging to dedicated file per agent
**External**: AgentLogger, filesystem I/O

Pseudocode:
  1. Create AgentLogger in new() with agent_id and name
  2. Logger writes to: ~/.local/share/awsdash/logs/agents/agent-{uuid}.log
  3. Log events:
     - Agent creation (metadata)
     - User messages
     - Assistant responses
     - System messages
     - Model requests (prompt, input, model_id)
     - Model responses (output, status, duration, tokens)
     - Tool executions (from callback handlers)
     - Agent termination (final status)
  4. UI can display log file path via logger.log_path()

### Pattern: Message History with Limits

**Algorithm**: VecDeque with max size enforcement
**External**: VecDeque::push_back, pop_front

Pseudocode:
  1. add_message(message):
     - Log based on role (User/Assistant/System)
     - Push to messages VecDeque
     - If len > MAX_MESSAGES (100): pop_front()
     - Update metadata.updated_at
  2. Prevents memory growth for long-running agents
  3. Full history preserved in log file

---

## External Dependencies

- **tokio**: Async runtime creation via Runtime::new()
- **stood**: Agent creation and execution
- **std::thread**: Background thread spawning
- **std::sync::mpsc**: Channel for thread communication
- **std::sync::Arc/Mutex**: Shared state (agent, logger)
- **chrono::Utc**: Timestamps
- **uuid**: AgentId generation
- **AgentLogger**: Per-agent file logging
- **AwsIdentityCenter**: Credential retrieval
- **OrchestrationAgent**: Agent factory

---

## Key Algorithms

### Agent Lifecycle State Machine
States: Running → Paused | Completed | Failed(String) | Cancelled
Transitions triggered by AgentResponse events
Terminal states logged via AgentLogger

### Credential Error Handling
User-friendly error messages for common failures:
- ExpiredTokenException → "AWS credentials have expired"
- UnknownServiceError → "AWS service error"
- timeout/Timeout → "Request timed out"
- Generic → "Error processing message: {}"

### Empty Response Handling
If agent.execute() returns empty response:
- Check if tools_called is non-empty
- Generate summary: "I've executed the following tools: {}"
- Fallback: Show error details with response length, tools used, success flag

---

**Last Updated**: 2025-11-25
**Status**: Updated for multi-agent task orchestration system (AgentType, type-based tools/prompts)
