use anyhow::{Result, Context};
use aws_sdk_bedrock as bedrock;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct BedrockService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl BedrockService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Bedrock foundation models (basic list data)
    pub async fn list_foundation_models(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = bedrock::Client::new(&aws_config);
        let response = client
            .list_foundation_models()
            .send()
            .await?;

        let mut models = Vec::new();
        if let Some(model_summaries) = response.model_summaries {
            for model in model_summaries {
                let model_json = self.foundation_model_summary_to_json(&model);
                models.push(model_json);
            }
        }

        Ok(models)
    }

    /// Get detailed information for a specific Bedrock foundation model
    pub async fn describe_foundation_model(
        &self,
        account_id: &str,
        region: &str,
        model_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = bedrock::Client::new(&aws_config);
        let response = client
            .get_foundation_model()
            .model_identifier(model_id)
            .send()
            .await?;

        if let Some(model_details) = response.model_details {
            Ok(self.foundation_model_details_to_json(&model_details))
        } else {
            Err(anyhow::anyhow!("Foundation model {} not found", model_id))
        }
    }

    // JSON conversion methods
    fn foundation_model_summary_to_json(&self, model: &bedrock::types::FoundationModelSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert("modelId".to_string(), serde_json::Value::String(model.model_id.clone()));

        if let Some(name) = &model.model_name {
            json.insert("modelName".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(provider_name) = &model.provider_name {
            json.insert("providerName".to_string(), serde_json::Value::String(provider_name.clone()));
        }

        json.insert("modelArn".to_string(), serde_json::Value::String(model.model_arn.clone()));

        // Add input modalities
        if let Some(ref input_modalities) = model.input_modalities {
            if !input_modalities.is_empty() {
            let modalities: Vec<serde_json::Value> = input_modalities
                .iter()
                .map(|m| serde_json::Value::String(format!("{:?}", m)))
                .collect();
            json.insert("inputModalities".to_string(), serde_json::Value::Array(modalities));
        }
        }

        // Add output modalities
        if let Some(ref output_modalities) = model.output_modalities {
            if !output_modalities.is_empty() {
            let modalities: Vec<serde_json::Value> = output_modalities
                .iter()
                .map(|m| serde_json::Value::String(format!("{:?}", m)))
                .collect();
            json.insert("outputModalities".to_string(), serde_json::Value::Array(modalities));
        }

        // Add response streaming supported
        if let Some(streaming_supported) = &model.response_streaming_supported {
            json.insert("responseStreamingSupported".to_string(), serde_json::Value::Bool(*streaming_supported));
        }

        // Add customizations supported
        if let Some(ref customizations) = model.customizations_supported {
            if !customizations.is_empty() {
            let customizations: Vec<serde_json::Value> = customizations
                .iter()
                .map(|c| serde_json::Value::String(format!("{:?}", c)))
                .collect();
            json.insert("customizationsSupported".to_string(), serde_json::Value::Array(customizations));
        }

        // Add inference types supported
        if let Some(ref inference_types) = model.inference_types_supported {
            if !inference_types.is_empty() {
            let inference_types: Vec<serde_json::Value> = inference_types
                .iter()
                .map(|i| serde_json::Value::String(format!("{:?}", i)))
                .collect();
            json.insert("inferenceTypesSupported".to_string(), serde_json::Value::Array(inference_types));
        }

        serde_json::Value::Object(json)
    }

    fn foundation_model_details_to_json(&self, model: &bedrock::types::FoundationModelDetails) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert("modelId".to_string(), serde_json::Value::String(model.model_id.clone()));

        if let Some(name) = &model.model_name {
            json.insert("modelName".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(provider_name) = &model.provider_name {
            json.insert("providerName".to_string(), serde_json::Value::String(provider_name.clone()));
        }

        json.insert("modelArn".to_string(), serde_json::Value::String(model.model_arn.clone()));

        // Add input modalities
        if let Some(ref input_modalities) = model.input_modalities {
            if !input_modalities.is_empty() {
            let modalities: Vec<serde_json::Value> = input_modalities
                .iter()
                .map(|m| serde_json::Value::String(format!("{:?}", m)))
                .collect();
            json.insert("inputModalities".to_string(), serde_json::Value::Array(modalities));
        }
        }

        // Add output modalities
        if let Some(ref output_modalities) = model.output_modalities {
            if !output_modalities.is_empty() {
            let modalities: Vec<serde_json::Value> = output_modalities
                .iter()
                .map(|m| serde_json::Value::String(format!("{:?}", m)))
                .collect();
            json.insert("outputModalities".to_string(), serde_json::Value::Array(modalities));
        }

        // Add response streaming supported
        if let Some(streaming_supported) = &model.response_streaming_supported {
            json.insert("responseStreamingSupported".to_string(), serde_json::Value::Bool(*streaming_supported));
        }

        // Add customizations supported
        if let Some(ref customizations) = model.customizations_supported {
            if !customizations.is_empty() {
            let customizations: Vec<serde_json::Value> = customizations
                .iter()
                .map(|c| serde_json::Value::String(format!("{:?}", c)))
                .collect();
            json.insert("customizationsSupported".to_string(), serde_json::Value::Array(customizations));
        }

        // Add inference types supported
        if let Some(ref inference_types) = model.inference_types_supported {
            if !inference_types.is_empty() {
            let inference_types: Vec<serde_json::Value> = inference_types
                .iter()
                .map(|i| serde_json::Value::String(format!("{:?}", i)))
                .collect();
            json.insert("inferenceTypesSupported".to_string(), serde_json::Value::Array(inference_types));
        }

        serde_json::Value::Object(json)
    }
}