# Agent Performance Testing - Work Tracking

## Overview
Testing and fixing agent framework's resource query capabilities. The agent needs to query AWS resources (especially security groups) and analyze their properties to answer user questions about infrastructure.

---

## Milestone 1: Fix Resource Query Data Inconsistency ✅ COMPLETED

### Problem
Agent was stuck in infinite loop unable to retrieve resources:
- `loadCache()` reported loading 33 security groups successfully
- `queryCachedResources()` immediately after returned 0 resources
- Agent kept retrying different approaches without making progress

### Root Cause Analysis ✅
**Issue**: `loadCache()` and `queryCachedResources()` were using different data access patterns:
- `loadCache()` - Called `query_aws_resources_parallel()` (proper query)
- `queryCachedResources()` - Read from `state_guard.cached_queries` directly (stale cache)
- **Result**: Data inconsistency between the two functions

### Tasks Completed

#### Task 1.1: Unified Cache Access Pattern ✅
- **File**: `src/app/agent_framework/v8_bindings/bindings/resources.rs`
- **Changes**:
  - Removed temporary cache creation in `loadCache()`
  - Both functions now use Explorer's shared cache directly
  - Eliminated dual data paths
- **Commit**: `fix(agent): unify loadCache and queryCachedResources to use same query infrastructure`

#### Task 1.2: Fixed Race Condition ✅
- **Problem**: `loadCache()` returned before spawned query task completed
- **Solution**: Added `query_handle.await` to wait for completion before syncing cache
- **Result**: Cache sync is now atomic and completes before function returns

#### Task 1.3: Atomic Cache Synchronization ✅
- **Pattern**: Both functions now follow Explorer's atomic sync pattern:
  1. Query completes fully
  2. Read final cache state
  3. Atomically replace explorer_state.cached_queries
  4. Only then return to caller

---

## Milestone 2: Align with Explorer's Query Pattern ✅ COMPLETED

### Tasks Completed

#### Task 2.1: Extract Black-Box Query Function ✅
- **Created**: `execute_complete_query()` - encapsulates Explorer's complete workflow
- **Pattern**: Transparent black-box API hiding Phase 1, Phase 2, caching internals
- **Location**: `resources.rs:448-715`
- **Documentation**: Comprehensive comments explaining architecture and integration points

#### Task 2.2: Refactor loadCache() to Use Shared Function ✅
- **Changes**: Thin wrapper calling `execute_complete_query()`
- **Returns**: Summary counts (not full resources)
- **Location**: `resources.rs:717-794`

#### Task 2.3: Refactor queryCachedResources() to Use Shared Function ✅
- **Changes**: Thin wrapper calling same `execute_complete_query()`
- **Returns**: Full resource objects with merged properties
- **Location**: `resources.rs:1224-1371`

#### Task 2.4: Code Cleanup ✅
- Removed duplicate query logic
- Eliminated temporary cache patterns
- Both functions now guarantee consistent data

---

## Milestone 3: Fix Explorer Integration Bugs 📋 TODO

### Bug 3.1: Explorer Window Not Visible When Opened by Agent
- **Issue**: When agent calls `showInExplorer()`, window opens but may not be visible
- **Required**: Window must become visible/focused when opened by agent
- **File**: `src/app/resource_explorer/window.rs`

### Bug 3.2: Explorer Window Needs Pre-Selected Scope
- **Issue**: When agent opens Explorer, all dropdowns are empty
- **Required**: All accounts, regions, and resource types from the query should be pre-selected
- **Context**: Agent calls showInExplorer with specific scope - UI should reflect that
- **Files**:
  - `src/app/agent_framework/v8_bindings/bindings/resources.rs` (showInExplorer)
  - `src/app/resource_explorer/window.rs` (apply_ephemeral_config)

---

## Milestone 4: Fix Schema Display for Nested Objects 📋 TODO

### Task 4.1: IpPermissions Shows Incomplete Data
- **Issue**: `getResourceSchema()` returns simplified IpPermissions:
  ```json
  "IpPermissions": [{"IpProtocol": "-1"}]
  ```
- **Missing fields needed for SSH analysis**:
  - `IpRanges` (contains CIDR blocks like "0.0.0.0/0")
  - `FromPort` / `ToPort` (to check for port 22)
  - `Ipv6Ranges`
  - `UserIdGroupPairs`
- **Required**: Show FULL nested structure from Phase 1 or Phase 2 data
- **File**: `src/app/agent_framework/v8_bindings/bindings/resources.rs` (get_resource_schema_internal)

### Task 4.2: Ensure Phase 2 Enrichment Data is Included
- **Issue**: Security groups need Phase 2 enrichment for full rule details
- **Verify**: Both `getResourceSchema()` and `queryCachedResources()` merge:
  1. properties (normalized minimal)
  2. raw_properties (Phase 1 List API)
  3. detailed_properties (Phase 2 Describe API)
