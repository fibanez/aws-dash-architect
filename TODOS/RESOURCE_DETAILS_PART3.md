# AWS Resource Details Implementation - Part 3: Analytics & Data Plane

## Overview

This document covers the implementation of missing detail-getting API functions for 8 AWS services focused on analytics capabilities, including **two full data plane implementations** for Athena (log querying) and CloudWatch (metrics).

**Services**: Athena (FULL DATA PLANE), CloudWatch (FULL DATA PLANE), OpenSearch, QuickSight, Redshift, Route53, SSM, BedrockAgentCore

---

## IMPORTANT: Two-Phase Loading Pattern

**All new services MUST implement two-phase loading from the start.**

> **Note**: The orchestration layer for two-phase loading is **already fully implemented** in PART1.
> New services only need to implement:
> 1. `include_details: bool` parameter on list functions
> 2. `get_*_details()` function for Phase 2 enrichment
> 3. Status bar reporting via `report_status()` / `report_status_done()`
>
> See `RESOURCE_DETAILS_PART1.md` for complete orchestration documentation including:
> - Key Files Modified (aws_client.rs, window.rs, state.rs)
> - Data Flow diagram
> - Implementation Pattern with detailed code examples

### Pattern for List Functions

```rust
pub async fn list_resources(
    &self,
    account_id: &str,
    region: &str,
    include_details: bool,  // REQUIRED PARAMETER
) -> Result<Vec<serde_json::Value>> {
    report_status("ServiceName", "list_resources", Some(region));

    // Basic list operation
    let response = client.list_resources().send().await?;

    for resource in resources {
        let mut json = resource_to_json(&resource);

        // Only fetch details if requested (Phase 2)
        if include_details {
            // All detail API calls here
            if let Ok(config) = self.get_config_internal(&client, &id).await {
                json.insert("Configuration".to_string(), config);
            }
            // ... more detail calls
        }

        results.push(json);
    }

    report_status_done("ServiceName", "list_resources", Some(region));
    Ok(results)
}
```

### Pattern for Detail Functions (Phase 2 Enrichment)

```rust
/// Get details for a single resource (for Phase 2 background enrichment)
pub async fn get_resource_details(
    &self,
    account_id: &str,
    region: &str,
    resource_id: &str,
) -> Result<serde_json::Value> {
    report_status("ServiceName", "get_resource_details", Some(resource_id));

    let mut details = serde_json::Map::new();

    // All detail API calls for single resource
    if let Ok(config) = self.get_config_internal(&client, resource_id).await {
        details.insert("Configuration".to_string(), config);
    }
    // ... more detail calls

    report_status_done("ServiceName", "get_resource_details", Some(resource_id));
    Ok(serde_json::Value::Object(details))
}
```

### Architecture Overview

1. **Phase 1 (`include_details: false`)**: UI updates immediately with basic resource info
2. **Phase 2 (Background Enrichment)**: Details fetched via `get_*_details()` functions
3. **Automatic Orchestration**: Phase 2 triggers automatically after Phase 1 completes
4. **Status Bar Integration**: Both phases report to status bar for user visibility

### Required Integration Steps (per service)

When implementing two-phase loading for a new service, you must complete these integration steps in `aws_client.rs`:

1. **Update list call** - Add `include_details: false` to the list function call in `list_resources_for_type()`
2. **Add to `fetch_resource_details()`** - Add match arm calling `get_*_details()` function
3. **Add to `enrichable_types`** - **CRITICAL**: Add resource type string to the `enrichable_types` array in `start_phase2_enrichment()` (around line 3756). Without this, Phase 2 will skip the resource type entirely!

```rust
// In start_phase2_enrichment(), add your resource type:
let enrichable_types = [
    "AWS::Lambda::Function",
    // ... existing types ...
    "AWS::YourService::ResourceType",  // <-- Add here
];
```

---

## Milestone 17: Athena - FULL DATA PLANE IMPLEMENTATION

### Part A: Control Plane Enhancements

**File**: `src/app/resource_explorer/aws_services/athena.rs`

#### Tasks

- [ ] **17.1** Add `get_work_group()` function
  - API: `get_work_group`
  - Purpose: Get workgroup configuration
  - Returns: Output location, encryption config, bytes scanned cutoff

- [ ] **17.2** Add `list_data_catalogs()` function
  - API: `list_data_catalogs`
  - Purpose: List available data catalogs
  - Returns: Catalog names and types (GLUE, LAMBDA, HIVE)

