//! AWS Describe Resource Tool
//!
//! This tool allows AI agents to get detailed information about specific AWS resources
//! by providing either an ARN or individual components (account, region, resource type, resource ID).

use crate::app::resource_explorer::{aws_client::AWSResourceClient, state::ResourceEntry};
use async_trait::async_trait;
use serde_json;
use std::sync::Arc;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::info;

use super::super::{get_global_aws_client, ResourceSummary};

/// AWS Describe Resource Tool - Manual Implementation
#[derive(Clone)]
pub struct AwsDescribeResourceTool {
    aws_client: Option<Arc<AWSResourceClient>>,
}

impl std::fmt::Debug for AwsDescribeResourceTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsDescribeResourceTool")
            .field("aws_client", &self.aws_client.is_some())
            .finish()
    }
}

impl AwsDescribeResourceTool {
    pub fn new(aws_client: Option<Arc<AWSResourceClient>>) -> Self {
        Self { aws_client }
    }

    pub fn new_uninitialized() -> Self {
        Self { aws_client: None }
    }

    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = aws_client;
    }

    /// Parse ARN to extract components
    fn parse_arn(arn: &str) -> Result<(String, String, String, String), ToolError> {
        let parts: Vec<&str> = arn.split(':').collect();
        if parts.len() < 6 {
            return Err(ToolError::InvalidParameters {
                message: format!("Invalid ARN format: {}", arn),
            });
        }

        let service = parts[2];
        let region = parts[3];
        let account_id = parts[4];
        let resource_part = parts[5];

        // Extract resource type and ID from resource part
        let (resource_type, resource_id) = if resource_part.contains('/') {
            let parts: Vec<&str> = resource_part.splitn(2, '/').collect();
            (
                format!("AWS::{}::{}", service.to_uppercase(), parts[0]),
                parts[1].to_string(),
            )
        } else {
            (
                format!("AWS::{}::Unknown", service.to_uppercase()),
                resource_part.to_string(),
            )
        };

        Ok((
            account_id.to_string(),
            region.to_string(),
            resource_type,
            resource_id,
        ))
    }
}

impl Default for AwsDescribeResourceTool {
    fn default() -> Self {
        Self::new_uninitialized()
    }
}

#[async_trait]
impl Tool for AwsDescribeResourceTool {
    fn name(&self) -> &str {
        "aws_describe_resources"
    }

    fn description(&self) -> &str {
        r#"Get detailed information about a specific AWS resource.

You can specify the resource using either:
1. Complete ARN: "arn:aws:ec2:us-east-1:123456789012:instance/i-1234567890abcdef0"
2. Individual components: resource_type, account_id, region, resource_id

Examples:
- Describe EC2 instance by ARN: {"resource_arn": "arn:aws:ec2:us-east-1:123456789012:instance/i-1234567890abcdef0"}
- Describe by components: {"resource_type": "AWS::EC2::Instance", "account_id": "123456789012", "region": "us-east-1", "resource_id": "i-1234567890abcdef0"}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "resource_arn": {
                    "type": "string",
                    "description": "AWS resource ARN (complete identifier). Alternative to using individual components.",
                    "examples": ["arn:aws:ec2:us-east-1:123456789012:instance/i-1234567890abcdef0"]
                },
                "resource_type": {
                    "type": "string",
                    "description": "CloudFormation resource type (e.g., 'AWS::EC2::Instance'). Required if not using resource_arn.",
                    "examples": ["AWS::EC2::Instance", "AWS::S3::Bucket", "AWS::Lambda::Function"]
                },
                "account_id": {
                    "type": "string",
                    "description": "AWS account ID. Required if not using resource_arn.",
                    "examples": ["123456789012"]
                },
                "region": {
                    "type": "string",
                    "description": "AWS region. Required if not using resource_arn.",
                    "examples": ["us-east-1", "us-west-2", "eu-west-1"]
                },
                "resource_id": {
                    "type": "string",
                    "description": "Resource identifier. Required if not using resource_arn.",
                    "examples": ["i-1234567890abcdef0", "my-bucket-name", "my-function-name"]
                }
            }
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        info!(
            "🔍 aws_describe_resources executing with parameters: {:?}",
            parameters
        );

