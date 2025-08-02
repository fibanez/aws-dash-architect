use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_neptune as neptune;
use std::sync::Arc;

pub struct NeptuneService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl NeptuneService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Neptune DB Clusters
    pub async fn list_db_clusters(
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

        let client = neptune::Client::new(&aws_config);

        let mut db_clusters = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.describe_db_clusters();
            if let Some(ref marker_value) = marker {
                request = request.marker(marker_value);
            }

            let response = request.send().await?;

            if let Some(clusters) = response.db_clusters {
                for cluster in clusters {
                    let cluster_json = self.db_cluster_to_json(&cluster);
                    db_clusters.push(cluster_json);
                }
            }

            if let Some(next_marker) = response.marker {
                marker = Some(next_marker);
            } else {
                break;
            }
        }

        Ok(db_clusters)
    }

    /// List Neptune DB Instances
    pub async fn list_db_instances(
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

        let client = neptune::Client::new(&aws_config);

        let mut db_instances = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.describe_db_instances();
            if let Some(ref marker_value) = marker {
                request = request.marker(marker_value);
            }

            let response = request.send().await?;

            if let Some(instances) = response.db_instances {
                for instance in instances {
                    let instance_json = self.db_instance_to_json(&instance);
                    db_instances.push(instance_json);
                }
            }

            if let Some(next_marker) = response.marker {
                marker = Some(next_marker);
            } else {
                break;
            }
        }

        Ok(db_instances)
    }

    /// Get detailed information for specific Neptune DB Cluster
    pub async fn describe_db_cluster(
        &self,
        account_id: &str,
        region: &str,
        cluster_id: &str,
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

        let client = neptune::Client::new(&aws_config);

        let response = client
            .describe_db_clusters()
            .db_cluster_identifier(cluster_id)
            .send()
            .await?;

        if let Some(clusters) = response.db_clusters {
            if let Some(cluster) = clusters.first() {
                Ok(self.db_cluster_to_json(cluster))
            } else {
                Err(anyhow::anyhow!(
                    "Neptune DB cluster {} not found",
                    cluster_id
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "Neptune DB cluster {} not found",
                cluster_id
            ))
        }
    }

    /// Get detailed information for specific Neptune DB Instance
    pub async fn describe_db_instance(
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

        let client = neptune::Client::new(&aws_config);

        let response = client
            .describe_db_instances()
            .db_instance_identifier(instance_id)
            .send()
            .await?;

        if let Some(instances) = response.db_instances {
            if let Some(instance) = instances.first() {
                Ok(self.db_instance_to_json(instance))
            } else {
                Err(anyhow::anyhow!(
                    "Neptune DB instance {} not found",
                    instance_id
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "Neptune DB instance {} not found",
                instance_id
            ))
        }
    }

    fn db_cluster_to_json(&self, cluster: &neptune::types::DbCluster) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(cluster_id) = &cluster.db_cluster_identifier {
            json.insert(
                "DBClusterIdentifier".to_string(),
                serde_json::Value::String(cluster_id.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(cluster_id.clone()),
            );
        }

        if let Some(status) = &cluster.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(engine) = &cluster.engine {
            json.insert(
                "Engine".to_string(),
                serde_json::Value::String(engine.clone()),
            );
        }

        if let Some(engine_version) = &cluster.engine_version {
            json.insert(
                "EngineVersion".to_string(),
                serde_json::Value::String(engine_version.clone()),
            );
        }

        if let Some(endpoint) = &cluster.endpoint {
            json.insert(
                "Endpoint".to_string(),
                serde_json::Value::String(endpoint.clone()),
            );
        }

        if let Some(reader_endpoint) = &cluster.reader_endpoint {
            json.insert(
                "ReaderEndpoint".to_string(),
                serde_json::Value::String(reader_endpoint.clone()),
            );
        }

        if let Some(port) = cluster.port {
            json.insert(
                "Port".to_string(),
                serde_json::Value::Number(serde_json::Number::from(port)),
            );
        }

        if let Some(master_username) = &cluster.master_username {
            json.insert(
                "MasterUsername".to_string(),
                serde_json::Value::String(master_username.clone()),
            );
        }

        if let Some(db_cluster_members) = &cluster.db_cluster_members {
            let members_array: Vec<serde_json::Value> = db_cluster_members
                .iter()
                .map(|member| {
                    let mut member_json = serde_json::Map::new();
                    if let Some(instance_id) = &member.db_instance_identifier {
                        member_json.insert(
                            "DBInstanceIdentifier".to_string(),
                            serde_json::Value::String(instance_id.clone()),
                        );
                    }
                    if let Some(is_writer) = member.is_cluster_writer {
                        member_json.insert(
                            "IsClusterWriter".to_string(),
                            serde_json::Value::Bool(is_writer),
                        );
                    }
                    if let Some(promotion_tier) = member.promotion_tier {
                        member_json.insert(
                            "PromotionTier".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(promotion_tier)),
                        );
                    }
                    serde_json::Value::Object(member_json)
                })
                .collect();
            json.insert(
                "DBClusterMembers".to_string(),
                serde_json::Value::Array(members_array),
            );
        }

        if let Some(availability_zones) = &cluster.availability_zones {
            json.insert(
                "AvailabilityZones".to_string(),
                serde_json::Value::Array(
                    availability_zones
                        .iter()
                        .map(|az| serde_json::Value::String(az.clone()))
                        .collect(),
                ),
            );
        }

        if let Some(backup_retention_period) = cluster.backup_retention_period {
            json.insert(
                "BackupRetentionPeriod".to_string(),
                serde_json::Value::Number(serde_json::Number::from(backup_retention_period)),
            );
        }

        if let Some(preferred_backup_window) = &cluster.preferred_backup_window {
            json.insert(
                "PreferredBackupWindow".to_string(),
                serde_json::Value::String(preferred_backup_window.clone()),
            );
        }

        if let Some(preferred_maintenance_window) = &cluster.preferred_maintenance_window {
            json.insert(
                "PreferredMaintenanceWindow".to_string(),
                serde_json::Value::String(preferred_maintenance_window.clone()),
            );
        }

        if let Some(vpc_security_groups) = &cluster.vpc_security_groups {
            let sg_array: Vec<serde_json::Value> = vpc_security_groups
                .iter()
                .filter_map(|sg| sg.vpc_security_group_id.as_ref())
                .map(|sg_id| serde_json::Value::String(sg_id.clone()))
                .collect();
            json.insert(
                "VpcSecurityGroups".to_string(),
                serde_json::Value::Array(sg_array),
            );
        }

        if let Some(db_subnet_group) = &cluster.db_subnet_group {
            json.insert(
                "DBSubnetGroup".to_string(),
                serde_json::Value::String(db_subnet_group.clone()),
            );
        }

        if let Some(cluster_create_time) = &cluster.cluster_create_time {
            json.insert(
                "ClusterCreateTime".to_string(),
                serde_json::Value::String(cluster_create_time.to_string()),
            );
        }

        if let Some(storage_encrypted) = cluster.storage_encrypted {
            json.insert(
                "StorageEncrypted".to_string(),
                serde_json::Value::Bool(storage_encrypted),
            );
        }

        if let Some(kms_key_id) = &cluster.kms_key_id {
            json.insert(
                "KmsKeyId".to_string(),
                serde_json::Value::String(kms_key_id.clone()),
            );
        }

        if let Some(deletion_protection) = cluster.deletion_protection {
            json.insert(
                "DeletionProtection".to_string(),
                serde_json::Value::Bool(deletion_protection),
            );
        }

        serde_json::Value::Object(json)
    }

    fn db_instance_to_json(&self, instance: &neptune::types::DbInstance) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(instance_id) = &instance.db_instance_identifier {
            json.insert(
                "DBInstanceIdentifier".to_string(),
                serde_json::Value::String(instance_id.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(instance_id.clone()),
            );
        }

        if let Some(instance_class) = &instance.db_instance_class {
            json.insert(
                "DBInstanceClass".to_string(),
                serde_json::Value::String(instance_class.clone()),
            );
        }

        if let Some(engine) = &instance.engine {
            json.insert(
                "Engine".to_string(),
                serde_json::Value::String(engine.clone()),
            );
        }

        if let Some(instance_status) = &instance.db_instance_status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(instance_status.clone()),
            );
        }

        if let Some(endpoint) = &instance.endpoint {
            if let Some(address) = &endpoint.address {
                json.insert(
                    "Endpoint".to_string(),
                    serde_json::Value::String(address.clone()),
                );
            }
            if let Some(port) = endpoint.port {
                json.insert(
                    "Port".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(port)),
                );
            }
        }

        if let Some(availability_zone) = &instance.availability_zone {
            json.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        if let Some(instance_create_time) = &instance.instance_create_time {
            json.insert(
                "InstanceCreateTime".to_string(),
                serde_json::Value::String(instance_create_time.to_string()),
            );
        }

        if let Some(backup_retention_period) = instance.backup_retention_period {
            json.insert(
                "BackupRetentionPeriod".to_string(),
                serde_json::Value::Number(serde_json::Number::from(backup_retention_period)),
            );
        }

        if let Some(vpc_security_groups) = &instance.vpc_security_groups {
            let sg_array: Vec<serde_json::Value> = vpc_security_groups
                .iter()
                .filter_map(|sg| sg.vpc_security_group_id.as_ref())
                .map(|sg_id| serde_json::Value::String(sg_id.clone()))
                .collect();
            json.insert(
                "VpcSecurityGroups".to_string(),
                serde_json::Value::Array(sg_array),
            );
        }

        if let Some(db_subnet_group) = &instance.db_subnet_group {
            if let Some(db_subnet_group_name) = &db_subnet_group.db_subnet_group_name {
                json.insert(
                    "DBSubnetGroupName".to_string(),
                    serde_json::Value::String(db_subnet_group_name.clone()),
                );
            }
        }

        if let Some(multi_az) = instance.multi_az {
            json.insert("MultiAZ".to_string(), serde_json::Value::Bool(multi_az));
        }

        // Note: publicly_accessible field is deprecated, skipping

        if let Some(storage_encrypted) = instance.storage_encrypted {
            json.insert(
                "StorageEncrypted".to_string(),
                serde_json::Value::Bool(storage_encrypted),
            );
        }

        if let Some(kms_key_id) = &instance.kms_key_id {
            json.insert(
                "KmsKeyId".to_string(),
                serde_json::Value::String(kms_key_id.clone()),
            );
        }

        if let Some(auto_minor_version_upgrade) = instance.auto_minor_version_upgrade {
            json.insert(
                "AutoMinorVersionUpgrade".to_string(),
                serde_json::Value::Bool(auto_minor_version_upgrade),
            );
        }

        if let Some(db_cluster_identifier) = &instance.db_cluster_identifier {
            json.insert(
                "DBClusterIdentifier".to_string(),
                serde_json::Value::String(db_cluster_identifier.clone()),
            );
        }

        serde_json::Value::Object(json)
    }
}
