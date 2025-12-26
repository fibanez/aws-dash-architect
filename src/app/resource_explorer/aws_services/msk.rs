use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_kafka as kafka;
use std::sync::Arc;

pub struct MskService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl MskService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List MSK clusters
    pub async fn list_clusters(
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

        let client = kafka::Client::new(&aws_config);
        let mut clusters = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_clusters_v2();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(cluster_info_list) = response.cluster_info_list {
                for cluster in cluster_info_list {
                    let cluster_json = self.cluster_to_json(&cluster);
                    clusters.push(cluster_json);
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(clusters)
    }

    /// Get detailed information for specific MSK cluster
    pub async fn describe_cluster(
        &self,
        account_id: &str,
        region: &str,
        cluster_arn: &str,
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

        let client = kafka::Client::new(&aws_config);
        self.describe_cluster_internal(&client, cluster_arn).await
    }

    async fn describe_cluster_internal(
        &self,
        client: &kafka::Client,
        cluster_arn: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_cluster_v2()
            .cluster_arn(cluster_arn)
            .send()
            .await?;

        if let Some(cluster_info) = response.cluster_info {
            Ok(self.cluster_detail_to_json(&cluster_info))
        } else {
            Err(anyhow::anyhow!("Cluster {} not found", cluster_arn))
        }
    }

    fn cluster_to_json(&self, cluster: &kafka::types::Cluster) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(cluster_arn) = &cluster.cluster_arn {
            json.insert(
                "ClusterArn".to_string(),
                serde_json::Value::String(cluster_arn.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(cluster_arn.clone()),
            );

            // Extract cluster name from ARN
            if let Some(cluster_name) = cluster_arn.split('/').nth(1) {
                json.insert(
                    "ClusterName".to_string(),
                    serde_json::Value::String(cluster_name.to_string()),
                );
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(cluster_name.to_string()),
                );
            }
        }

        if let Some(cluster_type) = &cluster.cluster_type {
            json.insert(
                "ClusterType".to_string(),
                serde_json::Value::String(cluster_type.as_str().to_string()),
            );
        }

        if let Some(creation_time) = cluster.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(current_version) = &cluster.current_version {
            json.insert(
                "CurrentVersion".to_string(),
                serde_json::Value::String(current_version.clone()),
            );
        }

        if let Some(state) = &cluster.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(state_info) = &cluster.state_info {
            let mut state_info_json = serde_json::Map::new();
            if let Some(code) = &state_info.code {
                state_info_json.insert("Code".to_string(), serde_json::Value::String(code.clone()));
            }
            if let Some(message) = &state_info.message {
                state_info_json.insert(
                    "Message".to_string(),
                    serde_json::Value::String(message.clone()),
                );
            }
            json.insert(
                "StateInfo".to_string(),
                serde_json::Value::Object(state_info_json),
            );
        }

        // Set default name if not available
        if !json.contains_key("Name") {
            json.insert(
                "Name".to_string(),
                serde_json::Value::String("unknown-cluster".to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn cluster_detail_to_json(&self, cluster: &kafka::types::Cluster) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(cluster_arn) = &cluster.cluster_arn {
            json.insert(
                "ClusterArn".to_string(),
                serde_json::Value::String(cluster_arn.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(cluster_arn.clone()),
            );

            // Extract cluster name from ARN
            if let Some(cluster_name) = cluster_arn.split('/').nth(1) {
                json.insert(
                    "ClusterName".to_string(),
                    serde_json::Value::String(cluster_name.to_string()),
                );
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(cluster_name.to_string()),
                );
            }
        }

        if let Some(cluster_type) = &cluster.cluster_type {
            json.insert(
                "ClusterType".to_string(),
                serde_json::Value::String(cluster_type.as_str().to_string()),
            );
        }

        if let Some(creation_time) = cluster.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(current_version) = &cluster.current_version {
            json.insert(
                "CurrentVersion".to_string(),
                serde_json::Value::String(current_version.clone()),
            );
        }

        if let Some(state) = &cluster.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(state_info) = &cluster.state_info {
            let mut state_info_json = serde_json::Map::new();
            if let Some(code) = &state_info.code {
                state_info_json.insert("Code".to_string(), serde_json::Value::String(code.clone()));
            }
            if let Some(message) = &state_info.message {
                state_info_json.insert(
                    "Message".to_string(),
                    serde_json::Value::String(message.clone()),
                );
            }
            json.insert(
                "StateInfo".to_string(),
                serde_json::Value::Object(state_info_json),
            );
        }

        if let Some(provisioned) = &cluster.provisioned {
            let mut provisioned_json = serde_json::Map::new();

            if let Some(broker_node_group_info) = &provisioned.broker_node_group_info {
                let mut broker_json = serde_json::Map::new();

                if let Some(instance_type) = &broker_node_group_info.instance_type {
                    broker_json.insert(
                        "InstanceType".to_string(),
                        serde_json::Value::String(instance_type.clone()),
                    );
                }

                if let Some(client_subnets) = &broker_node_group_info.client_subnets {
                    let subnets_json: Vec<serde_json::Value> = client_subnets
                        .iter()
                        .map(|subnet| serde_json::Value::String(subnet.clone()))
                        .collect();
                    broker_json.insert(
                        "ClientSubnets".to_string(),
                        serde_json::Value::Array(subnets_json),
                    );
                }

                provisioned_json.insert(
                    "BrokerNodeGroupInfo".to_string(),
                    serde_json::Value::Object(broker_json),
                );
            }

            // Note: kafka_version field not available in this SDK version

            if let Some(number_of_broker_nodes) = provisioned.number_of_broker_nodes {
                provisioned_json.insert(
                    "NumberOfBrokerNodes".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(number_of_broker_nodes)),
                );
            }

            json.insert(
                "Provisioned".to_string(),
                serde_json::Value::Object(provisioned_json),
            );
        }

        if let Some(serverless) = &cluster.serverless {
            let mut serverless_json = serde_json::Map::new();

            if let Some(vpc_configs) = &serverless.vpc_configs {
                let configs_json: Vec<serde_json::Value> = vpc_configs
                    .iter()
                    .map(|config| {
                        let mut config_json = serde_json::Map::new();
                        if let Some(subnet_ids) = &config.subnet_ids {
                            let subnets_json: Vec<serde_json::Value> = subnet_ids
                                .iter()
                                .map(|subnet| serde_json::Value::String(subnet.clone()))
                                .collect();
                            config_json.insert(
                                "SubnetIds".to_string(),
                                serde_json::Value::Array(subnets_json),
                            );
                        }
                        if let Some(security_group_ids) = &config.security_group_ids {
                            let sgs_json: Vec<serde_json::Value> = security_group_ids
                                .iter()
                                .map(|sg| serde_json::Value::String(sg.clone()))
                                .collect();
                            config_json.insert(
                                "SecurityGroupIds".to_string(),
                                serde_json::Value::Array(sgs_json),
                            );
                        }
                        serde_json::Value::Object(config_json)
                    })
                    .collect();
                serverless_json.insert(
                    "VpcConfigs".to_string(),
                    serde_json::Value::Array(configs_json),
                );
            }

            json.insert(
                "Serverless".to_string(),
                serde_json::Value::Object(serverless_json),
            );
        }

        // Set default name if not available
        if !json.contains_key("Name") {
            json.insert(
                "Name".to_string(),
                serde_json::Value::String("unknown-cluster".to_string()),
            );
        }

        serde_json::Value::Object(json)
    }
}
