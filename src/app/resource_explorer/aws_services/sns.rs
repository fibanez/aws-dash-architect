use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_sns as sns;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct SNSService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SNSService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List SNS topics with optional detailed security information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic topic info quickly.
    ///   If true (Phase 2), includes attributes, subscriptions, tags.
    pub async fn list_topics(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("SNS", "list_topics", Some(region));

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
                        // Extract topic name for status reporting
                        let topic_name = topic_arn.split(':').next_back().unwrap_or(&topic_arn);

                        // Only fetch details if requested (Phase 2)
                        let mut topic_details = if include_details {
                            // Get topic attributes for more detailed information
                            report_status("SNS", "get_topic_attributes", Some(topic_name));
                            match self
                                .get_topic_attributes_internal(&client, &topic_arn)
                                .await
                            {
                                Ok(details) => details,
                                Err(e) => {
                                    tracing::debug!(
                                        "Could not get topic attributes for {}: {}",
                                        topic_name,
                                        e
                                    );
                                    // Fallback to basic topic info if attributes fail
                                    let mut basic_topic = serde_json::Map::new();
                                    basic_topic.insert(
                                        "TopicArn".to_string(),
                                        serde_json::Value::String(topic_arn.clone()),
                                    );
                                    basic_topic.insert(
                                        "Name".to_string(),
                                        serde_json::Value::String(topic_name.to_string()),
                                    );
                                    serde_json::Value::Object(basic_topic)
                                }
                            }
                        } else {
                            // Phase 1: basic topic info only
                            let mut basic_topic = serde_json::Map::new();
                            basic_topic.insert(
                                "TopicArn".to_string(),
                                serde_json::Value::String(topic_arn.clone()),
                            );
                            basic_topic.insert(
                                "Name".to_string(),
                                serde_json::Value::String(topic_name.to_string()),
                            );
                            serde_json::Value::Object(basic_topic)
                        };

                        // Add additional details only if requested
                        if include_details {
                            if let serde_json::Value::Object(ref mut details) = topic_details {
                                // Get topic subscriptions
                                report_status(
                                    "SNS",
                                    "list_subscriptions_by_topic",
                                    Some(topic_name),
                                );
                                match self
                                    .list_subscriptions_by_topic_internal(&client, &topic_arn)
                                    .await
                                {
                                    Ok(subs) => {
                                        details.insert("Subscriptions".to_string(), subs);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get subscriptions for {}: {}",
                                            topic_name,
                                            e
                                        );
                                    }
                                }

                                // Get topic tags
                                report_status("SNS", "list_tags_for_resource", Some(topic_name));
                                match self
                                    .list_tags_for_resource_internal(&client, &topic_arn)
                                    .await
                                {
                                    Ok(tags) => {
                                        details.insert("Tags".to_string(), tags);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get tags for {}: {}",
                                            topic_name,
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        topics.push(topic_details);
                    }
                }
            }
        }

        report_status_done("SNS", "list_topics", Some(region));
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
        let response = timeout(
            Duration::from_secs(10),
            client.get_topic_attributes().topic_arn(topic_arn).send(),
        )
        .await
        .with_context(|| "get_topic_attributes timed out")?
        .with_context(|| format!("Failed to get topic attributes for {}", topic_arn))?;

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
                            // Try to parse as JSON
                            if let Ok(policy_json) =
                                serde_json::from_str::<serde_json::Value>(&value)
                            {
                                json.insert("DeliveryPolicy".to_string(), policy_json);
                            } else {
                                json.insert(
                                    "DeliveryPolicy".to_string(),
                                    serde_json::Value::String(value),
                                );
                            }
                        }
                    }
                    "EffectiveDeliveryPolicy" => {
                        if !value.is_empty() {
                            // Try to parse as JSON
                            if let Ok(policy_json) =
                                serde_json::from_str::<serde_json::Value>(&value)
                            {
                                json.insert("EffectiveDeliveryPolicy".to_string(), policy_json);
                            } else {
                                json.insert(
                                    "EffectiveDeliveryPolicy".to_string(),
                                    serde_json::Value::String(value),
                                );
                            }
                        }
                    }
                    "Policy" => {
                        if !value.is_empty() {
                            // Parse policy as JSON for better display
                            if let Ok(policy_json) =
                                serde_json::from_str::<serde_json::Value>(&value)
                            {
                                json.insert("Policy".to_string(), policy_json);
                            } else {
                                json.insert("Policy".to_string(), serde_json::Value::String(value));
                            }
                        }
                    }
                    "Owner" => {
                        json.insert("Owner".to_string(), serde_json::Value::String(value));
                    }
                    // Security-related attributes
                    "KmsMasterKeyId" => {
                        if !value.is_empty() {
                            json.insert(
                                "KmsMasterKeyId".to_string(),
                                serde_json::Value::String(value),
                            );
                        }
                    }
                    "FifoTopic" => {
                        let is_fifo = value.to_lowercase() == "true";
                        json.insert("FifoTopic".to_string(), serde_json::Value::Bool(is_fifo));
                    }
                    "ContentBasedDeduplication" => {
                        let enabled = value.to_lowercase() == "true";
                        json.insert(
                            "ContentBasedDeduplication".to_string(),
                            serde_json::Value::Bool(enabled),
                        );
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

    // Internal function to list subscriptions by topic
    async fn list_subscriptions_by_topic_internal(
        &self,
        client: &sns::Client,
        topic_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .list_subscriptions_by_topic()
                .topic_arn(topic_arn)
                .send(),
        )
        .await
        .with_context(|| "list_subscriptions_by_topic timed out")?;

        match response {
            Ok(result) => {
                let mut subscriptions = Vec::new();

                if let Some(subs) = result.subscriptions {
                    for sub in subs {
                        let mut sub_json = serde_json::Map::new();

                        if let Some(arn) = sub.subscription_arn {
                            sub_json.insert(
                                "SubscriptionArn".to_string(),
                                serde_json::Value::String(arn),
                            );
                        }
                        if let Some(protocol) = sub.protocol {
                            sub_json.insert(
                                "Protocol".to_string(),
                                serde_json::Value::String(protocol),
                            );
                        }
                        if let Some(endpoint) = sub.endpoint {
                            sub_json.insert(
                                "Endpoint".to_string(),
                                serde_json::Value::String(endpoint),
                            );
                        }
                        if let Some(owner) = sub.owner {
                            sub_json.insert("Owner".to_string(), serde_json::Value::String(owner));
                        }

                        subscriptions.push(serde_json::Value::Object(sub_json));
                    }
                }

                Ok(serde_json::json!({
                    "Items": subscriptions,
                    "Count": subscriptions.len()
                }))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("NotFound") || error_str.contains("AccessDenied") {
                    Ok(serde_json::json!({
                        "Items": [],
                        "Note": "No subscriptions or access denied"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to list subscriptions: {}", e))
                }
            }
        }
    }

    // Internal function to get subscription attributes
    async fn get_subscription_attributes_internal(
        &self,
        client: &sns::Client,
        subscription_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_subscription_attributes()
                .subscription_arn(subscription_arn)
                .send(),
        )
        .await
        .with_context(|| "get_subscription_attributes timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();

                if let Some(attributes) = result.attributes {
                    for (key, value) in attributes {
                        match key.as_str() {
                            "FilterPolicy" => {
                                if !value.is_empty() {
                                    if let Ok(policy_json) =
                                        serde_json::from_str::<serde_json::Value>(&value)
                                    {
                                        json.insert("FilterPolicy".to_string(), policy_json);
                                    } else {
                                        json.insert(
                                            "FilterPolicy".to_string(),
                                            serde_json::Value::String(value),
                                        );
                                    }
                                }
                            }
                            "RawMessageDelivery" => {
                                let enabled = value.to_lowercase() == "true";
                                json.insert(
                                    "RawMessageDelivery".to_string(),
                                    serde_json::Value::Bool(enabled),
                                );
                            }
                            "RedrivePolicy" => {
                                if !value.is_empty() {
                                    if let Ok(policy_json) =
                                        serde_json::from_str::<serde_json::Value>(&value)
                                    {
                                        json.insert("RedrivePolicy".to_string(), policy_json);
                                    } else {
                                        json.insert(
                                            "RedrivePolicy".to_string(),
                                            serde_json::Value::String(value),
                                        );
                                    }
                                }
                            }
                            "SubscriptionArn" => {
                                json.insert(
                                    "SubscriptionArn".to_string(),
                                    serde_json::Value::String(value),
                                );
                            }
                            "TopicArn" => {
                                json.insert(
                                    "TopicArn".to_string(),
                                    serde_json::Value::String(value),
                                );
                            }
                            "Owner" => {
                                json.insert("Owner".to_string(), serde_json::Value::String(value));
                            }
                            "Protocol" => {
                                json.insert(
                                    "Protocol".to_string(),
                                    serde_json::Value::String(value),
                                );
                            }
                            "Endpoint" => {
                                json.insert(
                                    "Endpoint".to_string(),
                                    serde_json::Value::String(value),
                                );
                            }
                            "ConfirmationWasAuthenticated" => {
                                let confirmed = value.to_lowercase() == "true";
                                json.insert(
                                    "ConfirmationWasAuthenticated".to_string(),
                                    serde_json::Value::Bool(confirmed),
                                );
                            }
                            _ => {
                                json.insert(key, serde_json::Value::String(value));
                            }
                        }
                    }
                }

                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("NotFound") || error_str.contains("AccessDenied") {
                    Ok(serde_json::json!({
                        "Note": "Subscription not found or access denied"
                    }))
                } else {
                    Err(anyhow::anyhow!(
                        "Failed to get subscription attributes: {}",
                        e
                    ))
                }
            }
        }
    }

    // Internal function to list tags for resource
    async fn list_tags_for_resource_internal(
        &self,
        client: &sns::Client,
        resource_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .list_tags_for_resource()
                .resource_arn(resource_arn)
                .send(),
        )
        .await
        .with_context(|| "list_tags_for_resource timed out")?;

        match response {
            Ok(result) => {
                let mut tags_json = serde_json::Map::new();
                if let Some(tags) = result.tags {
                    for tag in tags {
                        // tag.key and tag.value are String, not Option<String>
                        tags_json.insert(tag.key, serde_json::Value::String(tag.value));
                    }
                }
                Ok(serde_json::Value::Object(tags_json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("AccessDenied") || error_str.contains("AuthorizationError") {
                    Ok(serde_json::json!({
                        "Note": "Tags not accessible"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to list tags: {}", e))
                }
            }
        }
    }

    /// Public function to list subscriptions by topic
    pub async fn list_subscriptions_by_topic(
        &self,
        account_id: &str,
        region: &str,
        topic_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("SNS", "list_subscriptions_by_topic", Some(topic_arn));

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sns::Client::new(&aws_config);
        let result = self
            .list_subscriptions_by_topic_internal(&client, topic_arn)
            .await;

        report_status_done("SNS", "list_subscriptions_by_topic", Some(topic_arn));
        result
    }

    /// Public function to get subscription attributes
    pub async fn get_subscription_attributes(
        &self,
        account_id: &str,
        region: &str,
        subscription_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("SNS", "get_subscription_attributes", Some(subscription_arn));

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sns::Client::new(&aws_config);
        let result = self
            .get_subscription_attributes_internal(&client, subscription_arn)
            .await;

        report_status_done("SNS", "get_subscription_attributes", Some(subscription_arn));
        result
    }

    /// Public function to list tags for resource
    pub async fn list_tags_for_resource(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("SNS", "list_tags_for_resource", Some(resource_arn));

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sns::Client::new(&aws_config);
        let result = self
            .list_tags_for_resource_internal(&client, resource_arn)
            .await;

        report_status_done("SNS", "list_tags_for_resource", Some(resource_arn));
        result
    }

    /// Get details for a specific SNS topic (Phase 2 enrichment)
    /// Returns only the detail fields to be merged into existing resource data
    pub async fn get_topic_details(
        &self,
        account_id: &str,
        region: &str,
        topic_arn: &str,
    ) -> Result<serde_json::Value> {
        report_status("SNS", "get_topic_details", Some(topic_arn));
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sns::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Get topic attributes
        if let Ok(serde_json::Value::Object(attrs_map)) =
            self.get_topic_attributes_internal(&client, topic_arn).await
        {
            for (key, value) in attrs_map {
                details.insert(key, value);
            }
        }

        // Get topic subscriptions
        if let Ok(subs) = self
            .list_subscriptions_by_topic_internal(&client, topic_arn)
            .await
        {
            details.insert("Subscriptions".to_string(), subs);
        }

        // Get topic tags
        if let Ok(tags) = self
            .list_tags_for_resource_internal(&client, topic_arn)
            .await
        {
            details.insert("Tags".to_string(), tags);
        }

        report_status_done("SNS", "get_topic_details", Some(topic_arn));
        Ok(serde_json::Value::Object(details))
    }
}