        // Check if AWS client is available (try instance client first, then global)
        let global_client = get_global_aws_client();
        let aws_client = self.aws_client.as_ref()
            .or(global_client.as_ref())
            .ok_or_else(|| {
                ToolError::ExecutionFailed { 
                    message: "AWS client not available. Ensure AWS credentials are configured and ResourceExplorer is initialized.".to_string()
                }
            })?;

        // Parse parameters
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "No parameters provided".to_string(),
        })?;

        // Construct ResourceEntry from parameters
        let resource_entry = if let Some(arn) = params.get("resource_arn").and_then(|v| v.as_str())
        {
            // Parse ARN
            let (account_id, region, resource_type, resource_id) = Self::parse_arn(arn)?;

            ResourceEntry {
                resource_type,
                account_id,
                region,
                resource_id,
                display_name: arn.to_string(),
                status: None,
                properties: serde_json::Value::Null,
                raw_properties: serde_json::Value::Null,
                detailed_properties: None,
                detailed_timestamp: None,
                tags: Vec::new(),
                relationships: Vec::new(),
                account_color: egui::Color32::from_rgb(100, 150, 255),
                region_color: egui::Color32::from_rgb(150, 100, 255),
                query_timestamp: chrono::Utc::now(),
            }
        } else {
            // Use individual components
            let resource_type = params
                .get("resource_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "Either resource_arn or resource_type must be provided".to_string(),
                })?;
            let account_id = params
                .get("account_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "Either resource_arn or account_id must be provided".to_string(),
                })?;
            let region = params
                .get("region")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "Either resource_arn or region must be provided".to_string(),
                })?;
            let resource_id = params
                .get("resource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "Either resource_arn or resource_id must be provided".to_string(),
                })?;

            ResourceEntry {
                resource_type: resource_type.to_string(),
                account_id: account_id.to_string(),
                region: region.to_string(),
                resource_id: resource_id.to_string(),
                display_name: format!("{} ({})", resource_id, resource_type),
                status: None,
                properties: serde_json::Value::Null,
                raw_properties: serde_json::Value::Null,
                detailed_properties: None,
                detailed_timestamp: None,
                tags: Vec::new(),
                relationships: Vec::new(),
                account_color: egui::Color32::from_rgb(100, 150, 255),
                region_color: egui::Color32::from_rgb(150, 100, 255),
                query_timestamp: chrono::Utc::now(),
            }
        };

        // Get detailed properties using the describe_resource API
        info!(
            "🔍 Fetching detailed properties for {}",
            resource_entry.resource_id
        );
        let detailed_properties = aws_client
            .describe_resource(&resource_entry)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                message: format!("Failed to describe AWS resource: {}", e),
            })?;

        let duration = start_time.elapsed();

        let resource_summary = ResourceSummary {
            resource_type: resource_entry.resource_type.clone(),
            account_id: resource_entry.account_id.clone(),
            region: resource_entry.region.clone(),
            resource_id: resource_entry.resource_id.clone(),
            display_name: resource_entry.display_name.clone(),
            status: resource_entry.status.clone(),
            properties: resource_entry.properties.clone(),
            tags: resource_entry
                .tags
                .iter()
                .map(|tag| format!("{}={}", tag.key, tag.value))
                .collect(),
        };

        let execution_summary = format!(
            "Successfully retrieved detailed properties for {} resource {} in {}/{} in {:.2}s",
            resource_entry.resource_type,
            resource_entry.resource_id,
            resource_entry.account_id,
            resource_entry.region,
            duration.as_secs_f64()
        );

        info!("📊 aws_describe_resources completed: {}", execution_summary);

        // Create response JSON
        let response_data = serde_json::json!({
            "resource_info": resource_summary,
            "detailed_properties": detailed_properties,
            "from_cache": false, // describe_resource always fetches fresh data
            "execution_summary": execution_summary,
            "duration_seconds": duration.as_secs_f64()
        });

        Ok(ToolResult::success(response_data))
    }
}
