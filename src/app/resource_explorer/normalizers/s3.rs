use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for S3 Buckets
pub struct S3BucketNormalizer;

impl ResourceNormalizer for S3BucketNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let bucket_name = raw_response
            .get("BucketName")
            .or_else(|| raw_response.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-bucket")
            .to_string();

        let display_name = extract_display_name(&raw_response, &bucket_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

        // Extract creation date
        let _creation_date = raw_response
            .get("CreationDate")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::S3::Bucket".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: bucket_name.clone(),
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
        // S3 buckets typically don't have direct relationships with other resources
        // in the context of resource listing, but they might be referenced by other services
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::S3::Bucket"
    }
}
