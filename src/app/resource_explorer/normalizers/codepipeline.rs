use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for CodePipeline Pipelines
pub struct CodePipelinePipelineNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CodePipelinePipelineNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let pipeline_name = raw_response
            .get("PipelineName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-pipeline")
            .to_string();

        let display_name = extract_display_name(&raw_response, &pipeline_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);


        let mut entry = ResourceEntry {
            resource_type: "AWS::CodePipeline::Pipeline".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: pipeline_name,
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
        };

        // Fetch tags (will be empty for resources that don't support tagging)
        entry.tags = aws_client
            .fetch_tags_for_resource(&entry.resource_type, &entry.resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch tags for {} {}: {:?}", entry.resource_type, entry.resource_id, e);
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
        "AWS::CodePipeline::Pipeline"
    }
}
