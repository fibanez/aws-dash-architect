# Task Agent - Generic AWS Task Execution System

## Component Overview

The Task Agent is the core execution engine of the AGENT FRAMEWORK AI agent system.
It provides a flexible, LLM-powered agent that can handle any AWS task
based on natural language descriptions. Rather than hardcoded specialized
agents, it dynamically creates agents with comprehensive AWS tool access,
custom system prompts, and telemetry tracking for task execution.

---

## Major Methods/Functions

### `TaskAgent::create()`
Creates a new generic task agent with comprehensive AWS toolset and context

### `TaskAgent::execute_task()`
Executes a task description using the created agent and returns structured
results

### `TaskAgent::create_system_prompt()`
Generates dynamic system prompt with task description, AWS context, and
tool usage instructions

---

## Implementation Patterns

### Design Patterns
- **Builder Pattern**: Uses `Agent::builder()` for flexible agent construction
- **Strategy Pattern**: Dynamic system prompt generation based on task context
- **Facade Pattern**: Simplifies complex agent creation into single `create()` call
- **Template Method**: Structured workflow in system prompt (plan → execute → report)

### Rust Idioms
- **Result Error Propagation**: Uses `?` operator with `stood::StoodError` type
- **Option Chaining**: `model_id.or_else().unwrap_or_else()` for fallback logic
- **Async/Await**: Async agent creation and execution with tokio runtime
- **Macro Usage**: Custom `create_agent_with_model!` macro for model configuration
- **Structured Logging**: Uses `tracing` crate with info/debug/warn levels

### Performance Optimizations
- **PerformanceTimer**: Custom timing with `time_phase!` macro for profiling
- **Batch Processing**: Telemetry configured with batching for efficiency
- **Credential Reuse**: Global credential caching to avoid repeated authentication

### Security Patterns
- **Credential Sanitization**: System prompt includes security guidelines
- **No Credential Logging**: Explicitly instructs agent not to expose secrets
- **Session Isolation**: Unique session ID per agent instance

---

## External Systems

### AWS Services
- **CloudWatch Logs**: Log group enumeration and event retrieval
- **CloudTrail**: 90-day event history querying across accounts/regions
- **Resource Groups**: Multi-service resource listing and description
- **CloudFormation**: Template reading from current project context

### Third-Party Crates
- **stood**: Core agent framework (Agent, StoodError, TelemetryConfig)
- **serde_json**: JSON serialization for structured results
- **chrono**: Timestamp generation for telemetry and logging
- **uuid**: Task ID generation for execution tracking
- **tracing**: Structured logging with span support

### Internal Dependencies
- **tools_registry**: AWS tool definitions (12 AWS operation tools)
- **model_config**: Model selection and configuration
- **debug_logger**: Bridge event logging for debugging agent interactions
- **PerformanceTimer**: Custom timing infrastructure

---

## Function: create()

**Pattern**: Builder pattern with async initialization and credential injection
**Algorithm**: Sequential configuration with performance timing
**External**: stood::agent::Agent, AWS credential provider, telemetry system

**Pseudocode**:
```
1. Initialize performance timer for profiling agent creation
2. Log task description and AWS context (accounts, regions)
3. Determine model to use (parameter > global > default fallback)
4. Create dynamic system prompt with task and AWS context
5. Configure telemetry with OTLP endpoint and service attributes
   - Set agent type, task description, accounts, regions
   - Generate unique session ID for this agent instance
6. Build agent using builder pattern:
   - Set model (Claude Sonnet, Haiku, etc. via macro)
   - Set system prompt with task instructions
   - Configure telemetry for observability
   - Add Think tool for reasoning
   - Register 11 AWS tools (todo, CloudFormation, logs, trail, resources)
   - Remove streaming callbacks for natural event loop
7. Inject global AWS credentials if available, else use default chain
8. Build agent (async operation, critical timing point)
9. Log BridgeDebugEvent::TaskAgentCreated with full context
10. Complete performance timer and return Result<Agent, StoodError>
```

---

## Function: execute_task()

**Pattern**: Async execution with structured result formatting
**Algorithm**: Single-shot agent execution with telemetry logging
**External**: stood::Agent, BridgeDebugEvent logging system

