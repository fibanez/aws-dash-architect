# Memory Management System

The Resource Explorer implements automatic memory budget enforcement to prevent out-of-memory crashes when loading large AWS resource sets.

## Overview

The memory management system monitors memory usage and enforces an 80% system RAM limit. It uses three key strategies:
- **Memory budget enforcement** - Stop queries before reaching system memory limits
- **Arc sharing** - Reduce memory overhead by sharing data instead of cloning
- **Memory checkpoints** - Track memory allocations at key execution points

This prevents crashes when loading thousands of resources with large JSON properties.

## How to Use

### Understanding Memory Limits

The application automatically detects your system RAM at startup and sets a budget:
- **Total system RAM** - Detected via `sysinfo` crate
- **Memory limit** - 80% of total RAM
- **Budget status** - Shown in logs and status bars

For example, on a system with 16 GB RAM:
- Total memory: 16,384 MB
- Memory limit: 13,107 MB (80%)
- Available budget: 13,107 MB for resource data

### When Queries Stop

If memory usage exceeds 80% of system RAM:
1. Active queries stop immediately
2. User sees error message: "Memory limit exceeded: 12,500 MB / 13,107 MB (80% of system RAM)"
3. Already-loaded resources remain accessible
4. You can clear the cache or remove resources to free memory

This prevents the system from crashing while keeping your work intact.

### Monitoring Memory Usage

Memory usage appears in multiple places:

**Startup logs** (`~/.local/share/awsdash/logs/awsdash.log`):
```
Memory budget initialized: total 16384 MB, limit 13107 MB (80%)
```

**Performance checkpoints** (debug builds only):
```
[14:23:45.123] [ThreadId(5)] [mem: 245.3MB] CHECKPOINT: phase1_complete - 314 resources
[14:23:47.456] [ThreadId(5)] [mem: 892.1MB] CHECKPOINT: phase2_complete - 314 resources
```

**Per-pane status bars**:
```
Phase 1: 50/50 | Phase 2: 45/50 | Memory: 892 MB / 13107 MB (6%)
```

### Strategies to Reduce Memory Usage

If you hit the memory limit:

1. **Clear the cache** - Remove cached resources no longer needed
2. **Reduce scope** - Query fewer accounts or regions at once
3. **Filter resource types** - Select only the resource types you need
4. **Use bookmarks** - Save specific scopes and load them individually

## How it Works

### Memory Budget Enforcement

**MemoryBudget** (`src/app/resource_explorer/memory_budget.rs`)

The memory budget is initialized once at startup:
```rust
pub struct MemoryBudget {
    total_memory_bytes: u64,  // Total system RAM
    max_allowed_bytes: u64,   // 80% limit
}

impl MemoryBudget {
    pub fn initialize() -> &'static Self {
        let total_memory_bytes = System::total_memory();
        let max_allowed_bytes = (total_memory_bytes as f64 * 0.8) as u64;
        // ...
    }
}
```

Queries check the budget before Phase 1 (listing) and Phase 2 (enrichment):
```rust
// Check memory budget before query
if let Err(msg) = MemoryBudget::get().check_usage() {
    // Stop query, display error to user
    return Err(msg);
}
```

The budget is enforced globally across all panes in all windows.

### Arc Sharing for Resource Data

**Problem**: Cloning large resource vectors wastes memory

Before Arc sharing (commit `27aba5e`):
```rust
// QueryProgress callback - expensive clone
pub struct QueryProgress {
    resources: Vec<ResourceEntry>,  // Full clone on every callback!
}

// When query completes with 7753 resources:
// 1. Query engine clones entire vector
// 2. Callback receives cloned vector
// 3. UI adapter clones again for state update
// Total: 3 full copies = 3x memory usage
```

After Arc sharing (commit `658c896`):
```rust
// QueryProgress callback - cheap Arc clone
pub struct QueryProgress {
    resources: Arc<Vec<ResourceEntry>>,  // Reference counted pointer
}

// When query completes with 7753 resources:
// 1. Query engine wraps in Arc
// 2. Callback receives Arc (cheap pointer copy)
// 3. UI adapter unwraps or clones only if needed
// Total: 1 allocation, cheap pointer copies
```

Memory reduction example:
- Before: 1570 MB for 7753 resources (multiple clones)
- After: 200-300 MB for 7753 resources (Arc sharing)
- Savings: **84% reduction** (1270 MB saved)

### Memory Checkpoints

Memory checkpoints track RSS (Resident Set Size) at key execution points to diagnose memory growth:

**Tree Rendering Checkpoints** (`src/app/resource_explorer/tree.rs`)
```rust
#[cfg(debug_assertions)]
crate::perf_checkpoint!("tree.build_tree.start", &format!("resources: {}", resources.len()));

// Build tree from resources
let tree = build_tree(resources, grouping_mode);

#[cfg(debug_assertions)]
crate::perf_checkpoint!("tree.rebuild_complete", &format!("{} resources", resources.len()));
```

**Query Engine Checkpoints** (`src/app/resource_explorer/query_engine.rs`)
```rust
// After Phase 1 completes
crate::app::memory_profiling::memory_checkpoint(
    &format!("after_phase1_complete_{}_resources", resources.len())
);

// After Phase 2 completes
crate::app::memory_profiling::memory_checkpoint(
    &format!("after_phase2_complete_{}_resources", resources.len())
);
```

Checkpoint output in `agent_perf_timing.log`:
```
[14:23:45.123] [ThreadId(5)] [mem: 245.3MB] CHECKPOINT: after_phase1_complete_314_resources
[14:23:47.456] [ThreadId(5)] [mem: 892.1MB] CHECKPOINT: after_phase2_complete_314_resources
```

