//! AWS CLI command execution for resource verification.
//!
//! This module provides functionality to execute AWS CLI commands and parse their
//! output for comparison with Dash's cached resource data.
//!
//! # Module Organization
//!
//! CLI commands and field mappings are organized by AWS service:
//! - `ec2` - EC2 resources (Instance, SecurityGroup, VPC, Subnet, Volume)
//! - `lambda` - Lambda resources (Function)
//! - `s3` - S3 resources (Bucket)
//! - `iam` - IAM resources (Role, User)
//! - `other` - Other services (RDS, DynamoDB, Bedrock)
//! - `cloudformation` - CloudFormation resources (Stack)
//! - `ecs` - ECS resources (Cluster)
//! - `eks` - EKS resources (Cluster)
//! - `messaging` - SNS/SQS resources (Topic, Queue)
//! - `monitoring` - CloudWatch/Logs resources (Alarm, LogGroup)
//! - `security` - Security services (KMS Key)
//!
//! # Security
//!
//! Credentials are passed to the CLI via environment variables in the spawned
//! process, never written to files.

#![cfg(debug_assertions)]

mod cloudformation;
mod ec2;
mod ecs;
mod eks;
mod iam;
mod lambda;
mod messaging;
mod monitoring;
mod other;
mod s3;
mod security;

use super::credentials::AccountCredentials;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Command;
use std::time::Instant;
use tracing::{error, info, warn};

// ============================================================================
// Field Mapping Configuration
// ============================================================================

/// How to compare field values
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ComparisonType {
    /// Must match exactly (case-sensitive)
    Exact,
    /// String comparison ignoring case
    CaseInsensitive,
    /// Parse as numbers and compare
    Numeric,
    /// Don't compare - field is dynamic (e.g., LastModified)
    Ignore,
}

/// Mapping between Dash property name and CLI JSON path
#[derive(Debug, Clone)]
pub struct FieldMapping {
    /// Field name as it appears in Dash cache
    pub dash_field: &'static str,
    /// JSON path in CLI response (supports dot notation)
    pub cli_field: &'static str,
    /// How to compare the values
    pub comparison_type: ComparisonType,
}

/// Get field mappings for a resource type.
/// Delegates to service-specific modules.
pub fn get_field_mappings(resource_type: &str) -> Vec<FieldMapping> {
    match resource_type {
        // EC2 resources
        "AWS::EC2::Instance" => ec2::instance_field_mappings(),
        "AWS::EC2::SecurityGroup" => ec2::security_group_field_mappings(),
        "AWS::EC2::VPC" => ec2::vpc_field_mappings(),
        "AWS::EC2::Subnet" => ec2::subnet_field_mappings(),
        "AWS::EC2::Volume" => ec2::volume_field_mappings(),

        // Lambda resources
        "AWS::Lambda::Function" => lambda::function_field_mappings(),

        // S3 resources
        "AWS::S3::Bucket" => s3::bucket_field_mappings(),

        // IAM resources
        "AWS::IAM::Role" => iam::role_field_mappings(),
        "AWS::IAM::User" => iam::user_field_mappings(),

        // CloudFormation resources
        "AWS::CloudFormation::Stack" => cloudformation::stack_field_mappings(),

        // Container services
        "AWS::ECS::Cluster" => ecs::cluster_field_mappings(),
        "AWS::EKS::Cluster" => eks::cluster_field_mappings(),

        // Messaging services
        "AWS::SNS::Topic" => messaging::topic_field_mappings(),
        "AWS::SQS::Queue" => messaging::queue_field_mappings(),

        // Monitoring services
        "AWS::Logs::LogGroup" => monitoring::log_group_field_mappings(),
        "AWS::CloudWatch::Alarm" => monitoring::alarm_field_mappings(),

        // Security services
        "AWS::KMS::Key" => security::key_field_mappings(),
        "AWS::WAFv2::WebACL" => security::webacl_field_mappings(),

        // Other services
        "AWS::RDS::DBInstance" => other::rds_instance_field_mappings(),
        "AWS::DynamoDB::Table" => other::dynamodb_table_field_mappings(),
        "AWS::Bedrock::KnowledgeBase" => other::bedrock_knowledge_base_field_mappings(),

        _ => vec![], // No mappings defined - will compare all common fields
    }
}

// ============================================================================
// CLI Command Configuration
// ============================================================================

