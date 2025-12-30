//! Lambda service CLI commands and field mappings.
//!
//! Covers: Function

use super::{CliCommand, ComparisonType, DetailCommand, FieldMapping};

// ============================================================================
// Lambda Function
// ============================================================================

pub fn function_cli_command() -> CliCommand {
    CliCommand {
        service: "lambda",
        operation: "list-functions",
        json_path: "Functions",
        id_field: "FunctionName",
        is_global: false,
        extra_args: &[],
    }
}

pub fn function_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "FunctionName",
            cli_field: "FunctionName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "FunctionArn",
            cli_field: "FunctionArn",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Runtime",
            cli_field: "Runtime",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Role",
            cli_field: "Role",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Handler",
            cli_field: "Handler",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "CodeSize",
            cli_field: "CodeSize",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "Description",
            cli_field: "Description",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Timeout",
            cli_field: "Timeout",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "MemorySize",
            cli_field: "MemorySize",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "LastModified",
            cli_field: "LastModified",
            comparison_type: ComparisonType::Ignore,
        },
        FieldMapping {
            dash_field: "Version",
            cli_field: "Version",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "PackageType",
            cli_field: "PackageType",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Architectures",
            cli_field: "Architectures",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

pub fn function_detail_commands() -> Vec<DetailCommand> {
    vec![DetailCommand {
        service: "lambda",
        operation: "get-function",
        id_arg: "--function-name",
        json_path: "Configuration",
        is_global: false,
        extra_args: &[],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_cli_command() {
        let cmd = function_cli_command();
        assert_eq!(cmd.service, "lambda");
        assert_eq!(cmd.operation, "list-functions");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_function_field_mappings() {
        let mappings = function_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "FunctionName"));
        assert!(mappings.iter().any(|m| m.dash_field == "Runtime"));
        assert!(mappings.iter().any(|m| m.dash_field == "MemorySize"));
    }

    #[test]
    fn test_function_detail_commands() {
        let cmds = function_detail_commands();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].operation, "get-function");
    }
}