**Pseudocode**:
```
1. Log task execution start with description
2. Generate unique task ID using UUID v4
3. Log BridgeDebugEvent::TaskPromptSent with timestamp and user message
4. Execute task using agent.execute(task_description).await
   - Agent handles its own tool calling loop
   - No streaming callbacks - natural event loop
5. Log task completion success
6. Extract tool calls from response for debugging
7. Log BridgeDebugEvent::TaskResponseReceived with full response
8. Return structured JSON result:
   {
     "task_type": "generic-task-agent",
     "task_completed": true,
     "response": <agent response text>,
     "execution_summary": {
       "cycles": <agentic loop iterations>,
       "model_calls": <LLM invocations>,
       "tool_executions": <tool call count>,
       "used_tools": <tool names array>,
       "success": <boolean>
     },
     "timestamp": <RFC3339 timestamp>
   }
```

---

## Function: create_system_prompt()

**Pattern**: Template-based prompt engineering with context injection
**Algorithm**: String formatting with conditional pluralization
**External**: None (pure function)

**Pseudocode**:
```
1. Format account IDs text:
   - Single account: "Account ID: <id>"
   - Multiple accounts: "Account IDs: <id1>, <id2>, ..."
2. Format regions text:
   - Single region: "Region: <region>"
   - Multiple regions: "Regions: <region1>, <region2>, ..."
3. Build system prompt with 6 sections:
   a. Task Description: "You are an AWS task specialist. Execute: <task>"
   b. AWS Context: Account IDs and regions for operations
   c. Task Execution Workflow:
      - Step 1: Use TodoWrite to plan approach
      - Step 2: Select appropriate AWS tools based on task needs
      - Step 3: Execute systematically, marking todos complete
      - Step 4: Provide comprehensive summary
   d. Multi-Account/Region Operations: Guidance for cross-account tasks
   e. Security Guidelines: Never expose credentials, sanitize data
   f. Available Tools: Full tool catalog with descriptions
   g. Expected Output: Format for findings and recommendations
4. Return formatted prompt string with all context injected
```

---

## Tool Integration Architecture

The Task Agent integrates 11 tools organized in 4 categories:

**Task Management** (2 tools):
- `todo_write_tool()`: Progress tracking (required first step)
- `todo_read_tool()`: Task status checking

**CloudFormation** (1 tool):
- `read_cloudformation_template_tool()`: Project template access

**CloudWatch** (2 tools):
- `aws_describe_log_groups_tool(None)`: Log group enumeration
- `aws_get_log_events_tool(None)`: Event retrieval with filtering

**CloudTrail** (1 tool):
- `aws_cloudtrail_lookup_events_tool(None)`: 90-day event history

**Resource Operations** (2 tools):
- `aws_list_resources_tool(None)`: Resource listing (multi-account/region)
- `aws_describe_resource_tool(None)`: Detailed resource information

**Context Lookup** (2 tools):
- `aws_find_account_tool()`: Account ID resolution (no API calls)
- `aws_find_region_tool()`: Region resolution (no API calls)

All AWS tools accept `None` for credentials, relying on global credential
injection during agent building phase.

---

## Telemetry and Observability

**Telemetry Configuration**:
- Service: "aws-task-agent" v1.0.0
- OTLP Endpoint: http://localhost:4320 (HTTP, not gRPC)
- Processing: Batch mode for efficiency
- Debug Tracing: Enabled for detailed span tracking

**Service Attributes** (OpenTelemetry labels):
- `agent.type`: "generic-task-agent"
- `task.description`: User-provided task text
- `aws.account_ids`: Comma-separated account list
- `aws.regions`: Comma-separated region list
- `task.id`: Unique task identifier
- `session.id`: "task-agent-<unix_timestamp_millis>"

**Debug Events Logged**:
1. `TaskAgentCreated`: Full system prompt and model ID
2. `TaskPromptSent`: User message sent to agent
3. `TaskResponseReceived`: Full response and extracted tool calls

---

## Multi-Account/Multi-Region Support

The Task Agent supports cross-account and cross-region operations through:

