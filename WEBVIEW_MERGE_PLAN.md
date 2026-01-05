# Merge Plan: webview-test → bug-iam-queries

## Executive Summary

Merging `webview-test` into `bug-iam-queries` to add AWS Console webview functionality.

**Conflict Summary:**
- 3 files with conflicts: `mod.rs`, `window.rs`, `main.rs`
- 8 files auto-merged successfully
- 3 new files from webview-test

---

## Branch Comparison

### webview-test (2 commits) - AWS Console Integration
**Main Theme:** Embedded webview for AWS Console access with role selection

**Commits:**
1. `12ee1db` - feat: add AWS Console role selection submenu for resources
2. `344eda6` - feat: add embedded webview for AWS console access

**Common Ancestor:** `802b311` - feat(ui): add unified selection dialog and collapsible active selection

**Key New Files:**
- `docs/console_url_map.csv` - AWS Console URL mappings (167 lines)
- `src/app/resource_explorer/console_links.rs` - Console URL generation (858 lines)
- `src/app/webview.rs` - Webview integration (81 lines)

**Key Modifications:**
- `aws_identity.rs` - Added role management (+308 lines)
- `tree.rs` - Added console context menu (+196 lines)
- `window_rendering.rs` - Added webview window rendering (+126 lines)
- `mod.rs` - Added console_links module and actions
- `window.rs` - Added console role menu state
- `main.rs` - Changed return type for webview support
- `Cargo.toml` - Added webview dependencies

### bug-iam-queries (38 commits since divergence)
**Main Theme:** Explorer improvements + Agent framework enhancements

**Already includes:**
- All agent-performance-timings changes (just merged)
- Explorer query timing, caching, error handling
- Multi-pane architecture
- Performance instrumentation

---

## Detailed Conflict Analysis

### 1. src/app/resource_explorer/mod.rs - Module Declaration (Line 167)

**Conflict:**
```rust
<<<<<<< HEAD
pub mod cache;
=======
pub mod console_links;
>>>>>>> webview-test
```

**Resolution Strategy:** Keep BOTH modules
```rust
pub mod cache;
pub mod console_links;
```

### 2. src/app/resource_explorer/window.rs - Struct Initialization (Line 153)

**Conflict:**
```rust
<<<<<<< HEAD
            show_service_availability_dialog: false,
            last_failed_queries: std::collections::HashMap::new(),
            last_failed_queries_snapshotted: false,
=======
            console_role_menu_updates,
>>>>>>> webview-test
```

**Resolution Strategy:** Keep ALL fields (both branches add independent features)
```rust
            show_service_availability_dialog: false,
            last_failed_queries: std::collections::HashMap::new(),
            last_failed_queries_snapshotted: false,
            console_role_menu_updates,
```

### 3. src/main.rs - Function and Return Type (Line 66)

**Conflict:**
```rust
<<<<<<< HEAD
fn init_perf_timing_path() {
    // Performance timing initialization
    #[cfg(debug_assertions)]
    {
        // ... implementation
    }
}

fn main() -> eframe::Result {
=======
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
>>>>>>> webview-test
```

**Resolution Strategy:**
1. Keep `init_perf_timing_path()` function from bug-iam-queries
2. Keep webview's main() signature change: `Result<(), Box<dyn std::error::Error>>`
3. Ensure init_perf_timing_path() is called in main()

**Note:** Need to check webview's main() implementation to integrate properly.

---

## Auto-Merged Files (Needs Verification)

These files merged without conflicts but need verification:

### 1. Cargo.toml
- webview-test: Added webview dependencies
- bug-iam-queries: Added cache deps, changed stood dependency
- **Verification:** Ensure all dependencies present

### 2. src/app/aws_identity.rs
- webview-test: Added role management methods
- bug-iam-queries: Appears to have independent changes
- **Verification:** Check that role methods are present

### 3. src/app/dashui/app/window_rendering.rs
- webview-test: Added webview window rendering
- bug-iam-queries: Various window rendering updates
- **Verification:** Check webview rendering is integrated

### 4. src/app/resource_explorer/tree.rs
- webview-test: Added console context menu
- bug-iam-queries: Various tree improvements
- **Verification:** Ensure context menu works

---

## New Files (No Conflicts)

### From webview-test
- ✓ `docs/console_url_map.csv` - AWS Console URL mappings
- ✓ `src/app/resource_explorer/console_links.rs` - URL generation logic
- ✓ `src/app/webview.rs` - Webview integration module

---

## Merge Execution Plan

### Phase 1: Create Backup and Execute Merge
```bash
git branch bug-iam-queries-backup-2
git merge --no-ff webview-test
```

### Phase 2: Resolve Conflicts

#### mod.rs Resolution
```rust
pub mod cache;
pub mod console_links;
```

#### window.rs Resolution
Keep all initialization fields:
```rust
            show_service_availability_dialog: false,
            last_failed_queries: std::collections::HashMap::new(),
            last_failed_queries_snapshotted: false,
            console_role_menu_updates,
```

#### main.rs Resolution
This requires reading both versions to understand the complete integration:
1. Keep init_perf_timing_path() function
2. Adopt webview's main() signature
3. Integrate args parsing if needed
4. Ensure init_perf_timing_path() is called

### Phase 3: Verification

#### Build Verification
```bash
cargo check
cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all
```

#### Test Verification
```bash
TEST_MODE=detailed ./scripts/test-chunks.sh fast
```

#### Feature Verification Checklist
- [ ] Webview dependencies in Cargo.toml
- [ ] console_links module compiles
- [ ] webview module compiles
- [ ] AWS Console context menu appears in tree
- [ ] Role selection submenu works
- [ ] Explorer features still work (cache, query timing, etc.)
- [ ] Agent features still work (performance timing, etc.)

---

## Success Criteria

- [ ] All files merge successfully
- [ ] Project builds without errors
- [ ] Clippy passes
- [ ] Fast test suite passes
- [ ] Webview features from webview-test present
- [ ] Explorer features from bug-iam-queries preserved
- [ ] Agent features from agent-performance-timings preserved
- [ ] No functionality lost

---

## Dependencies Expected After Merge

From webview-test:
- Webview-related dependencies

From bug-iam-queries:
- moka, zstd, bincode, sysinfo (cache)
- stood-ai-agent with perf-timing (agent framework)

---

## Rollback Plan

```bash
git merge --abort  # If during merge
git reset --hard bug-iam-queries-backup-2  # If after merge
```

---

## Estimated Timeline

- Phase 1 (Execute merge): 2 minutes
- Phase 2 (Resolve conflicts): 10-15 minutes
- Phase 3 (Verification): 10-20 minutes

**Total: ~30 minutes**
