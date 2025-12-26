# Agent Framework Module - AI Agent System for AWS Operations

## Component Overview

Module exports for the Agent Framework system. This file provides public API
surface by re-exporting types from submodules. Actual implementation lives in
individual component files.

**Pattern**: Module organization with public re-exports
**External Systems**: AWS Bedrock (Claude models), AWS services via tools, V8 JavaScript engine
**Purpose**: AI-driven AWS infrastructure management through natural language

---

## Module Structure

### Core System Files
- `mod.rs` - This file: module exports and public API
- `agent_instance.rs` - AgentInstance with AgentType (TaskManager, TaskWorker)
- `agent_types.rs` - AgentId, AgentStatus, AgentType definitions
- `agent_creation.rs` - Agent creation request/response channels
- `agent_logger.rs` - Per-agent logging system
- `agent_ui.rs` - UI-related agent functionality
- `conversation.rs` - ConversationMessage, ConversationRole, ConversationResponse

### Prompts System
- `prompts/mod.rs` - Prompt module exports
- `prompts/task_manager.rs` - TASK_MANAGER_PROMPT for orchestration agents
- `prompts/task_worker.rs` - TASK_WORKER_PROMPT for worker agents

### Skills System
- `skills/mod.rs` - Skills module exports
- `skills/discovery.rs` - SkillDiscoveryService, SkillMetadata
- `skills/loader.rs` - SkillLoader, LoadedSkill
- `skills/manager.rs` - SkillManager global singleton

### V8 JavaScript Engine
- `v8_bindings/mod.rs` - V8 module exports
- `v8_bindings/platform.rs` - V8 platform initialization
- `v8_bindings/runtime.rs` - V8Runtime, RuntimeConfig, ExecutionResult
- `v8_bindings/console.rs` - JavaScript console API
- `v8_bindings/types.rs` - V8 type conversions
- `v8_bindings/bindings/mod.rs` - Function binding registration
- `v8_bindings/bindings/accounts.rs` - listAccounts() binding
- `v8_bindings/bindings/regions.rs` - listRegions() binding
- `v8_bindings/bindings/resources.rs` - queryResources() binding
- `v8_bindings/bindings/cloudwatch_logs.rs` - queryCloudWatchLogEvents() binding
- `v8_bindings/bindings/cloudtrail_events.rs` - getCloudTrailEvents() binding

### Tool System
- `tools_registry.rs` - Global state management (AWS client, credentials, cancellation, model)
- `tools/mod.rs` - Tool submodule exports
- `tools/execute_javascript.rs` - ExecuteJavaScriptTool (V8 engine)
- `tools/start_task.rs` - StartTaskTool for sub-agent spawning
- `tools/think.rs` - ThinkTool (reasoning/planning)
- `tools/todo_read.rs` - TodoReadTool
- `tools/todo_write.rs` - TodoWriteTool
- `tools/todo_types.rs` - TodoItem, TodoStatus type definitions
- `tools/file_security.rs` - File security validation utilities

### Supporting Systems
- `model_config.rs` - Bedrock model configuration
- `cancellation.rs` - AgentCancellationManager, cancellation tokens
- `tool_context.rs` - Tool execution context
- `ui_events.rs` - UI event handling
- `worker_completion.rs` - Worker completion tracking

---

## Public API Exports

All submodule types are re-exported via `pub use` statements:

```rust
pub use agent_creation::*;        // Agent creation channels
pub use agent_instance::*;        // AgentInstance
pub use agent_logger::*;          // AgentLogger, TokenUsage
pub use agent_types::*;           // AgentId, AgentStatus, AgentType
pub use agent_ui::*;              // UI-related functionality
pub use cancellation::*;          // AgentCancellationManager
pub use conversation::*;          // ConversationMessage, ConversationRole, ConversationResponse
pub use model_config::*;          // Model configuration functions
pub use prompts::{TASK_MANAGER_PROMPT, TASK_WORKER_PROMPT};
pub use skills::*;                // Skill system
pub use tool_context::*;          // Tool execution context
pub use tools::*;                 // All tool implementations
pub use tools_registry::*;        // Global state management
pub use ui_events::*;             // UI event handling
pub use v8_bindings::*;           // V8 JavaScript engine
pub use worker_completion::*;     // Worker completion tracking
```

---

## Key Architecture Patterns

### Pattern: Unified Agent Instance with Type Variants

