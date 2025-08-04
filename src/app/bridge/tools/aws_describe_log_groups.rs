//! AWS Describe Log Groups Tool
//!
//! This tool allows AI agents to describe CloudWatch log groups with optional filtering
//! by name prefix and limits.

use crate::app::resource_explorer::aws_client::AWSResourceClient;
use async_trait::async_trait;
use serde_json;
use std::sync::Arc;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{info, warn};

use super::super::get_global_aws_client;

/// AWS Describe Log Groups Tool
#[derive(Clone)]
pub struct AwsDescribeLogGroupsTool {
    aws_client: Option<Arc<AWSResourceClient>>,
}

impl std::fmt::Debug for AwsDescribeLogGroupsTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsDescribeLogGroupsTool")
            .field("aws_client", &self.aws_client.is_some())
            .finish()
    }
}

impl AwsDescribeLogGroupsTool {
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
}

#[async_trait]
impl Tool for AwsDescribeLogGroupsTool {
    fn name(&self) -> &str {
        "aws_describe_log_groups"
    }

    fn description(&self) -> &str {
        "Describe CloudWatch log groups with optional filtering by name prefix and limits. \
         Returns detailed information about log groups including creation time, retention, \
         size, and other metadata."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "account_id": {
                    "type": "string",
                    "description": "AWS account ID (optional, uses current account if not specified)"
                },
                "region": {
                    "type": "string", 
                    "description": "AWS region (optional, uses current region if not specified)"
                },
                "log_group_name_prefix": {
                    "type": "string",
                    "description": "Optional prefix to filter log groups by name"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of log groups to return (default: 50, max: 50)",
                    "minimum": 1,
                    "maximum": 50
                }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        info!("🔍 Executing AWS Describe Log Groups tool");

        // Get AWS client - prefer passed client over global
        let aws_client = self.aws_client
            .clone()
            .or_else(get_global_aws_client)
            .ok_or_else(|| {
                warn!("❌ AWS client not available for describe log groups operation");
                ToolError::ExecutionFailed {
                    message: "AWS client not configured. Please ensure AWS credentials are set up".to_string(),
                }
            })?;

        // Parse parameters
        let params = parameters.unwrap_or_default();
        
        let account_id = params
            .get("account_id")
            .and_then(|v| v.as_str())
            .unwrap_or("current"); // Use 'current' as placeholder - tools should specify account

        let region = params
            .get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("us-east-1"); // Default to us-east-1 if not specified

        let name_prefix = params
            .get("log_group_name_prefix")
            .and_then(|v| v.as_str());

        let limit = params
            .get("limit")
            .and_then(|v| v.as_i64())
            .map(|l| l as i32)
            .unwrap_or(50)
            .min(50);

        info!("📋 Describing log groups for account: {}, region: {}, prefix: {:?}, limit: {}", 
              account_id, region, name_prefix, limit);

        // Get logs service and describe log groups
        let logs_service = aws_client.get_logs_service();
        
        match logs_service.describe_log_groups(account_id, region, name_prefix, Some(limit)).await {
            Ok(log_groups) => {
                let count = log_groups.len();
                
                info!("✅ Successfully described {} log groups", count);
                
                let response = serde_json::json!({
                    "success": true,
                    "account_id": account_id,
                    "region": region,
                    "log_groups_count": count,
                    "log_groups": log_groups,
                    "query_parameters": {
                        "name_prefix": name_prefix,
                        "limit": limit
                    }
                });

                Ok(ToolResult::success(response))
            }
            Err(e) => {
                warn!("❌ Failed to describe log groups: {}", e);
                Err(ToolError::ExecutionFailed {
                    message: format!("Failed to describe log groups in account {} region {}: {}", account_id, region, e),
                })
            }
        }
    }
}