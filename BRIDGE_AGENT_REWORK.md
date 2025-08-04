# Bridge Agent-on-Demand Architecture - Implementation Plan

## 📋 **Executive Summary**

Transform the Bridge Agent from a fixed-toolset agent into a **Task Orchestrator** that creates specialized agents on-demand with targeted AWS toolsets. This architecture provides better separation of concerns, resource management, and user experience.

---

## 🎯 **Project Goals**

### **Primary Objectives:**
- **Flexibility**: Dynamic agent creation with optimal tool selection per task
- **Resource Management**: Efficient agent lifecycle (create → execute → destroy → cleanup)
- **User Control**: Cancel button stops all active agents immediately
- **Task Tracking**: Proactive todo management following Claude Code patterns
- **Security**: AWS-specific security guidelines and defensive practices

### **Success Metrics:**
- Bridge Agent response time < 2s for agent creation
- Memory cleanup: 100% of ephemeral agents destroyed after task completion
- Cancellation latency: < 500ms from "Stop" button to agent termination
- User satisfaction: Clear task progress visibility

---

## 🏗️ **Architecture Overview**

### **Current State vs Target State:**

| Component | Current | Target |
|-----------|---------|---------|
| **Bridge Agent** | Do-everything agent with 5 fixed tools | Task orchestrator with 5 orchestration tools |
| **Tool Pattern** | Direct tool execution | Agent-as-tool pattern via `Create_Agent` |
| **Specialization** | One agent, multiple domains | Multiple agents, single domain each |
| **Lifecycle** | Persistent agent only | Persistent orchestrator + ephemeral specialists |
| **Cancellation** | No cancellation support | Full cancellation with "Stop" button |
| **Task Management** | Ad-hoc responses | Structured todo tracking |

### **Agent Hierarchy:**
```
┌─────────────────────────────────────────┐
│           Bridge Agent                  │
│        (Task Orchestrator)              │
│  ┌─────────────────────────────────┐    │
│  │  TodoWrite, TodoRead           │    │
│  │  Create_Agent                  │    │
│  │  aws_find_account/region       │    │
│  └─────────────────────────────────┘    │
└─────────────────┬───────────────────────┘
                  │ Creates on-demand:
                  ▼
    ┌─────────────────────────────────────┐
    │       Specialized Agents            │
    │  ┌─────────────┬─────────────────┐  │
    │  │aws-log-     │aws-resource-    │  │
    │  │analyzer     │auditor          │  │
    │  └─────────────┼─────────────────┤  │
    │  │aws-security-│aws-cost-        │  │
    │  │scanner      │optimizer        │  │
    │  └─────────────┴─────────────────┘  │
    └─────────────────────────────────────┘
```

---

## 📅 **Implementation Milestones**

## **MILESTONE 1: Core Infrastructure** (Week 1)
*Build the foundation for agent-on-demand architecture*

### **M1.1: Task Management Tools**
**Files to Create:**
- `src/app/bridge/tools/todo_write.rs`
- `src/app/bridge/tools/todo_read.rs`
- `src/app/bridge/tools/mod.rs` (update exports)

**Implementation Details:**
```rust
// todo_write.rs
pub struct TodoWriteTool {
    // In-memory task storage (upgrade to persistent later)
    tasks: Arc<Mutex<HashMap<String, Vec<TodoItem>>>>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TodoItem {
    id: String,
    content: String,
    status: TodoStatus, // pending, in_progress, completed
    priority: TodoPriority, // high, medium, low
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}
```

**Usage Pattern:**
```rust
// Bridge Agent automatically uses TodoWrite for complex tasks
let todos = vec![
    TodoItem::new("Identify AWS account", TodoPriority::High),
    TodoItem::new("Create log analyzer agent", TodoPriority::High),
    TodoItem::new("Analyze CloudWatch logs", TodoPriority::High),
];
```

### **M1.2: Create_Agent Tool Foundation**
**Files to Create:**
- `src/app/bridge/tools/create_agent.rs`
- `src/app/bridge/agent_types.rs`

**Implementation Details:**
```rust
// create_agent.rs
pub struct CreateAgentTool {
    active_agents: Arc<Mutex<HashMap<String, ActiveAgent>>>,
}

#[derive(Debug)]
pub struct ActiveAgent {
    agent_id: String,
    agent_type: AgentType,
    cancel_token: tokio_util::sync::CancellationToken,
    created_at: DateTime<Utc>,
    task_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    AwsLogAnalyzer,
    AwsResourceAuditor, 
    AwsSecurityScanner,
    AwsCostOptimizer, // Future
}
```

