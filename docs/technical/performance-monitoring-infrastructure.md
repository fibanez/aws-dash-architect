# Performance Monitoring Infrastructure

The Resource Explorer includes debug-only performance timing infrastructure for diagnosing bottlenecks and tracking memory allocations.

## Overview

The performance monitoring system provides zero-overhead timing instrumentation for debug builds. It tracks:
- **Operation timing** - Duration of expensive operations (queries, tree rendering, etc.)
- **Memory usage** - RSS (Resident Set Size) at checkpoints
- **Nested operations** - Hierarchical timing with automatic indentation
- **Query lifecycle** - Phase 1/Phase 2 execution with cache hits/misses

All monitoring is compiled out in release builds for zero production overhead.

## How to Use

### Viewing Performance Data

Performance timing data is written to:
```
~/.local/share/awsdash/logs/agent_perf_timing.log
```

Tail the log in real-time to monitor operations:
```bash
tail -f ~/.local/share/awsdash/logs/agent_perf_timing.log
```

### Interpreting Timing Output

**Format**: `[timestamp] [thread] operation_name: duration (context)`

Example output:
```
[14:23:45.123] [ThreadId(5)/tokio-runtime-worker] query_engine.phase1: 1.23s (50 queries)
[14:23:45.456] [ThreadId(5)/tokio-runtime-worker]   ec2.describe_instances: 234.56ms (us-east-1)
[14:23:45.678] [ThreadId(5)/tokio-runtime-worker]   rds.describe_db_instances: 123.45ms (us-east-1)
[14:23:47.890] [ThreadId(5)/tokio-runtime-worker] query_engine.phase2: 2.43s (45 resources)
```

Indentation shows nesting:
- `query_engine.phase1` is the parent operation
- `ec2.describe_instances` is a child operation (indented)
- `rds.describe_db_instances` is another child operation

### Understanding Memory Checkpoints

Memory checkpoints show RSS usage at specific points:

```
[14:23:45.123] [ThreadId(5)] [mem: 245.3MB] CHECKPOINT: phase1_complete - 314 resources
[14:23:47.456] [ThreadId(5)] [mem: 892.1MB] CHECKPOINT: phase2_complete - 314 resources
```

The memory grew from 245 MB to 892 MB during Phase 2 enrichment (647 MB allocated).

### Analyzing Query Performance

**Find slow queries**:
```bash
grep "\.phase1\|\.phase2" agent_perf_timing.log | sort -t'(' -k1 -r
```

**Track cache performance**:
```bash
grep "cache.*hit\|cache.*miss" ~/.local/share/awsdash/logs/query_timing.log
```

**Find memory spikes**:
```bash
grep "CHECKPOINT" agent_perf_timing.log | sort -t':' -k3 -n
```

### Query Timing Log

In addition to `agent_perf_timing.log`, query-specific timing is written to:
```
~/.local/share/awsdash/logs/query_timing.log
```

This log shows:
- Phase boundaries (`[PHASE1] expecting N queries`, `[PHASE2] expecting N queries`)
- Cache hits/misses (`[CACHE] GET_HIT`, `[CACHE] GET_MISS`)
- Query execution (`[>] START AWS::EC2::Instance`, `[<] DONE AWS::EC2::Instance (1234ms)`)
- Tag fetching (`[TAGS] fetch_start`, `[TAGS] fetch_done`)

See [Query Timing & Monitoring](query-timing-monitoring.md) for details.

## How it Works

### Zero-Overhead Debug Macros

Performance monitoring uses conditional compilation to achieve zero overhead:

```rust
// Debug builds: full implementation
#[cfg(debug_assertions)]
pub fn log_timing(operation: &str, duration_us: u64, context: Option<&str>) {
    // Write to log file
}

// Release builds: no-op stub
#[cfg(not(debug_assertions))]
pub fn log_timing(_operation: &str, _duration_us: u64, _context: Option<&str>) {}
```

In release builds, the macros expand to nothing and are optimized away entirely.

### Timing Macros

**perf_start! / perf_end!** - Manual timing with named timers
```rust
perf_start!("my_operation");
// ... code to time ...
perf_end!("my_operation");
perf_end!("my_operation", "with context");
```

**perf_timed!** - Time an expression and return its value
```rust
let result = perf_timed!("fetch_data", expensive_fetch());
let result = perf_timed!("fetch_data", "us-east-1", expensive_fetch());
```

**perf_timed_block!** - Time a block of code
```rust
perf_timed_block!("initialization", {
    init_config();
    init_resources();
});
```

**perf_guard!** - RAII timing guard (logs on drop)
```rust
fn my_function() {
    let _timing = perf_guard!("my_function");
    // ... function body ...
    // Timing logged automatically when _timing drops
}
```

