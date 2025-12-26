# AWS Resource Details Implementation - Part 1: Core Security & Messaging

## Overview

This document covers the implementation of missing detail-getting API functions for 8 AWS services focused on core security and messaging capabilities.

**Services**: IAM, S3, Lambda, KMS, SQS, SNS, Cognito, CodeCommit

---

## Two-Phase Architecture (COMPLETE)

All Part 1 services have been retrofitted to support two-phase loading for better UI responsiveness.
**The orchestration layer is now fully implemented.**

### Architecture

**Phase 1 (Quick List)** - `include_details: false`
- Runs basic `list_*` commands (single API call per resource type)
- Returns minimal resource info (name, ID, ARN, tags from Resource Tagging API)
- UI updates immediately with resources visible
- Status bar shows "Getting [Service] list_[resources]..."

**Phase 2 (Detail Enrichment)** - Background thread
- Automatically triggered after Phase 1 completes
- Spawns separate thread with its own tokio runtime
- Fetches detailed information for each resource via `get_*_details()` functions
- Updates cache directly as details arrive
- Status bar shows "Getting [Service] get_[resource]_details..."
- UI refreshes automatically when enrichment completes

### Implementation Status

All services now have:
1. `include_details: bool` parameter on list functions
2. Separate `get_*_details()` functions for single-resource Phase 2 enrichment
3. Status bar reporting for both Phase 1 and Phase 2 operations

| Service | List Function | Detail Function | Status |
|---------|--------------|-----------------|--------|
| Lambda | `list_functions(include_details)` | `get_function_details()` | Complete |
| KMS | `list_keys(include_details)` | `get_key_details()` | Complete |
| IAM | `list_roles/users/policies(include_details)` | `get_role/user/policy_details()` | Complete |
| S3 | `list_buckets(include_details)` | `get_bucket_details()` | Complete |
| SQS | `list_queues(include_details)` | `get_queue_details()` | Complete |
| SNS | `list_topics(include_details)` | `get_topic_details()` | Complete |
| Cognito | `list_user/identity_pools(include_details)` | `get_user/identity_pool_details()` | Complete |
| CodeCommit | `list_repositories(include_details)` | `get_repository_details()` | Complete |

### Orchestration Implementation (COMPLETE)

- [x] Update aws_client.rs to pass `include_details: false` for Phase 1
- [x] Add `start_phase2_enrichment()` method to spawn background enrichment
- [x] Add `fetch_resource_details()` dispatcher for routing to service-specific detail functions
- [x] Add state tracking: `phase2_enrichment_in_progress`, `phase2_enrichment_completed`
- [x] Wire up Phase 2 to trigger automatically after Phase 1 in `window.rs`
- [x] Add cache refresh when Phase 2 completes (merges enriched data into displayed resources)
- [x] Add status bar reporting for all `get_*_details()` functions

### Key Files Modified

**aws_client.rs:**
- All service calls now pass `include_details: false`
- Added `QueryStatus::EnrichmentStarted/InProgress/Completed` variants
- Added `start_phase2_enrichment()` - spawns background thread for enrichment
- Added `fetch_resource_details()` - routes to appropriate `get_*_details()` function

**window.rs:**
- Phase 2 spawned automatically after Phase 1 results received
- State flags track enrichment progress
- Cache merged back into display state when enrichment completes

**state.rs:**
- Added `phase2_enrichment_in_progress: bool`
- Added `phase2_enrichment_completed: bool`

### Usage Pattern

```rust
// Phase 1: Quick list for immediate UI update (automatic)
// aws_client.rs calls: lambda_service.list_functions(account, region, false).await?;
// UI shows resources immediately

// Phase 2: Background enrichment (automatic, triggered after Phase 1)
// start_phase2_enrichment() spawns thread that calls:
for resource in resources_needing_details {
    let details = lambda_service.get_function_details(account, region, &name).await?;
    // Cache updated directly, UI refreshes when complete
}
```

### Data Flow

```
User triggers query
    |
    v
Phase 1: query_aws_resources_parallel(include_details: false)
    |
    +---> Resources appear in UI immediately (basic info)
    |
    v
Phase 2: start_phase2_enrichment() spawns background thread
    |
    +---> For each resource: fetch_resource_details()
    |         |
    |         +---> get_bucket_details() / get_role_details() / etc.
    |         |
    |         +---> Update cache with detailed_properties
    |
    v
phase2_enrichment_completed = true
    |
    v
UI frame detects flag, merges cache -> displayed resources
    |
    v
Resources now show detailed properties when expanded
```

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