**IMPORTANT Security Requirements:**
```rust
// Parameters validation
struct CreateAgentParams {
    agent_type: AgentType,
    task_description: String,
    // REQUIRED AWS context
    account_id: String,     // REQUIRED - Never proceed without
    region: String,         // REQUIRED - Never proceed without  
    resource_identifier: String, // REQUIRED - Resource ID/name/ARN
}

// DO NOT allow agent creation without all required AWS context
impl CreateAgentTool {
    fn validate_aws_context(&self, params: &CreateAgentParams) -> Result<(), ToolError> {
        if params.account_id.is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "account_id is REQUIRED for AWS operations".to_string()
            });
        }
        // Similar validation for region and resource_identifier
    }
}
```

### **M1.3: Cancellation Infrastructure**
**Files to Modify:**
- `src/app/dashui/control_bridge_window.rs` (add Stop button + cancellation)

**Implementation Details:**
```rust
// In ControlBridgeWindow
pub struct ControlBridgeWindow {
    active_agent_tokens: Arc<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>>,
    main_agent_token: Option<tokio_util::sync::CancellationToken>,
    // ... existing fields
}

impl ControlBridgeWindow {
    // Called when "Stop" button clicked
    fn cancel_all_agents(&mut self) {
        // Cancel main Bridge Agent
        if let Some(token) = &self.main_agent_token {
            token.cancel();
        }
        
        // Cancel all active specialized agents
        let tokens = self.active_agent_tokens.lock().unwrap();
        for (agent_id, token) in tokens.iter() {
            info!("🛑 Cancelling agent: {}", agent_id);
            token.cancel();
        }
        
        // Clear all tokens
        drop(tokens);
        self.active_agent_tokens.lock().unwrap().clear();
        
        // Reset UI state
        self.processing_message = false;
        info!("🛑 All agents cancelled");
    }
}
```

**UI Changes:**
```rust
// Add Stop button to Bridge UI
if ui.button("🛑 Stop").clicked() && self.processing_message {
    self.cancel_all_agents();
}
```

---

## **MILESTONE 2: Specialized Agent Types** (Week 2)
*Create the specialized agents that replace current fixed tools*

### **M2.1: AWS Log Analyzer Agent**
**Files to Create:**
- `src/app/bridge/agents/aws_log_analyzer.rs`

**Purpose:** Replace current `aws_get_log_entries_tool` with proper agent pattern

**Implementation Details:**
```rust
pub struct AwsLogAnalyzerAgent;

impl AwsLogAnalyzerAgent {
    pub async fn create(
        task_description: String,
        aws_context: AwsContext,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<Agent, StoodError> {
        
        let system_prompt = r#"You are an AWS CloudWatch logs analysis specialist.

IMPORTANT: You MUST use the TodoWrite tool to track your progress through the 3-step CloudWatch process.

DO NOT proceed without account_id, region, and resource_identifier.

Your task workflow:
1. Use TodoWrite to plan: ["Find log groups", "Get log streams", "Retrieve log events", "Analyze patterns"]
2. aws_describe_log_groups - Find relevant log groups for the resource
3. aws_get_log_events - Retrieve actual log data with filtering
4. Analyze patterns and summarize findings
5. Mark todos complete as you progress

IMPORTANT: Never expose AWS credentials or keys in responses.

Available tools:
- TodoWrite: Track your analysis progress  
- TodoRead: Check current task status
- aws_describe_log_groups: Find CloudWatch log groups
- aws_get_log_events: Retrieve log events with filtering
- aws_find_account: Account lookup (no API calls)
- aws_find_region: Region lookup (no API calls)

Current task: {task_description}
AWS Context: Account={account_id}, Region={region}, Resource={resource_identifier}"#;

        let agent = Agent::builder()
            .system_prompt(system_prompt)
            .with_service_name("aws-log-analyzer-agent")
            .with_cancellation() // CRITICAL: Enable cancellation
            .with_credentials(
                aws_context.access_key,
                aws_context.secret_key, 
                aws_context.session_token,
                aws_context.region.clone(),
            )
            .tools(vec![
                todo_write_tool(),
                todo_read_tool(),
                aws_describe_log_groups_tool(None),
                aws_get_log_events_tool(None),
                aws_find_account_tool(),
                aws_find_region_tool(),
            ])
            .build()
            .await?;
            
        Ok(agent)
    }
}
```

### **M2.2: AWS Resource Auditor Agent**
**Files to Create:**
- `src/app/bridge/agents/aws_resource_auditor.rs`

**Purpose:** Handle resource inventory and compliance checking

