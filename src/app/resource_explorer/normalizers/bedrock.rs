use super::{utils::*, ResourceNormalizer};
use crate::app::resource_explorer::state::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct BedrockModelNormalizer;

impl ResourceNormalizer for BedrockModelNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let model_id = raw_response
            .get("modelId")
            .or_else(|| raw_response.get("ModelId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-model")
            .to_string();

        let model_name = raw_response
            .get("modelName")
            .or_else(|| raw_response.get("ModelName"))
            .and_then(|v| v.as_str())
            .unwrap_or(&model_id)
            .to_string();

        let status = raw_response
            .get("modelStatus")
            .or_else(|| raw_response.get("Status"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let display_name = if model_name != model_id {
            model_name
        } else {
            extract_display_name(&raw_response, &model_id)
        };

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Bedrock::Model".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: model_id,
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
        // Bedrock models are standalone resources with no direct relationships
        // In the future, we might add relationships to knowledge bases or model customizations
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Bedrock::Model"
    }
}
