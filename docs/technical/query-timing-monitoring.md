# Query Timing & Performance Monitoring

AWS Dash includes comprehensive query timing instrumentation to troubleshoot slow queries and identify stuck operations.

## What You'll See

**Query Timing Log**: All resource queries log detailed timing information to `$HOME/.local/share/awsdash/logs/query_timing.log` (debug builds only).

**Query Lifecycle Tracking**:
- Phase boundaries (Phase 1 start, Phase 2 start)
- Expected queries at phase start
- Individual query start/end with duration
- Cache hits/misses
- Tag fetch operations
- Stuck query detection

## How to Use It

**Monitor Queries in Real-Time**:
```bash
tail -f ~/.local/share/awsdash/logs/query_timing.log
```

**Find Slow Queries**:
```bash
grep "\[<\] DONE" ~/.local/share/awsdash/logs/query_timing.log | sort -t'(' -k2 -n
```

**Check Cache Efficiency**:
```bash
grep "GET_HIT\|GET_MISS" ~/.local/share/awsdash/logs/query_timing.log
```

**Identify Stuck Tag Fetches**:
The log automatically dumps stuck tag fetches (running longer than threshold) with full context: service, resource ID, region, account.

## Log Format

**Phase Boundaries**:
```
[PHASE1] START - 15 expected queries
[PHASE2] START - 8 resources to enrich
```

**Query Execution**:
```
[>] START Lambda::Function:us-east-1
[<] DONE Lambda::Function:us-east-1 (1234ms)
```

**Cache Operations**:
```
[CACHE] GET_HIT S3::Bucket:us-west-2 (0ms)
[CACHE] GET_MISS Lambda::Function:us-east-1
[CACHE] INSERT Lambda::Function:us-east-1 (compressed 45KB)
```

**Source Code**: [query_timing.rs](../src/app/resource_explorer/query_timing.rs)

## Related Documentation

- [Resource Explorer Caching](resource-explorer-caching.md)
- [Resource Explorer System](resource-explorer-system.md)