**Implementation Details:**
```rust
pub struct AwsResourceAuditorAgent;

impl AwsResourceAuditorAgent {
    pub async fn create(
        task_description: String,
        aws_context: AwsContext,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<Agent, StoodError> {
        
        let system_prompt = r#"You are an AWS resource auditing specialist.

IMPORTANT: Use TodoWrite to break down complex auditing tasks.

DO NOT proceed without proper AWS context (account_id, region).

Your capabilities:
- Comprehensive resource inventory across AWS services
- Resource relationship mapping
- Compliance and tagging analysis
- Resource utilization assessment

Workflow:
1. TodoWrite to plan audit scope
2. aws_list_resources to discover resources
3. aws_describe_resource for detailed analysis
4. Generate structured audit reports

IMPORTANT: Never expose sensitive resource configurations unnecessarily.

Available tools:
- TodoWrite: Track audit progress
- TodoRead: Check audit status  
- aws_list_resources: Discover AWS resources
- aws_describe_resource: Get resource details
- aws_find_account: Account lookup
- aws_find_region: Region lookup

Task: {task_description}
AWS Context: Account={account_id}, Region={region}"#;

        // Similar agent creation pattern...
    }
}
```

### **M2.3: AWS Security Scanner Agent**
**Files to Create:**
- `src/app/bridge/agents/aws_security_scanner.rs`

**Purpose:** Security posture assessment and vulnerability scanning

**CRITICAL Security Implementation:**
```rust
let system_prompt = r#"You are an AWS security scanning specialist.

CRITICAL SECURITY RULES:
- NEVER log, display, or expose AWS credentials, keys, tokens, or secrets
- NEVER suggest actions that could compromise security
- NEVER bypass AWS security controls or policies
- DO NOT disable security features without explicit justification

IMPORTANT: This agent performs DEFENSIVE security analysis only.

You help with:
- Security group analysis (not modification)
- IAM policy review (not creation/modification)  
- Resource exposure assessment
- Security best practices recommendations

DO NOT:
- Create or modify IAM policies
- Change security group rules
- Disable security features
- Extract or expose sensitive data

Always use TodoWrite for security assessment planning."#;
```

---

## **MILESTONE 3: Bridge Integration** (Week 3)
*Update the main Bridge Agent to use the new architecture*

### **M3.1: Enhanced Bridge Agent System Prompt**
**Files to Modify:**
- `src/app/dashui/control_bridge_window.rs` (update system prompt)

**New System Prompt:**
```rust
let system_prompt = r#"You are the AWS Bridge Agent - a task orchestrator for AWS infrastructure management.

IMPORTANT: Always use TodoWrite to plan and track multi-step tasks. This is CRITICAL for user visibility.

DO NOT attempt complex AWS operations directly. Instead, create specialized agents via Create_Agent.

CRITICAL REQUIREMENTS for AWS operations:
- Account ID (use aws_find_account if user doesn't specify)
- Region (use aws_find_region if user doesn't specify)  
- Resource identifier (ID, name, or ARN)

NEVER proceed with AWS operations without these three pieces of information.

Available tools:
- Create_Agent: Launch specialized agents for complex AWS tasks
- TodoWrite: Track task progress (USE THIS PROACTIVELY)
- TodoRead: Query current task state
- aws_find_account: Search for AWS accounts (no API calls required)
- aws_find_region: Search for AWS regions (no API calls required)

Agent types you can create:
- aws-log-analyzer: CloudWatch logs analysis and troubleshooting
- aws-resource-auditor: Resource inventory and compliance checking  
- aws-security-scanner: Security posture assessment (DEFENSIVE ONLY)

Workflow for complex tasks:
1. TodoWrite to break down the task
2. Gather required AWS context (account, region, resource)
3. Create_Agent with appropriate specialist type
4. Monitor and report progress
5. Mark todos complete after specialist agent finishes

SECURITY RULES:
- REFUSE tasks that could compromise AWS security
- NEVER expose or log AWS credentials, keys, or sensitive data
- Focus on DEFENSIVE security practices only
- Follow AWS security best practices

Example interaction:
User: "Find errors in my Lambda function logs"
You: 
1. TodoWrite: ["Identify Lambda function", "Gather AWS context", "Create log analyzer", "Analyze logs"]
2. Ask for account/region if not provided
3. Create_Agent(type="aws-log-analyzer", task="Find Lambda errors", context={account, region, function_name})
4. Monitor specialist agent progress
5. Present results and mark todos complete

Be concise and direct. Minimize output while being helpful."#;
```

