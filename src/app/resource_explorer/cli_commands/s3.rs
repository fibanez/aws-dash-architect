//! S3 service CLI commands and field mappings.
//!
//! Covers: Bucket

use super::{CliCommand, ComparisonType, DetailCommand, FieldMapping};

// ============================================================================
// S3 Bucket
// ============================================================================

pub fn bucket_cli_command() -> CliCommand {
    CliCommand {
        service: "s3api",
        operation: "list-buckets",
        json_path: "Buckets",
        id_field: "Name",
        is_global: true,
        extra_args: &[],
    }
}

pub fn bucket_field_mappings() -> Vec<FieldMapping> {
    vec![
        // Basic fields from list-buckets
        FieldMapping {
            dash_field: "Name",
            cli_field: "Name",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "CreationDate",
            cli_field: "CreationDate",
            comparison_type: ComparisonType::Ignore,
        },
        // Versioning (from get-bucket-versioning)
        FieldMapping {
            dash_field: "VersioningStatus",
            cli_field: "Status",
            comparison_type: ComparisonType::Exact,
        },
        // Location (from get-bucket-location)
        FieldMapping {
            dash_field: "LocationConstraint",
            cli_field: "LocationConstraint",
            comparison_type: ComparisonType::Exact,
        },
        // ACL (from get-bucket-acl)
        FieldMapping {
            dash_field: "Owner",
            cli_field: "Owner",
            comparison_type: ComparisonType::Exact,
        },
        // Public Access Block (from get-public-access-block)
        FieldMapping {
            dash_field: "BlockPublicAcls",
            cli_field: "BlockPublicAcls",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "IgnorePublicAcls",
            cli_field: "IgnorePublicAcls",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "BlockPublicPolicy",
            cli_field: "BlockPublicPolicy",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "RestrictPublicBuckets",
            cli_field: "RestrictPublicBuckets",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

pub fn bucket_detail_commands() -> Vec<DetailCommand> {
    vec![
        DetailCommand {
            service: "s3api",
            operation: "get-bucket-versioning",
            id_arg: "--bucket",
            json_path: "",
            is_global: true,
        extra_args: &[],
        },
        DetailCommand {
            service: "s3api",
            operation: "get-bucket-encryption",
            id_arg: "--bucket",
            json_path: "ServerSideEncryptionConfiguration",
            is_global: true,
        extra_args: &[],
        },
        DetailCommand {
            service: "s3api",
            operation: "get-bucket-location",
            id_arg: "--bucket",
            json_path: "",
            is_global: true,
        extra_args: &[],
        },
        DetailCommand {
            service: "s3api",
            operation: "get-bucket-acl",
            id_arg: "--bucket",
            json_path: "",
            is_global: true,
        extra_args: &[],
        },
        DetailCommand {
            service: "s3api",
            operation: "get-public-access-block",
            id_arg: "--bucket",
            json_path: "PublicAccessBlockConfiguration",
            is_global: true,
        extra_args: &[],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_cli_command() {
        let cmd = bucket_cli_command();
        assert_eq!(cmd.service, "s3api");
        assert_eq!(cmd.operation, "list-buckets");
        assert!(cmd.is_global);
    }

    #[test]
    fn test_bucket_field_mappings() {
        let mappings = bucket_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "Name"));
        assert!(mappings.iter().any(|m| m.dash_field == "BlockPublicAcls"));
    }

    #[test]
    fn test_bucket_detail_commands() {
        let cmds = bucket_detail_commands();
        assert!(cmds.len() >= 4);
        assert!(cmds.iter().any(|c| c.operation == "get-bucket-versioning"));
        assert!(cmds.iter().any(|c| c.operation == "get-public-access-block"));
    }
}
