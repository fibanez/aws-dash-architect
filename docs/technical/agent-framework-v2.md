# Agent Framework

Simplified agent system using stood library directly for reliable AI-powered AWS infrastructure operations through code execution.

## Core Functionality

**Agent Architecture:**
- Direct integration with stood::agent::Agent library (no wrapper layer)
- Lazy agent initialization (created on first message)
- Dedicated tokio runtime per agent for async execution
- Background thread execution model for non-blocking UI
- Comprehensive logging with per-agent log files

**Key Features:**
- Code-first problem solving via JavaScript execution
- Single tool architecture (execute_javascript only)
- Automatic API documentation generation from V8 bindings
- Session-based credential management with AWS Identity Center
- Simplified message types (User/Assistant only)
- Status tracking and UI integration
- Markdown rendering for assistant responses with syntax highlighting

**Main Components:**
- **AgentInstance**: Core agent wrapper managing lifecycle and communication
- **ConversationMessage**: Message structure for conversations
- **ConversationResponse**: Success/Error response channel for UI feedback
- **AgentLogger**: Per-agent comprehensive debug logging
- **AgentMetadata**: Agent identification and configuration

**Integration Points:**
- stood library for LLM interactions via AWS Bedrock
- V8 bindings for JavaScript code execution
- AWS Identity Center for credential acquisition
- Agent UI for visual interaction
- Background thread pool for async operations

## Implementation Details

**Key Files:**
- `src/app/agent_framework/agent_instance.rs` - Core agent implementation
- `src/app/agent_framework/agent_ui.rs` - UI components
- `src/app/agent_framework/conversation.rs` - Message types and responses
- `src/app/agent_framework/mod.rs` - Module exports

**AgentInstance Structure:**
```rust
pub struct AgentInstance {
    id: AgentId,                                // Unique identifier
    metadata: AgentMetadata,                    // Name, description, model
    status: AgentStatus,                        // Running/Stopped
    stood_agent: Arc<Mutex<Option<Agent>>>,     // Lazy-initialized stood agent
    response_channel: (Sender, Receiver),       // UI communication
    messages: VecDeque<ConversationMessage>,    // Conversation history
    processing: bool,                           // Processing flag
    logger: Arc<AgentLogger>,                   // Per-agent logging
    runtime: Arc<tokio::runtime::Runtime>,      // Dedicated async runtime
}
```

**Agent Lifecycle:**
1. **Creation**: `AgentInstance::new()` creates wrapper with metadata
2. **Initialization**: First message triggers lazy stood agent creation
3. **Message Processing**: Background thread executes agent with tokio runtime
4. **Response Delivery**: Results sent via channel to UI
5. **Logging**: All operations logged to `~/.local/share/awsdash/logs/agents/agent-{uuid}.log`

**Lazy Initialization Pattern:**
```rust
fn create_stood_agent(&self, aws_identity: &mut AwsIdentityCenter)
    -> Result<stood::agent::Agent> {
    // Get credentials from Identity Center
    let creds = aws_identity.get_default_role_credentials()?;

    // Set global credentials for execute_javascript tool
    set_global_aws_credentials(access_key, secret_key, session_token, region);

    // Build system prompt with API documentation
    let api_docs = v8_bindings::get_api_documentation();
    let system_prompt = format!("You are an AWS infrastructure agent...\n\n{}", api_docs);

    // Build agent with stood library
    Agent::builder()
        .model(Bedrock::Claude35Sonnet)
        .system_prompt(&system_prompt)
        .with_streaming(false)
        .with_credentials(access_key, secret_key, session_token, region)
        .tools(vec![execute_javascript_tool()])
        .build()
        .await
}
```

**Background Execution Model:**
```rust
pub fn send_message(&mut self, user_message: String, aws_identity: &Arc<Mutex<AwsIdentityCenter>>) {
    // Add message to conversation
    self.messages.push_back(ConversationMessage::user(user_message.clone()));
    self.processing = true;

    // Clone for background thread
    let stood_agent = Arc::clone(&self.stood_agent);
    let sender = self.response_channel.0.clone();
    let logger = Arc::clone(&self.logger);
    let runtime = Arc::clone(&self.runtime);

    // Spawn background thread with tokio runtime
    std::thread::spawn(move || {
        runtime.block_on(async move {
            let agent = stood_agent.lock().unwrap().as_mut().unwrap();
            match agent.execute(&user_message).await {
                Ok(response) => sender.send(ConversationResponse::Success(response)),
                Err(e) => sender.send(ConversationResponse::Error(e.to_string())),
            }
        })
    });
}
```

