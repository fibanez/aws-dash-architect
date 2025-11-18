use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for API Gateway REST APIs
pub struct ApiGatewayRestApiNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ApiGatewayRestApiNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let api_id = raw_response
            .get("Id")
            .or_else(|| raw_response.get("RestApiId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-api")
            .to_string();

        let display_name = extract_display_name(&raw_response, &api_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::ApiGateway::RestApi", &api_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::ApiGateway::RestApi {}: {}", api_id, e);

                Vec::new()

            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::ApiGateway::RestApi".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: api_id,
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
        // API Gateway can have relationships with Lambda functions, IAM roles, etc.
        // but we'd need to parse the API configuration for that
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ApiGateway::RestApi"
    }
}

