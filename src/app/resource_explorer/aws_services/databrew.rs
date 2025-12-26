use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_databrew as databrew;
use std::sync::Arc;

pub struct DataBrewService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl DataBrewService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List DataBrew jobs
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

        let client = databrew::Client::new(&aws_config);
        let mut jobs = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_jobs();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            for job in response.jobs {
                let job_json = self.job_to_json(&job);
                jobs.push(job_json);
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(jobs)
    }

    /// Get detailed information for specific DataBrew job
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

        let client = databrew::Client::new(&aws_config);
        self.describe_job_internal(&client, job_name).await
    }

    async fn describe_job_internal(
        &self,
        client: &databrew::Client,
        job_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client.describe_job().name(job_name).send().await?;

        Ok(self.job_detail_to_json(&response))
    }

    fn job_to_json(&self, job: &databrew::types::Job) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "JobName".to_string(),
            serde_json::Value::String(job.name.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(job.name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(job.name.clone()),
        );

        if let Some(account_id) = &job.account_id {
            json.insert(
                "AccountId".to_string(),
                serde_json::Value::String(account_id.clone()),
            );
        }

        if let Some(created_by) = &job.created_by {
            json.insert(
                "CreatedBy".to_string(),
                serde_json::Value::String(created_by.clone()),
            );
        }

        if let Some(create_date) = job.create_date {
            json.insert(
                "CreateDate".to_string(),
                serde_json::Value::String(create_date.to_string()),
            );
        }

        if let Some(last_modified_date) = job.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        if let Some(last_modified_by) = &job.last_modified_by {
            json.insert(
                "LastModifiedBy".to_string(),
                serde_json::Value::String(last_modified_by.clone()),
            );
        }

        if let Some(job_type) = &job.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(job_type.as_str().to_string()),
            );
        }

        if let Some(project_name) = &job.project_name {
            json.insert(
                "ProjectName".to_string(),
                serde_json::Value::String(project_name.clone()),
            );
        }

        if let Some(dataset_name) = &job.dataset_name {
            json.insert(
                "DatasetName".to_string(),
                serde_json::Value::String(dataset_name.clone()),
            );
        }

        if let Some(role_arn) = &job.role_arn {
            json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(role_arn.clone()),
            );
        }

        if let Some(tags) = &job.tags {
            let tags_json: serde_json::Map<String, serde_json::Value> = tags
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        // Default status for jobs in list
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("UNKNOWN".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn job_detail_to_json(
        &self,
        response: &databrew::operation::describe_job::DescribeJobOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "JobName".to_string(),
            serde_json::Value::String(response.name.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(response.name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        if let Some(create_date) = response.create_date {
            json.insert(
                "CreateDate".to_string(),
                serde_json::Value::String(create_date.to_string()),
            );
        }

        if let Some(created_by) = &response.created_by {
            json.insert(
                "CreatedBy".to_string(),
                serde_json::Value::String(created_by.clone()),
            );
        }

        if let Some(last_modified_date) = response.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        if let Some(last_modified_by) = &response.last_modified_by {
            json.insert(
                "LastModifiedBy".to_string(),
                serde_json::Value::String(last_modified_by.clone()),
            );
        }

        if let Some(project_name) = &response.project_name {
            json.insert(
                "ProjectName".to_string(),
                serde_json::Value::String(project_name.clone()),
            );
        }

        if let Some(recipe_reference) = &response.recipe_reference {
            let mut recipe_json = serde_json::Map::new();
            recipe_json.insert(
                "Name".to_string(),
                serde_json::Value::String(recipe_reference.name.clone()),
            );
            if let Some(recipe_version) = &recipe_reference.recipe_version {
                recipe_json.insert(
                    "RecipeVersion".to_string(),
                    serde_json::Value::String(recipe_version.clone()),
                );
            }
            json.insert(
                "RecipeReference".to_string(),
                serde_json::Value::Object(recipe_json),
            );
        }

        if let Some(dataset_name) = &response.dataset_name {
            json.insert(
                "DatasetName".to_string(),
                serde_json::Value::String(dataset_name.clone()),
            );
        }

        if let Some(encryption_key_arn) = &response.encryption_key_arn {
            json.insert(
                "EncryptionKeyArn".to_string(),
                serde_json::Value::String(encryption_key_arn.clone()),
            );
        }

        if let Some(encryption_mode) = &response.encryption_mode {
            json.insert(
                "EncryptionMode".to_string(),
                serde_json::Value::String(encryption_mode.as_str().to_string()),
            );
        }

        if let Some(log_subscription) = &response.log_subscription {
            json.insert(
                "LogSubscription".to_string(),
                serde_json::Value::String(log_subscription.as_str().to_string()),
            );
        }

        json.insert(
            "MaxCapacity".to_string(),
            serde_json::Value::Number(serde_json::Number::from(response.max_capacity)),
        );

        json.insert(
            "MaxRetries".to_string(),
            serde_json::Value::Number(serde_json::Number::from(response.max_retries)),
        );

        if let Some(outputs) = &response.outputs {
            let outputs_json: Vec<serde_json::Value> = outputs
                .iter()
                .map(|output| {
                    let mut output_json = serde_json::Map::new();
                    // Note: name field not available in Output type
                    if let Some(location) = &output.location {
                        let mut location_json = serde_json::Map::new();
                        location_json.insert(
                            "Bucket".to_string(),
                            serde_json::Value::String(location.bucket.clone()),
                        );
                        if let Some(key) = &location.key {
                            location_json
                                .insert("Key".to_string(), serde_json::Value::String(key.clone()));
                        }
                        output_json.insert(
                            "Location".to_string(),
                            serde_json::Value::Object(location_json),
                        );
                    }
                    if let Some(compression_format) = &output.compression_format {
                        output_json.insert(
                            "CompressionFormat".to_string(),
                            serde_json::Value::String(compression_format.as_str().to_string()),
                        );
                    }
                    if let Some(format) = &output.format {
                        output_json.insert(
                            "Format".to_string(),
                            serde_json::Value::String(format.as_str().to_string()),
                        );
                    }
                    output_json.insert(
                        "Overwrite".to_string(),
                        serde_json::Value::Bool(output.overwrite),
                    );
                    serde_json::Value::Object(output_json)
                })
                .collect();
            json.insert(
                "Outputs".to_string(),
                serde_json::Value::Array(outputs_json),
            );
        }

        if let Some(role_arn) = &response.role_arn {
            json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(role_arn.clone()),
            );
        }

        if let Some(job_type) = &response.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(job_type.as_str().to_string()),
            );
        }

        json.insert(
            "Timeout".to_string(),
            serde_json::Value::Number(serde_json::Number::from(response.timeout)),
        );

        if let Some(tags) = &response.tags {
            let tags_json: serde_json::Map<String, serde_json::Value> = tags
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        // Status is available for detailed view
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("READY".to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// List DataBrew datasets
    pub async fn list_datasets(
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

        let client = databrew::Client::new(&aws_config);
        let mut datasets = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_datasets();
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            for dataset in response.datasets {
                let dataset_json = self.dataset_to_json(&dataset);
                datasets.push(dataset_json);
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(datasets)
    }

    fn dataset_to_json(&self, dataset: &databrew::types::Dataset) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "DatasetName".to_string(),
            serde_json::Value::String(dataset.name.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(dataset.name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(dataset.name.clone()),
        );

        if let Some(account_id) = &dataset.account_id {
            json.insert(
                "AccountId".to_string(),
                serde_json::Value::String(account_id.clone()),
            );
        }

        if let Some(created_by) = &dataset.created_by {
            json.insert(
                "CreatedBy".to_string(),
                serde_json::Value::String(created_by.clone()),
            );
        }

        if let Some(create_date) = dataset.create_date {
            json.insert(
                "CreateDate".to_string(),
                serde_json::Value::String(create_date.to_string()),
            );
        }

        if let Some(last_modified_date) = dataset.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        if let Some(last_modified_by) = &dataset.last_modified_by {
            json.insert(
                "LastModifiedBy".to_string(),
                serde_json::Value::String(last_modified_by.clone()),
            );
        }

        if let Some(source) = &dataset.source {
            json.insert(
                "Source".to_string(),
                serde_json::Value::String(source.as_str().to_string()),
            );
        }

        if let Some(tags) = &dataset.tags {
            let tags_json: serde_json::Map<String, serde_json::Value> = tags
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        // Status for datasets
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("AVAILABLE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    pub async fn describe_dataset(
        &self,
        account: &str,
        region: &str,
        dataset_name: &str,
    ) -> Result<serde_json::Value> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account, region
                )
            })?;

        let client = databrew::Client::new(&aws_config);

        let response = client
            .describe_dataset()
            .name(dataset_name)
            .send()
            .await
            .with_context(|| format!("Failed to describe DataBrew dataset: {}", dataset_name))?;

        Ok(self.dataset_detail_to_json(&response))
    }

    fn dataset_detail_to_json(
        &self,
        response: &databrew::operation::describe_dataset::DescribeDatasetOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "DatasetName".to_string(),
            serde_json::Value::String(response.name.clone()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(response.name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(response.name.clone()),
        );

        // Note: account_id field not available in DescribeDatasetOutput

        if let Some(created_by) = &response.created_by {
            json.insert(
                "CreatedBy".to_string(),
                serde_json::Value::String(created_by.clone()),
            );
        }

        if let Some(create_date) = response.create_date {
            json.insert(
                "CreateDate".to_string(),
                serde_json::Value::String(create_date.to_string()),
            );
        }

        if let Some(last_modified_date) = response.last_modified_date {
            json.insert(
                "LastModifiedDate".to_string(),
                serde_json::Value::String(last_modified_date.to_string()),
            );
        }

        if let Some(last_modified_by) = &response.last_modified_by {
            json.insert(
                "LastModifiedBy".to_string(),
                serde_json::Value::String(last_modified_by.clone()),
            );
        }

        if let Some(source) = &response.source {
            json.insert(
                "Source".to_string(),
                serde_json::Value::String(source.as_str().to_string()),
            );
        }

        if let Some(_input) = &response.input {
            json.insert(
                "Input".to_string(),
                serde_json::Value::String("TODO: Manual conversion needed".to_string()),
            );
        }

        if let Some(_format_options) = &response.format_options {
            json.insert(
                "FormatOptions".to_string(),
                serde_json::Value::String("TODO: Manual conversion needed".to_string()),
            );
        }

        if let Some(_path_options) = &response.path_options {
            json.insert(
                "PathOptions".to_string(),
                serde_json::Value::String("TODO: Manual conversion needed".to_string()),
            );
        }

        if let Some(tags) = &response.tags {
            let tags_json: serde_json::Map<String, serde_json::Value> = tags
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            json.insert("Tags".to_string(), serde_json::Value::Object(tags_json));
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String("AVAILABLE".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
