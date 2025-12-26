use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for S3 Buckets
pub struct S3BucketNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for S3BucketNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let bucket_name = raw_response
            .get("BucketName")
            .or_else(|| raw_response.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-bucket")
            .to_string();

        let display_name = extract_display_name(&raw_response, &bucket_name);
        let status = extract_status(&raw_response);

        // Fetch tags asynchronously from AWS API with caching
        let tags = aws_client
            .fetch_tags_for_resource("AWS::S3::Bucket", &bucket_name, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch tags for S3 bucket {}: {}", bucket_name, e);
                Vec::new()
            });

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
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::S3::Bucket"
    }
}
