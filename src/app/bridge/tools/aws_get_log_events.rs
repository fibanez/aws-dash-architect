//! AWS Get Log Events Tool
//!
//! This tool allows AI agents to retrieve log events from CloudWatch log groups and streams
//! with filtering capabilities including time ranges and filter patterns.

use crate::app::resource_explorer::aws_client::AWSResourceClient;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json;
use std::sync::Arc;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{info, warn};

use super::super::get_global_aws_client;

/// AWS Get Log Events Tool
#[derive(Clone)]
pub struct AwsGetLogEventsTool {
    aws_client: Option<Arc<AWSResourceClient>>,
}

impl std::fmt::Debug for AwsGetLogEventsTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsGetLogEventsTool")
            .field("aws_client", &self.aws_client.is_some())
            .finish()
    }
}

impl AwsGetLogEventsTool {
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

    /// Parse time string into DateTime<Utc>
    fn parse_time_string(&self, time_str: &str) -> Result<DateTime<Utc>, String> {
        // Try different formats
        if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
            return Ok(dt.with_timezone(&Utc));
        }

        if let Ok(dt) = DateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(dt.with_timezone(&Utc));
        }

        if let Ok(dt) = DateTime::parse_from_str(time_str, "%Y-%m-%d") {
            return Ok(dt.with_timezone(&Utc));
        }

        // Handle relative time strings like "1 hour ago", "30 minutes ago"
        if time_str.contains("ago") {
            let now = Utc::now();
            if time_str.contains("hour") {
                if let Some(hours_str) = time_str.split_whitespace().next() {
                    if let Ok(hours) = hours_str.parse::<i64>() {
                        return Ok(now - chrono::Duration::hours(hours));
                    }
                }
            }
            if time_str.contains("minute") {
                if let Some(minutes_str) = time_str.split_whitespace().next() {
                    if let Ok(minutes) = minutes_str.parse::<i64>() {
                        return Ok(now - chrono::Duration::minutes(minutes));
                    }
                }
            }
            if time_str.contains("day") {
                if let Some(days_str) = time_str.split_whitespace().next() {
                    if let Ok(days) = days_str.parse::<i64>() {
                        return Ok(now - chrono::Duration::days(days));
                    }
                }
            }
        }

        Err(format!("Unable to parse time string: {}", time_str))
    }
}

#[async_trait]
impl Tool for AwsGetLogEventsTool {
    fn name(&self) -> &str {
        "aws_get_log_events"
    }

    fn description(&self) -> &str {
        "Retrieve log events from CloudWatch log groups and streams with filtering capabilities. \
         Supports time range filtering, filter patterns, and log stream selection. \
         Time formats: ISO 8601 (2024-01-01T00:00:00Z), YYYY-MM-DD HH:MM:SS, YYYY-MM-DD, \
         or relative times like '1 hour ago', '30 minutes ago', '2 days ago'."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "log_group_name": {
                    "type": "string",
                    "description": "Name of the CloudWatch log group"
                },
                "account_id": {
                    "type": "string",
                    "description": "AWS account ID (optional, uses current account if not specified)"
                },
                "region": {
                    "type": "string",
                    "description": "AWS region (optional, uses current region if not specified)"
                },
                "log_stream_name": {
                    "type": "string",
                    "description": "Optional specific log stream name (if not provided, searches all streams)"
                },
                "start_time": {
                    "type": "string",
                    "description": "Start time for log event filtering (ISO 8601, YYYY-MM-DD, or relative like '1 hour ago')"
                },
                "end_time": {
                    "type": "string",
                    "description": "End time for log event filtering (ISO 8601, YYYY-MM-DD, or relative like '30 minutes ago')"
                },
                "filter_pattern": {
                    "type": "string",
                    "description": "CloudWatch filter pattern to match log messages"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of log events to return (default: 100, max: 1000)",
                    "minimum": 1,
                    "maximum": 1000
                }
            },
            "required": ["log_group_name"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        info!("🔍 Executing AWS Get Log Events tool");

        // Get AWS client - prefer passed client over global
        let aws_client = self
            .aws_client
            .clone()
            .or_else(get_global_aws_client)
            .ok_or_else(|| {
                warn!("❌ AWS client not available for get log events operation");
                ToolError::ExecutionFailed {
                    message: "AWS client not configured. Please ensure AWS credentials are set up"
                        .to_string(),
                }
            })?;

        // Parse parameters
        let params = parameters.unwrap_or_default();

        let log_group_name = params
            .get("log_group_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                warn!("❌ log_group_name parameter is required");
                ToolError::InvalidParameters {
                    message: "log_group_name parameter is required. Specify the CloudWatch log group name to retrieve events from".to_string(),
                }
            })?;

