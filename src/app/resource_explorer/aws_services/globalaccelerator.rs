use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_globalaccelerator as globalaccelerator;
use std::sync::Arc;

pub struct GlobalAcceleratorService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl GlobalAcceleratorService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List accelerators (basic list data)
    pub async fn list_accelerators(
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

        let client = globalaccelerator::Client::new(&aws_config);

        let mut paginator = client.list_accelerators().into_paginator().send();

        let mut accelerators = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(accelerator_list) = page.accelerators {
                for accelerator in accelerator_list {
                    let accelerator_json = self.accelerator_to_json(&accelerator);
                    accelerators.push(accelerator_json);
                }
            }
        }

        Ok(accelerators)
    }

    /// Get detailed information for specific accelerator (for describe functionality)
    pub async fn describe_accelerator(
        &self,
        account_id: &str,
        region: &str,
        accelerator_arn: &str,
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

        let client = globalaccelerator::Client::new(&aws_config);
        self.describe_accelerator_internal(&client, accelerator_arn)
            .await
    }

    async fn describe_accelerator_internal(
        &self,
        client: &globalaccelerator::Client,
        accelerator_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_accelerator()
            .accelerator_arn(accelerator_arn)
            .send()
            .await?;

        if let Some(accelerator) = response.accelerator {
            Ok(self.accelerator_details_to_json(&accelerator))
        } else {
            Err(anyhow::anyhow!("Accelerator {} not found", accelerator_arn))
        }
    }

    fn accelerator_to_json(
        &self,
        accelerator: &globalaccelerator::types::Accelerator,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(accelerator_arn) = &accelerator.accelerator_arn {
            json.insert(
                "AcceleratorArn".to_string(),
                serde_json::Value::String(accelerator_arn.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(accelerator_arn.clone()),
            );
        }

        if let Some(name) = &accelerator.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(status) = &accelerator.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        json.insert(
            "Enabled".to_string(),
            serde_json::Value::Bool(accelerator.enabled.unwrap_or(false)),
        );

        if let Some(ip_address_type) = &accelerator.ip_address_type {
            json.insert(
                "IpAddressType".to_string(),
                serde_json::Value::String(ip_address_type.as_str().to_string()),
            );
        }

        if let Some(ip_sets) = &accelerator.ip_sets {
            if !ip_sets.is_empty() {
                let ip_sets_json: Vec<serde_json::Value> = ip_sets
                    .iter()
                    .map(|ip_set| {
                        let mut ip_set_json = serde_json::Map::new();
                        if let Some(ip_address_family) = &ip_set.ip_address_family {
                            ip_set_json.insert(
                                "IpAddressFamily".to_string(),
                                serde_json::Value::String(ip_address_family.as_str().to_string()),
                            );
                        }
                        if let Some(ip_addresses) = &ip_set.ip_addresses {
                            if !ip_addresses.is_empty() {
                                let addresses: Vec<serde_json::Value> = ip_addresses
                                    .iter()
                                    .map(|addr| serde_json::Value::String(addr.clone()))
                                    .collect();
                                ip_set_json.insert(
                                    "IpAddresses".to_string(),
                                    serde_json::Value::Array(addresses),
                                );
                            }
                        }
                        serde_json::Value::Object(ip_set_json)
                    })
                    .collect();
                json.insert("IpSets".to_string(), serde_json::Value::Array(ip_sets_json));
            }
        }

        if let Some(dns_name) = &accelerator.dns_name {
            json.insert(
                "DnsName".to_string(),
                serde_json::Value::String(dns_name.clone()),
            );
        }

        if let Some(created_time) = accelerator.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        if let Some(last_modified_time) = accelerator.last_modified_time {
            json.insert(
                "LastModifiedTime".to_string(),
                serde_json::Value::String(last_modified_time.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn accelerator_details_to_json(
        &self,
        accelerator: &globalaccelerator::types::Accelerator,
    ) -> serde_json::Value {
        self.accelerator_to_json(accelerator)
    }
}