1. **Context Injection**: Account IDs and regions passed to `create()`
2. **System Prompt Guidance**: Instructions for multi-account workflows
3. **Tool Array Support**: AWS tools accept arrays of accounts/regions
4. **Aggregation Instructions**: Agent prompted to organize results by account/region

**Example**:
```rust
TaskAgent::create(
    "task-123".to_string(),
    "List all S3 buckets".to_string(),
    vec!["111111111111".to_string(), "222222222222".to_string()],
    vec!["us-east-1".to_string(), "eu-west-1".to_string()],
    None, // Use default model
).await?
```

Agent will systematically process all account/region combinations and
aggregate findings in structured format.

---

## Model Selection Logic

The agent uses a three-tier model selection strategy:

1. **Explicit Parameter**: Use `model_id` if provided to `create()`
2. **Global Configuration**: Fall back to `get_global_model()` setting
3. **Default Fallback**: Use `ModelConfig::default_model_id()` if no config

This allows per-task model override while respecting global user preferences.

**Supported Models** (via `create_agent_with_model!` macro):
- Claude Sonnet variants (default, high-capability)
- Claude Haiku variants (faster, lower cost)
- Configured via model ID strings

---

## Testing Strategy

**Unit Tests**:
- `test_system_prompt_creation`: Validates prompt formatting for single account/region
- `test_system_prompt_creation_multiple_accounts_regions`: Multi-context handling
- `test_agent_creation_components`: Component creation without live AWS calls

**Integration Tests**:
- Real agent execution tested in parent Bridge integration tests
- Requires live AWS credentials and Bedrock access
- Tests full agent lifecycle: create → execute → result parsing

**Test Coverage**:
- System prompt generation: ✅ Complete
- Multi-account/region formatting: ✅ Complete
- Agent creation: ⚠️ Component-only (no live API)
- Agent execution: ⚠️ Integration tests only

---

## Performance Characteristics

**Agent Creation**:
- System prompt: ~10µs (string formatting)
- Telemetry config: ~100µs (struct initialization)
- Agent builder: ~1ms (builder pattern)
- Credential injection: Variable (cached vs. fresh)
- Agent.build(): **Critical timing point** (model initialization)

**Task Execution**:
- Depends on task complexity and tool usage
- Each LLM call: ~1-5 seconds (Bedrock API latency)
- Tool executions: Variable by AWS service
- Agentic loop: Typically 2-5 cycles for simple tasks

**Memory**:
- Minimal overhead (~100KB per agent instance)
- Telemetry batching reduces memory pressure
- No persistent state between executions

---

## Error Handling

**Error Types**:
- `stood::StoodError`: Agent creation or execution failures
- Propagated via `?` operator to caller

**Error Sources**:
1. Agent build failure (model unavailable, config invalid)
2. Agent execution failure (tool errors, timeout)
3. Credential errors (missing or invalid AWS credentials)

**Recovery Strategy**:
- No automatic retry in TaskAgent (handled by caller)
- Errors logged via `warn!()` for debugging
- Structured error context via `anyhow` in tools

---

## Security Considerations

**Credential Handling**:
- Never logs AWS credentials (access key, secret key, session token)
- Uses global credential cache when available
- Falls back to AWS default credential chain securely

**Prompt Injection Defense**:
- System prompt includes explicit security guidelines
- Agent instructed to sanitize sensitive data
- No user input directly concatenated into prompt (uses structured format)

**Tool Access Control**:
- All tools registered explicitly (no dynamic tool loading)
- Read-only operations (no write/delete capabilities)
- CloudTrail and CloudWatch are audit tools, not control plane

---

## Future Enhancements

**Planned Features**:
- Streaming support for long-running tasks
- Tool call caching for repeated operations
- Parallel tool execution for multi-account queries
- Custom tool registration per task type
- Agent state persistence across executions

**Optimization Opportunities**:
- Lazy agent initialization (defer build until execute)
- Prompt template caching to avoid repeated formatting
- Telemetry sampling for high-frequency tasks
- Tool result caching with TTL

---

**Last Updated**: 2025-10-25
**Maintainer**: Development Team
**Status**: Active Development - Core AGENT FRAMEWORK component
