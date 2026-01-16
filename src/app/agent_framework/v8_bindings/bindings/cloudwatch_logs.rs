//! CloudWatch Logs function bindings
//!
//! Provides JavaScript access to AWS CloudWatch Logs querying functionality.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::app::agent_framework::vfs::{get_current_vfs_id, with_vfs_mut};
use crate::app::data_plane::cloudwatch_logs::{CloudWatchLogsClient, QueryOptions};

/// JavaScript function call arguments for queryCloudWatchLogEvents()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryCloudWatchLogEventsArgs {
    /// Log group name (required)
    pub log_group_name: String,

    /// Account ID (required)
    pub account_id: String,

    /// AWS region (required)
    pub region: String,

    /// Start time (Unix milliseconds timestamp, optional)
    pub start_time: Option<i64>,

    /// End time (Unix milliseconds timestamp, optional)
    pub end_time: Option<i64>,

    /// CloudWatch Logs filter pattern (optional)
    pub filter_pattern: Option<String>,

    /// Maximum number of events to return (optional, default 100, max 10000)
    pub limit: Option<i32>,

    /// Specific log stream names to query (optional, empty = all streams)
    pub log_stream_names: Option<Vec<String>>,

    /// Query in chronological order (optional, default false = most recent first)
    pub start_from_head: Option<bool>,
}

/// Log event information exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEventInfo {
    /// Event timestamp (Unix milliseconds)
    pub timestamp: i64,

    /// Log message content
    pub message: String,

    /// Time when event was ingested (Unix milliseconds)
    pub ingestion_time: i64,

    /// Log stream name
    pub log_stream_name: String,
}

/// Query statistics exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryStatisticsInfo {
    /// Bytes scanned during query
    pub bytes_scanned: f64,

    /// Number of records that matched the filter
    pub records_matched: f64,

    /// Total number of records scanned
    pub records_scanned: f64,
}

/// CloudWatch Logs query result exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudWatchLogsQueryResult {
    /// Log events returned by the query (None when saved to VFS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<LogEventInfo>>,

    /// Token for pagination (if more results available)
    pub next_token: Option<String>,

    /// Total number of events in this result
    pub total_events: usize,

    /// Query statistics
    pub statistics: QueryStatisticsInfo,

    /// Path to full events in VFS (when VFS is used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details_path: Option<String>,

    /// Sample of log messages for context (when VFS is used)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_messages: Option<Vec<String>>,

    /// Message explaining how to access full data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Register CloudWatch Logs functions into V8 context
pub fn register(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Register queryCloudWatchLogEvents() function
    let query_fn = v8::Function::new(scope, query_cloudwatch_log_events_callback)
        .expect("Failed to create queryCloudWatchLogEvents function");

    let fn_name = v8::String::new(scope, "queryCloudWatchLogEvents")
        .expect("Failed to create function name string");
    global.set(scope, fn_name.into(), query_fn.into());

    Ok(())
}

