# Tools Registry - Global Tool and State Management

## Component Overview

Central registry for agent framework tools, manages global shared state (AWS
clients, TODO storage, cancellation manager), and provides tool constructor
functions for agent builders.

**Pattern**: Global state management with static RwLock
**External**: RwLock, Arc/Mutex for thread-safe sharing
**Purpose**: Centralize configuration and reduce agent coupling

---

## Major Global State

- `GLOBAL_AWS_CLIENT` - Shared AWSResourceClient for all tools
- `GLOBAL_AGENT_SENDER` - mpsc channel for log event bubbling
- `GLOBAL_TODO_STORAGE` - Cross-agent TODO synchronization
- `GLOBAL_CANCELLATION_MANAGER` - Centralized agent cancellation
- `GLOBAL_MODEL_CONFIG` - Shared Bedrock model selection
- `GLOBAL_AWS_CREDENTIALS` - Standalone agent credentials

---

## Major Functions

### Global State Setters/Getters/Clearers
- `set_global_aws_client()` / `get_global_aws_client()`
- `set_global_aws_credentials()` / `get_global_aws_credentials()` / `clear_global_aws_credentials()`
- `set_global_agent_sender()` / `get_global_agent_sender()` / `clear_global_agent_sender()`
- `initialize_global_todo_storage()` / `get_global_todo_storage()` (auto-initializes if needed)
- `set_global_cancellation_manager()` / `get_global_cancellation_manager()` / `clear_global_cancellation_manager()`
- `set_global_model()` / `get_global_model()` / `clear_global_model()`
- `set_global_current_project()` / `get_global_current_project()` (stubbed - project management removed)

### Tool Constructors
- `aws_list_resources_tool()` - AWS resource listing
- `aws_describe_resource_tool()` - Resource detail retrieval
- `aws_find_account_tool()` - Account discovery
- `aws_find_region_tool()` - Region discovery
- `aws_describe_log_groups_tool()` - CloudWatch log group listing
- `aws_get_log_events_tool()` - CloudWatch log event retrieval
- `aws_cloudtrail_lookup_events_tool()` - CloudTrail event search
- `todo_write_tool()` - Shared TODO creation
- `todo_read_tool()` - Shared TODO querying
- `create_task_tool()` - Sub-agent creation

---

## Implementation Patterns

### Pattern: Static RwLock with Explicit Initialization

**Algorithm**: Thread-safe global state with manual initialization
**External**: std::sync::RwLock

Pseudocode:
  1. Define static GLOBAL_* with RwLock::new(None)
  2. Wrap value in RwLock for concurrent read/write
  3. Initially None, requires explicit setter call
  4. Setter function:
     - Acquire write lock via .write()
     - Handle lock poisoning with error logging
     - Set value to Some(T)
     - Log success with emoji prefix
  5. Getter function:
     - Acquire read lock via .read()
     - Handle lock poisoning with error logging
     - Clone Arc<T> for caller
     - Log access event (available/not set)
  6. Clearer function:
     - Acquire write lock
     - Set to None
     - Log clearing event
  7. Read locks: multiple concurrent readers OK
  8. Write locks: exclusive access, blocks readers
  9. TODO storage: auto-initializes on get if not set

### Pattern: Tool Constructor with Global Fallback

**Algorithm**: Local parameter OR global state with error handling
**External**: Option combinators, ToolError types

Pseudocode:
  1. Tool constructor accepts Option<Arc<AWSResourceClient>>
  2. Tool creation stores local client if provided
  3. On tool execution:
     - Try local client first
     - If None, call get_global_aws_client()
     - If both None, return user-friendly ToolError
     - Error message explains how to fix (open Explorer, login)
  4. Enables both standalone and integrated modes
  5. Reduces coupling: agents don't need AWS client directly

### Pattern: Shared TODO Storage

**Algorithm**: Arc<Mutex<HashMap>> for cross-agent synchronization
**External**: Arc for reference counting, Mutex for exclusive updates

Pseudocode:
  1. initialize_global_todo_storage() creates HashMap
  2. Key: agent_id String, Value: Vec<TodoItem>
  3. TodoWrite tool:
     - Lock mutex, get or create Vec for agent_id
     - Append/update TODO items
     - Unlock mutex
  4. TodoRead tool:
     - Lock mutex, query Vec for agent_id
     - Return filtered/sorted results
     - Unlock mutex
  5. Enables TODO sharing between orchestration + task agents
  6. Atomic updates prevent race conditions

---

## External Dependencies

- **std::sync**: RwLock, Mutex, Arc for thread safety
- **std::sync::mpsc**: Channel for agent messaging
- **tracing**: Logging for state management events with emoji prefixes
- **serde_json**: Tool parameter serialization
- **stood::tools::Tool**: Tool trait for stood agent library

---

## Key Algorithms

### Global State Lifecycle
- Initialization: Lazy on first access
- Usage: Read-heavy, write-rare pattern
- Cleanup: On app shutdown (drop all globals)

### Tool Registration Pattern
- Each tool constructor: fn() -> Box<dyn Tool>
- Agent builder calls constructors during setup
- Tools receive global state via getters during execution

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects tools_registry.rs with GLOBAL_* naming
