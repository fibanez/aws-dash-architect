# Two-Phase Resource Loading Implementation Plan

## Overview

Implement automatic Phase 2 enrichment after Phase 1 queries complete, with proper waiting mechanism for agents requesting `detail: "full"`.

## Current State

- **Phase 1**: Works - `query_aws_resources_parallel()` fetches basic resource info
- **Phase 2**: Code exists at `aws_client.rs:3766` but **never called**
- **Agent parameter**: `detail` with values `"count"`, `"summary"`, `"tags"`, `"full"`
- **Problem**: `"full"` returns `detailedProperties: null` because Phase 2 never runs

## Target Behavior

1. Phase 1 completes → resources shown immediately in UI
2. Phase 2 triggers automatically after ALL Phase 1 queries complete
3. UI status bar shows Phase 2 progress
4. Agent with `detail: "full"` waits for Phase 2 completion
5. Agent with `detail: "count"/"summary"/"tags"` returns immediately
6. Response includes `detailsLoaded` and `detailsPending` metadata

---

## Critical Files

| File | Purpose |
|------|---------|
| `src/app/resource_explorer/state.rs` | Phase 2 state flags, ResourceEntry |
| `src/app/resource_explorer/aws_client.rs` | `start_phase2_enrichment()` (lines 3766-3908) |
| `src/app/resource_explorer/window.rs` | UI query trigger, Phase 2 completion handler |
| `src/app/resource_explorer/unified_query.rs` | `UnifiedQueryResult`, `DetailLevel` enum |
| `src/app/agent_framework/v8_bindings/bindings/resources.rs` | Agent query path, detail level handling |

---

## Milestone 1: State Infrastructure for Phase 2 Coordination

### Task 1.1: Add Phase 2 Metadata to UnifiedQueryResult

**File**: `src/app/resource_explorer/unified_query.rs`

**Test first** (`tests/unified_query_phase2_tests.rs`):
```rust
#[test]
fn test_query_result_includes_phase2_metadata() {
    let result = UnifiedQueryResult::<Vec<()>>::success_with_phase2_status(
        vec![], 0, true, false
    );
    assert!(result.details_loaded);
    assert!(!result.details_pending);
}

#[test]
fn test_query_result_serialization_includes_metadata() {
    // Verify JSON output has detailsLoaded and detailsPending
}
```

**Changes**:
- Add `details_loaded: bool` field to `UnifiedQueryResult`
- Add `details_pending: bool` field to `UnifiedQueryResult`
- Add constructor `success_with_phase2_status()`
- Update all existing constructors to include defaults

### Task 1.2: Add Phase 2 Tracking to ResourceExplorerState

**File**: `src/app/resource_explorer/state.rs`

**Test first** (`tests/phase2_state_tests.rs`):
```rust
#[test]
fn test_phase2_flags_default_to_false() {
    let state = ResourceExplorerState::default();
    assert!(!state.phase2_enrichment_in_progress);
    assert!(!state.phase2_enrichment_completed);
}

#[test]
fn test_enrichable_resource_types_list() {
    let types = ResourceExplorerState::enrichable_resource_types();
    assert!(types.contains(&"AWS::S3::Bucket"));
    assert!(types.contains(&"AWS::Lambda::Function"));
}
```

**Changes**:
- Flags already exist at lines 628-629 (just verify initialization)
- Add `pub fn enrichable_resource_types() -> &'static [&'static str]` to centralize the list
- Add `pub fn has_pending_enrichment(&self, resource_type: &str) -> bool` helper

---

## Milestone 2: Trigger Phase 2 After Phase 1 Completion

### Task 2.1: Add Phase 2 Trigger to UI Query Flow

**File**: `src/app/resource_explorer/window.rs`

**Test first** (`tests/phase2_trigger_tests.rs`):
```rust
#[test]
fn test_phase2_triggered_after_phase1() {
    // Setup: Complete Phase 1 query
    // Assert: phase2_enrichment_in_progress becomes true
}

#[test]
fn test_phase2_not_triggered_for_empty_results() {
    // No resources = no Phase 2
}

#[test]
fn test_phase2_sets_completed_flag_when_done() {
    // Assert: phase2_enrichment_completed becomes true
}
```