- [ ] **17.3** Add `list_databases()` function
  - API: `list_databases`
  - Purpose: List databases in catalog
  - Returns: Database names and descriptions

- [ ] **17.4** Add `list_table_metadata()` function
  - API: `list_table_metadata`
  - Purpose: List tables in database
  - Returns: Table names, types, columns

- [ ] **17.5** Add `get_table_metadata()` function
  - API: `get_table_metadata`
  - Purpose: Get table schema details
  - Returns: Column names, types, partition keys

### Part B: Data Plane Implementation

**Files**: NEW `src/app/data_plane/athena/`

Reference implementations:
- `src/app/data_plane/cloudwatch_logs/`
- `src/app/data_plane/cloudtrail_events/`

#### Tasks

- [ ] **17.6** Create `src/app/data_plane/athena/mod.rs`
  - Module documentation and exports
  - Re-export client, types, resource_mapping

- [ ] **17.7** Create `src/app/data_plane/athena/client.rs`
  - AWS SDK wrapper with credential management
  - Arc<CredentialCoordinator> integration

- [ ] **17.8** Add `start_query_execution()` function
  - API: `start_query_execution`
  - Purpose: Execute SQL query on S3 data (CloudTrail, ALB logs, etc.)
  - Parameters: query_string, database, workgroup, output_location
  - Returns: query_execution_id

- [ ] **17.9** Add `get_query_execution()` function
  - API: `get_query_execution`
  - Purpose: Poll query status
  - Returns: Status (QUEUED/RUNNING/SUCCEEDED/FAILED), completion time, bytes scanned

- [ ] **17.10** Add `get_query_results()` function
  - API: `get_query_results`
  - Purpose: Get result rows with pagination
  - Returns: Column info, rows, next_token

- [ ] **17.11** Add `stop_query_execution()` function
  - API: `stop_query_execution`
  - Purpose: Cancel running query
  - Returns: Success/failure

- [ ] **17.12** Add `list_query_executions()` function
  - API: `list_query_executions`
  - Purpose: Get query history
  - Returns: Query IDs with pagination

- [ ] **17.13** Create `src/app/data_plane/athena/types.rs`
  - `QueryOptions` struct (query, database, workgroup, timeout)
  - `QueryResult` struct (columns, rows, statistics)
  - `QueryStatus` enum (Queued, Running, Succeeded, Failed, Cancelled)
  - `AthenaColumn` struct (name, type)
  - `AthenaRow` struct (data values)

- [ ] **17.14** Create `src/app/data_plane/athena/resource_mapping.rs`
  - Map resource types to common log tables
  - CloudTrail logs table mapping
  - ALB/NLB access logs mapping
  - S3 access logs mapping
  - VPC Flow Logs mapping

### Part C: V8 JavaScript Bindings

**File**: NEW `src/app/agent_framework/v8_bindings/bindings/athena.rs`

- [ ] **17.15** Create Athena V8 binding
  - Expose `start_query()` to JavaScript
  - Expose `get_query_status()` to JavaScript
  - Expose `get_query_results()` to JavaScript
  - Add LLM documentation for tool discovery

### Part D: UI Window

**File**: NEW `src/app/dashui/athena_query_window.rs`

- [ ] **17.16** Create `AthenaQueryWindow` struct
  - Implement `FocusableWindow` trait
  - Query input text area
  - Workgroup selector dropdown
  - Database/table browser

- [ ] **17.17** Add query execution UI
  - Execute button with loading state
  - Cancel button for running queries
  - Progress indicator with bytes scanned

- [ ] **17.18** Add results viewer
  - Table display for query results
  - Column sorting
  - CSV export option
  - Pagination for large results

- [ ] **17.19** Add query history panel
  - Recent queries list
  - Re-run previous queries
  - Query execution statistics

