use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for IoT Things
pub struct IoTThingNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for IoTThingNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let thing_name = raw_response
            .get("ThingName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-thing")
            .to_string();

        let display_name = extract_display_name(&raw_response, &thing_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::IoT::Thing", &thing_name, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::IoT::Thing {}: {}",
                    thing_name,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::IoT::Thing".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: thing_name,
            display_name,
            status,
            properties: raw_response,
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
        // IoT Things can have relationships with:
        // - Thing Types (for categorization)
        // - Thing Groups (for organization)
        // - Certificates (for authentication)
        // - Policies (for authorization)
        // - Billing Groups (for cost tracking)
        //
        // These would require additional API calls to discover the relationships
        // or parsing configuration details from the thing attributes
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::IoT::Thing"
    }
}
