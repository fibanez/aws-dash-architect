use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for SNS Topics
pub struct SNSTopicNormalizer;

impl ResourceNormalizer for SNSTopicNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
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
        let tags = extract_tags(&raw_response);
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
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // SNS topics can have relationships with subscriptions, Lambda functions, etc.
        // but we'd need subscription data for that
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::SNS::Topic"
    }
}
