# CLI Verification System

The CLI verification system compares AWS Dash's cached resource data against live AWS CLI output to validate data accuracy.

## Overview

This system enables you to verify that resources displayed in AWS Dash match the actual state in AWS. It executes AWS CLI commands and performs field-by-field comparison between Dash's cache and CLI responses, reporting matches and mismatches.

## How to Use

### Running Verification

Verification runs from the Resource Explorer's verification window:

1. Open the Resource Explorer
2. Select a resource type to verify
3. Click "Run Verification"
4. Review the comparison results

### Understanding Results

The verification output shows:
- **Match**: Field value in Dash matches CLI response
- **Mismatch**: Field values differ (shows both values)
- **Ignored**: Field excluded from comparison (e.g., timestamps)

## How it Works

### Architecture

The CLI verification module is organized by AWS service:

```
src/app/resource_explorer/cli_commands/
├── mod.rs           # Core types and routing
├── ec2.rs           # EC2 Instance, SecurityGroup, VPC, Subnet, Volume
├── lambda.rs        # Lambda Function
├── s3.rs            # S3 Bucket
├── iam.rs           # IAM Role, User
├── cloudformation.rs # CloudFormation Stack
├── ecs.rs           # ECS Cluster
├── eks.rs           # EKS Cluster
├── messaging.rs     # SNS Topic, SQS Queue
├── monitoring.rs    # CloudWatch Alarm, Logs LogGroup
├── security.rs      # KMS Key
└── other.rs         # RDS, DynamoDB, Bedrock
```

### Core Components

**CliCommand**: Defines how to execute a list command for a resource type.

```rust
pub struct CliCommand {
    pub service: &'static str,      // AWS CLI service name
    pub operation: &'static str,     // CLI operation
    pub json_path: &'static str,     // Path to extract resources
    pub id_field: &'static str,      // Unique identifier field
    pub is_global: bool,             // True for global services
}
```

**FieldMapping**: Maps Dash field names to CLI JSON paths.

```rust
pub struct FieldMapping {
    pub dash_field: &'static str,
    pub cli_field: &'static str,
    pub comparison_type: ComparisonType,
}
```

**ComparisonType**: Controls how values are compared.

```rust
pub enum ComparisonType {
    Exact,           // Exact string match
    CaseInsensitive, // Case-insensitive string match
    Numeric,         // Parse as numbers and compare
    Ignore,          // Skip comparison (dynamic fields)
}
```

**DetailCommand**: Fetches additional properties per resource.

```rust
pub struct DetailCommand {
    pub service: &'static str,
    pub operation: &'static str,
    pub id_arg: &'static str,
    pub json_path: &'static str,
    pub is_global: bool,
}
```

### Execution Flow

1. `get_cli_command()` returns the CLI command for a resource type
2. `execute_cli_command()` runs the AWS CLI with credentials
3. Resources are extracted from the JSON response using `json_path`
4. For each resource, `get_field_mappings()` provides field comparisons
5. If `get_detail_commands()` returns commands, additional properties are fetched
6. Results are written to `target/verification/` directory

### Security

Credentials are passed via environment variables to the spawned CLI process:
- `AWS_ACCESS_KEY_ID`
- `AWS_SECRET_ACCESS_KEY`
- `AWS_SESSION_TOKEN`

Credentials are never written to files or logged.

## How to Extend

### Adding a New Resource Type

1. Choose or create a service file in `cli_commands/`
2. Define the CLI command function:

```rust
pub fn my_resource_cli_command() -> CliCommand {
    CliCommand {
        service: "my-service",
        operation: "describe-resources",
        json_path: "Resources",
        id_field: "ResourceId",
        is_global: false,
    }
}
```

3. Define field mappings:

```rust
pub fn my_resource_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "ResourceId",
            cli_field: "ResourceId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Name",
            cli_field: "Name",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "LastModified",
            cli_field: "LastModified",
            comparison_type: ComparisonType::Ignore,
        },
    ]
}
```

4. Add routing in `mod.rs`:

```rust
// In get_cli_command()
"AWS::MyService::Resource" => Some(my_service::my_resource_cli_command()),

// In get_field_mappings()
"AWS::MyService::Resource" => my_service::my_resource_field_mappings(),

// In supported_resource_types()
"AWS::MyService::Resource",
```

5. Add unit tests:

```rust
#[test]
fn test_my_resource_cli_command() {
    let cmd = my_resource_cli_command();
    assert_eq!(cmd.service, "my-service");
    assert_eq!(cmd.operation, "describe-resources");
}
```

### Adding Detail Commands

Some resources require additional API calls for full property comparison:

```rust
pub fn my_resource_detail_commands() -> Vec<DetailCommand> {
    vec![DetailCommand {
        service: "my-service",
        operation: "describe-resource-details",
        id_arg: "--resource-id",
        json_path: "Details",
        is_global: false,
    }]
}
```

Register in `mod.rs`:

```rust
// In get_detail_commands()
"AWS::MyService::Resource" => my_service::my_resource_detail_commands(),
```

## Supported Resource Types

| Resource Type | Service File | Detail Commands |
|---------------|--------------|-----------------|
| AWS::EC2::Instance | ec2.rs | No |
| AWS::EC2::SecurityGroup | ec2.rs | No |
| AWS::EC2::VPC | ec2.rs | No |
| AWS::EC2::Subnet | ec2.rs | No |
| AWS::EC2::Volume | ec2.rs | No |
| AWS::S3::Bucket | s3.rs | Yes (5 commands) |
| AWS::Lambda::Function | lambda.rs | Yes (get-function) |
| AWS::IAM::Role | iam.rs | No |
| AWS::IAM::User | iam.rs | No |
| AWS::CloudFormation::Stack | cloudformation.rs | No |
| AWS::ECS::Cluster | ecs.rs | No |
| AWS::EKS::Cluster | eks.rs | Yes (describe-cluster) |
| AWS::SNS::Topic | messaging.rs | Yes (get-topic-attributes) |
| AWS::SQS::Queue | messaging.rs | Yes (get-queue-attributes) |
| AWS::Logs::LogGroup | monitoring.rs | No |
| AWS::CloudWatch::Alarm | monitoring.rs | No |
| AWS::KMS::Key | security.rs | Yes (describe-key) |
| AWS::WAFv2::WebACL | security.rs | Yes (get-web-acl) |
| AWS::RDS::DBInstance | other.rs | No |
| AWS::DynamoDB::Table | other.rs | No |
| AWS::Bedrock::KnowledgeBase | other.rs | No |

## Testing

Run the CLI verification unit tests:

```bash
cargo test --lib "cli_commands::"
```

All service modules include tests for:
- CLI command configuration
- Field mapping correctness
- Detail command definitions (where applicable)

## Related Documentation

- [Resource Explorer System](resource-explorer-system.md)
- [AWS Service Integration Patterns](aws-service-integration-patterns.md)
- [Source Code](../../src/app/resource_explorer/cli_commands/mod.rs)
