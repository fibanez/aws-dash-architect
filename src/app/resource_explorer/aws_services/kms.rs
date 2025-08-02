use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_kms as kms;
use std::sync::Arc;

pub struct KmsService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl KmsService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List KMS Keys
    pub async fn list_keys(
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

        let client = kms::Client::new(&aws_config);
        let mut paginator = client.list_keys().into_paginator().send();

        let mut keys = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(key_list) = page.keys {
                for key in key_list {
                    // Get detailed key information
                    if let Some(key_id) = &key.key_id {
                        if let Ok(key_details) = self.describe_key_internal(&client, key_id).await {
                            keys.push(key_details);
                        } else {
                            // Fallback to basic key info if describe fails
                            let key_json = self.key_list_entry_to_json(&key);
                            keys.push(key_json);
                        }
                    } else {
                        // Fallback to basic key info if no ID
                        let key_json = self.key_list_entry_to_json(&key);
                        keys.push(key_json);
                    }
                }
            }
        }

        Ok(keys)
    }

    /// Get detailed information for specific KMS key
    pub async fn describe_key(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
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

        let client = kms::Client::new(&aws_config);
        self.describe_key_internal(&client, key_id).await
    }

    async fn describe_key_internal(
        &self,
        client: &kms::Client,
        key_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client.describe_key().key_id(key_id).send().await?;

        if let Some(key_metadata) = response.key_metadata {
            Ok(self.key_metadata_to_json(&key_metadata))
        } else {
            Err(anyhow::anyhow!("Key {} not found", key_id))
        }
    }

    fn key_list_entry_to_json(&self, key: &kms::types::KeyListEntry) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(key_id) = &key.key_id {
            json.insert(
                "KeyId".to_string(),
                serde_json::Value::String(key_id.clone()),
            );
        }

        if let Some(key_arn) = &key.key_arn {
            json.insert(
                "Arn".to_string(),
                serde_json::Value::String(key_arn.clone()),
            );
        }

        // Add default fields for consistency
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(key.key_id.as_deref().unwrap_or("unknown-key").to_string()),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("UNKNOWN".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn key_metadata_to_json(&self, key_metadata: &kms::types::KeyMetadata) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(aws_account_id) = &key_metadata.aws_account_id {
            json.insert(
                "AwsAccountId".to_string(),
                serde_json::Value::String(aws_account_id.clone()),
            );
        }

        json.insert(
            "KeyId".to_string(),
            serde_json::Value::String(key_metadata.key_id.clone()),
        );

        if let Some(arn) = &key_metadata.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(creation_date) = key_metadata.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        json.insert(
            "Enabled".to_string(),
            serde_json::Value::Bool(key_metadata.enabled),
        );

        if let Some(description) = &key_metadata.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(key_usage) = &key_metadata.key_usage {
            json.insert(
                "KeyUsage".to_string(),
                serde_json::Value::String(key_usage.as_str().to_string()),
            );
        }

        if let Some(key_state) = &key_metadata.key_state {
            json.insert(
                "KeyState".to_string(),
                serde_json::Value::String(key_state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(key_state.as_str().to_string()),
            );
        }

        if let Some(deletion_date) = key_metadata.deletion_date {
            json.insert(
                "DeletionDate".to_string(),
                serde_json::Value::String(deletion_date.to_string()),
            );
        }

        if let Some(valid_to) = key_metadata.valid_to {
            json.insert(
                "ValidTo".to_string(),
                serde_json::Value::String(valid_to.to_string()),
            );
        }

        if let Some(origin) = &key_metadata.origin {
            json.insert(
                "Origin".to_string(),
                serde_json::Value::String(origin.as_str().to_string()),
            );
        }

        if let Some(custom_key_store_id) = &key_metadata.custom_key_store_id {
            json.insert(
                "CustomKeyStoreId".to_string(),
                serde_json::Value::String(custom_key_store_id.clone()),
            );
        }

        if let Some(cloud_hsm_cluster_id) = &key_metadata.cloud_hsm_cluster_id {
            json.insert(
                "CloudHsmClusterId".to_string(),
                serde_json::Value::String(cloud_hsm_cluster_id.clone()),
            );
        }

        if let Some(expiration_model) = &key_metadata.expiration_model {
            json.insert(
                "ExpirationModel".to_string(),
                serde_json::Value::String(expiration_model.as_str().to_string()),
            );
        }

        if let Some(key_manager) = &key_metadata.key_manager {
            json.insert(
                "KeyManager".to_string(),
                serde_json::Value::String(key_manager.as_str().to_string()),
            );
        }

        // Use the newer KeySpec field instead of deprecated customer_master_key_spec
        // if let Some(customer_master_key_spec) = &key_metadata.customer_master_key_spec {
        //     json.insert("CustomerMasterKeySpec".to_string(), serde_json::Value::String(customer_master_key_spec.as_str().to_string()));
        // }

        if let Some(key_spec) = &key_metadata.key_spec {
            json.insert(
                "KeySpec".to_string(),
                serde_json::Value::String(key_spec.as_str().to_string()),
            );
        }

        if let Some(encryption_algorithms) = &key_metadata.encryption_algorithms {
            if !encryption_algorithms.is_empty() {
                let algorithms_json: Vec<serde_json::Value> = encryption_algorithms
                    .iter()
                    .map(|alg| serde_json::Value::String(alg.as_str().to_string()))
                    .collect();
                json.insert(
                    "EncryptionAlgorithms".to_string(),
                    serde_json::Value::Array(algorithms_json),
                );
            }
        }

        if let Some(signing_algorithms) = &key_metadata.signing_algorithms {
            if !signing_algorithms.is_empty() {
                let algorithms_json: Vec<serde_json::Value> = signing_algorithms
                    .iter()
                    .map(|alg| serde_json::Value::String(alg.as_str().to_string()))
                    .collect();
                json.insert(
                    "SigningAlgorithms".to_string(),
                    serde_json::Value::Array(algorithms_json),
                );
            }
        }

        json.insert(
            "MultiRegion".to_string(),
            serde_json::Value::Bool(key_metadata.multi_region.unwrap_or(false)),
        );

        if let Some(multi_region_configuration) = &key_metadata.multi_region_configuration {
            let mut config_json = serde_json::Map::new();
            if let Some(multi_region_key_type) = &multi_region_configuration.multi_region_key_type {
                config_json.insert(
                    "MultiRegionKeyType".to_string(),
                    serde_json::Value::String(multi_region_key_type.as_str().to_string()),
                );
            }
            if let Some(primary_key) = &multi_region_configuration.primary_key {
                let mut primary_json = serde_json::Map::new();
                if let Some(arn) = &primary_key.arn {
                    primary_json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
                }
                if let Some(region) = &primary_key.region {
                    primary_json.insert(
                        "Region".to_string(),
                        serde_json::Value::String(region.clone()),
                    );
                }
                config_json.insert(
                    "PrimaryKey".to_string(),
                    serde_json::Value::Object(primary_json),
                );
            }
            json.insert(
                "MultiRegionConfiguration".to_string(),
                serde_json::Value::Object(config_json),
            );
        }

        // Use the description or key ID as name
        let name = key_metadata
            .description
            .as_deref()
            .unwrap_or(&key_metadata.key_id)
            .to_string();
        json.insert("Name".to_string(), serde_json::Value::String(name));

        serde_json::Value::Object(json)
    }
}