## Milestone 1: IAM Security Details

**File**: `src/app/resource_explorer/aws_services/iam.rs`

### Tasks

- [x] **1.1** Add `list_attached_role_policies()` function
  - API: `list_attached_role_policies`
  - Purpose: Get managed policies attached to role
  - Returns: Vec of policy ARNs and names

- [x] **1.2** Add `list_role_policies()` function
  - API: `list_role_policies`
  - Purpose: Get inline policy names for a role
  - Returns: Vec of policy names

- [x] **1.3** Add `get_role_policy()` function
  - API: `get_role_policy`
  - Purpose: Get inline policy document content
  - Returns: Policy document JSON

- [x] **1.4** Add `get_policy_version()` function
  - API: `get_policy_version`
  - Purpose: Get policy document JSON for specific version
  - Returns: Policy document with statements

- [x] **1.5** Add `list_access_keys()` function
  - API: `list_access_keys`
  - Purpose: Get user access keys for security audit
  - Returns: Access key IDs, status, creation dates

- [x] **1.6** Add `list_mfa_devices()` function
  - API: `list_mfa_devices`
  - Purpose: Check MFA enabled for users
  - Returns: MFA device serial numbers and types

- [x] **1.7** Add `get_login_profile()` function
  - API: `get_login_profile`
  - Purpose: Check console access enabled
  - Returns: Login profile creation date, password reset required

- [x] **1.8** Add `list_user_policies()` function
  - API: `list_user_policies`
  - Purpose: Get user inline policy names
  - Returns: Vec of policy names

- [x] **1.9** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new IAM calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 1 - IAM

**Prerequisites**:
- AWS credentials configured with IAM read permissions
- At least one IAM role, user, and policy in the account

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Login to AWS**
   - Open the AWS Login window
   - Authenticate with your AWS account

3. **Open Resource Explorer**
   - Navigate to Resource Explorer
   - Add your account and region
   - Select resource type: `AWS::IAM::Role`

4. **Test Role Policy Functions**
   - Click on any IAM role in the list
   - Verify the detail panel shows:
     - [ ] Attached managed policies (from `list_attached_role_policies`)
     - [ ] Inline policy names (from `list_role_policies`)
     - [ ] Inline policy documents (from `get_role_policy`)

5. **Test User Security Functions**
   - Select resource type: `AWS::IAM::User`
   - Click on any IAM user
   - Verify the detail panel shows:
     - [ ] Access keys with status and creation dates (from `list_access_keys`)
     - [ ] MFA devices if configured (from `list_mfa_devices`)
     - [ ] Login profile status (from `get_login_profile`)
     - [ ] Inline policies (from `list_user_policies`)

6. **Test Policy Version Function**
   - Select resource type: `AWS::IAM::Policy`
   - Click on any managed policy
   - Verify:
     - [ ] Policy document JSON is displayed (from `get_policy_version`)
     - [ ] Statement details are visible

7. **Test Error Handling**
   - Try accessing a resource you don't have permissions for
   - Verify: Graceful error message displayed, app doesn't crash

**Expected Results**:
- All IAM detail data loads without errors
- Policy documents display as formatted JSON
- MFA and access key status accurately reflects AWS console

---

## Milestone 2: S3 Security Configuration

**File**: `src/app/resource_explorer/aws_services/s3.rs`

### Tasks

- [x] **2.1** Add `get_bucket_acl()` function
  - API: `get_bucket_acl`
  - Purpose: Get bucket ACL for access audit
  - Returns: Owner, grants list with permissions

- [x] **2.2** Add `get_public_access_block()` function
  - API: `get_public_access_block`
  - Purpose: Check public access block settings
  - Returns: BlockPublicAcls, IgnorePublicAcls, BlockPublicPolicy, RestrictPublicBuckets

- [x] **2.3** Add `get_bucket_replication()` function
  - API: `get_bucket_replication_configuration`
  - Purpose: Get cross-region/account replication rules
  - Returns: Replication rules with destinations

- [x] **2.4** Add `get_bucket_cors()` function
  - API: `get_bucket_cors`
  - Purpose: Get CORS configuration
  - Returns: CORS rules with allowed origins/methods

- [x] **2.5** Add `get_bucket_website()` function
  - API: `get_bucket_website`
  - Purpose: Check static website hosting enabled
  - Returns: Index document, error document, redirect rules

