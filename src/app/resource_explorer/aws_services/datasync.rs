use anyhow::{Result, Context};
use aws_sdk_datasync as datasync;
use std::sync::Arc;
use super::super::credentials::CredentialCoordinator;

pub struct DataSyncService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl DataSyncService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List DataSync tasks
    pub async fn list_tasks(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = datasync::Client::new(&aws_config);
        
        let mut tasks = Vec::new();
        let mut next_token: Option<String> = None;
        
        loop {
            let mut request = client.list_tasks();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }
            
            match request.send().await {
                Ok(response) => {
                    if let Some(task_list) = response.tasks {
                        for task in task_list {
                            let task_json = self.task_to_json(&task);
                            tasks.push(task_json);
                        }
                    }
                    
                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to list DataSync tasks in account {} region {}: {}", account_id, region, e);
                    break;
                }
            }
        }

        Ok(tasks)
    }

    /// Get detailed information for a specific DataSync task
    pub async fn describe_task(
        &self,
        account_id: &str,
        region: &str,
        task_arn: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = datasync::Client::new(&aws_config);
        let response = client
            .describe_task()
            .task_arn(task_arn)
            .send()
            .await?;

        Ok(self.task_details_to_json(&response))
    }

    /// List DataSync locations
    pub async fn list_locations(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let aws_config = self.credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| format!("Failed to create AWS config for account {} in region {}", account_id, region))?;

        let client = datasync::Client::new(&aws_config);
        
        let mut locations = Vec::new();
        let mut next_token: Option<String> = None;
        
        loop {
            let mut request = client.list_locations();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }
            
            match request.send().await {
                Ok(response) => {
                    if let Some(location_list) = response.locations {
                        for location in location_list {
                            let location_json = self.location_to_json(&location);
                            locations.push(location_json);
                        }
                    }
                    
                    next_token = response.next_token;
                    if next_token.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to list DataSync locations in account {} region {}: {}", account_id, region, e);
                    break;
                }
            }
        }

        Ok(locations)
    }

    // JSON conversion methods - CRITICAL: Avoid serde_json::to_value for AWS SDK types
    fn task_to_json(&self, task: &datasync::types::TaskListEntry) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(task_arn) = &task.task_arn {
            json.insert("TaskArn".to_string(), serde_json::Value::String(task_arn.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(task_arn.clone()));
        }

        if let Some(name) = &task.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(status) = &task.status {
            json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
        }

        json.insert("ResourceType".to_string(), serde_json::Value::String("AWS::DataSync::Task".to_string()));

        serde_json::Value::Object(json)
    }

    fn task_details_to_json(&self, response: &datasync::operation::describe_task::DescribeTaskOutput) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(task_arn) = &response.task_arn {
            json.insert("TaskArn".to_string(), serde_json::Value::String(task_arn.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(task_arn.clone()));
        }

        if let Some(name) = &response.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(status) = &response.status {
            json.insert("Status".to_string(), serde_json::Value::String(status.as_str().to_string()));
        }

        if let Some(source_location_arn) = &response.source_location_arn {
            json.insert("SourceLocationArn".to_string(), serde_json::Value::String(source_location_arn.clone()));
        }

        if let Some(destination_location_arn) = &response.destination_location_arn {
            json.insert("DestinationLocationArn".to_string(), serde_json::Value::String(destination_location_arn.clone()));
        }

        if let Some(cloud_watch_log_group_arn) = &response.cloud_watch_log_group_arn {
            json.insert("CloudWatchLogGroupArn".to_string(), serde_json::Value::String(cloud_watch_log_group_arn.clone()));
        }

        if let Some(options) = &response.options {
            let mut options_json = serde_json::Map::new();
            
            if let Some(verify_mode) = &options.verify_mode {
                options_json.insert("VerifyMode".to_string(), serde_json::Value::String(verify_mode.as_str().to_string()));
            }

            if let Some(overwrite_mode) = &options.overwrite_mode {
                options_json.insert("OverwriteMode".to_string(), serde_json::Value::String(overwrite_mode.as_str().to_string()));
            }

            if let Some(atime) = &options.atime {
                options_json.insert("Atime".to_string(), serde_json::Value::String(atime.as_str().to_string()));
            }

            if let Some(mtime) = &options.mtime {
                options_json.insert("Mtime".to_string(), serde_json::Value::String(mtime.as_str().to_string()));
            }

            if let Some(uid) = &options.uid {
                options_json.insert("Uid".to_string(), serde_json::Value::String(uid.as_str().to_string()));
            }

            if let Some(gid) = &options.gid {
                options_json.insert("Gid".to_string(), serde_json::Value::String(gid.as_str().to_string()));
            }

            json.insert("Options".to_string(), serde_json::Value::Object(options_json));
        }

        if let Some(excludes) = &response.excludes {
            let excludes_array: Vec<serde_json::Value> = excludes
                .iter()
                .map(|exclude| {
                    let mut exclude_json = serde_json::Map::new();
                    if let Some(filter_type) = &exclude.filter_type {
                        exclude_json.insert("FilterType".to_string(), serde_json::Value::String(filter_type.as_str().to_string()));
                    }
                    if let Some(value) = &exclude.value {
                        exclude_json.insert("Value".to_string(), serde_json::Value::String(value.clone()));
                    }
                    serde_json::Value::Object(exclude_json)
                })
                .collect();
            json.insert("Excludes".to_string(), serde_json::Value::Array(excludes_array));
        }

        if let Some(schedule) = &response.schedule {
            let mut schedule_json = serde_json::Map::new();
            schedule_json.insert("ScheduleExpression".to_string(), serde_json::Value::String(schedule.schedule_expression.clone()));
            json.insert("Schedule".to_string(), serde_json::Value::Object(schedule_json));
        }

        if let Some(error_code) = &response.error_code {
            json.insert("ErrorCode".to_string(), serde_json::Value::String(error_code.clone()));
        }

        if let Some(error_detail) = &response.error_detail {
            json.insert("ErrorDetail".to_string(), serde_json::Value::String(error_detail.clone()));
        }

        if let Some(creation_time) = &response.creation_time {
            json.insert("CreationTime".to_string(), serde_json::Value::String(creation_time.fmt(aws_smithy_types::date_time::Format::DateTime).unwrap_or_default()));
        }

        json.insert("ResourceType".to_string(), serde_json::Value::String("AWS::DataSync::Task".to_string()));

        serde_json::Value::Object(json)
    }

    fn location_to_json(&self, location: &datasync::types::LocationListEntry) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        
        if let Some(location_arn) = &location.location_arn {
            json.insert("LocationArn".to_string(), serde_json::Value::String(location_arn.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(location_arn.clone()));
        }

        if let Some(location_uri) = &location.location_uri {
            json.insert("LocationUri".to_string(), serde_json::Value::String(location_uri.clone()));
        }

        json.insert("ResourceType".to_string(), serde_json::Value::String("AWS::DataSync::Location".to_string()));

        serde_json::Value::Object(json)
    }
}