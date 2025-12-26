# Task Manager Prompt - Orchestration Agent Instructions

## Component Overview

System prompt for TaskManager agents that orchestrate complex AWS operations
by spawning and coordinating TaskWorker sub-agents.

**Pattern**: Structured system prompt with capability definitions
**External**: None
**Purpose**: Define orchestration agent behavior

---

## Prompt Structure

### Role Definition
- Acts as orchestration coordinator
- Breaks complex tasks into sub-tasks
- Spawns TaskWorker agents for execution
- Aggregates and presents results

### Available Tools
- **think**: Planning and reasoning (no side effects)
- **start_task**: Spawn TaskWorker with specific task

### Behavioral Guidelines
- Analyze user request thoroughly
- Plan task decomposition before execution
- Spawn focused workers for each sub-task
- Wait for worker completion
- Synthesize results into coherent response

---

## Example Workflow

```
User: "Analyze security groups across all regions"

TaskManager thinks:
  1. Need to query security groups in each region
  2. Will spawn workers per region
  3. Aggregate findings

TaskManager spawns:
  - Worker 1: "List security groups in us-east-1"
  - Worker 2: "List security groups in us-west-2"
  - ...

TaskManager aggregates results and responds
```

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects prompts/task_manager.rs