- [ ] **17.20** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md`
  - Create `docs/technical/athena-data-plane.md`

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Athena Full Data Plane

**Prerequisites:**
- Have at least one AWS account connected with Athena access
- Have an existing Athena workgroup configured
- Have at least one database and table in Glue Data Catalog (or Athena-created)
- Optional: Have CloudTrail or ALB logs stored in S3 for querying

**Part A - Control Plane Testing:**

1. **Test workgroup details**
   - Open Resource Explorer and navigate to Athena > WorkGroups
   - Select a workgroup and view details panel
   - Verify: Output location, encryption config, and bytes scanned cutoff are displayed

2. **Test data catalog listing**
   - In the same view, check for data catalogs section
   - Verify: Catalogs show names and types (GLUE, LAMBDA, HIVE)

3. **Test database listing**
   - Select a data catalog
   - Verify: Databases are listed with descriptions

4. **Test table metadata**
   - Select a database
   - Verify: Tables are listed with column information

**Part B - Data Plane Testing:**

5. **Open Athena Query Window**
   - Press `Space` to open Command Palette
   - Search for "Athena Query" and select it
   - Verify: Window opens with query input area

6. **Test query execution**
   - Select a workgroup from dropdown
   - Select a database from dropdown
   - Enter a simple query: `SELECT * FROM your_table LIMIT 10`
   - Click Execute
   - Verify: Loading indicator appears with "Query Running..." status

7. **Test query status polling**
   - While query runs, observe status updates
   - Verify: Status shows QUEUED -> RUNNING -> SUCCEEDED
   - Verify: Bytes scanned counter updates

8. **Test query results display**
   - After query completes, check results panel
   - Verify: Column headers match table schema
   - Verify: Data rows are displayed correctly
   - Verify: Pagination works for large results (if applicable)

9. **Test query cancellation**
   - Start a long-running query (e.g., large table scan)
   - Click Cancel button
   - Verify: Query status changes to CANCELLED

10. **Test query history**
    - Check query history panel
    - Verify: Recent queries are listed with execution time
    - Click on a previous query to re-run
    - Verify: Query text is populated and can be executed

**Part C - V8 Bindings Testing (Agent):**

11. **Test Athena bindings from agent**
    - Create or open an Agent window
    - Ask: "Query my Athena database [database_name] with: SELECT 1"
    - Verify: Agent successfully executes query via V8 bindings
    - Verify: Results are returned to agent

**Expected Behaviors:**
- Queries against CloudTrail logs return security events
- Query results support CSV export
- Large result sets paginate correctly
- Network errors show helpful messages

---

## Milestone 18: CloudWatch - FULL DATA PLANE IMPLEMENTATION (Metrics)

### Part A: Control Plane Enhancements

**File**: `src/app/resource_explorer/aws_services/cloudwatch.rs`

#### Tasks

- [ ] **18.1** Add `list_metrics()` function
  - API: `list_metrics`
  - Purpose: List available metrics
  - Returns: Namespace, metric name, dimensions

- [ ] **18.2** Add `describe_anomaly_detectors()` function
  - API: `describe_anomaly_detectors`
  - Purpose: Get anomaly detectors
  - Returns: Metric name, stat, configuration

### Part B: Data Plane Implementation

**Files**: NEW `src/app/data_plane/cloudwatch_metrics/`

#### Tasks

- [ ] **18.3** Create `src/app/data_plane/cloudwatch_metrics/mod.rs`
  - Module documentation and exports

- [ ] **18.4** Create `src/app/data_plane/cloudwatch_metrics/client.rs`
  - AWS SDK wrapper with credential management

- [ ] **18.5** Add `get_metric_statistics()` function
  - API: `get_metric_statistics`
  - Purpose: Get historical statistics (Sum, Average, Max, Min, SampleCount)
  - Parameters: namespace, metric_name, dimensions, start_time, end_time, period
  - Returns: Datapoints with timestamps and values

- [ ] **18.6** Add `get_metric_data()` function
  - API: `get_metric_data`
  - Purpose: Get time-series data for multiple metrics (batch)
  - Parameters: Metric queries with math expressions
  - Returns: Time-series results with labels

- [ ] **18.7** Add `get_metric_widget_image()` function
  - API: `get_metric_widget_image`
  - Purpose: Get dashboard widget as PNG image
  - Returns: Base64 encoded image

- [ ] **18.8** Create `src/app/data_plane/cloudwatch_metrics/types.rs`
  - `MetricQuery` struct (namespace, name, dimensions, stat, period)
  - `MetricDataResult` struct (id, label, timestamps, values)
  - `Datapoint` struct (timestamp, value, unit)
  - `MetricDimension` struct (name, value)
  - `TimeRange` struct (start, end)

### Part C: V8 JavaScript Bindings

**File**: NEW `src/app/agent_framework/v8_bindings/bindings/cloudwatch_metrics.rs`

- [ ] **18.9** Create CloudWatch Metrics V8 binding
  - Expose `get_metric_statistics()` to JavaScript
  - Expose `get_metric_data()` to JavaScript
  - Add LLM documentation

### Part D: UI Window

**File**: NEW `src/app/dashui/cloudwatch_metrics_window.rs`

- [ ] **18.10** Create `CloudWatchMetricsWindow` struct
  - Implement `FocusableWindow` trait
  - Metric namespace browser
  - Metric name selector

- [ ] **18.11** Add metric selector UI
  - Namespace dropdown
  - Metric search/filter
  - Dimension selectors

- [ ] **18.12** Add time range picker
  - Preset ranges (1h, 6h, 24h, 7d, 30d)
  - Custom date/time picker
  - Relative time options

- [ ] **18.13** Add graph visualization
  - Line chart for time-series data
  - Multiple metrics overlay
  - Zoom and pan controls
  - Statistics display (avg, min, max)

- [ ] **18.14** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md`
  - Create `docs/technical/cloudwatch-metrics-data-plane.md`

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: CloudWatch Full Data Plane (Metrics)

