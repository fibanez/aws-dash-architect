# Agent Framework Execution Trace - "Find EC2 instance called Terminus"

Complete pseudocode trace showing message flow, storage, and UI updates.

---

## Scenario

User clicks "New Agent", types: "Find EC2 instance called Terminus"
Agent calls: aws_find_account → aws_find_region → aws_list_resources → create_task (sub-agent)

---

## Execution Trace with File References

### 1. UI: User Creates Agent

**File**: `src/app/dashui/agent_manager_window.rs` (hypothetical, not in this worktree)

```
Pseudocode:
  User clicks "New Agent" button
  UI creates dialog with name="AWS Agent", model="claude-3-5-haiku"
  User confirms

  → Call: agent_manager.launch_agent(name, desc, model_id)
```

**File**: `src/app/agent_framework/agent_manager.rs:23`

```
Pseudocode:
  fn launch_agent(name, description, model_id):
    agent_id = AgentId::new()  // Generate UUID
    metadata = AgentMetadata {
      name: "AWS Agent",
      description: "User's agent",
      model_id: "claude-3-5-haiku",
      created_at: Utc::now(),
      updated_at: Utc::now()
    }
    instance = AgentInstance::new(agent_id, metadata)

    → Store: agents HashMap[agent_id] = instance
    → Return: agent_id to UI
```

**File**: `src/app/agent_framework/agent_instance.rs:74`

```
Pseudocode:
  fn AgentInstance::new(id, metadata):
    (tx, rx) = mpsc::channel()  // Create message channel
    logger = AgentLogger::new(id, metadata.name)

    → Log to: ~/.local/share/awsdash/logs/agents/agent-{uuid}.log
       "🤖 AGENT SESSION STARTED"
       "Agent ID: {id}"
       "Agent Name: AWS Agent"

    → Store internally:
       self.id = agent_id
       self.metadata = metadata
       self.status = AgentStatus::Running
       self.agent = Arc::new(Mutex::new(None))  // Lazy init
       self.response_channel = (tx, rx)
       self.messages = VecDeque::new()  // Message history
       self.processing = false
       self.logger = Arc::new(logger)

    → Return: AgentInstance
```

---

### 2. UI: User Sends Message

**File**: `src/app/agent_framework/agent_instance.rs:202`

```
Pseudocode:
  User types: "Find EC2 instance called Terminus"
  UI calls: instance.send_message(input, aws_identity)

  fn send_message(input, aws_identity):
    // 2.1: Store user message
    user_msg = Message {
      id: Uuid::new_v4(),
      role: MessageRole::User,
      content: "Find EC2 instance called Terminus",
      timestamp: Utc::now(),
      agent_source: Some("User")
    }

    → Store: self.messages.push_back(user_msg)
    → Log to agent log file: "👤 USER: Find EC2 instance called Terminus"

    self.processing = true

    // 2.2: Spawn background thread
    std::thread::spawn(move || {
      // Get AWS credentials
      (aws_creds, region) = aws_identity.lock().get_default_role_credentials()

      // Create tokio runtime
      runtime = tokio::runtime::Runtime::new()

      runtime.block_on(async {
        // 2.3: Lazy create agent
        agent_guard = agent.lock()
        if agent_guard.is_none():
          credentials = AwsCredentials {
            access_key_id: aws_creds.access_key_id,
            secret_access_key: aws_creds.secret_access_key,
            session_token: aws_creds.session_token
          }

          → File: src/app/agent_framework/agents/orchestration_agent.rs:47
```

**File**: `src/app/agent_framework/agents/orchestration_agent.rs:47`

