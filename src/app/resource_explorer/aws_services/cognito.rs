use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cognitoidentity as cognito_identity;
use aws_sdk_cognitoidentityprovider as cognito_idp;
use std::sync::Arc;

pub struct CognitoService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CognitoService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Cognito User Pools
    pub async fn list_user_pools(
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

        let client = cognito_idp::Client::new(&aws_config);

        let mut user_pools = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.list_user_pools().max_results(60);
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(pools) = response.user_pools {
                for pool in pools {
                    if let Some(pool_id) = &pool.id {
                        // Get detailed pool information
                        if let Ok(pool_details) =
                            self.get_user_pool_internal(&client, pool_id).await
                        {
                            user_pools.push(pool_details);
                        } else {
                            // Fallback to basic pool info if describe fails
                            let pool_json = self.user_pool_summary_to_json(&pool);
                            user_pools.push(pool_json);
                        }
                    }
                }
            }

            if let Some(token) = response.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        Ok(user_pools)
    }

    /// List Cognito Identity Pools
    pub async fn list_identity_pools(
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

        let client = cognito_identity::Client::new(&aws_config);

        let mut identity_pools = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.list_identity_pools().max_results(60);
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(pools) = response.identity_pools {
                for pool in pools {
                    if let Some(pool_id) = &pool.identity_pool_id {
                        // Get detailed pool information
                        if let Ok(pool_details) =
                            self.get_identity_pool_internal(&client, pool_id).await
                        {
                            identity_pools.push(pool_details);
                        } else {
                            // Fallback to basic pool info if describe fails
                            let mut pool_json = serde_json::Map::new();
                            pool_json.insert(
                                "IdentityPoolId".to_string(),
                                serde_json::Value::String(pool_id.clone()),
                            );
                            if let Some(pool_name) = &pool.identity_pool_name {
                                pool_json.insert(
                                    "IdentityPoolName".to_string(),
                                    serde_json::Value::String(pool_name.clone()),
                                );
                                pool_json.insert(
                                    "Name".to_string(),
                                    serde_json::Value::String(pool_name.clone()),
                                );
                            }
                            identity_pools.push(serde_json::Value::Object(pool_json));
                        }
                    }
                }
            }

            if let Some(token) = response.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        Ok(identity_pools)
    }

    /// List User Pool Clients for a specific User Pool
    pub async fn list_user_pool_clients(
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

        let client = cognito_idp::Client::new(&aws_config);

        // First get all user pools, then get clients for each
        let mut all_clients = Vec::new();
        let pools_response = client.list_user_pools().max_results(60).send().await?;

        if let Some(pools) = pools_response.user_pools {
            for pool in pools {
                if let Some(pool_id) = &pool.id {
                    let mut next_token = None;

                    loop {
                        let mut request = client
                            .list_user_pool_clients()
                            .user_pool_id(pool_id)
                            .max_results(60);
                        if let Some(ref token) = next_token {
                            request = request.next_token(token);
                        }

                        let response = request.send().await?;

                        if let Some(clients) = response.user_pool_clients {
                            for client_summary in clients {
                                if let Some(client_id) = &client_summary.client_id {
                                    // Get detailed client information
                                    if let Ok(client_details) = self
                                        .get_user_pool_client_internal(&client, pool_id, client_id)
                                        .await
                                    {
                                        all_clients.push(client_details);
                                    } else {
                                        // Fallback to basic client info
                                        let mut client_json = serde_json::Map::new();
                                        client_json.insert(
                                            "ClientId".to_string(),
                                            serde_json::Value::String(client_id.clone()),
                                        );
                                        client_json.insert(
                                            "UserPoolId".to_string(),
                                            serde_json::Value::String(pool_id.clone()),
                                        );
                                        if let Some(client_name) = &client_summary.client_name {
                                            client_json.insert(
                                                "ClientName".to_string(),
                                                serde_json::Value::String(client_name.clone()),
                                            );
                                            client_json.insert(
                                                "Name".to_string(),
                                                serde_json::Value::String(client_name.clone()),
                                            );
                                        }
                                        all_clients.push(serde_json::Value::Object(client_json));
                                    }
                                }
                            }
                        }

                        if let Some(token) = response.next_token {
                            next_token = Some(token);
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(all_clients)
    }

    /// Get detailed information for specific User Pool
    pub async fn describe_user_pool(
        &self,
        account_id: &str,
        region: &str,
        user_pool_id: &str,
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

        let client = cognito_idp::Client::new(&aws_config);
        self.get_user_pool_internal(&client, user_pool_id).await
    }

    /// Get detailed information for specific Identity Pool
    pub async fn describe_identity_pool(
        &self,
        account_id: &str,
        region: &str,
        identity_pool_id: &str,
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

        let client = cognito_identity::Client::new(&aws_config);
        self.get_identity_pool_internal(&client, identity_pool_id)
            .await
    }

    /// Get detailed information for specific User Pool Client
    pub async fn describe_user_pool_client(
        &self,
        account_id: &str,
        region: &str,
        client_id: &str,
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

        let client = cognito_idp::Client::new(&aws_config);

        // We need to find the user pool ID for this client
        // This is a limitation of the API - we need the pool ID to describe a client
        // For now, we'll search through all pools
        let pools_response = client.list_user_pools().max_results(60).send().await?;

        if let Some(pools) = pools_response.user_pools {
            for pool in pools {
                if let Some(pool_id) = &pool.id {
                    if let Ok(client_details) = self
                        .get_user_pool_client_internal(&client, pool_id, client_id)
                        .await
                    {
                        return Ok(client_details);
                    }
                }
            }
        }

        Err(anyhow::anyhow!("User Pool Client {} not found", client_id))
    }

    async fn get_user_pool_internal(
        &self,
        client: &cognito_idp::Client,
        user_pool_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_user_pool()
            .user_pool_id(user_pool_id)
            .send()
            .await?;

        if let Some(user_pool) = response.user_pool {
            Ok(self.user_pool_to_json(&user_pool))
        } else {
            Err(anyhow::anyhow!("User Pool {} not found", user_pool_id))
        }
    }

    async fn get_identity_pool_internal(
        &self,
        client: &cognito_identity::Client,
        identity_pool_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_identity_pool()
            .identity_pool_id(identity_pool_id)
            .send()
            .await?;

        Ok(self.identity_pool_to_json(&response))
    }

    async fn get_user_pool_client_internal(
        &self,
        client: &cognito_idp::Client,
        user_pool_id: &str,
        client_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_user_pool_client()
            .user_pool_id(user_pool_id)
            .client_id(client_id)
            .send()
            .await?;

        if let Some(user_pool_client) = response.user_pool_client {
            Ok(self.user_pool_client_to_json(&user_pool_client, user_pool_id))
        } else {
            Err(anyhow::anyhow!("User Pool Client {} not found", client_id))
        }
    }

    fn user_pool_summary_to_json(
        &self,
        pool: &cognito_idp::types::UserPoolDescriptionType,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &pool.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(name) = &pool.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        // Note: status field is deprecated in the API
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        if let Some(creation_date) = &pool.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(last_modified_date) = &pool.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn user_pool_to_json(&self, pool: &cognito_idp::types::UserPoolType) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &pool.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(name) = &pool.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        // Note: status field is deprecated in the API
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        if let Some(creation_date) = &pool.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(last_modified_date) = &pool.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        if let Some(policies) = &pool.policies {
            let mut policies_json = serde_json::Map::new();
            if let Some(password_policy) = &policies.password_policy {
                let mut password_json = serde_json::Map::new();
                if let Some(min_length) = password_policy.minimum_length {
                    password_json.insert(
                        "MinimumLength".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(min_length)),
                    );
                }
                password_json.insert(
                    "RequireUppercase".to_string(),
                    serde_json::Value::Bool(password_policy.require_uppercase),
                );
                password_json.insert(
                    "RequireLowercase".to_string(),
                    serde_json::Value::Bool(password_policy.require_lowercase),
                );
                password_json.insert(
                    "RequireNumbers".to_string(),
                    serde_json::Value::Bool(password_policy.require_numbers),
                );
                password_json.insert(
                    "RequireSymbols".to_string(),
                    serde_json::Value::Bool(password_policy.require_symbols),
                );
                policies_json.insert(
                    "PasswordPolicy".to_string(),
                    serde_json::Value::Object(password_json),
                );
            }
            json.insert(
                "Policies".to_string(),
                serde_json::Value::Object(policies_json),
            );
        }

        // estimated_number_of_users is an i32, not Option<i32>
        json.insert(
            "EstimatedNumberOfUsers".to_string(),
            serde_json::Value::Number(serde_json::Number::from(pool.estimated_number_of_users)),
        );

        if let Some(email_configuration) = &pool.email_configuration {
            let mut email_json = serde_json::Map::new();
            if let Some(source_arn) = &email_configuration.source_arn {
                email_json.insert(
                    "SourceArn".to_string(),
                    serde_json::Value::String(source_arn.clone()),
                );
            }
            if let Some(reply_to_email_address) = &email_configuration.reply_to_email_address {
                email_json.insert(
                    "ReplyToEmailAddress".to_string(),
                    serde_json::Value::String(reply_to_email_address.clone()),
                );
            }
            if let Some(email_sending_account) = &email_configuration.email_sending_account {
                email_json.insert(
                    "EmailSendingAccount".to_string(),
                    serde_json::Value::String(format!("{:?}", email_sending_account)),
                );
            }
            json.insert(
                "EmailConfiguration".to_string(),
                serde_json::Value::Object(email_json),
            );
        }

        if let Some(sms_configuration) = &pool.sms_configuration {
            let mut sms_json = serde_json::Map::new();
            sms_json.insert(
                "SnsCallerArn".to_string(),
                serde_json::Value::String(sms_configuration.sns_caller_arn.clone()),
            );
            if let Some(external_id) = &sms_configuration.external_id {
                sms_json.insert(
                    "ExternalId".to_string(),
                    serde_json::Value::String(external_id.clone()),
                );
            }
            json.insert(
                "SmsConfiguration".to_string(),
                serde_json::Value::Object(sms_json),
            );
        }

        // Note: schema/attributes field structure changed in SDK - commenting out for now
        // Individual attributes need to be fetched separately via DescribeUserPoolAttributes
        json.insert(
            "AttributesNote".to_string(),
            serde_json::Value::String(
                "Use DescribeUserPoolAttributes for detailed schema".to_string(),
            ),
        );

        serde_json::Value::Object(json)
    }

    fn identity_pool_to_json(
        &self,
        pool: &cognito_identity::operation::describe_identity_pool::DescribeIdentityPoolOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "IdentityPoolId".to_string(),
            serde_json::Value::String(pool.identity_pool_id.clone()),
        );
        json.insert(
            "IdentityPoolName".to_string(),
            serde_json::Value::String(pool.identity_pool_name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(pool.identity_pool_name.clone()),
        );
        json.insert(
            "AllowUnauthenticatedIdentities".to_string(),
            serde_json::Value::Bool(pool.allow_unauthenticated_identities),
        );

        if let Some(allow_classic_flow) = pool.allow_classic_flow {
            json.insert(
                "AllowClassicFlow".to_string(),
                serde_json::Value::Bool(allow_classic_flow),
            );
        }

        if let Some(supported_login_providers) = &pool.supported_login_providers {
            let providers_json: serde_json::Map<String, serde_json::Value> =
                supported_login_providers
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect();
            json.insert(
                "SupportedLoginProviders".to_string(),
                serde_json::Value::Object(providers_json),
            );
        }

        if let Some(cognito_identity_providers) = &pool.cognito_identity_providers {
            let providers_array: Vec<serde_json::Value> = cognito_identity_providers
                .iter()
                .map(|provider| {
                    let mut provider_json = serde_json::Map::new();
                    if let Some(provider_name) = &provider.provider_name {
                        provider_json.insert(
                            "ProviderName".to_string(),
                            serde_json::Value::String(provider_name.clone()),
                        );
                    }
                    if let Some(client_id) = &provider.client_id {
                        provider_json.insert(
                            "ClientId".to_string(),
                            serde_json::Value::String(client_id.clone()),
                        );
                    }
                    if let Some(server_side_token_check) = provider.server_side_token_check {
                        provider_json.insert(
                            "ServerSideTokenCheck".to_string(),
                            serde_json::Value::Bool(server_side_token_check),
                        );
                    }
                    serde_json::Value::Object(provider_json)
                })
                .collect();
            json.insert(
                "CognitoIdentityProviders".to_string(),
                serde_json::Value::Array(providers_array),
            );
        }

        serde_json::Value::Object(json)
    }

    fn user_pool_client_to_json(
        &self,
        client: &cognito_idp::types::UserPoolClientType,
        user_pool_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "UserPoolId".to_string(),
            serde_json::Value::String(user_pool_id.to_string()),
        );

        if let Some(client_name) = &client.client_name {
            json.insert(
                "ClientName".to_string(),
                serde_json::Value::String(client_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(client_name.clone()),
            );
        }

        if let Some(client_id) = &client.client_id {
            json.insert(
                "ClientId".to_string(),
                serde_json::Value::String(client_id.clone()),
            );
        }

        // Note: generate_secret field not available in client details, access_token_validity is
        if let Some(access_token_validity) = client.access_token_validity {
            json.insert(
                "AccessTokenValidity".to_string(),
                serde_json::Value::Number(serde_json::Number::from(access_token_validity)),
            );
        }

        // refresh_token_validity is an i32, not Option<i32>
        json.insert(
            "RefreshTokenValidity".to_string(),
            serde_json::Value::Number(serde_json::Number::from(client.refresh_token_validity)),
        );

        if let Some(id_token_validity) = client.id_token_validity {
            json.insert(
                "IdTokenValidity".to_string(),
                serde_json::Value::Number(serde_json::Number::from(id_token_validity)),
            );
        }

        if let Some(creation_date) = &client.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(last_modified_date) = &client.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }
}