**perf_checkpoint!** - Log a checkpoint with memory usage
```rust
perf_checkpoint!("operation_complete");
perf_checkpoint!("operation_complete", &format!("{} items", count));
```

### Memory Tracking

Each checkpoint logs RSS (Resident Set Size) memory usage:

```rust
pub fn log_checkpoint(name: &str, context: Option<&str>) {
    let memory_str = if let Some(usage) = memory_stats::memory_stats() {
        let mem_mb = usage.physical_mem as f64 / 1024.0 / 1024.0;
        format!(" [mem: {:.1}MB]", mem_mb)
    } else {
        String::new()
    };

    writeln!(file, "[{}] [{}]{} CHECKPOINT: {}{}",
        timestamp, thread_id, memory_str, name, context_str);
}
```

Memory is fetched using the `memory_stats` crate (reads `/proc/self/status` on Linux).

### Checkpoint Caching

To avoid excessive `/proc` reads, memory stats are cached for 500ms:

```rust
static MEMORY_STATS_CACHE: Mutex<Option<(MemoryStats, Instant)>> = Mutex::new(None);
const MEMORY_STATS_CACHE_DURATION: Duration = Duration::from_millis(500);

fn get_cached_memory_stats() -> Option<MemoryStats> {
    let mut cache = MEMORY_STATS_CACHE.lock().ok()?;

    // Return cached value if recent (< 500ms old)
    if let Some((stats, timestamp)) = cache.as_ref() {
        if timestamp.elapsed() < MEMORY_STATS_CACHE_DURATION {
            return Some(stats.clone());
        }
    }

    // Fetch new stats and cache
    if let Some(new_stats) = memory_stats::memory_stats() {
        *cache = Some((new_stats.clone(), Instant::now()));
        Some(new_stats)
    } else {
        None
    }
}
```

This prevents reading `/proc/self/status` on every frame for every pane (which was causing 4.7 GB of allocations in 2.2 seconds).

### Thread-Local Timer Stack

Nested timing uses a thread-local stack for hierarchical display:

```rust
thread_local! {
    static TIMER_STACK: RefCell<Vec<(String, Instant)>> = RefCell::new(Vec::new());
}

pub fn push_timer(name: &str) {
    TIMER_STACK.with(|stack| {
        stack.borrow_mut().push((name.to_string(), Instant::now()));
    });
}

pub fn pop_timer(context: Option<&str>) {
    TIMER_STACK.with(|stack| {
        if let Some((name, start)) = stack.borrow_mut().pop() {
            let duration = start.elapsed();
            let indent = "  ".repeat(stack.borrow().len());  // Indent based on depth
            log_timing(&format!("{}{}", indent, name), duration.as_micros() as u64, context);
        }
    });
}
```

This produces indented output showing parent/child relationships.

## Integration Points

### Query Engine

The query engine uses timing macros extensively:

```rust
impl QueryEngine {
    pub async fn execute_phase1(&mut self) -> Result<(), String> {
        let _timing = perf_guard!("query_engine.phase1", &format!("{} queries", self.expected_queries));

        // Execute queries...
        for query in &self.queries {
            perf_checkpoint!("query.start", &query.resource_type);
            let result = execute_query(query).await?;
            perf_checkpoint!("query.complete", &format!("{} resources", result.len()));
        }

        Ok(())
    }
}
```

See [Resource Explorer System](resource-explorer-system.md) for query execution details.

### Tree Rendering

Tree rendering adds checkpoints at key points:

```rust
impl TreeRenderer {
    pub fn render_cached(&mut self, resources: &[ResourceEntry], ...) {
        #[cfg(debug_assertions)]
        perf_checkpoint!("tree.render_cached.start", &format!("{} resources", resources.len()));

        // Check if rebuild needed
        if self.cache_key != new_cache_key {
            #[cfg(debug_assertions)]
            perf_checkpoint!("tree.build_tree.start", &format!("resources: {}", resources.len()));

            self.cached_tree = Some(build_tree(resources, grouping_mode));

            #[cfg(debug_assertions)]
            perf_checkpoint!("tree.rebuild_complete", &format!("{} resources", resources.len()));
        }

        // Render tree
        #[cfg(debug_assertions)]
        perf_checkpoint!("tree.render_cached.before_render_node", "");

        self.render_node(ui, &tree, resources, 0, search_filter);
    }
}
```

See [Multi-Pane Architecture](multi-pane-architecture.md) for tree rendering details.

### Memory Management

Memory checkpoints track allocations at critical points:

```rust
// After Phase 1 completes
crate::app::memory_profiling::memory_checkpoint(
    &format!("after_phase1_complete_{}_resources", resources.len())
);

// After Phase 2 completes
crate::app::memory_profiling::memory_checkpoint(
    &format!("after_phase2_complete_{}_resources", resources.len())
);

// After tree rebuild
crate::app::memory_profiling::memory_checkpoint(
    &format!("after_tree_rebuild_{}_resources", resources.len())
);
```