/// Callback for queryCloudWatchLogEvents() JavaScript function
///
/// When VFS is available, saves log events to VFS and returns summary.
fn query_cloudwatch_log_events_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Parse JavaScript arguments
    let args_obj = match args.get(0).to_object(scope) {
        Some(obj) => obj,
        None => {
            let msg = v8::String::new(
                scope,
                "queryCloudWatchLogEvents() requires an object argument",
            )
            .unwrap();
            let error = v8::Exception::type_error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Convert V8 object to JSON string for parsing
    let json_str = match v8::json::stringify(scope, args_obj.into()) {
        Some(s) => s.to_rust_string_lossy(scope),
        None => {
            let msg = v8::String::new(scope, "Failed to stringify arguments").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse JSON into QueryCloudWatchLogEventsArgs
    let query_args: QueryCloudWatchLogEventsArgs = match serde_json::from_str(&json_str) {
        Ok(args) => args,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to parse arguments: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Execute async query
    let mut result = match execute_query(query_args) {
        Ok(result) => result,
        Err(e) => {
            let msg =
                v8::String::new(scope, &format!("CloudWatch Logs query failed: {}", e)).unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // If VFS is available and we have events, save to VFS and return summary
    if let Some(vfs_id) = get_current_vfs_id() {
        if let Some(ref events) = result.events {
            if !events.is_empty() {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let vfs_path = format!("/results/cloudwatch_logs_{}.json", timestamp);

                // Serialize events to JSON for VFS storage
                let events_json = match serde_json::to_string_pretty(events) {
                    Ok(json) => json,
                    Err(e) => {
                        warn!("Failed to serialize CloudWatch events for VFS: {}", e);
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
                                "Saved {} CloudWatch log events to VFS path: {}",
                                events.len(),
                                vfs_path
                            );

                            // Extract sample messages for context (first 3, truncated)
                            let sample_messages: Vec<String> = events
                                .iter()
                                .take(3)
                                .map(|e| {
                                    let msg = &e.message;
                                    if msg.len() > 100 {
                                        format!("{}...", &msg[..100])
                                    } else {
                                        msg.clone()
                                    }
                                })
                                .collect();

                            // Update result: remove inline events, add VFS path
                            result.events = None;
                            result.details_path = Some(vfs_path.clone());
                            result.sample_messages = Some(sample_messages);
                            result.message = Some(format!(
                                "Found {} log events. Full data saved to VFS. Use vfs.readJson('{}') to access.",
                                result.total_events,
                                vfs_path
                            ));
                        }
                        Some(Err(e)) => {
                            warn!("Failed to write CloudWatch events to VFS: {}", e);
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

    // Serialize result to JSON
    let result_json = match serde_json::to_string(&result) {
        Ok(json) => json,
        Err(e) => {
            let msg = v8::String::new(scope, &format!("Failed to serialize query result: {}", e))
                .unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Parse JSON string to V8 value
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

/// Execute CloudWatch Logs query using tokio runtime
pub fn execute_query(args: QueryCloudWatchLogEventsArgs) -> Result<CloudWatchLogsQueryResult> {
    // Use block_in_place to avoid nested runtime error
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { query_cloudwatch_logs_internal(args).await })
    })
}

/// Internal async implementation of CloudWatch Logs query
pub async fn query_cloudwatch_logs_internal(
    args: QueryCloudWatchLogEventsArgs,
) -> Result<CloudWatchLogsQueryResult> {
    info!(
        "Querying CloudWatch Logs: account={}, region={}, log_group={}",
        args.account_id, args.region, args.log_group_name
    );

    // Get global AWS client for credential coordinator
    let aws_client = crate::app::agent_framework::utils::registry::get_global_aws_client()
        .ok_or_else(|| anyhow!("AWS client not initialized"))?;

    let credential_coordinator = aws_client.get_credential_coordinator();

    // Create CloudWatch Logs client
    let logs_client = CloudWatchLogsClient::new(credential_coordinator);

    // Build query options
    let mut options = QueryOptions::new();

    if let Some(start_time) = args.start_time {
        options = options.with_start_time(start_time);
    }

    if let Some(end_time) = args.end_time {
        options = options.with_end_time(end_time);
    }

    if let Some(filter) = args.filter_pattern {
        options = options.with_filter_pattern(filter);
    }

    if let Some(limit) = args.limit {
        // Enforce maximum limit of 10000
        let safe_limit = limit.min(10000);
        options = options.with_limit(safe_limit);
    } else {
        // Default to 100 events
        options = options.with_limit(100);
    }

    if let Some(streams) = args.log_stream_names {
        options = options.with_log_stream_names(streams);
    }

    if let Some(from_head) = args.start_from_head {
        options = options.with_start_from_head(from_head);
    }

    // Execute query
    let result = logs_client
        .query_log_events(
            &args.account_id,
            &args.region,
            &args.log_group_name,
            options,
        )
        .await
        .map_err(|e| anyhow!("Failed to query CloudWatch Logs: {}", e))?;

    // Convert to V8-friendly format
    let events: Vec<LogEventInfo> = result
        .events
        .into_iter()
        .map(|event| LogEventInfo {
            timestamp: event.timestamp,
            message: event.message,
            ingestion_time: event.ingestion_time,
            log_stream_name: event.log_stream_name,
        })
        .collect();

    let statistics = QueryStatisticsInfo {
        bytes_scanned: result.query_statistics.bytes_scanned,
        records_matched: result.query_statistics.records_matched,
        records_scanned: result.query_statistics.records_scanned,
    };

    Ok(CloudWatchLogsQueryResult {
        events: Some(events),
        next_token: result.next_token,
        total_events: result.total_events,
        statistics,
        details_path: None,
        sample_messages: None,
        message: None,
    })
}

/// Get LLM documentation for CloudWatch Logs functions
pub fn get_documentation() -> String {
    r#"### queryCloudWatchLogEvents(params)

Query CloudWatch Logs for analysis and monitoring.

**Parameters** (object):
- `logGroupName` (string, required): Name of the log group to query
- `accountId` (string, required): AWS account ID
- `region` (string, required): AWS region code (e.g., "us-east-1")
- `startTime` (number, optional): Start time in Unix milliseconds (Date.now() format)
- `endTime` (number, optional): End time in Unix milliseconds
- `filterPattern` (string, optional): CloudWatch Logs filter pattern
- `limit` (number, optional): Max events to return (default: 100, max: 10000)
- `logStreamNames` (array of strings, optional): Specific log streams to query
- `startFromHead` (boolean, optional): Query chronologically (default: false, most recent first)

**Returns** (object):
- `events` (array): Log events matching the query
  - `timestamp` (number): Event timestamp in Unix milliseconds
  - `message` (string): Log message content
  - `ingestionTime` (number): When the event was ingested
  - `logStreamName` (string): Name of the log stream
- `nextToken` (string|null): Pagination token for fetching more results
- `totalEvents` (number): Number of events in this result
- `statistics` (object): Query performance statistics
  - `bytesScanned` (number): Bytes scanned during the query
  - `recordsMatched` (number): Number of records that matched
  - `recordsScanned` (number): Total records scanned

**Example - Get recent Lambda errors:**
```javascript
const errors = queryCloudWatchLogEvents({
  logGroupName: "/aws/lambda/my-function",
  accountId: "123456789012",
  region: "us-east-1",
  filterPattern: "ERROR",
  startTime: Date.now() - (60 * 60 * 1000), // Last hour
  limit: 100
});

console.log("Found " + errors.events.length + " errors");
errors.events.forEach(e => {
  console.log(new Date(e.timestamp).toISOString() + ": " + e.message);
});
```

**Example - Analyze API Gateway logs:**
```javascript
const apiLogs = queryCloudWatchLogEvents({
  logGroupName: "/aws/apigateway/my-api",
  accountId: "123456789012",
  region: "us-east-1",
  filterPattern: "[ip, timestamp, method, path, status>=400]",
  limit: 500
});

// Analyze error patterns
const errorPaths = {};
apiLogs.events.forEach(event => {
  const match = event.message.match(/path=([^ ]+)/);
  if (match) {
    errorPaths[match[1]] = (errorPaths[match[1]] || 0) + 1;
  }
});
```

**Filter Pattern Syntax:**
- Simple text: `ERROR` (matches lines containing "ERROR")
- JSON fields: `{ $.level = "ERROR" }` (for JSON-formatted logs)
- Structured: `[timestamp, request_id, level, msg]`
- Conditions: `[level=ERROR || level=FATAL]`
- Numeric: `[..., status>=400, ...]`

**Important Notes:**
- Queries are limited to 10,000 events maximum
- Large time ranges may be slow - use specific time windows
- Use filter patterns to reduce data scanned and improve performance
- Timestamps must be in Unix milliseconds (use `Date.now()`)
"#
    .to_string()
}
