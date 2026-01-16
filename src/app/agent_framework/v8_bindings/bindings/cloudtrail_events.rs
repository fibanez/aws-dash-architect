//! CloudTrail Events V8 JavaScript bindings
//!
//! Exposes CloudTrail event lookup functionality to the agent's JavaScript environment.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::app::agent_framework::vfs::{get_current_vfs_id, with_vfs_mut};
use crate::app::data_plane::cloudtrail_events::{
    CloudTrailEventsClient, LookupAttribute, LookupAttributeKey, LookupOptions,
};

/// JavaScript lookup attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupAttributeArgs {
    /// Attribute key (EventId, EventName, ResourceType, etc.)
    pub attribute_key: String,
    /// Attribute value
    pub attribute_value: String,
}

/// JavaScript function arguments for getCloudTrailEvents()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCloudTrailEventsArgs {
    /// AWS account ID (required)
    pub account_id: String,

    /// AWS region (required)
    pub region: String,

    /// Start time in Unix milliseconds (optional)
    pub start_time: Option<i64>,

    /// End time in Unix milliseconds (optional)
    pub end_time: Option<i64>,

    /// Lookup attributes for filtering (optional)
    pub lookup_attributes: Option<Vec<LookupAttributeArgs>>,

    /// Maximum number of results (optional, will fetch at least 100)
    pub max_results: Option<i32>,

    /// Pagination token (optional)
    pub next_token: Option<String>,
}

/// Event resource exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventResourceInfo {
    /// Resource type (e.g., "AWS::EC2::Instance")
    pub resource_type: Option<String>,
    /// Resource name
    pub resource_name: Option<String>,
}

