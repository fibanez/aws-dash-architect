# StartTaskTool - Sub-Agent Spawning Tool

## Component Overview

Tool for spawning TaskWorker sub-agents from a TaskManager agent.
Enables multi-agent orchestration patterns.

**Pattern**: Agent spawning with result waiting
**External**: stood::tools::Tool trait
**Purpose**: Spawn focused worker agents for specific tasks

---

## Tool Definition

### Name
`start_task`

### Description
Spawns a new TaskWorker agent to execute a specific task.
Waits for worker completion and returns result.

### Parameters
- `task_description` (string, required): What the worker should do
- `context` (string, optional): Additional context for the worker

### Returns
Worker's response as string, or error message.

---

## Implementation

```rust
pub struct StartTaskTool {
    // No state needed
}

impl Tool for StartTaskTool {
    fn name(&self) -> String {
        "start_task".to_string()
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let task = params["task_description"].as_str()?;

        // Create TaskWorker agent
        let worker = AgentInstance::new_task_worker(task, parent_id);

        // Wait for completion
        let result = worker.wait_for_completion().await?;

        Ok(result)
    }
}
```

---

## Usage by TaskManager

```
TaskManager receives: "Check security across all regions"

TaskManager calls start_task:
{
  "task_description": "List security groups in us-east-1 and identify open ports",
  "context": "Focus on ports 22, 3389, and 0.0.0.0/0 rules"
}

Worker executes and returns findings.
TaskManager aggregates with other workers.
```

---

## Execution Flow

1. TaskManager invokes start_task tool
2. StartTaskTool creates new TaskWorker AgentInstance
3. TaskWorker receives task description as input
4. TaskWorker uses execute_javascript for AWS queries
5. TaskWorker completes with result
6. StartTaskTool returns result to TaskManager
7. TaskManager continues orchestration

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects tools/start_task.rs
