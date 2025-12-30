//! IAM service CLI commands and field mappings.
//!
//! Covers: Role, User

use super::{CliCommand, ComparisonType, FieldMapping};

// ============================================================================
// IAM Role
// ============================================================================

pub fn role_cli_command() -> CliCommand {
    CliCommand {
        service: "iam",
        operation: "list-roles",
        json_path: "Roles",
        id_field: "RoleName",
        is_global: true,
        extra_args: &[],
    }
}

pub fn role_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "RoleName",
            cli_field: "RoleName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "RoleId",
            cli_field: "RoleId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Arn",
            cli_field: "Arn",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Path",
            cli_field: "Path",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "CreateDate",
            cli_field: "CreateDate",
            comparison_type: ComparisonType::Ignore,
        },
        FieldMapping {
            dash_field: "MaxSessionDuration",
            cli_field: "MaxSessionDuration",
            comparison_type: ComparisonType::Numeric,
        },
    ]
}

// ============================================================================
// IAM User
// ============================================================================

pub fn user_cli_command() -> CliCommand {
    CliCommand {
        service: "iam",
        operation: "list-users",
        json_path: "Users",
        id_field: "UserName",
        is_global: true,
        extra_args: &[],
    }
}

pub fn user_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "UserName",
            cli_field: "UserName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "UserId",
            cli_field: "UserId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Arn",
            cli_field: "Arn",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Path",
            cli_field: "Path",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "CreateDate",
            cli_field: "CreateDate",
            comparison_type: ComparisonType::Ignore,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_cli_command() {
        let cmd = role_cli_command();
        assert_eq!(cmd.service, "iam");
        assert_eq!(cmd.operation, "list-roles");
        assert!(cmd.is_global);
    }

    #[test]
    fn test_role_field_mappings() {
        let mappings = role_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "RoleName"));
        assert!(mappings.iter().any(|m| m.dash_field == "Arn"));
    }

    #[test]
    fn test_user_cli_command() {
        let cmd = user_cli_command();
        assert_eq!(cmd.service, "iam");
        assert_eq!(cmd.operation, "list-users");
        assert!(cmd.is_global);
    }

    #[test]
    fn test_user_field_mappings() {
        let mappings = user_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "UserName"));
        assert!(mappings.iter().any(|m| m.dash_field == "Arn"));
    }
}
