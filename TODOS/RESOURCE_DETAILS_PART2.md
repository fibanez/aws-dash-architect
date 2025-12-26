# AWS Resource Details Implementation - Part 2: Infrastructure & Data

## Overview

This document covers the implementation of missing detail-getting API functions for 8 AWS services focused on infrastructure and data management.

**Services**: DynamoDB, CloudFormation, ECS, ELB, EMR, EventBridge, Glue, AWS Backup

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
            if let Ok(policy) = self.get_policy_internal(&client, &id).await {
                json.insert("Policy".to_string(), policy);
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
    if let Ok(policy) = self.get_policy_internal(&client, resource_id).await {
        details.insert("Policy".to_string(), policy);
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

## Milestone 9: DynamoDB Table Details (COMPLETE)

**File**: `src/app/resource_explorer/aws_services/dynamodb.rs`

### Tasks

- [x] **9.1** Add `describe_table()` function (existing, enhanced with timeout)
  - API: `describe_table`
  - Purpose: Get table configuration, GSIs, encryption
  - Returns: Table status, key schema, GSIs, LSIs, billing mode, SSE description

- [x] **9.2** Add `describe_continuous_backups()` function
  - API: `describe_continuous_backups`
  - Purpose: Check Point-in-Time Recovery status
  - Returns: PITR status, earliest/latest restorable times

- [x] **9.3** Add `describe_time_to_live()` function
  - API: `describe_time_to_live`
  - Purpose: Get TTL configuration
  - Returns: TTL status, attribute name

- [x] **9.4** Add `list_tags_of_resource()` function
  - API: `list_tags_of_resource`
  - Purpose: Get table tags
  - Returns: Vec of tag key-value pairs

- [x] **9.5** Add two-phase loading support
  - Added `include_details: bool` parameter to `list_tables()`
  - Added `get_table_details()` for Phase 2 enrichment
  - Added status bar reporting via `report_status()` / `report_status_done()`
  - Added to `fetch_resource_details()` in aws_client.rs

- [ ] **9.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new DynamoDB calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 9 - DynamoDB

**Prerequisites**:
- AWS credentials configured with DynamoDB read permissions
- At least one DynamoDB table (ideally with GSIs and encryption)

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::DynamoDB::Table`
   - Run query to list tables

3. **Test Table Details**
   - Click on any DynamoDB table
   - Verify the detail panel shows:
     - [ ] Table name and ARN
     - [ ] Table status (ACTIVE, etc.)
     - [ ] Key schema (partition key, sort key)
     - [ ] Billing mode (PAY_PER_REQUEST or PROVISIONED)
     - [ ] SSE description (encryption settings)

4. **Test Global Secondary Indexes** (if table has GSIs)
   - Find a table with GSIs
   - Verify:
     - [ ] GSI names are listed
     - [ ] GSI key schema is shown
     - [ ] GSI projection type is displayed

5. **Test Local Secondary Indexes** (if table has LSIs)
   - Find a table with LSIs
   - Verify:
     - [ ] LSI names are listed
     - [ ] LSI key schema is shown

6. **Test Point-in-Time Recovery**
   - In the table detail panel, verify:
     - [ ] PITR status (Enabled/Disabled)
     - [ ] If enabled: earliest and latest restorable times

7. **Test Time-to-Live Configuration**
   - Verify:
     - [ ] TTL status (ENABLED/DISABLED)
     - [ ] TTL attribute name (if enabled)

8. **Test Table Tags**
   - Find a table with tags
   - Verify:
     - [ ] All tag key-value pairs are displayed

9. **Test Table Without Optional Features**
   - Select a basic table without GSIs, PITR, or TTL
   - Verify: Missing features displayed as "Not configured" or similar

**Expected Results**:
- All DynamoDB table details display correctly
- GSIs and LSIs are properly listed with schemas
- PITR and TTL status accurately reflect AWS console
- Tags display correctly

---

## Milestone 10: CloudFormation Stack Details (COMPLETE)

**File**: `src/app/resource_explorer/aws_services/cloudformation.rs`

### Tasks

- [x] **10.1** Add `list_stack_events()` function
  - API: `describe_stack_events`
  - Purpose: Get full paginated stack events
  - Returns: Event ID, timestamp, status, reason, resource type

- [x] **10.2** Add `list_stack_resources()` function
  - API: `list_stack_resources`
  - Purpose: Get all resources in stack
  - Returns: Logical/physical resource IDs, type, status, drift info

- [x] **10.3** Add `get_stack_policy()` function
  - API: `get_stack_policy`
  - Purpose: Get stack update policy
  - Returns: Policy document JSON

- [x] **10.4** Add `describe_stack_drift_detection_status()` function
  - API: `describe_stack_drift_detection_status`
  - Purpose: Check drift detection status
  - Returns: Detection status, drifted stack resource count

- [x] **10.5** Add two-phase loading support
  - Added `include_details: bool` parameter to `list_stacks()`
  - Added `get_stack_details()` for Phase 2 enrichment
  - Added status bar reporting via `report_status()` / `report_status_done()`
  - Added to `fetch_resource_details()` in aws_client.rs

- [ ] **10.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new CloudFormation calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 10 - CloudFormation

**Prerequisites**:
- AWS credentials configured with CloudFormation read permissions
- At least one CloudFormation stack (preferably with multiple resources)

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::CloudFormation::Stack`
   - Run query to list stacks

3. **Test Stack Events**
   - Click on any CloudFormation stack
   - Verify the detail panel shows:
     - [ ] List of stack events
     - [ ] Event timestamps
     - [ ] Event status (CREATE_COMPLETE, UPDATE_IN_PROGRESS, etc.)
     - [ ] Resource type for each event
     - [ ] Status reason (especially for failures)

4. **Test Stack Resources**
   - In the stack detail panel, verify:
     - [ ] List of all resources in the stack
     - [ ] Logical resource IDs
     - [ ] Physical resource IDs
     - [ ] Resource types
     - [ ] Resource status

5. **Test Stack Policy** (if stack has a policy)
   - Find a stack with a stack policy configured
   - Verify:
     - [ ] Policy document JSON is displayed
     - [ ] Policy statements are readable

6. **Test Drift Detection Status**
   - If drift detection was previously run:
     - [ ] Detection status is shown
     - [ ] Number of drifted resources (if any)
   - If never run:
     - [ ] Appropriate "Not detected" message

7. **Test Trigger Drift Detection** (optional - causes AWS API call)
   - If UI supports triggering drift detection:
     - [ ] Click to start drift detection
     - [ ] Verify detection ID is returned
     - [ ] Status updates after completion

8. **Test Stack With Many Events**
   - Find a stack with many events (updates, rollbacks)
   - Verify:
     - [ ] Pagination works correctly
     - [ ] All events are accessible
     - [ ] Performance is acceptable

9. **Test Stack Without Policy**
   - Select a stack without a stack policy
   - Verify: Shows "No stack policy" or similar message

**Expected Results**:
- Stack events display in chronological order with all details
- All stack resources are listed with correct status
- Stack policy JSON displays correctly when present
- Drift detection status accurately reflects AWS console

---

## Milestone 11: ECS Container Details (COMPLETE)

**File**: `src/app/resource_explorer/aws_services/ecs.rs`

### Tasks

- [x] **11.1** Add `describe_clusters()` function (batch)
  - API: `describe_clusters`
  - Purpose: Get cluster details with settings
  - Returns: Cluster settings, capacity providers, statistics

- [x] **11.2** Add `describe_services()` function (batch)
  - API: `describe_services`
  - Purpose: Get service details
  - Returns: Desired/running count, load balancers, deployment config

- [x] **11.3** Add `describe_container_instances()` function
  - API: `describe_container_instances`
  - Purpose: Get EC2 container instance details
  - Returns: Instance ID, status, resources, attributes

- [x] **11.4** Enhance `list_services()` with pagination
  - API: Enhanced `list_services`
  - Purpose: List all services across clusters
  - Returns: Service ARNs with pagination

- [x] **11.5** Enhance `describe_task_definition()` function
  - API: Enhanced `describe_task_definition`
  - Purpose: Get task definition with full container details
  - Returns: Container definitions, volumes, network mode

- [x] **11.6** Add two-phase loading support
  - Added `include_details: bool` parameter to `list_clusters()`, `list_services()`, `list_tasks()`, `list_task_definitions()`
  - Added `get_cluster_details()` and `get_service_details()` for Phase 2 enrichment
  - Added to `enrichable_types` array in aws_client.rs
  - Added to `fetch_resource_details()` in aws_client.rs

- [ ] **11.7** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new ECS calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 11 - ECS

**Prerequisites**:
- AWS credentials configured with ECS read permissions
- At least one ECS cluster with services and tasks

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::ECS::Cluster`
   - Run query to list clusters

3. **Test Cluster Details**
   - Click on any ECS cluster
   - Verify the detail panel shows:
     - [ ] Cluster ARN and name
     - [ ] Cluster status
     - [ ] Registered container instances count
     - [ ] Running tasks count
     - [ ] Active services count
     - [ ] Capacity providers (if configured)

4. **Test Service Details**
   - Select resource type: `AWS::ECS::Service`
   - Click on any service
   - Verify:
     - [ ] Service ARN and name
     - [ ] Desired count
     - [ ] Running count
     - [ ] Pending count
     - [ ] Launch type (EC2 or FARGATE)
     - [ ] Load balancers (if configured)
     - [ ] Deployment configuration

5. **Test Container Instance Details** (EC2 launch type)
   - If cluster has EC2 instances:
     - [ ] Container instance ARN
     - [ ] EC2 instance ID
     - [ ] Status (ACTIVE, DRAINING, etc.)
     - [ ] Registered resources (CPU, memory)
     - [ ] Remaining resources

6. **Test Task Definition Details**
   - Select resource type: `AWS::ECS::TaskDefinition`
   - Click on any task definition
   - Verify:
     - [ ] Family and revision
     - [ ] Task role ARN
     - [ ] Execution role ARN
     - [ ] Network mode
     - [ ] CPU and memory requirements
     - [ ] Container definitions with image, ports, environment

7. **Test Pagination for Services**
   - Find a cluster with many services (10+)
   - Verify:
     - [ ] All services are listed
     - [ ] Pagination works correctly

8. **Test Fargate Services**
   - Find a Fargate service
   - Verify:
     - [ ] Launch type shows "FARGATE"
     - [ ] Platform version is displayed

**Expected Results**:
- Cluster statistics accurately match AWS console
- Service desired/running counts are correct
- Task definitions show all container details
- Container instances (EC2) show resource utilization

---

## Milestone 12: ELB Load Balancer Details (COMPLETE)

**Files**: `src/app/resource_explorer/aws_services/elb.rs` and `elbv2.rs`

### Tasks

- [x] **12.1** Add `describe_load_balancer_attributes()` function
  - API: `describe_load_balancer_attributes` (ELBv2)
  - Purpose: Get access logs, deletion protection, idle timeout
  - Returns: Access logs S3 bucket, deletion protection enabled

- [x] **12.2** Add `describe_listeners()` function
  - API: `describe_listeners`
  - Purpose: Get listener configuration
  - Returns: Port, protocol, SSL policy, default actions

- [x] **12.3** Add `describe_rules()` function
  - API: `describe_rules`
  - Purpose: Get routing rules for listener
  - Returns: Rule conditions, actions, priority

- [x] **12.4** Add `describe_target_health()` function
  - API: `describe_target_health`
  - Purpose: Get target health status
  - Returns: Target ID, health state, reason

- [x] **12.5** Add two-phase loading support
  - Added `include_details: bool` parameter to `list_load_balancers()`
  - Added `get_load_balancer_details()` for Phase 2 enrichment
  - Added to `enrichable_types` array in aws_client.rs
  - Added to `fetch_resource_details()` in aws_client.rs

- [ ] **12.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new ELB calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 12 - ELB

**Prerequisites**:
- AWS credentials configured with ELB/ELBv2 read permissions
- At least one Application Load Balancer or Network Load Balancer

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::ElasticLoadBalancingV2::LoadBalancer`
   - Run query to list load balancers

3. **Test Load Balancer Attributes**
   - Click on any ALB/NLB
   - Verify the detail panel shows:
     - [ ] Load balancer ARN and name
     - [ ] Access logs enabled (true/false)
     - [ ] Access logs S3 bucket (if enabled)
     - [ ] Deletion protection status
     - [ ] Idle timeout (for ALB)

4. **Test Listeners**
   - In the load balancer detail panel, verify:
     - [ ] List of listeners
     - [ ] Port number for each listener
     - [ ] Protocol (HTTP, HTTPS, TCP, TLS)
     - [ ] SSL policy (for HTTPS/TLS)
     - [ ] Default action (forward, redirect, fixed-response)

5. **Test Routing Rules** (ALB)
   - Find an ALB with path-based or host-based routing
   - Verify:
     - [ ] Rule priority
     - [ ] Rule conditions (path, host header, etc.)
     - [ ] Rule actions (forward to target group)

6. **Test Target Health**
   - Select a target group or click through from load balancer
   - Verify:
     - [ ] Target IDs (instance IDs or IPs)
     - [ ] Health status (healthy, unhealthy, draining)
     - [ ] Health check port
     - [ ] Reason for unhealthy (if applicable)

7. **Test SSL/TLS Policies** (HTTPS listeners)
   - Find a load balancer with HTTPS listener
   - Verify:
     - [ ] SSL policy name
     - [ ] Supported protocols (TLSv1.2, etc.)
     - [ ] Supported ciphers (if shown)

8. **Test Classic Load Balancer** (if applicable)
   - Select resource type for Classic ELB
   - Verify basic attributes are shown

**Expected Results**:
- Load balancer attributes display correctly
- All listeners are listed with protocols and ports
- Routing rules show conditions and actions
- Target health accurately reflects AWS console

---

## Milestone 13: EMR Cluster Details (COMPLETE)

**File**: `src/app/resource_explorer/aws_services/emr.rs`

### Tasks

- [x] **13.1** Add `describe_cluster()` function
  - API: `describe_cluster`
  - Purpose: Get cluster configuration
  - Returns: Status, EC2 instance attributes, applications, configurations

- [x] **13.2** Add `list_instance_groups()` function
  - API: `list_instance_groups`
  - Purpose: Get instance groups in cluster
  - Returns: Instance type, count, market type, EBS config

- [x] **13.3** Add `list_steps()` function
  - API: `list_steps`
  - Purpose: Get cluster steps
  - Returns: Step name, status, action on failure

- [x] **13.4** Add two-phase loading support
  - Added `include_details: bool` parameter to `list_clusters()`
  - Added `get_cluster_details()` for Phase 2 enrichment
  - Added to `enrichable_types` array in aws_client.rs
  - Added to `fetch_resource_details()` in aws_client.rs

- [ ] **13.5** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new EMR calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 13 - EMR

**Prerequisites**:
- AWS credentials configured with EMR read permissions
- At least one EMR cluster (running or terminated)

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::EMR::Cluster`
   - Run query to list clusters

3. **Test Cluster Details**
   - Click on any EMR cluster
   - Verify the detail panel shows:
     - [ ] Cluster ID and name
     - [ ] Cluster status (WAITING, RUNNING, TERMINATED, etc.)
     - [ ] Release label (emr-6.x.x)
     - [ ] Applications (Spark, Hive, etc.)
     - [ ] EC2 key pair name
     - [ ] VPC/subnet information

4. **Test Instance Groups**
   - In the cluster detail panel, verify:
     - [ ] Master instance group (type, count)
     - [ ] Core instance group (type, count)
     - [ ] Task instance groups (if any)
     - [ ] Market type (ON_DEMAND or SPOT)
     - [ ] EBS volume configuration

5. **Test Security Configuration** (if configured)
   - Find a cluster with security configuration
   - Verify:
     - [ ] Security configuration name
     - [ ] Encryption at rest settings
     - [ ] Encryption in transit settings
     - [ ] Kerberos configuration (if enabled)

6. **Test Cluster Steps**
   - Verify:
     - [ ] List of steps
     - [ ] Step names
     - [ ] Step status (COMPLETED, FAILED, PENDING)
     - [ ] Action on failure setting

7. **Test Terminated Cluster**
   - Select a terminated cluster
   - Verify:
     - [ ] Historical details are still accessible
     - [ ] Termination reason is shown

**Expected Results**:
- Cluster details display correctly including applications
- Instance groups show all configuration details
- Security configuration displays encryption settings
- Steps show status and action on failure

---

## Milestone 14: EventBridge Details (COMPLETE)

**File**: `src/app/resource_explorer/aws_services/eventbridge.rs`

### Tasks

- [x] **14.1** Add `describe_event_bus()` function
  - API: `describe_event_bus`
  - Purpose: Get event bus details
  - Returns: Name, ARN, policy

- [x] **14.2** Add `list_targets_by_rule()` function
  - API: `list_targets_by_rule`
  - Purpose: Get targets for a rule
  - Returns: Target ID, ARN, role ARN, input transformation

- [x] **14.3** Add `list_archives()` function
  - API: `list_archives`
  - Purpose: List event archives
  - Returns: Archive names and source ARNs

- [x] **14.4** Add two-phase loading support
  - Added `include_details: bool` parameter to `list_event_buses()`
  - Added `get_event_bus_details()` for Phase 2 enrichment
  - Added internal helpers: `list_rules_for_bus_internal()`, `list_targets_by_rule_internal()`, `list_archives_internal()`
  - Added to `enrichable_types` array in aws_client.rs
  - Added to `fetch_resource_details()` in aws_client.rs

- [ ] **14.5** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new EventBridge calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 14 - EventBridge

**Prerequisites**:
- AWS credentials configured with EventBridge read permissions
- At least one EventBridge rule with targets

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::Events::EventBus`
   - Run query to list event buses

3. **Test Event Bus Details**
   - Click on any event bus (including default)
   - Verify the detail panel shows:
     - [ ] Event bus name
     - [ ] Event bus ARN
     - [ ] Event bus policy (if custom policy set)

4. **Test Rule Details**
   - Select resource type: `AWS::Events::Rule`
   - Click on any rule
   - Verify:
     - [ ] Rule name and ARN
     - [ ] Event pattern or schedule expression
     - [ ] State (ENABLED/DISABLED)

5. **Test Rule Targets**
   - In the rule detail panel, verify:
     - [ ] List of targets
     - [ ] Target ID
     - [ ] Target ARN (Lambda, SNS, SQS, etc.)
     - [ ] IAM role ARN (if applicable)
     - [ ] Input transformation (if configured)

6. **Test Event Archives** (if configured)
   - Find event archives
   - Verify:
     - [ ] Archive name
     - [ ] Source event bus
     - [ ] Event pattern filter
     - [ ] Retention days

7. **Test API Destination Connections** (if configured)
   - Find API destinations
   - Verify:
     - [ ] Connection name
     - [ ] Connection state
     - [ ] Authorization type (API_KEY, OAUTH, BASIC)

8. **Test Rule Without Targets**
   - Find or create a rule without targets
   - Verify: Empty target list displayed gracefully

**Expected Results**:
- Event bus details display correctly
- Rule targets show all configuration including input transformations
- Archives display retention and event patterns
- API connections show auth type and state

---

## Milestone 15: Glue Data Catalog Details (PARTIAL - Jobs COMPLETE)

**File**: `src/app/resource_explorer/aws_services/glue.rs`

### Tasks

- [x] **15.1** Enhance `get_job()` function
  - API: Enhanced with job runs/triggers
  - Purpose: Get job with run history
  - Returns: Job config plus recent run statuses
  - Added `include_details: bool` parameter to `list_jobs()`
  - Added `get_job_details()` for Phase 2 enrichment
  - Added internal helpers: `get_job_runs_internal()`, `get_triggers_internal()`
  - Added to `enrichable_types` array in aws_client.rs
  - Added to `fetch_resource_details()` in aws_client.rs

- [ ] **15.2** Add `get_databases()` function
  - API: `get_databases`
  - Purpose: List Data Catalog databases
  - Returns: Database names, descriptions, location URIs

- [ ] **15.3** Add `get_tables()` function
  - API: `get_tables`
  - Purpose: List tables in database
  - Returns: Table names, columns, storage descriptor

- [ ] **15.4** Add `get_crawlers()` function
  - API: `get_crawlers`
  - Purpose: Get crawler configurations
  - Returns: Crawler name, role, targets, schedule

- [ ] **15.5** Add `get_security_configurations()` function
  - API: `get_security_configurations`
  - Purpose: Get encryption settings
  - Returns: S3 encryption, CloudWatch encryption, job bookmark encryption

- [ ] **15.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new Glue calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 15 - Glue

**Prerequisites**:
- AWS credentials configured with Glue read permissions
- At least one Glue database/table or job

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::Glue::Job`
   - Run query to list Glue jobs

3. **Test Job Details with Run History**
   - Click on any Glue job
   - Verify the detail panel shows:
     - [ ] Job name and ARN
     - [ ] Job type (Spark, Python Shell, etc.)
     - [ ] IAM role
     - [ ] Worker type and number of workers
     - [ ] Recent job runs with status

4. **Test Data Catalog Databases**
   - Navigate to Glue databases
   - Verify:
     - [ ] Database names
     - [ ] Database descriptions
     - [ ] Location URIs (if set)

5. **Test Data Catalog Tables**
   - Find tables in a database
   - Verify:
     - [ ] Table names
     - [ ] Column names and types
     - [ ] Storage descriptor (S3 location, format)
     - [ ] Partition keys (if any)

6. **Test Crawlers**
   - Navigate to Glue crawlers
   - Verify:
     - [ ] Crawler name
     - [ ] IAM role
     - [ ] Crawler targets (S3 paths, JDBC, etc.)
     - [ ] Schedule (on-demand or cron)
     - [ ] Last run status

7. **Test Security Configurations**
   - Find Glue security configurations
   - Verify:
     - [ ] Configuration name
     - [ ] S3 encryption settings
     - [ ] CloudWatch logs encryption
     - [ ] Job bookmark encryption

8. **Test Job Without Runs**
   - Find a job that has never been run
   - Verify: "No runs" or empty history displayed gracefully

**Expected Results**:
- Job details display with recent run history
- Database and table schemas are accurate
- Crawlers show targets and schedule
- Security configurations display all encryption settings

---

## Milestone 16: AWS Backup Details (COMPLETE)

**File**: `src/app/resource_explorer/aws_services/backup.rs`

### Tasks

- [x] **16.1** Add `get_backup_plan()` function
  - API: `get_backup_plan`
  - Purpose: Get backup plan details
  - Returns: Backup rules, schedule, lifecycle, copy actions
  - Added `include_details: bool` parameter to `list_backup_plans()`
  - Added `get_backup_plan_details()` for Phase 2 enrichment
  - Added internal helper: `get_backup_plan_internal()`

- [x] **16.2** Add `describe_backup_vault()` function (enhanced)
  - API: `describe_backup_vault`
  - Purpose: Get vault details
  - Returns: Vault name, ARN, encryption key, recovery points count
  - Added `include_details: bool` parameter to `list_backup_vaults()`
  - Added `get_backup_vault_details()` for Phase 2 enrichment

- [x] **16.3** Add `list_backup_selections()` function
  - API: `list_backup_selections`
  - Purpose: Get resource selections for plan
  - Returns: Selection IDs and names
  - Added internal helper: `get_backup_selections_internal()`

- [x] **16.4** Add `get_backup_vault_access_policy()` function
  - API: `get_backup_vault_access_policy`
  - Purpose: Get vault access policy
  - Returns: Policy document JSON
  - Added internal helper: `get_vault_access_policy_internal()`

- [x] **16.5** Add `list_recovery_points_by_backup_vault()` function
  - API: `list_recovery_points_by_backup_vault`
  - Purpose: List recovery points in vault
  - Returns: Recovery point ARN, resource ARN, creation date, status
  - Added internal helper: `list_recovery_points_internal()`

- [x] **16.6** Integration with aws_client.rs
  - Added to `enrichable_types` array: `AWS::Backup::BackupPlan`, `AWS::Backup::BackupVault`
  - Updated `list_resources_for_type()` to pass `include_details: false`
  - Updated `fetch_resource_details()` to use `get_backup_plan_details()` and `get_backup_vault_details()`

- [ ] **16.7** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new AWS Backup calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 16 - AWS Backup

**Prerequisites**:
- AWS credentials configured with AWS Backup read permissions
- At least one backup plan and vault

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::Backup::BackupPlan`
   - Run query to list backup plans

3. **Test Backup Plan Details**
   - Click on any backup plan
   - Verify the detail panel shows:
     - [ ] Plan name and ARN
     - [ ] Plan version ID
     - [ ] Backup rules (name, schedule, lifecycle)
     - [ ] Target backup vault
     - [ ] Copy actions (if configured for cross-region)

4. **Test Backup Rules**
   - In the plan detail panel, verify:
     - [ ] Rule name
     - [ ] Schedule expression (cron)
     - [ ] Start window
     - [ ] Completion window
     - [ ] Lifecycle (delete after X days)

5. **Test Backup Selections**
   - Verify:
     - [ ] Selection IDs
     - [ ] Selection names
     - [ ] Resource types selected (EC2, RDS, etc.)

6. **Test Backup Vault Details**
   - Select resource type: `AWS::Backup::BackupVault`
   - Click on any vault
   - Verify:
     - [ ] Vault name and ARN
     - [ ] Encryption key ARN
     - [ ] Number of recovery points

7. **Test Vault Access Policy** (if configured)
   - Find a vault with access policy
   - Verify:
     - [ ] Policy document JSON is displayed

8. **Test Recovery Points**
   - In the vault detail panel, verify:
     - [ ] List of recovery points
     - [ ] Recovery point ARN
     - [ ] Source resource ARN
     - [ ] Creation date
     - [ ] Status (COMPLETED, PARTIAL, etc.)

9. **Test Vault Without Policy**
   - Select a vault without access policy
   - Verify: Shows "No access policy" or similar

10. **Test Empty Vault**
    - Find a vault with no recovery points
    - Verify: Empty recovery point list displayed gracefully

**Expected Results**:
- Backup plans display all rules and schedules
- Backup selections show resource types and selections
- Vault details include encryption and recovery point count
- Recovery points list with all metadata

---

## Implementation Pattern

```rust
/// Get detailed information for specific resource
pub async fn describe_<resource>(
    &self,
    account_id: &str,
    region: &str,
    resource_id: &str,
) -> Result<serde_json::Value> {
    let aws_config = self
        .credential_coordinator
        .create_aws_config_for_account(account_id, region)
        .await
        .with_context(|| {
            format!(
                "Failed to create AWS config for account {} in region {}",
                account_id, region
            )
        })?;

    let client = service::Client::new(&aws_config);
    let response = client.describe_xxx()
        .id(resource_id)
        .send()
        .await?;

    // Convert to JSON
    Ok(self.resource_to_json(&response))
}
```

## Testing Strategy

- Unit tests for JSON conversion functions
- Integration tests with real AWS responses (no mocks per CLAUDE.md)
- Test error handling for missing/inaccessible resources
- Test pagination for large result sets (ECS, CloudFormation events)

## Progress Tracking

| Milestone | Service | Tasks | Completed |
|-----------|---------|-------|-----------|
| 9 | DynamoDB | 5 | 0 |
| 10 | CloudFormation | 6 | 0 |
| 11 | ECS | 6 | 0 |
| 12 | ELB | 6 | 0 |
| 13 | EMR | 5 | 0 |
| 14 | EventBridge | 6 | 0 |
| 15 | Glue | 6 | 0 |
| 16 | AWS Backup | 6 | 0 |
| **Total** | **8** | **46** | **0** |
