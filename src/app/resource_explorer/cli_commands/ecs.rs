//! ECS (Elastic Container Service) CLI commands and field mappings.
//!
//! Covers: Cluster

use super::{CliCommand, ComparisonType, FieldMapping};

// ============================================================================
// ECS Cluster
// ============================================================================

/// ECS clusters require a two-step approach:
/// 1. list-clusters returns cluster ARNs
/// 2. describe-clusters with --clusters parameter returns details
///
/// For verification, we use describe-clusters which returns full cluster data
/// but requires the cluster ARNs/names as input.
pub fn cluster_cli_command() -> CliCommand {
    CliCommand {
        service: "ecs",
        operation: "list-clusters",
        json_path: "clusterArns",
        id_field: "", // list-clusters returns array of ARN strings
        is_global: false,
        extra_args: &[],
    }
}

pub fn cluster_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "ClusterName",
            cli_field: "clusterName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "ClusterArn",
            cli_field: "clusterArn",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Status",
            cli_field: "status",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "RegisteredContainerInstancesCount",
            cli_field: "registeredContainerInstancesCount",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "RunningTasksCount",
            cli_field: "runningTasksCount",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "PendingTasksCount",
            cli_field: "pendingTasksCount",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "ActiveServicesCount",
            cli_field: "activeServicesCount",
            comparison_type: ComparisonType::Numeric,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_cli_command() {
        let cmd = cluster_cli_command();
        assert_eq!(cmd.service, "ecs");
        assert_eq!(cmd.operation, "list-clusters");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_cluster_field_mappings() {
        let mappings = cluster_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "ClusterName"));
        assert!(mappings.iter().any(|m| m.dash_field == "ClusterArn"));
        assert!(mappings.iter().any(|m| m.dash_field == "Status"));
        assert!(mappings
            .iter()
            .any(|m| m.dash_field == "RunningTasksCount"));
    }
}
