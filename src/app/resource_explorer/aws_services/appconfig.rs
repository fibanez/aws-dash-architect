#![warn(clippy::all, rust_2018_idioms)]

use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_appconfig as appconfig;
use std::sync::Arc;

pub struct AppConfigService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AppConfigService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List AppConfig applications
    pub async fn list_applications(
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

        let client = appconfig::Client::new(&aws_config);
        let mut applications = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_applications();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(items) = response.items {
                for application in items {
                    let app_json = self.application_to_json(&application);
                    applications.push(app_json);
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(applications)
    }

    /// List AppConfig environments
    pub async fn list_environments(
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

        let client = appconfig::Client::new(&aws_config);
        let mut environments = Vec::new();

        // First get all applications
        let applications = self.list_applications(account_id, region).await?;

        for app in applications {
            if let Some(app_id) = app.get("Id").and_then(|v| v.as_str()) {
                let mut next_token: Option<String> = None;

                loop {
                    let mut request = client.list_environments().application_id(app_id);
                    if let Some(token) = next_token {
                        request = request.next_token(token);
                    }

                    let response = request.send().await?;

                    if let Some(items) = response.items {
                        for environment in items {
                            let env_json = self.environment_to_json(&environment, app_id);
                            environments.push(env_json);
                        }
                    }

                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
            }
        }

        Ok(environments)
    }

    /// List AppConfig configuration profiles
    pub async fn list_configuration_profiles(
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

        let client = appconfig::Client::new(&aws_config);
        let mut configuration_profiles = Vec::new();

        // First get all applications
        let applications = self.list_applications(account_id, region).await?;

        for app in applications {
            if let Some(app_id) = app.get("Id").and_then(|v| v.as_str()) {
                let mut next_token: Option<String> = None;

                loop {
                    let mut request = client.list_configuration_profiles().application_id(app_id);
                    if let Some(token) = next_token {
                        request = request.next_token(token);
                    }

                    let response = request.send().await?;

                    if let Some(items) = response.items {
                        for profile in items {
                            let profile_json = self.configuration_profile_to_json(&profile, app_id);
                            configuration_profiles.push(profile_json);
                        }
                    }

                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
            }
        }

        Ok(configuration_profiles)
    }

    /// Describe AppConfig application
    pub async fn describe_application(
        &self,
        account: &str,
        region: &str,
        application_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account, region
                )
            })?;

        let client = appconfig::Client::new(&aws_config);

        let response = client
            .get_application()
            .application_id(application_id)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to describe AppConfig application: {}",
                    application_id
                )
            })?;

        Ok(self.application_detail_to_json(&response))
    }

    /// Describe AppConfig environment
    pub async fn describe_environment(
        &self,
        account: &str,
        region: &str,
        environment_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account, region
                )
            })?;

        let client = appconfig::Client::new(&aws_config);

        // Parse environment ID (format: app_id:env_id)
        let parts: Vec<&str> = environment_id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid environment ID format: {}",
                environment_id
            ));
        }

        let app_id = parts[0];
        let env_id = parts[1];

        let response = client
            .get_environment()
            .application_id(app_id)
            .environment_id(env_id)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to describe AppConfig environment: {}",
                    environment_id
                )
            })?;

        Ok(self.environment_detail_to_json(&response, app_id))
    }

    /// Describe AppConfig configuration profile
    pub async fn describe_configuration_profile(
        &self,
        account: &str,
        region: &str,
        profile_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account, region
                )
            })?;

        let client = appconfig::Client::new(&aws_config);

        // Parse profile ID (format: app_id:profile_id)
        let parts: Vec<&str> = profile_id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid configuration profile ID format: {}",
                profile_id
            ));
        }

        let app_id = parts[0];
        let config_profile_id = parts[1];

        let response = client
            .get_configuration_profile()
            .application_id(app_id)
            .configuration_profile_id(config_profile_id)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to describe AppConfig configuration profile: {}",
                    profile_id
                )
            })?;

        Ok(self.configuration_profile_detail_to_json(&response, app_id))
    }

    fn application_to_json(
        &self,
        application: &appconfig::types::Application,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &application.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(name) = &application.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &application.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn environment_to_json(
        &self,
        environment: &appconfig::types::Environment,
        application_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &environment.id {
            let env_id = format!("{}:{}", application_id, id);
            json.insert(
                "EnvironmentId".to_string(),
                serde_json::Value::String(env_id.clone()),
            );
            json.insert("ResourceId".to_string(), serde_json::Value::String(env_id));
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
        }

        json.insert(
            "ApplicationId".to_string(),
            serde_json::Value::String(application_id.to_string()),
        );

        if let Some(name) = &environment.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &environment.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(state) = &environment.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn configuration_profile_to_json(
        &self,
        profile: &appconfig::types::ConfigurationProfileSummary,
        application_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &profile.id {
            let profile_id = format!("{}:{}", application_id, id);
            json.insert(
                "ConfigurationProfileId".to_string(),
                serde_json::Value::String(profile_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(profile_id),
            );
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
        }

        json.insert(
            "ApplicationId".to_string(),
            serde_json::Value::String(application_id.to_string()),
        );

        if let Some(name) = &profile.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(location_uri) = &profile.location_uri {
            json.insert(
                "LocationUri".to_string(),
                serde_json::Value::String(location_uri.clone()),
            );
        }

        if let Some(validator_types) = &profile.validator_types {
            let validator_types_json: Vec<serde_json::Value> = validator_types
                .iter()
                .map(|vt| serde_json::Value::String(vt.as_str().to_string()))
                .collect();
            json.insert(
                "ValidatorTypes".to_string(),
                serde_json::Value::Array(validator_types_json),
            );
        }

        if let Some(r#type) = &profile.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(r#type.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn application_detail_to_json(
        &self,
        application: &appconfig::operation::get_application::GetApplicationOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &application.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(name) = &application.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &application.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn environment_detail_to_json(
        &self,
        environment: &appconfig::operation::get_environment::GetEnvironmentOutput,
        application_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &environment.id {
            let env_id = format!("{}:{}", application_id, id);
            json.insert(
                "EnvironmentId".to_string(),
                serde_json::Value::String(env_id.clone()),
            );
            json.insert("ResourceId".to_string(), serde_json::Value::String(env_id));
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
        }

        json.insert(
            "ApplicationId".to_string(),
            serde_json::Value::String(application_id.to_string()),
        );

        if let Some(name) = &environment.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &environment.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(state) = &environment.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(_monitors) = &environment.monitors {
            json.insert(
                "Monitors".to_string(),
                serde_json::Value::String("TODO: Manual conversion needed".to_string()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn configuration_profile_detail_to_json(
        &self,
        profile: &appconfig::operation::get_configuration_profile::GetConfigurationProfileOutput,
        application_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &profile.id {
            let profile_id = format!("{}:{}", application_id, id);
            json.insert(
                "ConfigurationProfileId".to_string(),
                serde_json::Value::String(profile_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(profile_id),
            );
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
        }

        json.insert(
            "ApplicationId".to_string(),
            serde_json::Value::String(application_id.to_string()),
        );

        if let Some(name) = &profile.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &profile.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(location_uri) = &profile.location_uri {
            json.insert(
                "LocationUri".to_string(),
                serde_json::Value::String(location_uri.clone()),
            );
        }

        if let Some(retrieval_role_arn) = &profile.retrieval_role_arn {
            json.insert(
                "RetrievalRoleArn".to_string(),
                serde_json::Value::String(retrieval_role_arn.clone()),
            );
        }

        if let Some(_validators) = &profile.validators {
            json.insert(
                "Validators".to_string(),
                serde_json::Value::String("TODO: Manual conversion needed".to_string()),
            );
        }

        if let Some(r#type) = &profile.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(r#type.clone()),
            );
        }

        if let Some(kms_key_identifier) = &profile.kms_key_identifier {
            json.insert(
                "KmsKeyIdentifier".to_string(),
                serde_json::Value::String(kms_key_identifier.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
