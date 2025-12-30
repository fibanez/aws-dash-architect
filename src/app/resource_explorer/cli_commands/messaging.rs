//! Messaging services CLI commands and field mappings.
//!
//! Covers: SNS (Topic), SQS (Queue)

use super::{CliCommand, ComparisonType, DetailCommand, FieldMapping};

// ============================================================================
// SNS Topic
// ============================================================================

pub fn topic_cli_command() -> CliCommand {
    CliCommand {
        service: "sns",
        operation: "list-topics",
        json_path: "Topics",
        id_field: "TopicArn",
        is_global: false,
        extra_args: &[],
    }
}

pub fn topic_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "TopicArn",
            cli_field: "TopicArn",
            comparison_type: ComparisonType::Exact,
        },
        // list-topics only returns TopicArn
        // Additional fields require get-topic-attributes detail command
    ]
}

pub fn topic_detail_commands() -> Vec<DetailCommand> {
    vec![DetailCommand {
        service: "sns",
        operation: "get-topic-attributes",
        id_arg: "--topic-arn",
        json_path: "Attributes",
        is_global: false,
        extra_args: &[],
    }]
}

// ============================================================================
// SQS Queue
// ============================================================================

pub fn queue_cli_command() -> CliCommand {
    CliCommand {
        service: "sqs",
        operation: "list-queues",
        json_path: "QueueUrls",
        id_field: "", // list-queues returns array of URL strings
        is_global: false,
        extra_args: &[],
    }
}

pub fn queue_field_mappings() -> Vec<FieldMapping> {
    vec![
        FieldMapping {
            dash_field: "QueueUrl",
            cli_field: "QueueUrl",
            comparison_type: ComparisonType::Exact,
        },
        // list-queues only returns QueueUrls
        // Additional fields require get-queue-attributes detail command
    ]
}

pub fn queue_detail_commands() -> Vec<DetailCommand> {
    vec![DetailCommand {
        service: "sqs",
        operation: "get-queue-attributes",
        id_arg: "--queue-url",
        json_path: "Attributes",
        is_global: false,
        extra_args: &[],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_cli_command() {
        let cmd = topic_cli_command();
        assert_eq!(cmd.service, "sns");
        assert_eq!(cmd.operation, "list-topics");
        assert_eq!(cmd.id_field, "TopicArn");
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_topic_field_mappings() {
        let mappings = topic_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "TopicArn"));
    }

    #[test]
    fn test_topic_detail_commands() {
        let details = topic_detail_commands();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].operation, "get-topic-attributes");
    }

    #[test]
    fn test_queue_cli_command() {
        let cmd = queue_cli_command();
        assert_eq!(cmd.service, "sqs");
        assert_eq!(cmd.operation, "list-queues");
        assert!(cmd.id_field.is_empty());
        assert!(!cmd.is_global);
    }

    #[test]
    fn test_queue_field_mappings() {
        let mappings = queue_field_mappings();
        assert!(mappings.iter().any(|m| m.dash_field == "QueueUrl"));
    }

    #[test]
    fn test_queue_detail_commands() {
        let details = queue_detail_commands();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].operation, "get-queue-attributes");
    }
}
