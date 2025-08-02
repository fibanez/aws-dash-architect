use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for SSM Parameters
pub struct SSMParameterNormalizer;

impl ResourceNormalizer for SSMParameterNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let parameter_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-parameter")
            .to_string();

        let display_name = extract_display_name(&raw_response, &parameter_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::SSM::Parameter".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: parameter_name,
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
        // SSM parameters can be used by various AWS services
        // but we'd need to scan configurations across services to establish relationships
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::SSM::Parameter"
    }
}

/// Normalizer for SSM Documents
pub struct SSMDocumentNormalizer;

impl ResourceNormalizer for SSMDocumentNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let document_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-document")
            .to_string();

        let display_name = extract_display_name(&raw_response, &document_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::SSM::Document".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: document_name,
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
        // SSM documents can be used by EC2 instances, maintenance windows, etc.
        // but we'd need to analyze execution records to establish relationships
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::SSM::Document"
    }
}
