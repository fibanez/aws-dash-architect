# CloudWatch Logs Feature - Implementation Plan

## Overview

Add CloudWatch Logs querying capability to both the Resource Explorer and Agent V2, allowing users to view and analyze log data directly within the application.

## User Decisions

**Key decisions made during planning:**

1. **Function Name**: `queryCloudWatchLogEvents()`
2. **Resource Explorer Default Behavior**: Always show latest 100 events (simple, predictable)
3. **Auto-refresh**: Manual refresh only (user clicks Refresh button)
4. **Log Viewer Display Limit**: 1000 events maximum in UI
5. **Agent V2 Limit Parameter**: Expose as LLM-controllable parameter, mapped to actual SDK options
6. **Filtering**: Reuse fuzzy search pattern matching from Resource Explorer search

## Function Name Decision

**User Decision**: `queryCloudWatchLogEvents()`

This name combines:
- Clear action verb "query" indicating data retrieval
- Explicit service name "CloudWatch"
- Specific data type "LogEvents"
- Perfect balance of clarity and specificity for LLM context

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    CloudWatch Logs Feature                   │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────────┐         ┌──────────────────────────┐  │
│  │  Resource Explorer│         │      Agent V2            │  │
│  │                   │         │                          │  │
│  │  [View Logs] btn  │         │ queryCloudWatchLogEvents()│  │
│  │        │          │         │         │                │  │
│  │        v          │         │         v              │  │
│  │  CloudWatchLogs   │         │    V8 Binding         │  │
│  │     Window        │         │         │              │  │
│  └────────┬──────────┘         └─────────┬──────────────┘  │
│           │                              │                  │
│           └──────────┬───────────────────┘                  │
│                      v                                      │
│           ┌─────────────────────────┐                       │
│           │  CloudWatch Logs Client │                       │
│           │   (AWS SDK Integration) │                       │
│           └─────────────────────────┘                       │
└─────────────────────────────────────────────────────────────┘
```

---

## Milestone 1: CloudWatch Logs Client Foundation

**Goal**: Create reusable AWS CloudWatch Logs SDK integration

### Tasks

#### M1.T1: Create CloudWatch Logs Client Module
- **File**: `src/app/cloudwatch_logs/mod.rs`
- **Responsibilities**:
  - Wrap AWS SDK CloudWatch Logs client
  - Handle credential management (from AwsIdentityCenter)
  - Provide region-aware client creation
- **Dependencies**: `aws-sdk-cloudwatchlogs` crate
- **Acceptance Criteria**:
  - Can create client with IAM Identity Center credentials
  - Supports multi-region queries
  - Proper error handling with context

#### M1.T2: Implement Log Query Functionality
- **File**: `src/app/cloudwatch_logs/query.rs`
- **Functions**:
  ```rust
  /// Query CloudWatch Logs for a specific log group
  pub async fn query_log_events(
      client: &Client,
      log_group_name: &str,
      options: QueryOptions,
  ) -> Result<LogQueryResult>

  /// Get the latest log events (last 5 minutes or last available)
  pub async fn get_recent_log_events(
      client: &Client,
      log_group_name: &str,
  ) -> Result<LogQueryResult>
  ```
- **QueryOptions Structure**:
  ```rust
  pub struct QueryOptions {
      /// Start time (Unix timestamp in milliseconds)
      pub start_time: Option<i64>,
      /// End time (Unix timestamp in milliseconds)
      pub end_time: Option<i64>,
      /// Filter pattern (CloudWatch Logs filter syntax)
      pub filter_pattern: Option<String>,
      /// Maximum number of events to return
      pub limit: Option<i32>,
      /// Log stream names to query (empty = all streams)
      pub log_stream_names: Vec<String>,
      /// Whether to query in reverse chronological order
      pub start_from_head: bool,
  }
  ```
- **Acceptance Criteria**:
  - Can query logs with time range
  - Supports filter patterns
  - Handles pagination for large result sets
  - Returns structured log events with timestamps

#### M1.T3: Add CloudWatch Logs Data Models
- **File**: `src/app/cloudwatch_logs/types.rs`
- **Structures**:
  ```rust
  pub struct LogQueryResult {
      pub events: Vec<LogEvent>,
      pub next_token: Option<String>,
      pub total_events: usize,
      pub query_statistics: QueryStatistics,
  }

  pub struct LogEvent {
      pub timestamp: i64,
      pub message: String,
      pub ingestion_time: i64,
      pub log_stream_name: String,
  }

  pub struct QueryStatistics {
      pub bytes_scanned: f64,
      pub records_matched: f64,
      pub records_scanned: f64,
  }
  ```
- **Acceptance Criteria**:
  - Serializable to JSON for V8 binding
  - Deserializable from AWS SDK responses
  - Proper Debug and Clone implementations

#### M1.T4: Unit Tests for CloudWatch Logs Client
- **File**: `tests/cloudwatch_logs_test.rs`
- **Coverage**:
  - Client creation with credentials
  - Query options validation
  - Time range calculations
  - Filter pattern handling
- **Acceptance Criteria**:
  - All unit tests pass
  - Integration test with mock AWS responses

---

## Milestone 2: Resource Explorer Integration

**Goal**: Add CloudWatch Logs viewing capability to Resource Explorer

### Tasks

#### M2.T1: Identify Resources with CloudWatch Logs
- **File**: `src/app/resource_explorer/resource_metadata.rs`
- **Logic**:
  - Determine which AWS resource types have associated log groups
  - Map resource types to log group naming patterns
  - Examples:
    - Lambda Function → `/aws/lambda/{function-name}`
    - API Gateway → `/aws/apigateway/{api-name}`
    - ECS Task → `/ecs/{cluster}/{task-definition}`
    - RDS → `/aws/rds/instance/{instance-id}/error`
- **Acceptance Criteria**:
  - Comprehensive mapping of resource types to log group patterns
  - Helper function to determine if resource has logs
  - Function to generate log group name from resource ARN/name

#### M2.T2: Add "View Logs" Button to Resource Details
- **File**: `src/app/dashui/resource_explorer/resource_detail_panel.rs`
- **UI Changes**:
  - Add "View Logs" button next to existing action buttons
  - Only show button for resources with associated log groups
  - Button opens CloudWatch Logs window
- **Acceptance Criteria**:
  - Button appears for supported resource types only
  - Button click triggers log window creation
  - Disabled state when no logs available

#### M2.T3: Create CloudWatch Logs Viewer Window
- **File**: `src/app/dashui/cloudwatch_logs_window.rs`
- **Features**:
  - Implements `FocusableWindow` trait
  - Displays log events in scrollable list (max 1000 events)
  - Shows timestamp, log stream name, and message
  - Fuzzy search filter (reuses Resource Explorer search pattern matching)
  - Manual refresh button
  - Export logs to file button
- **UI Layout**:
  ```
  ┌─────────────────────────────────────────────────┐
  │ CloudWatch Logs: {resource-name}           [X]  │
  ├─────────────────────────────────────────────────┤
  │ Log Group: /aws/lambda/my-function              │
  │ ┌───────────────────────────────────────────┐   │
  │ │ Search (fuzzy): [______________] [Refresh]│   │
  │ └───────────────────────────────────────────┘   │
  │                                                  │
  │ ┌────────────────────────────────────────────┐  │
  │ │ 2025-11-16 10:23:45.123 [stream-1]        │  │
  │ │ START RequestId: abc-123                   │  │
  │ │                                            │  │
  │ │ 2025-11-16 10:23:45.234 [stream-1]        │  │
  │ │ Processing event: {...}                    │  │
  │ │                                            │  │
  │ │ 2025-11-16 10:23:45.456 [stream-1]        │  │
  │ │ END RequestId: abc-123                     │  │
  │ │                                            │  │
  │ └────────────────────────────────────────────┘  │
  │                                                  │
  │ Showing 100 of 100 events (latest)              │
  │ [Export to File]                                │
  └─────────────────────────────────────────────────┘
  ```
- **Acceptance Criteria**:
  - Window renders without performance issues
  - Logs update on refresh
  - Filter pattern works correctly
  - Export saves logs to file

#### M2.T4: Get Latest Log Events
- **File**: `src/app/cloudwatch_logs/recent_logs.rs`
- **Logic**:
  ```rust
  pub async fn get_latest_log_events(
      client: &Client,
      log_group_name: &str,
      limit: i32,
  ) -> Result<LogQueryResult> {
      // Get the most recent N events (default 100)
      query_log_events(client, log_group_name,
          QueryOptions::new()
              .with_limit(limit)
              .start_from_head(false) // Most recent first
      ).await
  }
  ```
- **Acceptance Criteria**:
  - Returns latest N events (default 100)
  - Handles empty log groups gracefully
  - Simple, predictable behavior

---

## Milestone 3: Agent V2 JavaScript Binding

**Goal**: Expose CloudWatch Logs querying to Agent V2 via V8 JavaScript API

### Tasks

#### M3.T1: Create V8 Binding for queryCloudWatchLogEvents()
- **File**: `src/app/agent_framework/v8_bindings/cloudwatch_logs.rs`
- **Function Signature**:
  ```javascript
  /**
   * Query CloudWatch Logs for a specific log group
   *
   * @param {Object} params - Query parameters
   * @param {string} params.logGroupName - Name of the log group (required)
   * @param {string} [params.region] - AWS region (default: us-east-1)
   * @param {number} [params.startTime] - Start time (Unix ms timestamp)
   * @param {number} [params.endTime] - End time (Unix ms timestamp)
   * @param {string} [params.filterPattern] - CloudWatch Logs filter pattern
   * @param {number} [params.limit] - Max events to return (default: 100, max: 10000)
   *   Note: Limit maps to AWS SDK's 'limit' parameter which controls max events returned
   * @param {string[]} [params.logStreamNames] - Specific streams to query
   * @param {boolean} [params.startFromHead] - Query chronologically (default: false)
   *
   * @returns {Object} Query result
   * @returns {Array<Object>} result.events - Log events
   * @returns {number} result.events[].timestamp - Event timestamp (Unix ms)
   * @returns {string} result.events[].message - Log message
   * @returns {string} result.events[].logStreamName - Stream name
   * @returns {string|null} result.nextToken - Pagination token
   * @returns {Object} result.statistics - Query statistics
   *
   * @example
   * // Get latest 100 events from Lambda logs
   * const logs = queryCloudWatchLogEvents({
   *   logGroupName: "/aws/lambda/my-function",
   *   limit: 100
   * });
   *
   * @example
   * // Query with filter pattern
   * const errors = queryCloudWatchLogEvents({
   *   logGroupName: "/aws/lambda/my-function",
   *   filterPattern: '[level=ERROR]',
   *   limit: 100
   * });
   */
  function queryCloudWatchLogEvents(params) { }
  ```
- **Implementation**:
  ```rust
  pub fn register_cloudwatch_logs_functions(scope: &mut v8::HandleScope) {
      // Register queryCloudWatchLogEvents function
      register_v8_function(scope, "queryCloudWatchLogEvents",
          query_cloudwatch_log_events_impl);
  }

  fn query_cloudwatch_log_events_impl(
      scope: &mut v8::HandleScope,
      args: v8::FunctionCallbackArguments,
      mut ret: v8::ReturnValue,
  ) {
      // 1. Parse parameters from V8 object
      // 2. Get AWS credentials from global context
      // 3. Create CloudWatch Logs client
      // 4. Execute query asynchronously
      // 5. Convert result to V8 object
      // 6. Return to JavaScript
  }
  ```
- **Acceptance Criteria**:
  - Function callable from JavaScript
  - Parameters validated correctly
  - Async execution doesn't block V8
  - Results properly serialized to JavaScript objects
  - Errors propagated as JavaScript exceptions

#### M3.T2: Add Helper Functions
- **Additional V8 Functions**:
  ```javascript
  /**
   * List all log groups in a region
   *
   * @param {Object} params
   * @param {string} [params.region] - AWS region
   * @param {string} [params.prefix] - Log group name prefix filter
   * @returns {Array<Object>} Log groups
   */
  function listLogGroups(params) { }

  /**
   * List log streams in a log group
   *
   * @param {Object} params
   * @param {string} params.logGroupName - Log group name
   * @param {string} [params.region] - AWS region
   * @returns {Array<Object>} Log streams
   */
  function listLogStreams(params) { }

  /**
   * Get recent logs (last 5 min or last available)
   * Convenience wrapper around queryCloudWatchLogEvents
   *
   * @param {Object} params
   * @param {string} params.logGroupName - Log group name
   * @param {string} [params.region] - AWS region
   * @returns {Object} Query result
   */
  function getRecentLogs(params) { }
  ```
- **Acceptance Criteria**:
  - All helper functions work as documented
  - Properly integrated with main query function
  - Error handling consistent across functions

#### M3.T3: Update Agent V2 System Prompt with API Documentation
- **File**: `src/app/agent_framework/v8_bindings/mod.rs`
- **Documentation to Add**:
  ```markdown
  ## CloudWatch Logs Functions

  ### queryCloudWatchLogEvents(params)

  Query CloudWatch Logs for analysis and monitoring.

  **Parameters:**
  - `logGroupName` (string, required): Log group to query
  - `region` (string, optional): AWS region (default: us-east-1)
  - `startTime` (number, optional): Start time in Unix milliseconds
  - `endTime` (number, optional): End time in Unix milliseconds
  - `filterPattern` (string, optional): CloudWatch Logs filter syntax
  - `limit` (number, optional): Max events (default: 100, max: 10000)
  - `logStreamNames` (array, optional): Specific streams to query
  - `startFromHead` (boolean, optional): Chronological order

  **Returns:** Object with `events`, `nextToken`, `statistics`

  **Common Use Cases:**

  1. **Find errors in Lambda function:**
  ```javascript
  const errors = queryCloudWatchLogEvents({
    logGroupName: "/aws/lambda/my-function",
    filterPattern: '[level=ERROR]',
    startTime: Date.now() - (1 * 60 * 60 * 1000), // Last hour
    limit: 100
  });
  ```

  2. **Analyze application logs:**
  ```javascript
  const logs = queryCloudWatchLogEvents({
    logGroupName: "/aws/ecs/my-app",
    startTime: Date.now() - (15 * 60 * 1000), // Last 15 min
    filterPattern: '[timestamp, request_id, level=INFO, msg]'
  });

  // Process logs with JavaScript
  const requestCounts = {};
  logs.events.forEach(event => {
    const match = event.message.match(/request_id=(\w+)/);
    if (match) {
      requestCounts[match[1]] = (requestCounts[match[1]] || 0) + 1;
    }
  });
  ```

  3. **Monitor API Gateway:**
  ```javascript
  const apiLogs = queryCloudWatchLogEvents({
    logGroupName: "/aws/apigateway/my-api",
    filterPattern: '[ip, timestamp, method, path, status>=400]',
    limit: 500
  });

  // Analyze error rates
  const errorRate = apiLogs.events.length / apiLogs.statistics.recordsScanned;
  ```

  ### getRecentLogs(params)

  Get the most recent logs (last 5 minutes or last available if none).

  **Parameters:**
  - `logGroupName` (string, required): Log group to query
  - `region` (string, optional): AWS region

  **Example:**
  ```javascript
  const recent = getRecentLogs({
    logGroupName: "/aws/lambda/my-function"
  });
  ```

  ### Filter Pattern Syntax

  CloudWatch Logs supports powerful filter patterns:

  - **Simple text:** `ERROR` (matches lines containing "ERROR")
  - **JSON fields:** `{ $.level = "ERROR" }` (JSON logs)
  - **Structured:** `[timestamp, request_id, level, msg]`
  - **Conditions:** `[level=ERROR || level=FATAL]`
  - **Numeric:** `[..., status>=400, ...]`

  **Important Notes:**
  - Queries are limited to 10,000 events maximum
  - Large time ranges may be slow - be specific
  - Use filter patterns to reduce data scanned
  - Timestamps are in Unix milliseconds (Date.now())
  ```
- **Acceptance Criteria**:
  - Documentation added to API docs string
  - LLM receives documentation in system prompt
  - Examples cover common use cases

#### M3.T4: Register Functions with V8 Runtime
- **File**: `src/app/agent_framework/v8_bindings/mod.rs`
- **Changes**:
  ```rust
  pub fn get_api_documentation() -> String {
      format!(
          "{}\n\n{}",
          get_existing_api_docs(),
          get_cloudwatch_logs_api_docs()
      )
  }

  pub fn register_all_functions(scope: &mut v8::HandleScope) {
      register_aws_functions(scope);
      register_cloudwatch_logs_functions(scope); // NEW
  }
  ```
- **Acceptance Criteria**:
  - Functions available in JavaScript runtime
  - API documentation included in system prompt
  - No conflicts with existing functions

---

## Milestone 4: Testing & Documentation

**Goal**: Comprehensive testing and user documentation

### Tasks

#### M4.T1: Unit Tests
- **Coverage**:
  - CloudWatch Logs client creation
  - Query parameter validation
  - Time range handling
  - Filter pattern parsing
  - V8 binding parameter conversion
  - Error handling and edge cases
- **Acceptance Criteria**:
  - 90%+ code coverage for new modules
  - All edge cases tested
  - Mock AWS responses for integration tests

#### M4.T2: Integration Tests
- **File**: `tests/cloudwatch_logs_integration_test.rs`
- **Scenarios**:
  - Query real log group (with test credentials)
  - Handle pagination for large results
  - Filter pattern application
  - Time range queries
  - Empty log group handling
  - Invalid log group name handling
- **Acceptance Criteria**:
  - Tests pass with real AWS credentials
  - Tests use dedicated test log group
  - Cleanup after test execution

#### M4.T3: Agent V2 End-to-End Test
- **File**: `tests/agent_v2_cloudwatch_logs_test.rs`
- **Scenario**:
  ```javascript
  // Agent receives query: "Show me errors from my Lambda function in the last hour"
  // Expected JavaScript code generated:
  const errors = queryCloudWatchLogEvents({
    logGroupName: "/aws/lambda/my-function",
    filterPattern: "ERROR",
    startTime: Date.now() - (60 * 60 * 1000),
    limit: 100
  });

  errors.events.forEach(e => {
    console.log(`${new Date(e.timestamp).toISOString()}: ${e.message}`);
  });
  ```
- **Acceptance Criteria**:
  - Agent successfully calls queryCloudWatchLogEvents
  - Results properly formatted in response
  - Error handling works correctly

#### M4.T4: UI Testing
- **File**: `tests/cloudwatch_logs_window_ui_test.rs`
- **Coverage**:
  - Window opens from Resource Explorer
  - Logs display correctly
  - Filter functionality works
  - Time range selector works
  - Refresh updates logs
  - Export saves file
- **Acceptance Criteria**:
  - All UI interactions tested
  - No performance regressions
  - Window resizing works correctly

#### M4.T5: User Documentation
- **File**: `docs/features/cloudwatch-logs.md`
- **Sections**:
  - Overview of feature
  - Using from Resource Explorer
  - Using from Agent V2
  - JavaScript API reference
  - Filter pattern syntax guide
  - Common use cases and examples
  - Troubleshooting
  - Performance considerations
- **Acceptance Criteria**:
  - Documentation complete and accurate
  - Screenshots included for UI features
  - Code examples tested and working

---

## Milestone 5: Performance & Polish

**Goal**: Optimize performance and add quality-of-life improvements

### Tasks

#### M5.T1: Implement Result Caching
- **File**: `src/app/cloudwatch_logs/cache.rs`
- **Features**:
  - Cache recent query results (5-minute TTL)
  - Cache log group metadata (30-minute TTL)
  - LRU eviction policy
  - Memory usage limits
- **Acceptance Criteria**:
  - Repeated queries use cache
  - Cache invalidation works correctly
  - Memory usage stays under limit

#### M5.T2: Add Pagination Support
- **File**: `src/app/cloudwatch_logs/pagination.rs`
- **Features**:
  - Handle nextToken for large result sets
  - Auto-pagination option for V8 binding
  - Progress indicator in UI
  - Cancel long-running queries
- **Acceptance Criteria**:
  - Large queries don't timeout
  - UI remains responsive during pagination
  - Users can cancel queries

#### M5.T3: Optimize UI Rendering
- **Improvements**:
  - Virtual scrolling for large log lists
  - Syntax highlighting for JSON logs
  - Collapsible log entries
  - Search highlighting
  - Copy to clipboard button per entry
- **Acceptance Criteria**:
  - Smooth scrolling with 10,000+ events
  - Syntax highlighting doesn't slow rendering
  - Memory usage reasonable for large logs

#### M5.T4: Add Error Recovery
- **Features**:
  - Retry logic for transient failures
  - Graceful degradation for missing permissions
  - User-friendly error messages
  - Logging of errors for debugging
- **Acceptance Criteria**:
  - Transient errors auto-retry
  - Permission errors show helpful message
  - No crashes on error conditions

---

## Implementation Order

### Phase 1: Foundation (Week 1-2)
- **M1**: CloudWatch Logs Client Foundation
  - Critical for both Explorer and Agent
  - Can be developed and tested independently
  - Blocks all other work

### Phase 2: Explorer Integration (Week 2-3)
- **M2**: Resource Explorer Integration
  - User-facing feature
  - Provides immediate value
  - Validates CloudWatch Logs client

### Phase 3: Agent Integration (Week 3-4)
- **M3**: Agent V2 JavaScript Binding
  - Builds on M1 foundation
  - More complex due to V8 integration
  - Requires careful API design

### Phase 4: Quality (Week 4-5)
- **M4**: Testing & Documentation
  - Parallel with M3 development
  - Ensures production readiness

### Phase 5: Optimization (Week 5-6)
- **M5**: Performance & Polish
  - Based on initial usage feedback
  - Nice-to-have improvements

---

## Dependencies

### External Crates
```toml
[dependencies]
aws-sdk-cloudwatchlogs = "1.x"  # AWS CloudWatch Logs SDK
```

### Internal Dependencies
- AWS Identity Center (existing)
- V8 JavaScript runtime (existing)
- Resource Explorer (existing)
- Agent V2 framework (existing)

---

## Success Criteria

### Functional Requirements
- ✅ Users can view CloudWatch Logs from Resource Explorer
- ✅ Agent V2 can query and analyze logs via JavaScript
- ✅ "Last 5 minutes or last available" logic works correctly
- ✅ Filter patterns apply correctly
- ✅ Pagination handles large result sets

### Performance Requirements
- ✅ UI remains responsive during log loading
- ✅ Queries complete within 5 seconds for typical use cases
- ✅ Memory usage under 100MB for 10,000 log events

### Quality Requirements
- ✅ 90%+ code coverage for new modules
- ✅ All integration tests pass
- ✅ Zero crashes in error conditions
- ✅ Complete user documentation

---

## Risk Mitigation

### Risk: AWS API Rate Limits
- **Mitigation**: Implement result caching, rate limiting, batching

### Risk: Large Log Volume Performance
- **Mitigation**: Virtual scrolling, pagination, query limits, time range constraints

### Risk: V8 Async Complexity
- **Mitigation**: Use tokio runtime properly, test thoroughly, clear error messages

### Risk: Permission Issues
- **Mitigation**: Clear error messages, permission checker, documentation

---

## Future Enhancements (Post-MVP)

1. **CloudWatch Logs Insights Integration**
   - Support CloudWatch Logs Insights query language
   - Visual query builder
   - Saved queries

2. **Real-time Log Streaming**
   - Tail logs in real-time
   - Auto-scroll to latest
   - Desktop notifications for errors

3. **Log Analytics Dashboard**
   - Pre-built charts and metrics
   - Custom dashboards
   - Anomaly detection

4. **Multi-Log Group Queries**
   - Query across multiple log groups
   - Correlation and aggregation
   - Cross-region queries

5. **Export Enhancements**
   - Export to S3
   - Export to external analytics tools
   - Scheduled exports
