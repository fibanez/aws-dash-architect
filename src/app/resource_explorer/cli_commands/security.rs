//! Security services CLI commands and field mappings.
//!
//! Covers: KMS (Key), WAFv2 (WebACL)

use super::{CliCommand, ComparisonType, DetailCommand, FieldMapping};

// ============================================================================
// KMS Key
// ============================================================================

/// KMS keys require a two-step approach:
/// 1. list-keys returns KeyId and KeyArn only
/// 2. describe-key with --key-id returns full key metadata
pub fn key_cli_command() -> CliCommand {
    CliCommand {
        service: "kms",
        operation: "list-keys",
        json_path: "Keys",
        id_field: "KeyId",
        is_global: false,
        extra_args: &[],
    }
}

pub fn key_detail_commands() -> Vec<DetailCommand> {
    vec![DetailCommand {
        service: "kms",
        operation: "describe-key",
        id_arg: "--key-id",
        json_path: "KeyMetadata",
        is_global: false,
        extra_args: &[],
    }]
}

pub fn key_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "KeyId",
            cli_field: "KeyId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Arn",
            cli_field: "Arn",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "KeyState",
            cli_field: "KeyState",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "KeyUsage",
            cli_field: "KeyUsage",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "KeySpec",
            cli_field: "KeySpec",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Description",
            cli_field: "Description",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Enabled",
            cli_field: "Enabled",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "KeyManager",
            cli_field: "KeyManager",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Origin",
            cli_field: "Origin",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "MultiRegion",
            cli_field: "MultiRegion",
            comparison_type: ComparisonType::Exact,
        },
        // Timestamps - ignore for comparison
        FieldMapping {
            dash_field: "CreationDate",
            cli_field: "CreationDate",
            comparison_type: ComparisonType::Ignore,
        },
    ]
}

// ============================================================================
// WAFv2 WebACL
// ============================================================================

/// WAFv2 WebACL uses list-web-acls with --scope REGIONAL
/// Note: CloudFront WebACLs use --scope CLOUDFRONT and must query us-east-1
/// For simplicity, we query REGIONAL scope in the current region.
pub fn webacl_cli_command() -> CliCommand {
    CliCommand {
        service: "wafv2",
        operation: "list-web-acls",
        json_path: "WebACLs",
        id_field: "Id",
        is_global: false,
        extra_args: &["--scope", "REGIONAL"],
    }
}

/// Get WebACL details via get-web-acl (requires both --id and --scope)
pub fn webacl_detail_commands() -> Vec<DetailCommand> {
    vec![DetailCommand {
        service: "wafv2",
        operation: "get-web-acl",
        id_arg: "--id",
        json_path: "WebACL",
        is_global: false,
        extra_args: &["--scope", "REGIONAL"],
    }]
}

pub fn webacl_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "Id",
            cli_field: "Id",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Name",
            cli_field: "Name",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "ARN",
            cli_field: "ARN",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Description",
            cli_field: "Description",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "DefaultAction",
            cli_field: "DefaultAction",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "ManagedByFirewallManager",
            cli_field: "ManagedByFirewallManager",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_cli_command() {
        let cmd = key_cli_command();
        assert_eq!(cmd.service, "kms");
        assert_eq!(cmd.operation, "list-keys");
        assert_eq!(cmd.id_field, "KeyId");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_key_detail_commands() {
        let details = key_detail_commands();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].operation, "describe-key");
        assert_eq!(details[0].id_arg, "--key-id");
    }

    #[test]
    fn test_key_field_mappings() {
        let mappings = key_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "KeyId"));
        assert!(mappings.iter().any(|m| m.dash_field == "Arn"));
        assert!(mappings.iter().any(|m| m.dash_field == "KeyState"));
        assert!(mappings.iter().any(|m| m.dash_field == "KeyUsage"));
        assert!(mappings.iter().any(|m| m.dash_field == "Enabled"));
    }

    #[test]
    fn test_webacl_cli_command() {
        let cmd = webacl_cli_command();
        assert_eq!(cmd.service, "wafv2");
        assert_eq!(cmd.operation, "list-web-acls");
        assert_eq!(cmd.extra_args, &["--scope", "REGIONAL"]);
        assert_eq!(cmd.json_path, "WebACLs");
        assert_eq!(cmd.id_field, "Id");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_webacl_detail_commands() {
        let details = webacl_detail_commands();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].operation, "get-web-acl");
        assert_eq!(details[0].extra_args, &["--scope", "REGIONAL"]);
        assert_eq!(details[0].id_arg, "--id");
    }

    #[test]
    fn test_webacl_field_mappings() {
        let mappings = webacl_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "Id"));
        assert!(mappings.iter().any(|m| m.dash_field == "Name"));
        assert!(mappings.iter().any(|m| m.dash_field == "ARN"));
    }
}
