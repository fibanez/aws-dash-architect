use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for X-Ray Sampling Rules
pub struct XRaySamplingRuleNormalizer;

impl ResourceNormalizer for XRaySamplingRuleNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("RuleName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-sampling-rule")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::XRay::SamplingRule".to_string(),
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
        // X-Ray sampling rules don't have direct relationships with other AWS resources
        // in the typical resource dependency sense, but they apply to traced services
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::XRay::SamplingRule"
    }
}