This shows memory grew from 245 MB to 892 MB during Phase 2 enrichment (647 MB allocated for detailed properties).

### Log Window Tail Mode

**Problem**: Loading entire log file causes 831 MB spike

The log window previously started reading from position 0, loading the entire log file into memory:
```rust
// BEFORE: Start at beginning (loads entire 230 MB file)
let mut last_position = 0;  // Allocates 831 MB!
```

**Solution**: Start from EOF (tail mode)
```rust
// AFTER: Start at end of file (loads on demand)
let mut last_position = if let Ok(metadata) = file.metadata() {
    metadata.len()  // Start at EOF
} else {
    0
};
```

Memory reduction:
- Before: 831 MB spike when opening log window
- After: 0 MB (logs load on demand as they're written)
- Savings: **831 MB saved** at startup

## Integration Points

### Query Engine

The query engine checks memory budget before executing queries:

```rust
impl QueryEngine {
    pub async fn execute_phase1(&mut self) -> Result<(), String> {
        // Check memory budget
        MemoryBudget::get().check_usage()?;

        // Execute queries...
    }
}
```

See [Resource Explorer System](resource-explorer-system.md) for query execution details.

### Resource Cache

The cache uses compressed entries to reduce memory footprint. When memory is tight, you can clear the cache to free space. See [Resource Explorer Caching](resource-explorer-caching.md).

### Multi-Pane Architecture

Memory budget is enforced globally across all panes. If one pane exhausts memory, queries stop in all panes. See [Multi-Pane Architecture](multi-pane-architecture.md).

## Code Example

### Checking Memory Budget

```rust
use crate::app::resource_explorer::memory_budget::MemoryBudget;

// Initialize at startup (call once)
MemoryBudget::initialize();

// Check before expensive operation
match MemoryBudget::get().check_usage() {
    Ok(()) => {
        // Within budget, proceed with operation
        execute_query().await?;
    }
    Err(msg) => {
        // Over budget, display error
        show_error_dialog(&msg);
    }
}

// Get current usage stats
let budget = MemoryBudget::get();
let usage_pct = budget.usage_percentage();
let current_mb = budget.current_usage_mb();
let limit_mb = budget.limit_mb();

println!("Memory: {} MB / {} MB ({:.1}%)", current_mb, limit_mb, usage_pct * 100.0);
```

### Using Arc for Shared Data

```rust
use std::sync::Arc;

// Wrap large data in Arc for sharing
let resources = Arc::new(vec![/* thousands of resources */]);

// Cheap clone (just increments reference count)
let resources_clone = Arc::clone(&resources);

// Unwrap when you need exclusive ownership
let owned = Arc::try_unwrap(resources).unwrap_or_else(|arc| (*arc).clone());
```

### Adding Memory Checkpoints

```rust
// Add checkpoint to track memory at specific point
#[cfg(debug_assertions)]
crate::perf_checkpoint!("my_operation.start", "beginning expensive work");

// ... expensive operation ...

#[cfg(debug_assertions)]
crate::perf_checkpoint!("my_operation.complete", &format!("processed {} items", count));
```

Checkpoints only run in debug builds (zero overhead in release).

## Testing

Test memory budget enforcement:

```rust
#[test]
fn test_memory_budget_enforcement() {
    use crate::app::resource_explorer::memory_budget::MemoryBudget;

    let budget = MemoryBudget::initialize();

    // Budget should be initialized with system RAM
    assert!(budget.total_memory_mb() > 0);
    assert!(budget.limit_mb() > 0);

    // Limit should be 80% of total
    let expected_limit = (budget.total_memory_mb() as f64 * 0.8) as u64;
    assert_eq!(budget.limit_mb(), expected_limit);

    // Usage percentage should be reasonable
    let usage_pct = budget.usage_percentage();
    assert!(usage_pct >= 0.0);
    assert!(usage_pct < 2.0);  // Should not be using 200% of limit!
}
```

## Performance Impact

### Memory Budget Checks

- **Overhead**: ~1-2 microseconds per check
- **Frequency**: Once before Phase 1, once before Phase 2
- **Impact**: Negligible (<0.001% of query time)

### Arc Clones vs Deep Clones

- **Arc::clone()**: ~10-20 nanoseconds (atomic increment)
- **Vec::clone()**: ~500 microseconds for 1000 resources (linear in size)
- **Speedup**: Arc is ~25,000x faster for large collections

### Memory Checkpoint Overhead

- **Debug builds**: ~2-5 milliseconds per checkpoint (reads `/proc/self/status`)
- **Release builds**: **Zero** (checkpoints compiled out)
- **Caching**: 500ms cache prevents per-frame reads

## Common Issues

### "Memory limit exceeded" error

**Cause**: Memory usage exceeded 80% of system RAM

**Solutions**:
1. Clear the cache to free memory
2. Query fewer accounts/regions at once
3. Filter to specific resource types only
4. Close other applications to free system RAM

### Memory keeps growing during queries

**Diagnosis**: Enable debug build and check `agent_perf_timing.log` for memory checkpoints

**Likely causes**:
1. Large JSON properties in Phase 2 enrichment
2. Many resources with tags (tag cache growth)
3. Search filter causing resource clones

**Solutions**:
1. Use property filters to fetch only needed properties
2. Clear tag cache periodically
3. Use Arc sharing for large data structures

## Related Documentation

- [Multi-Pane Architecture](multi-pane-architecture.md) - Global memory budget across panes
- [Resource Explorer Caching](resource-explorer-caching.md) - Cache compression and clearing
- [Performance Monitoring Infrastructure](performance-monitoring-infrastructure.md) - Memory checkpoints and profiling
- [Resource Explorer System](resource-explorer-system.md) - Query execution and memory checks