**Prerequisites:**
- Have at least one AWS account connected with CloudWatch access
- Have some resources generating CloudWatch metrics (EC2, Lambda, etc.)
- Optional: Have CloudWatch alarms configured for testing metric context

**Part A - Control Plane Testing:**

1. **Test metrics listing**
   - Open Resource Explorer and navigate to CloudWatch > Metrics
   - Verify: Metrics are listed with namespace, name, and dimensions
   - Verify: Common namespaces appear (AWS/EC2, AWS/Lambda, AWS/RDS, etc.)

2. **Test anomaly detectors**
   - If anomaly detectors are configured, view them in the list
   - Verify: Metric name, statistic, and configuration are displayed

**Part B - Data Plane Testing:**

3. **Open CloudWatch Metrics Window**
   - Press `Space` to open Command Palette
   - Search for "CloudWatch Metrics" and select it
   - Verify: Window opens with namespace browser

4. **Test namespace browser**
   - Browse through available namespaces
   - Verify: Namespaces are grouped logically (AWS services, custom)
   - Select a namespace (e.g., AWS/EC2)
   - Verify: Metrics for that namespace are shown

5. **Test metric selection**
   - Search/filter for a specific metric (e.g., "CPUUtilization")
   - Verify: Search results update in real-time
   - Select the metric
   - Verify: Available dimensions are shown (InstanceId, etc.)

6. **Test dimension filtering**
   - Select specific dimension values (e.g., a specific EC2 instance)
   - Verify: Graph updates to show data for selected dimensions

7. **Test time range picker**
   - Click preset ranges: 1h, 6h, 24h, 7d, 30d
   - Verify: Graph updates with appropriate time scale
   - Test custom date/time picker
   - Verify: Selected range is applied correctly

8. **Test graph visualization**
   - Verify: Line chart displays time-series data
   - Verify: Y-axis shows appropriate scale and units
   - Verify: X-axis shows timestamps correctly
   - Test zoom by selecting an area on the graph
   - Test pan by dragging the graph

9. **Test multiple metrics**
   - Add a second metric to the same graph
   - Verify: Both metrics display with different colors
   - Verify: Legend shows metric names and colors

10. **Test statistics display**
    - Check for statistics summary (avg, min, max, sum)
    - Verify: Values are calculated correctly for visible range

**Part C - V8 Bindings Testing (Agent):**

11. **Test CloudWatch metrics from agent**
    - Create or open an Agent window
    - Ask: "Show me the CPU utilization for my EC2 instances over the last hour"
    - Verify: Agent successfully retrieves metric data via V8 bindings
    - Verify: Results are summarized or presented to user

**Expected Behaviors:**
- Metric graphs render smoothly without UI lag
- Time range changes update graph quickly
- Multiple metrics can be compared on same graph
- Statistics are accurate for displayed time range
- Network errors show helpful messages

---

## Milestone 19: OpenSearch Domain Details

**File**: `src/app/resource_explorer/aws_services/opensearch.rs`

### Tasks

- [ ] **19.1** Add `describe_domain()` function
  - API: `describe_domain`
  - Purpose: Get domain configuration
  - Returns: Cluster config, EBS options, access policies, endpoints

