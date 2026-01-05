# Merge Plan: agent-performance-timings → bug-iam-queries

## Executive Summary

Merging `agent-performance-timings` into `bug-iam-queries` with priority on:
- **Explorer changes**: Keep from `bug-iam-queries` branch
- **Agent Framework changes**: Keep from `agent-performance-timings` branch
- **Overlapping changes**: Carefully merge both

**Conflict Summary:**
- 2 files with actual conflicts: `resources.rs`, `window.rs`
- 7 files auto-merged successfully: `CLAUDE.md`, `Cargo.toml`, `mod.rs`, `rendering.rs`, `window_rendering.rs`, `resource_explorer/mod.rs`, `resource_explorer/window.rs` (partial)
- 13 new files from `agent-performance-timings` (agent framework improvements)
- 10 new files from `bug-iam-queries` (explorer improvements)

---

## Branch Comparison

### bug-iam-queries (23 commits) - Explorer Focus
**Main Theme:** Resource Explorer query timing, caching, error handling, and UI improvements

**Key New Files:**
- `src/app/resource_explorer/cache.rs` - Moka cache with compression (635 lines)
- `src/app/resource_explorer/query_timing.rs` - Query timing instrumentation (1085 lines)
- `src/app/resource_explorer/retry_tracker.rs` - Failed query tracking (373 lines)
- `src/app/resource_explorer/sdk_errors.rs` - SDK error categorization (337 lines)
- `src/app/resource_explorer/instances/` - Multi-pane architecture (1387 lines total)
- `TODOS/FAILED_QUERIES_ERROR_CATEGORIES.md` - Error categorization docs (292 lines)

**Key Modifications:**
- `window.rs` - Extensive explorer UI changes (1832 lines total, heavily refactored)
- `aws_client.rs` - Query infrastructure improvements (510 lines, +196)
- `state.rs` - Resource state management (154 lines, +27)
- `credentials.rs` - Credential handling (103 lines, +21)
- `resource_tagging.rs` - Tag fetching with timeouts (158 lines, +76)

**Dependencies Added:**
- `moka = "0.12"` - High-performance caching
- `zstd = "0.13"` - Compression
- `bincode = "1.3"` - Binary serialization
- `sysinfo = "0.31"` - System monitoring

### agent-performance-timings (14 commits) - Agent Framework Focus
**Main Theme:** Agent framework performance optimization, context-optimized queries, Nova 2 models

**Key New Files:**
- `src/app/agent_framework/perf_timing.rs` - Performance timing (396 lines)
- `src/app/agent_framework/telemetry_init.rs` - CloudWatch telemetry (165 lines)
- `src/app/aws_identity.rs` - New AWS identity module (282 lines)
- `src/app/dashui/window_maximize.rs` - Soft-maximize UI (229 lines)
- `docs/technical/stood-perf-timing-plan.md` - Performance docs (573 lines)
- `TODO/AGENT-PERFORMANCE-TESTING.md` - Performance testing guide (345 lines)

**Key Modifications:**
- `resources.rs` - Complete rewrite with context-optimized API (1951 lines, +602)
- `execute_javascript.rs` - Two-execution pattern (255 lines, +114)
- `task_worker.rs` - Updated prompts for Nova 2 (395 lines, +232)
- `agent_instance.rs` - Performance integration (155 lines, +54)
- `agent_manager_window.rs` - Soft-maximize feature (386 lines, +248)
- `rendering.rs` - CloudWatch log group creation (113 lines, +106)

**Dependencies Changed:**
- `stood` → `stood-ai-agent` with `perf-timing` feature

---

## Detailed Conflict Analysis

### 1. resources.rs - CRITICAL FILE (3 conflict regions)

**File Statistics:**
- Base version: 1349 lines
- bug-iam-queries: 1326 lines (-23 lines) - Minor cache integration changes
- agent-performance-timings: 1951 lines (+602 lines) - MAJOR REWRITE

**Conflict #1 (Line 15): Import statements**
```rust
<<<<<<< HEAD
// bug-iam-queries: Removed HashMap (not needed)
=======
// agent-performance-timings: Added serde_json::json, kept HashMap
use serde_json::json;
use std::collections::HashMap;
>>>>>>>
```
**Resolution Strategy:** Accept agent-performance-timings version (needs both imports)

**Conflict #2 (Lines 1286-1323): Cache initialization vs property merging**
```rust
<<<<<<< HEAD
// bug-iam-queries: Switch to shared Moka cache
let explorer_state = get_global_explorer_state();
let shared_cache = crate::app::resource_explorer::cache::shared_cache();
=======
// agent-performance-timings: Property merging logic for JavaScript
for entry in &all_resources {
    let mut merged_properties = serde_json::Map::new();
    // ... merge properties, raw_properties, detailed_properties
}
>>>>>>>
```
**Resolution Strategy:** This is complex - the code is in completely different contexts
- agent-performance-timings version is in a different function (context-optimized API)
- bug-iam-queries version is a small cache refactor
- **Solution:** Accept agent-performance-timings version entirely, then apply shared cache integration as a follow-up change