```
Pseudocode:
  fn OrchestrationAgent::create(model_id, credentials, region, sender, input, logger):
    system_prompt = create_system_prompt()  // See orchestration_agent.rs:46

    → Log to agent log: "📤 MODEL REQUEST"
       "Model: claude-3-5-haiku"
       "System prompt: {length} chars"
       "User input: {length} chars"

    // Build stood Agent with tools and callbacks
    agent_builder = Agent::builder()
      .system_prompt(system_prompt)

    // Register tools
    → File: src/app/agent_framework/tools_registry.rs:125-179
      .add_tool(aws_find_account_tool())
      .add_tool(aws_find_region_tool())
      .add_tool(create_task_tool())
      .add_tool(todo_write_tool())
      .add_tool(todo_read_tool())

    // Attach callback handlers
    → File: src/app/agent_framework/callback_handlers.rs:19
      .add_handler(AgentToolCallbackHandler::new(sender.clone(), logger.clone()))
      .add_handler(JsonCaptureHandler::new(sender.clone()))

    // Create with model
    → File: src/app/agent_framework/model_config.rs:11
      agent = create_agent_with_model!(agent_builder, model_id).build()

    → Return: stood::Agent
```

---

### 3. Agent Execution: Model Reasoning & Tool Calls

**File**: `src/app/agent_framework/agent_instance.rs:302`

```
Pseudocode:
  // Agent now exists, execute
  result = agent.execute(&input).await

  → stood library calls AWS Bedrock Claude API:
     POST to AWS Bedrock with:
       - System prompt (orchestration instructions)
       - User message: "Find EC2 instance called Terminus"
       - Available tools: [aws_find_account, aws_find_region, create_task, ...]

  → Model thinks:
     "I need account and region before I can list EC2 instances"
     "First: call aws_find_account to get account ID"
```

---

### 4. Tool Call #1: aws_find_account

**File**: `src/app/agent_framework/callback_handlers.rs:145` (on_tool callback)

```
Pseudocode:
  → stood library triggers: ToolEvent::Started

  fn AgentToolCallbackHandler::on_tool(ToolEvent::Started):
    tool_name = "aws_find_account"
    tool_input = { query: "production" }  // Model decides to search for "production"
    tool_node_id = format!("tool_{}_{}", tool_name, timestamp)

    // 4.1: Create parent message for tool start
    parent_message = Message {
      id: tool_node_id.clone(),
      role: MessageRole::System,
      content: "Find Account",  // User-friendly mapping
      timestamp: Utc::now(),
      nested_messages: [
        Message {
          role: MessageRole::JsonRequest,
          content: pretty_json(tool_input),
          timestamp: Utc::now()
        }
      ]
    }

    → Store: active_tool_nodes[tool_name] = tool_node_id

    → Send via mpsc: AgentResponse::ToolCallStart { parent_message }

    → Log to agent log:
       "🔧 TOOL START: aws_find_account"
       "Input: { query: 'production' }"
```

**File**: `src/app/agent_framework/agent_instance.rs:156` (UI polling)

```
Pseudocode:
  // UI calls check_responses() each frame
  fn check_responses():
    responses = []
    while let Ok(response) = self.response_channel.1.try_recv():
      responses.push(response)
    return responses

  // UI receives: AgentResponse::ToolCallStart
  fn handle_response(AgentResponse::ToolCallStart { parent_message }):
    → Store: self.messages.push_back(parent_message)

    → UI displays:
       System message: "Find Account"
       Expandable JSON: { "query": "production" }
```

**File**: `src/app/agent_framework/tools/aws_find_account.rs` (tool execution)

```
Pseudocode:
  → stood calls: AwsFindAccountTool::execute(input)

  fn execute(input):
    query = input.query  // "production"

    → Access: global AWS Identity Center via set_global_aws_identity()
    identity = get_global_aws_identity()
    accounts = identity.accounts  // Cached from login

    // Fuzzy search
    results = accounts.filter(|acc|
      acc.name.contains(query) || acc.id.contains(query)
    )

    → Return: Vec<AccountSearchResult> {
      account_id: "123456789012",
      name: "Production",
      email: "prod@example.com",
      role_name: "AdministratorAccess"
    }
```

**File**: `src/app/agent_framework/callback_handlers.rs:185` (tool complete)

