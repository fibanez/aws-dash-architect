use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_mq as mq;
use std::sync::Arc;

pub struct MQService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl MQService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Amazon MQ brokers
    pub async fn list_brokers(
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

        let client = mq::Client::new(&aws_config);
        let response = client.list_brokers().send().await?;

        let mut brokers = Vec::new();
        if let Some(broker_summaries) = response.broker_summaries {
            for broker in broker_summaries {
                let broker_json = self.broker_to_json(&broker);
                brokers.push(broker_json);
            }
        }

        Ok(brokers)
    }

    /// Get detailed information for specific Amazon MQ broker
    pub async fn describe_broker(
        &self,
        account_id: &str,
        region: &str,
        broker_id: &str,
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

        let client = mq::Client::new(&aws_config);
        let response = client.describe_broker().broker_id(broker_id).send().await?;

        let mut broker_details = serde_json::Map::new();

        if let Some(broker_id) = response.broker_id {
            broker_details.insert("BrokerId".to_string(), serde_json::Value::String(broker_id));
        }

        if let Some(broker_name) = response.broker_name {
            broker_details.insert(
                "BrokerName".to_string(),
                serde_json::Value::String(broker_name),
            );
        }

        if let Some(broker_state) = response.broker_state {
            broker_details.insert(
                "BrokerState".to_string(),
                serde_json::Value::String(broker_state.as_str().to_string()),
            );
            broker_details.insert(
                "Status".to_string(),
                serde_json::Value::String(broker_state.as_str().to_string()),
            );
        }

        if let Some(engine_type) = response.engine_type {
            broker_details.insert(
                "EngineType".to_string(),
                serde_json::Value::String(engine_type.as_str().to_string()),
            );
        }

        if let Some(engine_version) = response.engine_version {
            broker_details.insert(
                "EngineVersion".to_string(),
                serde_json::Value::String(engine_version),
            );
        }

        if let Some(host_instance_type) = response.host_instance_type {
            broker_details.insert(
                "HostInstanceType".to_string(),
                serde_json::Value::String(host_instance_type),
            );
        }

        if let Some(deployment_mode) = response.deployment_mode {
            broker_details.insert(
                "DeploymentMode".to_string(),
                serde_json::Value::String(deployment_mode.as_str().to_string()),
            );
        }

        if let Some(auto_minor_version_upgrade) = response.auto_minor_version_upgrade {
            broker_details.insert(
                "AutoMinorVersionUpgrade".to_string(),
                serde_json::Value::Bool(auto_minor_version_upgrade),
            );
        }

        if let Some(publicly_accessible) = response.publicly_accessible {
            broker_details.insert(
                "PubliclyAccessible".to_string(),
                serde_json::Value::Bool(publicly_accessible),
            );
        }

        if let Some(subnet_ids) = response.subnet_ids {
            let subnet_ids_array: Vec<serde_json::Value> = subnet_ids
                .into_iter()
                .map(serde_json::Value::String)
                .collect();
            broker_details.insert(
                "SubnetIds".to_string(),
                serde_json::Value::Array(subnet_ids_array),
            );
        }

        if let Some(security_groups) = response.security_groups {
            let security_groups_array: Vec<serde_json::Value> = security_groups
                .into_iter()
                .map(serde_json::Value::String)
                .collect();
            broker_details.insert(
                "SecurityGroups".to_string(),
                serde_json::Value::Array(security_groups_array),
            );
        }

        if let Some(broker_arn) = response.broker_arn {
            broker_details.insert(
                "BrokerArn".to_string(),
                serde_json::Value::String(broker_arn),
            );
        }

        if let Some(created) = response.created {
            broker_details.insert(
                "Created".to_string(),
                serde_json::Value::String(created.to_string()),
            );
        }

        if let Some(tags) = response.tags {
            let tags_map: serde_json::Map<String, serde_json::Value> = tags
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            broker_details.insert("Tags".to_string(), serde_json::Value::Object(tags_map));
        }

        Ok(serde_json::Value::Object(broker_details))
    }

    fn broker_to_json(&self, broker: &mq::types::BrokerSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(broker_id) = &broker.broker_id {
            json.insert(
                "BrokerId".to_string(),
                serde_json::Value::String(broker_id.clone()),
            );
        }

        if let Some(broker_name) = &broker.broker_name {
            json.insert(
                "BrokerName".to_string(),
                serde_json::Value::String(broker_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(broker_name.clone()),
            );
        }

        if let Some(broker_state) = &broker.broker_state {
            json.insert(
                "BrokerState".to_string(),
                serde_json::Value::String(broker_state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(broker_state.as_str().to_string()),
            );
        }

        if let Some(engine_type) = &broker.engine_type {
            json.insert(
                "EngineType".to_string(),
                serde_json::Value::String(engine_type.as_str().to_string()),
            );
        }

        if let Some(host_instance_type) = &broker.host_instance_type {
            json.insert(
                "HostInstanceType".to_string(),
                serde_json::Value::String(host_instance_type.clone()),
            );
        }

        if let Some(deployment_mode) = &broker.deployment_mode {
            json.insert(
                "DeploymentMode".to_string(),
                serde_json::Value::String(deployment_mode.as_str().to_string()),
            );
        }

        if let Some(broker_arn) = &broker.broker_arn {
            json.insert(
                "BrokerArn".to_string(),
                serde_json::Value::String(broker_arn.clone()),
            );
            json.insert(
                "Arn".to_string(),
                serde_json::Value::String(broker_arn.clone()),
            );
        }

        if let Some(created) = broker.created {
            json.insert(
                "Created".to_string(),
                serde_json::Value::String(created.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }
}
