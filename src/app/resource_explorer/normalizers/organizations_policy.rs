use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for AWS Organizations Service Control Policies
pub struct OrganizationsPolicyNormalizer;

impl ResourceNormalizer for OrganizationsPolicyNormalizer {
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
            .unwrap_or("unknown-policy")
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

        if let Some(policy_type) = raw_response.get("Type") {
            properties.insert("policy_type".to_string(), policy_type.clone());
        }

        if let Some(description) = raw_response.get("Description") {
            properties.insert("description".to_string(), description.clone());
        }

        if let Some(aws_managed) = raw_response.get("AwsManaged") {
            properties.insert("aws_managed".to_string(), aws_managed.clone());
        }

        let account_color = assign_account_color(account);
        let region_color = assign_region_color(region);

        Ok(ResourceEntry {
            resource_type: "AWS::Organizations::Policy".to_string(),
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
        // Service Control Policy relationships would require additional API calls
        // to get policy attachments to OUs and accounts
        // For now, we'll leave this empty but this could be enhanced to:
        // - Map to OUs this policy is attached to
        // - Map to accounts this policy is attached to
        // - Show policy inheritance hierarchy

        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Organizations::Policy"
    }
}