- [x] **2.6** Add `get_bucket_notification()` function
  - API: `get_bucket_notification_configuration`
  - Purpose: Get event notification config
  - Returns: Lambda, SQS, SNS notification configs

- [x] **2.7** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new S3 calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 2 - S3

**Prerequisites**:
- AWS credentials configured with S3 read permissions
- At least one S3 bucket (ideally with various configurations)

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Navigate to Resource Explorer
   - Select resource type: `AWS::S3::Bucket`
   - Run query to list buckets

3. **Test Bucket ACL**
   - Click on any S3 bucket
   - Verify the detail panel shows:
     - [ ] Bucket owner information
     - [ ] Grant list with grantee and permissions

4. **Test Public Access Block**
   - In the bucket detail panel, verify:
     - [ ] BlockPublicAcls setting (true/false)
     - [ ] IgnorePublicAcls setting
     - [ ] BlockPublicPolicy setting
     - [ ] RestrictPublicBuckets setting

5. **Test Replication Configuration** (if bucket has replication)
   - Find a bucket with cross-region replication enabled
   - Verify:
     - [ ] Replication rules are displayed
     - [ ] Destination bucket ARN is shown

6. **Test CORS Configuration** (if bucket has CORS)
   - Find a bucket with CORS rules
   - Verify:
     - [ ] Allowed origins are listed
     - [ ] Allowed methods (GET, PUT, etc.) are shown

7. **Test Website Configuration** (if bucket is website-enabled)
   - Find a static website bucket
   - Verify:
     - [ ] Index document name is shown
     - [ ] Error document (if configured) is shown

8. **Test Notification Configuration** (if bucket has notifications)
   - Find a bucket with event notifications
   - Verify:
     - [ ] Lambda function ARNs (if configured)
     - [ ] SQS queue ARNs (if configured)
     - [ ] SNS topic ARNs (if configured)
     - [ ] Event types for each destination

9. **Test Bucket Without Configurations**
   - Select a basic bucket with no special configs
   - Verify: App handles missing configs gracefully (shows "Not configured" or similar)

**Expected Results**:
- All S3 security configurations display correctly
- Buckets without specific configurations show appropriate "not configured" state
- No crashes when accessing buckets with restricted permissions

---

## Milestone 3: Lambda Function Details

**File**: `src/app/resource_explorer/aws_services/lambda.rs`

### Tasks

- [x] **3.1** Add `get_function_configuration()` function
  - API: `get_function_configuration`
  - Purpose: Get detailed function configuration
  - Returns: Runtime, handler, memory, timeout, VPC config, layers, environment

- [x] **3.2** Add `get_function_policy()` function
  - API: `get_policy`
  - Purpose: Get resource-based policy
  - Returns: Policy document JSON

- [x] **3.3** Add `get_function_concurrency()` function
  - API: `get_function_concurrency`
  - Purpose: Get reserved concurrency setting
  - Returns: Reserved concurrent executions count

- [x] **3.4** Add `list_function_url_configs()` function
  - API: `list_function_url_configs`
  - Purpose: Get function URL endpoints
  - Returns: Function URLs with auth type, CORS config

- [x] **3.5** Add `get_function_code_signing()` function
  - API: `get_function_code_signing_config`
  - Purpose: Get code signing configuration
  - Returns: Allowed publishers, signing profile ARNs

- [x] **3.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new Lambda calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 3 - Lambda