```
Pseudocode:
  → stood triggers: ToolEvent::Completed

  fn AgentToolCallbackHandler::on_tool(ToolEvent::Completed):
    tool_name = "aws_find_account"
    tool_output = { results: [...] }
    parent_message_id = active_tool_nodes.remove(tool_name)

    // 4.2: Create child message for tool result
    child_message = Message {
      id: Uuid::new_v4(),
      role: MessageRole::Assistant,
      content: "✅ Found 1 account: Production (123456789012)",
      timestamp: Utc::now(),
      nested_messages: [
        Message {
          role: MessageRole::JsonResponse,
          content: pretty_json(tool_output)
        }
      ]
    }

    → Send via mpsc: AgentResponse::ToolCallComplete {
      parent_message_id,
      child_message
    }

    → Log to agent log:
       "✅ TOOL SUCCESS: aws_find_account (45ms)"
       "Output: { results: [...] }"
```

**File**: `src/app/agent_framework/agent_instance.rs:429` (UI update)

```
Pseudocode:
  fn handle_response(AgentResponse::ToolCallComplete { parent_message_id, child_message }):
    → Store: self.messages.push_back(child_message)

    → UI displays under parent:
       System: "Find Account"
         ├─ Request JSON: { "query": "production" }
         └─ Assistant: "✅ Found 1 account: Production (123456789012)"
              └─ Response JSON: { "results": [...] }
```

---

### 5. Tool Call #2: aws_find_region

**Same flow as above, summarized**:

```
Pseudocode:
  Model thinks: "Now I need the region"

  → ToolEvent::Started("aws_find_region", { query: "us-east" })
  → AgentResponse::ToolCallStart (parent message)
  → UI stores and displays: "Find Region" with request JSON

  → Tool executes: search static region list
  → Returns: { region_code: "us-east-1", region_name: "US East (N. Virginia)" }

  → ToolEvent::Completed
  → AgentResponse::ToolCallComplete (child message)
  → UI displays: "✅ Found region: us-east-1" with response JSON

  → Log to agent log:
     "🔧 TOOL START: aws_find_region"
     "✅ TOOL SUCCESS: aws_find_region (12ms)"
```

---

### 6. Tool Call #3: create_task (Sub-Agent Creation)

**File**: `src/app/agent_framework/tools/create_task.rs` (tool execution)

```
Pseudocode:
  Model thinks: "I should delegate EC2 listing to a specialized task agent"

  → ToolEvent::Started("create_task", {
      task_description: "List EC2 instances named 'Terminus' in Production account us-east-1",
      accounts: ["123456789012"],
      regions: ["us-east-1"]
    })

  → AgentResponse::ToolCallStart
  → UI displays: "Task: List EC2 instances named 'Terminus'..."
```

**File**: `src/app/agent_framework/tools/create_task.rs:50` (approximate)

