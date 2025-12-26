use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_transfer as transfer;
use std::sync::Arc;

pub struct TransferService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl TransferService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Transfer Family servers
    pub async fn list_servers(
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

        let client = transfer::Client::new(&aws_config);

        let mut servers = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_servers();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }

            match request.send().await {
                Ok(response) => {
                    for server in response.servers {
                        let server_json = self.server_to_json(&server);
                        servers.push(server_json);
                    }

                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to list Transfer Family servers in account {} region {}: {}",
                        account_id,
                        region,
                        e
                    );
                    break;
                }
            }
        }

        Ok(servers)
    }

    /// Get detailed information for a specific Transfer Family server
    pub async fn describe_server(
        &self,
        account_id: &str,
        region: &str,
        server_id: &str,
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

        let client = transfer::Client::new(&aws_config);
        let response = client.describe_server().server_id(server_id).send().await?;

        if let Some(server) = response.server {
            Ok(self.server_details_to_json(&server))
        } else {
            Err(anyhow::anyhow!(
                "Transfer Family server {} not found",
                server_id
            ))
        }
    }

    /// List Transfer Family users for a server
    pub async fn list_users(
        &self,
        account_id: &str,
        region: &str,
        server_id: &str,
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

        let client = transfer::Client::new(&aws_config);

        let mut users = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_users().server_id(server_id);
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }

            match request.send().await {
                Ok(response) => {
                    for user in response.users {
                        let user_json = self.user_to_json(&user, server_id);
                        users.push(user_json);
                    }

                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to list Transfer Family users for server {} in account {} region {}: {}", server_id, account_id, region, e);
                    break;
                }
            }
        }

        Ok(users)
    }

    // JSON conversion methods - CRITICAL: Avoid serde_json::to_value for AWS SDK types
    fn server_to_json(&self, server: &transfer::types::ListedServer) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(server_id) = &server.server_id {
            json.insert(
                "ServerId".to_string(),
                serde_json::Value::String(server_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(server_id.clone()),
            );
        }

        json.insert(
            "Arn".to_string(),
            serde_json::Value::String(server.arn.clone()),
        );

        if let Some(domain) = &server.domain {
            json.insert(
                "Domain".to_string(),
                serde_json::Value::String(domain.as_str().to_string()),
            );
        }

        if let Some(identity_provider_type) = &server.identity_provider_type {
            json.insert(
                "IdentityProviderType".to_string(),
                serde_json::Value::String(identity_provider_type.as_str().to_string()),
            );
        }

        if let Some(endpoint_type) = &server.endpoint_type {
            json.insert(
                "EndpointType".to_string(),
                serde_json::Value::String(endpoint_type.as_str().to_string()),
            );
        }

        if let Some(state) = &server.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(user_count) = &server.user_count {
            json.insert(
                "UserCount".to_string(),
                serde_json::Value::Number(serde_json::Number::from(*user_count)),
            );
        }

        json.insert(
            "ResourceType".to_string(),
            serde_json::Value::String("AWS::Transfer::Server".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn server_details_to_json(
        &self,
        server: &transfer::types::DescribedServer,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(server_id) = &server.server_id {
            json.insert(
                "ServerId".to_string(),
                serde_json::Value::String(server_id.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(server_id.clone()),
            );
        }

        json.insert(
            "Arn".to_string(),
            serde_json::Value::String(server.arn.clone()),
        );

        if let Some(domain) = &server.domain {
            json.insert(
                "Domain".to_string(),
                serde_json::Value::String(domain.as_str().to_string()),
            );
        }

        if let Some(identity_provider_type) = &server.identity_provider_type {
            json.insert(
                "IdentityProviderType".to_string(),
                serde_json::Value::String(identity_provider_type.as_str().to_string()),
            );
        }

        if let Some(endpoint_type) = &server.endpoint_type {
            json.insert(
                "EndpointType".to_string(),
                serde_json::Value::String(endpoint_type.as_str().to_string()),
            );
        }

        if let Some(state) = &server.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(certificate) = &server.certificate {
            json.insert(
                "Certificate".to_string(),
                serde_json::Value::String(certificate.clone()),
            );
        }

        if let Some(logging_role) = &server.logging_role {
            json.insert(
                "LoggingRole".to_string(),
                serde_json::Value::String(logging_role.clone()),
            );
        }

        // Protocols
        if let Some(protocols) = &server.protocols {
            let protocols_array: Vec<serde_json::Value> = protocols
                .iter()
                .map(|protocol| serde_json::Value::String(protocol.as_str().to_string()))
                .collect();
            json.insert(
                "Protocols".to_string(),
                serde_json::Value::Array(protocols_array),
            );
        }

        // Security policy
        if let Some(security_policy_name) = &server.security_policy_name {
            json.insert(
                "SecurityPolicyName".to_string(),
                serde_json::Value::String(security_policy_name.clone()),
            );
        }

        // Host key fingerprint
        if let Some(host_key_fingerprint) = &server.host_key_fingerprint {
            json.insert(
                "HostKeyFingerprint".to_string(),
                serde_json::Value::String(host_key_fingerprint.clone()),
            );
        }

        // Endpoint details
        if let Some(endpoint_details) = &server.endpoint_details {
            let mut endpoint_json = serde_json::Map::new();

            if let Some(address_allocation_ids) = &endpoint_details.address_allocation_ids {
                let ids_array: Vec<serde_json::Value> = address_allocation_ids
                    .iter()
                    .map(|id| serde_json::Value::String(id.clone()))
                    .collect();
                endpoint_json.insert(
                    "AddressAllocationIds".to_string(),
                    serde_json::Value::Array(ids_array),
                );
            }

            if let Some(subnet_ids) = &endpoint_details.subnet_ids {
                let ids_array: Vec<serde_json::Value> = subnet_ids
                    .iter()
                    .map(|id| serde_json::Value::String(id.clone()))
                    .collect();
                endpoint_json.insert("SubnetIds".to_string(), serde_json::Value::Array(ids_array));
            }

            if let Some(vpc_id) = &endpoint_details.vpc_id {
                endpoint_json.insert(
                    "VpcId".to_string(),
                    serde_json::Value::String(vpc_id.clone()),
                );
            }

            if let Some(vpc_endpoint_id) = &endpoint_details.vpc_endpoint_id {
                endpoint_json.insert(
                    "VpcEndpointId".to_string(),
                    serde_json::Value::String(vpc_endpoint_id.clone()),
                );
            }

            if let Some(security_group_ids) = &endpoint_details.security_group_ids {
                let ids_array: Vec<serde_json::Value> = security_group_ids
                    .iter()
                    .map(|id| serde_json::Value::String(id.clone()))
                    .collect();
                endpoint_json.insert(
                    "SecurityGroupIds".to_string(),
                    serde_json::Value::Array(ids_array),
                );
            }

            json.insert(
                "EndpointDetails".to_string(),
                serde_json::Value::Object(endpoint_json),
            );
        }

        // Identity provider details
        if let Some(identity_provider_details) = &server.identity_provider_details {
            let mut idp_json = serde_json::Map::new();

            if let Some(url) = &identity_provider_details.url {
                idp_json.insert("Url".to_string(), serde_json::Value::String(url.clone()));
            }

            if let Some(invocation_role) = &identity_provider_details.invocation_role {
                idp_json.insert(
                    "InvocationRole".to_string(),
                    serde_json::Value::String(invocation_role.clone()),
                );
            }

            if let Some(directory_id) = &identity_provider_details.directory_id {
                idp_json.insert(
                    "DirectoryId".to_string(),
                    serde_json::Value::String(directory_id.clone()),
                );
            }

            json.insert(
                "IdentityProviderDetails".to_string(),
                serde_json::Value::Object(idp_json),
            );
        }

        // Tags
        if let Some(tags) = &server.tags {
            let tags_array: Vec<serde_json::Value> = tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    tag_json.insert(
                        "Key".to_string(),
                        serde_json::Value::String(tag.key.clone()),
                    );
                    tag_json.insert(
                        "Value".to_string(),
                        serde_json::Value::String(tag.value.clone()),
                    );
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Array(tags_array));
        }

        json.insert(
            "ResourceType".to_string(),
            serde_json::Value::String("AWS::Transfer::Server".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn user_to_json(
        &self,
        user: &transfer::types::ListedUser,
        server_id: &str,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(username) = &user.user_name {
            json.insert(
                "UserName".to_string(),
                serde_json::Value::String(username.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(format!("{}:{}", server_id, username)),
            );
        }

        json.insert(
            "ServerId".to_string(),
            serde_json::Value::String(server_id.to_string()),
        );

        json.insert(
            "Arn".to_string(),
            serde_json::Value::String(user.arn.clone()),
        );

        if let Some(home_directory) = &user.home_directory {
            json.insert(
                "HomeDirectory".to_string(),
                serde_json::Value::String(home_directory.clone()),
            );
        }

        if let Some(home_directory_type) = &user.home_directory_type {
            json.insert(
                "HomeDirectoryType".to_string(),
                serde_json::Value::String(home_directory_type.as_str().to_string()),
            );
        }

        if let Some(role) = &user.role {
            json.insert("Role".to_string(), serde_json::Value::String(role.clone()));
        }

        if let Some(ssh_public_key_count) = &user.ssh_public_key_count {
            json.insert(
                "SshPublicKeyCount".to_string(),
                serde_json::Value::Number(serde_json::Number::from(*ssh_public_key_count)),
            );
        }

        json.insert(
            "ResourceType".to_string(),
            serde_json::Value::String("AWS::Transfer::User".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
