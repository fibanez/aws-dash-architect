use anyhow::{Result, Context};
use aws_sdk_amplify as amplify;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct AmplifyService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AmplifyService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Amplify apps (basic list data)
    pub async fn list_apps(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = amplify::Client::new(&aws_config);
        
        let mut paginator = client
            .list_apps()
            .into_paginator()
            .send();

        let mut apps = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            for app in page.apps {
                let app_json = self.app_to_json(&app);
                apps.push(app_json);
            }
        }

        Ok(apps)
    }

    /// Get detailed information for specific Amplify app (for describe functionality)
    pub async fn get_app(
        &self,
        account_id: &str,
        region: &str,
        app_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = amplify::Client::new(&aws_config);
        self.get_app_internal(&client, app_id).await
    }

    async fn get_app_internal(
        &self,
        client: &amplify::Client,
        app_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .get_app()
            .app_id(app_id)
            .send()
            .await?;

        if let Some(app) = response.app {
            Ok(self.app_to_json(&app))
        } else {
            Err(anyhow::anyhow!("Amplify app {} not found", app_id))
        }
    }

    fn app_to_json(&self, app: &amplify::types::App) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        json.insert("AppId".to_string(), serde_json::Value::String(app.app_id.clone()));
        json.insert("ResourceId".to_string(), serde_json::Value::String(app.app_id.clone()));
        
        json.insert("AppArn".to_string(), serde_json::Value::String(app.app_arn.clone()));
        json.insert("Name".to_string(), serde_json::Value::String(app.name.clone()));

        json.insert("Description".to_string(), serde_json::Value::String(app.description.clone()));
        json.insert("Repository".to_string(), serde_json::Value::String(app.repository.clone()));
        json.insert("Platform".to_string(), serde_json::Value::String(app.platform.as_str().to_string()));

        json.insert("CreateTime".to_string(), serde_json::Value::String(app.create_time.to_string()));
        json.insert("UpdateTime".to_string(), serde_json::Value::String(app.update_time.to_string()));

        if let Some(iam_service_role_arn) = &app.iam_service_role_arn {
            json.insert("IamServiceRoleArn".to_string(), serde_json::Value::String(iam_service_role_arn.clone()));
        }

        if !app.environment_variables.is_empty() {
            let env_vars_json: Vec<serde_json::Value> = app.environment_variables
                .iter()
                .map(|(key, value)| {
                    let mut env_var_json = serde_json::Map::new();
                    env_var_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    env_var_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                    serde_json::Value::Object(env_var_json)
                })
                .collect();
            json.insert("EnvironmentVariables".to_string(), serde_json::Value::Array(env_vars_json));
        }

        json.insert("DefaultDomain".to_string(), serde_json::Value::String(app.default_domain.clone()));
        json.insert("EnableBranchAutoBuild".to_string(), serde_json::Value::Bool(app.enable_branch_auto_build));
        json.insert("EnableBranchAutoDeletion".to_string(), serde_json::Value::Bool(app.enable_branch_auto_deletion.unwrap_or(false)));
        json.insert("EnableBasicAuth".to_string(), serde_json::Value::Bool(app.enable_basic_auth));

        if let Some(basic_auth_credentials) = &app.basic_auth_credentials {
            json.insert("BasicAuthCredentials".to_string(), serde_json::Value::String(basic_auth_credentials.clone()));
        }

        if let Some(custom_rules) = &app.custom_rules {
            if !custom_rules.is_empty() {
                let rules_json: Vec<serde_json::Value> = custom_rules
                    .iter()
                    .map(|rule| {
                        let mut rule_json = serde_json::Map::new();
                        rule_json.insert("Source".to_string(), serde_json::Value::String(rule.source.clone()));
                        rule_json.insert("Target".to_string(), serde_json::Value::String(rule.target.clone()));
                        if let Some(status) = &rule.status {
                            rule_json.insert("Status".to_string(), serde_json::Value::String(status.clone()));
                        }
                        if let Some(condition) = &rule.condition {
                            rule_json.insert("Condition".to_string(), serde_json::Value::String(condition.clone()));
                        }
                        serde_json::Value::Object(rule_json)
                    })
                    .collect();
                json.insert("CustomRules".to_string(), serde_json::Value::Array(rules_json));
            }
        }

        if let Some(production_branch) = &app.production_branch {
            let mut prod_branch_json = serde_json::Map::new();
            if let Some(last_deploy_time) = production_branch.last_deploy_time {
                prod_branch_json.insert("LastDeployTime".to_string(), serde_json::Value::String(last_deploy_time.to_string()));
            }
            if let Some(status) = &production_branch.status {
                prod_branch_json.insert("Status".to_string(), serde_json::Value::String(status.clone()));
            }
            if let Some(thumbnail_url) = &production_branch.thumbnail_url {
                prod_branch_json.insert("ThumbnailUrl".to_string(), serde_json::Value::String(thumbnail_url.clone()));
            }
            if let Some(branch_name) = &production_branch.branch_name {
                prod_branch_json.insert("BranchName".to_string(), serde_json::Value::String(branch_name.clone()));
            }
            json.insert("ProductionBranch".to_string(), serde_json::Value::Object(prod_branch_json));
        }

        if let Some(build_spec) = &app.build_spec {
            json.insert("BuildSpec".to_string(), serde_json::Value::String(build_spec.clone()));
        }

        if let Some(custom_headers) = &app.custom_headers {
            json.insert("CustomHeaders".to_string(), serde_json::Value::String(custom_headers.clone()));
        }

        json.insert("EnableAutoBranchCreation".to_string(), serde_json::Value::Bool(app.enable_auto_branch_creation.unwrap_or(false)));

        if let Some(tags) = &app.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|(key, value)| {
                        let mut tag_json = serde_json::Map::new();
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                        tag_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }
}