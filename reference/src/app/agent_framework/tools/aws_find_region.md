# AWS Find Region Tool

## Component Overview

Searches AWS regions by name or code. Returns region metadata including code,
name, and availability status.

**Pattern**: Tool trait with static region list
**Algorithm**: Search hardcoded AWS region list
**External**: stood::tools::Tool

---

## Major Methods

- `new_uninitialized()` - Create tool without client
- `execute()` - Search regions by query string

---

## Implementation Patterns

### Pattern: Static Region List Search

**Algorithm**: Fuzzy search on hardcoded region data
**External**: None (static data)

Pseudocode:
  1. Parse input: query (region name or code, e.g., "virginia" or "us-east")
  2. Search static list of AWS regions
  3. Fuzzy match on region code and full name
  4. Return Vec<RegionSearchResult>
  5. Each includes: region_code, region_name, available

### Pattern: No API Calls Required

**Algorithm**: Local search, no network
**External**: None

Pseudocode:
  1. Static region data: us-east-1 → "US East (N. Virginia)"
  2. Tool searches locally (instant, no latency)
  3. Used by orchestration agent to resolve region names
  4. Example: "virginia" → "us-east-1"

---

## Tool Parameters

- query: String (region name or code to search)

---

## External Dependencies

- **stood::tools::Tool**: Tool trait

---

**Last Updated**: 2025-01-28
