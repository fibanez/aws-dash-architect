# Memory Profiling Results

This directory contains bytehound memory profiling data for AWS Dash.

## Quick Start

Run the automated profiling script:
```bash
./scripts/profile-memory.sh
```

The script will:
1. Build the app if needed
2. Run the app with bytehound profiling
3. Guide you through testing steps
4. Generate a profile data file
5. Automatically launch the web UI for analysis

## Manual Analysis

To analyze an existing profile:
```bash
~/.local/bin/bytehound server memory-profile-YYYYMMDD_HHMMSS.dat
# Opens at http://localhost:8080
```

## Testing Protocol

When the app runs, perform these actions:
1. Wait for app to fully load (baseline memory)
2. Execute a query to load ~314 resources
3. Wait for Phase 1 completion
4. Wait for Phase 2 enrichment
5. Navigate the tree (expand, collapse, scroll)
6. Quit normally

## Expected Results (After Phase 2 + Phase 3 Optimization)

- **Baseline**: ~100-200MB after startup
- **After loading 314 resources**: ~200-400MB
- **During tree navigation**: Stable (no continuous growth)
- **Peak memory**: <500MB (was 1015MB before optimization)

## What to Check in Web UI

### Timeline View
- Should show memory stabilizing after query completion
- No continuous growth during tree navigation
- Clear distinction between phases

### Flamegraph
- `TreeNode` allocations should be small
- No large clones of resource data
- Most memory in actual resource storage (not tree structure)

### Live Allocations
- After query completion: ~200-400MB live
- Verify no leaked tree clones

### Top Allocators
- Check that resource storage dominates
- TreeNode allocations should be minimal
- No unexpected large allocations

## Profile Files

Each profiling session creates a timestamped `.dat` file:
- `memory-profile-YYYYMMDD_HHMMSS.dat` - Raw profiling data

**Note**: Profile files can be large (100MB-1GB). They are git-ignored.
