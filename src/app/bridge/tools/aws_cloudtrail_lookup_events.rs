//! AWS CloudTrail Lookup Events Tool
//!
//! This tool allows AI agents to query CloudTrail events from the 90-day event history
//! across multiple accounts and regions with various filtering options.

use crate::app::resource_explorer::aws_client::AWSResourceClient;
use crate::app::resource_explorer::aws_services::cloudtrail::{LookupAttribute, LookupEventsParams};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use serde_json;
use std::sync::Arc;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{info, warn};

use super::super::get_global_aws_client;

/// AWS CloudTrail Lookup Events Tool
#[derive(Clone)]
pub struct AwsCloudTrailLookupEventsTool {
    aws_client: Option<Arc<AWSResourceClient>>,
}

impl std::fmt::Debug for AwsCloudTrailLookupEventsTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsCloudTrailLookupEventsTool")
            .field("aws_client", &self.aws_client.is_some())
            .finish()
    }
}

impl AwsCloudTrailLookupEventsTool {
    pub fn new(aws_client: Option<Arc<AWSResourceClient>>) -> Self {
        Self { aws_client }
    }

    /// Create a new tool without AWS client (will be set later)
    pub fn new_uninitialized() -> Self {
        Self { aws_client: None }
    }

    /// Set the AWS client for this tool
    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = aws_client;
    }

    /// Parse parameter that can be either a string or an array of strings
    fn parse_string_or_array(value: &serde_json::Value) -> Result<Vec<String>, String> {
        match value {
            serde_json::Value::String(s) => Ok(vec![s.clone()]),
            serde_json::Value::Array(arr) => {
                let mut strings = Vec::new();
                for item in arr {
                    match item.as_str() {
                        Some(s) => strings.push(s.to_string()),
                        None => return Err("Array must contain only strings".to_string()),
                    }
                }
                if strings.is_empty() {
                    Err("Array cannot be empty".to_string())
                } else {
                    Ok(strings)
                }
            }
            _ => Err("Value must be a string or array of strings".to_string()),
        }
    }

    /// Parse time string into DateTime<Utc>
    fn parse_time_string(&self, time_str: &str) -> Result<DateTime<Utc>, String> {
        // Try different formats
        if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
            return Ok(dt.with_timezone(&Utc));
        }

        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
        }

        if let Ok(dt) = chrono::NaiveDate::parse_from_str(time_str, "%Y-%m-%d") {
            return Ok(DateTime::from_naive_utc_and_offset(
                dt.and_hms_opt(0, 0, 0).unwrap(),
                Utc,
            ));
        }

        // Handle relative time strings like "1 hour ago", "30 minutes ago", "7 days ago"
        if time_str.contains("ago") {
            let now = Utc::now();
            let parts: Vec<&str> = time_str.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[0].parse::<i64>() {
                    return match parts[1] {
                        "hour" | "hours" => Ok(now - Duration::hours(num)),
                        "minute" | "minutes" => Ok(now - Duration::minutes(num)),
                        "day" | "days" => Ok(now - Duration::days(num)),
                        "week" | "weeks" => Ok(now - Duration::weeks(num)),
                        _ => Err(format!("Unsupported time unit in: {}", time_str)),
                    };
                }
            }
        }

        Err(format!("Unable to parse time string: {}", time_str))
    }
}

#[async_trait]
impl Tool for AwsCloudTrailLookupEventsTool {
    fn name(&self) -> &str {
        "aws_cloudtrail_lookup_events"
    }

