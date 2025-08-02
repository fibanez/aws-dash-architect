use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Greengrass Component Versions
pub struct GreengrassComponentVersionNormalizer;

impl ResourceNormalizer for GreengrassComponentVersionNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let component_name = raw_response
            .get("ComponentName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-component")
            .to_string();

        // Create a unique resource ID by combining component name and version
        let component_version = raw_response
            .get("ComponentVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-version");

        let resource_id = format!("{}:{}", component_name, component_version);

        // Use component name for display, with version info
        let display_name = format!("{} ({})", component_name, component_version);

        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::GreengrassV2::ComponentVersion".to_string(),
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
        // Greengrass component versions can have relationships with:
        // - Core devices (where they are deployed)
        // - Deployments (that include this component)
        // - Other component versions (dependencies)
        // - Lambda functions (for Lambda components)
        // - Docker images (for Docker-based components)
        //
        // These relationships would require additional API calls to discover
        // deployments and core device associations
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::GreengrassV2::ComponentVersion"
    }
}
