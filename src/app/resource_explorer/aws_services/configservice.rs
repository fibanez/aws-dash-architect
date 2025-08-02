use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_config as configservice;
use std::sync::Arc;

pub struct ConfigService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ConfigService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List AWS Config Configuration Recorders
    pub async fn list_configuration_recorders(
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

        let client = configservice::Client::new(&aws_config);
        let response = client.describe_configuration_recorders().send().await?;

        let mut recorders = Vec::new();
        if let Some(recorders_list) = response.configuration_recorders {
            for recorder in recorders_list {
                let recorder_json = self.configuration_recorder_to_json(&recorder);
                recorders.push(recorder_json);
            }
        }

        Ok(recorders)
    }

    /// Get detailed information for specific configuration recorder
    pub async fn describe_configuration_recorder(
        &self,
        account_id: &str,
        region: &str,
        recorder_name: &str,
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

        let client = configservice::Client::new(&aws_config);
        self.describe_configuration_recorder_internal(&client, recorder_name)
            .await
    }

    async fn describe_configuration_recorder_internal(
        &self,
        client: &configservice::Client,
        recorder_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_configuration_recorders()
            .configuration_recorder_names(recorder_name)
            .send()
            .await?;

        if let Some(recorders) = response.configuration_recorders {
            if let Some(recorder) = recorders.first() {
                Ok(self.configuration_recorder_to_json(recorder))
            } else {
                Err(anyhow::anyhow!(
                    "Configuration recorder {} not found",
                    recorder_name
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "Configuration recorder {} not found",
                recorder_name
            ))
        }
    }

    fn configuration_recorder_to_json(
        &self,
        recorder: &configservice::types::ConfigurationRecorder,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &recorder.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(role_arn) = &recorder.role_arn {
            json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(role_arn.clone()),
            );
        }

        if let Some(recording_group) = &recorder.recording_group {
            let mut recording_group_json = serde_json::Map::new();

            recording_group_json.insert(
                "AllSupported".to_string(),
                serde_json::Value::Bool(recording_group.all_supported),
            );

            recording_group_json.insert(
                "IncludeGlobalResourceTypes".to_string(),
                serde_json::Value::Bool(recording_group.include_global_resource_types),
            );

            if let Some(resource_types) = &recording_group.resource_types {
                if !resource_types.is_empty() {
                    let resource_types_json: Vec<serde_json::Value> = resource_types
                        .iter()
                        .map(|rt| serde_json::Value::String(rt.as_str().to_string()))
                        .collect();
                    recording_group_json.insert(
                        "ResourceTypes".to_string(),
                        serde_json::Value::Array(resource_types_json),
                    );
                }
            }

            if let Some(exclusion_by_resource_types) = &recording_group.exclusion_by_resource_types
            {
                let mut exclusion_json = serde_json::Map::new();

                if let Some(resource_types) = &exclusion_by_resource_types.resource_types {
                    if !resource_types.is_empty() {
                        let excluded_types_json: Vec<serde_json::Value> = resource_types
                            .iter()
                            .map(|rt| serde_json::Value::String(rt.as_str().to_string()))
                            .collect();
                        exclusion_json.insert(
                            "ResourceTypes".to_string(),
                            serde_json::Value::Array(excluded_types_json),
                        );
                    }
                }

                recording_group_json.insert(
                    "ExclusionByResourceTypes".to_string(),
                    serde_json::Value::Object(exclusion_json),
                );
            }

            json.insert(
                "RecordingGroup".to_string(),
                serde_json::Value::Object(recording_group_json),
            );
        }

        if let Some(recording_mode) = &recorder.recording_mode {
            let mut recording_mode_json = serde_json::Map::new();

            recording_mode_json.insert(
                "RecordingFrequency".to_string(),
                serde_json::Value::String(recording_mode.recording_frequency.as_str().to_string()),
            );

            if let Some(recording_mode_overrides) = &recording_mode.recording_mode_overrides {
                if !recording_mode_overrides.is_empty() {
                    let overrides_json: Vec<serde_json::Value> = recording_mode_overrides
                        .iter()
                        .map(|override_item| {
                            let mut override_json = serde_json::Map::new();

                            if let Some(description) = &override_item.description {
                                override_json.insert(
                                    "Description".to_string(),
                                    serde_json::Value::String(description.clone()),
                                );
                            }

                            if !override_item.resource_types.is_empty() {
                                let types_json: Vec<serde_json::Value> = override_item
                                    .resource_types
                                    .iter()
                                    .map(|rt| serde_json::Value::String(rt.as_str().to_string()))
                                    .collect();
                                override_json.insert(
                                    "ResourceTypes".to_string(),
                                    serde_json::Value::Array(types_json),
                                );
                            }

                            override_json.insert(
                                "RecordingFrequency".to_string(),
                                serde_json::Value::String(
                                    override_item.recording_frequency.as_str().to_string(),
                                ),
                            );

                            serde_json::Value::Object(override_json)
                        })
                        .collect();
                    recording_mode_json.insert(
                        "RecordingModeOverrides".to_string(),
                        serde_json::Value::Array(overrides_json),
                    );
                }
            }

            json.insert(
                "RecordingMode".to_string(),
                serde_json::Value::Object(recording_mode_json),
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