**Changes** (around line 2990, after `state.resources = resources;`):
```rust
// After Phase 1 completes successfully with resources
if !resources.is_empty() {
    // Set in_progress flag
    state.phase2_enrichment_in_progress = true;

    // Spawn Phase 2 in background
    let resources_for_enrichment = resources.clone();
    let aws_client_for_phase2 = aws_client.clone();
    let cache_for_phase2 = cache.clone();
    let state_arc_for_phase2 = state_arc.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(100);
            let (result_tx, _) = tokio::sync::mpsc::channel(100);

            // Handle progress updates
            let state_clone = state_arc_for_phase2.clone();
            tokio::spawn(async move {
                while let Some(progress) = progress_rx.recv().await {
                    if matches!(progress.status, QueryStatus::EnrichmentCompleted) {
                        if let Ok(mut s) = state_clone.try_write() {
                            s.phase2_enrichment_in_progress = false;
                            s.phase2_enrichment_completed = true;
                        }
                    }
                }
            });

            aws_client_for_phase2.start_phase2_enrichment(
                resources_for_enrichment,
                result_tx,
                Some(progress_tx),
                cache_for_phase2,
            );
        });
    });
}
```

### Task 2.2: Track Current Enrichment Service in State

**File**: `src/app/resource_explorer/state.rs`

**Changes**:
```rust
// Add field to track current service being enriched
pub phase2_current_service: Option<String>,  // e.g., "S3 buckets", "Lambda functions"
pub phase2_progress_count: usize,            // e.g., 42
pub phase2_progress_total: usize,            // e.g., 156
```

### Task 2.3: Update Phase 2 to Report Current Service

**File**: `src/app/resource_explorer/aws_client.rs`

**Changes** (in `start_phase2_enrichment`, around line 3873):
The existing `QueryProgress` already has `resource_type` and `message` fields. Ensure progress messages include the service name:
```rust
// Already exists, just verify format:
message: format!("Enriching {} ({}/{})", display_name, processed, total),
```

### Task 2.4: Update Status Bar with Detailed Progress

**File**: `src/app/resource_explorer/window.rs`

**Changes** (in status bar rendering, around line 290):
```rust
// Show detailed Phase 2 progress with service name
if state.phase2_enrichment_in_progress {
    let message = if let Some(service) = &state.phase2_current_service {
        format!(
            "Loading {} details... ({}/{})",
            service,
            state.phase2_progress_count,
            state.phase2_progress_total
        )
    } else {
        "Loading additional details...".to_string()
    };

    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(
            egui::RichText::new(&message)
                .color(Color32::from_rgb(100, 180, 255))
                .small(),
        );
    });
}
```

### Task 2.5: Show Loading State Inside Resource Node

**File**: `src/app/resource_explorer/window.rs` (in resource detail panel rendering)

**Changes**: When rendering expanded resource node, check if details are pending:
```rust
// Inside resource node/detail panel rendering
fn render_resource_details(&self, ui: &mut Ui, resource: &ResourceEntry, state: &ResourceExplorerState) {
    // Check if this resource type supports Phase 2 and details are missing
    let enrichable = ResourceExplorerState::enrichable_resource_types();
    let needs_details = enrichable.contains(&resource.resource_type.as_str())
        && resource.detailed_properties.is_none();

    if needs_details && state.phase2_enrichment_in_progress {
        // Show inline loading message
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(
                egui::RichText::new("Loading details...")
                    .color(Color32::GRAY)
                    .italics(),
            );
        });
    }

    // Continue with normal detail rendering...
    // Use get_display_properties() which falls back to raw_properties
}
```

**Behavior**:
- Message appears when: node is expanded + resource type is enrichable + `detailed_properties` is None + Phase 2 in progress
- Message disappears when: Phase 2 completes OR this specific resource gets enriched
- After Phase 2: `detailed_properties` is populated, condition becomes false, message gone

---

## Milestone 3: Agent Wait Mechanism for detail="full"