**Message Types:**
```rust
#[derive(Debug, Clone)]
pub struct ConversationMessage {
    pub role: Role,       // User or Assistant
    pub content: String,  // Message content
}

#[derive(Debug, Clone)]
pub enum ConversationResponse {
    Success(String),    // Successful agent execution
    Error(String),      // Execution error
}
```

**System Prompt Structure:**
The agent's system prompt combines static instructions with dynamic API documentation:

```
You are an AWS infrastructure agent. Think about all the steps needed to accomplish
the goal and use the execute_javascript tool and available APIs to solve problems
by writing JavaScript code.

{API_DOCUMENTATION_FROM_V8_BINDINGS}

Important: Always use the last expression in your JavaScript code as the return value.

<critical_rules>
1. Show me the exact query results without interpretation
2. Only include resources that are explicitly returned in the query data
</critical_rules>
```

**Logging Architecture:**
- **Per-agent log files**: `~/.local/share/awsdash/logs/agents/agent-{uuid}.log`
- **Comprehensive tracing**: User messages, assistant responses, tool executions
- **stood library traces**: Captured via `RUST_LOG=stood=trace` environment variable
- **UI separation**: Verbose logs separate from clean UI display

**Markdown Rendering:**

Assistant responses are automatically rendered as markdown when detected. The system uses heuristic pattern matching to identify markdown content:

```rust
fn looks_like_markdown(content: &str) -> bool {
    let patterns = [
        "```",    // Code blocks
        "\n# ",   // H1 header
        "\n## ",  // H2 header
        "\n### ", // H3 header
        "\n* ",   // Unordered list
        "\n- ",   // Unordered list
        "\n1. ",  // Ordered list
        "**",     // Bold
        "](http", // Links
    ];
    patterns.iter().any(|p| content.contains(p))
}
```

Key aspects:
- Uses `egui_commonmark` library with `better_syntax_highlighting` feature
- `CommonMarkCache` shared across agents for efficient rendering
- Code blocks display with language-aware syntax coloring
- User messages remain plain text with ">" prefix
- Fallback to plain label for non-markdown responses

**Thread Safety:**
- `Arc<Mutex<Option<Agent>>>` for lazy agent initialization
- `Arc<AgentLogger>` for shared logging across threads
- `Arc<tokio::runtime::Runtime>` for shared async runtime
- Channel-based communication between background thread and UI

## Architecture Decisions

**Why This Architecture:**

1. **Simplified Architecture**: Direct stood library usage eliminates wrapper complexity
2. **Lazy Initialization**: Agent created only when needed, improving startup time
3. **Better Error Handling**: Clearer error propagation without wrapper layers
4. **Code-First Philosophy**: Single execute_javascript tool instead of multiple specialized tools
5. **Improved Reliability**: Fewer abstraction layers mean fewer failure points

**Why Single Tool (execute_javascript):**

Traditional multi-tool approach:
```
aws_find_account → aws_find_region → aws_list_resources → aws_describe_resource
(4 separate tool calls, complex multi-step flow)
```

Code execution approach:
```javascript
const accounts = listAccounts();
const regions = listRegions('us-east-1');
const resources = listResources(accounts[0].id, regions[0].name, 'EC2::Instance');
// Single tool call, all logic in JavaScript
```

Benefits:
- **Fewer round trips**: Complex operations in single tool call
- **Better context**: LLM maintains state in JavaScript code
- **Easier debugging**: Full execution trace visible in code
- **More flexible**: LLM can write any logic, not limited to predefined tools

## Developer Notes

**Adding New JavaScript APIs:**

1. **Create V8 binding** in `src/app/agent_framework/v8_bindings/bindings/{category}.rs`:
   ```rust
   pub fn register(scope: &mut v8::ContextScope) -> Result<()> {
       let global = scope.get_current_context().global(scope);
       let fn_name = v8_string(scope, "myNewFunction")?;
       let function = v8::Function::new(scope, my_callback)?;
       global.set(scope, fn_name.into(), function.into());
       Ok(())
   }

   pub fn get_documentation() -> String {
       r#"
       ### myNewFunction()

       Description of what the function does.

       **Signature:**
       ```typescript
       function myNewFunction(param: string): ResultType
       ```

       [Full TypeScript-style documentation with JSON schema]
       "#.to_string()
   }
   ```

2. **Register in binding registry** (`bindings/mod.rs`):
   ```rust
   pub fn register_bindings(scope: &mut v8::ContextScope) -> Result<()> {
       my_category::register(scope)?;  // Add your category
       Ok(())
   }

   pub fn get_api_documentation() -> String {
       let mut docs = String::new();
       docs.push_str("\n## My Category\n\n");
       docs.push_str(&my_category::get_documentation());
       docs
   }
   ```

3. **Test with agent**: The API will automatically appear in agent's system prompt

**Debugging Agent Execution:**

1. **Enable verbose logging**:
   ```bash
   export RUST_LOG=stood=trace,awsdash=trace
   ./awsdash
   ```

2. **Check agent log file**:
   ```bash
   tail -f ~/.local/share/awsdash/logs/agents/agent-*.log
   ```

3. **Common issues**:
   - **Agent not responding**: Check stood traces for model errors
   - **JavaScript errors**: Check execute_javascript stderr output
   - **Credential errors**: Verify Identity Center configuration
   - **Tool not found**: Ensure execute_javascript registered in agent builder

**Testing Agent Behavior:**

```rust
#[tokio::test]
async fn test_agent_execution() {
    let metadata = AgentMetadata {
        name: "Test Agent".to_string(),
        model_id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
        ..Default::default()
    };

    let mut agent = AgentInstance::new(metadata);

    // Initialize with mock credentials
    let mut identity_center = mock_identity_center();
    agent.initialize(&mut identity_center).await.unwrap();

    // Send test message
    agent.send_message("List all AWS accounts".to_string(), &identity_center);

    // Wait for response
    let response = agent.poll_response();
    assert!(matches!(response, Some(ConversationResponse::Success(_))));
}
```

**Performance Considerations:**

- **Lazy initialization**: Avoids agent creation overhead until needed
- **Dedicated runtime**: Each agent has isolated tokio runtime (no contention)
- **Background execution**: UI remains responsive during agent operations
- **Credential caching**: Global credentials avoid repeated Identity Center calls
- **V8 isolate per execution**: Fresh state prevents memory leaks

**Security Considerations:**

- **Sandboxed execution**: V8 isolates provide memory isolation
- **Credential isolation**: Global credentials cleared when agent destroyed
- **No persistent storage**: Agent state only in memory
- **Session tokens**: Temporary AWS credentials with expiration
- **Audit logging**: All operations logged to per-agent files

## Extension Points

**Custom Agent Types:**

```rust
pub struct SpecializedAgent {
    base: AgentInstance,
    specialized_config: MyConfig,
}

impl SpecializedAgent {
    pub fn new(metadata: AgentMetadata, config: MyConfig) -> Self {
        Self {
            base: AgentInstance::new(metadata),
            specialized_config: config,
        }
    }

    // Override system prompt generation
    fn custom_system_prompt(&self) -> String {
        let base_prompt = self.base.create_system_prompt();
        format!("{}\n\nSpecialized instructions: ...", base_prompt)
    }
}
```

**Custom Tool Integration:**

```rust
// In stood agent builder
Agent::builder()
    .tools(vec![
        execute_javascript_tool(),
        Box::new(MyCustomTool::new()),  // Add custom tools
    ])
    .build()
    .await
```

**Custom Response Handling:**

```rust
pub enum ConversationResponse {
    Success(String),
    Error(String),
    Progress(f32),          // Add progress tracking
    ToolExecution(String),  // Add tool execution feedback
}
```

## Related Documentation

- [Agent Feedback Systems](agent-feedback-systems.md) - Status display, message injection, and conversation middleware
- [Code Execution Tool](code-execution-tool.md) - JavaScript execution system
- [AWS Data Plane Integration](aws-data-plane-integration-guide.md) - Adding AWS service bindings
- [Credential Management](credential-management.md) - AWS Identity Center integration