- [ ] **19.2** Add `describe_domain_config()` function
  - API: `describe_domain_config`
  - Purpose: Get detailed configuration
  - Returns: All domain settings with change status

- [ ] **19.3** Add `list_tags()` function
  - API: `list_tags`
  - Purpose: Get domain tags
  - Returns: Tag key-value pairs

- [ ] **19.4** Add `describe_domain_health()` function
  - API: `describe_domain_health`
  - Purpose: Get health status
  - Returns: Cluster health, node count, storage

- [ ] **19.5** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new OpenSearch calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: OpenSearch Domain Details

**Prerequisites:**
- Have at least one AWS account connected with OpenSearch access
- Have at least one OpenSearch domain deployed

**Testing Steps:**

1. **Locate OpenSearch domain**
   - Open Resource Explorer
   - Navigate to OpenSearch > Domains
   - Verify: Domain(s) appear in the list

2. **Test domain configuration details**
   - Select an OpenSearch domain
   - View the details panel
   - Verify: Cluster configuration is displayed (instance type, instance count)
   - Verify: EBS options are shown (volume type, size)
   - Verify: Access policies are displayed
   - Verify: Domain endpoint URLs are shown

3. **Test detailed configuration**
   - Check for "Configuration" tab or section
   - Verify: All domain settings are displayed
   - Verify: Change status is shown for recent modifications

4. **Test tags display**
   - Scroll to Tags section in details panel
   - Verify: All tags are displayed as key-value pairs
   - Verify: Tags can be copied/selected

5. **Test domain health**
   - Check for health status indicator
   - Verify: Cluster health (green/yellow/red) is displayed
   - Verify: Node count is shown
   - Verify: Storage utilization is displayed

**Expected Behaviors:**
- Domain configuration loads within a few seconds
- Health status accurately reflects domain state
- Access policies are formatted for readability
- All tags are retrieved and displayed

---

## Milestone 20: QuickSight Analytics Details

**File**: `src/app/resource_explorer/aws_services/quicksight.rs`

### Tasks

- [ ] **20.1** Add `describe_data_source()` function
  - API: `describe_data_source`
  - Purpose: Get data source configuration
  - Returns: Connection parameters, credentials, VPC config

- [ ] **20.2** Add `describe_dashboard()` function
  - API: `describe_dashboard`
  - Purpose: Get dashboard details
  - Returns: Version, sheets, parameters

- [ ] **20.3** Add `describe_data_set()` function
  - API: `describe_data_set`
  - Purpose: Get dataset configuration
  - Returns: Physical table map, logical table map, columns

- [ ] **20.4** Add `describe_analysis()` function
  - API: `describe_analysis`
  - Purpose: Get analysis details
  - Returns: Data set ARNs, sheets, status

- [ ] **20.5** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new QuickSight calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: QuickSight Analytics Details

**Prerequisites:**
- Have at least one AWS account connected with QuickSight access
- Have QuickSight Enterprise or Standard edition enabled
- Have at least one data source, dataset, or dashboard configured

**Testing Steps:**

1. **Test data source details**
   - Open Resource Explorer and navigate to QuickSight > Data Sources
   - Select a data source
   - View the details panel
   - Verify: Connection parameters are displayed (type, host, port)
   - Verify: VPC configuration is shown (if applicable)
   - Verify: SSL/credentials status is displayed (not actual values)

2. **Test dashboard details**
   - Navigate to QuickSight > Dashboards
   - Select a dashboard
   - Verify: Version information is displayed
   - Verify: Number of sheets is shown
   - Verify: Parameters are listed (if any)

3. **Test dataset details**
   - Navigate to QuickSight > Data Sets
   - Select a dataset
   - Verify: Physical table mappings are displayed
   - Verify: Logical table structure is shown
   - Verify: Column names and types are listed

4. **Test analysis details**
   - Navigate to QuickSight > Analyses
   - Select an analysis
   - Verify: Associated dataset ARNs are displayed
   - Verify: Sheet information is shown
   - Verify: Status (creation/update state) is displayed

**Expected Behaviors:**
- QuickSight resources load without permission errors
- Data source details don't expose sensitive connection credentials
- All resource types show consistent detail formatting
- AWS account ID context is correct for QuickSight

---

## Milestone 21: Redshift Cluster Details

**File**: `src/app/resource_explorer/aws_services/redshift.rs`

### Tasks