```
Pseudocode:
  fn CreateTaskTool::execute(input):
    task_description = input.task_description
    accounts = input.accounts  // ["123456789012"]
    regions = input.regions    // ["us-east-1"]

    sub_agent_id = format!("task_{}", Uuid::new_v4())

    → Log to orchestration agent log:
       "🔧 TOOL START: create_task"
       "Creating sub-agent: {sub_agent_id}"
       "Task: {task_description}"
       "Accounts: {accounts:?}"
       "Regions: {regions:?}"

    // Create cancellation token
    cancellation_manager = get_global_cancellation_manager()
    token = cancellation_manager.create_token(sub_agent_id)

    // Get global AWS credentials
    aws_creds = get_global_aws_credentials()

    // Build TaskAgent (similar to OrchestrationAgent)
    → File: src/app/agent_framework/agents/task_agent.rs

    agent_builder = Agent::builder()
      .system_prompt("You are a TaskAgent for AWS operations...")

      // TaskAgent has MORE tools than OrchestrationAgent
      .add_tool(aws_list_resources_tool(global_client))
      .add_tool(aws_describe_resource_tool(global_client))
      .add_tool(aws_describe_log_groups_tool(global_client))
      .add_tool(aws_get_log_events_tool(global_client))
      .add_tool(aws_cloudtrail_lookup_events_tool(global_client))
      .add_tool(todo_write_tool())
      .add_tool(todo_read_tool())

      // TaskAgent does NOT have create_task (no recursive sub-agents)

      .add_handler(SubAgentCallbackHandler::new(sub_agent_id))

    task_agent = agent_builder.build()

    // Spawn async task for sub-agent execution
    tokio::spawn(async move {
      → Log to new sub-agent log: ~/.local/share/awsdash/logs/agents/agent-{sub_agent_id}.log
         "🤖 SUB-AGENT SESSION STARTED"
         "Task: List EC2 instances named 'Terminus'..."

      // Execute task agent
      result = tokio::select! {
        res = task_agent.execute(&task_description) => res,
        _ = token.cancelled() => return  // Cancellation
      }

      → Sub-agent calls: aws_list_resources
         → ToolEvent::Started("aws_list_resources", {
              resource_type: "ec2:instance",
              account_id: "123456789012",
              region: "us-east-1",
              filters: [{ name: "tag:Name", values: ["Terminus"] }]
            })

         → Tool queries AWS EC2:
            ec2_client.describe_instances(filters)

         → Returns: Vec<ResourceSummary> {
              resource_type: "ec2:instance",
              resource_id: "i-1234567890abcdef0",
              display_name: "Terminus",
              status: Some("running"),
              tags: ["Name=Terminus", "Environment=Production"]
            }

         → ToolEvent::Completed

         → Log to sub-agent log:
            "🔧 TOOL START: aws_list_resources"
            "✅ TOOL SUCCESS: aws_list_resources (234ms)"
            "Output: Found 1 EC2 instance"

      // Sub-agent completes
      → Log to sub-agent log:
         "📥 MODEL RESPONSE"
         "Status: Success"
         "Output: I found the EC2 instance 'Terminus' (i-1234567890abcdef0) in Production account, running in us-east-1"
         "✅ AGENT TERMINATED: Completed"
    })

    → Return to orchestration agent:
       "✅ Task agent created: {sub_agent_id}"
       "Results will be logged to agent-{sub_agent_id}.log"
```

**File**: `src/app/agent_framework/callback_handlers.rs:185`

```
Pseudocode:
  → ToolEvent::Completed("create_task")

  → AgentResponse::ToolCallComplete
  → UI displays under parent:
       "Task: List EC2 instances named 'Terminus'..."
         └─ "✅ Task agent created, see log file for results"

  → Log to orchestration agent log:
     "✅ TOOL SUCCESS: create_task (1.2s)"
```

---

### 7. Agent Completes: Final Response

**File**: `src/app/agent_framework/agent_instance.rs:302`

```
Pseudocode:
  // agent.execute() completes
  agent_result = AgentResult {
    response: "I found the EC2 instance 'Terminus' in your Production account (123456789012)
               in the us-east-1 region. The instance ID is i-1234567890abcdef0 and it's
               currently running. I created a task agent to perform the detailed lookup.",
    success: true,
    used_tools: true,
    tools_called: ["aws_find_account", "aws_find_region", "create_task"]
  }

  duration_ms = start_time.elapsed().as_millis()

  → Log to agent log:
     "📥 MODEL RESPONSE"
     "Status: Success"
     "Duration: {duration_ms}ms"
     "Output: {response} ({length} chars)"

  → Send via mpsc: AgentResponse::Success(agent_result)
```

**File**: `src/app/agent_framework/agent_instance.rs:353`

```
Pseudocode:
  fn handle_response(AgentResponse::Success(agent_result)):
    assistant_msg = Message {
      id: Uuid::new_v4(),
      role: MessageRole::Assistant,
      content: agent_result.response,
      timestamp: Utc::now(),
      agent_source: Some(self.metadata.name)
    }

    → Store: self.messages.push_back(assistant_msg)
    → Log to agent log: "⚡ ASSISTANT: {response}"

    self.processing = false

    → UI displays:
       Assistant: "I found the EC2 instance 'Terminus' in your Production account..."
```

---

## Message Storage Summary

