# Agent Framework Module - AI Agent System for AWS Operations

## Component Overview

Module exports for the Agent Framework system. This file provides public API
surface by re-exporting types from submodules. Actual implementation lives in
individual component files.

**Pattern**: Module organization with public re-exports
**External Systems**: AWS Bedrock (Claude models), AWS services via tools
**Purpose**: AI-driven AWS infrastructure management through natural language

---

## Module Structure

### Core System Files
- `mod.rs` - This file: module exports and public API
- `agent_manager.rs` - AgentManager, agent registry
- `agent_instance.rs` - AgentInstance, AgentId, AgentStatus, AgentMetadata
- `message.rs` - AgentResponse, Message, MessageRole, JsonDebugData

### Agent Types
- `agents/mod.rs` - Agent submodule exports
- `agents/orchestration_agent.rs` - OrchestrationAgent, AwsCredentials
- `agents/task_agent.rs` - Task-specific agent implementation (if exists)

### Tool System
- `tools_registry.rs` - Global state management, tool constructors
- `tools/mod.rs` - Tool submodule exports
- `tools/create_task.rs` - CreateTaskTool for sub-agent spawning
- `tools/aws_list_resources.rs` - AwsListResourcesTool
- `tools/aws_describe_resource.rs` - AwsDescribeResourceTool
- `tools/aws_find_account.rs` - AwsFindAccountTool
- `tools/aws_find_region.rs` - AwsFindRegionTool
- `tools/aws_describe_log_groups.rs` - AwsDescribeLogGroupsTool
- `tools/aws_get_log_events.rs` - AwsGetLogEventsTool
- `tools/aws_cloudtrail_lookup_events.rs` - AwsCloudTrailLookupEventsTool
- `tools/todo_read.rs` - TodoReadTool
- `tools/todo_write.rs` - TodoWriteTool

### Communication & Logging
- `callback_handlers.rs` - AgentToolCallbackHandler, JsonCaptureHandler
- `sub_agent_callback_handler.rs` - Sub-agent callback handling
- `debug_logger.rs` - Framework-wide debug logging (optional)
- `agent_logger.rs` - AgentLogger, per-agent log files

### Supporting Systems
- `model_config.rs` - Bedrock model configuration
- `performance.rs` - Agent creation performance metrics
- `cancellation.rs` - AgentCancellationManager, cancellation tokens

---

## Public API Exports

All submodule types are re-exported via `pub use` statements:

```rust
pub use agent_instance::*;    // AgentInstance, AgentId, AgentStatus, etc.
pub use agent_logger::*;      // AgentLogger, TokenUsage
pub use agent_manager::*;     // AgentManager
pub use agents::*;            // OrchestrationAgent, AwsCredentials
pub use cancellation::*;      // AgentCancellationManager
pub use debug_logger::*;      // Debug logging functions
pub use message::*;           // AgentResponse, Message, MessageRole, etc.
pub use model_config::*;      // Model configuration functions
pub use performance::*;       // Performance tracking
pub use sub_agent_callback_handler::*;  // Sub-agent callbacks
pub use tools::*;             // All tool implementations
pub use tools_registry::*;    // Global state, tool constructors
```

---

## Key Architecture Patterns

### Pattern: Module Re-exports for Simplified Imports

**Algorithm**: Flatten module hierarchy for consumers
**External**: Rust module system

Pseudocode:
  1. Define submodules with `pub mod`
  2. Re-export public items with `pub use`
  3. Consumers import from root: `use app::agent_framework::AgentId`
  4. Instead of: `use app::agent_framework::agent_instance::AgentId`
  5. Simplifies API, hides internal organization

### Pattern: Global State Management

**Algorithm**: Static RwLock in tools_registry.rs
**External**: std::sync::RwLock

Pseudocode:
  1. GLOBAL_AWS_CLIENT: shared AWSResourceClient
  2. GLOBAL_AWS_CREDENTIALS: (access_key, secret_key, session_token, region)
  3. GLOBAL_AGENT_SENDER: mpsc::Sender<AgentResponse>
  4. GLOBAL_TODO_STORAGE: Arc<Mutex<HashMap<String, Vec<TodoItem>>>>
  5. GLOBAL_CANCELLATION_MANAGER: Arc<AgentCancellationManager>
  6. GLOBAL_MODEL_CONFIG: String (model_id)
  7. Access via set_global_*/get_global_*/clear_global_* functions

### Pattern: Agent Execution Flow

**Algorithm**: Background thread with tokio runtime
**External**: std::thread, tokio::runtime::Runtime

Pseudocode:
  1. AgentManager creates AgentInstance
  2. UI calls instance.send_message(input, aws_identity)
  3. AgentInstance spawns std::thread:
     a. Get AWS credentials
     b. Create tokio::runtime::Runtime
     c. runtime.block_on(async):
        - Lazy create OrchestrationAgent (stood::Agent)
        - Call agent.execute(&input)
        - Log to AgentLogger
     d. Send AgentResponse via mpsc
  4. UI polls instance.check_responses()
  5. UI calls instance.handle_response() to update state

---

## External Dependencies

### AWS Services
- **Bedrock**: Claude 3.5 Sonnet/Haiku via stood library
- **CloudWatch Logs**: Log group/event querying
- **CloudTrail**: Event lookup for audit trails
- **Resource Groups Tagging API**: Resource discovery
- **IAM/STS**: Account discovery and credentials

### Rust Crates
- **stood**: Agent framework, tool trait, callback system
- **tokio**: Async runtime creation (not tokio::spawn)
- **std::thread**: Background thread spawning
- **std::sync**: RwLock, Mutex, Arc, mpsc
- **serde_json**: Tool parameter serialization
- **tracing**: Structured logging
- **chrono**: Timestamps
- **uuid**: AgentId generation

---

## Implementation Notes

### Agent Execution Model
- AgentManager: Simple HashMap registry, no async
- AgentInstance: Owns agent execution, background threads, channels
- OrchestrationAgent: Factory for creating stood::Agent with tools
- Background threads use std::thread, not tokio::spawn
- Tokio runtime created on-demand via Runtime::new()

### Logging Architecture
- Framework debug logger: Optional, logs high-level events
- Per-agent logger: Mandatory, one file per agent instance
- UI event logging: AgentResponse messages via mpsc channel
- Tool execution: Logged via AgentToolCallbackHandler + AgentLogger

### Tool System
- Tools registered via tool constructor functions
- Global state accessed at tool execution time
- Callback handlers convert tool events to UI messages
- Sub-agent creation via create_task tool

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects mod.rs structure and Agent Framework architecture
