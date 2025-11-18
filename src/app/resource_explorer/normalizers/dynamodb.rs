use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for DynamoDB Tables
pub struct DynamoDBTableNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for DynamoDBTableNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let table_name = raw_response
            .get("TableName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-table")
            .to_string();

        let display_name = extract_display_name(&raw_response, &table_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::DynamoDB::Table", &table_name, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::DynamoDB::Table {}: {}", table_name, e);

                Vec::new()

            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::DynamoDB::Table".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: table_name,
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
        // DynamoDB tables typically don't have direct relationships with other resources
        // in the context of resource listing
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::DynamoDB::Table"
    }
}