**Prerequisites**:
- AWS credentials configured with Lambda read permissions
- At least one Lambda function deployed

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::Lambda::Function`
   - Run query to list functions

3. **Test Function Configuration**
   - Click on any Lambda function
   - Verify the detail panel shows:
     - [ ] Runtime (e.g., python3.9, nodejs18.x)
     - [ ] Handler name
     - [ ] Memory size (MB)
     - [ ] Timeout (seconds)
     - [ ] Environment variables (keys, not values for security)

4. **Test VPC Configuration** (if function is in VPC)
   - Find a VPC-connected function
   - Verify:
     - [ ] VPC ID is shown
     - [ ] Subnet IDs are listed
     - [ ] Security group IDs are listed

5. **Test Layers** (if function has layers)
   - Find a function with Lambda layers
   - Verify:
     - [ ] Layer ARNs are displayed
     - [ ] Layer versions are shown

6. **Test Resource Policy**
   - Click on a function with a resource-based policy (e.g., API Gateway trigger)
   - Verify:
     - [ ] Policy document JSON is displayed
     - [ ] Principal and action statements are visible

7. **Test Concurrency Settings**
   - Find a function with reserved concurrency (or create one for testing)
   - Verify:
     - [ ] Reserved concurrent executions count is shown
     - [ ] Functions without reserved concurrency show appropriate state

8. **Test Function URLs** (if any function has URL enabled)
   - Find a function with Function URL enabled
   - Verify:
     - [ ] Function URL endpoint is displayed
     - [ ] Auth type (AWS_IAM or NONE) is shown
     - [ ] CORS configuration (if enabled) is displayed

9. **Test Function Without Policy**
   - Select a function with no resource-based policy
   - Verify: App shows "No resource policy" or similar, doesn't error

**Expected Results**:
- All Lambda configurations display correctly
- VPC, layer, and policy information is accurate
- Functions without optional configurations handled gracefully

---

## Milestone 4: KMS Key Security

**File**: `src/app/resource_explorer/aws_services/kms.rs`

### Tasks

- [x] **4.1** Add `get_key_policy()` function
  - API: `get_key_policy`
  - Purpose: Get key policy document
  - Returns: Policy document JSON

- [x] **4.2** Add `get_key_rotation_status()` function
  - API: `get_key_rotation_status`
  - Purpose: Check automatic key rotation enabled
  - Returns: Boolean rotation status

- [x] **4.3** Add `list_key_policies()` function
  - API: `list_key_policies`
  - Purpose: List policy names (usually just "default")
  - Returns: Vec of policy names

- [x] **4.4** Add `list_grants()` function
  - API: `list_grants`
  - Purpose: Get key grants for access audit
  - Returns: Grant ID, grantee principal, operations, constraints

- [x] **4.5** Add `list_aliases()` function
  - API: `list_aliases`
  - Purpose: Get key aliases
  - Returns: Alias names and target key IDs

- [x] **4.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new KMS calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 4 - KMS

**Prerequisites**:
- AWS credentials configured with KMS read permissions
- At least one customer-managed KMS key

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::KMS::Key`
   - Run query to list KMS keys

3. **Test Key Policy**
   - Click on any customer-managed KMS key
   - Verify the detail panel shows:
     - [ ] Key policy document JSON
     - [ ] Policy statements with principals and actions

4. **Test Key Rotation Status**
   - In the key detail panel, verify:
     - [ ] Automatic rotation status (Enabled/Disabled)
     - [ ] Note: AWS-managed keys always show rotation enabled

5. **Test Key Policies List**
   - Verify:
     - [ ] Policy names are listed (usually just "default")

6. **Test Key Grants** (if key has grants)
   - Find a key with grants (e.g., used by EBS, RDS)
   - Verify:
     - [ ] Grant IDs are displayed
     - [ ] Grantee principal ARNs are shown
     - [ ] Operations (Encrypt, Decrypt, etc.) are listed
     - [ ] Constraints (if any) are displayed

7. **Test Key Aliases**
   - Verify for keys with aliases:
     - [ ] Alias name (e.g., alias/my-key)
     - [ ] Target key ID

8. **Test AWS-Managed Keys**
   - Click on an AWS-managed key (e.g., aws/s3, aws/ebs)
   - Verify:
     - [ ] Key info displays (some details may be restricted)
     - [ ] App handles permission limitations gracefully

**Expected Results**:
- Key policies display as formatted JSON
- Rotation status accurately reflects AWS console
- Grants and aliases display correctly
- AWS-managed keys handled appropriately

---

## Milestone 5: SQS Queue Details

**File**: `src/app/resource_explorer/aws_services/sqs.rs`

### Tasks

- [x] **5.1** Enhance `get_queue_attributes()` function
  - Already exists, add missing attributes:
  - Add: KmsMasterKeyId, KmsDataKeyReusePeriodSeconds
  - Add: Policy (queue access policy)
  - Add: SqsManagedSseEnabled

- [x] **5.2** Add `list_queue_tags()` function
  - API: `list_queue_tags`
  - Purpose: Get queue tags
  - Returns: Map of tag key-value pairs

- [x] **5.3** Add `list_dead_letter_source_queues()` function
  - API: `list_dead_letter_source_queues`
  - Purpose: Find queues using this queue as DLQ
  - Returns: Vec of source queue URLs

