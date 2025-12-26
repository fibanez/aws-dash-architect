use super::{utils::*, AWSResourceClient, AsyncResourceNormalizer};
use crate::app::resource_explorer::state::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub struct BedrockModelNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockModelNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
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

        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Bedrock::Model", &model_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Bedrock::Model {}: {}",
                    model_id,
                    e
                );

                Vec::new()
            });
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

/// Normalizer for Bedrock Inference Profiles
pub struct BedrockInferenceProfileNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockInferenceProfileNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::InferenceProfile"
    }
}

/// Normalizer for Bedrock Guardrails
pub struct BedrockGuardrailNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockGuardrailNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::Guardrail"
    }
}

/// Normalizer for Bedrock Provisioned Model Throughput
pub struct BedrockProvisionedModelThroughputNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockProvisionedModelThroughputNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::ProvisionedModelThroughput"
    }
}

/// Normalizer for Bedrock Agents
pub struct BedrockAgentNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::Agent"
    }
}

/// Normalizer for Bedrock Knowledge Bases
pub struct BedrockKnowledgeBaseNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockKnowledgeBaseNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::KnowledgeBase"
    }
}

/// Normalizer for Bedrock Custom Models
pub struct BedrockCustomModelNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockCustomModelNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::CustomModel"
    }
}

/// Normalizer for Bedrock Imported Models
pub struct BedrockImportedModelNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockImportedModelNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::ImportedModel"
    }
}

/// Normalizer for Bedrock Evaluation Jobs
pub struct BedrockEvaluationJobNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockEvaluationJobNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::EvaluationJob"
    }
}

/// Normalizer for Bedrock Model Invocation Jobs
pub struct BedrockModelInvocationJobNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockModelInvocationJobNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::ModelInvocationJob"
    }
}

/// Normalizer for Bedrock Prompts
pub struct BedrockPromptNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockPromptNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::Prompt"
    }
}

/// Normalizer for Bedrock Flows
pub struct BedrockFlowNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockFlowNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::Flow"
    }
}

/// Normalizer for Bedrock Agent Aliases
pub struct BedrockAgentAliasNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentAliasNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::AgentAlias"
    }
}

/// Normalizer for Bedrock Agent Action Groups
pub struct BedrockAgentActionGroupNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockAgentActionGroupNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::AgentActionGroup"
    }
}

/// Normalizer for Bedrock Data Sources
pub struct BedrockDataSourceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockDataSourceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::DataSource"
    }
}

/// Normalizer for Bedrock Model Customization Jobs
pub struct BedrockModelCustomizationJobNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockModelCustomizationJobNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::ModelCustomizationJob"
    }
}

/// Normalizer for Bedrock Ingestion Jobs
pub struct BedrockIngestionJobNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockIngestionJobNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::IngestionJob"
    }
}

/// Normalizer for Bedrock Flow Aliases
pub struct BedrockFlowAliasNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for BedrockFlowAliasNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
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
        "AWS::Bedrock::FlowAlias"
    }
}
