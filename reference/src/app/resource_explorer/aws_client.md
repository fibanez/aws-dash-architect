# aws_client.rs - AWS Resource Discovery Engine

## Component Overview

Central orchestrator for multi-account, cross-region AWS resource discovery.
Manages parallel queries, credential coordination, service-specific clients,
data normalization, and caching. Supports 86 AWS services with lazy service
instantiation and parallel execution with semaphore-based concurrency control.

---

## Major Methods & Functions

### Core Public API

**new(credential_coordinator)** - Initialize client with shared credential manager

**get_credential_coordinator()** - Expose credential coordinator for bridge tools

**query_aws_resources_parallel()** - Execute parallel queries across accounts/regions

**query_aws_resources()** - Convenience wrapper for synchronous result collection

**describe_resource()** - Fetch detailed resource metadata by resource entry

### Service Factory Methods (Lazy Instantiation)

**get_ec2_service()** - Create EC2 service client on-demand

**get_iam_service()** - Create IAM service client on-demand

**get_s3_service()** - Create S3 service client on-demand

**get_logs_service()** - Create CloudWatch Logs service client on-demand

**get_cloudtrail_service()** - Create CloudTrail service client on-demand

*(+81 additional service factory methods for other AWS services)*

---

## Implementation Patterns

### Concurrency & Parallelism

**Pattern**: Tokio async with FuturesUnordered and Semaphore-based rate limiting
**Algorithm**: Parallel fan-out with configurable concurrency limits
**Rationale**: Maximize throughput while respecting AWS API rate limits

### Lazy Service Initialization

**Pattern**: Factory methods creating services only when needed
**Algorithm**: Service instantiation on first use, passed credential coordinator
**Rationale**: Reduce memory footprint, avoid pre-creating 86 service objects

### Caching Strategy

**Pattern**: Arc<RwLock<HashMap<String, Vec<ResourceEntry>>>>
**Algorithm**: Cache-first lookup with cache key: "account:region:resource_type"
**Rationale**: Minimize redundant AWS API calls, improve UI responsiveness

### Global Service Handling

**Pattern**: HashSet tracking (account_id, resource_type) pairs
**Algorithm**: Deduplicate global services (IAM, CloudFront, etc.) per account
**Rationale**: Global services exist in single region, avoid duplicate queries

### Error Propagation

**Pattern**: anyhow::Result with context chain using .context()
**Algorithm**: Error wrapping preserves stack trace and adds semantic context
**Rationale**: Rich error messages for debugging AWS API failures

---

## External Systems

### AWS SDK Crates (86 Services)
- aws-sdk-ec2, aws-sdk-s3, aws-sdk-lambda, aws-sdk-dynamodb, aws-sdk-rds
- aws-sdk-iam, aws-sdk-cloudformation, aws-sdk-cloudwatch, aws-sdk-logs
- aws-sdk-ecs, aws-sdk-eks, aws-sdk-elasticloadbalancing, aws-sdk-route53
- *(+73 additional AWS SDK crates)*

### Async Runtime
- tokio - Async executor, Semaphore, RwLock, mpsc channels

### Futures
- futures::stream::FuturesUnordered - Parallel async execution
- futures::stream::StreamExt - Stream combinators

### Logging
- tracing - Structured logging with spans (info, warn, error macros)

### Time
- chrono - Timestamp generation for cache keys

### Internal Dependencies
- CredentialCoordinator - Multi-account credential management
- NormalizerFactory - Service-specific data transformation
- GlobalServiceRegistry - Global service detection and region routing
- 86 Service modules (EC2Service, S3Service, etc.) in aws_services/
- 86 Normalizer modules in normalizers/

---

## Data Structures

### PaginationConfig
```
Purpose: Configure AWS API pagination behavior
Fields:
  - page_size: i32 (default 50) - Items per API request
  - max_items: usize (default 1000) - Prevent runaway queries
  - max_concurrent_requests: usize (default 20) - Concurrency limit
```

### QueryResult
```
Purpose: Result container for single parallel query
Fields:
  - account_id: String - AWS account identifier
  - region: String - AWS region name or "Global"
  - resource_type: String - Service resource type (e.g., "ec2:instance")
  - resources: Result<Vec<ResourceEntry>> - Query outcome or error
  - cache_key: String - Cache lookup key for deduplication
```

### QueryProgress
```
Purpose: Real-time progress notification during parallel queries
Fields:
  - account: String - Account being queried
  - region: String - Region being queried
  - resource_type: String - Resource type being queried
  - status: QueryStatus - Started, InProgress, Completed, Failed
  - message: String - Human-readable status message
  - items_processed: Option<usize> - Pagination progress
  - estimated_total: Option<usize> - Total items estimate
```

