use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_eks as eks;
use std::sync::Arc;

pub struct EKSService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl EKSService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List EKS clusters
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

        let client = eks::Client::new(&aws_config);
        let mut paginator = client.list_clusters().into_paginator().send();

        let mut clusters = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(cluster_names) = page.clusters {
                for cluster_name in cluster_names {
                    // Get detailed cluster information
                    if let Ok(cluster_details) =
                        self.describe_cluster_internal(&client, &cluster_name).await
                    {
                        clusters.push(cluster_details);
                    } else {
                        // Fallback to basic cluster info if describe fails
                        let mut basic_cluster = serde_json::Map::new();
                        basic_cluster.insert(
                            "Name".to_string(),
                            serde_json::Value::String(cluster_name.clone()),
                        );
                        clusters.push(serde_json::Value::Object(basic_cluster));
                    }
                }
            }
        }

        Ok(clusters)
    }

    /// Get detailed information for specific EKS cluster
    pub async fn describe_cluster(
        &self,
        account_id: &str,
        region: &str,
        cluster_name: &str,
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

        let client = eks::Client::new(&aws_config);
        self.describe_cluster_internal(&client, cluster_name).await
    }

    /// List EKS Fargate Profiles across all clusters
    pub async fn list_fargate_profiles(
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

        let client = eks::Client::new(&aws_config);
        let mut fargate_profiles = Vec::new();

        // First get all clusters
        let mut clusters_paginator = client.list_clusters().into_paginator().send();
        
        while let Some(page) = clusters_paginator.next().await {
            let page = page?;
            if let Some(cluster_names) = page.clusters {
                for cluster_name in cluster_names {
                    // List Fargate profiles for each cluster
                    let mut profiles_paginator = client
                        .list_fargate_profiles()
                        .cluster_name(&cluster_name)
                        .into_paginator()
                        .send();

                    while let Some(profiles_page) = profiles_paginator.next().await {
                        let profiles_page = profiles_page?;
                        if let Some(profile_names) = profiles_page.fargate_profile_names {
                            for profile_name in profile_names {
                                // Get detailed profile information
                                if let Ok(profile_details) = self
                                    .describe_fargate_profile_internal(&client, &cluster_name, &profile_name)
                                    .await
                                {
                                    fargate_profiles.push(profile_details);
                                } else {
                                    // Fallback to basic profile info if describe fails
                                    let mut basic_profile = serde_json::Map::new();
                                    basic_profile.insert(
                                        "FargateProfileName".to_string(),
                                        serde_json::Value::String(profile_name.clone()),
                                    );
                                    basic_profile.insert(
                                        "ClusterName".to_string(),
                                        serde_json::Value::String(cluster_name.clone()),
                                    );
                                    basic_profile.insert(
                                        "ResourceId".to_string(),
                                        serde_json::Value::String(format!("{}:{}", cluster_name, profile_name)),
                                    );
                                    fargate_profiles.push(serde_json::Value::Object(basic_profile));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(fargate_profiles)
    }

    async fn describe_fargate_profile_internal(
        &self,
        client: &eks::Client,
        cluster_name: &str,
        profile_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_fargate_profile()
            .cluster_name(cluster_name)
            .fargate_profile_name(profile_name)
            .send()
            .await?;

        if let Some(profile) = response.fargate_profile {
            Ok(self.fargate_profile_to_json(&profile))
        } else {
            Err(anyhow::anyhow!("Fargate profile {} not found", profile_name))
        }
    }

    async fn describe_cluster_internal(
        &self,
        client: &eks::Client,
        cluster_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client.describe_cluster().name(cluster_name).send().await?;

        if let Some(cluster) = response.cluster {
            Ok(self.cluster_to_json(&cluster))
        } else {
            Err(anyhow::anyhow!("Cluster {} not found", cluster_name))
        }
    }

    fn cluster_to_json(&self, cluster: &eks::types::Cluster) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &cluster.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(arn) = &cluster.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(created_at) = cluster.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.to_string()),
            );
        }

        if let Some(version) = &cluster.version {
            json.insert(
                "Version".to_string(),
                serde_json::Value::String(version.clone()),
            );
        }

        if let Some(endpoint) = &cluster.endpoint {
            json.insert(
                "Endpoint".to_string(),
                serde_json::Value::String(endpoint.clone()),
            );
        }

        if let Some(role_arn) = &cluster.role_arn {
            json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(role_arn.clone()),
            );
        }

        if let Some(resources_vpc_config) = &cluster.resources_vpc_config {
            let mut vpc_config_json = serde_json::Map::new();

            if let Some(subnet_ids) = &resources_vpc_config.subnet_ids {
                let subnets_json: Vec<serde_json::Value> = subnet_ids
                    .iter()
                    .map(|subnet_id| serde_json::Value::String(subnet_id.clone()))
                    .collect();
                vpc_config_json.insert(
                    "SubnetIds".to_string(),
                    serde_json::Value::Array(subnets_json),
                );
            }

            if let Some(security_group_ids) = &resources_vpc_config.security_group_ids {
                let sgs_json: Vec<serde_json::Value> = security_group_ids
                    .iter()
                    .map(|sg_id| serde_json::Value::String(sg_id.clone()))
                    .collect();
                vpc_config_json.insert(
                    "SecurityGroupIds".to_string(),
                    serde_json::Value::Array(sgs_json),
                );
            }

            if let Some(vpc_id) = &resources_vpc_config.vpc_id {
                vpc_config_json.insert(
                    "VpcId".to_string(),
                    serde_json::Value::String(vpc_id.clone()),
                );
            }

            vpc_config_json.insert(
                "EndpointPublicAccess".to_string(),
                serde_json::Value::Bool(resources_vpc_config.endpoint_public_access),
            );

            vpc_config_json.insert(
                "EndpointPrivateAccess".to_string(),
                serde_json::Value::Bool(resources_vpc_config.endpoint_private_access),
            );

            json.insert(
                "ResourcesVpcConfig".to_string(),
                serde_json::Value::Object(vpc_config_json),
            );
        }

        if let Some(kubernetes_network_config) = &cluster.kubernetes_network_config {
            let mut network_config_json = serde_json::Map::new();

            if let Some(service_ipv4_cidr) = &kubernetes_network_config.service_ipv4_cidr {
                network_config_json.insert(
                    "ServiceIpv4Cidr".to_string(),
                    serde_json::Value::String(service_ipv4_cidr.clone()),
                );
            }

            json.insert(
                "KubernetesNetworkConfig".to_string(),
                serde_json::Value::Object(network_config_json),
            );
        }

        if let Some(status) = &cluster.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(certificate_authority) = &cluster.certificate_authority {
            let mut ca_json = serde_json::Map::new();
            if let Some(data) = &certificate_authority.data {
                ca_json.insert("Data".to_string(), serde_json::Value::String(data.clone()));
            }
            json.insert(
                "CertificateAuthority".to_string(),
                serde_json::Value::Object(ca_json),
            );
        }

        if let Some(platform_version) = &cluster.platform_version {
            json.insert(
                "PlatformVersion".to_string(),
                serde_json::Value::String(platform_version.clone()),
            );
        }

        if let Some(tags) = &cluster.tags {
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

        serde_json::Value::Object(json)
    }

    fn fargate_profile_to_json(&self, profile: &eks::types::FargateProfile) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        // Create unique ID combining cluster and profile name
        let cluster_name = profile.cluster_name.as_deref().unwrap_or("unknown-cluster");
        let profile_name = profile.fargate_profile_name.as_deref().unwrap_or("unknown-profile");
        let resource_id = format!("{}:{}", cluster_name, profile_name);
        
        json.insert("ResourceId".to_string(), serde_json::Value::String(resource_id));

        if let Some(profile_name) = &profile.fargate_profile_name {
            json.insert("FargateProfileName".to_string(), serde_json::Value::String(profile_name.clone()));
            json.insert("Name".to_string(), serde_json::Value::String(profile_name.clone()));
        }

        if let Some(cluster_name) = &profile.cluster_name {
            json.insert("ClusterName".to_string(), serde_json::Value::String(cluster_name.clone()));
        }

        if let Some(profile_arn) = &profile.fargate_profile_arn {
            json.insert("FargateProfileArn".to_string(), serde_json::Value::String(profile_arn.clone()));
        }

        if let Some(created_at) = profile.created_at {
            json.insert("CreatedAt".to_string(), serde_json::Value::String(created_at.to_string()));
        }

        if let Some(status) = &profile.status {
            json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
        }

        if let Some(pod_execution_role_arn) = &profile.pod_execution_role_arn {
            json.insert("PodExecutionRoleArn".to_string(), serde_json::Value::String(pod_execution_role_arn.clone()));
        }

        // Handle selectors
        if let Some(selectors) = &profile.selectors {
            if !selectors.is_empty() {
                let selectors_json: Vec<serde_json::Value> = selectors
                    .iter()
                    .map(|selector| {
                        let mut selector_json = serde_json::Map::new();
                        if let Some(namespace) = &selector.namespace {
                            selector_json.insert("Namespace".to_string(), serde_json::Value::String(namespace.clone()));
                        }
                        if let Some(labels) = &selector.labels {
                            if !labels.is_empty() {
                                let labels_json: Vec<serde_json::Value> = labels
                                    .iter()
                                    .map(|(k, v)| {
                                        let mut label_json = serde_json::Map::new();
                                        label_json.insert("Key".to_string(), serde_json::Value::String(k.clone()));
                                        label_json.insert("Value".to_string(), serde_json::Value::String(v.clone()));
                                        serde_json::Value::Object(label_json)
                                    })
                                    .collect();
                                selector_json.insert("Labels".to_string(), serde_json::Value::Array(labels_json));
                            }
                        }
                        serde_json::Value::Object(selector_json)
                    })
                    .collect();
                json.insert("Selectors".to_string(), serde_json::Value::Array(selectors_json));
            }
        }

        // Handle subnets
        if let Some(subnets) = &profile.subnets {
            if !subnets.is_empty() {
                let subnets_json: Vec<serde_json::Value> = subnets
                    .iter()
                    .map(|subnet| serde_json::Value::String(subnet.clone()))
                    .collect();
                json.insert("Subnets".to_string(), serde_json::Value::Array(subnets_json));
            }
        }

        // Handle tags  
        if let Some(tags) = &profile.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|(k, v)| {
                        let mut tag_json = serde_json::Map::new();
                        tag_json.insert("Key".to_string(), serde_json::Value::String(k.clone()));
                        tag_json.insert("Value".to_string(), serde_json::Value::String(v.clone()));
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }
}