- **Current**: Merge logic exists but needs testing with real data

### Task 4.3: Implement Schema Merging from Multiple Samples 📋 TODO
- **Goal**: Merge up to 1000 cached samples to create comprehensive schema
- **Rationale**: Different resources have different configurations:
  - Some security groups have rules, others don't
  - Different regions/accounts have varying field populations
  - Single sample misses optional fields and nested structures
- **Implementation**:
  1. Sample up to 1000 resources of requested type from cache
  2. Deep merge all JSON properties (properties + raw_properties + detailed_properties)
  3. Handle nested arrays by merging all elements to show complete structure
  4. Replace values with type placeholders: `"<string>"`, `"<integer>"`, `"<boolean>"`
  5. Add metadata: `"_schema_metadata": { "samples_merged": N, "note": "..." }`
- **Performance**: In-memory JSON merge, expected <10ms even for 1000 samples
- **Benefits**:
  - Agent sees ALL possible fields including rare/optional ones
  - Nested arrays (IpPermissions, Tags) show full structure
  - Covers regional/configuration variations
- **Fallback**: If cache empty, returns error (agent workflow requires loadCache first)
- **File**: `src/app/agent_framework/v8_bindings/bindings/resources.rs` (get_resource_schema_internal)

---

## Milestone 5: End-to-End Agent Testing 📋 TODO

### Test 5.1: Security Group SSH Analysis
- **Goal**: Agent successfully identifies security groups with port 22 open
- **Test query**: "Which security groups allow SSH (port 22) from 0.0.0.0/0?"
- **Expected**: Agent can:
  1. Load security groups with `loadCache()`
  2. Query them with `queryCachedResources()`
  3. Analyze IpPermissions rules
  4. Identify groups with port 22 open to public

### Test 5.2: Complex Multi-Resource Query
- **Goal**: Test agent with multiple resource types
- **Test query**: "Show me all S3 buckets and their CloudFront distributions"
- **Verifies**: Shared query function works for multiple resource types

### Test 5.3: Explorer Visualization
- **Goal**: Agent opens Explorer window with query results
- **Test**: Call `showInExplorer()` and verify:
  - Window becomes visible
  - All accounts/regions/types are selected
  - Resources are displayed in tree
  - Agent can see the visualization

---

## Milestone 6: Agent JavaScript Efficiency Improvements ✅ COMPLETED

### Problem Identified
Analysis of agent logs (Nova 2 Pro and Claude Sonnet 4.5) revealed efficiency issues:
- Context window pollution: Nova returned 68 full bucket objects instead of counts
- Missing showInExplorer() usage: Neither model used UI for >10 results
- IIFE return confusion: Both models struggled with explicit return statements
- Redundant executions: Nova ran 5 JavaScript calls instead of 2
- Unclear workflow: Models didn't understand two-execution pattern

### Tasks Completed

#### Task 6.1: Enhance Worker Prompt with JavaScript Efficiency Rules ✅
- **File**: `src/app/agent_framework/prompts/task_worker.rs`
- **Added**: Section "7. JavaScript-First Efficiency (CRITICAL)"
  - Rule 7.1: NEVER Return Full Arrays, ALWAYS Return Summaries
  - Rule 7.2: Use showInExplorer() for Results > 10 Items (MANDATORY)
  - Rule 7.3: IIFE Pattern with Explicit Return Statement
  - Rule 7.4: Console.log for Debugging, Return for Results
  - Rule 7.5: All Processing in JavaScript
- **Examples**: BAD vs GOOD patterns with context window impact explained

#### Task 6.2: Clarify Two-Execution Pattern ✅
- **Files**: `task_worker.rs` and `execute_javascript.rs`
- **Added**: "Resource Query Workflow (CRITICAL - TWO EXECUTION PATTERN)"
  - EXECUTION 1: Load Cache + Get Schema (returns metadata to LLM)
  - Between executions: LLM analyzes schema structure
  - EXECUTION 2: Query + Filter (write code using discovered properties)
- **Rationale**: Prevents guessing at property names, reduces errors
- **Example output**: Shows what LLM sees after Execution 1

#### Task 6.3: Enhance Tool JSON with Efficiency Examples ✅
- **File**: `src/app/agent_framework/tools/execute_javascript.rs`
- **Updated**: `parameters_schema()` examples array
  - EXECUTION 1 example: Returns `{ loaded, schema }`
  - EXECUTION 2 example: Filters using discovered properties
  - showInExplorer() conditional pattern for >10 results
  - Context-efficient aggregation with `.reduce()`
- **Description**: Added CRITICAL warning about explicit return in IIFE

#### Task 6.4: Update Tool Description with Two-Execution Workflow ✅
- **File**: `execute_javascript.rs` description field
- **Added**: "TWO EXECUTION PATTERN" section with:
  - Detailed EXECUTION 1 code example
  - "After Execution 1" explanation of what LLM sees
  - Detailed EXECUTION 2 code example
  - "WHY TWO EXECUTIONS?" rationale section
