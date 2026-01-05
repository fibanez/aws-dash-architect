# Resource Explorer Caching

The Resource Explorer uses shared caching to accelerate queries and minimize AWS API calls.

## What You'll Experience

**Instant Results**: First query fetches from AWS, subsequent queries return instantly from cache. Cache expires after 30 minutes of inactivity.

**Shared Across Windows**: All Explorer windows, tabs, and panes share the same cache. Query in one window, see instant results in another.

**Automatic Sizing**: Cache auto-sizes to 25% of available system memory (512MB to 8GB). Uses 8-10x compression to maximize capacity.

## How It Works

**Two-Tier Structure**:
- Resource cache: Basic resource lists by `(account, region, type)`
- Details cache: Enriched properties by `(account, region, type, id)`

**Phase Integration**:
- Phase 1 caches resource lists
- Phase 2 caches detailed properties for individual resources
- Agent queries with `detail: "full"` wait for Phase 2, others use cached Phase 1 data

**Source Code**: [cache.rs](../src/app/resource_explorer/cache.rs)

## Related Documentation

- [Resource Explorer System](resource-explorer-system.md)
- [Code Execution Tool](code-execution-tool.md)