- [ ] **21.1** Enhance `describe_clusters()` function
  - API: Enhanced `describe_clusters`
  - Purpose: Get full cluster configuration
  - Returns: Node type, nodes, VPC, encryption, endpoint

- [ ] **21.2** Add `describe_logging_status()` function
  - API: `describe_logging_status`
  - Purpose: Get audit logging configuration
  - Returns: Logging enabled, S3 bucket, prefix

- [ ] **21.3** Add `describe_cluster_parameters()` function
  - API: `describe_cluster_parameters`
  - Purpose: Get parameter group settings
  - Returns: Parameter names, values, descriptions

- [ ] **21.4** Add `describe_cluster_security_groups()` function
  - API: `describe_cluster_security_groups`
  - Purpose: Get security groups (EC2-Classic)
  - Returns: Security group names, IPs

- [ ] **21.5** Add `describe_cluster_snapshots()` function
  - API: `describe_cluster_snapshots`
  - Purpose: List cluster snapshots
  - Returns: Snapshot ID, creation time, status, size

- [ ] **21.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new Redshift calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Redshift Cluster Details

**Prerequisites:**
- Have at least one AWS account connected with Redshift access
- Have at least one Redshift cluster deployed
- Optional: Have audit logging configured for logging test

**Testing Steps:**

1. **Test cluster configuration**
   - Open Resource Explorer and navigate to Redshift > Clusters
   - Select a Redshift cluster
   - View the details panel
   - Verify: Node type is displayed (e.g., dc2.large, ra3.xlplus)
   - Verify: Number of nodes is shown
   - Verify: VPC and subnet information is displayed
   - Verify: Encryption status is shown
   - Verify: Cluster endpoint is displayed

2. **Test logging status**
   - Check for "Logging" section in details panel
   - Verify: Audit logging enabled/disabled status is shown
   - If enabled, verify: S3 bucket name is displayed
   - Verify: Logging prefix is shown

3. **Test cluster parameters**
   - Look for "Parameters" tab or section
   - Verify: Parameter group name is shown
   - Verify: Individual parameter names and values are listed
   - Verify: Parameter descriptions are displayed

4. **Test security groups**
   - Check for "Security" section
   - Verify: Security group names are listed
   - Verify: Associated IPs/CIDR ranges are shown (if EC2-Classic)

5. **Test snapshots**
   - Navigate to cluster snapshots section or tab
   - Verify: Snapshot IDs are listed
   - Verify: Creation timestamps are displayed
   - Verify: Snapshot status (available, creating) is shown
   - Verify: Snapshot size is displayed

**Expected Behaviors:**
- All cluster details load within a few seconds
- Encryption status clearly indicates at-rest encryption
- Parameter values display correctly (including special characters)
- Snapshot list paginates correctly for clusters with many snapshots

---

## Milestone 22: Route53 DNS Details

**File**: `src/app/resource_explorer/aws_services/route53.rs`

### Tasks

- [ ] **22.1** Add `list_resource_record_sets()` function
  - API: `list_resource_record_sets`
  - Purpose: Get DNS records
  - Returns: Record name, type, TTL, values, alias info

- [ ] **22.2** Add `get_hosted_zone_count()` function
  - API: `get_hosted_zone_count`
  - Purpose: Get total zone count
  - Returns: Hosted zone count

- [ ] **22.3** Add `list_query_logging_configs()` function
  - API: `list_query_logging_configs`
  - Purpose: Get query logging configuration
  - Returns: CloudWatch log group ARN

- [ ] **22.4** Add `get_dnssec()` function
  - API: `get_dnssec`
  - Purpose: Get DNSSEC status
  - Returns: DNSSEC status, key signing keys

- [ ] **22.5** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new Route53 calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Route53 DNS Details

**Prerequisites:**
- Have at least one AWS account connected with Route53 access
- Have at least one hosted zone configured
- Have some DNS records in the hosted zone

**Testing Steps:**

1. **Test resource record sets**
   - Open Resource Explorer and navigate to Route53 > Hosted Zones
   - Select a hosted zone
   - View the details panel
   - Verify: DNS records (A, AAAA, CNAME, MX, etc.) are listed
   - Verify: Record names are displayed correctly
   - Verify: Record types are shown
   - Verify: TTL values are displayed
   - Verify: Record values (IPs, hostnames) are shown
   - Verify: Alias records show target information

