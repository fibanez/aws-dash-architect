use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_sqs as sqs;
use std::sync::Arc;

pub struct SQSService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SQSService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List SQS queues
    pub async fn list_queues(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
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
                // Get queue attributes for more detailed information
                if let Ok(queue_details) = self
                    .get_queue_attributes_internal(&client, &queue_url)
                    .await
                {
                    queues.push(queue_details);
                } else {
                    // Fallback to basic queue info if attributes fail
                    let mut basic_queue = serde_json::Map::new();
                    basic_queue.insert(
                        "QueueUrl".to_string(),
                        serde_json::Value::String(queue_url.clone()),
                    );
                    // Extract queue name from URL
                    let queue_name = queue_url.split('/').next_back().unwrap_or(&queue_url);
                    basic_queue.insert(
                        "Name".to_string(),
                        serde_json::Value::String(queue_name.to_string()),
                    );
                    queues.push(serde_json::Value::Object(basic_queue));
                }
            }
        }

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
        let response = client
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(sqs::types::QueueAttributeName::All)
            .send()
            .await?;

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
                        if let Ok(timeout) = value.parse::<i64>() {
                            json.insert(
                                "VisibilityTimeout".to_string(),
                                serde_json::Value::Number(timeout.into()),
                            );
                        }
                    }
                    sqs::types::QueueAttributeName::RedrivePolicy => {
                        json.insert(
                            "RedrivePolicy".to_string(),
                            serde_json::Value::String(value),
                        );
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
}