See [Memory Management System](memory-management-system.md) for memory tracking details.

## Code Examples

### Basic Timing

```rust
use crate::app::agent_framework::perf_timing::*;

// Time a function call
fn fetch_resources() -> Vec<Resource> {
    let _timing = perf_guard!("fetch_resources");
    // ... fetch logic ...
}

// Time an expression
let result = perf_timed!("parse_json", serde_json::from_str(&json_str)?);

// Manual start/end
perf_start!("complex_operation");
do_step_1();
do_step_2();
perf_end!("complex_operation");
```

### Nested Timing

```rust
fn process_queries() {
    let _timing = perf_guard!("process_queries");

    for query in queries {
        let _query_timing = perf_guard!("execute_query", &query.resource_type);
        execute_query(query);
        // Automatically logged with indentation when guard drops
    }
}
```

Output:
```
[14:23:45.123] [ThreadId(5)] process_queries: 5.67s
[14:23:45.234] [ThreadId(5)]   execute_query: 1.23s (AWS::EC2::Instance)
[14:23:46.456] [ThreadId(5)]   execute_query: 2.34s (AWS::RDS::DBInstance)
[14:23:48.789] [ThreadId(5)]   execute_query: 1.10s (AWS::S3::Bucket)
```

### Memory Checkpoints

```rust
fn allocate_resources() {
    perf_checkpoint!("before_allocation", "baseline memory");

    let resources = vec![/* thousands of resources */];

    perf_checkpoint!("after_allocation", &format!("{} resources", resources.len()));
}
```

Output:
```
[14:23:45.123] [ThreadId(5)] [mem: 245.3MB] CHECKPOINT: before_allocation - baseline memory
[14:23:47.456] [ThreadId(5)] [mem: 892.1MB] CHECKPOINT: after_allocation - 7753 resources
```

Shows memory grew from 245 MB to 892 MB (647 MB allocated).

### Conditional Logging

```rust
// Only log in debug builds
#[cfg(debug_assertions)]
{
    perf_checkpoint!("debug_point", "expensive debug info");
}

// Or use the macro directly (compiles to nothing in release)
perf_checkpoint!("always_present", "but zero overhead in release");
```

## Testing

Performance monitoring can be tested by verifying log output:

```rust
#[test]
fn test_timing_macros() {
    // Initialize timing system
    init_perf_log();

    // Test basic timing
    perf_start!("test_operation");
    std::thread::sleep(std::time::Duration::from_millis(10));
    perf_end!("test_operation");

    // Test timed expression
    let result = perf_timed!("timed_expr", {
        std::thread::sleep(std::time::Duration::from_millis(5));
        42
    });
    assert_eq!(result, 42);

    // Test checkpoint
    perf_checkpoint!("test_checkpoint");

    // Verify log file was created
    let log_path = dirs::data_local_dir()
        .unwrap()
        .join("awsdash/logs/agent_perf_timing.log");
    assert!(log_path.exists());
}
```

## Performance Impact

### Debug Builds

- **Timing overhead**: ~1-5 microseconds per measurement
- **Memory checkpoint overhead**: ~2-5 milliseconds (reads `/proc/self/status`)
- **Checkpoint caching**: Reduces overhead from per-frame to every 500ms
- **File I/O overhead**: ~10-50 microseconds per log write

**Total impact**: <1% of query execution time for typical workloads

### Release Builds

- **All overhead**: **ZERO** (macros compiled out entirely)
- **Binary size**: No increase (dead code elimination)
- **Runtime cost**: None

## Common Use Cases

### Diagnosing Slow Queries

1. Tail `agent_perf_timing.log` in real-time
2. Trigger the slow operation
3. Find the timing entry with high duration
4. Add more granular timing within that operation
5. Rebuild in debug mode and re-test

### Finding Memory Leaks

1. Add memory checkpoints before/after suspected operations
2. Run the operation multiple times
3. Compare memory usage across iterations
4. If memory grows linearly, you have a leak

### Optimizing Tree Rendering

1. Add checkpoints in `tree.rs` at key points
2. Trigger tree rebuild with large resource set
3. Find which checkpoint shows largest memory/time jump
4. Optimize that specific operation

### Profiling Agent Performance

1. Add timing guards in agent execution code
2. Run agent with realistic workload
3. Analyze timing distribution (model calls vs tool execution vs parsing)
4. Optimize the slowest component

## Related Documentation

- [Memory Management System](memory-management-system.md) - Memory checkpoints and tracking
- [Query Timing & Monitoring](query-timing-monitoring.md) - Query-specific timing logs
- [Resource Explorer System](resource-explorer-system.md) - Query execution with timing
- [Multi-Pane Architecture](multi-pane-architecture.md) - Per-pane rendering performance
