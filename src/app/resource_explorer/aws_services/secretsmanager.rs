use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_secretsmanager as secretsmanager;
use std::sync::Arc;

pub struct SecretsManagerService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SecretsManagerService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Secrets Manager Secrets
    pub async fn list_secrets(
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

        let client = secretsmanager::Client::new(&aws_config);
        let mut paginator = client.list_secrets().into_paginator().send();

        let mut secrets = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(secret_list) = page.secret_list {
                for secret in secret_list {
                    let secret_json = self.secret_to_json(&secret);
                    secrets.push(secret_json);
                }
            }
        }

        Ok(secrets)
    }

    /// Get detailed information for specific secret
    pub async fn describe_secret(
        &self,
        account_id: &str,
        region: &str,
        secret_id: &str,
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

        let client = secretsmanager::Client::new(&aws_config);
        let response = client.describe_secret().secret_id(secret_id).send().await?;

        Ok(self.secret_description_to_json(&response))
    }

    fn secret_to_json(&self, secret: &secretsmanager::types::SecretListEntry) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(arn) = &secret.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(name) = &secret.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &secret.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(kms_key_id) = &secret.kms_key_id {
            json.insert(
                "KmsKeyId".to_string(),
                serde_json::Value::String(kms_key_id.clone()),
            );
        }

        json.insert(
            "RotationEnabled".to_string(),
            serde_json::Value::Bool(secret.rotation_enabled.unwrap_or(false)),
        );

        if let Some(rotation_lambda_arn) = &secret.rotation_lambda_arn {
            json.insert(
                "RotationLambdaArn".to_string(),
                serde_json::Value::String(rotation_lambda_arn.clone()),
            );
        }

        if let Some(rotation_rules) = &secret.rotation_rules {
            let mut rotation_json = serde_json::Map::new();
            if let Some(automatically_after_days) = rotation_rules.automatically_after_days {
                rotation_json.insert(
                    "AutomaticallyAfterDays".to_string(),
                    serde_json::Value::Number(automatically_after_days.into()),
                );
            }
            json.insert(
                "RotationRules".to_string(),
                serde_json::Value::Object(rotation_json),
            );
        }

        if let Some(last_rotated_date) = secret.last_rotated_date {
            json.insert(
                "LastRotatedDate".to_string(),
                serde_json::Value::String(last_rotated_date.to_string()),
            );
        }

        if let Some(last_changed_date) = secret.last_changed_date {
            json.insert(
                "LastChangedDate".to_string(),
                serde_json::Value::String(last_changed_date.to_string()),
            );
        }

        if let Some(last_accessed_date) = secret.last_accessed_date {
            json.insert(
                "LastAccessedDate".to_string(),
                serde_json::Value::String(last_accessed_date.to_string()),
            );
        }

        if let Some(deleted_date) = secret.deleted_date {
            json.insert(
                "DeletedDate".to_string(),
                serde_json::Value::String(deleted_date.to_string()),
            );
        }

        if let Some(tags) = &secret.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
                        }
                        if let Some(value) = &tag.value {
                            tag_json.insert(
                                "Value".to_string(),
                                serde_json::Value::String(value.clone()),
                            );
                        }
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        if let Some(secret_versions_to_stages) = &secret.secret_versions_to_stages {
            let stages_json = serde_json::Map::from_iter(secret_versions_to_stages.iter().map(
                |(version, stages)| {
                    let stages_array: Vec<serde_json::Value> = stages
                        .iter()
                        .map(|stage| serde_json::Value::String(stage.clone()))
                        .collect();
                    (version.clone(), serde_json::Value::Array(stages_array))
                },
            ));
            json.insert(
                "SecretVersionsToStages".to_string(),
                serde_json::Value::Object(stages_json),
            );
        }

        if let Some(owning_service) = &secret.owning_service {
            json.insert(
                "OwningService".to_string(),
                serde_json::Value::String(owning_service.clone()),
            );
        }

        if let Some(created_date) = secret.created_date {
            json.insert(
                "CreatedDate".to_string(),
                serde_json::Value::String(created_date.to_string()),
            );
        }

        if let Some(primary_region) = &secret.primary_region {
            json.insert(
                "PrimaryRegion".to_string(),
                serde_json::Value::String(primary_region.clone()),
            );
        }

        // Add a default status for consistency
        let status = if secret.deleted_date.is_some() {
            "DELETED"
        } else if secret.rotation_enabled.unwrap_or(false) {
            "ROTATION_ENABLED"
        } else {
            "ACTIVE"
        };
        json.insert(
            "Status".to_string(),
            serde_json::Value::String(status.to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn secret_description_to_json(
        &self,
        response: &secretsmanager::operation::describe_secret::DescribeSecretOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(arn) = &response.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(name) = &response.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(kms_key_id) = &response.kms_key_id {
            json.insert(
                "KmsKeyId".to_string(),
                serde_json::Value::String(kms_key_id.clone()),
            );
        }

        json.insert(
            "RotationEnabled".to_string(),
            serde_json::Value::Bool(response.rotation_enabled.unwrap_or(false)),
        );

        if let Some(rotation_lambda_arn) = &response.rotation_lambda_arn {
            json.insert(
                "RotationLambdaArn".to_string(),
                serde_json::Value::String(rotation_lambda_arn.clone()),
            );
        }

        // Add a default status for consistency
        let status = if response.deleted_date.is_some() {
            "DELETED"
        } else if response.rotation_enabled.unwrap_or(false) {
            "ROTATION_ENABLED"
        } else {
            "ACTIVE"
        };
        json.insert(
            "Status".to_string(),
            serde_json::Value::String(status.to_string()),
        );

        serde_json::Value::Object(json)
    }
}