2. **Test hosted zone count**
   - Check the Route53 summary or dashboard view
   - Verify: Total hosted zone count is displayed
   - Compare with AWS Console to verify accuracy

3. **Test query logging configuration**
   - If query logging is configured, check for logging section
   - Verify: CloudWatch log group ARN is displayed
   - Verify: Logging status is shown (enabled/disabled)

4. **Test DNSSEC status**
   - For hosted zones with DNSSEC, check security section
   - Verify: DNSSEC status (enabled/disabled) is displayed
   - Verify: Key signing keys are listed (if enabled)

**Expected Behaviors:**
- DNS records load completely (pagination works for large zones)
- Alias records clearly show AWS service targets (ELB, CloudFront, etc.)
- TTL values display in human-readable format
- DNSSEC status is accurate for the hosted zone

---

## Milestone 23: SSM Systems Management Details

**File**: `src/app/resource_explorer/aws_services/ssm.rs`

### Tasks

- [ ] **23.1** Add `get_parameter()` function
  - API: `get_parameter`
  - Purpose: Get parameter value (with decryption option)
  - Returns: Parameter value, type, version

- [ ] **23.2** Add `get_parameters_by_path()` function
  - API: `get_parameters_by_path`
  - Purpose: Get parameter hierarchy
  - Returns: Parameters under path prefix

- [ ] **23.3** Add `describe_document()` function
  - API: `describe_document`
  - Purpose: Get document details
  - Returns: Document content, schema version, parameters

- [ ] **23.4** Add `list_command_invocations()` function
  - API: `list_command_invocations`
  - Purpose: Get command execution history
  - Returns: Command ID, instance ID, status, output

- [ ] **23.5** Add `describe_instance_information()` function
  - API: `describe_instance_information`
  - Purpose: Get managed instances
  - Returns: Instance ID, platform, agent version, status

- [ ] **23.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new SSM calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: SSM Systems Management Details

**Prerequisites:**
- Have at least one AWS account connected with SSM access
- Have SSM parameters configured (standard or SecureString)
- Have at least one managed instance (EC2 with SSM agent)
- Optional: Have SSM documents and command history

**Testing Steps:**

1. **Test parameter value retrieval**
   - Open Resource Explorer and navigate to SSM > Parameters
   - Select a parameter
   - View the details panel
   - Verify: Parameter value is displayed (decrypted for SecureString if permissions allow)
   - Verify: Parameter type (String, StringList, SecureString) is shown
   - Verify: Parameter version is displayed

2. **Test parameter path hierarchy**
   - Navigate to a parameter with hierarchical path (e.g., /app/config/setting)
   - Verify: Parameters under same path prefix are grouped or navigable
   - Verify: Full path is displayed correctly

3. **Test document details**
   - Navigate to SSM > Documents
   - Select a document (custom or AWS-provided)
   - Verify: Document content/schema is displayed
   - Verify: Schema version is shown
   - Verify: Document parameters are listed with descriptions

4. **Test command invocations**
   - Navigate to SSM > Command History (or Run Command)
   - Select a command execution
   - Verify: Command ID is displayed
   - Verify: Target instance IDs are shown
   - Verify: Command status (Success, Failed, etc.) is displayed
   - Verify: Output is available for completed commands

5. **Test managed instances**
   - Navigate to SSM > Managed Instances
   - Verify: Instance IDs are listed
   - Verify: Platform type (Windows, Linux) is shown
   - Verify: SSM Agent version is displayed
   - Verify: Connection status (Online, Offline) is shown

**Expected Behaviors:**
- SecureString parameters show decrypted values only with proper permissions
- Parameter versions allow viewing historical values
- Command output displays even for large outputs (with pagination)
- Managed instance list updates to reflect current connectivity status

---

## Milestone 24: BedrockAgentCore Details

**File**: `src/app/resource_explorer/aws_services/bedrockagentcore_control.rs`

### Tasks

- [ ] **24.1** Add `get_agent_runtime()` function
  - API: `get_agent_runtime`
  - Purpose: Get runtime details
  - Returns: Runtime config, status, endpoint

- [ ] **24.2** Add `get_gateway()` function
  - API: `get_gateway`
  - Purpose: Get gateway configuration
  - Returns: Gateway type, endpoint, status

- [ ] **24.3** Add `get_browser()` function
  - API: `get_browser`
  - Purpose: Get browser tool details
  - Returns: Browser config, capabilities

