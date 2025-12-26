use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_connect as connect;
use std::sync::Arc;

pub struct ConnectService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ConnectService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Connect instances (basic list data)
    pub async fn list_instances(
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

        let client = connect::Client::new(&aws_config);

        let mut paginator = client.list_instances().into_paginator().send();

        let mut instances = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(instance_summary_list) = page.instance_summary_list {
                for instance_summary in instance_summary_list {
                    let instance_json = self.instance_summary_to_json(&instance_summary);
                    instances.push(instance_json);
                }
            }
        }

        Ok(instances)
    }

    /// Get detailed information for specific Connect instance (for describe functionality)
    pub async fn describe_instance(
        &self,
        account_id: &str,
        region: &str,
        instance_id: &str,
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

        let client = connect::Client::new(&aws_config);
        self.describe_instance_internal(&client, instance_id).await
    }

    async fn describe_instance_internal(
        &self,
        client: &connect::Client,
        instance_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_instance()
            .instance_id(instance_id)
            .send()
            .await?;

        if let Some(instance) = response.instance {
            Ok(self.instance_to_json(&instance))
        } else {
            Err(anyhow::anyhow!(
                "Connect instance {} not found",
                instance_id
            ))
        }
    }

    fn instance_summary_to_json(
        &self,
        instance_summary: &connect::types::InstanceSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &instance_summary.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(arn) = &instance_summary.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(identity_management_type) = &instance_summary.identity_management_type {
            json.insert(
                "IdentityManagementType".to_string(),
                serde_json::Value::String(identity_management_type.as_str().to_string()),
            );
        }

        if let Some(instance_alias) = &instance_summary.instance_alias {
            json.insert(
                "InstanceAlias".to_string(),
                serde_json::Value::String(instance_alias.clone()),
            );
        }

        if let Some(created_time) = instance_summary.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        if let Some(service_role) = &instance_summary.service_role {
            json.insert(
                "ServiceRole".to_string(),
                serde_json::Value::String(service_role.clone()),
            );
        }

        if let Some(instance_status) = &instance_summary.instance_status {
            json.insert(
                "InstanceStatus".to_string(),
                serde_json::Value::String(instance_status.as_str().to_string()),
            );
        }

        json.insert(
            "InboundCallsEnabled".to_string(),
            serde_json::Value::Bool(instance_summary.inbound_calls_enabled.unwrap_or(false)),
        );
        json.insert(
            "OutboundCallsEnabled".to_string(),
            serde_json::Value::Bool(instance_summary.outbound_calls_enabled.unwrap_or(false)),
        );

        if let Some(instance_access_url) = &instance_summary.instance_access_url {
            json.insert(
                "InstanceAccessUrl".to_string(),
                serde_json::Value::String(instance_access_url.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn instance_to_json(&self, instance: &connect::types::Instance) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &instance.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(arn) = &instance.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(identity_management_type) = &instance.identity_management_type {
            json.insert(
                "IdentityManagementType".to_string(),
                serde_json::Value::String(identity_management_type.as_str().to_string()),
            );
        }

        if let Some(instance_alias) = &instance.instance_alias {
            json.insert(
                "InstanceAlias".to_string(),
                serde_json::Value::String(instance_alias.clone()),
            );
        }

        if let Some(created_time) = instance.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        if let Some(service_role) = &instance.service_role {
            json.insert(
                "ServiceRole".to_string(),
                serde_json::Value::String(service_role.clone()),
            );
        }

        if let Some(instance_status) = &instance.instance_status {
            json.insert(
                "InstanceStatus".to_string(),
                serde_json::Value::String(instance_status.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(instance_status.as_str().to_string()),
            );
        }

        json.insert(
            "InboundCallsEnabled".to_string(),
            serde_json::Value::Bool(instance.inbound_calls_enabled.unwrap_or(false)),
        );
        json.insert(
            "OutboundCallsEnabled".to_string(),
            serde_json::Value::Bool(instance.outbound_calls_enabled.unwrap_or(false)),
        );

        if let Some(status_reason) = &instance.status_reason {
            let mut status_reason_json = serde_json::Map::new();
            if let Some(message) = &status_reason.message {
                status_reason_json.insert(
                    "Message".to_string(),
                    serde_json::Value::String(message.clone()),
                );
            }
            json.insert(
                "StatusReason".to_string(),
                serde_json::Value::Object(status_reason_json),
            );
        }

        if let Some(instance_access_url) = &instance.instance_access_url {
            json.insert(
                "InstanceAccessUrl".to_string(),
                serde_json::Value::String(instance_access_url.clone()),
            );
        }

        if let Some(tags) = &instance.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|(key, value)| {
                        let mut tag_json = serde_json::Map::new();
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                        tag_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }
}
