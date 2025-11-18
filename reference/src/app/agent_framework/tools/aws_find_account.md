# AWS Find Account Tool

## Component Overview

Searches available AWS accounts by name or partial ID. Returns account metadata
including ID, name, email, and role information from Identity Center.

**Pattern**: Tool trait with global AWS Identity access
**Algorithm**: Search cached account list from set_global_aws_identity()
**External**: stood::tools::Tool, AwsIdentityCenter

---

## Major Methods

- `new_uninitialized()` - Create tool without client
- `execute()` - Search accounts by query string

---

## Implementation Patterns

### Pattern: Global AWS Identity Search

**Algorithm**: Query pre-loaded account list
**External**: set_global_aws_identity() stores identity

Pseudocode:
  1. Parse input: query (account name or partial ID)
  2. Access global AWS Identity Center
  3. Search accounts by fuzzy match on name/ID
  4. Return Vec<AccountSearchResult>
  5. Each includes: account_id, name, email, role_name

### Pattern: No API Calls Required

**Algorithm**: Search cached data from Identity Center login
**External**: None (local search)

Pseudocode:
  1. Identity Center login caches account list
  2. Tool searches cached list (fast, no network)
  3. Enables account discovery without AWS API calls
  4. Used by orchestration agent to resolve account names

---

## Tool Parameters

- query: String (account name or partial ID to search)

---

## External Dependencies

- **stood::tools::Tool**: Tool trait
- **AwsIdentityCenter**: Account metadata cache
- **set_global_aws_identity()**: Global identity storage

---

**Last Updated**: 2025-01-28
