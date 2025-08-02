use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_apigatewayv2 as apigatewayv2;
use std::sync::Arc;

pub struct ApiGatewayV2Service {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ApiGatewayV2Service {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List API Gateway v2 APIs (HTTP APIs)
    pub async fn list_apis(
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

        let client = apigatewayv2::Client::new(&aws_config);
        let mut apis = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.get_apis();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(items) = response.items {
                for api in items {
                    let api_json = self.api_to_json(&api);
                    apis.push(api_json);
                }
            }

            if response.next_token.is_some() {
                next_token = response.next_token;
            } else {
                break;
            }
        }

        Ok(apis)
    }

    /// Get detailed information for specific API
    pub async fn describe_api(
        &self,
        account_id: &str,
        region: &str,
        api_id: &str,
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

        let client = apigatewayv2::Client::new(&aws_config);
        let response = client.get_api().api_id(api_id).send().await?;

        Ok(self.api_output_to_json(&response))
    }

    fn api_to_json(&self, api: &apigatewayv2::types::Api) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(api_id) = &api.api_id {
            json.insert(
                "ApiId".to_string(),
                serde_json::Value::String(api_id.clone()),
            );
            json.insert("Id".to_string(), serde_json::Value::String(api_id.clone()));
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

        if let Some(protocol_type) = &api.protocol_type {
            json.insert(
                "ProtocolType".to_string(),
                serde_json::Value::String(protocol_type.as_str().to_string()),
            );
        }

        if let Some(route_selection_expression) = &api.route_selection_expression {
            json.insert(
                "RouteSelectionExpression".to_string(),
                serde_json::Value::String(route_selection_expression.clone()),
            );
        }

        if let Some(api_endpoint) = &api.api_endpoint {
            json.insert(
                "ApiEndpoint".to_string(),
                serde_json::Value::String(api_endpoint.clone()),
            );
        }

        if let Some(cors_configuration) = &api.cors_configuration {
            let mut cors_json = serde_json::Map::new();
            if let Some(allow_credentials) = cors_configuration.allow_credentials {
                cors_json.insert(
                    "AllowCredentials".to_string(),
                    serde_json::Value::Bool(allow_credentials),
                );
            }

            if let Some(allow_headers) = &cors_configuration.allow_headers {
                let headers_json: Vec<serde_json::Value> = allow_headers
                    .iter()
                    .map(|header| serde_json::Value::String(header.clone()))
                    .collect();
                cors_json.insert(
                    "AllowHeaders".to_string(),
                    serde_json::Value::Array(headers_json),
                );
            }

            if let Some(allow_methods) = &cors_configuration.allow_methods {
                let methods_json: Vec<serde_json::Value> = allow_methods
                    .iter()
                    .map(|method| serde_json::Value::String(method.clone()))
                    .collect();
                cors_json.insert(
                    "AllowMethods".to_string(),
                    serde_json::Value::Array(methods_json),
                );
            }

            if let Some(allow_origins) = &cors_configuration.allow_origins {
                let origins_json: Vec<serde_json::Value> = allow_origins
                    .iter()
                    .map(|origin| serde_json::Value::String(origin.clone()))
                    .collect();
                cors_json.insert(
                    "AllowOrigins".to_string(),
                    serde_json::Value::Array(origins_json),
                );
            }

            json.insert(
                "CorsConfiguration".to_string(),
                serde_json::Value::Object(cors_json),
            );
        }

        if let Some(tags) = &api.tags {
            let tags_json = serde_json::Map::from_iter(
                tags.iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))),
            );
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        // Add a status field for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("AVAILABLE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn api_output_to_json(
        &self,
        output: &apigatewayv2::operation::get_api::GetApiOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(api_id) = &output.api_id {
            json.insert(
                "ApiId".to_string(),
                serde_json::Value::String(api_id.clone()),
            );
            json.insert("Id".to_string(), serde_json::Value::String(api_id.clone()));
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