- [ ] **24.4** Add `get_code_interpreter()` function
  - API: `get_code_interpreter`
  - Purpose: Get code interpreter configuration
  - Returns: Runtime, capabilities, limits

- [ ] **24.5** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new BedrockAgentCore calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: BedrockAgentCore Details

**Prerequisites:**
- Have at least one AWS account connected with BedrockAgentCore access
- Have BedrockAgentCore resources deployed (agent runtimes, gateways, etc.)
- Note: BedrockAgentCore is a newer service - availability varies by region

**Testing Steps:**

1. **Test agent runtime details**
   - Open Resource Explorer and navigate to BedrockAgentCore > Agent Runtimes
   - Select an agent runtime
   - View the details panel
   - Verify: Runtime configuration is displayed
   - Verify: Status (active, creating, etc.) is shown
   - Verify: Endpoint information is displayed

2. **Test gateway configuration**
   - Navigate to BedrockAgentCore > Gateways
   - Select a gateway
   - Verify: Gateway type is displayed
   - Verify: Endpoint URL is shown
   - Verify: Status is displayed

3. **Test browser tool details**
   - If browser tools are configured, navigate to the appropriate section
   - Verify: Browser configuration is displayed
   - Verify: Capabilities list is shown

4. **Test code interpreter details**
   - If code interpreter is configured, view its details
   - Verify: Runtime information is displayed
   - Verify: Capabilities are listed
   - Verify: Resource limits are shown

**Expected Behaviors:**
- BedrockAgentCore resources are discoverable in supported regions
- Status reflects current operational state
- Endpoint URLs are valid and formatted correctly
- Permission errors are clearly indicated for restricted resources

---

## Data Plane Implementation Reference

### Directory Structure

```
src/app/data_plane/
├── mod.rs                        # Re-exports all data plane modules
├── cloudwatch_logs/              # EXISTING - Reference implementation
│   ├── mod.rs
│   ├── client.rs
│   ├── types.rs
│   └── resource_mapping.rs
├── cloudtrail_events/            # EXISTING - Reference implementation
│   ├── mod.rs
│   ├── client.rs
│   ├── types.rs
│   └── resource_mapping.rs
├── athena/                       # NEW - Log querying
│   ├── mod.rs
│   ├── client.rs
│   ├── types.rs
│   └── resource_mapping.rs
└── cloudwatch_metrics/           # NEW - Metrics data
    ├── mod.rs
    ├── client.rs
    └── types.rs
```

### Athena Query Execution Pattern

```rust
// Asynchronous query execution flow
pub async fn execute_query(&self, query: &str, database: &str) -> Result<QueryResult> {
    // 1. Start query
    let execution_id = self.start_query_execution(query, database).await?;

    // 2. Poll until complete (with timeout)
    loop {
        let status = self.get_query_execution(&execution_id).await?;
        match status.state {
            QueryState::Succeeded => break,
            QueryState::Failed => return Err(anyhow!("Query failed: {}", status.reason)),
            QueryState::Cancelled => return Err(anyhow!("Query cancelled")),
            _ => tokio::time::sleep(Duration::from_millis(500)).await,
        }
    }

    // 3. Get results
    self.get_query_results(&execution_id).await
}
```

## Testing Strategy

- Unit tests for JSON conversion functions
- Integration tests with real AWS responses (no mocks per CLAUDE.md)
- Test Athena query execution flow with sample queries
- Test CloudWatch metrics retrieval with time ranges
- UI tests using egui_kittest for windows

## Progress Tracking

| Milestone | Service | Tasks | Completed |
|-----------|---------|-------|-----------|
| 17 | Athena (Full Data Plane) | 20 | 0 |
| 18 | CloudWatch (Full Data Plane) | 14 | 0 |
| 19 | OpenSearch | 5 | 0 |
| 20 | QuickSight | 5 | 0 |
| 21 | Redshift | 6 | 0 |
| 22 | Route53 | 5 | 0 |
| 23 | SSM | 6 | 0 |
| 24 | BedrockAgentCore | 5 | 0 |
| **Total** | **8** | **66** | **0** |

## Grand Total (All 3 Parts)

| Part | Services | Tasks |
|------|----------|-------|
| Part 1 | 8 | 48 |
| Part 2 | 8 | 46 |
| Part 3 | 8 | 66 |
| **Total** | **24** | **160** |
