use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_rds as rds;
use std::sync::Arc;

pub struct RDSService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl RDSService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List RDS DB instances
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

        let client = rds::Client::new(&aws_config);
        let mut instances = Vec::new();

        // Use describe_db_instances for comprehensive instance data
        let mut paginator = client.describe_db_instances().into_paginator().send();

        while let Some(result) = paginator.try_next().await? {
            let instance_list = result.db_instances.unwrap_or_default();
            for instance in instance_list {
                let instance_json = self.db_instance_to_json(&instance);
                instances.push(instance_json);
            }
        }

        Ok(instances)
    }

    /// List RDS DB clusters
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

        let client = rds::Client::new(&aws_config);
        let mut clusters = Vec::new();

        // Use describe_db_clusters for comprehensive cluster data
        let mut paginator = client.describe_db_clusters().into_paginator().send();

        while let Some(result) = paginator.try_next().await? {
            let cluster_list = result.db_clusters.unwrap_or_default();
            for cluster in cluster_list {
                let cluster_json = self.db_cluster_to_json(&cluster);
                clusters.push(cluster_json);
            }
        }

        Ok(clusters)
    }

    /// List RDS DB snapshots
    pub async fn list_db_snapshots(
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

        let client = rds::Client::new(&aws_config);
        let mut snapshots = Vec::new();

        // Use describe_db_snapshots for comprehensive snapshot data
        let mut paginator = client
            .describe_db_snapshots()
            .snapshot_type("manual") // Only manual snapshots to avoid too much data
            .into_paginator()
            .send();

        while let Some(result) = paginator.try_next().await? {
            let snapshot_list = result.db_snapshots.unwrap_or_default();
            for snapshot in snapshot_list {
                let snapshot_json = self.db_snapshot_to_json(&snapshot);
                snapshots.push(snapshot_json);
            }
        }

        Ok(snapshots)
    }

    /// List RDS DB parameter groups
    pub async fn list_db_parameter_groups(
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

        let client = rds::Client::new(&aws_config);
        let mut parameter_groups = Vec::new();

        // Use describe_db_parameter_groups for comprehensive parameter group data
        let mut paginator = client
            .describe_db_parameter_groups()
            .into_paginator()
            .send();

        while let Some(result) = paginator.try_next().await? {
            let parameter_group_list = result.db_parameter_groups.unwrap_or_default();
            for parameter_group in parameter_group_list {
                let parameter_group_json = self.db_parameter_group_to_json(&parameter_group);
                parameter_groups.push(parameter_group_json);
            }
        }

        Ok(parameter_groups)
    }

    /// List RDS DB subnet groups
    pub async fn list_db_subnet_groups(
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

        let client = rds::Client::new(&aws_config);
        let mut subnet_groups = Vec::new();

        // Use describe_db_subnet_groups for comprehensive subnet group data
        let mut paginator = client.describe_db_subnet_groups().into_paginator().send();

        while let Some(result) = paginator.try_next().await? {
            let subnet_group_list = result.db_subnet_groups.unwrap_or_default();
            for subnet_group in subnet_group_list {
                let subnet_group_json = self.db_subnet_group_to_json(&subnet_group);
                subnet_groups.push(subnet_group_json);
            }
        }

        Ok(subnet_groups)
    }

    /// Get detailed DB instance information
    pub async fn describe_db_instance(
        &self,
        account_id: &str,
        region: &str,
        instance_identifier: &str,
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

        let client = rds::Client::new(&aws_config);

        let response = client
            .describe_db_instances()
            .db_instance_identifier(instance_identifier)
            .send()
            .await?;

        if let Some(instances) = response.db_instances {
            if let Some(instance) = instances.into_iter().next() {
                return Ok(self.db_instance_to_json(&instance));
            }
        }

        Err(anyhow::anyhow!(
            "DB instance not found: {}",
            instance_identifier
        ))
    }

    /// Describe specific RDS DB Cluster
    pub async fn describe_db_cluster(
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

        let client = rds::Client::new(&aws_config);

        let response = client
            .describe_db_clusters()
            .db_cluster_identifier(cluster_identifier)
            .send()
            .await?;

        if let Some(clusters) = response.db_clusters {
            if let Some(cluster) = clusters.into_iter().next() {
                return Ok(self.db_cluster_to_json(&cluster));
            }
        }

        Err(anyhow::anyhow!(
            "DB cluster not found: {}",
            cluster_identifier
        ))
    }

    /// Describe specific RDS DB Snapshot
    pub async fn describe_db_snapshot(
        &self,
        account_id: &str,
        region: &str,
        snapshot_identifier: &str,
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

        let client = rds::Client::new(&aws_config);

        let response = client
            .describe_db_snapshots()
            .db_snapshot_identifier(snapshot_identifier)
            .send()
            .await?;

        if let Some(snapshots) = response.db_snapshots {
            if let Some(snapshot) = snapshots.into_iter().next() {
                return Ok(self.db_snapshot_to_json(&snapshot));
            }
        }

        Err(anyhow::anyhow!(
            "DB snapshot not found: {}",
            snapshot_identifier
        ))
    }

    /// Describe specific RDS DB Parameter Group
    pub async fn describe_db_parameter_group(
        &self,
        account_id: &str,
        region: &str,
        parameter_group_name: &str,
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

        let client = rds::Client::new(&aws_config);

        let response = client
            .describe_db_parameter_groups()
            .db_parameter_group_name(parameter_group_name)
            .send()
            .await?;

        if let Some(parameter_groups) = response.db_parameter_groups {
            if let Some(parameter_group) = parameter_groups.into_iter().next() {
                return Ok(self.db_parameter_group_to_json(&parameter_group));
            }
        }

        Err(anyhow::anyhow!(
            "DB parameter group not found: {}",
            parameter_group_name
        ))
    }

    /// Describe specific RDS DB Subnet Group
    pub async fn describe_db_subnet_group(
        &self,
        account_id: &str,
        region: &str,
        subnet_group_name: &str,
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

        let client = rds::Client::new(&aws_config);

        let response = client
            .describe_db_subnet_groups()
            .db_subnet_group_name(subnet_group_name)
            .send()
            .await?;

        if let Some(subnet_groups) = response.db_subnet_groups {
            if let Some(subnet_group) = subnet_groups.into_iter().next() {
                return Ok(self.db_subnet_group_to_json(&subnet_group));
            }
        }

        Err(anyhow::anyhow!(
            "DB subnet group not found: {}",
            subnet_group_name
        ))
    }

    /// Convert DB instance to JSON format
    fn db_instance_to_json(&self, instance: &rds::types::DbInstance) -> serde_json::Value {
        let mut instance_map = serde_json::Map::new();

        if let Some(identifier) = &instance.db_instance_identifier {
            instance_map.insert(
                "DBInstanceIdentifier".to_string(),
                serde_json::Value::String(identifier.clone()),
            );
            instance_map.insert(
                "Name".to_string(),
                serde_json::Value::String(identifier.clone()),
            );
        }

        if let Some(class) = &instance.db_instance_class {
            instance_map.insert(
                "DBInstanceClass".to_string(),
                serde_json::Value::String(class.clone()),
            );
        }

        if let Some(engine) = &instance.engine {
            instance_map.insert(
                "Engine".to_string(),
                serde_json::Value::String(engine.clone()),
            );
        }

        if let Some(engine_version) = &instance.engine_version {
            instance_map.insert(
                "EngineVersion".to_string(),
                serde_json::Value::String(engine_version.clone()),
            );
        }

        if let Some(status) = &instance.db_instance_status {
            instance_map.insert(
                "DBInstanceStatus".to_string(),
                serde_json::Value::String(status.clone()),
            );
            instance_map.insert(
                "Status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(allocated_storage) = instance.allocated_storage {
            instance_map.insert(
                "AllocatedStorage".to_string(),
                serde_json::Value::Number(allocated_storage.into()),
            );
        }

        if let Some(creation_time) = instance.instance_create_time {
            instance_map.insert(
                "InstanceCreateTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(availability_zone) = &instance.availability_zone {
            instance_map.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        // TODO: Manually convert complex types to avoid serialization issues
        // if let Some(vpc_security_groups) = &instance.vpc_security_groups {
        //     instance_map.insert("VpcSecurityGroups".to_string(),
        //         serde_json::to_value(vpc_security_groups).unwrap_or(serde_json::Value::Null));
        // }

        // if let Some(db_subnet_group) = &instance.db_subnet_group {
        //     instance_map.insert("DBSubnetGroup".to_string(),
        //         serde_json::to_value(db_subnet_group).unwrap_or(serde_json::Value::Null));
        // }

        // if let Some(endpoint) = &instance.endpoint {
        //     instance_map.insert("Endpoint".to_string(),
        //         serde_json::to_value(endpoint).unwrap_or(serde_json::Value::Null));
        // }

        if let Some(multi_az) = instance.multi_az {
            instance_map.insert("MultiAZ".to_string(), serde_json::Value::Bool(multi_az));
        }

        if let Some(publicly_accessible) = instance.publicly_accessible {
            instance_map.insert(
                "PubliclyAccessible".to_string(),
                serde_json::Value::Bool(publicly_accessible),
            );
        }

        if let Some(storage_encrypted) = instance.storage_encrypted {
            instance_map.insert(
                "StorageEncrypted".to_string(),
                serde_json::Value::Bool(storage_encrypted),
            );
        }

        if let Some(backup_retention_period) = instance.backup_retention_period {
            instance_map.insert(
                "BackupRetentionPeriod".to_string(),
                serde_json::Value::Number(backup_retention_period.into()),
            );
        }

        if let Some(tags) = &instance.tag_list {
            let tags_json: Vec<serde_json::Value> = tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    if let Some(key) = &tag.key {
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    }
                    if let Some(value) = &tag.value {
                        tag_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                    }
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            instance_map.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
        }

        serde_json::Value::Object(instance_map)
    }

    /// Convert DB cluster to JSON format
    fn db_cluster_to_json(&self, cluster: &rds::types::DbCluster) -> serde_json::Value {
        let mut cluster_map = serde_json::Map::new();

        if let Some(identifier) = &cluster.db_cluster_identifier {
            cluster_map.insert(
                "DBClusterIdentifier".to_string(),
                serde_json::Value::String(identifier.clone()),
            );
            cluster_map.insert(
                "Name".to_string(),
                serde_json::Value::String(identifier.clone()),
            );
        }

        if let Some(engine) = &cluster.engine {
            cluster_map.insert(
                "Engine".to_string(),
                serde_json::Value::String(engine.clone()),
            );
        }

        if let Some(engine_version) = &cluster.engine_version {
            cluster_map.insert(
                "EngineVersion".to_string(),
                serde_json::Value::String(engine_version.clone()),
            );
        }

        if let Some(status) = &cluster.status {
            cluster_map.insert(
                "Status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(creation_time) = cluster.cluster_create_time {
            cluster_map.insert(
                "ClusterCreateTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        // TODO: Manually convert complex types to avoid serialization issues
        // if let Some(members) = &cluster.db_cluster_members {
        //     cluster_map.insert("DBClusterMembers".to_string(),
        //         serde_json::to_value(members).unwrap_or(serde_json::Value::Null));
        // }

        if let Some(availability_zones) = &cluster.availability_zones {
            let zones_json: Vec<serde_json::Value> = availability_zones
                .iter()
                .map(|az| serde_json::Value::String(az.clone()))
                .collect();
            cluster_map.insert(
                "AvailabilityZones".to_string(),
                serde_json::Value::Array(zones_json),
            );
        }

        // if let Some(vpc_security_groups) = &cluster.vpc_security_groups {
        //     cluster_map.insert("VpcSecurityGroups".to_string(),
        //         serde_json::to_value(vpc_security_groups).unwrap_or(serde_json::Value::Null));
        // }

        if let Some(storage_encrypted) = cluster.storage_encrypted {
            cluster_map.insert(
                "StorageEncrypted".to_string(),
                serde_json::Value::Bool(storage_encrypted),
            );
        }

        if let Some(backup_retention_period) = cluster.backup_retention_period {
            cluster_map.insert(
                "BackupRetentionPeriod".to_string(),
                serde_json::Value::Number(backup_retention_period.into()),
            );
        }

        if let Some(tags) = &cluster.tag_list {
            let tags_json: Vec<serde_json::Value> = tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    if let Some(key) = &tag.key {
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    }
                    if let Some(value) = &tag.value {
                        tag_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                    }
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            cluster_map.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
        }

        serde_json::Value::Object(cluster_map)
    }

    /// Convert DB snapshot to JSON format
    fn db_snapshot_to_json(&self, snapshot: &rds::types::DbSnapshot) -> serde_json::Value {
        let mut snapshot_map = serde_json::Map::new();

        if let Some(identifier) = &snapshot.db_snapshot_identifier {
            snapshot_map.insert(
                "DBSnapshotIdentifier".to_string(),
                serde_json::Value::String(identifier.clone()),
            );
            snapshot_map.insert(
                "Name".to_string(),
                serde_json::Value::String(identifier.clone()),
            );
        }

        if let Some(db_instance_identifier) = &snapshot.db_instance_identifier {
            snapshot_map.insert(
                "DBInstanceIdentifier".to_string(),
                serde_json::Value::String(db_instance_identifier.clone()),
            );
        }

        if let Some(snapshot_type) = &snapshot.snapshot_type {
            snapshot_map.insert(
                "SnapshotType".to_string(),
                serde_json::Value::String(snapshot_type.clone()),
            );
        }

        if let Some(status) = &snapshot.status {
            snapshot_map.insert(
                "Status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(creation_time) = snapshot.snapshot_create_time {
            snapshot_map.insert(
                "SnapshotCreateTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(allocated_storage) = snapshot.allocated_storage {
            snapshot_map.insert(
                "AllocatedStorage".to_string(),
                serde_json::Value::Number(allocated_storage.into()),
            );
        }

        if let Some(engine) = &snapshot.engine {
            snapshot_map.insert(
                "Engine".to_string(),
                serde_json::Value::String(engine.clone()),
            );
        }

        if let Some(engine_version) = &snapshot.engine_version {
            snapshot_map.insert(
                "EngineVersion".to_string(),
                serde_json::Value::String(engine_version.clone()),
            );
        }

        if let Some(availability_zone) = &snapshot.availability_zone {
            snapshot_map.insert(
                "AvailabilityZone".to_string(),
                serde_json::Value::String(availability_zone.clone()),
            );
        }

        if let Some(encrypted) = snapshot.encrypted {
            snapshot_map.insert("Encrypted".to_string(), serde_json::Value::Bool(encrypted));
        }

        if let Some(tags) = &snapshot.tag_list {
            let tags_json: Vec<serde_json::Value> = tags
                .iter()
                .map(|tag| {
                    let mut tag_json = serde_json::Map::new();
                    if let Some(key) = &tag.key {
                        tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                    }
                    if let Some(value) = &tag.value {
                        tag_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                    }
                    serde_json::Value::Object(tag_json)
                })
                .collect();
            snapshot_map.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
        }

        serde_json::Value::Object(snapshot_map)
    }

    /// Convert DB parameter group to JSON format
    fn db_parameter_group_to_json(
        &self,
        parameter_group: &rds::types::DbParameterGroup,
    ) -> serde_json::Value {
        let mut parameter_group_map = serde_json::Map::new();

        if let Some(name) = &parameter_group.db_parameter_group_name {
            parameter_group_map.insert(
                "DBParameterGroupName".to_string(),
                serde_json::Value::String(name.clone()),
            );
            parameter_group_map.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(family) = &parameter_group.db_parameter_group_family {
            parameter_group_map.insert(
                "DBParameterGroupFamily".to_string(),
                serde_json::Value::String(family.clone()),
            );
        }

        if let Some(description) = &parameter_group.description {
            parameter_group_map.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(arn) = &parameter_group.db_parameter_group_arn {
            parameter_group_map.insert(
                "DBParameterGroupArn".to_string(),
                serde_json::Value::String(arn.clone()),
            );
        }

        // TODO: Add tags if present - need to check correct field name for parameter groups
        // DB Parameter Groups might not have direct tag access in list operation

        serde_json::Value::Object(parameter_group_map)
    }

    /// Convert DB subnet group to JSON format
    fn db_subnet_group_to_json(
        &self,
        subnet_group: &rds::types::DbSubnetGroup,
    ) -> serde_json::Value {
        let mut subnet_group_map = serde_json::Map::new();

        if let Some(name) = &subnet_group.db_subnet_group_name {
            subnet_group_map.insert(
                "DBSubnetGroupName".to_string(),
                serde_json::Value::String(name.clone()),
            );
            subnet_group_map.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &subnet_group.db_subnet_group_description {
            subnet_group_map.insert(
                "DBSubnetGroupDescription".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(vpc_id) = &subnet_group.vpc_id {
            subnet_group_map.insert(
                "VpcId".to_string(),
                serde_json::Value::String(vpc_id.clone()),
            );
        }

        if let Some(status) = &subnet_group.subnet_group_status {
            subnet_group_map.insert(
                "SubnetGroupStatus".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(arn) = &subnet_group.db_subnet_group_arn {
            subnet_group_map.insert(
                "DBSubnetGroupArn".to_string(),
                serde_json::Value::String(arn.clone()),
            );
        }

        // Add subnets if present
        if let Some(ref subnets) = subnet_group.subnets {
            if !subnets.is_empty() {
                let subnets_json: Vec<serde_json::Value> = subnets
                    .iter()
                    .map(|subnet| {
                        let mut subnet_json = serde_json::Map::new();
                        if let Some(subnet_id) = &subnet.subnet_identifier {
                            subnet_json.insert(
                                "SubnetIdentifier".to_string(),
                                serde_json::Value::String(subnet_id.clone()),
                            );
                        }
                        if let Some(availability_zone) = &subnet.subnet_availability_zone {
                            if let Some(name) = &availability_zone.name {
                                subnet_json.insert(
                                    "AvailabilityZone".to_string(),
                                    serde_json::Value::String(name.clone()),
                                );
                            }
                        }
                        if let Some(status) = &subnet.subnet_status {
                            subnet_json.insert(
                                "SubnetStatus".to_string(),
                                serde_json::Value::String(status.clone()),
                            );
                        }
                        serde_json::Value::Object(subnet_json)
                    })
                    .collect();
                subnet_group_map.insert(
                    "Subnets".to_string(),
                    serde_json::Value::Array(subnets_json),
                );
            }
        }

        // TODO: Add tags if present - need to check correct field name for subnet groups
        // DB Subnet Groups might not have direct tag access in list operation

        serde_json::Value::Object(subnet_group_map)
    }
}
