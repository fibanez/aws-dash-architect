# Bridge Agent - Orchestration Agent Factory and System Prompt

## Component Overview

Bridge Agent factory module that creates AWS infrastructure management agents with
specialized tools, telemetry, and callback handlers. Extracted from UI layer to
enable standalone agent creation and testing.

---

## Major Methods/Functions

- `BridgeAgent::create_system_prompt()` - Returns AI agent system prompt
- `BridgeAgent::create()` - Async factory method creating configured Agent
- `AwsCredentials` - Credential container struct for type-safe parameter passing

---

## Implementation Patterns

### Factory Pattern
Static factory method `create()` encapsulates complex Agent builder configuration

### Type-Safe Credentials
`AwsCredentials` struct wraps credential fields reducing parameter count

### Global State Management
Sets global AWS credentials and configuration via static mutexes

### Callback Handler Registration
Registers BridgeToolCallbackHandler for tool execution tree visualization

### Async Agent Building
Uses `stood::agent::Agent::builder()` with async `build()` method

---

## External Systems

### Crates
- `stood` - Agent framework, telemetry, tool registry
- `chrono` - Timestamp generation
- `tokio` - Async runtime (implicit via stood)
- `serde_json` - JSON serialization for debugging
- `tracing` - Structured logging

### AWS Services
- AWS Identity Center - Authentication via injected credentials
- OpenTelemetry - Telemetry endpoint at localhost:4320

### Internal Dependencies
- `bridge::tools` - 5 AWS operation tools (create_task, todo_*, aws_find_*)
- `bridge::callback_handlers` - BridgeToolCallbackHandler for tool tree visualization
- `bridge::debug_logger` - AI operation logging and BridgeDebugEvent tracking
- `bridge::global_state` - Global credential management for subtask agents

---

## Struct: AwsCredentials

Purpose: Type-safe credential container for Agent creation

Fields:
- `access_key_id`: AWS access key (String)
- `secret_access_key`: AWS secret key (String)
- `session_token`: Optional session token (Option<String>)

---

## Function: create_system_prompt()

Pattern: Static method returning &'static str
Algorithm: Compile-time string literal embedding
External: None (pure function)

Pseudocode:
  1. Return comprehensive AI agent instruction string embedded at compile time
  2. Includes role definition, critical rules, tool usage guidelines
  3. Defines response format, error handling, and task completion criteria

---

## Function: create()

Pattern: Async factory method with Result<Agent, String>
Algorithm: Builder pattern with error propagation
External: stood::agent, telemetry, tools_registry, global_state

Pseudocode:
  1. Configure telemetry with OTLP endpoint and service attributes
     - Service name, version, environment (dev/staging/prod)
     - Session ID, agent type, capabilities
     - Batch processing enabled for performance

  2. Initialize debug logger with JSON capture enabled
     - Logger tracks AI operation debugging events

  3. Set global AWS credentials for task agent tool access
     - Injects credentials into static mutex for subtask agents
     - Enables TaskAgent instances to inherit credentials

  4. Build Agent using model-specific macro
     - `create_agent_with_model!` handles provider-specific config
     - Anthropic (Claude), OpenAI (GPT), Bedrock, etc.

  5. Attach system prompt via create_system_prompt()

  6. Configure AWS credentials via with_credentials()
     - Access key ID, secret access key, session token, region

  7. Attach telemetry configuration to agent builder

  8. Enable think tool for agent reasoning

  9. Register tools via tools_registry
     - create_task_tool() - Spawns specialized task agents
     - todo_write_tool(), todo_read_tool() - Task tracking
     - aws_find_account_tool() - AWS account discovery
     - aws_find_region_tool() - AWS region discovery

 10. Register callback handler
     - BridgeToolCallbackHandler - Creates tool execution tree structure

 11. Log BridgeDebugEvent::BridgeAgentStart for debugging

 12. Call async build() to instantiate Agent
     - Returns Result<Agent, String>
     - Agent ready for execute() calls with user requests

---

## Integration Points

### UI Integration (control_bridge_window.rs)
UI calls `BridgeAgent::create()` with user credentials and model selection
Agent returned to UI thread via Result type for error handling

### Task Agent Integration (task_agent.rs)
Global credentials and model config allow TaskAgent to inherit settings
Subtask agents created by bridge agent tools use same configuration

### Telemetry Integration
All agent operations traced to OTLP endpoint for observability
Spans include AWS account, region, model ID, and operation context

### Callback Integration
Tool callbacks create hierarchical message tree structure in UI
Each tool execution appears as a parent node with input/output child nodes
Enables visual tracking of tool execution flow and debugging

---

## Testing Strategy

### Unit Tests
- `test_bridge_agent_creation()` - Verifies successful agent instantiation
- `test_aws_credentials_struct()` - Validates credential container
- `test_system_prompt_not_empty()` - Ensures prompt has content
- `test_system_prompt_contains_critical_instructions()` - Validates key sections
- `test_system_prompt_contains_tool_instructions()` - Checks tool guidance
- `test_system_prompt_contains_output_format()` - Validates format spec
- `test_system_prompt_contains_error_handling()` - Checks error instructions

### Integration Tests
Would test:
- End-to-end agent creation with real AWS credentials
- Tool execution through agent.execute()
- Callback handler triggering during operations
- Telemetry span creation and propagation

---

## Error Handling

### Agent Creation Failure
Returns Result<Agent, String> with error message
UI displays error to user, retains previous agent if available

### Missing Credentials
Global credential injection validates presence of required fields
Missing credentials cause agent creation failure

### Tool Registration Failure
Tool registry errors propagate through Result type
Invalid tool definitions caught at build time

---

## Performance Considerations

### Lazy Agent Creation
Agent created only on first user message, not at window open
Reduces startup overhead when bridge window opened but unused

### Telemetry Batching
OTLP telemetry uses batch processing to reduce network overhead
Spans aggregated before transmission to telemetry endpoint

### Static String Prompt
System prompt embedded as &'static str avoids runtime allocation
Comprehensive instruction set compiled directly into binary

### Credential Cloning
AwsCredentials struct uses owned Strings to avoid lifetime complexity
Simplifies async/threading but requires String allocation

---

## Future Enhancements

- Support for dynamic tool registration beyond fixed 5 tools
- Configurable telemetry endpoint (currently hardcoded localhost:4320)
- Credential validation before agent creation (AWS STS GetCallerIdentity)
- System prompt templating for environment-specific instructions
- Agent pooling for faster subsequent requests (avoid rebuild)

---

**Last Updated**: 2025-10-25
**Source File**: `src/app/bridge/agents/bridge_agent.rs`
**Related Files**:
  - `src/app/bridge/callback_handlers.rs` - Event handling
  - `src/app/bridge/agents/task_agent.rs` - Subtask execution
  - `src/app/dashui/control_bridge_window.rs` - UI integration
