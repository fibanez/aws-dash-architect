use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_apprunner as apprunner;
use std::sync::Arc;

pub struct AppRunnerService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AppRunnerService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List App Runner services
    pub async fn list_services(
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

        let client = apprunner::Client::new(&aws_config);

        let mut services = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_services();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }

            match request.send().await {
                Ok(response) => {
                    for service in response.service_summary_list {
                        let service_json = self.service_to_json(&service);
                        services.push(service_json);
                    }

                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to list App Runner services in account {} region {}: {}",
                        account_id,
                        region,
                        e
                    );
                    break;
                }
            }
        }

        Ok(services)
    }

    /// Get detailed information for a specific App Runner service
    pub async fn describe_service(
        &self,
        account_id: &str,
        region: &str,
        service_arn: &str,
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

        let client = apprunner::Client::new(&aws_config);
        let response = client
            .describe_service()
            .service_arn(service_arn)
            .send()
            .await?;

        if let Some(service) = response.service {
            Ok(self.service_details_to_json(&service))
        } else {
            Err(anyhow::anyhow!(
                "App Runner service {} not found",
                service_arn
            ))
        }
    }

    /// List App Runner connections
    pub async fn list_connections(
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

        let client = apprunner::Client::new(&aws_config);

        let mut connections = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_connections();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }

            match request.send().await {
                Ok(response) => {
                    for connection in response.connection_summary_list {
                        let connection_json = self.connection_to_json(&connection);
                        connections.push(connection_json);
                    }

                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to list App Runner connections in account {} region {}: {}",
                        account_id,
                        region,
                        e
                    );
                    break;
                }
            }
        }

        Ok(connections)
    }

    // JSON conversion methods - CRITICAL: Avoid serde_json::to_value for AWS SDK types
    fn service_to_json(&self, service: &apprunner::types::ServiceSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(service_name) = &service.service_name {
            json.insert(
                "ServiceName".to_string(),
                serde_json::Value::String(service_name.clone()),
            );
        }
        if let Some(service_id) = &service.service_id {
            json.insert(
                "ServiceId".to_string(),
                serde_json::Value::String(service_id.clone()),
            );
        }
        if let Some(service_arn) = &service.service_arn {
            json.insert(
                "ServiceArn".to_string(),
                serde_json::Value::String(service_arn.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(service_arn.clone()),
            );
        }

        if let Some(service_url) = &service.service_url {
            json.insert(
                "ServiceUrl".to_string(),
                serde_json::Value::String(service_url.clone()),
            );
        }

        if let Some(status) = &service.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(created_at) = &service.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(
                    created_at
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }
        if let Some(updated_at) = &service.updated_at {
            json.insert(
                "UpdatedAt".to_string(),
                serde_json::Value::String(
                    updated_at
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        json.insert(
            "ResourceType".to_string(),
            serde_json::Value::String("AWS::AppRunner::Service".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn service_details_to_json(&self, service: &apprunner::types::Service) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "ServiceName".to_string(),
            serde_json::Value::String(service.service_name.clone()),
        );
        json.insert(
            "ServiceId".to_string(),
            serde_json::Value::String(service.service_id.clone()),
        );
        json.insert(
            "ServiceArn".to_string(),
            serde_json::Value::String(service.service_arn.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(service.service_arn.clone()),
        );

        if let Some(service_url) = &service.service_url {
            json.insert(
                "ServiceUrl".to_string(),
                serde_json::Value::String(service_url.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String(service.status.as_str().to_string()),
        );

        json.insert(
            "CreatedAt".to_string(),
            serde_json::Value::String(
                service
                    .created_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );
        json.insert(
            "UpdatedAt".to_string(),
            serde_json::Value::String(
                service
                    .updated_at
                    .fmt(aws_smithy_types::date_time::Format::DateTime)
                    .unwrap_or_default(),
            ),
        );

        // Source configuration
        if let Some(source_configuration) = &service.source_configuration {
            let mut source_json = serde_json::Map::new();

            if let Some(code_repository) = &source_configuration.code_repository {
                let mut code_repo_json = serde_json::Map::new();

                code_repo_json.insert(
                    "RepositoryUrl".to_string(),
                    serde_json::Value::String(code_repository.repository_url.clone()),
                );

                if let Some(source_code_version) = &code_repository.source_code_version {
                    let mut version_json = serde_json::Map::new();
                    version_json.insert(
                        "Type".to_string(),
                        serde_json::Value::String(source_code_version.r#type.as_str().to_string()),
                    );
                    version_json.insert(
                        "Value".to_string(),
                        serde_json::Value::String(source_code_version.value.clone()),
                    );
                    code_repo_json.insert(
                        "SourceCodeVersion".to_string(),
                        serde_json::Value::Object(version_json),
                    );
                }

                source_json.insert(
                    "CodeRepository".to_string(),
                    serde_json::Value::Object(code_repo_json),
                );
            }

            if let Some(image_repository) = &source_configuration.image_repository {
                let mut image_repo_json = serde_json::Map::new();

                image_repo_json.insert(
                    "ImageIdentifier".to_string(),
                    serde_json::Value::String(image_repository.image_identifier.clone()),
                );

                image_repo_json.insert(
                    "ImageRepositoryType".to_string(),
                    serde_json::Value::String(
                        image_repository.image_repository_type.as_str().to_string(),
                    ),
                );

                source_json.insert(
                    "ImageRepository".to_string(),
                    serde_json::Value::Object(image_repo_json),
                );
            }

            if let Some(auto_deployments_enabled) = &source_configuration.auto_deployments_enabled {
                source_json.insert(
                    "AutoDeploymentsEnabled".to_string(),
                    serde_json::Value::Bool(*auto_deployments_enabled),
                );
            }

            json.insert(
                "SourceConfiguration".to_string(),
                serde_json::Value::Object(source_json),
            );
        }

        // Instance configuration
        if let Some(instance_configuration) = &service.instance_configuration {
            let mut instance_json = serde_json::Map::new();

            if let Some(cpu) = &instance_configuration.cpu {
                instance_json.insert("Cpu".to_string(), serde_json::Value::String(cpu.clone()));
            }

            if let Some(memory) = &instance_configuration.memory {
                instance_json.insert(
                    "Memory".to_string(),
                    serde_json::Value::String(memory.clone()),
                );
            }

            if let Some(instance_role_arn) = &instance_configuration.instance_role_arn {
                instance_json.insert(
                    "InstanceRoleArn".to_string(),
                    serde_json::Value::String(instance_role_arn.clone()),
                );
            }

            json.insert(
                "InstanceConfiguration".to_string(),
                serde_json::Value::Object(instance_json),
            );
        }

        // Health check configuration
        if let Some(health_check_configuration) = &service.health_check_configuration {
            let mut health_check_json = serde_json::Map::new();

            if let Some(protocol) = &health_check_configuration.protocol {
                health_check_json.insert(
                    "Protocol".to_string(),
                    serde_json::Value::String(protocol.as_str().to_string()),
                );
            }

            if let Some(path) = &health_check_configuration.path {
                health_check_json
                    .insert("Path".to_string(), serde_json::Value::String(path.clone()));
            }

            if let Some(interval) = &health_check_configuration.interval {
                health_check_json.insert(
                    "Interval".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(*interval)),
                );
            }

            if let Some(timeout) = &health_check_configuration.timeout {
                health_check_json.insert(
                    "Timeout".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(*timeout)),
                );
            }

            if let Some(healthy_threshold) = &health_check_configuration.healthy_threshold {
                health_check_json.insert(
                    "HealthyThreshold".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(*healthy_threshold)),
                );
            }

            if let Some(unhealthy_threshold) = &health_check_configuration.unhealthy_threshold {
                health_check_json.insert(
                    "UnhealthyThreshold".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(*unhealthy_threshold)),
                );
            }

            json.insert(
                "HealthCheckConfiguration".to_string(),
                serde_json::Value::Object(health_check_json),
            );
        }

        // Auto scaling configuration
        if let Some(auto_scaling_configuration_summary) =
            &service.auto_scaling_configuration_summary
        {
            let mut auto_scaling_json = serde_json::Map::new();

            if let Some(auto_scaling_configuration_arn) =
                &auto_scaling_configuration_summary.auto_scaling_configuration_arn
            {
                auto_scaling_json.insert(
                    "AutoScalingConfigurationArn".to_string(),
                    serde_json::Value::String(auto_scaling_configuration_arn.clone()),
                );
            }

            if let Some(auto_scaling_configuration_name) =
                &auto_scaling_configuration_summary.auto_scaling_configuration_name
            {
                auto_scaling_json.insert(
                    "AutoScalingConfigurationName".to_string(),
                    serde_json::Value::String(auto_scaling_configuration_name.clone()),
                );
            }

            auto_scaling_json.insert(
                "AutoScalingConfigurationRevision".to_string(),
                serde_json::Value::Number(serde_json::Number::from(
                    auto_scaling_configuration_summary.auto_scaling_configuration_revision,
                )),
            );

            json.insert(
                "AutoScalingConfigurationSummary".to_string(),
                serde_json::Value::Object(auto_scaling_json),
            );
        }

        json.insert(
            "ResourceType".to_string(),
            serde_json::Value::String("AWS::AppRunner::Service".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn connection_to_json(
        &self,
        connection: &apprunner::types::ConnectionSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(connection_name) = &connection.connection_name {
            json.insert(
                "ConnectionName".to_string(),
                serde_json::Value::String(connection_name.clone()),
            );
        }

        if let Some(connection_arn) = &connection.connection_arn {
            json.insert(
                "ConnectionArn".to_string(),
                serde_json::Value::String(connection_arn.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(connection_arn.clone()),
            );
        }

        if let Some(status) = &connection.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(provider_type) = &connection.provider_type {
            json.insert(
                "ProviderType".to_string(),
                serde_json::Value::String(provider_type.as_str().to_string()),
            );
        }

        if let Some(created_at) = &connection.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(
                    created_at
                        .fmt(aws_smithy_types::date_time::Format::DateTime)
                        .unwrap_or_default(),
                ),
            );
        }

        json.insert(
            "ResourceType".to_string(),
            serde_json::Value::String("AWS::AppRunner::Connection".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
