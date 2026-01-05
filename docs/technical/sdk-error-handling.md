# SDK Error Handling

AWS Dash categorizes SDK errors to provide clear feedback and track transient failures.

## What You'll See

**Error Categories**: Status bar shows categorized errors during queries:
- Throttled (rate limiting)
- Timeout
- Network errors
- Service unavailable
- Permission errors

**Retry Statistics**: Shows active retries and recovered queries in real-time. AWS SDK handles retries automatically; this provides visibility.

**Service Availability Indicators**: Persistent indicators when services are unavailable in specific regions.

## How It Works

**Error Types**:
- Retryable: Throttling, timeouts, network errors (SDK auto-retries)
- Non-retryable: Permission errors, validation errors

**Global Tracking**: Tracks retry counts, error categories, and recovery statistics across all queries.

**Source Code**: [sdk_errors.rs](../src/app/resource_explorer/sdk_errors.rs), [retry_tracker.rs](../src/app/resource_explorer/retry_tracker.rs)

## Related Documentation

- [Resource Explorer System](resource-explorer-system.md)
- [Query Timing & Monitoring](query-timing-monitoring.md)
