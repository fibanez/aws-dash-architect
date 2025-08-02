use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_redshift as redshift;
use std::sync::Arc;

pub struct RedshiftService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl RedshiftService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Redshift Clusters
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

        let client = redshift::Client::new(&aws_config);
        let mut paginator = client.describe_clusters().into_paginator().send();

        let mut clusters = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(cluster_list) = page.clusters {
                for cluster in cluster_list {
                    let cluster_json = self.cluster_to_json(&cluster);
                    clusters.push(cluster_json);
                }
            }
        }

        Ok(clusters)
    }

    /// Get detailed information for specific Redshift cluster
    pub async fn describe_cluster(
        &self,
        account_id: &str,
        region: &str,
        cluster_identifier: &str,
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

        let client = redshift::Client::new(&aws_config);
        let response = client
            .describe_clusters()
            .cluster_identifier(cluster_identifier)
            .send()
            .await?;

        if let Some(clusters) = response.clusters {
            if let Some(cluster) = clusters.first() {
                Ok(self.cluster_to_json(cluster))
            } else {
                Err(anyhow::anyhow!("Cluster {} not found", cluster_identifier))
            }
        } else {
            Err(anyhow::anyhow!("Cluster {} not found", cluster_identifier))
        }
    }

    fn cluster_to_json(&self, cluster: &redshift::types::Cluster) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(cluster_identifier) = &cluster.cluster_identifier {
            json.insert(
                "ClusterIdentifier".to_string(),
                serde_json::Value::String(cluster_identifier.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(cluster_identifier.clone()),
            );
        }

        if let Some(node_type) = &cluster.node_type {
            json.insert(
                "NodeType".to_string(),
                serde_json::Value::String(node_type.clone()),
            );
        }

        if let Some(cluster_status) = &cluster.cluster_status {
            json.insert(
                "ClusterStatus".to_string(),
                serde_json::Value::String(cluster_status.clone()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(cluster_status.clone()),
            );
        }

        if let Some(cluster_availability_status) = &cluster.cluster_availability_status {
            json.insert(
                "ClusterAvailabilityStatus".to_string(),
                serde_json::Value::String(cluster_availability_status.clone()),
            );
        }

        if let Some(modify_status) = &cluster.modify_status {
            json.insert(
                "ModifyStatus".to_string(),
                serde_json::Value::String(modify_status.clone()),
            );
        }

        if let Some(master_username) = &cluster.master_username {
            json.insert(
                "MasterUsername".to_string(),
                serde_json::Value::String(master_username.clone()),
            );
        }

        if let Some(db_name) = &cluster.db_name {
            json.insert(
                "DBName".to_string(),
                serde_json::Value::String(db_name.clone()),
            );
        }

        if let Some(endpoint) = &cluster.endpoint {
            let mut endpoint_json = serde_json::Map::new();
            if let Some(address) = &endpoint.address {
                endpoint_json.insert(
                    "Address".to_string(),
                    serde_json::Value::String(address.clone()),
                );
            }
            if let Some(port) = endpoint.port {
                endpoint_json.insert("Port".to_string(), serde_json::Value::Number(port.into()));
            }
            json.insert(
                "Endpoint".to_string(),
                serde_json::Value::Object(endpoint_json),
            );
        }

        if let Some(cluster_create_time) = cluster.cluster_create_time {
            json.insert(
                "ClusterCreateTime".to_string(),
                serde_json::Value::String(cluster_create_time.to_string()),
            );
        }

        if let Some(automated_snapshot_retention_period) =
            cluster.automated_snapshot_retention_period
        {
            json.insert(
                "AutomatedSnapshotRetentionPeriod".to_string(),
                serde_json::Value::Number(automated_snapshot_retention_period.into()),
            );
        }

        if let Some(manual_snapshot_retention_period) = cluster.manual_snapshot_retention_period {
            json.insert(
                "ManualSnapshotRetentionPeriod".to_string(),
                serde_json::Value::Number(manual_snapshot_retention_period.into()),
            );
        }

        if let Some(cluster_security_groups) = &cluster.cluster_security_groups {
            let security_groups_json: Vec<serde_json::Value> = cluster_security_groups
                .iter()
                .map(|sg| {
                    let mut sg_json = serde_json::Map::new();
                    if let Some(cluster_security_group_name) = &sg.cluster_security_group_name {
                        sg_json.insert(
                            "ClusterSecurityGroupName".to_string(),
                            serde_json::Value::String(cluster_security_group_name.clone()),
                        );
                    }
                    if let Some(status) = &sg.status {
                        sg_json.insert(
                            "Status".to_string(),
                            serde_json::Value::String(status.clone()),
                        );
                    }
                    serde_json::Value::Object(sg_json)
                })
                .collect();
            json.insert(
                "ClusterSecurityGroups".to_string(),
                serde_json::Value::Array(security_groups_json),
            );
        }

        if let Some(vpc_security_groups) = &cluster.vpc_security_groups {
            let vpc_sgs_json: Vec<serde_json::Value> = vpc_security_groups
                .iter()
                .map(|vpc_sg| {
                    let mut vpc_sg_json = serde_json::Map::new();
                    if let Some(vpc_security_group_id) = &vpc_sg.vpc_security_group_id {
                        vpc_sg_json.insert(
                            "VpcSecurityGroupId".to_string(),
                            serde_json::Value::String(vpc_security_group_id.clone()),
                        );
                    }
                    if let Some(status) = &vpc_sg.status {
                        vpc_sg_json.insert(
                            "Status".to_string(),
                            serde_json::Value::String(status.clone()),
                        );
                    }
                    serde_json::Value::Object(vpc_sg_json)
                })
                .collect();
            json.insert(
                "VpcSecurityGroups".to_string(),
                serde_json::Value::Array(vpc_sgs_json),
            );
        }

        if let Some(cluster_parameter_groups) = &cluster.cluster_parameter_groups {
            let param_groups_json: Vec<serde_json::Value> = cluster_parameter_groups
                .iter()
                .map(|pg| {
                    let mut pg_json = serde_json::Map::new();
                    if let Some(parameter_group_name) = &pg.parameter_group_name {
                        pg_json.insert(
                            "ParameterGroupName".to_string(),
                            serde_json::Value::String(parameter_group_name.clone()),
                        );
                    }
                    if let Some(parameter_apply_status) = &pg.parameter_apply_status {
                        pg_json.insert(
                            "ParameterApplyStatus".to_string(),
                            serde_json::Value::String(parameter_apply_status.clone()),
                        );
                    }
                    serde_json::Value::Object(pg_json)
                })
                .collect();
            json.insert(
                "ClusterParameterGroups".to_string(),
                serde_json::Value::Array(param_groups_json),
            );
        }

        if let Some(cluster_subnet_group_name) = &cluster.cluster_subnet_group_name {
            json.insert(
                "ClusterSubnetGroupName".to_string(),
                serde_json::Value::String(cluster_subnet_group_name.clone()),
            );
        }

        if let Some(vpc_id) = &cluster.vpc_id {
            json.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(availability_zone) = &cluster.availability_zone {
            json.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        if let Some(preferred_maintenance_window) = &cluster.preferred_maintenance_window {
            json.insert(
                "PreferredMaintenanceWindow".to_string(),
                serde_json::Value::String(preferred_maintenance_window.clone()),
            );
        }

        json.insert(
            "PubliclyAccessible".to_string(),
            serde_json::Value::Bool(cluster.publicly_accessible.unwrap_or(false)),
        );
        json.insert(
            "Encrypted".to_string(),
            serde_json::Value::Bool(cluster.encrypted.unwrap_or(false)),
        );

        if let Some(number_of_nodes) = cluster.number_of_nodes {
            json.insert(
                "NumberOfNodes".to_string(),
                serde_json::Value::Number(number_of_nodes.into()),
            );
        }

        serde_json::Value::Object(json)
    }
}
