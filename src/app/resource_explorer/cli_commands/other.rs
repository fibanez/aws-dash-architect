//! Other AWS services CLI commands and field mappings.
//!
//! Covers: RDS, DynamoDB, Bedrock
//!
//! These services have minimal field mappings currently and can be expanded
//! as needed. When a service grows to have multiple resource types or
//! complex mappings, consider moving it to its own module.

use super::{CliCommand, ComparisonType, FieldMapping};

// ============================================================================
// RDS DBInstance
// ============================================================================

pub fn rds_instance_cli_command() -> CliCommand {
    CliCommand {
        service: "rds",
        operation: "describe-db-instances",
        json_path: "DBInstances",
        id_field: "DBInstanceIdentifier",
        is_global: false,
        extra_args: &[],
    }
}

pub fn rds_instance_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "DBInstanceIdentifier",
            cli_field: "DBInstanceIdentifier",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "DBInstanceClass",
            cli_field: "DBInstanceClass",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Engine",
            cli_field: "Engine",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "EngineVersion",
            cli_field: "EngineVersion",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "DBInstanceStatus",
            cli_field: "DBInstanceStatus",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "AllocatedStorage",
            cli_field: "AllocatedStorage",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "MultiAZ",
            cli_field: "MultiAZ",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

// ============================================================================
// DynamoDB Table
// ============================================================================

pub fn dynamodb_table_cli_command() -> CliCommand {
    CliCommand {
        service: "dynamodb",
        operation: "list-tables",
        json_path: "TableNames",
        id_field: "", // TableNames is just a list of strings
        is_global: false,
        extra_args: &[],
    }
}

pub fn dynamodb_table_field_mappings() -> Vec<FieldMapping> {
    // DynamoDB list-tables only returns table names
    // Full table details require describe-table per table
    vec![FieldMapping {
        dash_field: "TableName",
        cli_field: "TableName",
        comparison_type: ComparisonType::Exact,
    }]
}

// ============================================================================
// Bedrock KnowledgeBase
// ============================================================================

pub fn bedrock_knowledge_base_cli_command() -> CliCommand {
    CliCommand {
        service: "bedrock-agent",
        operation: "list-knowledge-bases",
        json_path: "knowledgeBaseSummaries",
        id_field: "knowledgeBaseId",
        is_global: false,
        extra_args: &[],
    }
}

pub fn bedrock_knowledge_base_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "knowledgeBaseId",
            cli_field: "knowledgeBaseId",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "name",
            cli_field: "name",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "status",
            cli_field: "status",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "updatedAt",
            cli_field: "updatedAt",
            comparison_type: ComparisonType::Ignore,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rds_instance_cli_command() {
        let cmd = rds_instance_cli_command();
        assert_eq!(cmd.service, "rds");
        assert_eq!(cmd.operation, "describe-db-instances");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_rds_instance_field_mappings() {
        let mappings = rds_instance_field_mappings();
        assert!(mappings
            .iter()
            .any(|m| m.dash_field == "DBInstanceIdentifier"));
        assert!(mappings.iter().any(|m| m.dash_field == "Engine"));
    }

    #[test]
    fn test_dynamodb_table_cli_command() {
        let cmd = dynamodb_table_cli_command();
        assert_eq!(cmd.service, "dynamodb");
        assert_eq!(cmd.operation, "list-tables");
        assert!(cmd.id_field.is_empty());
    }

    #[test]
    fn test_bedrock_knowledge_base_cli_command() {
        let cmd = bedrock_knowledge_base_cli_command();
        assert_eq!(cmd.service, "bedrock-agent");
        assert_eq!(cmd.operation, "list-knowledge-bases");
    }
}
