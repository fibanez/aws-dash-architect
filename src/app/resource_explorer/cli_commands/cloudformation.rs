//! CloudFormation CLI commands and field mappings.
//!
//! Covers: Stack

use super::{CliCommand, ComparisonType, FieldMapping};

// ============================================================================
// CloudFormation Stack
// ============================================================================

pub fn stack_cli_command() -> CliCommand {
    CliCommand {
        service: "cloudformation",
        operation: "describe-stacks",
        json_path: "Stacks",
        id_field: "StackName",
        is_global: false,
        extra_args: &[],
    }
}

pub fn stack_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "StackName",
            cli_field: "StackName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "StackId",
            cli_field: "StackId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "StackStatus",
            cli_field: "StackStatus",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Description",
            cli_field: "Description",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "RoleARN",
            cli_field: "RoleARN",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "DisableRollback",
            cli_field: "DisableRollback",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "EnableTerminationProtection",
            cli_field: "EnableTerminationProtection",
            comparison_type: ComparisonType::Exact,
        },
        // Timestamps - ignore for comparison
        FieldMapping {
            dash_field: "CreationTime",
            cli_field: "CreationTime",
            comparison_type: ComparisonType::Ignore,
        },
        FieldMapping {
            dash_field: "LastUpdatedTime",
            cli_field: "LastUpdatedTime",
            comparison_type: ComparisonType::Ignore,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_cli_command() {
        let cmd = stack_cli_command();
        assert_eq!(cmd.service, "cloudformation");
        assert_eq!(cmd.operation, "describe-stacks");
        assert_eq!(cmd.id_field, "StackName");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_stack_field_mappings() {
        let mappings = stack_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "StackName"));
        assert!(mappings.iter().any(|m| m.dash_field == "StackId"));
        assert!(mappings.iter().any(|m| m.dash_field == "StackStatus"));
    }
}
