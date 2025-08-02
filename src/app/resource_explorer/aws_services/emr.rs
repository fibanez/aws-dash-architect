use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_emr as emr;
use std::sync::Arc;

pub struct EmrService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl EmrService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List EMR Clusters
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

        let client = emr::Client::new(&aws_config);
        let mut paginator = client.list_clusters().into_paginator().send();

        let mut clusters = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(cluster_list) = page.clusters {
                for cluster in cluster_list {
                    // Get detailed cluster information
                    if let Some(cluster_id) = &cluster.id {
                        if let Ok(cluster_details) =
                            self.describe_cluster_internal(&client, cluster_id).await
                        {
                            clusters.push(cluster_details);
                        } else {
                            // Fallback to basic cluster info if describe fails
                            let cluster_json = self.cluster_summary_to_json(&cluster);
                            clusters.push(cluster_json);
                        }
                    } else {
                        // Fallback to basic cluster info if no ID
                        let cluster_json = self.cluster_summary_to_json(&cluster);
                        clusters.push(cluster_json);
                    }
                }
            }
        }

        Ok(clusters)
    }

    /// Get detailed information for specific EMR cluster
    pub async fn describe_cluster(
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

        let client = emr::Client::new(&aws_config);
        self.describe_cluster_internal(&client, cluster_id).await
    }

    async fn describe_cluster_internal(
        &self,
        client: &emr::Client,
        cluster_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_cluster()
            .cluster_id(cluster_id)
            .send()
            .await?;

        if let Some(cluster) = response.cluster {
            Ok(self.cluster_to_json(&cluster))
        } else {
            Err(anyhow::anyhow!("Cluster {} not found", cluster_id))
        }
    }

    fn cluster_summary_to_json(&self, cluster: &emr::types::ClusterSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &cluster.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "ClusterId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(name) = &cluster.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(status) = &cluster.status {
            if let Some(state) = &status.state {
                json.insert(
                    "State".to_string(),
                    serde_json::Value::String(state.as_str().to_string()),
                );
                json.insert(
                    "Status".to_string(),
                    serde_json::Value::String(state.as_str().to_string()),
                );
            }
            if let Some(state_change_reason) = &status.state_change_reason {
                if let Some(message) = &state_change_reason.message {
                    json.insert(
                        "StateChangeReason".to_string(),
                        serde_json::Value::String(message.clone()),
                    );
                }
            }
        }

        if let Some(normalized_instance_hours) = cluster.normalized_instance_hours {
            json.insert(
                "NormalizedInstanceHours".to_string(),
                serde_json::Value::Number(normalized_instance_hours.into()),
            );
        }

        if let Some(cluster_arn) = &cluster.cluster_arn {
            json.insert(
                "ClusterArn".to_string(),
                serde_json::Value::String(cluster_arn.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn cluster_to_json(&self, cluster: &emr::types::Cluster) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &cluster.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "ClusterId".to_string(),
                serde_json::Value::String(id.clone()),
            );
        }

        if let Some(name) = &cluster.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(status) = &cluster.status {
            if let Some(state) = &status.state {
                json.insert(
                    "State".to_string(),
                    serde_json::Value::String(state.as_str().to_string()),
                );
                json.insert(
                    "Status".to_string(),
                    serde_json::Value::String(state.as_str().to_string()),
                );
            }
        }

        if let Some(ec2_instance_attributes) = &cluster.ec2_instance_attributes {
            let mut ec2_json = serde_json::Map::new();
            if let Some(ec2_key_name) = &ec2_instance_attributes.ec2_key_name {
                ec2_json.insert(
                    "Ec2KeyName".to_string(),
                    serde_json::Value::String(ec2_key_name.clone()),
                );
            }
            if let Some(ec2_subnet_id) = &ec2_instance_attributes.ec2_subnet_id {
                ec2_json.insert(
                    "Ec2SubnetId".to_string(),
                    serde_json::Value::String(ec2_subnet_id.clone()),
                );
            }
            if let Some(iam_instance_profile) = &ec2_instance_attributes.iam_instance_profile {
                ec2_json.insert(
                    "IamInstanceProfile".to_string(),
                    serde_json::Value::String(iam_instance_profile.clone()),
                );
            }
            json.insert(
                "Ec2InstanceAttributes".to_string(),
                serde_json::Value::Object(ec2_json),
            );
        }

        if let Some(instance_collection_type) = &cluster.instance_collection_type {
            json.insert(
                "InstanceCollectionType".to_string(),
                serde_json::Value::String(instance_collection_type.as_str().to_string()),
            );
        }

        if let Some(log_uri) = &cluster.log_uri {
            json.insert(
                "LogUri".to_string(),
                serde_json::Value::String(log_uri.clone()),
            );
        }

        if let Some(requested_ami_version) = &cluster.requested_ami_version {
            json.insert(
                "RequestedAmiVersion".to_string(),
                serde_json::Value::String(requested_ami_version.clone()),
            );
        }

        if let Some(running_ami_version) = &cluster.running_ami_version {
            json.insert(
                "RunningAmiVersion".to_string(),
                serde_json::Value::String(running_ami_version.clone()),
            );
        }

        if let Some(release_label) = &cluster.release_label {
            json.insert(
                "ReleaseLabel".to_string(),
                serde_json::Value::String(release_label.clone()),
            );
        }

        json.insert(
            "AutoTerminate".to_string(),
            serde_json::Value::Bool(cluster.auto_terminate.unwrap_or(false)),
        );
        json.insert(
            "TerminationProtected".to_string(),
            serde_json::Value::Bool(cluster.termination_protected.unwrap_or(false)),
        );
        json.insert(
            "VisibleToAllUsers".to_string(),
            serde_json::Value::Bool(cluster.visible_to_all_users.unwrap_or(false)),
        );

        if let Some(applications) = &cluster.applications {
            if !applications.is_empty() {
                let apps_json: Vec<serde_json::Value> = applications
                    .iter()
                    .map(|app| {
                        let mut app_json = serde_json::Map::new();
                        if let Some(name) = &app.name {
                            app_json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(name.clone()),
                            );
                        }
                        if let Some(version) = &app.version {
                            app_json.insert(
                                "Version".to_string(),
                                serde_json::Value::String(version.clone()),
                            );
                        }
                        serde_json::Value::Object(app_json)
                    })
                    .collect();
                json.insert(
                    "Applications".to_string(),
                    serde_json::Value::Array(apps_json),
                );
            }
        }

        if let Some(tags) = &cluster.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
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
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        serde_json::Value::Object(json)
    }
}
