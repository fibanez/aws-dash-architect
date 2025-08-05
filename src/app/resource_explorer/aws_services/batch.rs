use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_batch as batch;
use std::sync::Arc;

pub struct BatchService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl BatchService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List AWS Batch Job Queues
    pub async fn list_job_queues(
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

        let client = batch::Client::new(&aws_config);

        let mut job_queues = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.describe_job_queues().max_results(100);
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(queues) = response.job_queues {
                for queue in queues {
                    let queue_json = self.job_queue_to_json(&queue);
                    job_queues.push(queue_json);
                }
            }

            if let Some(token) = response.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        Ok(job_queues)
    }

    /// List AWS Batch Compute Environments
    pub async fn list_compute_environments(
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

        let client = batch::Client::new(&aws_config);

        let mut compute_environments = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.describe_compute_environments().max_results(100);
            if let Some(ref token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if let Some(environments) = response.compute_environments {
                for env in environments {
                    let env_json = self.compute_environment_to_json(&env);
                    compute_environments.push(env_json);
                }
            }

            if let Some(token) = response.next_token {
                next_token = Some(token);
            } else {
                break;
            }
        }

        Ok(compute_environments)
    }

    /// Get detailed information for specific Job Queue
    pub async fn describe_job_queue(
        &self,
        account_id: &str,
        region: &str,
        job_queue_name: &str,
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

        let client = batch::Client::new(&aws_config);

        let response = client
            .describe_job_queues()
            .job_queues(job_queue_name)
            .send()
            .await?;

        if let Some(job_queues) = response.job_queues {
            if let Some(queue) = job_queues.first() {
                return Ok(self.job_queue_to_json(queue));
            }
        }

        Err(anyhow::anyhow!("Job Queue {} not found", job_queue_name))
    }

    /// Get detailed information for specific Compute Environment
    pub async fn describe_compute_environment(
        &self,
        account_id: &str,
        region: &str,
        compute_environment_name: &str,
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

        let client = batch::Client::new(&aws_config);

        let response = client
            .describe_compute_environments()
            .compute_environments(compute_environment_name)
            .send()
            .await?;

        if let Some(compute_environments) = response.compute_environments {
            if let Some(env) = compute_environments.first() {
                return Ok(self.compute_environment_to_json(env));
            }
        }

        Err(anyhow::anyhow!(
            "Compute Environment {} not found",
            compute_environment_name
        ))
    }

    fn job_queue_to_json(&self, queue: &batch::types::JobQueueDetail) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(job_queue_name) = &queue.job_queue_name {
            json.insert(
                "JobQueueName".to_string(),
                serde_json::Value::String(job_queue_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(job_queue_name.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(job_queue_name.clone()),
            );
        }

        if let Some(job_queue_arn) = &queue.job_queue_arn {
            json.insert(
                "JobQueueArn".to_string(),
                serde_json::Value::String(job_queue_arn.clone()),
            );
        }

        if let Some(state) = &queue.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(format!("{:?}", state)),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(format!("{:?}", state)),
            );
        }

        if let Some(priority) = queue.priority {
            json.insert(
                "Priority".to_string(),
                serde_json::Value::Number(serde_json::Number::from(priority)),
            );
        }

        if let Some(scheduling_policy_arn) = &queue.scheduling_policy_arn {
            json.insert(
                "SchedulingPolicyArn".to_string(),
                serde_json::Value::String(scheduling_policy_arn.clone()),
            );
        }

        // Compute Environment Order
        if let Some(compute_environment_order) = &queue.compute_environment_order {
            let ce_order_array: Vec<serde_json::Value> = compute_environment_order
                .iter()
                .map(|ce| {
                    let mut ce_json = serde_json::Map::new();
                    if let Some(order) = ce.order {
                        ce_json.insert(
                            "Order".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(order)),
                        );
                    }
                    if let Some(compute_environment) = &ce.compute_environment {
                        ce_json.insert(
                            "ComputeEnvironment".to_string(),
                            serde_json::Value::String(compute_environment.clone()),
                        );
                    }
                    serde_json::Value::Object(ce_json)
                })
                .collect();
            json.insert(
                "ComputeEnvironmentOrder".to_string(),
                serde_json::Value::Array(ce_order_array),
            );
        }

        // Job State Time Limit Actions
        if let Some(job_state_time_limit_actions) = &queue.job_state_time_limit_actions {
            let actions_array: Vec<serde_json::Value> = job_state_time_limit_actions
                .iter()
                .map(|action| {
                    let mut action_json = serde_json::Map::new();
                    if let Some(reason) = &action.reason {
                        action_json.insert(
                            "Reason".to_string(),
                            serde_json::Value::String(reason.clone()),
                        );
                    }
                    action_json.insert(
                        "State".to_string(),
                        serde_json::Value::String(format!("{:?}", action.state)),
                    );
                    action_json.insert(
                        "Action".to_string(),
                        serde_json::Value::String(format!("{:?}", action.action)),
                    );
                    serde_json::Value::Object(action_json)
                })
                .collect();
            json.insert(
                "JobStateTimeLimitActions".to_string(),
                serde_json::Value::Array(actions_array),
            );
        }

        // Tags
        if let Some(tags) = &queue.tags {
            let tags_json: serde_json::Map<String, serde_json::Value> = tags
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        serde_json::Value::Object(json)
    }

    fn compute_environment_to_json(
        &self,
        env: &batch::types::ComputeEnvironmentDetail,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(compute_environment_name) = &env.compute_environment_name {
            json.insert(
                "ComputeEnvironmentName".to_string(),
                serde_json::Value::String(compute_environment_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(compute_environment_name.clone()),
            );
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(compute_environment_name.clone()),
            );
        }

        if let Some(compute_environment_arn) = &env.compute_environment_arn {
            json.insert(
                "ComputeEnvironmentArn".to_string(),
                serde_json::Value::String(compute_environment_arn.clone()),
            );
        }

        if let Some(ecs_cluster_arn) = &env.ecs_cluster_arn {
            json.insert(
                "EcsClusterArn".to_string(),
                serde_json::Value::String(ecs_cluster_arn.clone()),
            );
        }

        if let Some(type_field) = &env.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(format!("{:?}", type_field)),
            );
        }

        if let Some(state) = &env.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(format!("{:?}", state)),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(format!("{:?}", state)),
            );
        }

        if let Some(status) = &env.status {
            json.insert(
                "StatusReason".to_string(),
                serde_json::Value::String(format!("{:?}", status)),
            );
        }

        if let Some(status_reason) = &env.status_reason {
            json.insert(
                "StatusReasonDetail".to_string(),
                serde_json::Value::String(status_reason.clone()),
            );
        }

        if let Some(compute_resources) = &env.compute_resources {
            let mut cr_json = serde_json::Map::new();

            if let Some(type_field) = &compute_resources.r#type {
                cr_json.insert(
                    "Type".to_string(),
                    serde_json::Value::String(format!("{:?}", type_field)),
                );
            }

            if let Some(allocation_strategy) = &compute_resources.allocation_strategy {
                cr_json.insert(
                    "AllocationStrategy".to_string(),
                    serde_json::Value::String(format!("{:?}", allocation_strategy)),
                );
            }

            if let Some(min_vcpus) = compute_resources.minv_cpus {
                cr_json.insert(
                    "MinvCpus".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(min_vcpus)),
                );
            }

            if let Some(max_vcpus) = compute_resources.maxv_cpus {
                cr_json.insert(
                    "MaxvCpus".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(max_vcpus)),
                );
            }

            if let Some(desired_vcpus) = compute_resources.desiredv_cpus {
                cr_json.insert(
                    "DesiredvCpus".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(desired_vcpus)),
                );
            }

            if let Some(instance_types) = &compute_resources.instance_types {
                cr_json.insert(
                    "InstanceTypes".to_string(),
                    serde_json::Value::Array(
                        instance_types
                            .iter()
                            .map(|t| serde_json::Value::String(t.clone()))
                            .collect(),
                    ),
                );
            }

            if let Some(subnets) = &compute_resources.subnets {
                cr_json.insert(
                    "Subnets".to_string(),
                    serde_json::Value::Array(
                        subnets
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }

            if let Some(security_group_ids) = &compute_resources.security_group_ids {
                cr_json.insert(
                    "SecurityGroupIds".to_string(),
                    serde_json::Value::Array(
                        security_group_ids
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }

            if let Some(instance_role) = &compute_resources.instance_role {
                cr_json.insert(
                    "InstanceRole".to_string(),
                    serde_json::Value::String(instance_role.clone()),
                );
            }

            json.insert(
                "ComputeResources".to_string(),
                serde_json::Value::Object(cr_json),
            );
        }

        if let Some(service_role) = &env.service_role {
            json.insert(
                "ServiceRole".to_string(),
                serde_json::Value::String(service_role.clone()),
            );
        }

        // Tags
        if let Some(tags) = &env.tags {
            let tags_json: serde_json::Map<String, serde_json::Value> = tags
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        serde_json::Value::Object(json)
    }
}
