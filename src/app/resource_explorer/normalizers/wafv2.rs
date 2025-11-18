use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for AWS WAFv2 Web ACLs
pub struct WafV2WebAclNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for WafV2WebAclNormalizer {
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
            .unwrap_or("unknown-web-acl")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Scope")
            .and_then(|v| v.as_str())
            .map(|s| format!("ACTIVE ({})", s));

        // Fetch tags asynchronously from AWS API with caching


        let tags = aws_client


            .fetch_tags_for_resource("AWS::WAFv2::WebACL", &resource_id, account, region)


            .await


            .unwrap_or_else(|e| {


                tracing::warn!("Failed to fetch tags for AWS::WAFv2::WebACL {}: {}", resource_id, e);


                Vec::new()


            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::WAFv2::WebACL".to_string(),
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
        // WAFv2 Web ACLs may be associated with:
        // - CloudFront distributions
        // - Application Load Balancers
        // - API Gateway stages
        // - AppSync GraphQL APIs
        // - Amazon Cognito user pools
        // These relationships would require additional API calls to AWS WAF and target services
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::WAFv2::WebACL"
    }
}

