use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_codebuild as codebuild;
use std::sync::Arc;

pub struct CodeBuildService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CodeBuildService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CodeBuild projects
    pub async fn list_projects(
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

        let client = codebuild::Client::new(&aws_config);
        let response = client.list_projects().send().await?;

        let mut projects = Vec::new();
        if let Some(project_names) = response.projects {
            // Get detailed information for each project
            if !project_names.is_empty() {
                let batch_response = client
                    .batch_get_projects()
                    .set_names(Some(project_names))
                    .send()
                    .await?;

                if let Some(project_list) = batch_response.projects {
                    for project in project_list {
                        let project_json = self.project_to_json(&project);
                        projects.push(project_json);
                    }
                }
            }
        }

        Ok(projects)
    }

    /// Get detailed information for specific CodeBuild project
    pub async fn describe_project(
        &self,
        account_id: &str,
        region: &str,
        project_name: &str,
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

        let client = codebuild::Client::new(&aws_config);
        let response = client
            .batch_get_projects()
            .names(project_name)
            .send()
            .await?;

        if let Some(projects) = response.projects {
            if let Some(project) = projects.first() {
                return Ok(self.project_to_json(project));
            }
        }

        Err(anyhow::anyhow!("Project {} not found", project_name))
    }

    fn project_to_json(&self, project: &codebuild::types::Project) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &project.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "ProjectName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &project.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(description) = &project.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(service_role) = &project.service_role {
            json.insert(
                "ServiceRole".to_string(),
                serde_json::Value::String(service_role.clone()),
            );
        }

        if let Some(created) = project.created {
            json.insert(
                "Created".to_string(),
                serde_json::Value::String(created.to_string()),
            );
        }

        if let Some(last_modified) = project.last_modified {
            json.insert(
                "LastModified".to_string(),
                serde_json::Value::String(last_modified.to_string()),
            );
        }

        // Convert source to JSON
        if let Some(source) = &project.source {
            let mut source_json = serde_json::Map::new();
            source_json.insert(
                "Type".to_string(),
                serde_json::Value::String(source.r#type.as_str().to_string()),
            );

            if let Some(location) = &source.location {
                source_json.insert(
                    "Location".to_string(),
                    serde_json::Value::String(location.clone()),
                );
            }

            if let Some(buildspec) = &source.buildspec {
                source_json.insert(
                    "Buildspec".to_string(),
                    serde_json::Value::String(buildspec.clone()),
                );
            }

            if let Some(git_clone_depth) = source.git_clone_depth {
                source_json.insert(
                    "GitCloneDepth".to_string(),
                    serde_json::Value::Number(git_clone_depth.into()),
                );
            }

            json.insert("Source".to_string(), serde_json::Value::Object(source_json));
        }

        // Convert environment to JSON
        if let Some(environment) = &project.environment {
            let mut env_json = serde_json::Map::new();
            env_json.insert(
                "Type".to_string(),
                serde_json::Value::String(environment.r#type.as_str().to_string()),
            );
            env_json.insert(
                "Image".to_string(),
                serde_json::Value::String(environment.image.clone()),
            );
            env_json.insert(
                "ComputeType".to_string(),
                serde_json::Value::String(environment.compute_type.as_str().to_string()),
            );

            if let Some(privileged_mode) = environment.privileged_mode {
                env_json.insert(
                    "PrivilegedMode".to_string(),
                    serde_json::Value::Bool(privileged_mode),
                );
            }

            if let Some(environment_variables) = &environment.environment_variables {
                let env_vars: Vec<serde_json::Value> = environment_variables
                    .iter()
                    .map(|env_var| {
                        let mut var_json = serde_json::Map::new();
                        var_json.insert(
                            "Name".to_string(),
                            serde_json::Value::String(env_var.name.clone()),
                        );
                        var_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(env_var.value.clone()),
                        );
                        if let Some(var_type) = &env_var.r#type {
                            var_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String(var_type.as_str().to_string()),
                            );
                        }
                        serde_json::Value::Object(var_json)
                    })
                    .collect();
                env_json.insert(
                    "EnvironmentVariables".to_string(),
                    serde_json::Value::Array(env_vars),
                );
            }

            json.insert(
                "Environment".to_string(),
                serde_json::Value::Object(env_json),
            );
        }

        // Convert artifacts to JSON
        if let Some(artifacts) = &project.artifacts {
            let mut artifacts_json = serde_json::Map::new();
            artifacts_json.insert(
                "Type".to_string(),
                serde_json::Value::String(artifacts.r#type.as_str().to_string()),
            );

            if let Some(location) = &artifacts.location {
                artifacts_json.insert(
                    "Location".to_string(),
                    serde_json::Value::String(location.clone()),
                );
            }

            if let Some(name) = &artifacts.name {
                artifacts_json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            }

            json.insert(
                "Artifacts".to_string(),
                serde_json::Value::Object(artifacts_json),
            );
        }

        if let Some(timeout_in_minutes) = project.timeout_in_minutes {
            json.insert(
                "TimeoutInMinutes".to_string(),
                serde_json::Value::Number(timeout_in_minutes.into()),
            );
        }

        if let Some(queued_timeout_in_minutes) = project.queued_timeout_in_minutes {
            json.insert(
                "QueuedTimeoutInMinutes".to_string(),
                serde_json::Value::Number(queued_timeout_in_minutes.into()),
            );
        }

        if let Some(badge) = &project.badge {
            json.insert(
                "BadgeEnabled".to_string(),
                serde_json::Value::Bool(badge.badge_enabled),
            );
            if let Some(badge_request_url) = &badge.badge_request_url {
                json.insert(
                    "BadgeRequestUrl".to_string(),
                    serde_json::Value::String(badge_request_url.clone()),
                );
            }
        }

        if let Some(tags) = &project.tags {
            let tags_array: Vec<serde_json::Value> = tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    if let Some(key) = &tag.key {
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
            json.insert("Tags".to_string(), serde_json::Value::Array(tags_array));
        }

        // Set status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
