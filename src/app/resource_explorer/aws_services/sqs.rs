use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_sqs as sqs;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct SQSService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SQSService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List SQS queues with optional detailed security information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic queue info quickly.
    ///   If true (Phase 2), includes attributes, tags, DLQ sources.
    pub async fn list_queues(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("SQS", "list_queues", Some(region));

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = sqs::Client::new(&aws_config);
        let response = client.list_queues().send().await?;

        let mut queues = Vec::new();
        if let Some(queue_urls) = response.queue_urls {
            for queue_url in queue_urls {
                // Extract queue name for status reporting
                let queue_name = queue_url.split('/').next_back().unwrap_or(&queue_url);

                // Only fetch details if requested (Phase 2)
                let mut queue_details = if include_details {
                    // Get queue attributes for more detailed information
                    report_status("SQS", "get_queue_attributes", Some(queue_name));
                    match self
                        .get_queue_attributes_internal(&client, &queue_url)
                        .await
                    {
                        Ok(details) => details,
                        Err(e) => {
                            tracing::debug!(
                                "Could not get queue attributes for {}: {}",
                                queue_name,
                                e
                            );
                            // Fallback to basic queue info if attributes fail
                            let mut basic_queue = serde_json::Map::new();
                            basic_queue.insert(
                                "QueueUrl".to_string(),
                                serde_json::Value::String(queue_url.clone()),
                            );
                            basic_queue.insert(
                                "Name".to_string(),
                                serde_json::Value::String(queue_name.to_string()),
                            );
                            serde_json::Value::Object(basic_queue)
                        }
                    }
                } else {
                    // Phase 1: basic queue info only
                    let mut basic_queue = serde_json::Map::new();
                    basic_queue.insert(
                        "QueueUrl".to_string(),
                        serde_json::Value::String(queue_url.clone()),
                    );
                    basic_queue.insert(
                        "Name".to_string(),
                        serde_json::Value::String(queue_name.to_string()),
                    );
                    serde_json::Value::Object(basic_queue)
                };

                // Add additional details only if requested
                if include_details {
                    if let serde_json::Value::Object(ref mut details) = queue_details {
                        // Get queue tags
                        report_status("SQS", "list_queue_tags", Some(queue_name));
                        match self.list_queue_tags_internal(&client, &queue_url).await {
                            Ok(tags) => {
                                details.insert("Tags".to_string(), tags);
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Could not get queue tags for {}: {}",
                                    queue_name,
                                    e
                                );
                            }
                        }

                        // Get dead letter source queues
                        report_status("SQS", "list_dead_letter_source_queues", Some(queue_name));
                        match self
                            .list_dead_letter_source_queues_internal(&client, &queue_url)
                            .await
                        {
                            Ok(sources) => {
                                details.insert("DeadLetterSourceQueues".to_string(), sources);
                            }
                            Err(e) => {
                                tracing::debug!(
                                    "Could not get DLQ sources for {}: {}",
                                    queue_name,
                                    e
                                );
                            }
                        }
                    }
                }

                queues.push(queue_details);
            }
        }

        report_status_done("SQS", "list_queues", Some(region));
        Ok(queues)
    }

    /// Get detailed information for specific SQS queue
    pub async fn describe_queue(
        &self,
        account_id: &str,
        region: &str,
        queue_url: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = sqs::Client::new(&aws_config);
        self.get_queue_attributes_internal(&client, queue_url).await
    }

    async fn get_queue_attributes_internal(
        &self,
        client: &sqs::Client,
        queue_url: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_queue_attributes()
                .queue_url(queue_url)
                .attribute_names(sqs::types::QueueAttributeName::All)
                .send(),
        )
        .await
        .with_context(|| "get_queue_attributes timed out")?
        .with_context(|| format!("Failed to get queue attributes for {}", queue_url))?;

        let mut json = serde_json::Map::new();

        // Add the queue URL
        json.insert(
            "QueueUrl".to_string(),
            serde_json::Value::String(queue_url.to_string()),
        );

        // Extract queue name from URL
        let queue_name = queue_url.split('/').next_back().unwrap_or(queue_url);
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(queue_name.to_string()),
        );

        // Add attributes if available
        if let Some(attributes) = response.attributes {
            for (attr_name, value) in attributes {
                match attr_name {
                    sqs::types::QueueAttributeName::QueueArn => {
                        json.insert("QueueArn".to_string(), serde_json::Value::String(value));
                    }
                    sqs::types::QueueAttributeName::ApproximateNumberOfMessages => {
                        if let Ok(count) = value.parse::<i64>() {
                            json.insert(
                                "ApproximateNumberOfMessages".to_string(),
                                serde_json::Value::Number(count.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::ApproximateNumberOfMessagesNotVisible => {
                        if let Ok(count) = value.parse::<i64>() {
                            json.insert(
                                "ApproximateNumberOfMessagesNotVisible".to_string(),
                                serde_json::Value::Number(count.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::ApproximateNumberOfMessagesDelayed => {
                        if let Ok(count) = value.parse::<i64>() {
                            json.insert(
                                "ApproximateNumberOfMessagesDelayed".to_string(),
                                serde_json::Value::Number(count.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::CreatedTimestamp => {
                        if let Ok(timestamp) = value.parse::<i64>() {
                            json.insert(
                                "CreatedTimestamp".to_string(),
                                serde_json::Value::Number(timestamp.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::LastModifiedTimestamp => {
                        if let Ok(timestamp) = value.parse::<i64>() {
                            json.insert(
                                "LastModifiedTimestamp".to_string(),
                                serde_json::Value::Number(timestamp.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::VisibilityTimeout => {
                        if let Ok(timeout_val) = value.parse::<i64>() {
                            json.insert(
                                "VisibilityTimeout".to_string(),
                                serde_json::Value::Number(timeout_val.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::RedrivePolicy => {
                        // Try to parse as JSON, fallback to string
                        if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&value) {
                            json.insert("RedrivePolicy".to_string(), policy_json);
                        } else {
                            json.insert(
                                "RedrivePolicy".to_string(),
                                serde_json::Value::String(value),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::MessageRetentionPeriod => {
                        if let Ok(period) = value.parse::<i64>() {
                            json.insert(
                                "MessageRetentionPeriod".to_string(),
                                serde_json::Value::Number(period.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::DelaySeconds => {
                        if let Ok(delay) = value.parse::<i64>() {
                            json.insert(
                                "DelaySeconds".to_string(),
                                serde_json::Value::Number(delay.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::ReceiveMessageWaitTimeSeconds => {
                        if let Ok(wait_time) = value.parse::<i64>() {
                            json.insert(
                                "ReceiveMessageWaitTimeSeconds".to_string(),
                                serde_json::Value::Number(wait_time.into()),
                            );
                        }
                    }
                    // Security-related attributes
                    sqs::types::QueueAttributeName::KmsMasterKeyId => {
                        json.insert(
                            "KmsMasterKeyId".to_string(),
                            serde_json::Value::String(value),
                        );
                    }
                    sqs::types::QueueAttributeName::KmsDataKeyReusePeriodSeconds => {
                        if let Ok(period) = value.parse::<i64>() {
                            json.insert(
                                "KmsDataKeyReusePeriodSeconds".to_string(),
                                serde_json::Value::Number(period.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::SqsManagedSseEnabled => {
                        let enabled = value.to_lowercase() == "true";
                        json.insert(
                            "SqsManagedSseEnabled".to_string(),
                            serde_json::Value::Bool(enabled),
                        );
                    }
                    sqs::types::QueueAttributeName::Policy => {
                        // Parse policy as JSON for better display
                        if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&value) {
                            json.insert("Policy".to_string(), policy_json);
                        } else {
                            json.insert("Policy".to_string(), serde_json::Value::String(value));
                        }
                    }
                    sqs::types::QueueAttributeName::FifoQueue => {
                        let is_fifo = value.to_lowercase() == "true";
                        json.insert("FifoQueue".to_string(), serde_json::Value::Bool(is_fifo));
                    }
                    sqs::types::QueueAttributeName::ContentBasedDeduplication => {
                        let enabled = value.to_lowercase() == "true";
                        json.insert(
                            "ContentBasedDeduplication".to_string(),
                            serde_json::Value::Bool(enabled),
                        );
                    }
                    sqs::types::QueueAttributeName::DeduplicationScope => {
                        json.insert(
                            "DeduplicationScope".to_string(),
                            serde_json::Value::String(value),
                        );
                    }
                    sqs::types::QueueAttributeName::FifoThroughputLimit => {
                        json.insert(
                            "FifoThroughputLimit".to_string(),
                            serde_json::Value::String(value),
                        );
                    }
                    sqs::types::QueueAttributeName::MaximumMessageSize => {
                        if let Ok(size) = value.parse::<i64>() {
                            json.insert(
                                "MaximumMessageSize".to_string(),
                                serde_json::Value::Number(size.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::RedriveAllowPolicy => {
                        // Parse redrive allow policy as JSON
                        if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&value) {
                            json.insert("RedriveAllowPolicy".to_string(), policy_json);
                        } else {
                            json.insert(
                                "RedriveAllowPolicy".to_string(),
                                serde_json::Value::String(value),
                            );
                        }
                    }
                    _ => {
                        // Store other attributes as-is
                        json.insert(format!("{:?}", attr_name), serde_json::Value::String(value));
                    }
                }
            }
        }

        // Add a status field for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        Ok(serde_json::Value::Object(json))
    }

    // Internal function to list queue tags
    async fn list_queue_tags_internal(
        &self,
        client: &sqs::Client,
        queue_url: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.list_queue_tags().queue_url(queue_url).send(),
        )
        .await
        .with_context(|| "list_queue_tags timed out")?;

        match response {
            Ok(result) => {
                let mut tags_json = serde_json::Map::new();
                if let Some(tags) = result.tags {
                    for (key, value) in tags {
                        tags_json.insert(key, serde_json::Value::String(value));
                    }
                }
                Ok(serde_json::Value::Object(tags_json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("AccessDenied") || error_str.contains("InvalidAddress") {
                    Ok(serde_json::json!({
                        "Note": "Tags not accessible or not configured"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to list queue tags: {}", e))
                }
            }
        }
    }

    // Internal function to list dead letter source queues
    async fn list_dead_letter_source_queues_internal(
        &self,
        client: &sqs::Client,
        queue_url: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .list_dead_letter_source_queues()
                .queue_url(queue_url)
                .send(),
        )
        .await
        .with_context(|| "list_dead_letter_source_queues timed out")?;

        match response {
            Ok(result) => {
                let source_urls: Vec<serde_json::Value> = result
                    .queue_urls
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect();

                if source_urls.is_empty() {
                    Ok(serde_json::json!({
                        "SourceQueues": [],
                        "Note": "This queue is not configured as a DLQ for any other queues"
                    }))
                } else {
                    Ok(serde_json::json!({
                        "SourceQueues": source_urls,
                        "Count": source_urls.len()
                    }))
                }
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("AccessDenied") {
                    Ok(serde_json::json!({
                        "SourceQueues": [],
                        "Note": "Access denied to list DLQ sources"
                    }))
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to list dead letter source queues: {}",
                        e
                    ))
                }
            }
        }
    }

    /// Public function to list queue tags
    pub async fn list_queue_tags(
        &self,
        account_id: &str,
        region: &str,
        queue_url: &str,
    ) -> Result<serde_json::Value> {
        report_status("SQS", "list_queue_tags", Some(queue_url));

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sqs::Client::new(&aws_config);
        let result = self.list_queue_tags_internal(&client, queue_url).await;

        report_status_done("SQS", "list_queue_tags", Some(queue_url));
        result
    }

    /// Public function to list dead letter source queues
    pub async fn list_dead_letter_source_queues(
        &self,
        account_id: &str,
        region: &str,
        queue_url: &str,
    ) -> Result<serde_json::Value> {
        report_status("SQS", "list_dead_letter_source_queues", Some(queue_url));

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sqs::Client::new(&aws_config);
        let result = self
            .list_dead_letter_source_queues_internal(&client, queue_url)
            .await;

        report_status_done("SQS", "list_dead_letter_source_queues", Some(queue_url));
        result
    }

    /// Get details for a specific SQS queue (Phase 2 enrichment)
    /// Returns only the detail fields to be merged into existing resource data
    pub async fn get_queue_details(
        &self,
        account_id: &str,
        region: &str,
        queue_url: &str,
    ) -> Result<serde_json::Value> {
        report_status("SQS", "get_queue_details", Some(queue_url));
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sqs::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Get queue attributes
        if let Ok(serde_json::Value::Object(attrs_map)) =
            self.get_queue_attributes_internal(&client, queue_url).await
        {
            for (key, value) in attrs_map {
                details.insert(key, value);
            }
        }

        // Get queue tags
        if let Ok(tags) = self.list_queue_tags_internal(&client, queue_url).await {
            details.insert("Tags".to_string(), tags);
        }

        // Get dead letter source queues
        if let Ok(sources) = self
            .list_dead_letter_source_queues_internal(&client, queue_url)
            .await
        {
            details.insert("DeadLetterSourceQueues".to_string(), sources);
        }

        report_status_done("SQS", "get_queue_details", Some(queue_url));
        Ok(serde_json::Value::Object(details))
    }
}