### AWSResourceClient
```
Purpose: Main client struct with service factory methods
Fields:
  - normalizer_factory: NormalizerFactory - Data transformation engine
  - credential_coordinator: Arc<CredentialCoordinator> - Shared credentials
  - pagination_config: PaginationConfig - API pagination settings
Architecture: Services created lazily, not pre-instantiated
```

---

## Key Algorithms

### Parallel Query Execution

**Method**: `query_aws_resources_parallel()`

**Pattern**: Fan-out parallelism with semaphore-based concurrency control

**Pseudocode**:
```
1. Create semaphore with max_concurrent_requests limit (default 20)
2. Initialize FuturesUnordered to collect all query futures
3. Build cartesian product: accounts × regions × resource_types
4. For each (account, region, resource_type) tuple:
   a. Check if resource_type is global service (IAM, CloudFront, etc.)
   b. If global:
      - Deduplicate using HashSet<(account_id, resource_type)>
      - Force query to us-east-1 region only
      - Cache key: "account:Global:resource_type"
   c. If regional:
      - Cache key: "account:region:resource_type"
   d. Check cache first:
      - If hit: Send cached result immediately, skip query
      - If miss: Create query future
5. For each query future:
   a. Acquire semaphore permit (blocks if limit reached)
   b. Send QueryProgress::Started notification
   c. Execute query via service-specific client method
   d. Normalize result using NormalizerFactory
   e. Update cache on success
   f. Send QueryResult via result_sender channel
   g. Send QueryProgress::Completed/Failed notification
   h. Release semaphore permit
6. Await all futures in FuturesUnordered (unordered completion)
7. Return Ok(()) when all queries complete
```

**Concurrency Model**: Up to 20 concurrent AWS API requests in flight
**Error Handling**: Individual query failures don't abort entire operation
**Progress Tracking**: Real-time updates via mpsc channel to UI

---

### Synchronous Result Collection

**Method**: `query_aws_resources()`

**Pattern**: Convenience wrapper around query_aws_resources_parallel()

**Used By**: CloudFormation Manager's ResourceLookupService for parameter dropdowns

**Pseudocode**:
```
1. Convert mutable HashMap cache to Arc<RwLock<HashMap>>
2. Create mpsc channel for QueryResult (buffer size: 1000)
3. Spawn query_aws_resources_parallel() future with result_sender
4. Concurrently collect results:
   a. Await query_future completion
   b. Receive QueryResults from result_receiver channel
   c. For each QueryResult:
      - If Ok: Extend all_resources Vec with resources
      - If Err: Skip (errors already logged)
5. Update original cache from Arc<RwLock> final state
6. Extract relationships between resources (extract_all_relationships)
7. Return aggregated Vec<ResourceEntry>
```

**Purpose**: Simplifies API for callers needing synchronous result aggregation
**Implementation**: Wraps parallel version, handles channels, collects into Vec
**Use Case**: CloudFormation parameter resource lookup (EC2, S3, Lambda, etc.)

---

### Resource Description Lookup

**Method**: `describe_resource()`

**Pattern**: Polymorphic dispatch to service-specific describe methods

**Pseudocode**:
```
1. Extract resource_type from ResourceEntry
2. Dispatch via match statement on resource_type:
   - "ec2:instance" → get_ec2_service().describe_instance()
   - "s3:bucket" → get_s3_service().describe_bucket()
   - "lambda:function" → get_lambda_service().describe_function()
   - *(86 resource types)*
3. Service executes detailed describe API call:
   a. Use resource.id as primary lookup key
   b. Parse additional identifiers from resource.metadata
   c. Execute AWS SDK describe call (e.g., DescribeInstances)
4. Return raw JSON from AWS SDK response
5. Error propagation: anyhow::Result with context
```

**Lookup Strategy**: ID-based describe calls for detailed resource inspection
**JSON Output**: Preserves full AWS API response for maximum detail
**Error Context**: Adds resource type, ID, account, region to error chain

---

### Global Service Deduplication

**Method**: Internal logic in `query_aws_resources_parallel()`

**Pattern**: HashSet-based memoization

**Pseudocode**:
```
1. Initialize queried_global_services: HashSet<(String, String)>
2. Initialize global_registry: GlobalServiceRegistry
3. For each (account, resource_type) pair:
   a. Check if global_registry.is_global(resource_type)
   b. If not global: Process normally (query all regions)
   c. If global:
      - Build key: (account.account_id, resource_type)
      - If key in queried_global_services: Skip (already queried)
      - Else:
        * Insert key into queried_global_services
        * Override region to global_registry.get_query_region() (us-east-1)
        * Cache key uses "Global" as region: "account:Global:resource_type"
        * Execute single query for this account+resource_type pair
```