    fn description(&self) -> &str {
        "Query CloudTrail events from the 90-day event history across multiple AWS accounts and regions. \
         Supports filtering by event name, username, resource type, event source, and time range. \
         This queries the CloudTrail event history directly without requiring a trail to be configured. \
         Time formats: ISO 8601 (2024-01-01T00:00:00Z), YYYY-MM-DD HH:MM:SS, YYYY-MM-DD, \
         or relative times like '7 days ago', '1 hour ago'. \
         Returns up to 50 events per account/region by default."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "account_ids": {
                    "oneOf": [
                        {
                            "type": "string",
                            "description": "Single AWS account ID"
                        },
                        {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Array of AWS account IDs"
                        }
                    ],
                    "description": "AWS account ID(s) to query events from"
                },
                "regions": {
                    "oneOf": [
                        {
                            "type": "string",
                            "description": "Single AWS region"
                        },
                        {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Array of AWS regions"
                        }
                    ],
                    "description": "AWS region(s) to query events from"
                },
                "start_time": {
                    "type": "string",
                    "description": "Start time for event lookup (default: 90 days ago)",
                    "examples": ["2024-01-01T00:00:00Z", "2024-01-01", "7 days ago"]
                },
                "end_time": {
                    "type": "string",
                    "description": "End time for event lookup (default: now)",
                    "examples": ["2024-01-07T23:59:59Z", "2024-01-07", "1 hour ago"]
                },
                "attribute_key": {
                    "type": "string",
                    "enum": ["EventId", "EventName", "ReadOnly", "Username", "ResourceType", "ResourceName", "EventSource", "AccessKeyId"],
                    "description": "Attribute to filter events by"
                },
                "attribute_value": {
                    "type": "string",
                    "description": "Value to match for the specified attribute_key",
                    "examples": ["CreateBucket", "DeleteObject", "s3.amazonaws.com", "alice@example.com"]
                },
                "event_category": {
                    "type": "string",
                    "enum": ["management", "insight"],
                    "description": "Type of events to retrieve (default: management)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of events to return per account/region (default: 50, max: 1000)",
                    "minimum": 1,
                    "maximum": 1000
                }
            },
            "required": ["account_ids", "regions"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        info!("🔍 Executing AWS CloudTrail Lookup Events tool");

        // Get AWS client
        let global_client = get_global_aws_client();
        let aws_client = self
            .aws_client
            .as_ref()
            .or(global_client.as_ref())
            .ok_or_else(|| ToolError::ExecutionFailed {
                message: "AWS client not available. Please ensure AWS Explorer is initialized."
                    .to_string(),
            })?;

        // Parse parameters
        let params = parameters.unwrap_or_else(|| serde_json::json!({}));

        // Parse account IDs (required)
        let account_ids = params
            .get("account_ids")
            .map(Self::parse_string_or_array)
            .transpose()
            .map_err(|e| ToolError::InvalidParameters {
                message: format!("Invalid account_ids parameter: {}", e),
            })?
            .ok_or_else(|| ToolError::InvalidParameters {
                message: "account_ids parameter is required".to_string(),
            })?;

        // Parse regions (required)
        let regions = params
            .get("regions")
            .map(Self::parse_string_or_array)
            .transpose()
            .map_err(|e| ToolError::InvalidParameters {
                message: format!("Invalid regions parameter: {}", e),
            })?
            .ok_or_else(|| ToolError::InvalidParameters {
                message: "regions parameter is required".to_string(),
            })?;

        // Parse time parameters
        let start_time_param = if let Some(start_str) = params.get("start_time").and_then(|v| v.as_str()) {
            match self.parse_time_string(start_str) {
                Ok(dt) => Some(dt),
                Err(e) => {
                    return Err(ToolError::InvalidParameters {
                        message: format!("Invalid start_time: {}", e),
                    });
                }
            }
        } else {
            // Default to 90 days ago (CloudTrail's maximum retention)
            Some(Utc::now() - Duration::days(90))
        };

        let end_time_param = if let Some(end_str) = params.get("end_time").and_then(|v| v.as_str()) {
            match self.parse_time_string(end_str) {
                Ok(dt) => Some(dt),
                Err(e) => {
                    return Err(ToolError::InvalidParameters {
                        message: format!("Invalid end_time: {}", e),
                    });
                }
            }
        } else {
            // Default to now
            Some(Utc::now())
        };

        // Parse lookup attribute if provided
        let lookup_attribute = if let (Some(key), Some(value)) = (
            params.get("attribute_key").and_then(|v| v.as_str()),
            params.get("attribute_value").and_then(|v| v.as_str()),
        ) {
            Some(LookupAttribute {
                attribute_key: key.to_string(),
                attribute_value: value.to_string(),
            })
        } else {
            None
        };

        // Parse event category
        let event_category = params
            .get("event_category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse max results
        let max_results = params
            .get("max_results")
            .and_then(|v| v.as_i64())
            .map(|n| n as usize)
            .unwrap_or(50)
            .min(1000);

        info!(
            "📋 Looking up CloudTrail events for {} account(s) in {} region(s)",
            account_ids.len(),
            regions.len()
        );

        if let Some(ref attr) = lookup_attribute {
            info!(
                "   Filter: {} = {}",
                attr.attribute_key, attr.attribute_value
            );
        }

        // Get CloudTrail service
        let cloudtrail_service = aws_client.get_cloudtrail_service();

        // Execute parallel queries for all account/region combinations
        let mut futures = FuturesUnordered::new();

        for account_id in &account_ids {
            for region in &regions {
                let service = cloudtrail_service.clone();
                let account = account_id.clone();
                let region_str = region.clone();
                let params = LookupEventsParams {
                    start_time: start_time_param,
                    end_time: end_time_param,
                    lookup_attribute: lookup_attribute.clone(),
                    max_results,
                    event_category: event_category.clone(),
                };

                let future = async move {
                    let result = service.lookup_events(&account, &region_str, params).await;
                    (account, region_str, result)
                };
                futures.push(future);
            }
        }

        // Collect all results
        let mut all_events = Vec::new();
        let mut query_errors = Vec::new();
        let mut successful_queries = 0;

        while let Some((account_id, region, result)) = futures.next().await {
            match result {
                Ok(events) => {
                    info!(
                        "✅ Retrieved {} events from account {} in region {}",
                        events.len(),
                        account_id,
                        region
                    );
                    successful_queries += 1;

                    // Add account and region metadata to each event
                    for mut event in events {
                        if let Some(obj) = event.as_object_mut() {
                            obj.insert("AccountId".to_string(), serde_json::Value::String(account_id.clone()));
                            obj.insert("Region".to_string(), serde_json::Value::String(region.clone()));
                        }
                        all_events.push(event);
                    }
                }
                Err(e) => {
                    let error_msg = format!(
                        "Failed to query events for account {} in region {}: {}",
                        account_id, region, e
                    );
                    warn!("❌ {}", error_msg);
                    query_errors.push(error_msg);
                }
            }
        }

        // Sort all events by timestamp (most recent first)
        all_events.sort_by(|a, b| {
            let time_a = a
                .get("EventTime")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let time_b = b
                .get("EventTime")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            time_b.cmp(&time_a)
        });

        let duration = start_time.elapsed();
        let total_count = all_events.len();

        let execution_summary = if query_errors.is_empty() {
            format!(
                "Successfully retrieved {} CloudTrail events from {} account(s) across {} region(s) in {:.2}s",
                total_count,
                account_ids.len(),
                regions.len(),
                duration.as_secs_f64()
            )
        } else {
            format!(
                "Retrieved {} CloudTrail events from {}/{} successful queries in {:.2}s. {} queries failed.",
                total_count,
                successful_queries,
                account_ids.len() * regions.len(),
                duration.as_secs_f64(),
                query_errors.len()
            )
        };

        info!("✅ {}", execution_summary);

        let response = serde_json::json!({
            "success": !all_events.is_empty() || query_errors.is_empty(),
            "events_count": total_count,
            "events": all_events,
            "query_summary": {
                "accounts_queried": account_ids,
                "regions_queried": regions,
                "successful_queries": successful_queries,
                "failed_queries": query_errors.len(),
                "time_range": {
                    "start": start_time_param.map(|dt| dt.to_rfc3339()),
                    "end": end_time_param.map(|dt| dt.to_rfc3339())
                },
                "filter": lookup_attribute.as_ref().map(|attr| {
                    serde_json::json!({
                        "key": attr.attribute_key,
                        "value": attr.attribute_value
                    })
                }),
                "execution_time_seconds": duration.as_secs_f64()
            },
            "errors": if query_errors.is_empty() { None } else { Some(query_errors) },
            "message": execution_summary
        });

        Ok(ToolResult::success(response))
    }
}