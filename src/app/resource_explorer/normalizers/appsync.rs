use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for AppSync GraphQL APIs
pub struct AppSyncGraphQLApiNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for AppSyncGraphQLApiNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let api_id = raw_response
            .get("ApiId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-api")
            .to_string();

        // Use API name if available, otherwise fallback to API ID
        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&api_id)
            .to_string();

        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::AppSync::GraphQLApi", &api_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::AppSync::GraphQLApi {}: {}",
                    api_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::AppSync::GraphQLApi".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: api_id,
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
        // AppSync APIs can have relationships with:
        // - Cognito User Pools (for authentication)
        // - IAM roles (for execution)
        // - Lambda functions (as data sources)
        // - DynamoDB tables (as data sources)
        //
        // These would require parsing the configuration details or additional API calls
        // to discover data sources and resolvers
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::AppSync::GraphQLApi"
    }
}
