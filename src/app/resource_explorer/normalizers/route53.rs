use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Route53 Hosted Zones
pub struct Route53HostedZoneNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for Route53HostedZoneNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("Id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-hosted-zone")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Route53::HostedZone", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Route53::HostedZone {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Route53::HostedZone".to_string(),
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
        // Route53 hosted zones can be related to ELBs, CloudFront distributions, etc.
        // Implementation would analyze other resources for DNS references
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Route53::HostedZone"
    }
}