        let account_id = params
            .get("account_id")
            .and_then(|v| v.as_str())
            .unwrap_or("current"); // Use 'current' as placeholder - tools should specify account

        let region = params
            .get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("us-east-1"); // Default to us-east-1 if not specified

        let log_stream_name = params.get("log_stream_name").and_then(|v| v.as_str());

        let filter_pattern = params.get("filter_pattern").and_then(|v| v.as_str());

        let limit = params
            .get("limit")
            .and_then(|v| v.as_i64())
            .map(|l| l as i32)
            .unwrap_or(100)
            .min(1000);

        // Parse time parameters
        let start_time = if let Some(start_str) = params.get("start_time").and_then(|v| v.as_str())
        {
            match self.parse_time_string(start_str) {
                Ok(dt) => Some(dt),
                Err(e) => {
                    warn!("❌ Failed to parse start_time: {}", e);
                    return Err(ToolError::InvalidParameters {
                        message: format!("Invalid start_time format: {}. Use ISO 8601 format, YYYY-MM-DD, or relative time like '1 hour ago'", e),
                    });
                }
            }
        } else {
            None
        };

        let end_time = if let Some(end_str) = params.get("end_time").and_then(|v| v.as_str()) {
            match self.parse_time_string(end_str) {
                Ok(dt) => Some(dt),
                Err(e) => {
                    warn!("❌ Failed to parse end_time: {}", e);
                    return Err(ToolError::InvalidParameters {
                        message: format!("Invalid end_time format: {}. Use ISO 8601 format, YYYY-MM-DD, or relative time like '30 minutes ago'", e),
                    });
                }
            }
        } else {
            None
        };

        info!("📋 Getting log events from group: {}, account: {}, region: {}, stream: {:?}, limit: {}", 
              log_group_name, account_id, region, log_stream_name, limit);

        // Get logs service and retrieve log events
        let logs_service = aws_client.get_logs_service();

        match logs_service
            .get_log_events(
                account_id,
                region,
                log_group_name,
                log_stream_name,
                start_time,
                end_time,
                filter_pattern,
                Some(limit),
            )
            .await
        {
            Ok(log_events) => {
                let count = log_events.len();

                info!("✅ Successfully retrieved {} log events", count);

                let response = serde_json::json!({
                    "success": true,
                    "account_id": account_id,
                    "region": region,
                    "log_group_name": log_group_name,
                    "log_stream_name": log_stream_name,
                    "events_count": count,
                    "events": log_events,
                    "query_parameters": {
                        "start_time": start_time.map(|dt| dt.to_rfc3339()),
                        "end_time": end_time.map(|dt| dt.to_rfc3339()),
                        "filter_pattern": filter_pattern,
                        "limit": limit
                    }
                });

                Ok(ToolResult::success(response))
            }
            Err(e) => {
                warn!("❌ Failed to get log events: {}", e);
                Err(ToolError::ExecutionFailed {
                    message: format!(
                        "Failed to get log events from {} in account {} region {}: {}",
                        log_group_name, account_id, region, e
                    ),
                })
            }
        }
    }
}
