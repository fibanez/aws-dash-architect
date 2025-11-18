# AWS List Resources Tool

## Component Overview

Lists AWS resources by type, account, and region. Queries AWS Resource Groups
Tagging API to discover resources across services.

**Pattern**: Tool trait implementation with global client fallback
**Algorithm**: AWS SDK resource_groups_tagging_api::get_resources()
**External**: stood::tools::Tool, AWSResourceClient

---

## Major Methods

- `new()` - Create with optional AWSResourceClient
- `execute()` - List resources with filters (type, account, region)

---

## Implementation Patterns

### Pattern: Global Client Fallback

**Algorithm**: Local OR global AWS client with error handling
**External**: get_global_aws_client() from tools_registry

Pseudocode:
  1. Tool stores Option<Arc<AWSResourceClient>>
  2. On execute():
     - Try local client if Some
     - If None, call get_global_aws_client()
     - If both None, return ToolError with user guidance
  3. Enables standalone and integrated modes

### Pattern: Resource Type Filtering

**Algorithm**: AWS resource type string (e.g., "ec2:instance")
**External**: AWS Resource Groups Tagging API

Pseudocode:
  1. Parse input: resource_type, account_id, region
  2. Call AWS get_resources() with filters
  3. Return list of ResourceSummary structs
  4. Each includes: type, account, region, ID, name, status, tags

---

## Tool Parameters

- resource_type: String (e.g., "s3", "ec2:instance", "lambda")
- account_id: String (AWS account ID)
- region: String (e.g., "us-east-1")

---

## External Dependencies

- **stood::tools::Tool**: Tool trait
- **AWSResourceClient**: Multi-account SDK wrapper
- **GLOBAL_AWS_CLIENT**: Fallback client from tools_registry

---

**Last Updated**: 2025-01-28
