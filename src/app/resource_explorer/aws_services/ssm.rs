use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_ssm as ssm;
use std::sync::Arc;

pub struct SSMService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SSMService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List SSM parameters
    pub async fn list_parameters(
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

        let client = ssm::Client::new(&aws_config);
        let mut paginator = client.describe_parameters().into_paginator().send();

        let mut parameters = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(parameter_list) = page.parameters {
                for parameter in parameter_list {
                    let parameter_json = self.parameter_metadata_to_json(&parameter);
                    parameters.push(parameter_json);
                }
            }
        }

        Ok(parameters)
    }

    /// Get detailed information for specific SSM parameter
    pub async fn describe_parameter(
        &self,
        account_id: &str,
        region: &str,
        parameter_name: &str,
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

        let client = ssm::Client::new(&aws_config);
        let response = client
            .get_parameter()
            .name(parameter_name)
            .with_decryption(false) // Don't decrypt SecureString values for security
            .send()
            .await?;

        if let Some(parameter) = response.parameter {
            return Ok(self.parameter_to_json(&parameter));
        }

        Err(anyhow::anyhow!("Parameter {} not found", parameter_name))
    }

    /// List SSM documents
    pub async fn list_documents(
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

        let client = ssm::Client::new(&aws_config);
        let mut paginator = client.list_documents().into_paginator().send();

        let mut documents = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(document_identifiers) = page.document_identifiers {
                for document in document_identifiers {
                    let document_json = self.document_identifier_to_json(&document);
                    documents.push(document_json);
                }
            }
        }

        Ok(documents)
    }

    /// Get detailed information for specific SSM document
    pub async fn describe_document(
        &self,
        account_id: &str,
        region: &str,
        document_name: &str,
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

        let client = ssm::Client::new(&aws_config);
        let response = client
            .describe_document()
            .name(document_name)
            .send()
            .await?;

        if let Some(document) = response.document {
            return Ok(self.document_description_to_json(&document));
        }

        Err(anyhow::anyhow!("Document {} not found", document_name))
    }

    fn parameter_metadata_to_json(
        &self,
        parameter: &ssm::types::ParameterMetadata,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &parameter.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "ParameterName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(parameter_type) = &parameter.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(parameter_type.as_str().to_string()),
            );
        }

        if let Some(key_id) = &parameter.key_id {
            json.insert(
                "KeyId".to_string(),
                serde_json::Value::String(key_id.clone()),
            );
        }

        if let Some(last_modified_date) = parameter.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        if let Some(last_modified_user) = &parameter.last_modified_user {
            json.insert(
                "LastModifiedUser".to_string(),
                serde_json::Value::String(last_modified_user.clone()),
            );
        }

        if let Some(description) = &parameter.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(allowed_pattern) = &parameter.allowed_pattern {
            json.insert(
                "AllowedPattern".to_string(),
                serde_json::Value::String(allowed_pattern.clone()),
            );
        }

        let version = parameter.version;
        if version > 0 {
            json.insert(
                "Version".to_string(),
                serde_json::Value::Number(version.into()),
            );
        }

        if let Some(tier) = &parameter.tier {
            json.insert(
                "Tier".to_string(),
                serde_json::Value::String(tier.as_str().to_string()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn parameter_to_json(&self, parameter: &ssm::types::Parameter) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &parameter.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "ParameterName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(parameter_type) = &parameter.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(parameter_type.as_str().to_string()),
            );
        }

        if let Some(value) = &parameter.value {
            // For SecureString parameters, mask the value for security
            let display_value = if let Some(param_type) = &parameter.r#type {
                if param_type.as_str() == "SecureString" {
                    "[MASKED]".to_string()
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            };
            json.insert(
                "Value".to_string(),
                serde_json::Value::String(display_value),
            );
        }

        let version = parameter.version;
        if version > 0 {
            json.insert(
                "Version".to_string(),
                serde_json::Value::Number(version.into()),
            );
        }

        if let Some(selector) = &parameter.selector {
            json.insert(
                "Selector".to_string(),
                serde_json::Value::String(selector.clone()),
            );
        }

        if let Some(source_result) = &parameter.source_result {
            json.insert(
                "SourceResult".to_string(),
                serde_json::Value::String(source_result.clone()),
            );
        }

        if let Some(last_modified_date) = parameter.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        if let Some(arn) = &parameter.arn {
            json.insert("ARN".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(data_type) = &parameter.data_type {
            json.insert(
                "DataType".to_string(),
                serde_json::Value::String(data_type.clone()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn document_identifier_to_json(
        &self,
        document: &ssm::types::DocumentIdentifier,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &document.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "DocumentName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(owner) = &document.owner {
            json.insert(
                "Owner".to_string(),
                serde_json::Value::String(owner.clone()),
            );
        }

        if let Some(version_name) = &document.version_name {
            json.insert(
                "VersionName".to_string(),
                serde_json::Value::String(version_name.clone()),
            );
        }

        if let Some(platform_types) = &document.platform_types {
            let platforms: Vec<serde_json::Value> = platform_types
                .iter()
                .map(|platform| serde_json::Value::String(platform.as_str().to_string()))
                .collect();
            json.insert(
                "PlatformTypes".to_string(),
                serde_json::Value::Array(platforms),
            );
        }

        if let Some(document_version) = &document.document_version {
            json.insert(
                "DocumentVersion".to_string(),
                serde_json::Value::String(document_version.clone()),
            );
        }

        if let Some(document_type) = &document.document_type {
            json.insert(
                "DocumentType".to_string(),
                serde_json::Value::String(document_type.as_str().to_string()),
            );
        }

        if let Some(schema_version) = &document.schema_version {
            json.insert(
                "SchemaVersion".to_string(),
                serde_json::Value::String(schema_version.clone()),
            );
        }

        if let Some(document_format) = &document.document_format {
            json.insert(
                "DocumentFormat".to_string(),
                serde_json::Value::String(document_format.as_str().to_string()),
            );
        }

        if let Some(target_type) = &document.target_type {
            json.insert(
                "TargetType".to_string(),
                serde_json::Value::String(target_type.clone()),
            );
        }

        if let Some(created_date) = document.created_date {
            json.insert(
                "CreatedDate".to_string(),
                serde_json::Value::String(created_date.to_string()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn document_description_to_json(
        &self,
        document: &ssm::types::DocumentDescription,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(sha1) = &document.sha1 {
            json.insert("Sha1".to_string(), serde_json::Value::String(sha1.clone()));
        }

        if let Some(hash) = &document.hash {
            json.insert("Hash".to_string(), serde_json::Value::String(hash.clone()));
        }

        if let Some(hash_type) = &document.hash_type {
            json.insert(
                "HashType".to_string(),
                serde_json::Value::String(hash_type.as_str().to_string()),
            );
        }

        if let Some(name) = &document.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "DocumentName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(version_name) = &document.version_name {
            json.insert(
                "VersionName".to_string(),
                serde_json::Value::String(version_name.clone()),
            );
        }

        if let Some(owner) = &document.owner {
            json.insert(
                "Owner".to_string(),
                serde_json::Value::String(owner.clone()),
            );
        }

        if let Some(created_date) = document.created_date {
            json.insert(
                "CreatedDate".to_string(),
                serde_json::Value::String(created_date.to_string()),
            );
        }

        if let Some(status) = &document.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(status_information) = &document.status_information {
            json.insert(
                "StatusInformation".to_string(),
                serde_json::Value::String(status_information.clone()),
            );
        }

        if let Some(document_version) = &document.document_version {
            json.insert(
                "DocumentVersion".to_string(),
                serde_json::Value::String(document_version.clone()),
            );
        }

        if let Some(description) = &document.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(parameters) = &document.parameters {
            let params: Vec<serde_json::Value> = parameters
                .iter()
                .map(|param| {
                    let mut param_json = serde_json::Map::new();
                    if let Some(name) = &param.name {
                        param_json
                            .insert("Name".to_string(), serde_json::Value::String(name.clone()));
                    }
                    if let Some(param_type) = &param.r#type {
                        param_json.insert(
                            "Type".to_string(),
                            serde_json::Value::String(param_type.as_str().to_string()),
                        );
                    }
                    if let Some(description) = &param.description {
                        param_json.insert(
                            "Description".to_string(),
                            serde_json::Value::String(description.clone()),
                        );
                    }
                    serde_json::Value::Object(param_json)
                })
                .collect();
            json.insert("Parameters".to_string(), serde_json::Value::Array(params));
        }

        if let Some(platform_types) = &document.platform_types {
            let platforms: Vec<serde_json::Value> = platform_types
                .iter()
                .map(|platform| serde_json::Value::String(platform.as_str().to_string()))
                .collect();
            json.insert(
                "PlatformTypes".to_string(),
                serde_json::Value::Array(platforms),
            );
        }

        if let Some(document_type) = &document.document_type {
            json.insert(
                "DocumentType".to_string(),
                serde_json::Value::String(document_type.as_str().to_string()),
            );
        }

        if let Some(schema_version) = &document.schema_version {
            json.insert(
                "SchemaVersion".to_string(),
                serde_json::Value::String(schema_version.clone()),
            );
        }

        if let Some(latest_version) = &document.latest_version {
            json.insert(
                "LatestVersion".to_string(),
                serde_json::Value::String(latest_version.clone()),
            );
        }

        if let Some(default_version) = &document.default_version {
            json.insert(
                "DefaultVersion".to_string(),
                serde_json::Value::String(default_version.clone()),
            );
        }

        if let Some(document_format) = &document.document_format {
            json.insert(
                "DocumentFormat".to_string(),
                serde_json::Value::String(document_format.as_str().to_string()),
            );
        }

        if let Some(target_type) = &document.target_type {
            json.insert(
                "TargetType".to_string(),
                serde_json::Value::String(target_type.clone()),
            );
        }

        serde_json::Value::Object(json)
    }
}
