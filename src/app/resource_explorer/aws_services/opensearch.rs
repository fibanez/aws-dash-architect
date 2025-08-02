use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_opensearch as opensearch;
use std::sync::Arc;

pub struct OpenSearchService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl OpenSearchService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List OpenSearch Domains
    pub async fn list_domains(
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

        let client = opensearch::Client::new(&aws_config);

        let response = client.list_domain_names().send().await?;

        let mut domains = Vec::new();

        if let Some(domain_names) = response.domain_names {
            for domain_info in domain_names {
                if let Some(domain_name) = &domain_info.domain_name {
                    // Get detailed domain information
                    if let Ok(domain_details) = self.get_domain_internal(&client, domain_name).await
                    {
                        domains.push(domain_details);
                    } else {
                        // Fallback to basic domain info if describe fails
                        let mut domain_json = serde_json::Map::new();
                        domain_json.insert(
                            "DomainName".to_string(),
                            serde_json::Value::String(domain_name.clone()),
                        );
                        domain_json.insert(
                            "Name".to_string(),
                            serde_json::Value::String(domain_name.clone()),
                        );
                        if let Some(engine_type) = &domain_info.engine_type {
                            domain_json.insert(
                                "EngineType".to_string(),
                                serde_json::Value::String(format!("{:?}", engine_type)),
                            );
                        }
                        domains.push(serde_json::Value::Object(domain_json));
                    }
                }
            }
        }

        Ok(domains)
    }

    /// Get detailed information for specific OpenSearch Domain
    pub async fn describe_domain(
        &self,
        account_id: &str,
        region: &str,
        domain_name: &str,
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

        let client = opensearch::Client::new(&aws_config);
        self.get_domain_internal(&client, domain_name).await
    }

    async fn get_domain_internal(
        &self,
        client: &opensearch::Client,
        domain_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_domain()
            .domain_name(domain_name)
            .send()
            .await?;

        if let Some(domain_status) = response.domain_status {
            Ok(self.domain_to_json(&domain_status))
        } else {
            Err(anyhow::anyhow!(
                "OpenSearch domain {} not found",
                domain_name
            ))
        }
    }

    fn domain_to_json(&self, domain: &opensearch::types::DomainStatus) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "DomainId".to_string(),
            serde_json::Value::String(domain.domain_id.clone()),
        );

        json.insert(
            "DomainName".to_string(),
            serde_json::Value::String(domain.domain_name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(domain.domain_name.clone()),
        );

        json.insert(
            "ARN".to_string(),
            serde_json::Value::String(domain.arn.clone()),
        );

        if let Some(created) = domain.created {
            json.insert("Created".to_string(), serde_json::Value::Bool(created));
        }

        if let Some(deleted) = domain.deleted {
            json.insert("Deleted".to_string(), serde_json::Value::Bool(deleted));
        }

        if let Some(endpoint) = &domain.endpoint {
            json.insert(
                "Endpoint".to_string(),
                serde_json::Value::String(endpoint.clone()),
            );
        }

        if let Some(endpoints) = &domain.endpoints {
            let endpoints_json: serde_json::Map<String, serde_json::Value> = endpoints
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert(
                "Endpoints".to_string(),
                serde_json::Value::Object(endpoints_json),
            );
        }

        if let Some(processing) = domain.processing {
            json.insert(
                "Processing".to_string(),
                serde_json::Value::Bool(processing),
            );
        }

        if let Some(upgrade_processing) = domain.upgrade_processing {
            json.insert(
                "UpgradeProcessing".to_string(),
                serde_json::Value::Bool(upgrade_processing),
            );
        }

        if let Some(engine_version) = &domain.engine_version {
            json.insert(
                "EngineVersion".to_string(),
                serde_json::Value::String(engine_version.clone()),
            );
        }

        // Cluster Config
        if let Some(cluster_config) = &domain.cluster_config {
            let mut cluster_json = serde_json::Map::new();

            if let Some(instance_type) = &cluster_config.instance_type {
                cluster_json.insert(
                    "InstanceType".to_string(),
                    serde_json::Value::String(format!("{:?}", instance_type)),
                );
            }

            if let Some(instance_count) = cluster_config.instance_count {
                cluster_json.insert(
                    "InstanceCount".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(instance_count)),
                );
            }

            if let Some(dedicated_master_enabled) = cluster_config.dedicated_master_enabled {
                cluster_json.insert(
                    "DedicatedMasterEnabled".to_string(),
                    serde_json::Value::Bool(dedicated_master_enabled),
                );
            }

            if let Some(dedicated_master_type) = &cluster_config.dedicated_master_type {
                cluster_json.insert(
                    "MasterInstanceType".to_string(),
                    serde_json::Value::String(format!("{:?}", dedicated_master_type)),
                );
            }

            if let Some(dedicated_master_count) = cluster_config.dedicated_master_count {
                cluster_json.insert(
                    "MasterInstanceCount".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(dedicated_master_count)),
                );
            }

            if let Some(zone_awareness_enabled) = cluster_config.zone_awareness_enabled {
                cluster_json.insert(
                    "ZoneAwarenessEnabled".to_string(),
                    serde_json::Value::Bool(zone_awareness_enabled),
                );
            }

            json.insert(
                "ClusterConfig".to_string(),
                serde_json::Value::Object(cluster_json),
            );
        }

        // EBS Options
        if let Some(ebs_options) = &domain.ebs_options {
            let mut ebs_json = serde_json::Map::new();

            if let Some(ebs_enabled) = ebs_options.ebs_enabled {
                ebs_json.insert(
                    "EBSEnabled".to_string(),
                    serde_json::Value::Bool(ebs_enabled),
                );
            }

            if let Some(volume_type) = &ebs_options.volume_type {
                ebs_json.insert(
                    "VolumeType".to_string(),
                    serde_json::Value::String(format!("{:?}", volume_type)),
                );
            }

            if let Some(volume_size) = ebs_options.volume_size {
                ebs_json.insert(
                    "VolumeSize".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(volume_size)),
                );
            }

            if let Some(iops) = ebs_options.iops {
                ebs_json.insert(
                    "Iops".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(iops)),
                );
            }

            json.insert(
                "EBSOptions".to_string(),
                serde_json::Value::Object(ebs_json),
            );
        }

        // Access Policies
        if let Some(access_policies) = &domain.access_policies {
            json.insert(
                "AccessPolicies".to_string(),
                serde_json::Value::String(access_policies.clone()),
            );
        }

        // VPC Options
        if let Some(vpc_options) = &domain.vpc_options {
            let mut vpc_json = serde_json::Map::new();

            if let Some(vpc_id) = &vpc_options.vpc_id {
                vpc_json.insert(
                    "VPCId".to_string(),
                    serde_json::Value::String(vpc_id.clone()),
                );
            }

            if let Some(subnet_ids) = &vpc_options.subnet_ids {
                vpc_json.insert(
                    "SubnetIds".to_string(),
                    serde_json::Value::Array(
                        subnet_ids
                            .iter()
                            .map(|id| serde_json::Value::String(id.clone()))
                            .collect(),
                    ),
                );
            }

            if let Some(availability_zones) = &vpc_options.availability_zones {
                vpc_json.insert(
                    "AvailabilityZones".to_string(),
                    serde_json::Value::Array(
                        availability_zones
                            .iter()
                            .map(|az| serde_json::Value::String(az.clone()))
                            .collect(),
                    ),
                );
            }

            if let Some(security_group_ids) = &vpc_options.security_group_ids {
                vpc_json.insert(
                    "SecurityGroupIds".to_string(),
                    serde_json::Value::Array(
                        security_group_ids
                            .iter()
                            .map(|id| serde_json::Value::String(id.clone()))
                            .collect(),
                    ),
                );
            }

            json.insert(
                "VPCOptions".to_string(),
                serde_json::Value::Object(vpc_json),
            );
        }

        // Cognito Options
        if let Some(cognito_options) = &domain.cognito_options {
            let mut cognito_json = serde_json::Map::new();

            if let Some(enabled) = cognito_options.enabled {
                cognito_json.insert("Enabled".to_string(), serde_json::Value::Bool(enabled));
            }

            if let Some(user_pool_id) = &cognito_options.user_pool_id {
                cognito_json.insert(
                    "UserPoolId".to_string(),
                    serde_json::Value::String(user_pool_id.clone()),
                );
            }

            if let Some(identity_pool_id) = &cognito_options.identity_pool_id {
                cognito_json.insert(
                    "IdentityPoolId".to_string(),
                    serde_json::Value::String(identity_pool_id.clone()),
                );
            }

            if let Some(role_arn) = &cognito_options.role_arn {
                cognito_json.insert(
                    "RoleArn".to_string(),
                    serde_json::Value::String(role_arn.clone()),
                );
            }

            json.insert(
                "CognitoOptions".to_string(),
                serde_json::Value::Object(cognito_json),
            );
        }

        // Encryption At Rest
        if let Some(encryption_at_rest_options) = &domain.encryption_at_rest_options {
            let mut encryption_json = serde_json::Map::new();

            if let Some(enabled) = encryption_at_rest_options.enabled {
                encryption_json.insert("Enabled".to_string(), serde_json::Value::Bool(enabled));
            }

            if let Some(kms_key_id) = &encryption_at_rest_options.kms_key_id {
                encryption_json.insert(
                    "KmsKeyId".to_string(),
                    serde_json::Value::String(kms_key_id.clone()),
                );
            }

            json.insert(
                "EncryptionAtRestOptions".to_string(),
                serde_json::Value::Object(encryption_json),
            );
        }

        // Node to Node Encryption
        if let Some(node_to_node_encryption_options) = &domain.node_to_node_encryption_options {
            if let Some(enabled) = node_to_node_encryption_options.enabled {
                json.insert(
                    "NodeToNodeEncryptionEnabled".to_string(),
                    serde_json::Value::Bool(enabled),
                );
            }
        }

        // Domain Endpoint Options
        if let Some(domain_endpoint_options) = &domain.domain_endpoint_options {
            let mut endpoint_json = serde_json::Map::new();

            if let Some(enforce_https) = domain_endpoint_options.enforce_https {
                endpoint_json.insert(
                    "EnforceHTTPS".to_string(),
                    serde_json::Value::Bool(enforce_https),
                );
            }

            if let Some(tls_security_policy) = &domain_endpoint_options.tls_security_policy {
                endpoint_json.insert(
                    "TLSSecurityPolicy".to_string(),
                    serde_json::Value::String(format!("{:?}", tls_security_policy)),
                );
            }

            if let Some(custom_endpoint_enabled) = domain_endpoint_options.custom_endpoint_enabled {
                endpoint_json.insert(
                    "CustomEndpointEnabled".to_string(),
                    serde_json::Value::Bool(custom_endpoint_enabled),
                );
            }

            if let Some(custom_endpoint) = &domain_endpoint_options.custom_endpoint {
                endpoint_json.insert(
                    "CustomEndpoint".to_string(),
                    serde_json::Value::String(custom_endpoint.clone()),
                );
            }

            json.insert(
                "DomainEndpointOptions".to_string(),
                serde_json::Value::Object(endpoint_json),
            );
        }

        serde_json::Value::Object(json)
    }
}
