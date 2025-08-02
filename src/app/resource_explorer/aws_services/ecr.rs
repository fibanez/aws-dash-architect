use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_ecr as ecr;
use std::sync::Arc;

pub struct EcrService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl EcrService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List ECR Repositories
    pub async fn list_repositories(
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

        let client = ecr::Client::new(&aws_config);
        let mut paginator = client.describe_repositories().into_paginator().send();

        let mut repositories = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(repo_list) = page.repositories {
                for repo in repo_list {
                    let repo_json = self.repository_to_json(&repo);
                    repositories.push(repo_json);
                }
            }
        }

        Ok(repositories)
    }

    /// Get detailed information for specific ECR repository
    pub async fn describe_repository(
        &self,
        account_id: &str,
        region: &str,
        repository_name: &str,
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

        let client = ecr::Client::new(&aws_config);
        let response = client
            .describe_repositories()
            .repository_names(repository_name)
            .send()
            .await?;

        if let Some(repositories) = response.repositories {
            if let Some(repository) = repositories.first() {
                Ok(self.repository_to_json(repository))
            } else {
                Err(anyhow::anyhow!("Repository {} not found", repository_name))
            }
        } else {
            Err(anyhow::anyhow!("Repository {} not found", repository_name))
        }
    }

    fn repository_to_json(&self, repository: &ecr::types::Repository) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(repository_arn) = &repository.repository_arn {
            json.insert(
                "RepositoryArn".to_string(),
                serde_json::Value::String(repository_arn.clone()),
            );
        }

        if let Some(registry_id) = &repository.registry_id {
            json.insert(
                "RegistryId".to_string(),
                serde_json::Value::String(registry_id.clone()),
            );
        }

        if let Some(repository_name) = &repository.repository_name {
            json.insert(
                "RepositoryName".to_string(),
                serde_json::Value::String(repository_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(repository_name.clone()),
            );
        }

        if let Some(repository_uri) = &repository.repository_uri {
            json.insert(
                "RepositoryUri".to_string(),
                serde_json::Value::String(repository_uri.clone()),
            );
        }

        if let Some(created_at) = repository.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.to_string()),
            );
        }

        if let Some(image_tag_mutability) = &repository.image_tag_mutability {
            json.insert(
                "ImageTagMutability".to_string(),
                serde_json::Value::String(image_tag_mutability.as_str().to_string()),
            );
        }

        if let Some(image_scanning_configuration) = &repository.image_scanning_configuration {
            let mut scanning_json = serde_json::Map::new();
            scanning_json.insert(
                "ScanOnPush".to_string(),
                serde_json::Value::Bool(image_scanning_configuration.scan_on_push),
            );
            json.insert(
                "ImageScanningConfiguration".to_string(),
                serde_json::Value::Object(scanning_json),
            );
        }

        if let Some(encryption_configuration) = &repository.encryption_configuration {
            let mut encryption_json = serde_json::Map::new();
            encryption_json.insert(
                "EncryptionType".to_string(),
                serde_json::Value::String(
                    encryption_configuration
                        .encryption_type
                        .as_str()
                        .to_string(),
                ),
            );
            if let Some(kms_key) = &encryption_configuration.kms_key {
                encryption_json.insert(
                    "KmsKey".to_string(),
                    serde_json::Value::String(kms_key.clone()),
                );
            }
            json.insert(
                "EncryptionConfiguration".to_string(),
                serde_json::Value::Object(encryption_json),
            );
        }

        // Add a default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
