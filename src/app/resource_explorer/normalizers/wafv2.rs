use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for AWS WAFv2 Web ACLs
pub struct WafV2WebAclNormalizer;

impl ResourceNormalizer for WafV2WebAclNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
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

        let tags = extract_tags(&raw_response);
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
