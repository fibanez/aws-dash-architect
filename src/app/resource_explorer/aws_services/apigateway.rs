use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_apigateway as apigateway;
use std::sync::Arc;

pub struct ApiGatewayService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ApiGatewayService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List API Gateway REST APIs
    pub async fn list_rest_apis(
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

        let client = apigateway::Client::new(&aws_config);
        let mut apis = Vec::new();
        let mut position: Option<String> = None;

        loop {
            let mut request = client.get_rest_apis();
            if let Some(pos) = &position {
                request = request.position(pos);
            }

            let response = request.send().await?;

            if let Some(items) = response.items {
                for api in items {
                    let api_json = self.rest_api_to_json(&api);
                    apis.push(api_json);
                }
            }

            if response.position.is_some() {
                position = response.position;
            } else {
                break;
            }
        }

        Ok(apis)
    }

    /// Get detailed information for specific REST API
    pub async fn describe_rest_api(
        &self,
        account_id: &str,
        region: &str,
        rest_api_id: &str,
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

        let client = apigateway::Client::new(&aws_config);
        let response = client
            .get_rest_api()
            .rest_api_id(rest_api_id)
            .send()
            .await?;

        Ok(self.rest_api_output_to_json(&response))
    }

    fn rest_api_to_json(&self, api: &apigateway::types::RestApi) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(api_id) = &api.id {
            json.insert("Id".to_string(), serde_json::Value::String(api_id.clone()));
            json.insert(
                "RestApiId".to_string(),
                serde_json::Value::String(api_id.clone()),
            );
        }

        if let Some(name) = &api.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &api.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(created_date) = api.created_date {
            json.insert(
                "CreatedDate".to_string(),
                serde_json::Value::String(created_date.to_string()),
            );
        }

        if let Some(version) = &api.version {
            json.insert(
                "Version".to_string(),
                serde_json::Value::String(version.clone()),
            );
        }

        if let Some(warnings) = &api.warnings {
            let warnings_json: Vec<serde_json::Value> = warnings
                .iter()
                .map(|warning| serde_json::Value::String(warning.clone()))
                .collect();
            json.insert(
                "Warnings".to_string(),
                serde_json::Value::Array(warnings_json),
            );
        }

        if let Some(binary_media_types) = &api.binary_media_types {
            let media_types_json: Vec<serde_json::Value> = binary_media_types
                .iter()
                .map(|media_type| serde_json::Value::String(media_type.clone()))
                .collect();
            json.insert(
                "BinaryMediaTypes".to_string(),
                serde_json::Value::Array(media_types_json),
            );
        }

        if let Some(minimum_compression_size) = api.minimum_compression_size {
            json.insert(
                "MinimumCompressionSize".to_string(),
                serde_json::Value::Number(minimum_compression_size.into()),
            );
        }

        if let Some(api_key_source) = &api.api_key_source {
            json.insert(
                "ApiKeySource".to_string(),
                serde_json::Value::String(api_key_source.as_str().to_string()),
            );
        }

        if let Some(endpoint_configuration) = &api.endpoint_configuration {
            let mut endpoint_json = serde_json::Map::new();
            if let Some(types) = &endpoint_configuration.types {
                let types_json: Vec<serde_json::Value> = types
                    .iter()
                    .map(|endpoint_type| {
                        serde_json::Value::String(endpoint_type.as_str().to_string())
                    })
                    .collect();
                endpoint_json.insert("Types".to_string(), serde_json::Value::Array(types_json));
            }
            json.insert(
                "EndpointConfiguration".to_string(),
                serde_json::Value::Object(endpoint_json),
            );
        }

        if let Some(policy) = &api.policy {
            json.insert(
                "Policy".to_string(),
                serde_json::Value::String(policy.clone()),
            );
        }

        // Add a status field for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("AVAILABLE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn rest_api_output_to_json(
        &self,
        output: &apigateway::operation::get_rest_api::GetRestApiOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(api_id) = &output.id {
            json.insert("Id".to_string(), serde_json::Value::String(api_id.clone()));
            json.insert(
                "RestApiId".to_string(),
                serde_json::Value::String(api_id.clone()),
            );
        }

        if let Some(name) = &output.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &output.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(created_date) = output.created_date {
            json.insert(
                "CreatedDate".to_string(),
                serde_json::Value::String(created_date.to_string()),
            );
        }

        if let Some(version) = &output.version {
            json.insert(
                "Version".to_string(),
                serde_json::Value::String(version.clone()),
            );
        }

        // Add a status field for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("AVAILABLE".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
