use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_glue as glue;
use std::sync::Arc;

pub struct GlueService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl GlueService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Glue Jobs
    pub async fn list_jobs(
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

        let client = glue::Client::new(&aws_config);
        let mut paginator = client.get_jobs().into_paginator().send();

        let mut jobs = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(job_list) = page.jobs {
                for job in job_list {
                    let job_json = self.job_to_json(&job);
                    jobs.push(job_json);
                }
            }
        }

        Ok(jobs)
    }

    /// Get detailed information for specific Glue job
    pub async fn describe_job(
        &self,
        account_id: &str,
        region: &str,
        job_name: &str,
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

        let client = glue::Client::new(&aws_config);
        let response = client.get_job().job_name(job_name).send().await?;

        if let Some(job) = response.job {
            Ok(self.job_to_json(&job))
        } else {
            Err(anyhow::anyhow!("Job {} not found", job_name))
        }
    }

    fn job_to_json(&self, job: &glue::types::Job) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &job.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(description) = &job.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(log_uri) = &job.log_uri {
            json.insert(
                "LogUri".to_string(),
                serde_json::Value::String(log_uri.clone()),
            );
        }

        if let Some(role) = &job.role {
            json.insert("Role".to_string(), serde_json::Value::String(role.clone()));
        }

        if let Some(created_on) = job.created_on {
            json.insert(
                "CreatedOn".to_string(),
                serde_json::Value::String(created_on.to_string()),
            );
        }

        if let Some(last_modified_on) = job.last_modified_on {
            json.insert(
                "LastModifiedOn".to_string(),
                serde_json::Value::String(last_modified_on.to_string()),
            );
        }

        if let Some(execution_property) = &job.execution_property {
            let mut exec_json = serde_json::Map::new();
            exec_json.insert(
                "MaxConcurrentRuns".to_string(),
                serde_json::Value::Number(execution_property.max_concurrent_runs.into()),
            );
            json.insert(
                "ExecutionProperty".to_string(),
                serde_json::Value::Object(exec_json),
            );
        }

        if let Some(command) = &job.command {
            let mut command_json = serde_json::Map::new();
            if let Some(name) = &command.name {
                command_json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            }
            if let Some(script_location) = &command.script_location {
                command_json.insert(
                    "ScriptLocation".to_string(),
                    serde_json::Value::String(script_location.clone()),
                );
            }
            if let Some(python_version) = &command.python_version {
                command_json.insert(
                    "PythonVersion".to_string(),
                    serde_json::Value::String(python_version.clone()),
                );
            }
            json.insert(
                "Command".to_string(),
                serde_json::Value::Object(command_json),
            );
        }

        if let Some(default_arguments) = &job.default_arguments {
            let args_json = serde_json::Map::from_iter(
                default_arguments
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))),
            );
            json.insert(
                "DefaultArguments".to_string(),
                serde_json::Value::Object(args_json),
            );
        }

        if let Some(non_overridable_arguments) = &job.non_overridable_arguments {
            let args_json = serde_json::Map::from_iter(
                non_overridable_arguments
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))),
            );
            json.insert(
                "NonOverridableArguments".to_string(),
                serde_json::Value::Object(args_json),
            );
        }

        if let Some(connections) = &job.connections {
            if let Some(connections_list) = &connections.connections {
                let connections_json: Vec<serde_json::Value> = connections_list
                    .iter()
                    .map(|conn| serde_json::Value::String(conn.clone()))
                    .collect();
                json.insert(
                    "Connections".to_string(),
                    serde_json::Value::Array(connections_json),
                );
            }
        }

        json.insert(
            "MaxRetries".to_string(),
            serde_json::Value::Number(job.max_retries.into()),
        );

        if let Some(timeout) = job.timeout {
            json.insert(
                "Timeout".to_string(),
                serde_json::Value::Number(timeout.into()),
            );
        }

        if let Some(max_capacity) = job.max_capacity {
            json.insert(
                "MaxCapacity".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(max_capacity)
                        .unwrap_or_else(|| serde_json::Number::from(0)),
                ),
            );
        }

        if let Some(worker_type) = &job.worker_type {
            json.insert(
                "WorkerType".to_string(),
                serde_json::Value::String(worker_type.as_str().to_string()),
            );
        }

        if let Some(number_of_workers) = job.number_of_workers {
            json.insert(
                "NumberOfWorkers".to_string(),
                serde_json::Value::Number(number_of_workers.into()),
            );
        }

        if let Some(security_configuration) = &job.security_configuration {
            json.insert(
                "SecurityConfiguration".to_string(),
                serde_json::Value::String(security_configuration.clone()),
            );
        }

        if let Some(notification_property) = &job.notification_property {
            let mut notification_json = serde_json::Map::new();
            if let Some(notify_delay_after) = notification_property.notify_delay_after {
                notification_json.insert(
                    "NotifyDelayAfter".to_string(),
                    serde_json::Value::Number(notify_delay_after.into()),
                );
            }
            json.insert(
                "NotificationProperty".to_string(),
                serde_json::Value::Object(notification_json),
            );
        }

        if let Some(glue_version) = &job.glue_version {
            json.insert(
                "GlueVersion".to_string(),
                serde_json::Value::String(glue_version.clone()),
            );
        }

        // Add a status field for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("AVAILABLE".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
