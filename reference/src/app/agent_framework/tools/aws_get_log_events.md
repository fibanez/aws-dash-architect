# AWS Get Log Events Tool

## Component Overview

Retrieves log events from CloudWatch log stream. Supports time-based filtering
and pagination for log analysis.

**Pattern**: Tool trait with global client fallback
**Algorithm**: AWS SDK cloudwatch_logs::get_log_events()
**External**: stood::tools::Tool, AWSResourceClient

---

## Major Methods

- `new()` - Create with optional AWSResourceClient
- `execute()` - Get log events from stream with time filters

---

## Implementation Patterns

### Pattern: CloudWatch Log Stream Query

**Algorithm**: Time-range filtered log retrieval
**External**: AWS CloudWatch Logs SDK

Pseudocode:
  1. Parse input: account_id, region, log_group, log_stream, start_time, end_time
  2. Create CloudWatch Logs client
  3. Call get_log_events() with time range
  4. Parse log events: timestamp, message
  5. Return formatted log entries

---

## Tool Parameters

- account_id: String
- region: String
- log_group_name: String
- log_stream_name: String
- start_time: Optional&lt;i64&gt; (Unix timestamp)
- end_time: Optional&lt;i64&gt; (Unix timestamp)

---

## External Dependencies

- **stood::tools::Tool**: Tool trait
- **AWSResourceClient**: CloudWatch Logs client
- **GLOBAL_AWS_CLIENT**: Fallback from tools_registry

---

**Last Updated**: 2025-01-28