- [x] **5.4** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new SQS calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 5 - SQS

**Prerequisites**:
- AWS credentials configured with SQS read permissions
- At least one SQS queue (ideally with encryption and DLQ configured)

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::SQS::Queue`
   - Run query to list queues

3. **Test Enhanced Queue Attributes**
   - Click on any SQS queue
   - Verify the detail panel shows:
     - [ ] Existing attributes (message count, visibility timeout, etc.)
     - [ ] KMS Master Key ID (if encrypted with CMK)
     - [ ] KMS Data Key Reuse Period
     - [ ] SQS Managed SSE status
     - [ ] Queue access policy JSON

4. **Test Queue Tags**
   - Find a queue with tags
   - Verify:
     - [ ] All tag key-value pairs are displayed
     - [ ] Tags match what's shown in AWS console

5. **Test Dead Letter Source Queues**
   - Find a queue configured as a DLQ
   - Verify:
     - [ ] Source queue URLs are listed
     - [ ] Queues without DLQ sources show empty list

6. **Test Queue Without Encryption**
   - Select a queue without KMS encryption
   - Verify:
     - [ ] KMS fields show "Not configured" or similar
     - [ ] SqsManagedSseEnabled shows correct status

7. **Test Queue Policy**
   - Find a queue with an access policy
   - Verify:
     - [ ] Policy JSON is displayed and formatted
     - [ ] Queues without policies show appropriate message

**Expected Results**:
- All SQS attributes display correctly including new encryption fields
- Tags display as key-value pairs
- DLQ relationships are accurately shown
- Queues without optional configs handled gracefully

---

## Milestone 6: SNS Topic Details

**File**: `src/app/resource_explorer/aws_services/sns.rs`

### Tasks

- [x] **6.1** Add `get_topic_attributes()` function
  - API: `get_topic_attributes`
  - Purpose: Get encryption, policy, delivery settings
  - Returns: KmsMasterKeyId, Policy, DisplayName, SubscriptionsConfirmed

- [x] **6.2** Add `list_subscriptions_by_topic()` function
  - API: `list_subscriptions_by_topic`
  - Purpose: Get topic subscriptions
  - Returns: Subscription ARN, protocol, endpoint, owner

- [x] **6.3** Add `get_subscription_attributes()` function
  - API: `get_subscription_attributes`
  - Purpose: Get subscription details
  - Returns: FilterPolicy, RawMessageDelivery, RedrivePolicy

- [x] **6.4** Add `list_tags_for_resource()` function
  - API: `list_tags_for_resource`
  - Purpose: Get topic/subscription tags
  - Returns: Vec of tag key-value pairs

- [x] **6.5** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new SNS calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 6 - SNS

**Prerequisites**:
- AWS credentials configured with SNS read permissions
- At least one SNS topic with subscriptions

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::SNS::Topic`
   - Run query to list topics

3. **Test Topic Attributes**
   - Click on any SNS topic
   - Verify the detail panel shows:
     - [ ] Topic ARN
     - [ ] Display name (if set)
     - [ ] KMS Master Key ID (if encrypted)
     - [ ] Subscriptions confirmed count
     - [ ] Subscriptions pending count
     - [ ] Topic access policy JSON

4. **Test Topic Subscriptions**
   - In the topic detail panel, verify:
     - [ ] List of subscriptions
     - [ ] Subscription protocol (email, sqs, lambda, etc.)
     - [ ] Subscription endpoint
     - [ ] Subscription ARN

5. **Test Subscription Attributes**
   - Click on a specific subscription (if UI supports)
   - Verify:
     - [ ] Filter policy (if configured)
     - [ ] Raw message delivery setting
     - [ ] Redrive policy (if configured for DLQ)

6. **Test Topic Tags**
   - Find a topic with tags
   - Verify:
     - [ ] All tag key-value pairs are displayed

7. **Test Topic Without Subscriptions**
   - Find or create a topic with no subscriptions
   - Verify:
     - [ ] Empty subscription list displayed gracefully

8. **Test Topic Without Encryption**
   - Select an unencrypted topic
   - Verify:
     - [ ] KMS field shows "Not configured" or similar

**Expected Results**:
- Topic attributes including encryption and policy display correctly
- All subscriptions listed with correct protocols and endpoints
- Subscription filter policies (if any) are visible
- Tags display correctly

---

## Milestone 7: Cognito Identity Details

