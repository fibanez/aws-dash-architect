use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for CloudTrail Trails
pub struct CloudTrailNormalizer;

impl ResourceNormalizer for CloudTrailNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-trail")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CloudTrail::Trail".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
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
        // CloudTrail trails can be related to S3 buckets, SNS topics, CloudWatch logs, etc.
        // Implementation would analyze S3BucketName, SnsTopicArn, CloudWatchLogsLogGroupArn
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CloudTrail::Trail"
    }
}