**Global Services**: IAM users/roles, CloudFront distributions, Route53 zones
**Query Region**: Always us-east-1 for global services (AWS convention)
**Cache Optimization**: Prevents N×M queries for global services (M regions)

---

### Lazy Service Instantiation

**Pattern**: Factory methods for 86 AWS services

**Pseudocode**:
```
fn get_<service>_service(&self) -> <Service>Service {
  <Service>Service::new(Arc::clone(&self.credential_coordinator))
}

Example:
fn get_ec2_service(&self) -> EC2Service {
  EC2Service::new(Arc::clone(&self.credential_coordinator))
}
```

**Rationale**: Avoid allocating 86 service objects upfront
**Memory**: Services created on-demand when query() routes to them
**Lifetime**: Service objects dropped after query completes (not retained)

---

## Performance Characteristics

### Concurrency
- Default: 20 concurrent AWS API requests in flight
- Configurable via PaginationConfig.max_concurrent_requests
- Semaphore prevents thundering herd to AWS APIs

### Caching
- In-memory HashMap with RwLock for thread-safe access
- Cache key: account:region:resource_type (or account:Global:resource_type)
- No expiration policy (cache valid for application lifetime)

### Pagination
- Page size: 50 items per request (configurable)
- Max items: 1000 per query (safety limit)
- Pagination logic delegated to service-specific methods

### Memory
- Lazy service instantiation: Services not pre-allocated
- Cache grows unbounded (no eviction policy)
- Arc<CredentialCoordinator> shared across all services

---

## Error Handling Strategy

### Error Types
- AWS SDK errors: Network failures, authentication, API throttling
- Cache errors: RwLock poisoning (rare, panic-inducing)
- Channel errors: mpsc send failures (receiver dropped)

### Error Propagation
```
Pattern: anyhow::Result with .context() chaining
Example:
  service.query()
    .await
    .context("Failed to query EC2 instances")?
```

### Error Recovery
- Individual query failures: Logged, sent as Err in QueryResult
- Channel send failures: Logged as warnings, query continues
- Cache poisoning: Panic (unrecoverable, indicates severe bug)

---

## Testing Considerations

### Integration Testing
- Requires real AWS credentials (no mocks per CLAUDE.md)
- Test against actual AWS accounts with known resources
- Verify cache hit/miss behavior with multiple queries

### Unit Testing
- Test pagination config defaults
- Test cache key generation
- Test global service deduplication logic (HashSet behavior)

### Performance Testing
- Measure query latency with different concurrency limits
- Verify semaphore prevents >N concurrent requests
- Test cache effectiveness (second query should be instant)

---

## Dependencies Graph

```
AWSResourceClient
  ├── CredentialCoordinator (Arc shared)
  ├── NormalizerFactory
  ├── GlobalServiceRegistry
  ├── 86 × Service modules (lazy instantiation)
  │     ├── EC2Service, S3Service, LambdaService, ...
  │     └── Each service wraps AWS SDK client
  ├── 86 × Normalizer modules
  │     ├── EC2Normalizer, S3Normalizer, LambdaNormalizer, ...
  │     └── Transform SDK types → ResourceEntry
  ├── tokio runtime (Semaphore, RwLock, mpsc)
  ├── futures (FuturesUnordered, StreamExt)
  └── tracing (structured logging)
```

---

## Common Pitfalls

### Global Service Queries
**Issue**: Querying IAM users in all 20 regions wastes 19 queries
**Solution**: GlobalServiceRegistry detects global services, queries only us-east-1

### Cache Key Conflicts
**Issue**: Same resource in different accounts/regions must have unique keys
**Solution**: Cache key includes account:region:resource_type (3-tuple)

### Semaphore Deadlock
**Issue**: All permits acquired but futures await each other
**Solution**: Semaphore released when future completes (via RAII)

### Channel Backpressure
**Issue**: Receiver too slow, mpsc channel buffer fills up
**Solution**: Use bounded channel with appropriate buffer size

---

## Future Improvements

### Cache Eviction
- Implement TTL-based cache expiration
- Add LRU eviction policy for large resource sets

### Query Prioritization
- Allow high-priority queries to bypass semaphore queue
- Implement query cancellation for user navigation changes

### Incremental Updates
- Detect changed resources via AWS Resource Groups Tagging API
- Refresh only changed resources instead of full re-query

### Metrics & Observability
- Track query latency per service/region
- Monitor cache hit rate
- Expose Prometheus metrics endpoint

---

**Last Updated**: 2025-10-24
**Source File**: `src/app/resource_explorer/aws_client.rs`
**Lines of Code**: ~106 KB
**Complexity**: High (multi-service orchestration with parallelism)