### Task 3.1: Add Phase 2 Wait Logic to Agent Query

**File**: `src/app/agent_framework/v8_bindings/bindings/resources.rs`

**Test first** (`tests/agent_phase2_wait_tests.rs`):
```rust
#[tokio::test]
async fn test_agent_full_detail_waits_for_phase2() {
    // Setup: Phase 2 in progress
    // Query with detail="full"
    // Assert: blocks until Phase 2 completes
}

#[tokio::test]
async fn test_agent_summary_returns_immediately() {
    // Setup: Phase 2 in progress
    // Query with detail="summary"
    // Assert: returns immediately without waiting
}

#[tokio::test]
async fn test_agent_full_detail_timeout() {
    // Setup: Phase 2 never completes
    // Assert: returns after timeout with partial data
}
```

**Changes** (in `query_resources_internal()`, after collecting `all_entries`):
```rust
// If detail="full" and Phase 2 is in progress, wait for completion
if detail_level == DetailLevel::Full {
    if let Some(state) = &explorer_state {
        let should_wait = {
            let guard = state.read().await;
            guard.phase2_enrichment_in_progress
        };

        if should_wait {
            // Wait for Phase 2 with 60-second timeout
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_secs(60);

            loop {
                if start.elapsed() > timeout {
                    warn!("Phase 2 wait timeout");
                    break;
                }

                let still_waiting = {
                    let guard = state.read().await;
                    guard.phase2_enrichment_in_progress
                };

                if !still_waiting {
                    break;
                }

                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            // Refresh entries from cache with enriched data
            all_entries = refresh_entries_from_cache(&scope, state.clone()).await;
        }
    }
}
```

### Task 3.2: Add Phase 2 Metadata to Agent Response

**File**: `src/app/agent_framework/v8_bindings/bindings/resources.rs`

**Changes** (when building `UnifiedQueryResult`, around line 385):
```rust
// Determine Phase 2 status
let (details_loaded, details_pending) = if let Some(state) = &explorer_state {
    let guard = state.read().await;
    let pending = guard.phase2_enrichment_in_progress;
    let loaded = all_entries.iter().any(|e| e.detailed_properties.is_some());
    (loaded, pending)
} else {
    (false, false)
};

// Build result with metadata
UnifiedQueryResult {
    status,
    data,
    count: total_count,
    warnings,
    errors,
    details_loaded,
    details_pending,
}
```

### Task 3.3: Add Helper Function to Refresh from Cache

**File**: `src/app/agent_framework/v8_bindings/bindings/resources.rs`

```rust
async fn refresh_entries_from_cache(
    scope: &QueryScope,
    state: Arc<RwLock<ResourceExplorerState>>,
) -> Vec<ResourceEntry> {
    let guard = state.read().await;
    let mut entries = Vec::new();

    for account in &scope.accounts {
        for region in &scope.regions {
            for resource_type in &scope.resource_types {
                let cache_key = format!(
                    "{}:{}:{}",
                    account.account_id, region.region_code, resource_type.resource_type
                );
                if let Some(cached) = guard.cached_queries.get(&cache_key) {
                    entries.extend(cached.clone());
                }
            }
        }
    }

    entries
}
```

---

## Milestone 4: Update Agent Function Documentation

### Task 4.1: Update JavaScript Function Description

**File**: `src/app/agent_framework/tools/execute_javascript.rs`

**Changes** (in tool description):
```rust
- queryResources(options): Query AWS resources
  Parameters: {
    accounts: string[]|null,
    regions: string[]|null,
    resourceTypes: string[],
    detail: "count" | "summary" | "tags" | "full"
  }

  Detail levels:
  - "count": Just the count (fastest, minimal context)
  - "summary": Basic info only - id, name, type, account, region (fast)
  - "tags": Summary + tags array (for tag-based filtering)
  - "full": Complete data with policies/encryption (may wait for background loading)

  Response includes:
  - status: "success" | "partial" | "error"
  - data: array of resources (null for count)
  - count: total number found
  - detailsLoaded: true if detailed properties are included
  - detailsPending: true if background loading is in progress
```

