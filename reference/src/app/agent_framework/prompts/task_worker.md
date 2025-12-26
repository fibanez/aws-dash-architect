# Task Worker Prompt - Execution Agent Instructions

## Component Overview

System prompt for TaskWorker agents that execute specific AWS operations
using JavaScript with AWS API bindings.

**Pattern**: Structured system prompt with execution guidelines
**External**: V8 JavaScript engine
**Purpose**: Define worker agent behavior

---

## Prompt Structure

### Role Definition
- Executes specific, focused tasks
- Uses JavaScript for AWS API access
- Returns structured results
- Single-purpose, efficient execution

### Available Tools
- **execute_javascript**: Run JS with AWS API bindings

### JavaScript API Bindings
Available in execute_javascript:
- `listAccounts()` - Get available AWS accounts
- `listRegions()` - Get available AWS regions
- `queryResources(type, account, region)` - Query AWS resources
- `queryCloudWatchLogEvents(...)` - Get CloudWatch logs
- `getCloudTrailEvents(...)` - Get CloudTrail events

### Behavioral Guidelines
- Focus on assigned task only
- Use appropriate AWS API bindings
- Return structured JSON results
- Handle errors gracefully
- Be concise in responses

---

## Example Execution

```
Task: "List EC2 instances in us-east-1"

Worker executes JavaScript:
  const instances = await queryResources(
    'AWS::EC2::Instance',
    'account-id',
    'us-east-1'
  );
  return JSON.stringify(instances);

Worker returns structured result to parent
```

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects prompts/task_worker.rs
