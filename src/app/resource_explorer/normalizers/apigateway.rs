use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for API Gateway REST APIs
pub struct ApiGatewayRestApiNormalizer;

impl ResourceNormalizer for ApiGatewayRestApiNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let api_id = raw_response
            .get("Id")
            .or_else(|| raw_response.get("RestApiId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-api")
            .to_string();

        let display_name = extract_display_name(&raw_response, &api_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
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
        // API Gateway can have relationships with Lambda functions, IAM roles, etc.
        // but we'd need to parse the API configuration for that
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ApiGateway::RestApi"
    }
}
