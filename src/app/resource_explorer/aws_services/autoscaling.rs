use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_autoscaling as autoscaling;
use std::sync::Arc;

pub struct AutoScalingService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl AutoScalingService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Auto Scaling Groups
    pub async fn list_auto_scaling_groups(
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

        let client = autoscaling::Client::new(&aws_config);
        let mut paginator = client.describe_auto_scaling_groups().into_paginator().send();

        let mut groups = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(group_list) = page.auto_scaling_groups {
                for group in group_list {
                    let group_json = self.auto_scaling_group_to_json(&group);
                    groups.push(group_json);
                }
            }
        }

        Ok(groups)
    }

    /// List Scaling Policies
    pub async fn list_scaling_policies(
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

        let client = autoscaling::Client::new(&aws_config);
        let mut paginator = client.describe_policies().into_paginator().send();

        let mut policies = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(policy_list) = page.scaling_policies {
                for policy in policy_list {
                    let policy_json = self.scaling_policy_to_json(&policy);
                    policies.push(policy_json);
                }
            }
        }

        Ok(policies)
    }

    /// Get detailed information for specific Auto Scaling Group
    pub async fn describe_auto_scaling_group(
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

        let client = autoscaling::Client::new(&aws_config);
        let response = client
            .describe_auto_scaling_groups()
            .auto_scaling_group_names(group_name)
            .send()
            .await?;

        if let Some(groups) = response.auto_scaling_groups {
            if let Some(group) = groups.first() {
                Ok(self.auto_scaling_group_to_json(group))
            } else {
                Err(anyhow::anyhow!("Auto Scaling Group {} not found", group_name))
            }
        } else {
            Err(anyhow::anyhow!("Auto Scaling Group {} not found", group_name))
        }
    }

    fn auto_scaling_group_to_json(&self, group: &autoscaling::types::AutoScalingGroup) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(group_name) = &group.auto_scaling_group_name {
            json.insert(
                "AutoScalingGroupName".to_string(),
                serde_json::Value::String(group_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(group_name.clone()),
            );
        }

        if let Some(group_arn) = &group.auto_scaling_group_arn {
            json.insert(
                "AutoScalingGroupARN".to_string(),
                serde_json::Value::String(group_arn.clone()),
            );
        }

        if let Some(launch_config_name) = &group.launch_configuration_name {
            json.insert(
                "LaunchConfigurationName".to_string(),
                serde_json::Value::String(launch_config_name.clone()),
            );
        }

        if let Some(launch_template) = &group.launch_template {
            let mut template_json = serde_json::Map::new();
            if let Some(template_id) = &launch_template.launch_template_id {
                template_json.insert(
                    "LaunchTemplateId".to_string(),
                    serde_json::Value::String(template_id.clone()),
                );
            }
            if let Some(template_name) = &launch_template.launch_template_name {
                template_json.insert(
                    "LaunchTemplateName".to_string(),
                    serde_json::Value::String(template_name.clone()),
                );
            }
            if let Some(version) = &launch_template.version {
                template_json.insert(
                    "Version".to_string(),
                    serde_json::Value::String(version.clone()),
                );
            }
            json.insert(
                "LaunchTemplate".to_string(),
                serde_json::Value::Object(template_json),
            );
        }

        if let Some(min_size) = group.min_size {
            json.insert(
                "MinSize".to_string(),
                serde_json::Value::Number(serde_json::Number::from(min_size)),
            );
        }
        
        if let Some(max_size) = group.max_size {
            json.insert(
                "MaxSize".to_string(),
                serde_json::Value::Number(serde_json::Number::from(max_size)),
            );
        }
        
        if let Some(desired_capacity) = group.desired_capacity {
            json.insert(
                "DesiredCapacity".to_string(),
                serde_json::Value::Number(serde_json::Number::from(desired_capacity)),
            );
        }

        if let Some(default_cooldown) = group.default_cooldown {
            json.insert(
                "DefaultCooldown".to_string(),
                serde_json::Value::Number(serde_json::Number::from(default_cooldown)),
            );
        }

        if let Some(availability_zones) = &group.availability_zones {
            let az_json: Vec<serde_json::Value> = availability_zones
                .iter()
                .map(|az| serde_json::Value::String(az.clone()))
                .collect();
            json.insert(
                "AvailabilityZones".to_string(),
                serde_json::Value::Array(az_json),
            );
        }

        if let Some(load_balancer_names) = &group.load_balancer_names {
            if !load_balancer_names.is_empty() {
                let lb_json: Vec<serde_json::Value> = load_balancer_names
                    .iter()
                    .map(|lb| serde_json::Value::String(lb.clone()))
                    .collect();
                json.insert(
                    "LoadBalancerNames".to_string(),
                    serde_json::Value::Array(lb_json),
                );
            }
        }

        if let Some(target_group_arns) = &group.target_group_arns {
            if !target_group_arns.is_empty() {
                let tg_json: Vec<serde_json::Value> = target_group_arns
                    .iter()
                    .map(|tg| serde_json::Value::String(tg.clone()))
                    .collect();
                json.insert(
                    "TargetGroupARNs".to_string(),
                    serde_json::Value::Array(tg_json),
                );
            }
        }

        if let Some(health_check_type) = &group.health_check_type {
            json.insert(
                "HealthCheckType".to_string(),
                serde_json::Value::String(health_check_type.clone()),
            );
        }

        if let Some(health_check_grace_period) = group.health_check_grace_period {
            json.insert(
                "HealthCheckGracePeriod".to_string(),
                serde_json::Value::Number(serde_json::Number::from(health_check_grace_period)),
            );
        }

        if let Some(instances) = &group.instances {
            json.insert(
                "InstanceCount".to_string(),
                serde_json::Value::Number(serde_json::Number::from(instances.len())),
            );
        }

        if let Some(vpc_zone_identifier) = &group.vpc_zone_identifier {
            if !vpc_zone_identifier.is_empty() {
                json.insert(
                    "VPCZoneIdentifier".to_string(),
                    serde_json::Value::String(vpc_zone_identifier.clone()),
                );
            }
        }

        if let Some(termination_policies) = &group.termination_policies {
            if !termination_policies.is_empty() {
                let tp_json: Vec<serde_json::Value> = termination_policies
                    .iter()
                    .map(|tp| serde_json::Value::String(tp.clone()))
                    .collect();
                json.insert(
                    "TerminationPolicies".to_string(),
                    serde_json::Value::Array(tp_json),
                );
            }
        }

        if let Some(created_time) = group.created_time {
            json.insert(
                "CreatedTime".to_string(),
                serde_json::Value::String(created_time.to_string()),
            );
        }

        if let Some(tags) = &group.tags {
            if !tags.is_empty() {
                let tags_json: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|tag| {
                        let mut tag_json = serde_json::Map::new();
                        if let Some(key) = &tag.key {
                            tag_json.insert("Key".to_string(), serde_json::Value::String(key.clone()));
                        }
                        if let Some(value) = &tag.value {
                            tag_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                        }
                        serde_json::Value::Object(tag_json)
                    })
                    .collect();
                json.insert("Tags".to_string(), serde_json::Value::Array(tags_json));
            }
        }

        // Default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn scaling_policy_to_json(&self, policy: &autoscaling::types::ScalingPolicy) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(policy_name) = &policy.policy_name {
            json.insert(
                "PolicyName".to_string(),
                serde_json::Value::String(policy_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(policy_name.clone()),
            );
        }

        if let Some(policy_arn) = &policy.policy_arn {
            json.insert(
                "PolicyARN".to_string(),
                serde_json::Value::String(policy_arn.clone()),
            );
        }

        if let Some(asg_name) = &policy.auto_scaling_group_name {
            json.insert(
                "AutoScalingGroupName".to_string(),
                serde_json::Value::String(asg_name.clone()),
            );
        }

        if let Some(policy_type) = &policy.policy_type {
            json.insert(
                "PolicyType".to_string(),
                serde_json::Value::String(policy_type.clone()),
            );
        }

        if let Some(adjustment_type) = &policy.adjustment_type {
            json.insert(
                "AdjustmentType".to_string(),
                serde_json::Value::String(adjustment_type.clone()),
            );
        }

        if let Some(scaling_adjustment) = policy.scaling_adjustment {
            json.insert(
                "ScalingAdjustment".to_string(),
                serde_json::Value::Number(serde_json::Number::from(scaling_adjustment)),
            );
        }

        if let Some(cooldown) = policy.cooldown {
            json.insert(
                "Cooldown".to_string(),
                serde_json::Value::Number(serde_json::Number::from(cooldown)),
            );
        }

        if let Some(enabled) = policy.enabled {
            json.insert(
                "Enabled".to_string(),
                serde_json::Value::Bool(enabled),
            );
        }

        // Default status for consistency  
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }
}