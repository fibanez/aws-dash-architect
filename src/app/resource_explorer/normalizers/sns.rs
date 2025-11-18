use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for SNS Topics
pub struct SNSTopicNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for SNSTopicNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let topic_arn = raw_response
            .get("TopicArn")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-topic-arn")
            .to_string();

        // Extract topic name from ARN for display
        let topic_name = topic_arn.split(':').next_back().unwrap_or(&topic_arn);

        let display_name = raw_response
            .get("DisplayName")
            .and_then(|v| v.as_str())
            .unwrap_or(topic_name)
            .to_string();

        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::SNS::Topic", &topic_arn, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::SNS::Topic {}: {}", topic_arn, e);

                Vec::new()

            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::SNS::Topic".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: topic_arn,
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        __entry: &ResourceEntry,
        __all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // SNS topics can have relationships with subscriptions, Lambda functions, etc.
        // but we'd need subscription data for that
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::SNS::Topic"
    }
}