- **Impact**: Makes workflow explicit, not implicit

### Expected Impact
- Context window usage: 80% reduction (800 lines → <100 lines)
- showInExplorer() usage: 0% → 80% for >10 results
- IIFE return issues: 30-60% fail rate → <10% fail rate
- JavaScript executions: 5 → 2 (standardized two-execution pattern)
- Data duplication: Eliminated (return counts, not arrays)

---

## Technical Debt & Improvements 📋 BACKLOG

### Code Quality
- [ ] Add unit tests for `query_resources_shared()`
- [ ] Add integration test: loadCache → queryCachedResources consistency
- [ ] Document the wrapper pattern in code comments
- [ ] Add performance timing instrumentation

### Performance
- [ ] Profile query execution time
- [ ] Optimize cache cloning (currently clones entire cache)
- [ ] Consider incremental cache updates instead of full replacement

### Documentation
- [ ] Update `docs/technical/` with agent resource query pattern
- [ ] Document the showInExplorer integration
- [ ] Add examples of proper agent resource queries

---

## Key Architectural Decisions

### Pattern: Thin Wrappers Around Explorer Engine
**Decision**: Both `loadCache()` and `queryCachedResources()` are thin wrappers that:
1. Call the same shared query function
2. Let Explorer's query engine handle caching transparently (cache hit vs miss)
3. Differ only in return format (summary vs full resources)

**Rationale**:
- Single source of truth for query logic
- Explorer's caching strategy applies transparently
- No need to duplicate cache management logic

### Pattern: Atomic Cache Synchronization
**Decision**: Use Explorer's atomic sync pattern:
1. Query completes fully in cloned cache
2. Sync entire final cache to explorer state in one write
3. Never partial/incremental updates during query

**Rationale**:
- Prevents race conditions
- Ensures consistency between loadCache and queryCachedResources
- Matches Explorer's proven pattern

### Pattern: Follow Explorer's Resource Collection
**Decision**: Use `Arc<Mutex<Vec<ResourceEntry>>>` to collect resources during streaming results

**Rationale**:
- Matches Explorer's proven implementation
- Handles concurrent result processing
- Natural fit with tokio::join! pattern

---

## Files Modified

### Primary Files
- `src/app/agent_framework/v8_bindings/bindings/resources.rs`
  - Functions: `load_cache_internal`, `query_cached_resources_internal`, `query_resources_shared` (new)
  - Lines: ~450-1400

### Reference Files (for pattern matching)
- `src/app/resource_explorer/window.rs`
  - Functions: Explorer's query pattern (lines 3340-3434)
  - Function: `refresh_resources_from_cache_filtered` (line 3774)

---

## Session Notes

### Session 1: Initial Diagnosis
- Identified data inconsistency between loadCache and queryCachedResources
- Reviewed agent logs showing 33 resources loaded but 0 returned
- Traced through code to find different data paths

### Session 2: First Fix Attempt
- Unified both functions to use Explorer's cache
- Fixed race condition with query completion
- Committed changes

### Session 3: Deeper Investigation (Current)
- Discovered queryCachedResources still returns 0 despite fixes
- Analyzed Explorer's exact query pattern
- Started refactoring to shared query function
- **Status**: In middle of extracting shared function - need to complete refactoring

---

## Next Session Action Items

### Immediate (Resume Work)
1. ✅ Complete `query_resources_shared()` extraction
2. ⏳ Refactor `load_cache_internal()` to call shared function
3. ⏳ Refactor `query_cached_resources_internal()` to call shared function
4. ⏳ Test both functions return consistent data
5. ⏳ Verify agent can now query security groups successfully

### Follow-Up
6. Fix Explorer window visibility bug
7. Fix Explorer pre-selection bug
8. Fix getResourceSchema nested object display
9. Run end-to-end agent test with SSH analysis

---

## Success Criteria

### Milestone 2 Success
- [ ] `loadCache()` returns summary with totalCount: 33
- [ ] `queryCachedResources()` returns 33 full resources
- [ ] Both called with identical params return consistent data
- [ ] Agent can access and analyze resource properties

### Milestone 3 Success
- [ ] `showInExplorer()` makes window visible
- [ ] Explorer shows pre-selected accounts/regions/types from agent query
- [ ] Agent can verify visualization opened correctly

### Milestone 4 Success
- [ ] `getResourceSchema()` shows full nested IpPermissions structure
- [ ] All Phase 2 enrichment data is visible to agent
- [ ] Agent can analyze security group rules properly

### Final Success
- [ ] Agent successfully completes: "Which security groups allow SSH from 0.0.0.0/0?"
- [ ] Agent can open Explorer to show results visually
- [ ] No false positives or missed resources
