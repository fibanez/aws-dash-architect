# Agent Instance - Individual Agent Execution and State Management

## Component Overview

Manages a single agent's complete lifecycle including execution, message handling,
and UI communication. Handles async agent creation, message processing via background
threads, and response channel management. This is where the real agent execution happens.

**Pattern**: Active object pattern with async execution and message passing
**External**: tokio runtime, stood Agent library, mpsc channels
**State**: AgentMetadata, AgentStatus, Messages, Agent, mpsc channel, AgentLogger

---

## Major Fields

- `id: AgentId` - Unique identifier (UUID wrapper)
- `metadata: AgentMetadata` - Name, description, model_id, timestamps
- `status: AgentStatus` - Running/Paused/Completed/Failed/Cancelled
- `agent: Arc<Mutex<Option<Agent>>>` - Stood Agent instance (lazy initialized)
- `response_channel: (Sender, Receiver)` - mpsc channel for background thread communication
- `messages: VecDeque<Message>` - Conversation history (max 100 messages)
- `processing: bool` - True when agent is actively executing
- `viewing_windows: HashSet<String>` - UI windows displaying this agent
- `logger: Arc<AgentLogger>` - Per-agent log file

---

## Major Methods

**Lifecycle**:
- `new()` - Create instance with metadata, mpsc channel, logger
- `send_message()` - Spawn background thread to execute agent with input
- `check_responses()` - Non-blocking receive from response channel
- `handle_response()` - Process AgentResponse variants (Success/Error/ToolCall/etc)

**State Management**:
- `status()` - Get current status
- `set_status()` - Update status, log terminal states
- `is_processing()` - Check if actively executing
- `add_message()` - Append to history, log by role, limit to 100 messages
- `get_messages()` - Access message history for UI rendering

**Window Management**:
- `register_window()` - Track UI window viewing this agent
- `unregister_window()` - Remove window from viewing set
- `has_viewing_windows()` - Check if any windows are open
- `close_all_windows()` - Clear all windows, return IDs for cleanup

**Logger Access**:
- `logger()` - Get AgentLogger reference for UI display

---

## Implementation Patterns

### Pattern: Lazy Agent Creation with Background Execution

**Algorithm**: Spawn std::thread with tokio runtime, lazy Agent initialization
**External**: std::thread::spawn, tokio::runtime::Runtime, stood::Agent

Pseudocode:
  1. User calls send_message(input, aws_identity)
  2. Add user message to conversation history
  3. Set processing = true
  4. Clone agent Arc, sender, aws_identity, model_id, logger
  5. Spawn std::thread:
     a. Get AWS credentials from AwsIdentityCenter (outside tokio)
     b. Create tokio::runtime::Runtime
     c. runtime.block_on(async):
        - Lock agent Arc
        - If None: create OrchestrationAgent, store in Arc
        - Call agent.execute(&input)
        - Log model request/response with logger
        - Return AgentResult
     d. Convert result to AgentResponse (Success/Error)
     e. Send via mpsc channel
  6. UI polls check_responses() to receive results

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

**Last Updated**: 2025-01-28
**Status**: Accurately reflects agent_instance.rs implementation
