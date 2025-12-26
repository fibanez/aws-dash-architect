use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_glue as glue;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

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
        include_details: bool,
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
                    let mut job_json = self.job_to_json(&job);

                    if include_details {
                        if let Some(job_name) = &job.name {
                            report_status("Glue", "get_job_details", Some(job_name));

                            // Fetch job runs
                            if let Ok(job_runs) =
                                self.get_job_runs_internal(&client, job_name).await
                            {
                                if let serde_json::Value::Object(ref mut map) = job_json {
                                    map.insert("JobRuns".to_string(), job_runs);
                                }
                            }

                            // Fetch triggers for this job
                            if let Ok(triggers) =
                                self.get_triggers_internal(&client, job_name).await
                            {
                                if let serde_json::Value::Object(ref mut map) = job_json {
                                    map.insert("Triggers".to_string(), triggers);
                                }
                            }

                            report_status_done("Glue", "get_job_details", Some(job_name));
                        }
                    }

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

    /// Get detailed information for a Glue job (Phase 2 enrichment)
    pub async fn get_job_details(
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

        // Get the job first
        let response = client.get_job().job_name(job_name).send().await?;

        let mut job_json = if let Some(job) = response.job {
            self.job_to_json(&job)
        } else {
            return Err(anyhow::anyhow!("Job {} not found", job_name));
        };

        // Fetch job runs
        if let Ok(job_runs) = self.get_job_runs_internal(&client, job_name).await {
            if let serde_json::Value::Object(ref mut map) = job_json {
                map.insert("JobRuns".to_string(), job_runs);
            }
        }

        // Fetch triggers for this job
        if let Ok(triggers) = self.get_triggers_internal(&client, job_name).await {
            if let serde_json::Value::Object(ref mut map) = job_json {
                map.insert("Triggers".to_string(), triggers);
            }
        }

        Ok(job_json)
    }

    /// Internal helper to get job runs
    async fn get_job_runs_internal(
        &self,
        client: &glue::Client,
        job_name: &str,
    ) -> Result<serde_json::Value> {
        let timeout_duration = Duration::from_secs(30);

        let result = timeout(
            timeout_duration,
            client
                .get_job_runs()
                .job_name(job_name)
                .max_results(10)
                .send(),
        )
        .await
        .with_context(|| format!("Timeout fetching job runs for {}", job_name))?
        .with_context(|| format!("Failed to get job runs for {}", job_name))?;

        let job_runs: Vec<serde_json::Value> = result
            .job_runs
            .unwrap_or_default()
            .into_iter()
            .map(|run| self.job_run_to_json(&run))
            .collect();

        Ok(serde_json::Value::Array(job_runs))
    }

    /// Internal helper to get triggers for a job
    async fn get_triggers_internal(
        &self,
        client: &glue::Client,
        job_name: &str,
    ) -> Result<serde_json::Value> {
        let timeout_duration = Duration::from_secs(30);

        let result = timeout(
            timeout_duration,
            client
                .get_triggers()
                .dependent_job_name(job_name)
                .max_results(25)
                .send(),
        )
        .await
        .with_context(|| format!("Timeout fetching triggers for {}", job_name))?
        .with_context(|| format!("Failed to get triggers for {}", job_name))?;

        let triggers: Vec<serde_json::Value> = result
            .triggers
            .unwrap_or_default()
            .into_iter()
            .map(|trigger| self.trigger_to_json(&trigger))
            .collect();

        Ok(serde_json::Value::Array(triggers))
    }

    /// Convert job run to JSON
    fn job_run_to_json(&self, run: &glue::types::JobRun) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &run.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(job_name) = &run.job_name {
            json.insert(
                "JobName".to_string(),
                serde_json::Value::String(job_name.clone()),
            );
        }

        if let Some(job_run_state) = &run.job_run_state {
            json.insert(
                "JobRunState".to_string(),
                serde_json::Value::String(job_run_state.as_str().to_string()),
            );
        }

        if let Some(started_on) = run.started_on {
            json.insert(
                "StartedOn".to_string(),
                serde_json::Value::String(started_on.to_string()),
            );
        }

        if let Some(completed_on) = run.completed_on {
            json.insert(
                "CompletedOn".to_string(),
                serde_json::Value::String(completed_on.to_string()),
            );
        }

        json.insert(
            "ExecutionTime".to_string(),
            serde_json::Value::Number(run.execution_time.into()),
        );

        if let Some(error_message) = &run.error_message {
            json.insert(
                "ErrorMessage".to_string(),
                serde_json::Value::String(error_message.clone()),
            );
        }

        if let Some(dpu_seconds) = run.dpu_seconds {
            json.insert(
                "DPUSeconds".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(dpu_seconds)
                        .unwrap_or_else(|| serde_json::Number::from(0)),
                ),
            );
        }

        serde_json::Value::Object(json)
    }

    /// Convert trigger to JSON
    fn trigger_to_json(&self, trigger: &glue::types::Trigger) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &trigger.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(workflow_name) = &trigger.workflow_name {
            json.insert(
                "WorkflowName".to_string(),
                serde_json::Value::String(workflow_name.clone()),
            );
        }

        if let Some(trigger_type) = &trigger.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(trigger_type.as_str().to_string()),
            );
        }

        if let Some(state) = &trigger.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(schedule) = &trigger.schedule {
            json.insert(
                "Schedule".to_string(),
                serde_json::Value::String(schedule.clone()),
            );
        }

        if let Some(actions) = &trigger.actions {
            let actions_json: Vec<serde_json::Value> = actions
                .iter()
                .map(|action| {
                    let mut action_json = serde_json::Map::new();
                    if let Some(job_name) = &action.job_name {
                        action_json.insert(
                            "JobName".to_string(),
                            serde_json::Value::String(job_name.clone()),
                        );
                    }
                    if let Some(crawler_name) = &action.crawler_name {
                        action_json.insert(
                            "CrawlerName".to_string(),
                            serde_json::Value::String(crawler_name.clone()),
                        );
                    }
                    if let Some(timeout) = action.timeout {
                        action_json.insert(
                            "Timeout".to_string(),
                            serde_json::Value::Number(timeout.into()),
                        );
                    }
                    serde_json::Value::Object(action_json)
                })
                .collect();
            json.insert(
                "Actions".to_string(),
                serde_json::Value::Array(actions_json),
            );
        }

        serde_json::Value::Object(json)
    }
}