### **M3.2: Updated Bridge Agent Toolset**
**Implementation:**
```rust
// Replace current toolset
.tools(vec![
    create_agent_tool(),          // NEW: Agent orchestration
    todo_write_tool(),           // NEW: Task management
    todo_read_tool(),            // NEW: Task querying
    aws_find_account_tool(),     // KEEP: Account search (no API)
    aws_find_region_tool(),      // KEEP: Region search (no API)
    // REMOVE: aws_list_resources, aws_describe_resource, aws_get_log_entries
])
```

### **M3.3: Agent Lifecycle Management**
**Files to Modify:**
- `src/app/bridge/tools/create_agent.rs` (implement full lifecycle)

**Implementation Details:**
```rust
impl CreateAgentTool {
    async fn execute(&self, params: CreateAgentParams) -> Result<ToolResult, ToolError> {
        // 1. Validate AWS context
        self.validate_aws_context(&params)?;
        
        // 2. Create cancellation token
        let cancel_token = tokio_util::sync::CancellationToken::new();
        let agent_id = Uuid::new_v4().to_string();
        
        // 3. Store active agent info
        {
            let mut active_agents = self.active_agents.lock().unwrap();
            active_agents.insert(agent_id.clone(), ActiveAgent {
                agent_id: agent_id.clone(),
                agent_type: params.agent_type.clone(),
                cancel_token: cancel_token.clone(),
                created_at: Utc::now(),
                task_description: params.task_description.clone(),
            });
        }
        
        // 4. Create specialist agent
        let agent = match params.agent_type {
            AgentType::AwsLogAnalyzer => {
                AwsLogAnalyzerAgent::create(
                    params.task_description,
                    params.aws_context,
                    cancel_token.clone(),
                ).await?
            },
            AgentType::AwsResourceAuditor => {
                AwsResourceAuditorAgent::create(
                    params.task_description,
                    params.aws_context,
                    cancel_token.clone(),
                ).await?
            },
            // ... other agent types
        };
        
        // 5. Execute agent task
        let task_result = agent.execute(&params.task_description).await;
        
        // 6. Cleanup (CRITICAL)
        {
            let mut active_agents = self.active_agents.lock().unwrap();
            active_agents.remove(&agent_id);
        }
        
        // 7. Return results
        match task_result {
            Ok(result) => Ok(ToolResult::success(serde_json::json!({
                "agent_type": params.agent_type,
                "agent_id": agent_id,
                "task_completed": true,
                "result": result.response,
                "execution_time": result.duration,
            }))),
            Err(e) => Err(ToolError::ExecutionFailed {
                message: format!("Agent {} failed: {}", agent_id, e)
            })
        }
    }
}
```

---

## **MILESTONE 4: Testing & Polish** (Week 4)
*Comprehensive testing and user experience improvements*

### **M4.1: Cancellation Testing**
**Test Scenarios:**
1. **Mid-execution cancellation**: Stop button during log analysis
2. **Tool execution cancellation**: Stop during AWS API calls
3. **Multiple agent cancellation**: Stop with multiple active agents
4. **Memory leak testing**: Verify agent cleanup after cancellation

