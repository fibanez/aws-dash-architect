#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Result, Context};
use aws_sdk_codedeploy as codedeploy;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct CodeDeployService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CodeDeployService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CodeDeploy applications
    pub async fn list_applications(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = codedeploy::Client::new(&aws_config);
        let mut applications = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_applications();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(applications_list) = response.applications {
                for app_name in applications_list {
                    // Get application details
                    let app_detail = client
                        .get_application()
                        .application_name(&app_name)
                        .send()
                        .await?;

                    if let Some(app_info) = app_detail.application {
                        let app_json = self.application_to_json(&app_info);
                        applications.push(app_json);
                    }
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(applications)
    }

    /// List CodeDeploy deployment groups
    pub async fn list_deployment_groups(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = codedeploy::Client::new(&aws_config);
        let mut deployment_groups = Vec::new();

        // First get all applications
        let applications = self.list_applications(account_id, region).await?;

        for app in applications {
            if let Some(app_name) = app.get("ApplicationName").and_then(|v| v.as_str()) {
                let mut next_token: Option<String> = None;

                loop {
                    let mut request = client.list_deployment_groups().application_name(app_name);
                    if let Some(token) = next_token {
                        request = request.next_token(token);
                    }

                    let response = request.send().await?;

                    if let Some(deployment_groups_list) = response.deployment_groups {
                        for dg_name in deployment_groups_list {
                            // Get deployment group details
                            let dg_detail = client
                                .get_deployment_group()
                                .application_name(app_name)
                                .deployment_group_name(&dg_name)
                                .send()
                                .await?;

                            if let Some(dg_info) = dg_detail.deployment_group_info {
                                let dg_json = self.deployment_group_to_json(&dg_info, app_name);
                                deployment_groups.push(dg_json);
                            }
                        }
                    }

                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
            }
        }

        Ok(deployment_groups)
    }

    /// Describe CodeDeploy application
    pub async fn describe_application(
        &self,
        account: &str,
        region: &str,
        application_name: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account, region))?;

        let client = codedeploy::Client::new(&aws_config);

        let response = client
            .get_application()
            .application_name(application_name)
            .send()
            .await
            .with_context(|| format!("Failed to describe CodeDeploy application: {}", application_name))?;

        if let Some(application) = response.application {
            Ok(self.application_detail_to_json(&application))
        } else {
            Err(anyhow::anyhow!("Application not found: {}", application_name))
        }
    }

    /// Describe CodeDeploy deployment group
    pub async fn describe_deployment_group(
        &self,
        account: &str,
        region: &str,
        deployment_group_id: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account, region))?;

        let client = codedeploy::Client::new(&aws_config);

        // Parse deployment group ID (format: app_name:dg_name)
        let parts: Vec<&str> = deployment_group_id.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid deployment group ID format: {}", deployment_group_id));
        }

        let app_name = parts[0];
        let dg_name = parts[1];

        let response = client
            .get_deployment_group()
            .application_name(app_name)
            .deployment_group_name(dg_name)
            .send()
            .await
            .with_context(|| format!("Failed to describe CodeDeploy deployment group: {}", deployment_group_id))?;

        if let Some(deployment_group) = response.deployment_group_info {
            Ok(self.deployment_group_detail_to_json(&deployment_group, app_name))
        } else {
            Err(anyhow::anyhow!("Deployment group not found: {}", deployment_group_id))
        }
    }

    fn application_to_json(&self, application: &codedeploy::types::ApplicationInfo) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert("ApplicationName".to_string(), serde_json::Value::String(application.application_name.clone().unwrap_or_default()));
        json.insert("ResourceId".to_string(), serde_json::Value::String(application.application_name.clone().unwrap_or_default()));
        json.insert("Name".to_string(), serde_json::Value::String(application.application_name.clone().unwrap_or_default()));

        if let Some(application_id) = &application.application_id {
            json.insert("ApplicationId".to_string(), serde_json::Value::String(application_id.clone()));
        }

        if let Some(create_time) = application.create_time {
            json.insert("CreateTime".to_string(), serde_json::Value::String(create_time.to_string()));
        }

        json.insert("LinkedToGitHub".to_string(), serde_json::Value::Bool(application.linked_to_git_hub));

        if let Some(git_hub_account_name) = &application.git_hub_account_name {
            json.insert("GitHubAccountName".to_string(), serde_json::Value::String(git_hub_account_name.clone()));
        }

        if let Some(compute_platform) = &application.compute_platform {
            json.insert("ComputePlatform".to_string(), serde_json::Value::String(compute_platform.as_str().to_string()));
        }

        json.insert("Status".to_string(), serde_json::Value::String("ACTIVE".to_string()));

        serde_json::Value::Object(json)
    }

    fn deployment_group_to_json(&self, deployment_group: &codedeploy::types::DeploymentGroupInfo, application_name: &str) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        let dg_id = format!("{}:{}", application_name, deployment_group.deployment_group_name.clone().unwrap_or_default());
        json.insert("DeploymentGroupId".to_string(), serde_json::Value::String(dg_id.clone()));
        json.insert("ResourceId".to_string(), serde_json::Value::String(dg_id));
        json.insert("Name".to_string(), serde_json::Value::String(deployment_group.deployment_group_name.clone().unwrap_or_default()));
        json.insert("DeploymentGroupName".to_string(), serde_json::Value::String(deployment_group.deployment_group_name.clone().unwrap_or_default()));
        json.insert("ApplicationName".to_string(), serde_json::Value::String(application_name.to_string()));

        if let Some(deployment_group_id) = &deployment_group.deployment_group_id {
            json.insert("DeploymentGroupId".to_string(), serde_json::Value::String(deployment_group_id.clone()));
        }

        if let Some(service_role_arn) = &deployment_group.service_role_arn {
            json.insert("ServiceRoleArn".to_string(), serde_json::Value::String(service_role_arn.clone()));
        }

        if let Some(_target_revision) = &deployment_group.target_revision {
            json.insert("TargetRevision".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(deployment_config_name) = &deployment_group.deployment_config_name {
            json.insert("DeploymentConfigName".to_string(), serde_json::Value::String(deployment_config_name.clone()));
        }

        if let Some(compute_platform) = &deployment_group.compute_platform {
            json.insert("ComputePlatform".to_string(), serde_json::Value::String(compute_platform.as_str().to_string()));
        }

        json.insert("Status".to_string(), serde_json::Value::String("ACTIVE".to_string()));

        serde_json::Value::Object(json)
    }

    fn application_detail_to_json(&self, application: &codedeploy::types::ApplicationInfo) -> serde_json::Value {
        self.application_to_json(application)
    }

    fn deployment_group_detail_to_json(&self, deployment_group: &codedeploy::types::DeploymentGroupInfo, application_name: &str) -> serde_json::Value {
        let base_json = self.deployment_group_to_json(deployment_group, application_name);
        let mut json = base_json.as_object().unwrap().clone();

        // Add additional detailed fields
        if let Some(_ec2_tag_filters) = &deployment_group.ec2_tag_filters {
            json.insert("Ec2TagFilters".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(_on_premises_instance_tag_filters) = &deployment_group.on_premises_instance_tag_filters {
            json.insert("OnPremisesInstanceTagFilters".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(auto_scaling_groups) = &deployment_group.auto_scaling_groups {
            let asg_names: Vec<serde_json::Value> = auto_scaling_groups
                .iter()
                .map(|asg| serde_json::Value::String(asg.name.clone().unwrap_or_default()))
                .collect();
            json.insert("AutoScalingGroups".to_string(), serde_json::Value::Array(asg_names));
        }

        if let Some(_trigger_configurations) = &deployment_group.trigger_configurations {
            json.insert("TriggerConfigurations".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(_alarm_configuration) = &deployment_group.alarm_configuration {
            json.insert("AlarmConfiguration".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(_auto_rollback_configuration) = &deployment_group.auto_rollback_configuration {
            json.insert("AutoRollbackConfiguration".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(_deployment_style) = &deployment_group.deployment_style {
            json.insert("DeploymentStyle".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(outdated_instances_strategy) = &deployment_group.outdated_instances_strategy {
            json.insert("OutdatedInstancesStrategy".to_string(), serde_json::Value::String(outdated_instances_strategy.as_str().to_string()));
        }

        if let Some(_blue_green_deployment_configuration) = &deployment_group.blue_green_deployment_configuration {
            json.insert("BlueGreenDeploymentConfiguration".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(_load_balancer_info) = &deployment_group.load_balancer_info {
            json.insert("LoadBalancerInfo".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(_last_successful_deployment) = &deployment_group.last_successful_deployment {
            json.insert("LastSuccessfulDeployment".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        if let Some(_last_attempted_deployment) = &deployment_group.last_attempted_deployment {
            json.insert("LastAttemptedDeployment".to_string(), serde_json::Value::String("TODO: Manual conversion needed".to_string()));
        }

        serde_json::Value::Object(json)
    }
}