/// Represents a CLI command configuration for a resource type.
#[derive(Debug, Clone)]
pub struct CliCommand {
    /// AWS service name (e.g., "ec2", "s3api", "bedrock-agent")
    pub service: &'static str,
    /// Operation name (e.g., "describe-instances", "list-buckets")
    pub operation: &'static str,
    /// JMESPath-like path to extract resources from the JSON response
    pub json_path: &'static str,
    /// Field name that uniquely identifies each resource
    pub id_field: &'static str,
    /// Whether this is a global service (doesn't need region)
    pub is_global: bool,
    /// Extra arguments to pass (e.g., ["--scope", "REGIONAL"] for WAFv2)
    pub extra_args: &'static [&'static str],
}

/// Detail command to run per-resource for additional properties
#[derive(Debug, Clone)]
pub struct DetailCommand {
    /// AWS service name
    pub service: &'static str,
    /// Operation name (e.g., "get-bucket-versioning")
    pub operation: &'static str,
    /// Argument name for the resource ID (e.g., "--bucket", "--function-name")
    pub id_arg: &'static str,
    /// JSON path to extract the result (empty string for root)
    pub json_path: &'static str,
    /// Whether this is a global service (doesn't need region)
    pub is_global: bool,
    /// Extra arguments to pass (e.g., ["--scope", "REGIONAL"] for WAFv2)
    pub extra_args: &'static [&'static str],
}

/// Get the CLI command configuration for a resource type.
/// Delegates to service-specific modules.
pub fn get_cli_command(resource_type: &str) -> Option<CliCommand> {
    match resource_type {
        // EC2 resources
        "AWS::EC2::Instance" => Some(ec2::instance_cli_command()),
        "AWS::EC2::SecurityGroup" => Some(ec2::security_group_cli_command()),
        "AWS::EC2::VPC" => Some(ec2::vpc_cli_command()),
        "AWS::EC2::Subnet" => Some(ec2::subnet_cli_command()),
        "AWS::EC2::Volume" => Some(ec2::volume_cli_command()),

        // Lambda resources
        "AWS::Lambda::Function" => Some(lambda::function_cli_command()),

        // S3 resources
        "AWS::S3::Bucket" => Some(s3::bucket_cli_command()),

        // IAM resources
        "AWS::IAM::Role" => Some(iam::role_cli_command()),
        "AWS::IAM::User" => Some(iam::user_cli_command()),

        // CloudFormation resources
        "AWS::CloudFormation::Stack" => Some(cloudformation::stack_cli_command()),

        // Container services
        "AWS::ECS::Cluster" => Some(ecs::cluster_cli_command()),
        "AWS::EKS::Cluster" => Some(eks::cluster_cli_command()),

        // Messaging services
        "AWS::SNS::Topic" => Some(messaging::topic_cli_command()),
        "AWS::SQS::Queue" => Some(messaging::queue_cli_command()),

        // Monitoring services
        "AWS::Logs::LogGroup" => Some(monitoring::log_group_cli_command()),
        "AWS::CloudWatch::Alarm" => Some(monitoring::alarm_cli_command()),

        // Security services
        "AWS::KMS::Key" => Some(security::key_cli_command()),
        "AWS::WAFv2::WebACL" => Some(security::webacl_cli_command()),

        // Other services
        "AWS::RDS::DBInstance" => Some(other::rds_instance_cli_command()),
        "AWS::DynamoDB::Table" => Some(other::dynamodb_table_cli_command()),
        "AWS::Bedrock::KnowledgeBase" => Some(other::bedrock_knowledge_base_cli_command()),

        _ => None,
    }
}

/// Get detail commands for fetching per-resource properties.
/// Delegates to service-specific modules.
pub fn get_detail_commands(resource_type: &str) -> Vec<DetailCommand> {
    match resource_type {
        "AWS::S3::Bucket" => s3::bucket_detail_commands(),
        "AWS::Lambda::Function" => lambda::function_detail_commands(),
        "AWS::EKS::Cluster" => eks::cluster_detail_commands(),
        "AWS::SNS::Topic" => messaging::topic_detail_commands(),
        "AWS::SQS::Queue" => messaging::queue_detail_commands(),
        "AWS::KMS::Key" => security::key_detail_commands(),
        "AWS::WAFv2::WebACL" => security::webacl_detail_commands(),
        _ => vec![],
    }
}