**Test Implementation:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_cancellation() {
        // Create agent with cancellation
        let agent = Agent::builder()
            .with_cancellation()
            .build().await.unwrap();
            
        let cancel_token = agent.cancellation_token().unwrap();
        
        // Start long-running task
        let task = tokio::spawn(async move {
            agent.execute("Analyze all logs in the account").await
        });
        
        // Cancel after 1 second
        tokio::time::sleep(Duration::from_secs(1)).await;
        cancel_token.cancel();
        
        // Verify task was cancelled
        let result = task.await.unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cancelled"));
    }
}
```

### **M4.2: UI/UX Improvements**
**Features to Add:**
- Active agent counter in Bridge window title
- Progress indicators for each active agent
- Agent execution time display
- Memory usage monitoring
- Agent creation/destruction notifications

**Implementation:**
```rust
// Bridge window title update
let active_count = self.active_agent_tokens.lock().unwrap().len();
let title = if active_count > 0 {
    format!("🚢 Control Bridge ({} active agents)", active_count)
} else {
    "🚢 Control Bridge".to_string()
};
```

### **M4.3: Error Handling & Recovery**
**Robust Error Scenarios:**
- AWS credential expiration during agent execution
- Network timeouts during AWS API calls
- Agent creation failures
- Cancellation during tool execution

**Error Recovery Patterns:**
```rust
impl CreateAgentTool {
    async fn handle_agent_failure(&self, agent_id: &str, error: &StoodError) -> ToolResult {
        // Log the failure
        error!("Agent {} failed: {}", agent_id, error);
        
        // Clean up active agent tracking
        {
            let mut active_agents = self.active_agents.lock().unwrap();
            active_agents.remove(agent_id);
        }
        
        // Return user-friendly error
        ToolResult::error(format!(
            "Task failed: {}. You can try again or rephrase your request.",
            error.user_friendly_message()
        ))
    }
}
```

---

## 🔒 **Security Implementation**

### **AWS Context Validation**
```rust
// NEVER allow operations without proper context
fn validate_aws_context(context: &AwsContext) -> Result<(), SecurityError> {
    // Account ID validation
    if context.account_id.is_empty() || context.account_id == "current" {
        return Err(SecurityError::MissingAccountId);
    }
    
    // Region validation  
    if context.region.is_empty() {
        return Err(SecurityError::MissingRegion);
    }
    
    // Resource identifier validation
    if context.resource_identifier.is_empty() {
        return Err(SecurityError::MissingResourceIdentifier);
    }
    
    Ok(())
}
```

### **Credential Protection**
```rust
// NEVER log or expose credentials
impl AwsContext {
    pub fn sanitized_for_logging(&self) -> AwsContextSanitized {
        AwsContextSanitized {
            account_id: self.account_id.clone(),
            region: self.region.clone(),
            resource_identifier: self.resource_identifier.clone(),
            // NEVER include access_key, secret_key, session_token
        }
    }
}
```

### **Defensive Security Practices**
- All agents operate in READ-ONLY mode by default
- Security scanner agent explicitly prohibits destructive operations
- All AWS API calls are logged for audit trail
- Resource access is limited to specified account/region/resource scope

---

## 📊 **Performance Considerations**

### **Memory Management**
- **Ephemeral agents**: Destroyed immediately after task completion
- **Token cleanup**: All cancellation tokens removed from tracking maps
- **Callback cleanup**: All callback handlers properly disposed

### **Concurrency Limits**
```rust
// Limit concurrent agents to prevent resource exhaustion
const MAX_CONCURRENT_AGENTS: usize = 3;

impl CreateAgentTool {
    fn check_concurrency_limit(&self) -> Result<(), ToolError> {
        let active_count = self.active_agents.lock().unwrap().len();
        if active_count >= MAX_CONCURRENT_AGENTS {
            return Err(ToolError::ResourceExhausted {
                message: format!(
                    "Maximum concurrent agents ({}) reached. Please wait for current tasks to complete.",
                    MAX_CONCURRENT_AGENTS
                )
            });
        }
        Ok(())
    }
}
```

### **Resource Monitoring**
- Track agent creation/destruction rates
- Monitor memory usage per agent type
- Alert on abnormal resource consumption patterns

---

## 🎯 **Success Criteria**

### **Functional Requirements**
- ✅ Bridge Agent can create 3 types of specialized agents
- ✅ "Stop" button cancels all active agents within 500ms  
- ✅ All ephemeral agents are properly destroyed after task completion
- ✅ TodoWrite integration provides clear task progress visibility
- ✅ AWS context validation prevents operations without required parameters

### **Performance Requirements**
- ✅ Agent creation time < 2 seconds
- ✅ Memory cleanup: 100% of ephemeral agents destroyed
- ✅ Concurrent agent limit: Max 3 active agents
- ✅ Cancellation latency < 500ms

### **Security Requirements**
- ✅ No AWS credentials logged or exposed in responses
- ✅ All operations require account_id, region, resource_identifier
- ✅ Security scanner operates in defensive mode only
- ✅ Proper audit trail for all AWS operations

### **User Experience Requirements**
- ✅ Clear task breakdown via TodoWrite integration
- ✅ Progress visibility for multi-step operations
- ✅ Immediate feedback on agent creation/destruction
- ✅ Intuitive "Stop" functionality that works reliably

---

## 📝 **Migration Strategy**

### **Phase 1: Parallel Implementation**
- Keep existing `aws_get_log_entries_tool` until replacement is tested
- Run new architecture alongside current implementation
- A/B test user experience improvements

### **Phase 2: Gradual Replacement**
- Replace `aws_get_log_entries_tool` with `Create_Agent(aws-log-analyzer)`
- Update Bridge Agent toolset
- Monitor for regressions

### **Phase 3: Full Deployment**
- Remove old tool implementations
- Update documentation and user guides
- Monitor system performance and user feedback

This implementation plan provides a comprehensive roadmap for transforming the Bridge Agent into a flexible, secure, and user-friendly task orchestration system.