**File**: `src/app/resource_explorer/aws_services/cognito.rs`

### Tasks

- [x] **7.1** Add `describe_user_pool()` function
  - API: `describe_user_pool`
  - Purpose: Get user pool configuration
  - Returns: Policies, MFA config, schema, lambda triggers

- [x] **7.2** Add `describe_identity_pool()` function
  - API: `describe_identity_pool`
  - Purpose: Get identity pool configuration
  - Returns: Auth providers, roles, allow unauthenticated

- [x] **7.3** Add `get_user_pool_mfa_config()` function
  - API: `get_user_pool_mfa_config`
  - Purpose: Check MFA settings
  - Returns: MFA mode (OFF/ON/OPTIONAL), SMS/TOTP config

- [x] **7.4** Add `describe_user_pool_client()` function
  - API: `describe_user_pool_client`
  - Purpose: Get app client configuration
  - Returns: OAuth flows, scopes, callback URLs, token validity

- [x] **7.5** Add `list_user_pool_clients()` function
  - API: `list_user_pool_clients`
  - Purpose: List app clients in pool
  - Returns: Client IDs and names

- [x] **7.6** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new Cognito calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 7 - Cognito

**Prerequisites**:
- AWS credentials configured with Cognito read permissions
- At least one Cognito User Pool and/or Identity Pool

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::Cognito::UserPool`
   - Run query to list user pools

3. **Test User Pool Details**
   - Click on any User Pool
   - Verify the detail panel shows:
     - [ ] Pool ID and name
     - [ ] Password policy (min length, requirements)
     - [ ] Schema attributes
     - [ ] Lambda triggers (if configured)

4. **Test MFA Configuration**
   - In the user pool detail panel, verify:
     - [ ] MFA mode (OFF, ON, OPTIONAL)
     - [ ] SMS MFA configuration (if enabled)
     - [ ] TOTP MFA configuration (if enabled)

5. **Test User Pool Clients**
   - Verify list of app clients:
     - [ ] Client IDs
     - [ ] Client names
   - Click on a specific client to see:
     - [ ] OAuth flows (implicit, code, client_credentials)
     - [ ] OAuth scopes
     - [ ] Callback URLs
     - [ ] Token validity settings

6. **Test Identity Pool** (if available)
   - Select resource type: `AWS::Cognito::IdentityPool`
   - Click on an identity pool
   - Verify:
     - [ ] Identity pool ID and name
     - [ ] Allow unauthenticated identities setting
     - [ ] Authentication providers (Cognito, social, SAML, etc.)
     - [ ] IAM roles (authenticated and unauthenticated)

7. **Test Pool Without Optional Configs**
   - Find a basic user pool with minimal configuration
   - Verify: Missing optional fields handled gracefully

**Expected Results**:
- User pool configurations display correctly
- MFA settings accurately reflect AWS console
- App clients with OAuth configs display all settings
- Identity pool auth providers and roles are visible

---

## Milestone 8: CodeCommit Repository Details

**File**: `src/app/resource_explorer/aws_services/codecommit.rs`

### Tasks

- [x] **8.1** Add `get_repository()` function
  - API: `get_repository`
  - Purpose: Get repository details
  - Returns: Repo name, ARN, clone URLs, default branch, creation date

- [x] **8.2** Add `get_repository_triggers()` function
  - API: `get_repository_triggers`
  - Purpose: Get repository triggers
  - Returns: Trigger name, destination ARN, events, branches

- [x] **8.3** Add `list_branches()` function
  - API: `list_branches`
  - Purpose: List repository branches
  - Returns: Vec of branch names

- [x] **8.4** Add `get_branch()` function
  - API: `get_branch`
  - Purpose: Get branch details
  - Returns: Branch name, commit ID

- [x] **8.5** Update documentation
  - Update `docs/technical/aws-api-calls-inventory.md` with new CodeCommit calls

### Pre-Test Verification

Before requesting user testing, ensure:

- [ ] **Code compiles with zero errors**: `cargo build`
- [ ] **Code has zero warnings**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] **Code is formatted**: `cargo fmt --all --check`
- [ ] **Fast tests pass**: `./scripts/test-chunks.sh fast`

### User Testing: Milestone 8 - CodeCommit

**Prerequisites**:
- AWS credentials configured with CodeCommit read permissions
- At least one CodeCommit repository

**Step-by-Step Testing**:

1. **Launch the application**
   ```bash
   cargo run
   ```

2. **Open Resource Explorer**
   - Select resource type: `AWS::CodeCommit::Repository`
   - Run query to list repositories

3. **Test Repository Details**
   - Click on any CodeCommit repository
   - Verify the detail panel shows:
     - [ ] Repository name
     - [ ] Repository ARN
     - [ ] Clone URL (HTTPS)
     - [ ] Clone URL (SSH)
     - [ ] Default branch name
     - [ ] Creation date

4. **Test Repository Triggers** (if configured)
   - Find a repository with triggers
   - Verify:
     - [ ] Trigger names
     - [ ] Destination ARNs (Lambda, SNS)
     - [ ] Events that trigger (push, etc.)
     - [ ] Branches filter (if configured)

5. **Test Branch Listing**
   - In the repository detail panel, verify:
     - [ ] List of all branches
     - [ ] Branch names displayed correctly

6. **Test Branch Details**
   - Click on a specific branch (if UI supports)
   - Verify:
     - [ ] Branch name
     - [ ] Latest commit ID

7. **Test Repository Without Triggers**
   - Select a repository with no triggers configured
   - Verify:
     - [ ] Empty trigger list displayed gracefully

8. **Test Empty Repository**
   - If you have an empty repository (no commits)
   - Verify:
     - [ ] App handles empty repos gracefully
     - [ ] No default branch (or shows appropriate message)

**Expected Results**:
- Repository details including clone URLs display correctly
- All branches are listed
- Triggers (if any) show correct destinations and events
- Empty or trigger-less repos handled gracefully

---

## Implementation Pattern - CRITICAL GUIDELINES

### Overview

When implementing resource details, ALL detail functions MUST be called during the `list_*` operation. This ensures that when resources are queried, all their details are fetched in one pass.

**Key Principle**: The `list_*` function is the main entry point. After getting the basic resource list, it must call all detail functions for EACH resource before returning.

### Required Imports

```rust
use super::super::status::{report_status, report_status_done};
use std::time::Duration;
use tokio::time::timeout;
```

### Pattern 1: List Function with Integrated Detail Queries

The `list_*` function must:
1. Create the AWS client ONCE
2. Get the basic resource list
3. For EACH resource, call all detail functions
4. Merge detail data into the resource JSON
5. Handle errors gracefully (log and continue, don't fail the whole list)

```rust
pub async fn list_resources(
    &self,
    account_id: &str,
    region: &str,
) -> Result<Vec<serde_json::Value>> {
    report_status("ServiceName", "list_resources", Some(region));

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

    // Create client ONCE - reuse for all detail queries
    let client = service::Client::new(&aws_config);
    let mut paginator = client.list_resources().into_paginator().send();

    let mut resources = Vec::new();
    while let Some(page) = paginator.next().await {
        let page = page?;
        if let Some(resource_list) = page.resources {
            for resource in resource_list {
                let mut resource_json = self.resource_to_json(&resource);

                // Get resource identifier for additional queries
                if let Some(resource_id) = &resource.id {
                    if let serde_json::Value::Object(ref mut details) = resource_json {

                        // === CALL ALL DETAIL FUNCTIONS HERE ===

                        // Detail function 1: Get policy
                        report_status("ServiceName", "get_policy", Some(resource_id));
                        match self.get_policy_internal(&client, resource_id).await {
                            Ok(policy) => {
                                details.insert("Policy".to_string(), policy);
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Could not get policy for {}: {}",
                                    resource_id, e
                                );
                            }
                        }

                        // Detail function 2: Get tags
                        report_status("ServiceName", "list_tags", Some(resource_id));
                        match self.list_tags_internal(&client, resource_id).await {
                            Ok(tags) => {
                                details.insert("Tags".to_string(), tags);
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Could not get tags for {}: {}",
                                    resource_id, e
                                );
                            }
                        }

                        // Add more detail functions as needed...
                    }
                }

                resources.push(resource_json);
            }
        }
    }

    report_status_done("ServiceName", "list_resources", Some(region));
    Ok(resources)
}
```

### Pattern 2: Internal Detail Functions

Create `_internal` versions of detail functions that take a `&Client` reference. This avoids recreating the client for each detail query.

```rust
// Internal version - takes client reference, used by list_* function
async fn get_policy_internal(
    &self,
    client: &service::Client,
    resource_id: &str,
) -> Result<serde_json::Value> {
    // Use timeout to prevent hanging
    let response = timeout(
        Duration::from_secs(10),
        client.get_policy().resource_id(resource_id).send(),
    )
    .await
    .with_context(|| "get_policy timed out")?;

    match response {
        Ok(result) => {
            let mut json = serde_json::Map::new();
            if let Some(policy) = result.policy {
                // Try to parse policy as JSON, fallback to string
                if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&policy) {
                    json.insert("Policy".to_string(), policy_json);
                } else {
                    json.insert("Policy".to_string(), serde_json::Value::String(policy));
                }
            }
            Ok(serde_json::Value::Object(json))
        }
        Err(e) => {
            let error_str = format!("{:?}", e);
            // Handle "not found" gracefully - return null, not error
            if error_str.contains("ResourceNotFoundException")
                || error_str.contains("NoSuchEntity")
                || error_str.contains("NotFoundException") {
                Ok(serde_json::json!({
                    "Policy": null,
                    "Note": "No policy configured"
                }))
            } else {
                Err(anyhow::anyhow!("Failed to get policy: {}", e))
            }
        }
    }
}
```

### Pattern 3: Public Detail Functions (for describe_* calls)

Keep public versions that create their own client for standalone use:

```rust
// Public version - creates own client, used by describe_* function
pub async fn get_policy(
    &self,
    account_id: &str,
    region: &str,
    resource_id: &str,
) -> Result<serde_json::Value> {
    report_status("ServiceName", "get_policy", Some(resource_id));

    let aws_config = self
        .credential_coordinator
        .create_aws_config_for_account(account_id, region)
        .await?;

    let client = service::Client::new(&aws_config);
    let result = self.get_policy_internal(&client, resource_id).await;

    report_status_done("ServiceName", "get_policy", Some(resource_id));
    result
}
```

### Error Handling Guidelines

1. **Never fail the entire list** because one detail query failed
2. **Use `tracing::debug!`** to log failures, not `tracing::error!`
3. **Handle "not found" errors gracefully** - return null/empty, not error
4. **Use timeouts** (10 seconds recommended) to prevent hanging
5. **Check for common error patterns**:
   - `ResourceNotFoundException`
   - `NoSuchEntity`
   - `NotFoundException`
   - `AccessDeniedException` (permission issues)

### Status Reporting

Always use status reporting for visibility:

```rust
// Before starting operation
report_status("ServiceName", "operation_name", Some(resource_id));