// ============================================================================
// CLI Execution Results
// ============================================================================

/// Record of a CLI command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliExecution {
    /// When the command was executed
    pub timestamp: DateTime<Utc>,
    /// Full command string (without credentials)
    pub command: String,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Size of response in bytes
    pub response_size_bytes: usize,
    /// Number of resources returned
    pub resource_count: usize,
    /// Raw JSON response (for debugging)
    pub raw_response: Value,
    /// Error message if command failed
    pub error: Option<String>,
}

/// Result of executing a CLI command with full details
#[derive(Debug)]
pub struct CliResult {
    /// Resource type being queried
    pub resource_type: String,
    /// Extracted resource JSON objects
    pub resources: Vec<Value>,
    /// Extracted resource IDs (for quick lookup)
    pub resource_ids: Vec<String>,
    /// Map of resource ID -> full JSON object
    pub resources_by_id: std::collections::HashMap<String, Value>,
    /// Execution details for logging
    pub execution: CliExecution,
    /// Error message if command failed
    pub error: Option<String>,
}

/// Execute an AWS CLI command with the given credentials.
/// Returns detailed results including raw response and timing.
pub fn execute_cli_command(
    cmd: &CliCommand,
    creds: &AccountCredentials,
    region: &str,
) -> Result<CliResult> {
    let mut args = vec![cmd.service, cmd.operation, "--output", "json"];

    // Add extra arguments (e.g., --scope REGIONAL for WAFv2)
    for arg in cmd.extra_args {
        args.push(arg);
    }

    // Add region for non-global services
    if !cmd.is_global {
        args.push("--region");
        args.push(region);
    }

    let extra_str = if cmd.extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", cmd.extra_args.join(" "))
    };
    let command_str = format!(
        "aws {} {}{} --region {}",
        cmd.service, cmd.operation, extra_str, region
    );
    info!("[CLI] Executing: {}", command_str);

    let start = Instant::now();
    let timestamp = Utc::now();

    let output = Command::new("aws")
        .args(&args)
        .env("AWS_ACCESS_KEY_ID", &creds.access_key_id)
        .env("AWS_SECRET_ACCESS_KEY", &creds.secret_access_key)
        .env("AWS_SESSION_TOKEN", &creds.session_token)
        .output()
        .context("Failed to execute AWS CLI command")?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let response_size = output.stdout.len();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        error!("[CLI] Command failed after {}ms: {}", duration_ms, stderr);

        return Ok(CliResult {
            resource_type: String::new(),
            resources: Vec::new(),
            resource_ids: Vec::new(),
            resources_by_id: std::collections::HashMap::new(),
            execution: CliExecution {
                timestamp,
                command: command_str,
                duration_ms,
                response_size_bytes: response_size,
                resource_count: 0,
                raw_response: Value::Null,
                error: Some(stderr.clone()),
            },
            error: Some(stderr),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).context("Failed to parse CLI JSON output")?;

    // Extract resources using the json_path
    let resources = extract_resources(&json, cmd.json_path);
    let resource_ids = extract_ids(&resources, cmd.id_field);

    // Build lookup map
    let mut resources_by_id = std::collections::HashMap::new();
    for resource in &resources {
        if let Some(id) = extract_single_id(resource, cmd.id_field) {
            resources_by_id.insert(id, resource.clone());
        }
    }

    info!(
        "[CLI] Response: {}ms, {} bytes, {} resources",
        duration_ms,
        response_size,
        resources.len()
    );

    Ok(CliResult {
        resource_type: String::new(),
        resources: resources.clone(),
        resource_ids,
        resources_by_id,
        execution: CliExecution {
            timestamp,
            command: command_str,
            duration_ms,
            response_size_bytes: response_size,
            resource_count: resources.len(),
            raw_response: json,
            error: None,
        },
        error: None,
    })
}

