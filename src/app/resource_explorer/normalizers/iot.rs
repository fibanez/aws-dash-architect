use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for IoT Things
pub struct IoTThingNormalizer;

impl ResourceNormalizer for IoTThingNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let thing_name = raw_response
            .get("ThingName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-thing")
            .to_string();

        let display_name = extract_display_name(&raw_response, &thing_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::IoT::Thing".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: thing_name,
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
