use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_efs as efs;
use std::sync::Arc;

pub struct EfsService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl EfsService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List EFS File Systems
    pub async fn list_file_systems(
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

        let client = efs::Client::new(&aws_config);
        let mut paginator = client.describe_file_systems().into_paginator().send();

        let mut file_systems = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(fs_list) = page.file_systems {
                for fs in fs_list {
                    let fs_json = self.file_system_to_json(&fs);
                    file_systems.push(fs_json);
                }
            }
        }

        Ok(file_systems)
    }

    /// Get detailed information for specific file system
    pub async fn describe_file_system(
        &self,
        account_id: &str,
        region: &str,
        file_system_id: &str,
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

        let client = efs::Client::new(&aws_config);
        self.describe_file_system_internal(&client, file_system_id)
            .await
    }

    async fn describe_file_system_internal(
        &self,
        client: &efs::Client,
        file_system_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_file_systems()
            .file_system_id(file_system_id)
            .send()
            .await?;

        if let Some(file_systems) = response.file_systems {
            if let Some(file_system) = file_systems.first() {
                Ok(self.file_system_to_json(file_system))
            } else {
                Err(anyhow::anyhow!("File system {} not found", file_system_id))
            }
        } else {
            Err(anyhow::anyhow!("File system {} not found", file_system_id))
        }
    }

    fn file_system_to_json(
        &self,
        file_system: &efs::types::FileSystemDescription,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "OwnerId".to_string(),
            serde_json::Value::String(file_system.owner_id.clone()),
        );

        json.insert(
            "CreationToken".to_string(),
            serde_json::Value::String(file_system.creation_token.clone()),
        );

        json.insert(
            "FileSystemId".to_string(),
            serde_json::Value::String(file_system.file_system_id.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(file_system.file_system_id.clone()),
        );

        if let Some(file_system_arn) = &file_system.file_system_arn {
            json.insert(
                "FileSystemArn".to_string(),
                serde_json::Value::String(file_system_arn.clone()),
            );
        }

        json.insert(
            "CreationTime".to_string(),
            serde_json::Value::String(file_system.creation_time.to_string()),
        );

        json.insert(
            "LifeCycleState".to_string(),
            serde_json::Value::String(file_system.life_cycle_state.as_str().to_string()),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String(file_system.life_cycle_state.as_str().to_string()),
        );

        if let Some(name) = &file_system.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        } else {
            // Use file system ID as name if no name is set
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(file_system.file_system_id.clone()),
            );
        }

        json.insert(
            "NumberOfMountTargets".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                file_system.number_of_mount_targets,
            )),
        );

        if let Some(size_in_bytes) = &file_system.size_in_bytes {
            let mut size_json = serde_json::Map::new();

            size_json.insert(
                "Value".to_string(),
                serde_json::Value::Number(serde_json::Number::from(size_in_bytes.value as u64)),
            );

            if let Some(timestamp) = size_in_bytes.timestamp {
                size_json.insert(
                    "Timestamp".to_string(),
                    serde_json::Value::String(timestamp.to_string()),
                );
            }

            if let Some(value_in_ia) = size_in_bytes.value_in_ia {
                size_json.insert(
                    "ValueInIA".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(value_in_ia as u64)),
                );
            }

            if let Some(value_in_standard) = size_in_bytes.value_in_standard {
                size_json.insert(
                    "ValueInStandard".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(value_in_standard as u64)),
                );
            }

            json.insert(
                "SizeInBytes".to_string(),
                serde_json::Value::Object(size_json),
            );
        }

        json.insert(
            "PerformanceMode".to_string(),
            serde_json::Value::String(file_system.performance_mode.as_str().to_string()),
        );

        if let Some(throughput_mode) = &file_system.throughput_mode {
            json.insert(
                "ThroughputMode".to_string(),
                serde_json::Value::String(throughput_mode.as_str().to_string()),
            );
        }

        if let Some(provisioned_throughput_in_mibps) = file_system.provisioned_throughput_in_mibps {
            if let Some(throughput_num) =
                serde_json::Number::from_f64(provisioned_throughput_in_mibps)
            {
                json.insert(
                    "ProvisionedThroughputInMibps".to_string(),
                    serde_json::Value::Number(throughput_num),
                );
            }
        }

        json.insert(
            "Encrypted".to_string(),
            serde_json::Value::Bool(file_system.encrypted.unwrap_or(false)),
        );

        if let Some(kms_key_id) = &file_system.kms_key_id {
            json.insert(
                "KmsKeyId".to_string(),
                serde_json::Value::String(kms_key_id.clone()),
            );
        }

        if let Some(availability_zone_name) = &file_system.availability_zone_name {
            json.insert(
                "AvailabilityZoneName".to_string(),
                serde_json::Value::String(availability_zone_name.clone()),
            );
        }

        if let Some(availability_zone_id) = &file_system.availability_zone_id {
            json.insert(
                "AvailabilityZoneId".to_string(),
                serde_json::Value::String(availability_zone_id.clone()),
            );
        }

        if !file_system.tags.is_empty() {
            let tags_json: Vec<serde_json::Value> = file_system
                .tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    tag_json.insert(
                        "Key".to_string(),
                        serde_json::Value::String(tag.key.clone()),
                    );
                    tag_json.insert(
                        "Value".to_string(),
                        serde_json::Value::String(tag.value.clone()),
                    );
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
        }

        serde_json::Value::Object(json)
    }
}