/// CloudTrail event exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudTrailEventInfo {
    /// Event ID
    pub event_id: String,
    /// Event name (API operation)
    pub event_name: String,
    /// Event time (Unix milliseconds)
    pub event_time: i64,
    /// Event source (AWS service)
    pub event_source: String,
    /// Username (IAM user/role)
    pub username: String,
    /// Resources affected
    pub resources: Vec<EventResourceInfo>,
    /// Full CloudTrail event JSON (optional)
    pub cloud_trail_event: Option<String>,
    /// Access key ID
    pub access_key_id: Option<String>,
    /// Read-only event
    pub read_only: Option<String>,
    /// Error code (if failed)
    pub error_code: Option<String>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Result exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudTrailEventsResult {
    /// Events returned (None when saved to VFS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<CloudTrailEventInfo>>,
    /// Pagination token
    pub next_token: Option<String>,
    /// Total events in this result
    pub total_events: usize,
    /// Path to full events in VFS (when VFS is used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details_path: Option<String>,
    /// Sample of event names for context (when VFS is used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_event_names: Option<Vec<String>>,
    /// Message explaining how to access full data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Register CloudTrail Events functions into V8 context
pub fn register(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Register getCloudTrailEvents() function
    let get_events_fn = v8::Function::new(scope, get_cloudtrail_events_callback)
        .expect("Failed to create getCloudTrailEvents function");

    let fn_name = v8::String::new(scope, "getCloudTrailEvents")
        .expect("Failed to create function name string");

    global.set(scope, fn_name.into(), get_events_fn.into());

    Ok(())
}

/// V8 callback for getCloudTrailEvents() JavaScript function
fn get_cloudtrail_events_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Step 1: Parse JavaScript arguments
    let args_obj = match args.get(0).to_object(scope) {
        Some(obj) => obj,
        None => {
            let msg = v8::String::new(
                scope,
                "getCloudTrailEvents() requires an object argument with { accountId, region, ... }",
            )
            .unwrap();
            let error = v8::Exception::type_error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 2: Convert V8 object to JSON string
    let json_str = match v8::json::stringify(scope, args_obj.into()) {
        Some(s) => s.to_rust_string_lossy(scope),
        None => {
            let msg = v8::String::new(scope, "Failed to stringify arguments").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 3: Parse JSON into typed Args struct
    let lookup_args: GetCloudTrailEventsArgs = match serde_json::from_str(&json_str) {
        Ok(args) => args,
        Err(e) => {
            let msg = v8::String::new(
                scope,
                &format!(
                    "Failed to parse getCloudTrailEvents arguments: {}. Expected {{ accountId: string, region: string, ... }}",
                    e
                ),
            )
            .unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 4: Execute lookup (async operation in blocking context)
    let mut result = match execute_lookup(lookup_args) {
        Ok(result) => result,
        Err(e) => {
            let msg =
                v8::String::new(scope, &format!("CloudTrail events lookup failed: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 4.5: If VFS is available and we have events, save to VFS and return summary
    if let Some(vfs_id) = get_current_vfs_id() {
        if let Some(ref events) = result.events {
            if !events.is_empty() {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let vfs_path = format!("/results/cloudtrail_events_{}.json", timestamp);

                // Serialize events to JSON for VFS storage
                let events_json = match serde_json::to_string_pretty(events) {
                    Ok(json) => json,
                    Err(e) => {
                        warn!("Failed to serialize CloudTrail events for VFS: {}", e);
                        String::new()
                    }
                };

                if !events_json.is_empty() {
                    // Write to VFS
                    let write_result = with_vfs_mut(&vfs_id, |vfs| {
                        vfs.write_file(&vfs_path, events_json.as_bytes())
                    });

                    match write_result {
                        Some(Ok(())) => {
                            debug!(
                                "Saved {} CloudTrail events to VFS path: {}",
                                events.len(),
                                vfs_path
                            );

                            // Extract sample event names for context (first 5)
                            let sample_event_names: Vec<String> = events
                                .iter()
                                .take(5)
                                .map(|e| e.event_name.clone())
                                .collect();

                            // Update result: remove inline events, add VFS path
                            result.events = None;
                            result.details_path = Some(vfs_path.clone());
                            result.sample_event_names = Some(sample_event_names);
                            result.message = Some(format!(
                                "Found {} CloudTrail events. Full data saved to VFS. Use vfs.readJson('{}') to access.",
                                result.total_events,
                                vfs_path
                            ));
                        }
                        Some(Err(e)) => {
                            warn!("Failed to write CloudTrail events to VFS: {}", e);
                            // Fall back to inline return
                        }
                        None => {
                            warn!("VFS not found for id: {}", vfs_id);
                            // Fall back to inline return
                        }
                    }
                }
            }
        }
    }

    // Step 5: Serialize result to JSON
    let result_json = match serde_json::to_string(&result) {
        Ok(json) => json,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to serialize lookup result: {}", e))
                .unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 6: Parse JSON string back to V8 value and return
    let result_value = match v8::json::parse(scope, v8::String::new(scope, &result_json).unwrap()) {
        Some(val) => val,
        None => {
            let msg = v8::String::new(scope, "Failed to parse result JSON").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    rv.set(result_value);
}

/// Execute lookup using tokio runtime
pub fn execute_lookup(args: GetCloudTrailEventsArgs) -> Result<CloudTrailEventsResult> {
    // CRITICAL: Use block_in_place to avoid "Cannot start a runtime from within a runtime" error
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { get_cloudtrail_events_internal(args).await })
    })
}

/// Internal async implementation of CloudTrail events lookup
pub async fn get_cloudtrail_events_internal(
    args: GetCloudTrailEventsArgs,
) -> Result<CloudTrailEventsResult> {
    info!(
        "Looking up CloudTrail events: account={}, region={}, filters={}",
        args.account_id,
        args.region,
        args.lookup_attributes
            .as_ref()
            .map(|a| a.len())
            .unwrap_or(0)
    );

    // Get global AWS client for credential coordinator access
    let aws_client = crate::app::agent_framework::utils::registry::get_global_aws_client()
        .ok_or_else(|| anyhow!("AWS client not initialized"))?;

    let credential_coordinator = aws_client.get_credential_coordinator();

    // Create CloudTrail Events client
    let cloudtrail_client = CloudTrailEventsClient::new(credential_coordinator);

    // Build lookup options
    let mut options = LookupOptions::new();

    if let Some(start_time) = args.start_time {
        options = options.with_start_time(start_time);
    }

    if let Some(end_time) = args.end_time {
        options = options.with_end_time(end_time);
    }

    // Convert lookup attributes
    if let Some(attrs) = args.lookup_attributes {
        let lookup_attrs: Vec<LookupAttribute> = attrs
            .into_iter()
            .filter_map(|attr| {
                // Convert attribute key string to enum
                let key = match attr.attribute_key.as_str() {
                    "EventId" => LookupAttributeKey::EventId,
                    "EventName" => LookupAttributeKey::EventName,
                    "ReadOnly" => LookupAttributeKey::ReadOnly,
                    "Username" => LookupAttributeKey::Username,
                    "ResourceType" => LookupAttributeKey::ResourceType,
                    "ResourceName" => LookupAttributeKey::ResourceName,
                    "EventSource" => LookupAttributeKey::EventSource,
                    "AccessKeyId" => LookupAttributeKey::AccessKeyId,
                    _ => return None, // Skip unknown keys
                };

                Some(LookupAttribute::new(key, attr.attribute_value))
            })
            .collect();

        options = options.with_lookup_attributes(lookup_attrs);
    }

    let has_next_token = args.next_token.is_some();
    if let Some(token) = args.next_token {
        options = options.with_next_token(token);
    }

    // Execute lookup
    // Use get_recent_events for automatic pagination (at least 2 pages, 100 events)
    let result = if args.max_results.is_some() || has_next_token {
        // User specified max_results or next_token, use direct lookup
        if let Some(max_results) = args.max_results {
            let safe_max = max_results.min(50); // CloudTrail max per request
            options = options.with_max_results(safe_max);
        }
        cloudtrail_client
            .lookup_events(&args.account_id, &args.region, options)
            .await
            .map_err(|e| anyhow!("Failed to lookup CloudTrail events: {}", e))?
    } else {
        // Default: fetch at least 2 pages (100 events)
        cloudtrail_client
            .get_recent_events(&args.account_id, &args.region, 100)
            .await
            .map_err(|e| anyhow!("Failed to get recent CloudTrail events: {}", e))?
    };

    // Convert to V8-friendly format
    let events: Vec<CloudTrailEventInfo> = result
        .events
        .into_iter()
        .map(|event| {
            let resources: Vec<EventResourceInfo> = event
                .resources
                .into_iter()
                .map(|res| EventResourceInfo {
                    resource_type: res.resource_type,
                    resource_name: res.resource_name,
                })
                .collect();

            CloudTrailEventInfo {
                event_id: event.event_id,
                event_name: event.event_name,
                event_time: event.event_time,
                event_source: event.event_source,
                username: event.username,
                resources,
                cloud_trail_event: event.cloud_trail_event,
                access_key_id: event.access_key_id,
                read_only: event.read_only,
                error_code: event.error_code,
                error_message: event.error_message,
            }
        })
        .collect();

    Ok(CloudTrailEventsResult {
        events: Some(events),
        next_token: result.next_token,
        total_events: result.total_events,
        details_path: None,
        sample_event_names: None,
        message: None,
    })
}

/// Get LLM documentation for CloudTrail Events functions
pub fn get_documentation() -> String {
    r#"### getCloudTrailEvents(params)

Query AWS CloudTrail events for governance, compliance, and security analysis.

CloudTrail records all API calls made in your AWS account, providing an audit trail
of who did what, when, and from where.

**Parameters** (object):
- `accountId` (string, required): AWS account ID (12 digits)
- `region` (string, required): AWS region code (e.g., "us-east-1", "eu-west-1")
- `startTime` (number, optional): Start time in Unix milliseconds timestamp
  - Example: `Date.now() - (24 * 60 * 60 * 1000)` for last 24 hours
- `endTime` (number, optional): End time in Unix milliseconds timestamp
  - Example: `Date.now()` for current time
- `lookupAttributes` (array, optional): Filters for events
  - Each filter: `{ attributeKey: string, attributeValue: string }`
  - Attribute keys: "EventId", "EventName", "ResourceType", "ResourceName", "Username", "EventSource", "AccessKeyId", "ReadOnly"
- `maxResults` (number, optional): Maximum events to return
  - Default: Automatically fetches at least 100 events (2 pages)
  - Max per request: 50 (CloudTrail API limit)
- `nextToken` (string, optional): Pagination token from previous response

**Returns** (object):
- `events` (array): CloudTrail events matching the query
  - `eventId` (string): Unique event identifier
  - `eventName` (string): API operation (e.g., "RunInstances", "CreateBucket")
  - `eventTime` (number): Event timestamp in Unix milliseconds
  - `eventSource` (string): AWS service (e.g., "ec2.amazonaws.com")
  - `username` (string): IAM user/role that made the call
  - `resources` (array): Resources affected by this event
    - `resourceType` (string): CloudFormation resource type
    - `resourceName` (string): Resource identifier
  - `cloudTrailEvent` (string): Full event JSON (optional, large)
  - `accessKeyId` (string): Access key used
  - `readOnly` (string): "true" if read-only, "false" if write operation
  - `errorCode` (string): Error code if call failed
  - `errorMessage` (string): Error message if call failed
- `nextToken` (string|null): Pagination token (null if no more events)
- `totalEvents` (number): Number of events in this response

**Example 1 - Get recent events (auto-pagination, 100 events):**
```javascript
const result = getCloudTrailEvents({
  accountId: "123456789012",
  region: "us-east-1"
});

console.log("Found " + result.events.length + " recent events");
result.events.forEach(event => {
  const date = new Date(event.eventTime).toISOString();
  console.log(date + ": " + event.eventName + " by " + event.username);
});
```

**Example 2 - Track specific Lambda function by NAME:**
```javascript
// IMPORTANT: Use the function NAME, not the ARN
// CloudTrail will match this against events that reference this function
const result = getCloudTrailEvents({
  accountId: "123456789012",
  region: "us-east-1",
  lookupAttributes: [
    { attributeKey: "ResourceName", attributeValue: "my-lambda-function" }
  ]
});

console.log("Events for Lambda function 'my-lambda-function':");
result.events.forEach(event => {
  console.log("  " + event.eventName + " by " + event.username);
  // Note: event.resources will contain full ARNs like:
  // "arn:aws:lambda:us-east-1:123456789012:function:my-lambda-function"
});
```

**Example 3 - Track specific EC2 instance by ID:**
```javascript
// For EC2: use instance ID (like "i-1234567890abcdef0"), not ARN
const result = getCloudTrailEvents({
  accountId: "123456789012",
  region: "us-east-1",
  lookupAttributes: [
    { attributeKey: "ResourceName", attributeValue: "i-1234567890abcdef0" }
  ]
});

console.log("Events for EC2 instance i-1234567890abcdef0:");
result.events.forEach(event => {
  console.log("  " + event.eventName + " by " + event.username);
});
```

**Example 4 - Security audit (find failures):**
```javascript
const result = getCloudTrailEvents({
  accountId: "123456789012",
  region: "us-east-1",
  startTime: Date.now() - (7 * 24 * 60 * 60 * 1000) // Last 7 days
});

const failures = result.events.filter(e => e.errorCode);
console.log("Found " + failures.length + " failed API calls:");
failures.forEach(event => {
  console.log("  " + event.eventName + ": " + event.errorCode);
});
```

**Example 5 - IAM changes for compliance:**
```javascript
const result = getCloudTrailEvents({
  accountId: "123456789012",
  region: "us-east-1",
  lookupAttributes: [
    { attributeKey: "ResourceType", attributeValue: "AWS::IAM::Role" }
  ],
  startTime: Date.now() - (30 * 24 * 60 * 60 * 1000) // Last 30 days
});

console.log("IAM role changes in last 30 days:");
result.events.forEach(event => {
  const date = new Date(event.eventTime).toISOString();
  console.log(date + ": " + event.eventName);
});
```

**Example 6 - Pagination for large results:**
```javascript
let allEvents = [];
let nextToken = null;

do {
  const result = getCloudTrailEvents({
    accountId: "123456789012",
    region: "us-east-1",
    maxResults: 50,
    nextToken: nextToken
  });

  allEvents = allEvents.concat(result.events);
  nextToken = result.nextToken;
} while (nextToken !== null);

console.log("Total events: " + allEvents.length);
```

**CRITICAL ResourceName Format Rules:**
- **Lambda**: Use function NAME (e.g., "my-function"), NOT ARN
- **EC2**: Use instance ID (e.g., "i-1234567890abcdef0"), NOT ARN
- **S3**: Use bucket name (e.g., "my-bucket"), NOT ARN
- **IAM**: Use role/user name (e.g., "MyRole"), NOT ARN
- **Result**: Event resources contain FULL ARNs in the response

**CRITICAL API Limitations:**
- **ONLY ONE lookupAttribute allowed per query** (AWS CloudTrail API restriction)
- Cannot combine filters (e.g., cannot do ResourceType AND ResourceName together)
- If multiple attributes provided, only the LAST one is used
- To filter by multiple criteria, use broader filter then filter results in JavaScript

**Important Notes:**
- CloudTrail events may take 5-15 minutes to appear after the API call
- By default, fetches at least 100 events (2 pages) automatically for better coverage
- Maximum 50 events per request (CloudTrail API limit)
- Timestamps must be in Unix milliseconds (use `Date.now()`)
- CloudTrail supports ALL AWS resource types (all API calls are logged)
- Read-only operations: "true", Write operations: "false"
- Check `errorCode` to find failed API calls (potential security issues)
"#
    .to_string()
}