/// Execute detail commands for a single resource and return merged properties.
/// This runs per-resource CLI commands (like get-bucket-versioning) and merges results.
pub fn execute_detail_commands(
    resource_type: &str,
    resource_id: &str,
    creds: &AccountCredentials,
    region: &str,
) -> Value {
    let detail_commands = get_detail_commands(resource_type);
    if detail_commands.is_empty() {
        return Value::Object(serde_json::Map::new());
    }

    let mut merged = serde_json::Map::new();

    for cmd in detail_commands {
        let mut args = vec![
            cmd.service,
            cmd.operation,
            cmd.id_arg,
            resource_id,
            "--output",
            "json",
        ];

        // Add extra arguments (e.g., --scope REGIONAL for WAFv2)
        for arg in cmd.extra_args {
            args.push(arg);
        }

        // Add region for non-global services
        if !cmd.is_global {
            args.push("--region");
            args.push(region);
        }

        let output = Command::new("aws")
            .args(&args)
            .env("AWS_ACCESS_KEY_ID", &creds.access_key_id)
            .env("AWS_SECRET_ACCESS_KEY", &creds.secret_access_key)
            .env("AWS_SESSION_TOKEN", &creds.session_token)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
                    // Extract from json_path if specified
                    let value = if cmd.json_path.is_empty() {
                        json
                    } else {
                        get_json_value(&json, cmd.json_path).unwrap_or(Value::Null)
                    };

                    // Merge the result into our merged object
                    // For S3 versioning, the response is like {"Status": "Enabled"}
                    // For S3 encryption, it's the ServerSideEncryptionConfiguration object
                    if let Value::Object(obj) = value {
                        for (k, v) in obj {
                            merged.insert(k, v);
                        }
                    } else if !value.is_null() {
                        // Store under the operation name if not an object
                        let key = cmd.operation.replace("get-bucket-", "").replace("get-", "");
                        merged.insert(key, value);
                    }
                }
            } else {
                // Some commands may fail (e.g., no encryption configured)
                // That's OK - we just won't have that property
            }
        }
    }

    Value::Object(merged)
}

/// Execute CLI command and fetch detail properties for each resource.
/// Returns resources with merged detail properties.
pub fn execute_cli_with_details(
    cmd: &CliCommand,
    resource_type: &str,
    creds: &AccountCredentials,
    region: &str,
) -> Result<CliResult> {
    // Use the progress version with no callback
    execute_cli_with_details_progress(cmd, resource_type, creds, region, None)
}

/// Progress callback type for CLI detail fetching.
/// Called with (current_index, total_count, resource_id) for each resource.
pub type DetailProgressCallback = Box<dyn Fn(usize, usize, &str) + Send>;

/// Execute CLI command and fetch detail properties with progress callback.
/// The callback is invoked after each resource's details are fetched.
pub fn execute_cli_with_details_progress(
    cmd: &CliCommand,
    resource_type: &str,
    creds: &AccountCredentials,
    region: &str,
    progress_callback: Option<DetailProgressCallback>,
) -> Result<CliResult> {
    // First get the list result
    let mut result = execute_cli_command(cmd, creds, region)?;
    result.resource_type = resource_type.to_string();

    // Check if we have detail commands for this resource type
    let detail_commands = get_detail_commands(resource_type);
    if detail_commands.is_empty() {
        return Ok(result);
    }

    let total = result.resources.len();
    info!(
        "[CLI] Fetching details for {} {} resources...",
        total, resource_type
    );

    // For each resource, fetch details and merge
    let mut enriched_resources = Vec::new();
    let mut enriched_by_id = std::collections::HashMap::new();

    for (index, resource) in result.resources.iter().enumerate() {
        let resource_id = extract_single_id(resource, cmd.id_field).unwrap_or_default();
        if resource_id.is_empty() {
            enriched_resources.push(resource.clone());
            continue;
        }

        // Report progress before fetching
        if let Some(ref callback) = progress_callback {
            callback(index + 1, total, &resource_id);
        }

        // Get detail properties
        let details = execute_detail_commands(resource_type, &resource_id, creds, region);

        // Merge details into resource
        let merged = if let (Value::Object(mut base), Value::Object(detail_obj)) =
            (resource.clone(), details)
        {
            for (k, v) in detail_obj {
                base.insert(k, v);
            }
            Value::Object(base)
        } else {
            resource.clone()
        };

        enriched_by_id.insert(resource_id, merged.clone());
        enriched_resources.push(merged);
    }

    result.resources = enriched_resources;
    result.resources_by_id = enriched_by_id;

    Ok(result)
}

