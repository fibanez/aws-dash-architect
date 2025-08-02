use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_athena as athena;
use std::sync::Arc;

pub struct AthenaService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AthenaService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Athena Workgroups
    pub async fn list_work_groups(
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

        let client = athena::Client::new(&aws_config);
        let mut paginator = client.list_work_groups().into_paginator().send();

        let mut workgroups = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(workgroup_list) = page.work_groups {
                for workgroup in workgroup_list {
                    // Get detailed workgroup information
                    if let Some(workgroup_name) = &workgroup.name {
                        if let Ok(workgroup_details) = self
                            .describe_workgroup_internal(&client, workgroup_name)
                            .await
                        {
                            workgroups.push(workgroup_details);
                        } else {
                            // Fallback to basic workgroup info if describe fails
                            let workgroup_json = self.workgroup_summary_to_json(&workgroup);
                            workgroups.push(workgroup_json);
                        }
                    } else {
                        // Fallback to basic workgroup info if no name
                        let workgroup_json = self.workgroup_summary_to_json(&workgroup);
                        workgroups.push(workgroup_json);
                    }
                }
            }
        }

        Ok(workgroups)
    }

    /// Get detailed information for specific Athena workgroup
    pub async fn describe_work_group(
        &self,
        account_id: &str,
        region: &str,
        work_group: &str,
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

        let client = athena::Client::new(&aws_config);
        self.describe_workgroup_internal(&client, work_group).await
    }

    async fn describe_workgroup_internal(
        &self,
        client: &athena::Client,
        work_group: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_work_group()
            .work_group(work_group)
            .send()
            .await?;

        if let Some(workgroup) = response.work_group {
            Ok(self.workgroup_to_json(&workgroup))
        } else {
            Err(anyhow::anyhow!("Workgroup {} not found", work_group))
        }
    }

    fn workgroup_summary_to_json(
        &self,
        workgroup: &athena::types::WorkGroupSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &workgroup.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(state) = &workgroup.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(description) = &workgroup.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(creation_time) = workgroup.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(engine_version) = &workgroup.engine_version {
            let mut engine_json = serde_json::Map::new();
            if let Some(selected_engine_version) = &engine_version.selected_engine_version {
                engine_json.insert(
                    "SelectedEngineVersion".to_string(),
                    serde_json::Value::String(selected_engine_version.clone()),
                );
            }
            if let Some(effective_engine_version) = &engine_version.effective_engine_version {
                engine_json.insert(
                    "EffectiveEngineVersion".to_string(),
                    serde_json::Value::String(effective_engine_version.clone()),
                );
            }
            json.insert(
                "EngineVersion".to_string(),
                serde_json::Value::Object(engine_json),
            );
        }

        serde_json::Value::Object(json)
    }

    fn workgroup_to_json(&self, workgroup: &athena::types::WorkGroup) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(workgroup.name.clone()),
        );

        if let Some(state) = &workgroup.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(configuration) = &workgroup.configuration {
            let mut config_json = serde_json::Map::new();

            if let Some(result_configuration) = &configuration.result_configuration {
                let mut result_config_json = serde_json::Map::new();
                if let Some(output_location) = &result_configuration.output_location {
                    result_config_json.insert(
                        "OutputLocation".to_string(),
                        serde_json::Value::String(output_location.clone()),
                    );
                }
                if let Some(encryption_configuration) =
                    &result_configuration.encryption_configuration
                {
                    let mut encryption_json = serde_json::Map::new();
                    encryption_json.insert(
                        "EncryptionOption".to_string(),
                        serde_json::Value::String(
                            encryption_configuration
                                .encryption_option
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
                    result_config_json.insert(
                        "EncryptionConfiguration".to_string(),
                        serde_json::Value::Object(encryption_json),
                    );
                }
                config_json.insert(
                    "ResultConfiguration".to_string(),
                    serde_json::Value::Object(result_config_json),
                );
            }

            if let Some(enforce_config) = configuration.enforce_work_group_configuration {
                config_json.insert(
                    "EnforceWorkGroupConfiguration".to_string(),
                    serde_json::Value::Bool(enforce_config),
                );
            }
            if let Some(publish_metrics) = configuration.publish_cloud_watch_metrics_enabled {
                config_json.insert(
                    "PublishCloudWatchMetrics".to_string(),
                    serde_json::Value::Bool(publish_metrics),
                );
            }

            if let Some(bytes_scanned_cutoff_per_query) =
                configuration.bytes_scanned_cutoff_per_query
            {
                config_json.insert(
                    "BytesScannedCutoffPerQuery".to_string(),
                    serde_json::Value::Number(bytes_scanned_cutoff_per_query.into()),
                );
            }

            if let Some(requester_pays) = configuration.requester_pays_enabled {
                config_json.insert(
                    "RequesterPaysEnabled".to_string(),
                    serde_json::Value::Bool(requester_pays),
                );
            }

            if let Some(engine_version) = &configuration.engine_version {
                let mut engine_json = serde_json::Map::new();
                if let Some(selected_engine_version) = &engine_version.selected_engine_version {
                    engine_json.insert(
                        "SelectedEngineVersion".to_string(),
                        serde_json::Value::String(selected_engine_version.clone()),
                    );
                }
                if let Some(effective_engine_version) = &engine_version.effective_engine_version {
                    engine_json.insert(
                        "EffectiveEngineVersion".to_string(),
                        serde_json::Value::String(effective_engine_version.clone()),
                    );
                }
                config_json.insert(
                    "EngineVersion".to_string(),
                    serde_json::Value::Object(engine_json),
                );
            }

            json.insert(
                "Configuration".to_string(),
                serde_json::Value::Object(config_json),
            );
        }

        if let Some(description) = &workgroup.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(creation_time) = workgroup.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }
}
