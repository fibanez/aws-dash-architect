# AWS Describe Resource Tool

## Component Overview

Gets detailed information about a specific AWS resource. Returns full resource
properties, configuration, and metadata.

**Pattern**: Tool trait with global client fallback
**Algorithm**: AWS SDK service-specific describe calls
**External**: stood::tools::Tool, AWSResourceClient

---

## Major Methods

- `new()` - Create with optional AWSResourceClient
- `execute()` - Describe resource by ID/ARN, account, region

---

## Implementation Patterns

### Pattern: Service-Specific Describe

**Algorithm**: Route to appropriate AWS service client
**External**: AWSResourceClient with 86+ service clients

Pseudocode:
  1. Parse input: resource_id/ARN, account_id, region
  2. Determine service from resource type or ARN
  3. Call service-specific describe (e.g., ec2::describe_instances)
  4. Normalize response to ResourceSummary format
  5. Return detailed properties as JSON

---

## Tool Parameters

- resource_id: String (resource ID or ARN)
- account_id: String
- region: String

---

## External Dependencies

- **stood::tools::Tool**: Tool trait
- **AWSResourceClient**: Multi-service client
- **GLOBAL_AWS_CLIENT**: Fallback from tools_registry

---

**Last Updated**: 2025-01-28