/// Execute a CLI command for child resources (e.g., DataSources for a KnowledgeBase).
pub fn execute_child_cli_command(
    parent_type: &str,
    parent_id: &str,
    creds: &AccountCredentials,
    region: &str,
) -> Result<CliResult> {
    match parent_type {
        "AWS::Bedrock::KnowledgeBase" => {
            let args = vec![
                "bedrock-agent",
                "list-data-sources",
                "--knowledge-base-id",
                parent_id,
                "--region",
                region,
                "--output",
                "json",
            ];

            let command_str = format!(
                "aws bedrock-agent list-data-sources --knowledge-base-id {} --region {}",
                parent_id, region
            );
            info!("[CLI] Executing: {}", command_str);

            let start = Instant::now();
            let timestamp = Utc::now();

            let output = Command::new("aws")
                .args(&args)
                .env("AWS_ACCESS_KEY_ID", &creds.access_key_id)
                .env("AWS_SECRET_ACCESS_KEY", &creds.secret_access_key)
                .env("AWS_SESSION_TOKEN", &creds.session_token)
                .output()
                .context("Failed to execute AWS CLI command")?;

            let duration_ms = start.elapsed().as_millis() as u64;
            let response_size = output.stdout.len();

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Ok(CliResult {
                    resource_type: "AWS::Bedrock::DataSource".to_string(),
                    resources: Vec::new(),
                    resource_ids: Vec::new(),
                    resources_by_id: std::collections::HashMap::new(),
                    execution: CliExecution {
                        timestamp,
                        command: command_str,
                        duration_ms,
                        response_size_bytes: response_size,
                        resource_count: 0,
                        raw_response: Value::Null,
                        error: Some(stderr.clone()),
                    },
                    error: Some(stderr),
                });
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let json: Value = serde_json::from_str(&stdout)?;

            let resources = extract_resources(&json, "dataSourceSummaries");
            let resource_ids = extract_ids(&resources, "dataSourceId");

            let mut resources_by_id = std::collections::HashMap::new();
            for resource in &resources {
                if let Some(id) = extract_single_id(resource, "dataSourceId") {
                    resources_by_id.insert(id, resource.clone());
                }
            }

            info!(
                "[CLI] Response: {}ms, {} bytes, {} resources",
                duration_ms,
                response_size,
                resources.len()
            );

            Ok(CliResult {
                resource_type: "AWS::Bedrock::DataSource".to_string(),
                resources: resources.clone(),
                resource_ids,
                resources_by_id,
                execution: CliExecution {
                    timestamp,
                    command: command_str,
                    duration_ms,
                    response_size_bytes: response_size,
                    resource_count: resources.len(),
                    raw_response: json,
                    error: None,
                },
                error: None,
            })
        }
        _ => {
            warn!("No child command mapping for parent type: {}", parent_type);
            Ok(CliResult {
                resource_type: String::new(),
                resources: Vec::new(),
                resource_ids: Vec::new(),
                resources_by_id: std::collections::HashMap::new(),
                execution: CliExecution {
                    timestamp: Utc::now(),
                    command: String::new(),
                    duration_ms: 0,
                    response_size_bytes: 0,
                    resource_count: 0,
                    raw_response: Value::Null,
                    error: Some(format!("No child command for {}", parent_type)),
                },
                error: Some(format!("No child command for {}", parent_type)),
            })
        }
    }
}

/// Check if AWS CLI is available on the system.
pub fn check_cli_available() -> Result<String> {
    let output = Command::new("aws")
        .arg("--version")
        .output()
        .context("AWS CLI not found")?;

    if !output.status.success() {
        anyhow::bail!("AWS CLI not functioning properly");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// ============================================================================
// JSON Value Extraction Helpers
// ============================================================================

/// Extract a value from JSON using dot-notation path (e.g., "State.Name")
pub fn get_json_value(json: &Value, path: &str) -> Option<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current.clone())
}

/// Extract resources from JSON using a simple path notation.
fn extract_resources(json: &Value, path: &str) -> Vec<Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = vec![json.clone()];

    for part in parts {
        let part_clean = part.trim_end_matches("[]");
        let is_array = part.ends_with("[]");

        let mut next = Vec::new();
        for val in current {
            if let Some(extracted) = val.get(part_clean) {
                if is_array {
                    if let Some(arr) = extracted.as_array() {
                        next.extend(arr.clone());
                    }
                } else {
                    next.push(extracted.clone());
                }
            }
        }
        current = next;
    }

    // If the final result is a single array, return its contents
    if current.len() == 1 {
        if let Some(arr) = current[0].as_array() {
            return arr.clone();
        }
    }

    current
}

