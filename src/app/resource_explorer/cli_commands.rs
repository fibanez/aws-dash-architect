//! AWS CLI command execution for resource verification.
//!
//! This module provides functionality to execute AWS CLI commands and parse their
//! output for comparison with Dash's cached resource data.
//!
//! # Security
//!
//! Credentials are passed to the CLI via environment variables in the spawned
//! process, never written to files.

#![cfg(debug_assertions)]

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

/// Get field mappings for a resource type
pub fn get_field_mappings(resource_type: &str) -> Vec<FieldMapping> {
    match resource_type {
        "AWS::Lambda::Function" => vec![
            FieldMapping { dash_field: "FunctionName", cli_field: "FunctionName", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "FunctionArn", cli_field: "FunctionArn", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Runtime", cli_field: "Runtime", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Role", cli_field: "Role", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Handler", cli_field: "Handler", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "CodeSize", cli_field: "CodeSize", comparison_type: ComparisonType::Numeric },
            FieldMapping { dash_field: "Description", cli_field: "Description", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Timeout", cli_field: "Timeout", comparison_type: ComparisonType::Numeric },
            FieldMapping { dash_field: "MemorySize", cli_field: "MemorySize", comparison_type: ComparisonType::Numeric },
            FieldMapping { dash_field: "LastModified", cli_field: "LastModified", comparison_type: ComparisonType::Ignore },
            FieldMapping { dash_field: "Version", cli_field: "Version", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "PackageType", cli_field: "PackageType", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Architectures", cli_field: "Architectures", comparison_type: ComparisonType::Exact },
        ],
        "AWS::EC2::VPC" => vec![
            FieldMapping { dash_field: "VpcId", cli_field: "VpcId", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "CidrBlock", cli_field: "CidrBlock", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "State", cli_field: "State", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "IsDefault", cli_field: "IsDefault", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "InstanceTenancy", cli_field: "InstanceTenancy", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "DhcpOptionsId", cli_field: "DhcpOptionsId", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "OwnerId", cli_field: "OwnerId", comparison_type: ComparisonType::Exact },
        ],
        "AWS::EC2::Instance" => vec![
            FieldMapping { dash_field: "InstanceId", cli_field: "InstanceId", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "InstanceType", cli_field: "InstanceType", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "ImageId", cli_field: "ImageId", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "State", cli_field: "State.Name", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "PrivateIpAddress", cli_field: "PrivateIpAddress", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "PublicIpAddress", cli_field: "PublicIpAddress", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "VpcId", cli_field: "VpcId", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "SubnetId", cli_field: "SubnetId", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Architecture", cli_field: "Architecture", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Platform", cli_field: "Platform", comparison_type: ComparisonType::Exact },
        ],
        "AWS::S3::Bucket" => vec![
            // Basic fields from list-buckets
            FieldMapping { dash_field: "Name", cli_field: "Name", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "CreationDate", cli_field: "CreationDate", comparison_type: ComparisonType::Ignore },
            // Versioning (from get-bucket-versioning)
            FieldMapping { dash_field: "VersioningStatus", cli_field: "Status", comparison_type: ComparisonType::Exact },
            // Location (from get-bucket-location)
            FieldMapping { dash_field: "LocationConstraint", cli_field: "LocationConstraint", comparison_type: ComparisonType::Exact },
            // ACL (from get-bucket-acl)
            FieldMapping { dash_field: "Owner", cli_field: "Owner", comparison_type: ComparisonType::Exact },
            // Public Access Block (from get-public-access-block)
            FieldMapping { dash_field: "BlockPublicAcls", cli_field: "BlockPublicAcls", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "IgnorePublicAcls", cli_field: "IgnorePublicAcls", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "BlockPublicPolicy", cli_field: "BlockPublicPolicy", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "RestrictPublicBuckets", cli_field: "RestrictPublicBuckets", comparison_type: ComparisonType::Exact },
        ],
        "AWS::IAM::Role" => vec![
            FieldMapping { dash_field: "RoleName", cli_field: "RoleName", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "RoleId", cli_field: "RoleId", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Arn", cli_field: "Arn", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "Path", cli_field: "Path", comparison_type: ComparisonType::Exact },
            FieldMapping { dash_field: "CreateDate", cli_field: "CreateDate", comparison_type: ComparisonType::Ignore },
            FieldMapping { dash_field: "MaxSessionDuration", cli_field: "MaxSessionDuration", comparison_type: ComparisonType::Numeric },
        ],
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
}

/// Get the CLI command configuration for a resource type.
pub fn get_cli_command(resource_type: &str) -> Option<CliCommand> {
    match resource_type {
        "AWS::EC2::Instance" => Some(CliCommand {
            service: "ec2",
            operation: "describe-instances",
            json_path: "Reservations[].Instances[]",
            id_field: "InstanceId",
            is_global: false,
        }),
        "AWS::EC2::SecurityGroup" => Some(CliCommand {
            service: "ec2",
            operation: "describe-security-groups",
            json_path: "SecurityGroups",
            id_field: "GroupId",
            is_global: false,
        }),
        "AWS::EC2::VPC" => Some(CliCommand {
            service: "ec2",
            operation: "describe-vpcs",
            json_path: "Vpcs",
            id_field: "VpcId",
            is_global: false,
        }),
        "AWS::S3::Bucket" => Some(CliCommand {
            service: "s3api",
            operation: "list-buckets",
            json_path: "Buckets",
            id_field: "Name",
            is_global: true,
        }),
        "AWS::Lambda::Function" => Some(CliCommand {
            service: "lambda",
            operation: "list-functions",
            json_path: "Functions",
            id_field: "FunctionName",
            is_global: false,
        }),
        "AWS::IAM::Role" => Some(CliCommand {
            service: "iam",
            operation: "list-roles",
            json_path: "Roles",
            id_field: "RoleName",
            is_global: true,
        }),
        "AWS::IAM::User" => Some(CliCommand {
            service: "iam",
            operation: "list-users",
            json_path: "Users",
            id_field: "UserName",
            is_global: true,
        }),
        "AWS::RDS::DBInstance" => Some(CliCommand {
            service: "rds",
            operation: "describe-db-instances",
            json_path: "DBInstances",
            id_field: "DBInstanceIdentifier",
            is_global: false,
        }),
        "AWS::DynamoDB::Table" => Some(CliCommand {
            service: "dynamodb",
            operation: "list-tables",
            json_path: "TableNames",
            id_field: "", // TableNames is just a list of strings
            is_global: false,
        }),
        "AWS::Bedrock::KnowledgeBase" => Some(CliCommand {
            service: "bedrock-agent",
            operation: "list-knowledge-bases",
            json_path: "knowledgeBaseSummaries",
            id_field: "knowledgeBaseId",
            is_global: false,
        }),
        _ => None,
    }
}

/// Get detail commands for fetching per-resource properties
pub fn get_detail_commands(resource_type: &str) -> Vec<DetailCommand> {
    match resource_type {
        "AWS::S3::Bucket" => vec![
            DetailCommand {
                service: "s3api",
                operation: "get-bucket-versioning",
                id_arg: "--bucket",
                json_path: "",
                is_global: true,
            },
            DetailCommand {
                service: "s3api",
                operation: "get-bucket-encryption",
                id_arg: "--bucket",
                json_path: "ServerSideEncryptionConfiguration",
                is_global: true,
            },
            DetailCommand {
                service: "s3api",
                operation: "get-bucket-location",
                id_arg: "--bucket",
                json_path: "",
                is_global: true,
            },
            DetailCommand {
                service: "s3api",
                operation: "get-bucket-acl",
                id_arg: "--bucket",
                json_path: "",
                is_global: true,
            },
            DetailCommand {
                service: "s3api",
                operation: "get-public-access-block",
                id_arg: "--bucket",
                json_path: "PublicAccessBlockConfiguration",
                is_global: true,
            },
        ],
        "AWS::Lambda::Function" => vec![
            DetailCommand {
                service: "lambda",
                operation: "get-function",
                id_arg: "--function-name",
                json_path: "Configuration",
                is_global: false,
            },
        ],
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

    // Add region for non-global services
    if !cmd.is_global {
        args.push("--region");
        args.push(region);
    }

    let command_str = format!("aws {} {} --region {}", cmd.service, cmd.operation, region);
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
        duration_ms, response_size, resources.len()
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
        let mut args = vec![cmd.service, cmd.operation, cmd.id_arg, resource_id, "--output", "json"];

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
    // First get the list result
    let mut result = execute_cli_command(cmd, creds, region)?;
    result.resource_type = resource_type.to_string();

    // Check if we have detail commands for this resource type
    let detail_commands = get_detail_commands(resource_type);
    if detail_commands.is_empty() {
        return Ok(result);
    }

    info!("[CLI] Fetching details for {} {} resources...", result.resources.len(), resource_type);

    // For each resource, fetch details and merge
    let mut enriched_resources = Vec::new();
    let mut enriched_by_id = std::collections::HashMap::new();

    for resource in &result.resources {
        let resource_id = extract_single_id(resource, cmd.id_field).unwrap_or_default();
        if resource_id.is_empty() {
            enriched_resources.push(resource.clone());
            continue;
        }

        // Get detail properties
        let details = execute_detail_commands(resource_type, &resource_id, creds, region);

        // Merge details into resource
        let merged = if let (Value::Object(mut base), Value::Object(detail_obj)) = (resource.clone(), details) {
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
                duration_ms, response_size, resources.len()
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
    resource.get(id_field).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Get list of resource types that have CLI command mappings.
pub fn supported_resource_types() -> Vec<&'static str> {
    vec![
        "AWS::EC2::Instance",
        "AWS::EC2::SecurityGroup",
        "AWS::EC2::VPC",
        "AWS::S3::Bucket",
        "AWS::Lambda::Function",
        "AWS::IAM::Role",
        "AWS::IAM::User",
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
        assert_eq!(get_json_value(&json, "Name"), Some(Value::String("test".to_string())));
        assert_eq!(get_json_value(&json, "Size"), Some(serde_json::json!(100)));
    }

    #[test]
    fn test_get_json_value_nested() {
        let json = serde_json::json!({"State": {"Name": "running", "Code": 16}});
        assert_eq!(get_json_value(&json, "State.Name"), Some(Value::String("running".to_string())));
    }

    #[test]
    fn test_field_mappings() {
        let mappings = get_field_mappings("AWS::Lambda::Function");
        assert!(!mappings.is_empty());
        assert!(mappings.iter().any(|m| m.dash_field == "FunctionName"));
    }
}
