use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_codepipeline as codepipeline;
use std::sync::Arc;

pub struct CodePipelineService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CodePipelineService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CodePipeline pipelines
    pub async fn list_pipelines(
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

        let client = codepipeline::Client::new(&aws_config);
        let response = client.list_pipelines().send().await?;

        let mut pipelines = Vec::new();
        if let Some(pipeline_summaries) = response.pipelines {
            for pipeline in pipeline_summaries {
                let pipeline_json = self.pipeline_summary_to_json(&pipeline);
                pipelines.push(pipeline_json);
            }
        }

        Ok(pipelines)
    }

    /// Get detailed information for specific CodePipeline pipeline
    pub async fn describe_pipeline(
        &self,
        account_id: &str,
        region: &str,
        pipeline_name: &str,
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

        let client = codepipeline::Client::new(&aws_config);
        let response = client.get_pipeline().name(pipeline_name).send().await?;

        if let Some(pipeline) = response.pipeline {
            return Ok(self.pipeline_to_json(&pipeline));
        }

        Err(anyhow::anyhow!("Pipeline {} not found", pipeline_name))
    }

    fn pipeline_summary_to_json(
        &self,
        pipeline: &codepipeline::types::PipelineSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &pipeline.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "PipelineName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(version) = pipeline.version {
            json.insert(
                "Version".to_string(),
                serde_json::Value::Number(version.into()),
            );
        }

        if let Some(created) = pipeline.created {
            json.insert(
                "Created".to_string(),
                serde_json::Value::String(created.to_string()),
            );
        }

        if let Some(updated) = pipeline.updated {
            json.insert(
                "Updated".to_string(),
                serde_json::Value::String(updated.to_string()),
            );
        }

        // Set default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn pipeline_to_json(
        &self,
        pipeline: &codepipeline::types::PipelineDeclaration,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "Name".to_string(),
            serde_json::Value::String(pipeline.name.clone()),
        );
        json.insert(
            "PipelineName".to_string(),
            serde_json::Value::String(pipeline.name.clone()),
        );

        json.insert(
            "RoleArn".to_string(),
            serde_json::Value::String(pipeline.role_arn.clone()),
        );

        if let Some(version) = pipeline.version {
            json.insert(
                "Version".to_string(),
                serde_json::Value::Number(version.into()),
            );
        }

        // Convert artifact store to JSON
        if let Some(artifact_store) = &pipeline.artifact_store {
            let mut artifact_store_json = serde_json::Map::new();
            artifact_store_json.insert(
                "Type".to_string(),
                serde_json::Value::String(artifact_store.r#type.as_str().to_string()),
            );

            artifact_store_json.insert(
                "Location".to_string(),
                serde_json::Value::String(artifact_store.location.clone()),
            );

            if let Some(encryption_key) = &artifact_store.encryption_key {
                let mut encryption_json = serde_json::Map::new();
                encryption_json.insert(
                    "Type".to_string(),
                    serde_json::Value::String(encryption_key.r#type.as_str().to_string()),
                );
                encryption_json.insert(
                    "Id".to_string(),
                    serde_json::Value::String(encryption_key.id.clone()),
                );
                artifact_store_json.insert(
                    "EncryptionKey".to_string(),
                    serde_json::Value::Object(encryption_json),
                );
            }

            json.insert(
                "ArtifactStore".to_string(),
                serde_json::Value::Object(artifact_store_json),
            );
        }

        // Convert stages to JSON
        let stages: Vec<serde_json::Value> = pipeline
            .stages
            .iter()
            .map(|stage| {
                let mut stage_json = serde_json::Map::new();
                stage_json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(stage.name.clone()),
                );

                let actions: Vec<serde_json::Value> = stage
                    .actions
                    .iter()
                    .map(|action| {
                        let mut action_json = serde_json::Map::new();
                        action_json.insert(
                            "Name".to_string(),
                            serde_json::Value::String(action.name.clone()),
                        );
                        if let Some(action_type_id) = &action.action_type_id {
                            action_json.insert(
                                "ActionTypeId".to_string(),
                                serde_json::Value::String(format!(
                                    "{}:{}:{}:{}",
                                    action_type_id.category.as_str(),
                                    action_type_id.owner.as_str(),
                                    action_type_id.provider.clone(),
                                    action_type_id.version.clone()
                                )),
                            );
                        }

                        if let Some(run_order) = action.run_order {
                            action_json.insert(
                                "RunOrder".to_string(),
                                serde_json::Value::Number(run_order.into()),
                            );
                        }

                        if let Some(configuration) = &action.configuration {
                            let config_map: serde_json::Map<String, serde_json::Value> =
                                configuration
                                    .iter()
                                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                                    .collect();
                            action_json.insert(
                                "Configuration".to_string(),
                                serde_json::Value::Object(config_map),
                            );
                        }

                        serde_json::Value::Object(action_json)
                    })
                    .collect();

                stage_json.insert("Actions".to_string(), serde_json::Value::Array(actions));
                serde_json::Value::Object(stage_json)
            })
            .collect();

        json.insert("Stages".to_string(), serde_json::Value::Array(stages));

        // Set status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
