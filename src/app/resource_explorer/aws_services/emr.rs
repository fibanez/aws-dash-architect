use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_emr as emr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

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
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("EMR", "list_clusters", Some(region));

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
                    if include_details {
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
                    } else {
                        // Phase 1: Just basic cluster summary
                        let cluster_json = self.cluster_summary_to_json(&cluster);
                        clusters.push(cluster_json);
                    }
                }
            }
        }

        report_status_done("EMR", "list_clusters", Some(region));
        Ok(clusters)
    }

    /// Get detailed information for a single EMR cluster (Phase 2 enrichment)
    pub async fn get_cluster_details(
        &self,
        account_id: &str,
        region: &str,
        cluster_id: &str,
    ) -> Result<serde_json::Value> {
        report_status("EMR", "get_cluster_details", Some(cluster_id));

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
        let mut details = serde_json::Map::new();

        // Get cluster details
        report_status("EMR", "describe_cluster", Some(cluster_id));
        if let Ok(cluster_info) = self.describe_cluster_internal(&client, cluster_id).await {
            if let Some(obj) = cluster_info.as_object() {
                for (key, value) in obj {
                    details.insert(key.clone(), value.clone());
                }
            }
        }

        // Get instance groups
        report_status("EMR", "list_instance_groups", Some(cluster_id));
        if let Ok(instance_groups) = self
            .list_instance_groups_internal(&client, cluster_id)
            .await
        {
            details.insert("InstanceGroups".to_string(), instance_groups);
        }

        // Get steps
        report_status("EMR", "list_steps", Some(cluster_id));
        if let Ok(steps) = self.list_steps_internal(&client, cluster_id).await {
            details.insert("Steps".to_string(), steps);
        }

        report_status_done("EMR", "get_cluster_details", Some(cluster_id));
        Ok(serde_json::Value::Object(details))
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

    // ============= Internal Helper Functions for Detail Fetching =============

    /// Internal: List instance groups for a cluster
    async fn list_instance_groups_internal(
        &self,
        client: &emr::Client,
        cluster_id: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.list_instance_groups().cluster_id(cluster_id).send(),
        )
        .await
        .with_context(|| "list_instance_groups timed out")?
        .with_context(|| "Failed to list instance groups")?;

        let mut groups = Vec::new();
        if let Some(instance_groups) = response.instance_groups {
            for group in instance_groups {
                let mut g_json = serde_json::Map::new();

                if let Some(id) = &group.id {
                    g_json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
                }

                if let Some(name) = &group.name {
                    g_json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
                }

                if let Some(market) = &group.market {
                    g_json.insert(
                        "Market".to_string(),
                        serde_json::Value::String(market.as_str().to_string()),
                    );
                }

                if let Some(instance_group_type) = &group.instance_group_type {
                    g_json.insert(
                        "InstanceGroupType".to_string(),
                        serde_json::Value::String(instance_group_type.as_str().to_string()),
                    );
                }

                if let Some(instance_type) = &group.instance_type {
                    g_json.insert(
                        "InstanceType".to_string(),
                        serde_json::Value::String(instance_type.clone()),
                    );
                }

                if let Some(requested_instance_count) = group.requested_instance_count {
                    g_json.insert(
                        "RequestedInstanceCount".to_string(),
                        serde_json::Value::Number(requested_instance_count.into()),
                    );
                }

                if let Some(running_instance_count) = group.running_instance_count {
                    g_json.insert(
                        "RunningInstanceCount".to_string(),
                        serde_json::Value::Number(running_instance_count.into()),
                    );
                }

                if let Some(status) = &group.status {
                    if let Some(state) = &status.state {
                        g_json.insert(
                            "State".to_string(),
                            serde_json::Value::String(state.as_str().to_string()),
                        );
                    }
                }

                // EBS config
                if let Some(ebs_block_devices) = &group.ebs_block_devices {
                    let ebs_json: Vec<serde_json::Value> = ebs_block_devices
                        .iter()
                        .map(|ebs| {
                            let mut e = serde_json::Map::new();
                            if let Some(device) = &ebs.device {
                                e.insert(
                                    "Device".to_string(),
                                    serde_json::Value::String(device.clone()),
                                );
                            }
                            if let Some(spec) = &ebs.volume_specification {
                                if let Some(vol_type) = &spec.volume_type {
                                    e.insert(
                                        "VolumeType".to_string(),
                                        serde_json::Value::String(vol_type.clone()),
                                    );
                                }
                                if let Some(size) = spec.size_in_gb {
                                    e.insert(
                                        "SizeInGB".to_string(),
                                        serde_json::Value::Number(size.into()),
                                    );
                                }
                            }
                            serde_json::Value::Object(e)
                        })
                        .collect();
                    g_json.insert(
                        "EbsBlockDevices".to_string(),
                        serde_json::Value::Array(ebs_json),
                    );
                }

                groups.push(serde_json::Value::Object(g_json));
            }
        }

        Ok(serde_json::Value::Array(groups))
    }

    /// Internal: List steps for a cluster
    async fn list_steps_internal(
        &self,
        client: &emr::Client,
        cluster_id: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.list_steps().cluster_id(cluster_id).send(),
        )
        .await
        .with_context(|| "list_steps timed out")?
        .with_context(|| "Failed to list steps")?;

        let mut steps = Vec::new();
        if let Some(step_list) = response.steps {
            for step in step_list {
                let mut s_json = serde_json::Map::new();

                if let Some(id) = &step.id {
                    s_json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
                }

                if let Some(name) = &step.name {
                    s_json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
                }

                if let Some(action_on_failure) = &step.action_on_failure {
                    s_json.insert(
                        "ActionOnFailure".to_string(),
                        serde_json::Value::String(action_on_failure.as_str().to_string()),
                    );
                }

                if let Some(status) = &step.status {
                    let mut status_json = serde_json::Map::new();
                    if let Some(state) = &status.state {
                        status_json.insert(
                            "State".to_string(),
                            serde_json::Value::String(state.as_str().to_string()),
                        );
                    }
                    if let Some(state_change_reason) = &status.state_change_reason {
                        if let Some(message) = &state_change_reason.message {
                            status_json.insert(
                                "StateChangeReasonMessage".to_string(),
                                serde_json::Value::String(message.clone()),
                            );
                        }
                    }
                    if let Some(timeline) = &status.timeline {
                        if let Some(creation_time) = &timeline.creation_date_time {
                            status_json.insert(
                                "CreationDateTime".to_string(),
                                serde_json::Value::String(creation_time.to_string()),
                            );
                        }
                        if let Some(start_time) = &timeline.start_date_time {
                            status_json.insert(
                                "StartDateTime".to_string(),
                                serde_json::Value::String(start_time.to_string()),
                            );
                        }
                        if let Some(end_time) = &timeline.end_date_time {
                            status_json.insert(
                                "EndDateTime".to_string(),
                                serde_json::Value::String(end_time.to_string()),
                            );
                        }
                    }
                    s_json.insert("Status".to_string(), serde_json::Value::Object(status_json));
                }

                // Config (JAR, arguments)
                if let Some(config) = &step.config {
                    let mut config_json = serde_json::Map::new();
                    if let Some(jar) = &config.jar {
                        config_json
                            .insert("Jar".to_string(), serde_json::Value::String(jar.clone()));
                    }
                    if let Some(main_class) = &config.main_class {
                        config_json.insert(
                            "MainClass".to_string(),
                            serde_json::Value::String(main_class.clone()),
                        );
                    }
                    if let Some(args) = &config.args {
                        let args_json: Vec<serde_json::Value> = args
                            .iter()
                            .map(|a| serde_json::Value::String(a.clone()))
                            .collect();
                        config_json.insert("Args".to_string(), serde_json::Value::Array(args_json));
                    }
                    s_json.insert("Config".to_string(), serde_json::Value::Object(config_json));
                }

                steps.push(serde_json::Value::Object(s_json));
            }
        }

        Ok(serde_json::Value::Array(steps))
    }
}
