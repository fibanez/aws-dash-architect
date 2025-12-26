use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_lambda as lambda;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct LambdaService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl LambdaService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Lambda functions with optional detailed security information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic function info quickly.
    ///   If true (Phase 2), includes policy, concurrency, URL configs, and code signing.
    pub async fn list_functions(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("Lambda", "list_functions", Some(region));

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
                    let mut function_json = self.function_to_json(&function);

                    // Only fetch details if requested (Phase 2)
                    if include_details {
                        if let Some(function_name) = &function.function_name {
                            if let serde_json::Value::Object(ref mut details) = function_json {
                                // Get function resource policy
                                report_status("Lambda", "get_policy", Some(function_name));
                                match self
                                    .get_function_policy_internal(&client, function_name)
                                    .await
                                {
                                    Ok(policy) => {
                                        details.insert("ResourcePolicy".to_string(), policy);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get function policy for {}: {}",
                                            function_name,
                                            e
                                        );
                                    }
                                }

                                // Get function concurrency
                                report_status("Lambda", "get_concurrency", Some(function_name));
                                match self
                                    .get_function_concurrency_internal(&client, function_name)
                                    .await
                                {
                                    Ok(concurrency) => {
                                        details.insert("Concurrency".to_string(), concurrency);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get concurrency for {}: {}",
                                            function_name,
                                            e
                                        );
                                    }
                                }

                                // Get function URL configs
                                report_status("Lambda", "get_url_configs", Some(function_name));
                                match self
                                    .list_function_url_configs_internal(&client, function_name)
                                    .await
                                {
                                    Ok(url_configs) => {
                                        details.insert("UrlConfigs".to_string(), url_configs);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get URL configs for {}: {}",
                                            function_name,
                                            e
                                        );
                                    }
                                }

                                // Get code signing config
                                report_status("Lambda", "get_code_signing", Some(function_name));
                                match self
                                    .get_function_code_signing_internal(&client, function_name)
                                    .await
                                {
                                    Ok(code_signing) => {
                                        details.insert("CodeSigning".to_string(), code_signing);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get code signing for {}: {}",
                                            function_name,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }

                    functions.push(function_json);
                }
            }
        }

        report_status_done("Lambda", "list_functions", Some(region));
        Ok(functions)
    }

    /// Get security details for a single Lambda function (Phase 2 enrichment)
    ///
    /// This function fetches detailed security information for a single function,
    /// including resource policy, concurrency settings, URL configs, and code signing.
    /// Used for incremental detail fetching after the initial fast list.
    pub async fn get_function_details(
        &self,
        account_id: &str,
        region: &str,
        function_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("Lambda", "get_function_details", Some(function_name));

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
        let mut details = serde_json::Map::new();

        // Get function resource policy
        report_status("Lambda", "get_policy", Some(function_name));
        match self
            .get_function_policy_internal(&client, function_name)
            .await
        {
            Ok(policy) => {
                details.insert("ResourcePolicy".to_string(), policy);
            }
            Err(e) => {
                tracing::debug!("Could not get function policy for {}: {}", function_name, e);
            }
        }

        // Get function concurrency
        report_status("Lambda", "get_concurrency", Some(function_name));
        match self
            .get_function_concurrency_internal(&client, function_name)
            .await
        {
            Ok(concurrency) => {
                details.insert("Concurrency".to_string(), concurrency);
            }
            Err(e) => {
                tracing::debug!("Could not get concurrency for {}: {}", function_name, e);
            }
        }

        // Get function URL configs
        report_status("Lambda", "get_url_configs", Some(function_name));
        match self
            .list_function_url_configs_internal(&client, function_name)
            .await
        {
            Ok(url_configs) => {
                details.insert("UrlConfigs".to_string(), url_configs);
            }
            Err(e) => {
                tracing::debug!("Could not get URL configs for {}: {}", function_name, e);
            }
        }

        // Get code signing config
        report_status("Lambda", "get_code_signing", Some(function_name));
        match self
            .get_function_code_signing_internal(&client, function_name)
            .await
        {
            Ok(code_signing) => {
                details.insert("CodeSigning".to_string(), code_signing);
            }
            Err(e) => {
                tracing::debug!("Could not get code signing for {}: {}", function_name, e);
            }
        }

        report_status_done("Lambda", "get_function_details", Some(function_name));
        Ok(serde_json::Value::Object(details))
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

        // Get basic function info
        report_status("Lambda", "get_function", Some(function_name));
        let response = timeout(
            Duration::from_secs(10),
            client.get_function().function_name(function_name).send(),
        )
        .await
        .with_context(|| "get_function timed out")?
        .with_context(|| format!("Failed to get function {}", function_name))?;

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

        if let Some(tags) = response.tags {
            let tags_json: serde_json::Map<String, serde_json::Value> = tags
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            function_details.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        // Get function resource policy
        report_status("Lambda", "get_policy", Some(function_name));
        match self
            .get_function_policy(account_id, region, function_name)
            .await
        {
            Ok(policy) => {
                function_details.insert("ResourcePolicy".to_string(), policy);
            }
            Err(e) => {
                tracing::debug!("Could not get function policy: {}", e);
                function_details.insert(
                    "ResourcePolicy".to_string(),
                    serde_json::json!({
                        "Error": format!("{}", e)
                    }),
                );
            }
        }

        // Get function concurrency
        report_status("Lambda", "get_concurrency", Some(function_name));
        match self
            .get_function_concurrency(account_id, region, function_name)
            .await
        {
            Ok(concurrency) => {
                function_details.insert("Concurrency".to_string(), concurrency);
            }
            Err(e) => {
                tracing::debug!("Could not get function concurrency: {}", e);
            }
        }

        // Get function URL configs
        report_status("Lambda", "get_url_configs", Some(function_name));
        match self
            .list_function_url_configs(account_id, region, function_name)
            .await
        {
            Ok(url_configs) => {
                function_details.insert("UrlConfigs".to_string(), url_configs);
            }
            Err(e) => {
                tracing::debug!("Could not get function URL configs: {}", e);
            }
        }

        // Get code signing config
        report_status("Lambda", "get_code_signing", Some(function_name));
        match self
            .get_function_code_signing(account_id, region, function_name)
            .await
        {
            Ok(code_signing) => {
                function_details.insert("CodeSigning".to_string(), code_signing);
            }
            Err(e) => {
                tracing::debug!("Could not get code signing config: {}", e);
            }
        }

        report_status_done("Lambda", "describe_function", Some(function_name));
        Ok(serde_json::Value::Object(function_details))
    }

    /// Get function configuration details
    pub async fn get_function_configuration(
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
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_function_configuration()
                .function_name(function_name)
                .send(),
        )
        .await
        .with_context(|| "get_function_configuration timed out")?
        .with_context(|| format!("Failed to get configuration for function {}", function_name))?;

        // Build detailed JSON from FunctionConfiguration
        let mut json = serde_json::Map::new();

        if let Some(name) = &response.function_name {
            json.insert(
                "FunctionName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }
        if let Some(arn) = &response.function_arn {
            json.insert(
                "FunctionArn".to_string(),
                serde_json::Value::String(arn.clone()),
            );
        }
        if let Some(runtime) = &response.runtime {
            json.insert(
                "Runtime".to_string(),
                serde_json::Value::String(runtime.as_str().to_string()),
            );
        }
        if let Some(handler) = &response.handler {
            json.insert(
                "Handler".to_string(),
                serde_json::Value::String(handler.clone()),
            );
        }
        if let Some(role) = &response.role {
            json.insert("Role".to_string(), serde_json::Value::String(role.clone()));
        }
        if let Some(timeout_secs) = response.timeout {
            json.insert(
                "Timeout".to_string(),
                serde_json::Value::Number(timeout_secs.into()),
            );
        }
        if let Some(memory) = response.memory_size {
            json.insert(
                "MemorySize".to_string(),
                serde_json::Value::Number(memory.into()),
            );
        }
        if response.code_size > 0 {
            json.insert(
                "CodeSize".to_string(),
                serde_json::Value::Number(response.code_size.into()),
            );
        }
        if let Some(description) = &response.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }
        if let Some(last_modified) = &response.last_modified {
            json.insert(
                "LastModified".to_string(),
                serde_json::Value::String(last_modified.clone()),
            );
        }
        if let Some(code_sha) = &response.code_sha256 {
            json.insert(
                "CodeSha256".to_string(),
                serde_json::Value::String(code_sha.clone()),
            );
        }
        if let Some(version) = &response.version {
            json.insert(
                "Version".to_string(),
                serde_json::Value::String(version.clone()),
            );
        }
        if let Some(state) = &response.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }
        if let Some(state_reason) = &response.state_reason {
            json.insert(
                "StateReason".to_string(),
                serde_json::Value::String(state_reason.clone()),
            );
        }
        if let Some(state_reason_code) = &response.state_reason_code {
            json.insert(
                "StateReasonCode".to_string(),
                serde_json::Value::String(state_reason_code.as_str().to_string()),
            );
        }
        if let Some(package_type) = &response.package_type {
            json.insert(
                "PackageType".to_string(),
                serde_json::Value::String(package_type.as_str().to_string()),
            );
        }
        if let Some(ephemeral_storage) = &response.ephemeral_storage {
            json.insert(
                "EphemeralStorageSize".to_string(),
                serde_json::Value::Number(ephemeral_storage.size.into()),
            );
        }
        if let Some(architectures) = &response.architectures {
            let arch_array: Vec<serde_json::Value> = architectures
                .iter()
                .map(|a| serde_json::Value::String(a.as_str().to_string()))
                .collect();
            json.insert(
                "Architectures".to_string(),
                serde_json::Value::Array(arch_array),
            );
        }

        // VPC Configuration
        if let Some(vpc_config) = &response.vpc_config {
            let mut vpc_json = serde_json::Map::new();
            if let Some(vpc_id) = &vpc_config.vpc_id {
                vpc_json.insert(
                    "VpcId".to_string(),
                    serde_json::Value::String(vpc_id.clone()),
                );
            }
            if let Some(subnet_ids) = &vpc_config.subnet_ids {
                let subnets: Vec<serde_json::Value> = subnet_ids
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect();
                vpc_json.insert("SubnetIds".to_string(), serde_json::Value::Array(subnets));
            }
            if let Some(sg_ids) = &vpc_config.security_group_ids {
                let sgs: Vec<serde_json::Value> = sg_ids
                    .iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect();
                vpc_json.insert(
                    "SecurityGroupIds".to_string(),
                    serde_json::Value::Array(sgs),
                );
            }
            if !vpc_json.is_empty() {
                json.insert("VpcConfig".to_string(), serde_json::Value::Object(vpc_json));
            }
        }

        // Environment variables (keys only for security)
        if let Some(env) = &response.environment {
            if let Some(variables) = &env.variables {
                let env_keys: Vec<serde_json::Value> = variables
                    .keys()
                    .map(|k| serde_json::Value::String(k.clone()))
                    .collect();
                json.insert(
                    "EnvironmentVariableKeys".to_string(),
                    serde_json::Value::Array(env_keys),
                );
            }
        }

        // Layers
        if let Some(layers) = &response.layers {
            let layer_array: Vec<serde_json::Value> = layers
                .iter()
                .map(|layer| {
                    let mut layer_json = serde_json::Map::new();
                    if let Some(arn) = &layer.arn {
                        layer_json
                            .insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
                    }
                    if layer.code_size > 0 {
                        layer_json.insert(
                            "CodeSize".to_string(),
                            serde_json::Value::Number(layer.code_size.into()),
                        );
                    }
                    serde_json::Value::Object(layer_json)
                })
                .collect();
            json.insert("Layers".to_string(), serde_json::Value::Array(layer_array));
        }

        // Dead letter config
        if let Some(dlq) = &response.dead_letter_config {
            if let Some(target_arn) = &dlq.target_arn {
                let mut dlq_json = serde_json::Map::new();
                dlq_json.insert(
                    "TargetArn".to_string(),
                    serde_json::Value::String(target_arn.clone()),
                );
                json.insert(
                    "DeadLetterConfig".to_string(),
                    serde_json::Value::Object(dlq_json),
                );
            }
        }

        // Tracing config
        if let Some(tracing) = &response.tracing_config {
            if let Some(mode) = &tracing.mode {
                json.insert(
                    "TracingConfigMode".to_string(),
                    serde_json::Value::String(mode.as_str().to_string()),
                );
            }
        }

        // Logging config
        if let Some(logging) = &response.logging_config {
            let mut logging_json = serde_json::Map::new();
            if let Some(log_format) = &logging.log_format {
                logging_json.insert(
                    "LogFormat".to_string(),
                    serde_json::Value::String(log_format.as_str().to_string()),
                );
            }
            if let Some(log_group) = &logging.log_group {
                logging_json.insert(
                    "LogGroup".to_string(),
                    serde_json::Value::String(log_group.clone()),
                );
            }
            if let Some(app_level) = &logging.application_log_level {
                logging_json.insert(
                    "ApplicationLogLevel".to_string(),
                    serde_json::Value::String(app_level.as_str().to_string()),
                );
            }
            if let Some(sys_level) = &logging.system_log_level {
                logging_json.insert(
                    "SystemLogLevel".to_string(),
                    serde_json::Value::String(sys_level.as_str().to_string()),
                );
            }
            if !logging_json.is_empty() {
                json.insert(
                    "LoggingConfig".to_string(),
                    serde_json::Value::Object(logging_json),
                );
            }
        }

        Ok(serde_json::Value::Object(json))
    }

    /// Get function resource-based policy
    pub async fn get_function_policy(
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
        let response = timeout(
            Duration::from_secs(10),
            client.get_policy().function_name(function_name).send(),
        )
        .await
        .with_context(|| "get_policy timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();
                if let Some(policy) = result.policy {
                    // Try to parse the policy as JSON
                    if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&policy) {
                        json.insert("Policy".to_string(), policy_json);
                    } else {
                        json.insert("Policy".to_string(), serde_json::Value::String(policy));
                    }
                }
                if let Some(revision_id) = result.revision_id {
                    json.insert(
                        "RevisionId".to_string(),
                        serde_json::Value::String(revision_id),
                    );
                }
                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                // ResourceNotFoundException means no policy attached
                let error_str = format!("{:?}", e);
                if error_str.contains("ResourceNotFoundException") {
                    Ok(serde_json::json!({
                        "Policy": null,
                        "Note": "No resource policy attached"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get function policy: {}", e))
                }
            }
        }
    }

    /// Get function reserved concurrency setting
    pub async fn get_function_concurrency(
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
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_function_concurrency()
                .function_name(function_name)
                .send(),
        )
        .await
        .with_context(|| "get_function_concurrency timed out")?
        .with_context(|| format!("Failed to get concurrency for function {}", function_name))?;

        let mut json = serde_json::Map::new();
        if let Some(reserved) = response.reserved_concurrent_executions {
            json.insert(
                "ReservedConcurrentExecutions".to_string(),
                serde_json::Value::Number(reserved.into()),
            );
        } else {
            json.insert(
                "ReservedConcurrentExecutions".to_string(),
                serde_json::Value::Null,
            );
            json.insert(
                "Note".to_string(),
                serde_json::Value::String("No reserved concurrency configured".to_string()),
            );
        }
        Ok(serde_json::Value::Object(json))
    }

    /// List function URL configurations
    pub async fn list_function_url_configs(
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
        let response = timeout(
            Duration::from_secs(10),
            client
                .list_function_url_configs()
                .function_name(function_name)
                .send(),
        )
        .await
        .with_context(|| "list_function_url_configs timed out")?
        .with_context(|| format!("Failed to list URL configs for function {}", function_name))?;

        let mut url_configs = Vec::new();
        for config in &response.function_url_configs {
            let mut config_json = serde_json::Map::new();
            config_json.insert(
                "FunctionUrl".to_string(),
                serde_json::Value::String(config.function_url.clone()),
            );
            config_json.insert(
                "FunctionArn".to_string(),
                serde_json::Value::String(config.function_arn.clone()),
            );
            config_json.insert(
                "AuthType".to_string(),
                serde_json::Value::String(config.auth_type.as_str().to_string()),
            );
            config_json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(config.creation_time.clone()),
            );
            config_json.insert(
                "LastModifiedTime".to_string(),
                serde_json::Value::String(config.last_modified_time.clone()),
            );
            if let Some(invoke_mode) = &config.invoke_mode {
                config_json.insert(
                    "InvokeMode".to_string(),
                    serde_json::Value::String(invoke_mode.as_str().to_string()),
                );
            }

            // CORS configuration
            if let Some(cors) = &config.cors {
                let mut cors_json = serde_json::Map::new();
                if let Some(allow_credentials) = cors.allow_credentials {
                    cors_json.insert(
                        "AllowCredentials".to_string(),
                        serde_json::Value::Bool(allow_credentials),
                    );
                }
                if let Some(allow_headers) = &cors.allow_headers {
                    let headers: Vec<serde_json::Value> = allow_headers
                        .iter()
                        .map(|h| serde_json::Value::String(h.clone()))
                        .collect();
                    cors_json.insert(
                        "AllowHeaders".to_string(),
                        serde_json::Value::Array(headers),
                    );
                }
                if let Some(allow_methods) = &cors.allow_methods {
                    let methods: Vec<serde_json::Value> = allow_methods
                        .iter()
                        .map(|m| serde_json::Value::String(m.clone()))
                        .collect();
                    cors_json.insert(
                        "AllowMethods".to_string(),
                        serde_json::Value::Array(methods),
                    );
                }
                if let Some(allow_origins) = &cors.allow_origins {
                    let origins: Vec<serde_json::Value> = allow_origins
                        .iter()
                        .map(|o| serde_json::Value::String(o.clone()))
                        .collect();
                    cors_json.insert(
                        "AllowOrigins".to_string(),
                        serde_json::Value::Array(origins),
                    );
                }
                if let Some(expose_headers) = &cors.expose_headers {
                    let headers: Vec<serde_json::Value> = expose_headers
                        .iter()
                        .map(|h| serde_json::Value::String(h.clone()))
                        .collect();
                    cors_json.insert(
                        "ExposeHeaders".to_string(),
                        serde_json::Value::Array(headers),
                    );
                }
                if let Some(max_age) = cors.max_age {
                    cors_json.insert(
                        "MaxAge".to_string(),
                        serde_json::Value::Number(max_age.into()),
                    );
                }
                if !cors_json.is_empty() {
                    config_json.insert("Cors".to_string(), serde_json::Value::Object(cors_json));
                }
            }

            url_configs.push(serde_json::Value::Object(config_json));
        }

        Ok(serde_json::json!({
            "FunctionUrlConfigs": url_configs
        }))
    }

    /// Get function code signing configuration
    pub async fn get_function_code_signing(
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
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_function_code_signing_config()
                .function_name(function_name)
                .send(),
        )
        .await
        .with_context(|| "get_function_code_signing_config timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();
                json.insert(
                    "CodeSigningConfigArn".to_string(),
                    serde_json::Value::String(result.code_signing_config_arn.clone()),
                );
                json.insert(
                    "FunctionName".to_string(),
                    serde_json::Value::String(result.function_name.clone()),
                );
                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                // ResourceNotFoundException means no code signing configured
                let error_str = format!("{:?}", e);
                if error_str.contains("ResourceNotFoundException") {
                    Ok(serde_json::json!({
                        "CodeSigningConfigArn": null,
                        "Note": "No code signing configuration"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get code signing config: {}", e))
                }
            }
        }
    }

    // Internal versions that take a client reference for use in list_functions
    async fn get_function_policy_internal(
        &self,
        client: &lambda::Client,
        function_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.get_policy().function_name(function_name).send(),
        )
        .await
        .with_context(|| "get_policy timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();
                if let Some(policy) = result.policy {
                    if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&policy) {
                        json.insert("Policy".to_string(), policy_json);
                    } else {
                        json.insert("Policy".to_string(), serde_json::Value::String(policy));
                    }
                }
                if let Some(revision_id) = result.revision_id {
                    json.insert(
                        "RevisionId".to_string(),
                        serde_json::Value::String(revision_id),
                    );
                }
                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("ResourceNotFoundException") {
                    Ok(serde_json::json!({
                        "Policy": null,
                        "Note": "No resource policy attached"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get function policy: {}", e))
                }
            }
        }
    }

    async fn get_function_concurrency_internal(
        &self,
        client: &lambda::Client,
        function_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_function_concurrency()
                .function_name(function_name)
                .send(),
        )
        .await
        .with_context(|| "get_function_concurrency timed out")?
        .with_context(|| format!("Failed to get concurrency for function {}", function_name))?;

        let mut json = serde_json::Map::new();
        if let Some(reserved) = response.reserved_concurrent_executions {
            json.insert(
                "ReservedConcurrentExecutions".to_string(),
                serde_json::Value::Number(reserved.into()),
            );
        } else {
            json.insert(
                "ReservedConcurrentExecutions".to_string(),
                serde_json::Value::Null,
            );
            json.insert(
                "Note".to_string(),
                serde_json::Value::String("No reserved concurrency configured".to_string()),
            );
        }
        Ok(serde_json::Value::Object(json))
    }

    async fn list_function_url_configs_internal(
        &self,
        client: &lambda::Client,
        function_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .list_function_url_configs()
                .function_name(function_name)
                .send(),
        )
        .await
        .with_context(|| "list_function_url_configs timed out")?
        .with_context(|| format!("Failed to list URL configs for function {}", function_name))?;

        let mut url_configs = Vec::new();
        for config in &response.function_url_configs {
            let mut config_json = serde_json::Map::new();
            config_json.insert(
                "FunctionUrl".to_string(),
                serde_json::Value::String(config.function_url.clone()),
            );
            config_json.insert(
                "AuthType".to_string(),
                serde_json::Value::String(config.auth_type.as_str().to_string()),
            );
            if let Some(cors) = &config.cors {
                let mut cors_json = serde_json::Map::new();
                if let Some(allow_origins) = &cors.allow_origins {
                    let origins: Vec<serde_json::Value> = allow_origins
                        .iter()
                        .map(|o| serde_json::Value::String(o.clone()))
                        .collect();
                    cors_json.insert(
                        "AllowOrigins".to_string(),
                        serde_json::Value::Array(origins),
                    );
                }
                if !cors_json.is_empty() {
                    config_json.insert("Cors".to_string(), serde_json::Value::Object(cors_json));
                }
            }
            url_configs.push(serde_json::Value::Object(config_json));
        }

        Ok(serde_json::json!({
            "FunctionUrlConfigs": url_configs
        }))
    }

    async fn get_function_code_signing_internal(
        &self,
        client: &lambda::Client,
        function_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_function_code_signing_config()
                .function_name(function_name)
                .send(),
        )
        .await
        .with_context(|| "get_function_code_signing_config timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();
                json.insert(
                    "CodeSigningConfigArn".to_string(),
                    serde_json::Value::String(result.code_signing_config_arn.clone()),
                );
                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("ResourceNotFoundException") {
                    Ok(serde_json::json!({
                        "CodeSigningConfigArn": null,
                        "Note": "No code signing configuration"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get code signing config: {}", e))
                }
            }
        }
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