/// Extract IDs from a list of resource JSON objects.
fn extract_ids(resources: &[Value], id_field: &str) -> Vec<String> {
    if id_field.is_empty() {
        // Handle case where resources are just strings (like DynamoDB table names)
        return resources
            .iter()
            .filter_map(|r| r.as_str().map(|s| s.to_string()))
            .collect();
    }

    resources
        .iter()
        .filter_map(|r| extract_single_id(r, id_field))
        .collect()
}

/// Extract a single ID from a resource JSON object
fn extract_single_id(resource: &Value, id_field: &str) -> Option<String> {
    if id_field.is_empty() {
        return resource.as_str().map(|s| s.to_string());
    }
    resource
        .get(id_field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Get list of resource types that have CLI command mappings.
pub fn supported_resource_types() -> Vec<&'static str> {
    vec![
        // EC2 resources
        "AWS::EC2::Instance",
        "AWS::EC2::SecurityGroup",
        "AWS::EC2::VPC",
        "AWS::EC2::Subnet",
        "AWS::EC2::Volume",
        // S3 resources
        "AWS::S3::Bucket",
        // Lambda resources
        "AWS::Lambda::Function",
        // IAM resources
        "AWS::IAM::Role",
        "AWS::IAM::User",
        // CloudFormation resources
        "AWS::CloudFormation::Stack",
        // Container services
        "AWS::ECS::Cluster",
        "AWS::EKS::Cluster",
        // Messaging services
        "AWS::SNS::Topic",
        "AWS::SQS::Queue",
        // Monitoring services
        "AWS::Logs::LogGroup",
        "AWS::CloudWatch::Alarm",
        // Security services
        "AWS::KMS::Key",
        // Other services
        "AWS::RDS::DBInstance",
        "AWS::DynamoDB::Table",
        "AWS::Bedrock::KnowledgeBase",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resources_simple() {
        let json: Value = serde_json::json!({
            "Buckets": [
                {"Name": "bucket1"},
                {"Name": "bucket2"}
            ]
        });

        let resources = extract_resources(&json, "Buckets");
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn test_extract_resources_nested() {
        let json: Value = serde_json::json!({
            "Reservations": [
                {
                    "Instances": [
                        {"InstanceId": "i-1"},
                        {"InstanceId": "i-2"}
                    ]
                },
                {
                    "Instances": [
                        {"InstanceId": "i-3"}
                    ]
                }
            ]
        });

        let resources = extract_resources(&json, "Reservations[].Instances[]");
        assert_eq!(resources.len(), 3);
    }

    #[test]
    fn test_extract_ids() {
        let resources: Vec<Value> = vec![
            serde_json::json!({"InstanceId": "i-123"}),
            serde_json::json!({"InstanceId": "i-456"}),
        ];

        let ids = extract_ids(&resources, "InstanceId");
        assert_eq!(ids, vec!["i-123", "i-456"]);
    }

    #[test]
    fn test_get_cli_command() {
        let cmd = get_cli_command("AWS::EC2::Instance");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(cmd.service, "ec2");
        assert_eq!(cmd.operation, "describe-instances");
    }

    #[test]
    fn test_get_json_value_simple() {
        let json = serde_json::json!({"Name": "test", "Size": 100});
        assert_eq!(
            get_json_value(&json, "Name"),
            Some(Value::String("test".to_string()))
        );
        assert_eq!(get_json_value(&json, "Size"), Some(serde_json::json!(100)));
    }

    #[test]
    fn test_get_json_value_nested() {
        let json = serde_json::json!({"State": {"Name": "running", "Code": 16}});
        assert_eq!(
            get_json_value(&json, "State.Name"),
            Some(Value::String("running".to_string()))
        );
    }

    #[test]
    fn test_field_mappings() {
        let mappings = get_field_mappings("AWS::Lambda::Function");
        assert!(!mappings.is_empty());
        assert!(mappings.iter().any(|m| m.dash_field == "FunctionName"));
    }

    #[test]
    fn test_security_group_field_mappings() {
        let mappings = get_field_mappings("AWS::EC2::SecurityGroup");
        assert!(!mappings.is_empty());
        assert!(mappings.iter().any(|m| m.dash_field == "GroupId"));
        assert!(mappings.iter().any(|m| m.dash_field == "OwnerId"));
    }
}
