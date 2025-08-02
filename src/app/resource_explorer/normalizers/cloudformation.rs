use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for CloudFormation Stacks
pub struct CloudFormationStackNormalizer;

impl ResourceNormalizer for CloudFormationStackNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let _stack_id = raw_response
            .get("StackId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-stack-id")
            .to_string();

        let stack_name = raw_response
            .get("StackName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-stack")
            .to_string();

        let display_name = extract_display_name(&raw_response, &stack_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

        // Extract creation time
        let _creation_date = raw_response
            .get("CreationTime")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CloudFormation::Stack".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: stack_name,
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
        // CloudFormation stacks create and manage other resources,
        // but we'd need the stack resources to establish these relationships
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CloudFormation::Stack"
    }
}
