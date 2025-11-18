use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_bedrock as bedrock;
use std::sync::Arc;

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
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client.list_foundation_models().send().await?;

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
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

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
    fn foundation_model_summary_to_json(
        &self,
        model: &bedrock::types::FoundationModelSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "modelId".to_string(),
            serde_json::Value::String(model.model_id.clone()),
        );

        if let Some(name) = &model.model_name {
            json.insert(
                "modelName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(provider_name) = &model.provider_name {
            json.insert(
                "providerName".to_string(),
                serde_json::Value::String(provider_name.clone()),
            );
        }

        json.insert(
            "modelArn".to_string(),
            serde_json::Value::String(model.model_arn.clone()),
        );

        // Add input modalities
        if let Some(ref input_modalities) = model.input_modalities {
            if !input_modalities.is_empty() {
                let modalities: Vec<serde_json::Value> = input_modalities
                    .iter()
                    .map(|m| serde_json::Value::String(format!("{:?}", m)))
                    .collect();
                json.insert(
                    "inputModalities".to_string(),
                    serde_json::Value::Array(modalities),
                );
            }
        }

        // Add output modalities
        if let Some(ref output_modalities) = model.output_modalities {
            if !output_modalities.is_empty() {
                let modalities: Vec<serde_json::Value> = output_modalities
                    .iter()
                    .map(|m| serde_json::Value::String(format!("{:?}", m)))
                    .collect();
                json.insert(
                    "outputModalities".to_string(),
                    serde_json::Value::Array(modalities),
                );
            }
        }

        // Add response streaming supported
        if let Some(streaming_supported) = &model.response_streaming_supported {
            json.insert(
                "responseStreamingSupported".to_string(),
                serde_json::Value::Bool(*streaming_supported),
            );
        }

        // Add customizations supported
        if let Some(ref customizations) = model.customizations_supported {
            if !customizations.is_empty() {
                let customizations: Vec<serde_json::Value> = customizations
                    .iter()
                    .map(|c| serde_json::Value::String(format!("{:?}", c)))
                    .collect();
                json.insert(
                    "customizationsSupported".to_string(),
                    serde_json::Value::Array(customizations),
                );
            }
        }

        // Add inference types supported
        if let Some(ref inference_types) = model.inference_types_supported {
            if !inference_types.is_empty() {
                let inference_types: Vec<serde_json::Value> = inference_types
                    .iter()
                    .map(|i| serde_json::Value::String(format!("{:?}", i)))
                    .collect();
                json.insert(
                    "inferenceTypesSupported".to_string(),
                    serde_json::Value::Array(inference_types),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    fn foundation_model_details_to_json(
        &self,
        model: &bedrock::types::FoundationModelDetails,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "modelId".to_string(),
            serde_json::Value::String(model.model_id.clone()),
        );

        if let Some(name) = &model.model_name {
            json.insert(
                "modelName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(provider_name) = &model.provider_name {
            json.insert(
                "providerName".to_string(),
                serde_json::Value::String(provider_name.clone()),
            );
        }

        json.insert(
            "modelArn".to_string(),
            serde_json::Value::String(model.model_arn.clone()),
        );

        // Add input modalities
        if let Some(ref input_modalities) = model.input_modalities {
            if !input_modalities.is_empty() {
                let modalities: Vec<serde_json::Value> = input_modalities
                    .iter()
                    .map(|m| serde_json::Value::String(format!("{:?}", m)))
                    .collect();
                json.insert(
                    "inputModalities".to_string(),
                    serde_json::Value::Array(modalities),
                );
            }
        }

        // Add output modalities
        if let Some(ref output_modalities) = model.output_modalities {
            if !output_modalities.is_empty() {
                let modalities: Vec<serde_json::Value> = output_modalities
                    .iter()
                    .map(|m| serde_json::Value::String(format!("{:?}", m)))
                    .collect();
                json.insert(
                    "outputModalities".to_string(),
                    serde_json::Value::Array(modalities),
                );
            }
        }

        // Add response streaming supported
        if let Some(streaming_supported) = &model.response_streaming_supported {
            json.insert(
                "responseStreamingSupported".to_string(),
                serde_json::Value::Bool(*streaming_supported),
            );
        }

        // Add customizations supported
        if let Some(ref customizations) = model.customizations_supported {
            if !customizations.is_empty() {
                let customizations: Vec<serde_json::Value> = customizations
                    .iter()
                    .map(|c| serde_json::Value::String(format!("{:?}", c)))
                    .collect();
                json.insert(
                    "customizationsSupported".to_string(),
                    serde_json::Value::Array(customizations),
                );
            }
        }

        // Add inference types supported
        if let Some(ref inference_types) = model.inference_types_supported {
            if !inference_types.is_empty() {
                let inference_types: Vec<serde_json::Value> = inference_types
                    .iter()
                    .map(|i| serde_json::Value::String(format!("{:?}", i)))
                    .collect();
                json.insert(
                    "inferenceTypesSupported".to_string(),
                    serde_json::Value::Array(inference_types),
                );
            }
        }

        serde_json::Value::Object(json)
    }

    /// List Bedrock inference profiles
    pub async fn list_inference_profiles(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);

        let mut profiles = Vec::new();
        let mut paginator = client.list_inference_profiles().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(profile_summaries) = page.inference_profile_summaries {
                for profile in profile_summaries {
                    profiles.push(self.inference_profile_to_json(&profile));
                }
            }
        }

        Ok(profiles)
    }

    /// Get detailed information for a specific inference profile
    pub async fn describe_inference_profile(
        &self,
        account_id: &str,
        region: &str,
        profile_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client
            .get_inference_profile()
            .inference_profile_identifier(profile_id)
            .send()
            .await?;

        Ok(self.inference_profile_details_to_json(&response))
    }

    fn inference_profile_to_json(
        &self,
        profile: &bedrock::types::InferenceProfileSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "InferenceProfileId".to_string(),
            serde_json::Value::String(profile.inference_profile_id.clone()),
        );

        json.insert(
            "InferenceProfileName".to_string(),
            serde_json::Value::String(profile.inference_profile_name.clone()),
        );

        json.insert(
            "InferenceProfileArn".to_string(),
            serde_json::Value::String(profile.inference_profile_arn.clone()),
        );

        if let Some(description) = &profile.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Type".to_string(),
            serde_json::Value::String(profile.r#type.as_str().to_string()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(profile.status.as_str().to_string()),
        );

        if let Some(created_at) = &profile.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.to_string()),
            );
        }

        if let Some(updated_at) = &profile.updated_at {
            json.insert(
                "UpdatedAt".to_string(),
                serde_json::Value::String(updated_at.to_string()),
            );
        }

        if !profile.models.is_empty() {
            let models_json: Vec<serde_json::Value> = profile
                .models
                .iter()
                .map(|m| {
                    let mut model_json = serde_json::Map::new();
                    if let Some(model_arn) = &m.model_arn {
                        model_json.insert(
                            "ModelArn".to_string(),
                            serde_json::Value::String(model_arn.clone()),
                        );
                    }
                    serde_json::Value::Object(model_json)
                })
                .collect();
            json.insert("Models".to_string(), serde_json::Value::Array(models_json));
        }

        serde_json::Value::Object(json)
    }

    fn inference_profile_details_to_json(
        &self,
        response: &bedrock::operation::get_inference_profile::GetInferenceProfileOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "InferenceProfileId".to_string(),
            serde_json::Value::String(response.inference_profile_id.clone()),
        );

        json.insert(
            "InferenceProfileName".to_string(),
            serde_json::Value::String(response.inference_profile_name.clone()),
        );

        json.insert(
            "InferenceProfileArn".to_string(),
            serde_json::Value::String(response.inference_profile_arn.clone()),
        );

        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Type".to_string(),
            serde_json::Value::String(response.r#type.as_str().to_string()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        if let Some(created_at) = &response.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.to_string()),
            );
        }

        if let Some(updated_at) = &response.updated_at {
            json.insert(
                "UpdatedAt".to_string(),
                serde_json::Value::String(updated_at.to_string()),
            );
        }

        if !response.models.is_empty() {
            let models_json: Vec<serde_json::Value> = response
                .models
                .iter()
                .map(|m| {
                    let mut model_json = serde_json::Map::new();
                    if let Some(model_arn) = &m.model_arn {
                        model_json.insert(
                            "ModelArn".to_string(),
                            serde_json::Value::String(model_arn.clone()),
                        );
                    }
                    serde_json::Value::Object(model_json)
                })
                .collect();
            json.insert("Models".to_string(), serde_json::Value::Array(models_json));
        }

        serde_json::Value::Object(json)
    }

    /// List Bedrock guardrails
    pub async fn list_guardrails(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);

        let mut guardrails = Vec::new();
        let mut paginator = client.list_guardrails().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            for guardrail in page.guardrails {
                guardrails.push(self.guardrail_to_json(&guardrail));
            }
        }

        Ok(guardrails)
    }

    /// Get detailed information for a specific guardrail
    pub async fn describe_guardrail(
        &self,
        account_id: &str,
        region: &str,
        guardrail_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client
            .get_guardrail()
            .guardrail_identifier(guardrail_id)
            .send()
            .await?;

        Ok(self.guardrail_details_to_json(&response))
    }

    fn guardrail_to_json(
        &self,
        guardrail: &bedrock::types::GuardrailSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Id".to_string(),
            serde_json::Value::String(guardrail.id.clone()),
        );
        json.insert(
            "Arn".to_string(),
            serde_json::Value::String(guardrail.arn.clone()),
        );

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(guardrail.name.clone()),
        );

        if let Some(description) = &guardrail.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(guardrail.status.as_str().to_string()),
        );

        json.insert(
            "Version".to_string(),
            serde_json::Value::String(guardrail.version.clone()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(guardrail.created_at.to_string()),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(guardrail.updated_at.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn guardrail_details_to_json(
        &self,
        response: &bedrock::operation::get_guardrail::GetGuardrailOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "GuardrailId".to_string(),
            serde_json::Value::String(response.guardrail_id.clone()),
        );
        json.insert(
            "GuardrailArn".to_string(),
            serde_json::Value::String(response.guardrail_arn.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );
        json.insert(
            "Version".to_string(),
            serde_json::Value::String(response.version.clone()),
        );

        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(response.created_at.to_string()),
        );

        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(response.updated_at.to_string()),
        );

        json.insert(
            "BlockedInputMessaging".to_string(),
            serde_json::Value::String(response.blocked_input_messaging.clone()),
        );
        json.insert(
            "BlockedOutputsMessaging".to_string(),
            serde_json::Value::String(response.blocked_outputs_messaging.clone()),
        );

        serde_json::Value::Object(json)
    }

    /// List provisioned model throughput
    pub async fn list_provisioned_model_throughputs(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);

        let mut throughputs = Vec::new();
        let mut paginator = client
            .list_provisioned_model_throughputs()
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(throughput_summaries) = page.provisioned_model_summaries {
                for throughput in throughput_summaries {
                    throughputs.push(self.provisioned_throughput_to_json(&throughput));
                }
            }
        }

        Ok(throughputs)
    }

    /// Get detailed information for a specific provisioned model throughput
    pub async fn describe_provisioned_model_throughput(
        &self,
        account_id: &str,
        region: &str,
        throughput_arn: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client
            .get_provisioned_model_throughput()
            .provisioned_model_id(throughput_arn)
            .send()
            .await?;

        Ok(self.provisioned_throughput_details_to_json(&response))
    }

    fn provisioned_throughput_to_json(
        &self,
        throughput: &bedrock::types::ProvisionedModelSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "ProvisionedModelArn".to_string(),
            serde_json::Value::String(throughput.provisioned_model_arn.clone()),
        );
        json.insert(
            "ProvisionedModelName".to_string(),
            serde_json::Value::String(throughput.provisioned_model_name.clone()),
        );
        json.insert(
            "ModelArn".to_string(),
            serde_json::Value::String(throughput.model_arn.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(throughput.status.as_str().to_string()),
        );

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(throughput.creation_time.to_string()),
        );
        json.insert(
            "LastModifiedTime".to_string(),
            serde_json::Value::String(throughput.last_modified_time.to_string()),
        );

        json.insert(
            "ModelUnits".to_string(),
            serde_json::Value::Number(throughput.model_units.into()),
        );

        if let Some(commitment_duration) = &throughput.commitment_duration {
            json.insert(
                "CommitmentDuration".to_string(),
                serde_json::Value::String(commitment_duration.as_str().to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn provisioned_throughput_details_to_json(
        &self,
        response: &bedrock::operation::get_provisioned_model_throughput::GetProvisionedModelThroughputOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "ProvisionedModelArn".to_string(),
            serde_json::Value::String(response.provisioned_model_arn.clone()),
        );
        json.insert(
            "ProvisionedModelName".to_string(),
            serde_json::Value::String(response.provisioned_model_name.clone()),
        );
        json.insert(
            "ModelArn".to_string(),
            serde_json::Value::String(response.model_arn.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(response.creation_time.to_string()),
        );
        json.insert(
            "LastModifiedTime".to_string(),
            serde_json::Value::String(response.last_modified_time.to_string()),
        );

        json.insert(
            "ModelUnits".to_string(),
            serde_json::Value::Number(response.model_units.into()),
        );

        json.insert(
            "DesiredModelUnits".to_string(),
            serde_json::Value::Number(response.desired_model_units.into()),
        );

        json.insert(
            "FoundationModelArn".to_string(),
            serde_json::Value::String(response.foundation_model_arn.clone()),
        );

        if let Some(commitment_duration) = &response.commitment_duration {
            json.insert(
                "CommitmentDuration".to_string(),
                serde_json::Value::String(commitment_duration.as_str().to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    /// List custom models
    pub async fn list_custom_models(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);

        let mut models = Vec::new();
        let mut paginator = client.list_custom_models().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(model_summaries) = page.model_summaries {
                for model in model_summaries {
                    models.push(self.custom_model_to_json(&model));
                }
            }
        }

        Ok(models)
    }

    /// Get detailed information for a specific custom model
    pub async fn describe_custom_model(
        &self,
        account_id: &str,
        region: &str,
        model_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client.get_custom_model().model_identifier(model_id).send().await?;

        Ok(self.custom_model_details_to_json(&response))
    }

    fn custom_model_to_json(&self, model: &bedrock::types::CustomModelSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "ModelArn".to_string(),
            serde_json::Value::String(model.model_arn.clone()),
        );
        json.insert(
            "ModelName".to_string(),
            serde_json::Value::String(model.model_name.clone()),
        );

        json.insert(
            "BaseModelArn".to_string(),
            serde_json::Value::String(model.base_model_arn.clone()),
        );

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(model.creation_time.to_string()),
        );

        if let Some(customization_type) = &model.customization_type {
            json.insert(
                "CustomizationType".to_string(),
                serde_json::Value::String(customization_type.as_str().to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn custom_model_details_to_json(
        &self,
        response: &bedrock::operation::get_custom_model::GetCustomModelOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "ModelArn".to_string(),
            serde_json::Value::String(response.model_arn.clone()),
        );
        json.insert(
            "ModelName".to_string(),
            serde_json::Value::String(response.model_name.clone()),
        );

        if let Some(base_model_arn) = &response.base_model_arn {
            json.insert(
                "BaseModelArn".to_string(),
                serde_json::Value::String(base_model_arn.clone()),
            );
        }

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(response.creation_time.to_string()),
        );

        if let Some(customization_type) = &response.customization_type {
            json.insert(
                "CustomizationType".to_string(),
                serde_json::Value::String(customization_type.as_str().to_string()),
            );
        }

        if let Some(job_arn) = &response.job_arn {
            json.insert(
                "JobArn".to_string(),
                serde_json::Value::String(job_arn.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    /// List imported models
    pub async fn list_imported_models(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);

        let mut models = Vec::new();
        let mut paginator = client.list_imported_models().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(model_summaries) = page.model_summaries {
                for model in model_summaries {
                    models.push(self.imported_model_to_json(&model));
                }
            }
        }

        Ok(models)
    }

    /// Get detailed information for a specific imported model
    pub async fn describe_imported_model(
        &self,
        account_id: &str,
        region: &str,
        model_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client.get_imported_model().model_identifier(model_id).send().await?;

        Ok(self.imported_model_details_to_json(&response))
    }

    fn imported_model_to_json(&self, model: &bedrock::types::ImportedModelSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "ModelArn".to_string(),
            serde_json::Value::String(model.model_arn.clone()),
        );
        json.insert(
            "ModelName".to_string(),
            serde_json::Value::String(model.model_name.clone()),
        );

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(model.creation_time.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn imported_model_details_to_json(
        &self,
        response: &bedrock::operation::get_imported_model::GetImportedModelOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(model_arn) = &response.model_arn {
            json.insert(
                "ModelArn".to_string(),
                serde_json::Value::String(model_arn.clone()),
            );
        }
        if let Some(model_name) = &response.model_name {
            json.insert(
                "ModelName".to_string(),
                serde_json::Value::String(model_name.clone()),
            );
        }

        if let Some(creation_time) = &response.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(job_arn) = &response.job_arn {
            json.insert(
                "JobArn".to_string(),
                serde_json::Value::String(job_arn.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    /// List evaluation jobs
    pub async fn list_evaluation_jobs(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);

        let mut jobs = Vec::new();
        let mut paginator = client.list_evaluation_jobs().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(job_summaries) = page.job_summaries {
                for job in job_summaries {
                    jobs.push(self.evaluation_job_to_json(&job));
                }
            }
        }

        Ok(jobs)
    }

    /// Get detailed information for a specific evaluation job
    pub async fn describe_evaluation_job(
        &self,
        account_id: &str,
        region: &str,
        job_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client.get_evaluation_job().job_identifier(job_id).send().await?;

        Ok(self.evaluation_job_details_to_json(&response))
    }

    fn evaluation_job_to_json(&self, job: &bedrock::types::EvaluationSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "JobArn".to_string(),
            serde_json::Value::String(job.job_arn.clone()),
        );
        json.insert(
            "JobName".to_string(),
            serde_json::Value::String(job.job_name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(job.status.as_str().to_string()),
        );

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(job.creation_time.to_string()),
        );

        json.insert(
            "JobType".to_string(),
            serde_json::Value::String(job.job_type.as_str().to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn evaluation_job_details_to_json(
        &self,
        response: &bedrock::operation::get_evaluation_job::GetEvaluationJobOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "JobArn".to_string(),
            serde_json::Value::String(response.job_arn.clone()),
        );
        json.insert(
            "JobName".to_string(),
            serde_json::Value::String(response.job_name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(response.status.as_str().to_string()),
        );

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(response.creation_time.to_string()),
        );

        json.insert(
            "JobType".to_string(),
            serde_json::Value::String(response.job_type.as_str().to_string()),
        );

        json.insert(
            "RoleArn".to_string(),
            serde_json::Value::String(response.role_arn.clone()),
        );

        serde_json::Value::Object(json)
    }

    /// List model invocation jobs
    pub async fn list_model_invocation_jobs(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);

        let mut jobs = Vec::new();
        let mut paginator = client.list_model_invocation_jobs().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(invocation_job_summaries) = page.invocation_job_summaries {
                for job in invocation_job_summaries {
                    jobs.push(self.model_invocation_job_to_json(&job));
                }
            }
        }

        Ok(jobs)
    }

    /// Get detailed information for a specific model invocation job
    pub async fn describe_model_invocation_job(
        &self,
        account_id: &str,
        region: &str,
        job_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client.get_model_invocation_job().job_identifier(job_id).send().await?;

        Ok(self.model_invocation_job_details_to_json(&response))
    }

    fn model_invocation_job_to_json(&self, job: &bedrock::types::ModelInvocationJobSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "JobArn".to_string(),
            serde_json::Value::String(job.job_arn.clone()),
        );
        json.insert(
            "JobName".to_string(),
            serde_json::Value::String(job.job_name.clone()),
        );

        json.insert(
            "ModelId".to_string(),
            serde_json::Value::String(job.model_id.clone()),
        );

        if let Some(status) = &job.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        json.insert(
            "SubmitTime".to_string(),
            serde_json::Value::String(job.submit_time.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn model_invocation_job_details_to_json(
        &self,
        response: &bedrock::operation::get_model_invocation_job::GetModelInvocationJobOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "JobArn".to_string(),
            serde_json::Value::String(response.job_arn.clone()),
        );
        if let Some(job_name) = &response.job_name {
            json.insert(
                "JobName".to_string(),
                serde_json::Value::String(job_name.clone()),
            );
        }

        json.insert(
            "ModelId".to_string(),
            serde_json::Value::String(response.model_id.clone()),
        );

        if let Some(status) = &response.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        json.insert(
            "RoleArn".to_string(),
            serde_json::Value::String(response.role_arn.clone()),
        );

        json.insert(
            "SubmitTime".to_string(),
            serde_json::Value::String(response.submit_time.to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// List model customization jobs
    pub async fn list_model_customization_jobs(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);

        let mut jobs = Vec::new();
        let mut paginator = client.list_model_customization_jobs().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(job_summaries) = page.model_customization_job_summaries {
                for job in job_summaries {
                    jobs.push(self.model_customization_job_to_json(&job));
                }
            }
        }

        Ok(jobs)
    }

    /// Get detailed information for a specific model customization job
    pub async fn describe_model_customization_job(
        &self,
        account_id: &str,
        region: &str,
        job_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = bedrock::Client::new(&aws_config);
        let response = client
            .get_model_customization_job()
            .job_identifier(job_id)
            .send()
            .await?;

        Ok(self.model_customization_job_details_to_json(&response))
    }

    fn model_customization_job_to_json(
        &self,
        job: &bedrock::types::ModelCustomizationJobSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "JobArn".to_string(),
            serde_json::Value::String(job.job_arn.clone()),
        );
        json.insert(
            "JobName".to_string(),
            serde_json::Value::String(job.job_name.clone()),
        );

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(job.status.as_str().to_string()),
        );

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(job.creation_time.to_string()),
        );

        if let Some(end_time) = &job.end_time {
            json.insert(
                "EndTime".to_string(),
                serde_json::Value::String(end_time.to_string()),
            );
        }

        if let Some(custom_model_name) = &job.custom_model_name {
            json.insert(
                "CustomModelName".to_string(),
                serde_json::Value::String(custom_model_name.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn model_customization_job_details_to_json(
        &self,
        response: &bedrock::operation::get_model_customization_job::GetModelCustomizationJobOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "JobArn".to_string(),
            serde_json::Value::String(response.job_arn.clone()),
        );
        json.insert(
            "JobName".to_string(),
            serde_json::Value::String(response.job_name.clone()),
        );

        if let Some(status) = &response.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(response.creation_time.to_string()),
        );

        if let Some(end_time) = &response.end_time {
            json.insert(
                "EndTime".to_string(),
                serde_json::Value::String(end_time.to_string()),
            );
        }

        if let Some(custom_model_arn) = &response.output_model_arn {
            json.insert(
                "OutputModelArn".to_string(),
                serde_json::Value::String(custom_model_arn.clone()),
            );
        }

        json.insert(
            "BaseModelArn".to_string(),
            serde_json::Value::String(response.base_model_arn.clone()),
        );

        json.insert(
            "RoleArn".to_string(),
            serde_json::Value::String(response.role_arn.clone()),
        );

        serde_json::Value::Object(json)
    }
}