---

## Milestone 5: Edge Cases and Cache Consistency

### Task 5.1: Handle Phase 2 Already Complete

**File**: `src/app/agent_framework/v8_bindings/bindings/resources.rs`

**Changes**: Before waiting logic, check if details already exist:
```rust
// Check if we even need to wait
let enrichable = ResourceExplorerState::enrichable_resource_types();
let needs_phase2 = all_entries.iter().any(|e|
    enrichable.contains(&e.resource_type.as_str())
    && e.detailed_properties.is_none()
);

if !needs_phase2 {
    // All enrichable resources already have details
    details_loaded = true;
    details_pending = false;
    // Skip waiting
}
```

### Task 5.2: Handle New Query During Phase 2

**File**: `src/app/resource_explorer/window.rs`

**Changes**: If user triggers new query while Phase 2 running:
- Let existing Phase 2 complete (it updates the shared cache)
- New Phase 1 runs independently
- New Phase 2 will start after new Phase 1 completes

No special handling needed - the cache is updated by both, and the latest query's Phase 2 will overwrite stale data.

### Task 5.3: Ensure Cache Consistency Between UI and Agent

Both UI and Agent use `ResourceExplorerState.cached_queries`:
- UI writes to cache after Phase 1 (window.rs:2986-2988)
- Agent reads from cache (resources.rs:261-280)
- Agent writes to cache after queries (resources.rs:316-325)
- Phase 2 updates cache in-place (aws_client.rs:3854-3867)

**No changes needed** - the shared cache already works correctly.

---

## Implementation Order

1. **Milestone 1** - Foundation (can be tested in isolation)
   - Task 1.1: UnifiedQueryResult metadata fields
   - Task 1.2: State helpers

2. **Milestone 2** - UI Path (enables manual testing)
   - Task 2.1: Phase 2 trigger
   - Task 2.2-2.5: Status bar and node feedback

3. **Milestone 3** - Agent Path (builds on Milestone 2)
   - Task 3.1: Wait logic
   - Task 3.2: Response metadata
   - Task 3.3: Cache refresh helper

4. **Milestone 4** - Documentation
   - Task 4.1: Update function description

5. **Milestone 5** - Edge cases
   - Task 5.1: Already complete handling
   - Task 5.2: Concurrent query handling
   - Task 5.3: Cache consistency verification

---

## Testing Strategy

### Unit Tests (TDD - Write First)
- `tests/unified_query_phase2_tests.rs` - Metadata serialization
- `tests/phase2_state_tests.rs` - State flag transitions

### Integration Tests (Real Behavior)
- `tests/phase2_trigger_tests.rs` - End-to-end Phase 2 triggering
- `tests/agent_phase2_wait_tests.rs` - Agent wait mechanism

### Manual Testing Checklist
- [ ] Run Explorer query, verify Phase 2 triggers automatically
- [ ] Status bar shows service-specific progress: "Loading S3 bucket details... (5/23)"
- [ ] Status bar animates as different services are enriched
- [ ] Expand resource node during Phase 2, see "Loading details..." inside node
- [ ] "Loading details..." disappears when that resource's enrichment completes
- [ ] Non-enrichable resources (EC2, VPC) never show loading message
- [ ] Agent with `detail: "full"` waits and gets enriched data
- [ ] Agent with `detail: "summary"` returns immediately
- [ ] Response includes `detailsLoaded` and `detailsPending`

---

## Files to Modify Summary

| File | Changes |
|------|---------|
| `unified_query.rs` | Add `details_loaded`, `details_pending` to UnifiedQueryResult |
| `state.rs` | Add `enrichable_resource_types()`, `phase2_current_service`, `phase2_progress_count/total` |
| `window.rs` | Add Phase 2 trigger, detailed status bar, inline loading in resource nodes |
| `resources.rs` | Add wait logic for `detail="full"`, response metadata, cache refresh helper |
| `execute_javascript.rs` | Update function documentation |
| `aws_client.rs` | Verify progress message format includes service name (likely already correct) |