### In-Memory (Runtime)
**File**: `src/app/agent_framework/agent_instance.rs:22`
```
AgentInstance {
  messages: VecDeque<Message>  // Max 100 messages
    ├─ Message { role: User, content: "Find EC2 instance called Terminus" }
    ├─ Message { role: System, content: "Find Account", nested_messages: [...] }
    │   ├─ Message { role: JsonRequest, content: '{"query":"production"}' }
    │   └─ Message { role: Assistant, content: "✅ Found 1 account", nested_messages: [...] }
    │       └─ Message { role: JsonResponse, content: '{"results":[...]}' }
    ├─ Message { role: System, content: "Find Region", nested_messages: [...] }
    ├─ Message { role: System, content: "Task: List EC2...", nested_messages: [...] }
    └─ Message { role: Assistant, content: "I found the EC2 instance 'Terminus'..." }
}
```

### On Disk (Persistent)
**File**: `~/.local/share/awsdash/logs/agents/agent-{uuid}.log`
```
================================================================================
🤖 AGENT SESSION STARTED: 2025-01-28 17:30:00 UTC
Agent ID: 12345678-1234-1234-1234-123456789012
Agent Name: AWS Agent
================================================================================

👤 USER: Find EC2 instance called Terminus

📤 MODEL REQUEST
Model: claude-3-5-haiku
System prompt: 1234 chars
User input: 32 chars

🔧 TOOL START: aws_find_account (2025-01-28 17:30:01)
Input: {"query":"production"}
✅ TOOL SUCCESS: aws_find_account (45ms)
Output: {"results":[{"account_id":"123456789012","name":"Production"}]}

🔧 TOOL START: aws_find_region (2025-01-28 17:30:01)
Input: {"query":"us-east"}
✅ TOOL SUCCESS: aws_find_region (12ms)
Output: {"region_code":"us-east-1","region_name":"US East (N. Virginia)"}

🔧 TOOL START: create_task (2025-01-28 17:30:01)
Creating sub-agent: task_abc123
Task: List EC2 instances named 'Terminus' in Production account us-east-1
Accounts: ["123456789012"]
Regions: ["us-east-1"]
✅ TOOL SUCCESS: create_task (1234ms)
Output: Task agent created

📥 MODEL RESPONSE (2025-01-28 17:30:03)
Status: Success
Duration: 1891ms
Output: I found the EC2 instance 'Terminus' in your Production account (123456789012)...
Tokens: input=456, output=123, total=579

⚡ ASSISTANT: I found the EC2 instance 'Terminus' in your Production account...
```

**Sub-agent log**: `~/.local/share/awsdash/logs/agents/agent-task_abc123.log`
```
================================================================================
🤖 SUB-AGENT SESSION STARTED: 2025-01-28 17:30:02 UTC
Agent ID: task_abc123
Agent Name: TaskAgent
Task: List EC2 instances named 'Terminus' in Production account us-east-1
================================================================================

🔧 TOOL START: aws_list_resources (2025-01-28 17:30:02)
Input: {"resource_type":"ec2:instance","account_id":"123456789012","region":"us-east-1"}
✅ TOOL SUCCESS: aws_list_resources (234ms)
Output: {"resources":[{"resource_id":"i-1234567890abcdef0","display_name":"Terminus"}]}

📥 MODEL RESPONSE (2025-01-28 17:30:02)
Status: Success
Duration: 456ms
Output: I found the EC2 instance 'Terminus' (i-1234567890abcdef0) running in us-east-1

✅ AGENT TERMINATED: Completed
```

---

## Key Takeaways

1. **Message Flow**: User → AgentInstance → Background Thread → Model → Tools → Callbacks → mpsc → AgentInstance → UI
2. **Storage Layers**:
   - In-memory: `VecDeque<Message>` in AgentInstance (limited to 100)
   - On-disk: Per-agent log files with full execution trace
3. **Message Passing**: All agent→UI communication via `mpsc::channel<AgentResponse>`
4. **Tool Callbacks**: `AgentToolCallbackHandler` converts `ToolEvent` to `AgentResponse` messages
5. **UI Tree**: Parent messages (tool starts) with nested children (tool results) for expandable display
6. **Sub-Agents**: Independent execution with own log files, results not streamed to parent UI

