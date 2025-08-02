use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_sns as sns;
use std::sync::Arc;

pub struct SNSService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SNSService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List SNS topics
    pub async fn list_topics(
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

        let client = sns::Client::new(&aws_config);
        let mut paginator = client.list_topics().into_paginator().send();

        let mut topics = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(topic_list) = page.topics {
                for topic in topic_list {
                    if let Some(topic_arn) = topic.topic_arn {
                        // Get topic attributes for more detailed information
                        if let Ok(topic_details) = self
                            .get_topic_attributes_internal(&client, &topic_arn)
                            .await
                        {
                            topics.push(topic_details);
                        } else {
                            // Fallback to basic topic info if attributes fail
                            let mut basic_topic = serde_json::Map::new();
                            basic_topic.insert(
                                "TopicArn".to_string(),
                                serde_json::Value::String(topic_arn.clone()),
                            );
                            // Extract topic name from ARN
                            let topic_name = topic_arn.split(':').next_back().unwrap_or(&topic_arn);
                            basic_topic.insert(
                                "Name".to_string(),
                                serde_json::Value::String(topic_name.to_string()),
                            );
                            topics.push(serde_json::Value::Object(basic_topic));
                        }
                    }
                }
            }
        }

        Ok(topics)
    }

    /// Get detailed information for specific SNS topic
    pub async fn describe_topic(
        &self,
        account_id: &str,
        region: &str,
        topic_arn: &str,
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

        let client = sns::Client::new(&aws_config);
        self.get_topic_attributes_internal(&client, topic_arn).await
    }

    async fn get_topic_attributes_internal(
        &self,
        client: &sns::Client,
        topic_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_topic_attributes()
            .topic_arn(topic_arn)
            .send()
            .await?;

        let mut json = serde_json::Map::new();

        // Add the topic ARN
        json.insert(
            "TopicArn".to_string(),
            serde_json::Value::String(topic_arn.to_string()),
        );

        // Extract topic name from ARN
        let topic_name = topic_arn.split(':').next_back().unwrap_or(topic_arn);
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(topic_name.to_string()),
        );

        // Add attributes if available
        if let Some(attributes) = response.attributes {
            for (key, value) in attributes {
                match key.as_str() {
                    "DisplayName" => {
                        if !value.is_empty() {
                            json.insert(
                                "DisplayName".to_string(),
                                serde_json::Value::String(value),
                            );
                        }
                    }
                    "SubscriptionsConfirmed" => {
                        if let Ok(count) = value.parse::<i64>() {
                            json.insert(
                                "SubscriptionsConfirmed".to_string(),
                                serde_json::Value::Number(count.into()),
                            );
                        }
                    }
                    "SubscriptionsPending" => {
                        if let Ok(count) = value.parse::<i64>() {
                            json.insert(
                                "SubscriptionsPending".to_string(),
                                serde_json::Value::Number(count.into()),
                            );
                        }
                    }
                    "SubscriptionsDeleted" => {
                        if let Ok(count) = value.parse::<i64>() {
                            json.insert(
                                "SubscriptionsDeleted".to_string(),
                                serde_json::Value::Number(count.into()),
                            );
                        }
                    }
                    "DeliveryPolicy" => {
                        if !value.is_empty() {
                            json.insert(
                                "DeliveryPolicy".to_string(),
                                serde_json::Value::String(value),
                            );
                        }
                    }
                    "EffectiveDeliveryPolicy" => {
                        if !value.is_empty() {
                            json.insert(
                                "EffectiveDeliveryPolicy".to_string(),
                                serde_json::Value::String(value),
                            );
                        }
                    }
                    "Policy" => {
                        if !value.is_empty() {
                            json.insert("Policy".to_string(), serde_json::Value::String(value));
                        }
                    }
                    "Owner" => {
                        json.insert("Owner".to_string(), serde_json::Value::String(value));
                    }
                    _ => {
                        // Store other attributes as-is
                        json.insert(key, serde_json::Value::String(value));
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
