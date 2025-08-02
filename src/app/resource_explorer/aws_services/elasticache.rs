use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_elasticache as elasticache;
use std::sync::Arc;

pub struct ElastiCacheService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ElastiCacheService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List ElastiCache Cache Clusters
    pub async fn list_cache_clusters(
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

        let client = elasticache::Client::new(&aws_config);

        let mut cache_clusters = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.describe_cache_clusters();
            if let Some(ref marker_value) = marker {
                request = request.marker(marker_value);
            }

            let response = request.send().await?;

            if let Some(clusters) = response.cache_clusters {
                for cluster in clusters {
                    let cluster_json = self.cache_cluster_to_json(&cluster);
                    cache_clusters.push(cluster_json);
                }
            }

            if let Some(next_marker) = response.marker {
                marker = Some(next_marker);
            } else {
                break;
            }
        }

        Ok(cache_clusters)
    }

    /// List ElastiCache Replication Groups
    pub async fn list_replication_groups(
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

        let client = elasticache::Client::new(&aws_config);

        let mut replication_groups = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.describe_replication_groups();
            if let Some(ref marker_value) = marker {
                request = request.marker(marker_value);
            }

            let response = request.send().await?;

            if let Some(groups) = response.replication_groups {
                for group in groups {
                    let group_json = self.replication_group_to_json(&group);
                    replication_groups.push(group_json);
                }
            }

            if let Some(next_marker) = response.marker {
                marker = Some(next_marker);
            } else {
                break;
            }
        }

        Ok(replication_groups)
    }

    /// List ElastiCache Parameter Groups
    pub async fn list_cache_parameter_groups(
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

        let client = elasticache::Client::new(&aws_config);

        let mut parameter_groups = Vec::new();
        let mut marker = None;

        loop {
            let mut request = client.describe_cache_parameter_groups();
            if let Some(ref marker_value) = marker {
                request = request.marker(marker_value);
            }

            let response = request.send().await?;

            if let Some(groups) = response.cache_parameter_groups {
                for group in groups {
                    let group_json = self.cache_parameter_group_to_json(&group);
                    parameter_groups.push(group_json);
                }
            }

            if let Some(next_marker) = response.marker {
                marker = Some(next_marker);
            } else {
                break;
            }
        }

        Ok(parameter_groups)
    }

    /// Get detailed information for specific Cache Cluster
    pub async fn describe_cache_cluster(
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

        let client = elasticache::Client::new(&aws_config);

        let response = client
            .describe_cache_clusters()
            .cache_cluster_id(cluster_id)
            .show_cache_node_info(true)
            .send()
            .await?;

        if let Some(clusters) = response.cache_clusters {
            if let Some(cluster) = clusters.first() {
                Ok(self.cache_cluster_to_json(cluster))
            } else {
                Err(anyhow::anyhow!("Cache cluster {} not found", cluster_id))
            }
        } else {
            Err(anyhow::anyhow!("Cache cluster {} not found", cluster_id))
        }
    }

    /// Get detailed information for specific Replication Group
    pub async fn describe_replication_group(
        &self,
        account_id: &str,
        region: &str,
        group_id: &str,
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

        let client = elasticache::Client::new(&aws_config);

        let response = client
            .describe_replication_groups()
            .replication_group_id(group_id)
            .send()
            .await?;

        if let Some(groups) = response.replication_groups {
            if let Some(group) = groups.first() {
                Ok(self.replication_group_to_json(group))
            } else {
                Err(anyhow::anyhow!("Replication group {} not found", group_id))
            }
        } else {
            Err(anyhow::anyhow!("Replication group {} not found", group_id))
        }
    }

    /// Get detailed information for specific Cache Parameter Group
    pub async fn describe_cache_parameter_group(
        &self,
        account_id: &str,
        region: &str,
        group_name: &str,
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

        let client = elasticache::Client::new(&aws_config);

        let response = client
            .describe_cache_parameter_groups()
            .cache_parameter_group_name(group_name)
            .send()
            .await?;

        if let Some(groups) = response.cache_parameter_groups {
            if let Some(group) = groups.first() {
                Ok(self.cache_parameter_group_to_json(group))
            } else {
                Err(anyhow::anyhow!(
                    "Cache parameter group {} not found",
                    group_name
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "Cache parameter group {} not found",
                group_name
            ))
        }
    }

    fn cache_cluster_to_json(
        &self,
        cluster: &elasticache::types::CacheCluster,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(cluster_id) = &cluster.cache_cluster_id {
            json.insert(
                "CacheClusterId".to_string(),
                serde_json::Value::String(cluster_id.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(cluster_id.clone()),
            );
        }

        if let Some(status) = &cluster.cache_cluster_status {
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

        if let Some(node_type) = &cluster.cache_node_type {
            json.insert(
                "CacheNodeType".to_string(),
                serde_json::Value::String(node_type.clone()),
            );
        }

        if let Some(num_nodes) = cluster.num_cache_nodes {
            json.insert(
                "NumCacheNodes".to_string(),
                serde_json::Value::Number(serde_json::Number::from(num_nodes)),
            );
        }

        if let Some(created_time) = &cluster.cache_cluster_create_time {
            json.insert(
                "CacheClusterCreateTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        if let Some(preferred_az) = &cluster.preferred_availability_zone {
            json.insert(
                "PreferredAvailabilityZone".to_string(),
                serde_json::Value::String(preferred_az.clone()),
            );
        }

        if let Some(parameter_group) = &cluster.cache_parameter_group {
            if let Some(group_name) = &parameter_group.cache_parameter_group_name {
                json.insert(
                    "CacheParameterGroupName".to_string(),
                    serde_json::Value::String(group_name.clone()),
                );
            }
        }

        if let Some(subnet_group_name) = &cluster.cache_subnet_group_name {
            json.insert(
                "CacheSubnetGroupName".to_string(),
                serde_json::Value::String(subnet_group_name.clone()),
            );
        }

        if let Some(security_groups) = &cluster.security_groups {
            let sg_array: Vec<serde_json::Value> = security_groups
                .iter()
                .filter_map(|sg| sg.security_group_id.as_ref())
                .map(|sg_id| serde_json::Value::String(sg_id.clone()))
                .collect();
            json.insert(
                "SecurityGroups".to_string(),
                serde_json::Value::Array(sg_array),
            );
        }

        if let Some(cache_nodes) = &cluster.cache_nodes {
            let nodes_array: Vec<serde_json::Value> = cache_nodes
                .iter()
                .map(|node| {
                    let mut node_json = serde_json::Map::new();
                    if let Some(node_id) = &node.cache_node_id {
                        node_json.insert(
                            "CacheNodeId".to_string(),
                            serde_json::Value::String(node_id.clone()),
                        );
                    }
                    if let Some(node_status) = &node.cache_node_status {
                        node_json.insert(
                            "CacheNodeStatus".to_string(),
                            serde_json::Value::String(node_status.clone()),
                        );
                    }
                    if let Some(endpoint) = &node.endpoint {
                        if let Some(address) = &endpoint.address {
                            node_json.insert(
                                "Address".to_string(),
                                serde_json::Value::String(address.clone()),
                            );
                        }
                        if let Some(port) = endpoint.port {
                            node_json.insert(
                                "Port".to_string(),
                                serde_json::Value::Number(serde_json::Number::from(port)),
                            );
                        }
                    }
                    serde_json::Value::Object(node_json)
                })
                .collect();
            json.insert(
                "CacheNodes".to_string(),
                serde_json::Value::Array(nodes_array),
            );
        }

        serde_json::Value::Object(json)
    }

    fn replication_group_to_json(
        &self,
        group: &elasticache::types::ReplicationGroup,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(group_id) = &group.replication_group_id {
            json.insert(
                "ReplicationGroupId".to_string(),
                serde_json::Value::String(group_id.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(group_id.clone()),
            );
        }

        if let Some(description) = &group.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(status) = &group.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(primary_endpoint) = &group.configuration_endpoint {
            if let Some(address) = &primary_endpoint.address {
                json.insert(
                    "PrimaryEndpoint".to_string(),
                    serde_json::Value::String(address.clone()),
                );
            }
            if let Some(port) = primary_endpoint.port {
                json.insert(
                    "Port".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(port)),
                );
            }
        }

        // Note: Reader endpoint may not be available in this structure

        if let Some(member_clusters) = &group.member_clusters {
            json.insert(
                "MemberClusters".to_string(),
                serde_json::Value::Array(
                    member_clusters
                        .iter()
                        .map(|cluster| serde_json::Value::String(cluster.clone()))
                        .collect(),
                ),
            );
            json.insert(
                "NumMemberClusters".to_string(),
                serde_json::Value::Number(serde_json::Number::from(member_clusters.len())),
            );
        }

        if let Some(node_groups) = &group.node_groups {
            let node_groups_array: Vec<serde_json::Value> = node_groups
                .iter()
                .map(|ng| {
                    let mut ng_json = serde_json::Map::new();
                    if let Some(node_group_id) = &ng.node_group_id {
                        ng_json.insert(
                            "NodeGroupId".to_string(),
                            serde_json::Value::String(node_group_id.clone()),
                        );
                    }
                    if let Some(status) = &ng.status {
                        ng_json.insert(
                            "Status".to_string(),
                            serde_json::Value::String(status.clone()),
                        );
                    }
                    if let Some(slots) = &ng.slots {
                        ng_json.insert(
                            "Slots".to_string(),
                            serde_json::Value::String(slots.clone()),
                        );
                    }
                    serde_json::Value::Object(ng_json)
                })
                .collect();
            json.insert(
                "NodeGroups".to_string(),
                serde_json::Value::Array(node_groups_array),
            );
        }

        if let Some(automatic_failover) = &group.automatic_failover {
            json.insert(
                "AutomaticFailover".to_string(),
                serde_json::Value::String(format!("{:?}", automatic_failover)),
            );
        }

        if let Some(multi_az) = &group.multi_az {
            json.insert(
                "MultiAZ".to_string(),
                serde_json::Value::String(format!("{:?}", multi_az)),
            );
        }

        if let Some(snapshot_retention_limit) = group.snapshot_retention_limit {
            json.insert(
                "SnapshotRetentionLimit".to_string(),
                serde_json::Value::Number(serde_json::Number::from(snapshot_retention_limit)),
            );
        }

        if let Some(snapshot_window) = &group.snapshot_window {
            json.insert(
                "SnapshotWindow".to_string(),
                serde_json::Value::String(snapshot_window.clone()),
            );
        }

        if let Some(cluster_enabled) = group.cluster_enabled {
            json.insert(
                "ClusterEnabled".to_string(),
                serde_json::Value::Bool(cluster_enabled),
            );
        }

        if let Some(cache_node_type) = &group.cache_node_type {
            json.insert(
                "CacheNodeType".to_string(),
                serde_json::Value::String(cache_node_type.clone()),
            );
        }

        if let Some(auth_token_enabled) = group.auth_token_enabled {
            json.insert(
                "AuthTokenEnabled".to_string(),
                serde_json::Value::Bool(auth_token_enabled),
            );
        }

        if let Some(transit_encryption_enabled) = group.transit_encryption_enabled {
            json.insert(
                "TransitEncryptionEnabled".to_string(),
                serde_json::Value::Bool(transit_encryption_enabled),
            );
        }

        if let Some(at_rest_encryption_enabled) = group.at_rest_encryption_enabled {
            json.insert(
                "AtRestEncryptionEnabled".to_string(),
                serde_json::Value::Bool(at_rest_encryption_enabled),
            );
        }

        serde_json::Value::Object(json)
    }

    fn cache_parameter_group_to_json(
        &self,
        group: &elasticache::types::CacheParameterGroup,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(group_name) = &group.cache_parameter_group_name {
            json.insert(
                "CacheParameterGroupName".to_string(),
                serde_json::Value::String(group_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(group_name.clone()),
            );
        }

        if let Some(group_family) = &group.cache_parameter_group_family {
            json.insert(
                "CacheParameterGroupFamily".to_string(),
                serde_json::Value::String(group_family.clone()),
            );
        }

        if let Some(description) = &group.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(is_global) = group.is_global {
            json.insert("IsGlobal".to_string(), serde_json::Value::Bool(is_global));
        }

        // Add a status field for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Available".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
