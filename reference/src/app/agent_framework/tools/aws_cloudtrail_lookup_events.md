# AWS CloudTrail Lookup Events Tool

## Component Overview

Searches CloudTrail audit events by various criteria. Supports filtering by
event name, resource, user, and time range for security and compliance analysis.

**Pattern**: Tool trait with global client fallback
**Algorithm**: AWS SDK cloudtrail::lookup_events()
**External**: stood::tools::Tool, AWSResourceClient

---

## Major Methods

- `new()` - Create with optional AWSResourceClient
- `execute()` - Search CloudTrail events with filters

---

## Implementation Patterns

### Pattern: CloudTrail Event Search

**Algorithm**: Multi-criteria CloudTrail query
**External**: AWS CloudTrail SDK

Pseudocode:
  1. Parse input: account_id, region, event_name, resource_name, username, start_time, end_time
  2. Create CloudTrail client
  3. Build lookup_events() query with attribute filters
  4. Paginate through results
  5. Return formatted audit events with metadata

---

## Tool Parameters

- account_id: String
- region: String
- event_name: Optional&lt;String&gt; (e.g., "DeleteBucket")
- resource_name: Optional&lt;String&gt; (ARN or name)
- username: Optional&lt;String&gt; (IAM principal)
- start_time: Optional&lt;i64&gt; (Unix timestamp)
- end_time: Optional&lt;i64&gt; (Unix timestamp)

---

## External Dependencies

- **stood::tools::Tool**: Tool trait
- **AWSResourceClient**: CloudTrail client
- **GLOBAL_AWS_CLIENT**: Fallback from tools_registry

---

**Last Updated**: 2025-01-28
