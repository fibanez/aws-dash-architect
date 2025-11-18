use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for SSM Parameters
pub struct SSMParameterNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for SSMParameterNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let parameter_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-parameter")
            .to_string();

        let display_name = extract_display_name(&raw_response, &parameter_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::SSM::Parameter", &parameter_name, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::SSM::Parameter {}: {}", parameter_name, e);

                Vec::new()

            });
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

#[async_trait]
impl AsyncResourceNormalizer for SSMDocumentNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let document_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-document")
            .to_string();

        let display_name = extract_display_name(&raw_response, &document_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::SSM::Document", &document_name, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::SSM::Document {}: {}", document_name, e);

                Vec::new()

            });
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
        // SSM documents can be used by EC2 instances, maintenance windows, etc.
        // but we'd need to analyze execution records to establish relationships
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::SSM::Document"
    }
}

