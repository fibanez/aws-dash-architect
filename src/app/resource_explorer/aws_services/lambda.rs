use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_lambda as lambda;
use std::sync::Arc;

pub struct LambdaService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl LambdaService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Lambda functions
    pub async fn list_functions(
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

        let client = lambda::Client::new(&aws_config);
        let mut paginator = client.list_functions().into_paginator().send();

        let mut functions = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(function_list) = page.functions {
                for function in function_list {
                    let function_json = self.function_to_json(&function);
                    functions.push(function_json);
                }
            }
        }

        Ok(functions)
    }

    /// Get detailed information for specific Lambda function
    pub async fn describe_function(
        &self,
        account_id: &str,
        region: &str,
        function_name: &str,
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

        let client = lambda::Client::new(&aws_config);
        let response = client
            .get_function()
            .function_name(function_name)
            .send()
            .await?;

        let mut function_details = serde_json::Map::new();

        if let Some(configuration) = response.configuration {
            let config_json = self.function_configuration_to_json(&configuration);
            function_details.insert("Configuration".to_string(), config_json);
        }

        if let Some(code) = response.code {
            let mut code_json = serde_json::Map::new();
            if let Some(repo_type) = code.repository_type {
                code_json.insert(
                    "RepositoryType".to_string(),
                    serde_json::Value::String(repo_type.clone()),
                );
            }
            if let Some(location) = code.location {
                code_json.insert("Location".to_string(), serde_json::Value::String(location));
            }
            function_details.insert("Code".to_string(), serde_json::Value::Object(code_json));
        }

        Ok(serde_json::Value::Object(function_details))
    }

    fn function_to_json(
        &self,
        function: &lambda::types::FunctionConfiguration,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(function_name) = &function.function_name {
            json.insert(
                "FunctionName".to_string(),
                serde_json::Value::String(function_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(function_name.clone()),
            );
        }

        if let Some(function_arn) = &function.function_arn {
            json.insert(
                "FunctionArn".to_string(),
                serde_json::Value::String(function_arn.clone()),
            );
        }

        if let Some(runtime) = &function.runtime {
            json.insert(
                "Runtime".to_string(),
                serde_json::Value::String(runtime.as_str().to_string()),
            );
        }

        if let Some(role) = &function.role {
            json.insert("Role".to_string(), serde_json::Value::String(role.clone()));
        }

        if let Some(handler) = &function.handler {
            json.insert(
                "Handler".to_string(),
                serde_json::Value::String(handler.clone()),
            );
        }

        if function.code_size > 0 {
            json.insert(
                "CodeSize".to_string(),
                serde_json::Value::Number(function.code_size.into()),
            );
        }

        if let Some(description) = &function.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(timeout) = function.timeout {
            json.insert(
                "Timeout".to_string(),
                serde_json::Value::Number(timeout.into()),
            );
        }

        if let Some(memory_size) = function.memory_size {
            json.insert(
                "MemorySize".to_string(),
                serde_json::Value::Number(memory_size.into()),
            );
        }

        if let Some(last_modified) = &function.last_modified {
            json.insert(
                "LastModified".to_string(),
                serde_json::Value::String(last_modified.clone()),
            );
        }

        if let Some(state) = &function.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn function_configuration_to_json(
        &self,
        config: &lambda::types::FunctionConfiguration,
    ) -> serde_json::Value {
        // Reuse the same conversion logic
        self.function_to_json(config)
    }
}
