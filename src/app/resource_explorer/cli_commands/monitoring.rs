//! Monitoring services CLI commands and field mappings.
//!
//! Covers: CloudWatch Logs (LogGroup), CloudWatch Alarms

use super::{CliCommand, ComparisonType, FieldMapping};

// ============================================================================
// CloudWatch Logs - LogGroup
// ============================================================================

pub fn log_group_cli_command() -> CliCommand {
    CliCommand {
        service: "logs",
        operation: "describe-log-groups",
        json_path: "logGroups",
        id_field: "logGroupName",
        is_global: false,
        extra_args: &[],
    }
}

pub fn log_group_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "LogGroupName",
            cli_field: "logGroupName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Arn",
            cli_field: "arn",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "StoredBytes",
            cli_field: "storedBytes",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "RetentionInDays",
            cli_field: "retentionInDays",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "MetricFilterCount",
            cli_field: "metricFilterCount",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "KmsKeyId",
            cli_field: "kmsKeyId",
            comparison_type: ComparisonType::Exact,
        },
        // Timestamps - ignore for comparison
        FieldMapping {
            dash_field: "CreationTime",
            cli_field: "creationTime",
            comparison_type: ComparisonType::Ignore,
        },
    ]
}

// ============================================================================
// CloudWatch Alarm
// ============================================================================

pub fn alarm_cli_command() -> CliCommand {
    CliCommand {
        service: "cloudwatch",
        operation: "describe-alarms",
        json_path: "MetricAlarms",
        id_field: "AlarmName",
        is_global: false,
        extra_args: &[],
    }
}

pub fn alarm_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "AlarmName",
            cli_field: "AlarmName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "AlarmArn",
            cli_field: "AlarmArn",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "StateValue",
            cli_field: "StateValue",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "MetricName",
            cli_field: "MetricName",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Namespace",
            cli_field: "Namespace",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Statistic",
            cli_field: "Statistic",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "ComparisonOperator",
            cli_field: "ComparisonOperator",
            comparison_type: ComparisonType::Exact,
        },
        FieldMapping {
            dash_field: "Threshold",
            cli_field: "Threshold",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "Period",
            cli_field: "Period",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "EvaluationPeriods",
            cli_field: "EvaluationPeriods",
            comparison_type: ComparisonType::Numeric,
        },
        FieldMapping {
            dash_field: "ActionsEnabled",
            cli_field: "ActionsEnabled",
            comparison_type: ComparisonType::Exact,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_group_cli_command() {
        let cmd = log_group_cli_command();
        assert_eq!(cmd.service, "logs");
        assert_eq!(cmd.operation, "describe-log-groups");
        assert_eq!(cmd.id_field, "logGroupName");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_log_group_field_mappings() {
        let mappings = log_group_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "LogGroupName"));
        assert!(mappings.iter().any(|m| m.dash_field == "Arn"));
        assert!(mappings.iter().any(|m| m.dash_field == "StoredBytes"));
    }

    #[test]
    fn test_alarm_cli_command() {
        let cmd = alarm_cli_command();
        assert_eq!(cmd.service, "cloudwatch");
        assert_eq!(cmd.operation, "describe-alarms");
        assert_eq!(cmd.id_field, "AlarmName");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_alarm_field_mappings() {
        let mappings = alarm_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "AlarmName"));
        assert!(mappings.iter().any(|m| m.dash_field == "StateValue"));
        assert!(mappings.iter().any(|m| m.dash_field == "MetricName"));
        assert!(mappings.iter().any(|m| m.dash_field == "Threshold"));
    }
}
