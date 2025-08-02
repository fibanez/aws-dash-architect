use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_appsync as appsync;
use std::sync::Arc;

pub struct AppSyncService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AppSyncService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List AppSync GraphQL APIs
    pub async fn list_graphql_apis(
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

        let client = appsync::Client::new(&aws_config);
        let response = client.list_graphql_apis().send().await?;

        let mut graphql_apis = Vec::new();
        if let Some(graphql_apis_list) = response.graphql_apis {
            for graphql_api in graphql_apis_list {
                let graphql_api_json = self.graphql_api_to_json(&graphql_api);
                graphql_apis.push(graphql_api_json);
            }
        }

        Ok(graphql_apis)
    }

    /// Get detailed information for specific AppSync GraphQL API
    pub async fn describe_graphql_api(
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

        let client = appsync::Client::new(&aws_config);
        let response = client.get_graphql_api().api_id(api_id).send().await?;

        if let Some(graphql_api) = response.graphql_api {
            return Ok(self.graphql_api_to_json(&graphql_api));
        }

        Err(anyhow::anyhow!("GraphQL API {} not found", api_id))
    }

    fn graphql_api_to_json(&self, graphql_api: &appsync::types::GraphqlApi) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &graphql_api.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "ApiName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(api_id) = &graphql_api.api_id {
            json.insert(
                "ApiId".to_string(),
                serde_json::Value::String(api_id.clone()),
            );
        }

        if let Some(authentication_type) = &graphql_api.authentication_type {
            json.insert(
                "AuthenticationType".to_string(),
                serde_json::Value::String(authentication_type.as_str().to_string()),
            );
        }

        if let Some(log_config) = &graphql_api.log_config {
            let mut log_config_json = serde_json::Map::new();
            log_config_json.insert(
                "FieldLogLevel".to_string(),
                serde_json::Value::String(log_config.field_log_level.as_str().to_string()),
            );
            log_config_json.insert(
                "CloudWatchLogsRoleArn".to_string(),
                serde_json::Value::String(log_config.cloud_watch_logs_role_arn.clone()),
            );
            json.insert(
                "LogConfig".to_string(),
                serde_json::Value::Object(log_config_json),
            );
        }

        if let Some(user_pool_config) = &graphql_api.user_pool_config {
            let mut user_pool_config_json = serde_json::Map::new();
            user_pool_config_json.insert(
                "UserPoolId".to_string(),
                serde_json::Value::String(user_pool_config.user_pool_id.clone()),
            );
            user_pool_config_json.insert(
                "AwsRegion".to_string(),
                serde_json::Value::String(user_pool_config.aws_region.clone()),
            );
            user_pool_config_json.insert(
                "DefaultAction".to_string(),
                serde_json::Value::String(user_pool_config.default_action.as_str().to_string()),
            );
            json.insert(
                "UserPoolConfig".to_string(),
                serde_json::Value::Object(user_pool_config_json),
            );
        }

        if let Some(open_id_connect_config) = &graphql_api.open_id_connect_config {
            let mut oidc_config_json = serde_json::Map::new();
            oidc_config_json.insert(
                "Issuer".to_string(),
                serde_json::Value::String(open_id_connect_config.issuer.clone()),
            );
            if let Some(client_id) = &open_id_connect_config.client_id {
                oidc_config_json.insert(
                    "ClientId".to_string(),
                    serde_json::Value::String(client_id.clone()),
                );
            }
            oidc_config_json.insert(
                "IatTTL".to_string(),
                serde_json::Value::Number(open_id_connect_config.iat_ttl.into()),
            );
            oidc_config_json.insert(
                "AuthTTL".to_string(),
                serde_json::Value::Number(open_id_connect_config.auth_ttl.into()),
            );
            json.insert(
                "OpenIDConnectConfig".to_string(),
                serde_json::Value::Object(oidc_config_json),
            );
        }

        if let Some(arn) = &graphql_api.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(uris) = &graphql_api.uris {
            let uris_map: serde_json::Map<String, serde_json::Value> = uris
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Uris".to_string(), serde_json::Value::Object(uris_map));
        }

        if let Some(tags) = &graphql_api.tags {
            let tags_map: serde_json::Map<String, serde_json::Value> = tags
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_map));
        }

        if let Some(additional_authentication_providers) =
            &graphql_api.additional_authentication_providers
        {
            let auth_providers: Vec<serde_json::Value> = additional_authentication_providers
                .iter()
                .map(|provider| {
                    let mut provider_json = serde_json::Map::new();
                    if let Some(auth_type) = &provider.authentication_type {
                        provider_json.insert(
                            "AuthenticationType".to_string(),
                            serde_json::Value::String(auth_type.as_str().to_string()),
                        );
                    }
                    if let Some(lambda_authorizer_config) = &provider.lambda_authorizer_config {
                        let mut lambda_config_json = serde_json::Map::new();
                        lambda_config_json.insert(
                            "AuthorizerResultTtlInSeconds".to_string(),
                            serde_json::Value::Number(
                                lambda_authorizer_config
                                    .authorizer_result_ttl_in_seconds
                                    .into(),
                            ),
                        );
                        lambda_config_json.insert(
                            "AuthorizerUri".to_string(),
                            serde_json::Value::String(
                                lambda_authorizer_config.authorizer_uri.clone(),
                            ),
                        );
                        provider_json.insert(
                            "LambdaAuthorizerConfig".to_string(),
                            serde_json::Value::Object(lambda_config_json),
                        );
                    }
                    serde_json::Value::Object(provider_json)
                })
                .collect();
            json.insert(
                "AdditionalAuthenticationProviders".to_string(),
                serde_json::Value::Array(auth_providers),
            );
        }

        json.insert(
            "XrayEnabled".to_string(),
            serde_json::Value::Bool(graphql_api.xray_enabled),
        );

        if let Some(waf_web_acl_arn) = &graphql_api.waf_web_acl_arn {
            json.insert(
                "WafWebAclArn".to_string(),
                serde_json::Value::String(waf_web_acl_arn.clone()),
            );
        }

        if let Some(dns) = &graphql_api.dns {
            let dns_map: serde_json::Map<String, serde_json::Value> = dns
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Dns".to_string(), serde_json::Value::Object(dns_map));
        }

        if let Some(visibility) = &graphql_api.visibility {
            json.insert(
                "Visibility".to_string(),
                serde_json::Value::String(visibility.as_str().to_string()),
            );
        }

        if let Some(api_type) = &graphql_api.api_type {
            json.insert(
                "ApiType".to_string(),
                serde_json::Value::String(api_type.as_str().to_string()),
            );
        }

        if let Some(merged_api_execution_role_arn) = &graphql_api.merged_api_execution_role_arn {
            json.insert(
                "MergedApiExecutionRoleArn".to_string(),
                serde_json::Value::String(merged_api_execution_role_arn.clone()),
            );
        }

        if let Some(owner) = &graphql_api.owner {
            json.insert(
                "Owner".to_string(),
                serde_json::Value::String(owner.clone()),
            );
        }

        if let Some(owner_contact) = &graphql_api.owner_contact {
            json.insert(
                "OwnerContact".to_string(),
                serde_json::Value::String(owner_contact.clone()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
