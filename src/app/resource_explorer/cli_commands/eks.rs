//! EKS (Elastic Kubernetes Service) CLI commands and field mappings.
//!
//! Covers: Cluster

use super::{CliCommand, ComparisonType, DetailCommand, FieldMapping};

// ============================================================================
// EKS Cluster
// ============================================================================

/// EKS clusters require a two-step approach:
/// 1. list-clusters returns cluster names
/// 2. describe-cluster with --name parameter returns full details
pub fn cluster_cli_command() -> CliCommand {
    CliCommand {
        service: "eks",
        operation: "list-clusters",
        json_path: "clusters",
        id_field: "", // list-clusters returns array of cluster name strings
        is_global: false,
        extra_args: &[],
    }
}

pub fn cluster_detail_commands() -> Vec<DetailCommand> {
    vec![DetailCommand {
        service: "eks",
        operation: "describe-cluster",
        id_arg: "--name",
        json_path: "cluster",
        is_global: false,
        extra_args: &[],
    }]
}

pub fn cluster_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "Name",
            cli_field: "name",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Arn",
            cli_field: "arn",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Status",
            cli_field: "status",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Version",
            cli_field: "version",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "PlatformVersion",
            cli_field: "platformVersion",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "RoleArn",
            cli_field: "roleArn",
            comparison_type: ComparisonType::Exact,
        },
        // Timestamps - ignore for comparison
        FieldMapping {
            dash_field: "CreatedAt",
            cli_field: "createdAt",
            comparison_type: ComparisonType::Ignore,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_cli_command() {
        let cmd = cluster_cli_command();
        assert_eq!(cmd.service, "eks");
        assert_eq!(cmd.operation, "list-clusters");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_cluster_detail_commands() {
        let details = cluster_detail_commands();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].operation, "describe-cluster");
        assert_eq!(details[0].id_arg, "--name");
    }

    #[test]
    fn test_cluster_field_mappings() {
        let mappings = cluster_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "Name"));
        assert!(mappings.iter().any(|m| m.dash_field == "Arn"));
        assert!(mappings.iter().any(|m| m.dash_field == "Status"));
        assert!(mappings.iter().any(|m| m.dash_field == "Version"));
    }
}