// After completing (in list functions)
report_status_done("ServiceName", "list_operation", Some(region));
```

### Checklist for Each Service Implementation

- [ ] Add imports: `report_status`, `report_status_done`, `Duration`, `timeout`
- [ ] Create `_internal` versions of all detail functions
- [ ] Update `list_*` function to call ALL detail functions for each resource
- [ ] Add status reporting for each API call
- [ ] Use 10-second timeouts on all API calls
- [ ] Handle "not found" errors gracefully (return null, not error)
- [ ] Log failures with `tracing::debug!`, not `tracing::error!`
- [ ] Update documentation in `aws-api-calls-inventory.md`

### Common AWS SDK Type Gotchas

When working with AWS SDK response types, be careful:

```rust
// WRONG - field may not be Option<T>
if let Some(value) = response.field { ... }

// CHECK the actual type first! Some fields are:
// - Option<T> - use if let Some()
// - T directly (String, i64, Vec) - use directly
// - Empty string/vec for "not set" - check with is_empty()

// Example: Lambda layer.code_size is i64, not Option<i64>
if layer.code_size > 0 {
    json.insert("CodeSize", layer.code_size.into());
}

// Example: Response Vec fields are usually Vec, not Option<Vec>
for item in response.items {  // Not: if let Some(items) = response.items
    ...
}
```

## Testing Strategy

- Unit tests for JSON conversion functions
- Integration tests with real AWS responses (no mocks per CLAUDE.md)
- Test error handling for missing/inaccessible resources

## Progress Tracking

| Milestone | Service | Tasks | Completed |
|-----------|---------|-------|-----------|
| 1 | IAM | 9 | 9 |
| 2 | S3 | 7 | 7 |
| 3 | Lambda | 6 | 6 |
| 4 | KMS | 6 | 6 |
| 5 | SQS | 4 | 4 |
| 6 | SNS | 5 | 5 |
| 7 | Cognito | 6 | 6 |
| 8 | CodeCommit | 5 | 5 |
| **Total** | **8** | **48** | **48** |
