# Agents Module - Agent Framework Task Agents

## Component Overview

Module exports for agent implementations. Provides two agent types:
- OrchestrationAgent: Main coordinator for AWS operations
- TaskAgent: Specialized agents for specific tasks

**Pattern**: Module organization with re-exports
**External**: stood::agent::Agent
**Purpose**: Agent type implementations for Agent Framework

---

## Module Structure

### Agent Implementations
- `mod.rs` - This file: agent module exports
- `orchestration_agent.rs` - OrchestrationAgent, AwsCredentials
- `task_agent.rs` - TaskAgent for specialized operations

---

## Public API Exports

```rust
pub use orchestration_agent::OrchestrationAgent;
pub use task_agent::TaskAgent;
```

---

## Agent Types

### OrchestrationAgent
Main orchestration agent that coordinates AWS infrastructure tasks:
- Receives natural language user requests
- Has access to create_task tool for spawning task agents
- Uses TodoWrite/TodoRead for task planning
- Uses aws_find_account/aws_find_region for context gathering
- Delegates complex operations to TaskAgent instances

### TaskAgent
Specialized agent for focused AWS operations:
- Created by OrchestrationAgent via create_task tool
- Has full AWS operation toolset (list, describe, logs, cloudtrail)
- Executes specific tasks with account/region context
- Reports results back through tool execution system

---

## Implementation Notes

### Agent Creation
Both agents are created using the OrchestrationAgent factory:
- OrchestrationAgent::create() - Creates main agent with credentials
- Called from AgentInstance::send_message() for lazy initialization
- Returns stood::Agent configured with tools and callbacks

### Tool Access
Different tool sets for each agent type:
- OrchestrationAgent: create_task, todo_write, todo_read, aws_find_account, aws_find_region
- TaskAgent: All AWS tools (list, describe, logs, cloudtrail) + todo tools

### Credential Management
Both agents use AwsCredentials struct:
- access_key_id: String
- secret_access_key: String
- session_token: Option<String>
- Obtained from AwsIdentityCenter before agent creation

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects agents/mod.rs structure
