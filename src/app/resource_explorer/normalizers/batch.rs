use super::super::state::{ResourceEntry, ResourceRelationship};
use super::{utils, AsyncResourceNormalizer};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub struct BatchJobQueueNormalizer;
pub struct BatchComputeEnvironmentNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BatchJobQueueNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let resource_id = raw_response
            .get("JobQueueName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing JobQueueName"))?
            .to_string();

        let display_name = raw_response
            .get("JobQueueName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = utils::extract_status(&raw_response);
        let tags = utils::extract_tags(&raw_response);
        let normalized_properties = utils::create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
            resource_type: "AWS::Batch::JobQueue".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: normalized_properties,
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: egui::Color32::PLACEHOLDER,
            region_color: egui::Color32::PLACEHOLDER,
            query_timestamp,
        };

        // Fetch tags (will be empty for resources that don't support tagging)
        entry.tags = aws_client
            .fetch_tags_for_resource(&entry.resource_type, &entry.resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for {} {}: {:?}",
                    entry.resource_type,
                    entry.resource_id,
                    e
                );
                Vec::new()
            });

        Ok(entry)
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Batch::JobQueue"
    }
}

#[async_trait]
impl AsyncResourceNormalizer for BatchComputeEnvironmentNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let resource_id = raw_response
            .get("JobQueueName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing JobQueueName"))?
            .to_string();

        let display_name = raw_response
            .get("JobQueueName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = utils::extract_status(&raw_response);
        let tags = utils::extract_tags(&raw_response);
        let normalized_properties = utils::create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
            resource_type: "AWS::Batch::JobQueue".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: normalized_properties,
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: egui::Color32::PLACEHOLDER,
            region_color: egui::Color32::PLACEHOLDER,
            query_timestamp,
        };

        // Fetch tags (will be empty for resources that don't support tagging)
        entry.tags = aws_client
            .fetch_tags_for_resource(&entry.resource_type, &entry.resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for {} {}: {:?}",
                    entry.resource_type,
                    entry.resource_id,
                    e
                );
                Vec::new()
            });

        Ok(entry)
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Batch::ComputeEnvironment"
    }
}