**Conflict #3 (Lines 1337-1379): Query execution**
```rust
<<<<<<< HEAD
// bug-iam-queries: Parallel query with shared cache
tokio::spawn(async move {
    client_clone.query_aws_resources_parallel(
        &scope_for_query,
        result_tx,
        None,
        cache_clone, // Uses shared Moka cache
    ).await
});
=======
// agent-performance-timings: Different query flow (context-optimized)
// ... tag serialization, different structure
>>>>>>>
```
**Resolution Strategy:** Accept agent-performance-timings version, then integrate shared cache

**Overall Strategy for resources.rs:**
1. Accept agent-performance-timings version entirely (black-box API is a major improvement)
2. After merge, apply shared cache integration from bug-iam-queries as a follow-up commit
3. This preserves the context-optimized API while gaining the caching improvements

---

### 2. window.rs - UI Logic (1 conflict region)

**Conflict (Lines 311-323): Frame initialization**
```rust
<<<<<<< HEAD
// bug-iam-queries: Reset minimize flag
self.minimize_requested = false;
=======
// agent-performance-timings: Poll V8 JavaScript action queue
let v8_actions = super::drain_explorer_actions();
for action in v8_actions {
    match action {
        super::ExplorerAction::OpenWithConfig(config) => {
            self.apply_ephemeral_config(config);
        }
    }
}
>>>>>>>
```
**Resolution Strategy:** Keep BOTH - they're independent features
1. Reset minimize flag (from bug-iam-queries)
2. Poll V8 action queue (from agent-performance-timings)

---

## Auto-Merged Files (Needs Verification)

These files merged without conflicts but need careful review:

### 1. CLAUDE.md - Documentation (Auto-merged)
- bug-iam-queries: Added "Query Timing Log" section
- agent-performance-timings: Added "Agent Performance Timing Log" section
- **Verification:** Ensure both sections are present and not duplicated

### 2. Cargo.toml - Dependencies (Auto-merged)
- bug-iam-queries: Added `moka`, `zstd`, `bincode`, `sysinfo`
- agent-performance-timings: Changed `stood` to `stood-ai-agent` with `perf-timing`
- **Verification:** Ensure all dependencies are present

### 3. src/app/dashui/app/mod.rs (Auto-merged)
- bug-iam-queries: Added explorer instance management
- agent-performance-timings: Added CloudWatch telemetry
- **Verification:** Check that both features are integrated correctly

### 4. src/app/dashui/app/rendering.rs (Auto-merged)
- bug-iam-queries: Minor explorer rendering changes
- agent-performance-timings: Added CloudWatch log group pre-creation
- **Verification:** Ensure rendering logic is coherent

### 5. src/app/dashui/app/window_rendering.rs (Auto-merged)
- bug-iam-queries: Explorer window rendering updates
- agent-performance-timings: Agent manager window updates
- **Verification:** Check window rendering logic

### 6. src/app/resource_explorer/mod.rs (Auto-merged)
- bug-iam-queries: Added new modules (cache, query_timing, retry_tracker, sdk_errors, instances)
- agent-performance-timings: Added exports for agent integration
- **Verification:** Ensure all modules are properly exported

### 7. src/app/resource_explorer/window.rs (Mostly auto-merged, 1 conflict)
- Both branches made extensive changes
- **Verification:** After resolving conflict, check that all features work together

---

## New Files (No Conflicts)

### From bug-iam-queries (Explorer Focus)
- ✓ `src/app/resource_explorer/cache.rs`
- ✓ `src/app/resource_explorer/query_timing.rs`
- ✓ `src/app/resource_explorer/retry_tracker.rs`
- ✓ `src/app/resource_explorer/sdk_errors.rs`
- ✓ `src/app/resource_explorer/instances/instance.rs`
- ✓ `src/app/resource_explorer/instances/manager.rs`
- ✓ `src/app/resource_explorer/instances/mod.rs`
- ✓ `src/app/resource_explorer/instances/pane.rs`
- ✓ `src/app/resource_explorer/instances/pane_renderer.rs`
- ✓ `src/app/resource_explorer/instances/tab.rs`
- ✓ `TODOS/FAILED_QUERIES_ERROR_CATEGORIES.md`

### From agent-performance-timings (Agent Focus)
- ✓ `src/app/agent_framework/perf_timing.rs`
- ✓ `src/app/agent_framework/telemetry_init.rs`
- ✓ `src/app/aws_identity.rs`
- ✓ `src/app/dashui/window_maximize.rs`
- ✓ `docs/technical/stood-perf-timing-plan.md`
- ✓ `TODO/AGENT-PERFORMANCE-TESTING.md`

---

## Merge Execution Plan

