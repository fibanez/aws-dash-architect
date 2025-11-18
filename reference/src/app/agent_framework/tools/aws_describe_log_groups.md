# AWS Describe Log Groups Tool

## Component Overview

Lists CloudWatch log groups in specified account and region. Returns log group
metadata including names, retention policies, and sizes.

**Pattern**: Tool trait with global client fallback
**Algorithm**: AWS SDK cloudwatch_logs::describe_log_groups()
**External**: stood::tools::Tool, AWSResourceClient

---

## Major Methods

- `new()` - Create with optional AWSResourceClient
- `execute()` - List log groups with optional prefix filter

---

## Implementation Patterns

### Pattern: CloudWatch Logs Query

**Algorithm**: Paginated log group listing
**External**: AWS CloudWatch Logs SDK

Pseudocode:
  1. Parse input: account_id, region, optional log_group_prefix
  2. Create CloudWatch Logs client for account/region
  3. Call describe_log_groups() with prefix filter
  4. Paginate through results if needed
  5. Return list of log group metadata

---

## Tool Parameters

- account_id: String
- region: String
- log_group_prefix: Optional&lt;String&gt; (filter by prefix)

---

## External Dependencies

- **stood::tools::Tool**: Tool trait
- **AWSResourceClient**: CloudWatch Logs client
- **GLOBAL_AWS_CLIENT**: Fallback from tools_registry

---

**Last Updated**: 2025-01-28
