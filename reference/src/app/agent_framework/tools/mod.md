# Tools Module - AWS Agent Framework Tools

## Component Overview

Module exports for Agent Framework tool implementations. Each tool provides
specific AWS operation capabilities to agents through the stood::tools::Tool trait.

**Pattern**: Module organization with re-exports
**External**: stood::tools::Tool, AWS SDK clients
**Purpose**: Tool implementations for agent-driven AWS operations

---

## Module Structure

### Tool Implementations
- `mod.rs` - This file: tool module exports
- `aws_list_resources.rs` - AwsListResourcesTool
- `aws_describe_resource.rs` - AwsDescribeResourceTool
- `aws_find_account.rs` - AwsFindAccountTool, AccountSearchResult
- `aws_find_region.rs` - AwsFindRegionTool, RegionSearchResult
- `aws_describe_log_groups.rs` - AwsDescribeLogGroupsTool
- `aws_get_log_events.rs` - AwsGetLogEventsTool
- `aws_cloudtrail_lookup_events.rs` - AwsCloudTrailLookupEventsTool
- `create_task.rs` - CreateTaskTool, ActiveTask
- `todo_read.rs` - TodoReadTool
- `todo_write.rs` - TodoWriteTool, TodoItem, TodoPriority, TodoStatus

---

## Public API Exports

```rust
pub use aws_cloudtrail_lookup_events::AwsCloudTrailLookupEventsTool;
pub use aws_describe_log_groups::AwsDescribeLogGroupsTool;
pub use aws_describe_resource::AwsDescribeResourceTool;
pub use aws_find_account::{set_global_aws_identity, AccountSearchResult, AwsFindAccountTool};
pub use aws_find_region::{AwsFindRegionTool, RegionSearchResult};
pub use aws_get_log_events::AwsGetLogEventsTool;
pub use aws_list_resources::AwsListResourcesTool;
pub use create_task::{ActiveTask, CreateTaskTool};
pub use todo_read::TodoReadTool;
pub use todo_write::{TodoItem, TodoPriority, TodoStatus, TodoWriteTool};
```

---

## Tool Categories

### AWS Resource Tools
Query and describe AWS resources across accounts/regions:
- **AwsListResourcesTool** - List resources by type, account, region
- **AwsDescribeResourceTool** - Get detailed resource information
- **AwsFindAccountTool** - Search available AWS accounts
- **AwsFindRegionTool** - Search AWS regions

### AWS Observability Tools
CloudWatch Logs and CloudTrail querying:
- **AwsDescribeLogGroupsTool** - List CloudWatch log groups
- **AwsGetLogEventsTool** - Retrieve log events from log streams
- **AwsCloudTrailLookupEventsTool** - Search CloudTrail audit events

### Agent Coordination Tools
Multi-agent orchestration and task tracking:
- **CreateTaskTool** - Spawn specialized task agents
- **TodoWriteTool** - Create/update shared TODO lists
- **TodoReadTool** - Query shared TODO items

---

## Tool Registration

Tools are registered with agents via constructor functions in `tools_registry.rs`:
- `aws_list_resources_tool()` → Box&lt;dyn Tool&gt;
- `aws_describe_resource_tool()` → Box&lt;dyn Tool&gt;
- `create_task_tool()` → Box&lt;dyn Tool&gt;
- etc.

Agents receive tools during stood::Agent builder configuration:
```rust
Agent::builder()
    .add_tool(aws_list_resources_tool(client))
    .add_tool(create_task_tool())
    .build()
```

---

## Implementation Notes

### Global State Access
Most tools access global state from `tools_registry.rs`:
- GLOBAL_AWS_CLIENT: For AWS SDK operations
- GLOBAL_TODO_STORAGE: For cross-agent TODO synchronization
- GLOBAL_CANCELLATION_MANAGER: For sub-agent cancellation

### Tool Parameter Patterns
Tools accept Optional&lt;Arc&lt;AWSResourceClient&gt;&gt;:
- If provided: use local client
- If None: fall back to get_global_aws_client()
- If both None: return user-friendly error

### Error Handling
Tools return ToolError with user-friendly messages:
- Explain what went wrong
- Suggest remediation steps
- Never expose credentials or internal implementation

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects tools/mod.rs structure