**Algorithm**: Single AgentInstance class with AgentType enum
**External**: stood::Agent, prompts module

Pseudocode:
  1. AgentInstance created with AgentType (TaskManager or TaskWorker)
  2. TaskManager agents orchestrate complex tasks, spawn workers
  3. TaskWorker agents execute specific tasks using JavaScript
  4. Parent-child relationships tracked via parent_id
  5. Prompt selected based on agent type from prompts module

### Pattern: Module Re-exports for Simplified Imports

**Algorithm**: Flatten module hierarchy for consumers
**External**: Rust module system

Pseudocode:
  1. Define submodules with `pub mod`
  2. Re-export public items with `pub use`
  3. Consumers import from root: `use app::agent_framework::AgentId`
  4. Instead of: `use app::agent_framework::agent_types::AgentId`
  5. Simplifies API, hides internal organization

### Pattern: Global State Management

**Algorithm**: Static RwLock in tools_registry.rs
**External**: std::sync::RwLock

Pseudocode:
  1. GLOBAL_AWS_CLIENT: shared AWSResourceClient
  2. GLOBAL_AWS_CREDENTIALS: (access_key, secret_key, session_token, region)
  3. GLOBAL_CANCELLATION_MANAGER: Arc<AgentCancellationManager>
  4. GLOBAL_MODEL_CONFIG: String (model_id)
  5. Access via set_global_*/get_global_*/clear_global_* functions

### Pattern: Agent Execution Flow

**Algorithm**: Background thread with tokio runtime
**External**: std::thread, tokio::runtime::Runtime

Pseudocode:
  1. AgentInstance spawns std::thread:
     a. Get AWS credentials
     b. Create tokio::runtime::Runtime
     c. runtime.block_on(async):
        - Select prompt based on AgentType
        - Lazy create stood::Agent with tools
        - Call agent.execute(&input)
        - Log to AgentLogger
     d. Send ConversationResponse via mpsc channel
  2. UI polls instance.check_responses()
  3. UI calls instance.handle_response() to update state
  4. ConversationMessage added to message history

### Pattern: JavaScript Execution via V8

**Algorithm**: V8 sandbox with AWS API bindings
**External**: v8 crate, rusty_v8

Pseudocode:
  1. initialize_v8_platform() called at app startup
  2. V8Runtime created per execution
  3. Bindings registered: listAccounts(), listRegions(), queryResources(), etc.
  4. Console API for debug output
  5. Execution with timeout and memory limits
  6. Results captured as JSON strings

---

## External Dependencies

### AWS Services
- **Bedrock**: Claude 3.5 Sonnet/Haiku via stood library
- **CloudWatch Logs**: Log group/event querying (via V8 bindings)
- **CloudTrail**: Event lookup for audit trails (via V8 bindings)
- **Resource Groups Tagging API**: Resource discovery (via V8 bindings)
- **IAM/STS**: Account discovery and credentials

### Rust Crates
- **stood**: Agent framework, tool trait, callback system
- **v8/rusty_v8**: JavaScript execution engine
- **tokio**: Async runtime creation
- **std::thread**: Background thread spawning
- **std::sync**: RwLock, Mutex, Arc, mpsc
- **serde_json**: Tool parameter serialization
- **serde_yaml**: Skill metadata parsing
- **walkdir**: Skill directory scanning
- **tracing**: Structured logging
- **chrono**: Timestamps
- **uuid**: AgentId generation

---

## Implementation Notes

### Agent Type System
- AgentType enum: TaskManager, TaskWorker
- TaskManager: Orchestrates complex tasks, spawns workers
- TaskWorker: Executes specific tasks via JavaScript
- Parent-child tracking via parent_id field
- Prompt selection based on agent type

### JavaScript Execution Model
- V8 engine with memory limits (256MB default)
- Timeout enforcement (30s default)
- AWS API bindings for resource queries
- Console output capture (stdout/stderr)
- JSON serialization of results

### Skill System
- Skills discovered from ~/.claude/skills and ~/.awsdash/skills
- YAML frontmatter in SKILL.md files for metadata
- Progressive disclosure: metadata cheap, content on-demand
- Global SkillManager singleton for access

### Tool System
- Tools registered via AgentInstance based on AgentType
- TaskManager gets: ThinkTool, StartTaskTool
- TaskWorker gets: ExecuteJavaScriptTool
- Global state accessed at tool execution time via tools_registry

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects mod.rs structure and Agent Framework architecture
