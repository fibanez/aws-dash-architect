use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for AWS Organizations Organizational Units
pub struct OrganizationsOUNormalizer;

impl ResourceNormalizer for OrganizationsOUNormalizer {
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
            .unwrap_or("unknown-ou")
            .to_string();

        let display_name = raw_response
            .get("DisplayName")
            .and_then(|v| v.as_str())
            .or_else(|| raw_response.get("Name").and_then(|v| v.as_str()))
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some("Active".to_string()));

        // Extract basic properties for normalized view
        let mut properties = serde_json::Map::new();

        if let Some(status) = &status {
            properties.insert(
                "status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(arn) = raw_response.get("Arn") {
            properties.insert("arn".to_string(), arn.clone());
        }

        if let Some(display_name_val) = raw_response.get("DisplayName") {
            properties.insert("display_name".to_string(), display_name_val.clone());
        }

        let account_color = assign_account_color(account);
        let region_color = assign_region_color(region);

        Ok(ResourceEntry {
            resource_type: "AWS::Organizations::OrganizationalUnit".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: serde_json::Value::Object(properties),
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags: Vec::new(),
            relationships: Vec::new(), // Will be populated by extract_relationships
            account_color,
            region_color,
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Organizations relationships are complex and would require additional API calls
        // to get account memberships and policy attachments
        // For now, we'll leave this empty but this could be enhanced to:
        // - Map to child OUs
        // - Map to accounts in this OU
        // - Map to attached Service Control Policies

        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Organizations::OrganizationalUnit"
    }
}
