use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for OpenSearch Domains
pub struct OpenSearchDomainNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for OpenSearchDomainNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let resource_id = raw_response
            .get("DomainName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-domain")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        // OpenSearch domains don't have a simple status field like other services
        let status = if raw_response
            .get("Processing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("Processing".to_string())
        } else if raw_response
            .get("UpgradeProcessing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("UpgradeProcessing".to_string())
        } else if raw_response
            .get("Deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("Deleted".to_string())
        } else if raw_response
            .get("Created")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("Available".to_string())
        } else {
            Some("Unknown".to_string())
        };

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
            resource_type: "AWS::OpenSearchService::Domain".to_string(),
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
        };

        // Fetch tags (will be empty for resources that don't support tagging)
        entry.tags = aws_client
            .fetch_tags_for_resource(&entry.resource_type, &entry.resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for {} {}: {:?}",
                    entry.resource_type,
                    entry.resource_id,
                    e
                );
                Vec::new()
            });

        Ok(entry)
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::OpenSearchService::Domain"
    }
}