### Phase 1: Execute Merge
1. **Start merge**: `git merge --no-ff agent-performance-timings`
2. **Resolve conflicts** (2 files):
   - `resources.rs`: Accept agent-performance-timings version entirely
   - `window.rs`: Combine both changes

### Phase 2: Conflict Resolution

#### resources.rs Resolution
```bash
# Accept their version (agent-performance-timings)
git checkout --theirs src/app/agent_framework/v8_bindings/bindings/resources.rs
git add src/app/agent_framework/v8_bindings/bindings/resources.rs
```
**Note:** This accepts the context-optimized API completely. We'll integrate shared cache in a follow-up commit.

#### window.rs Resolution
**Manual merge required** - Combine both features:
1. Keep minimize flag reset (bug-iam-queries)
2. Add V8 action queue polling (agent-performance-timings)

Resulting code:
```rust
pub fn show(&mut self, ctx: &Context) -> WindowAction {
    if !self.is_open {
        return WindowAction::None;
    }

    // Reset minimize_requested flag at the start of each frame
    self.minimize_requested = false;

    // Poll V8 JavaScript action queue for showInExplorer() calls
    let v8_actions = super::drain_explorer_actions();
    for action in v8_actions {
        match action {
            super::ExplorerAction::OpenWithConfig(config) => {
                self.apply_ephemeral_config(config);
            }
        }
    }

    // ... rest of function
}
```

### Phase 3: Verification

#### Build Verification
```bash
# Check that project builds
cargo check

# Run clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all

# Format check
cargo fmt --all -- --check
```

#### Test Verification
```bash
# Run fast test suite
./scripts/test-chunks.sh fast

# If all pass, run full suite
./scripts/test-chunks.sh all
```

#### Manual Verification Checklist
- [ ] CLAUDE.md has both "Query Timing Log" and "Agent Performance Timing Log" sections
- [ ] Cargo.toml has all dependencies: `moka`, `zstd`, `bincode`, `sysinfo`, AND `stood-ai-agent`
- [ ] Explorer cache system uses Moka shared cache
- [ ] Agent framework has context-optimized API (loadCache, getResourceSchema)
- [ ] Explorer window has both minimize flag and V8 action queue
- [ ] All new modules from both branches are present
- [ ] Performance timing works in debug builds
- [ ] CloudWatch telemetry initializes correctly

### Phase 4: Post-Merge Integration

After successful merge, create follow-up commits to integrate features:

#### Commit 1: Integrate shared cache with context-optimized API
- Modify `resources.rs` to use shared Moka cache in the new API functions
- Update `loadCache()` and `queryCachedResources()` to use shared cache
- Test that Agent and Explorer share cache correctly

#### Commit 2: Testing and Documentation
- Run full test suite to ensure integration
- Update any tests that broke due to API changes
- Verify documentation is accurate

---

## Risk Assessment

### High Risk
- **resources.rs**: Accepting entire file from agent-performance-timings means losing bug-iam-queries cache integration temporarily
  - **Mitigation**: Reapply shared cache integration in Phase 4

### Medium Risk
- **window.rs**: Combining two different features in the same initialization flow
  - **Mitigation**: Manual merge with careful testing

### Low Risk
- Auto-merged files: Git handled them correctly
  - **Mitigation**: Quick verification that both features are present

---

## Success Criteria

- [ ] All files merge successfully (manual resolution complete)
- [ ] Project builds without errors
- [ ] Clippy passes with no warnings
- [ ] Fast test suite passes
- [ ] Explorer features from bug-iam-queries work correctly:
  - [ ] Query timing instrumentation
  - [ ] Shared Moka cache
  - [ ] Retry tracker
  - [ ] SDK error categorization
  - [ ] Multi-pane instances
- [ ] Agent features from agent-performance-timings work correctly:
  - [ ] Context-optimized API (loadCache, getResourceSchema)
  - [ ] Performance timing (debug builds)
  - [ ] CloudWatch telemetry
  - [ ] Nova 2 model support
  - [ ] Soft-maximize window feature
- [ ] No functionality lost from either branch
- [ ] All new files from both branches are present

---

## Rollback Plan

If merge fails or breaks functionality:
```bash
# Abort merge
git merge --abort

# Alternative: Manual cherry-pick approach
# Cherry-pick agent-performance-timings commits one by one
# Resolve conflicts incrementally
```

---

## Timeline Estimate

- Phase 1 (Execute merge): 5 minutes
- Phase 2 (Conflict resolution): 15-30 minutes
- Phase 3 (Verification): 30-60 minutes (depends on test suite)
- Phase 4 (Post-merge integration): 1-2 hours

**Total: 2-4 hours for complete merge and integration**

---

## Recommendations

1. **Execute merge during a dedicated session** - Don't rush this
2. **Use a backup branch** - Create `bug-iam-queries-backup` before merging
3. **Test incrementally** - Don't wait until the end to test
4. **Focus on integration in Phase 4** - The shared cache integration is important for performance

**Approved to proceed?**
