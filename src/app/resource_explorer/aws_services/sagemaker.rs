use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_sagemaker as sagemaker;
use std::sync::Arc;

pub struct SageMakerService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl SageMakerService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List SageMaker Endpoints
    pub async fn list_endpoints(
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

        let client = sagemaker::Client::new(&aws_config);
        let mut paginator = client.list_endpoints().into_paginator().send();

        let mut endpoints = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(endpoint_list) = page.endpoints {
                for endpoint in endpoint_list {
                    // Get detailed endpoint information
                    if let Some(endpoint_name) = &endpoint.endpoint_name {
                        if let Ok(endpoint_details) = self
                            .describe_endpoint_internal(&client, endpoint_name)
                            .await
                        {
                            endpoints.push(endpoint_details);
                        } else {
                            // Fallback to basic endpoint info if describe fails
                            let endpoint_json = self.endpoint_summary_to_json(&endpoint);
                            endpoints.push(endpoint_json);
                        }
                    } else {
                        // Fallback to basic endpoint info if no name
                        let endpoint_json = self.endpoint_summary_to_json(&endpoint);
                        endpoints.push(endpoint_json);
                    }
                }
            }
        }

        Ok(endpoints)
    }

    /// Get detailed information for specific SageMaker endpoint
    pub async fn describe_endpoint(
        &self,
        account_id: &str,
        region: &str,
        endpoint_name: &str,
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

        let client = sagemaker::Client::new(&aws_config);
        self.describe_endpoint_internal(&client, endpoint_name)
            .await
    }

    /// List SageMaker Training Jobs
    pub async fn list_training_jobs(
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

        let client = sagemaker::Client::new(&aws_config);
        let mut paginator = client.list_training_jobs().into_paginator().send();

        let mut training_jobs = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(training_job_summaries) = page.training_job_summaries {
                for training_job in training_job_summaries {
                    let training_job_json = self.training_job_summary_to_json(&training_job);
                    training_jobs.push(training_job_json);
                }
            }
        }

        Ok(training_jobs)
    }

    /// Get detailed information for specific SageMaker training job
    pub async fn describe_training_job(
        &self,
        account_id: &str,
        region: &str,
        training_job_name: &str,
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

        let client = sagemaker::Client::new(&aws_config);
        let response = client
            .describe_training_job()
            .training_job_name(training_job_name)
            .send()
            .await?;

        Ok(self.training_job_description_to_json(&response))
    }

    /// List SageMaker Models
    pub async fn list_models(
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

        let client = sagemaker::Client::new(&aws_config);
        let mut paginator = client.list_models().into_paginator().send();

        let mut models = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(model_summaries) = page.models {
                for model in model_summaries {
                    let model_json = self.model_summary_to_json(&model);
                    models.push(model_json);
                }
            }
        }

        Ok(models)
    }

    /// Get detailed information for specific SageMaker model
    pub async fn describe_model(
        &self,
        account_id: &str,
        region: &str,
        model_name: &str,
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

        let client = sagemaker::Client::new(&aws_config);
        let response = client
            .describe_model()
            .model_name(model_name)
            .send()
            .await?;

        Ok(self.model_description_to_json(&response))
    }

    async fn describe_endpoint_internal(
        &self,
        client: &sagemaker::Client,
        endpoint_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_endpoint()
            .endpoint_name(endpoint_name)
            .send()
            .await?;

        Ok(self.endpoint_description_to_json(&response))
    }

    fn endpoint_summary_to_json(
        &self,
        endpoint: &sagemaker::types::EndpointSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(endpoint_name) = &endpoint.endpoint_name {
            json.insert(
                "EndpointName".to_string(),
                serde_json::Value::String(endpoint_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(endpoint_name.clone()),
            );
        }

        if let Some(endpoint_arn) = &endpoint.endpoint_arn {
            json.insert(
                "EndpointArn".to_string(),
                serde_json::Value::String(endpoint_arn.clone()),
            );
        }

        if let Some(endpoint_status) = &endpoint.endpoint_status {
            json.insert(
                "EndpointStatus".to_string(),
                serde_json::Value::String(endpoint_status.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(endpoint_status.as_str().to_string()),
            );
        }

        if let Some(creation_time) = endpoint.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(last_modified_time) = endpoint.last_modified_time {
            json.insert(
                "LastModifiedTime".to_string(),
                serde_json::Value::String(last_modified_time.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn endpoint_description_to_json(
        &self,
        response: &sagemaker::operation::describe_endpoint::DescribeEndpointOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(endpoint_name) = &response.endpoint_name {
            json.insert(
                "EndpointName".to_string(),
                serde_json::Value::String(endpoint_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(endpoint_name.clone()),
            );
        }

        if let Some(endpoint_arn) = &response.endpoint_arn {
            json.insert(
                "EndpointArn".to_string(),
                serde_json::Value::String(endpoint_arn.clone()),
            );
        }

        if let Some(endpoint_config_name) = &response.endpoint_config_name {
            json.insert(
                "EndpointConfigName".to_string(),
                serde_json::Value::String(endpoint_config_name.clone()),
            );
        }

        if let Some(endpoint_status) = &response.endpoint_status {
            json.insert(
                "EndpointStatus".to_string(),
                serde_json::Value::String(endpoint_status.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(endpoint_status.as_str().to_string()),
            );
        }

        if let Some(failure_reason) = &response.failure_reason {
            json.insert(
                "FailureReason".to_string(),
                serde_json::Value::String(failure_reason.clone()),
            );
        }

        if let Some(creation_time) = response.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(last_modified_time) = response.last_modified_time {
            json.insert(
                "LastModifiedTime".to_string(),
                serde_json::Value::String(last_modified_time.to_string()),
            );
        }

        if let Some(production_variants) = &response.production_variants {
            let variants_json: Vec<serde_json::Value> = production_variants
                .iter()
                .map(|variant| {
                    let mut variant_json = serde_json::Map::new();
                    if let Some(variant_name) = &variant.variant_name {
                        variant_json.insert(
                            "VariantName".to_string(),
                            serde_json::Value::String(variant_name.clone()),
                        );
                    }
                    if let Some(deployed_images) = &variant.deployed_images {
                        let images_json: Vec<serde_json::Value> = deployed_images
                            .iter()
                            .map(|image| {
                                let mut image_json = serde_json::Map::new();
                                if let Some(specified_image) = &image.specified_image {
                                    image_json.insert(
                                        "SpecifiedImage".to_string(),
                                        serde_json::Value::String(specified_image.clone()),
                                    );
                                }
                                if let Some(resolved_image) = &image.resolved_image {
                                    image_json.insert(
                                        "ResolvedImage".to_string(),
                                        serde_json::Value::String(resolved_image.clone()),
                                    );
                                }
                                serde_json::Value::Object(image_json)
                            })
                            .collect();
                        variant_json.insert(
                            "DeployedImages".to_string(),
                            serde_json::Value::Array(images_json),
                        );
                    }
                    if let Some(current_weight) = variant.current_weight {
                        if let Some(weight_num) =
                            serde_json::Number::from_f64(current_weight as f64)
                        {
                            variant_json.insert(
                                "CurrentWeight".to_string(),
                                serde_json::Value::Number(weight_num),
                            );
                        }
                    }
                    if let Some(desired_weight) = variant.desired_weight {
                        if let Some(weight_num) =
                            serde_json::Number::from_f64(desired_weight as f64)
                        {
                            variant_json.insert(
                                "DesiredWeight".to_string(),
                                serde_json::Value::Number(weight_num),
                            );
                        }
                    }
                    if let Some(current_instance_count) = variant.current_instance_count {
                        variant_json.insert(
                            "CurrentInstanceCount".to_string(),
                            serde_json::Value::Number(current_instance_count.into()),
                        );
                    }
                    if let Some(desired_instance_count) = variant.desired_instance_count {
                        variant_json.insert(
                            "DesiredInstanceCount".to_string(),
                            serde_json::Value::Number(desired_instance_count.into()),
                        );
                    }
                    serde_json::Value::Object(variant_json)
                })
                .collect();
            json.insert(
                "ProductionVariants".to_string(),
                serde_json::Value::Array(variants_json),
            );
        }

        if let Some(data_capture_config) = &response.data_capture_config {
            let mut capture_json = serde_json::Map::new();
            capture_json.insert(
                "EnableCapture".to_string(),
                serde_json::Value::Bool(data_capture_config.enable_capture.unwrap_or(false)),
            );
            if let Some(destination_s3_uri) = &data_capture_config.destination_s3_uri {
                capture_json.insert(
                    "DestinationS3Uri".to_string(),
                    serde_json::Value::String(destination_s3_uri.clone()),
                );
            }
            json.insert(
                "DataCaptureConfig".to_string(),
                serde_json::Value::Object(capture_json),
            );
        }

        serde_json::Value::Object(json)
    }

    fn training_job_summary_to_json(
        &self,
        training_job: &sagemaker::types::TrainingJobSummary,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(training_job_name) = &training_job.training_job_name {
            json.insert(
                "TrainingJobName".to_string(),
                serde_json::Value::String(training_job_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(training_job_name.clone()),
            );
        }

        if let Some(training_job_arn) = &training_job.training_job_arn {
            json.insert(
                "TrainingJobArn".to_string(),
                serde_json::Value::String(training_job_arn.clone()),
            );
        }

        if let Some(training_job_status) = &training_job.training_job_status {
            json.insert(
                "TrainingJobStatus".to_string(),
                serde_json::Value::String(training_job_status.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(training_job_status.as_str().to_string()),
            );
        }

        if let Some(creation_time) = training_job.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(training_end_time) = training_job.training_end_time {
            json.insert(
                "TrainingEndTime".to_string(),
                serde_json::Value::String(training_end_time.to_string()),
            );
        }

        if let Some(last_modified_time) = training_job.last_modified_time {
            json.insert(
                "LastModifiedTime".to_string(),
                serde_json::Value::String(last_modified_time.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn training_job_description_to_json(
        &self,
        response: &sagemaker::operation::describe_training_job::DescribeTrainingJobOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(training_job_name) = &response.training_job_name {
            json.insert(
                "TrainingJobName".to_string(),
                serde_json::Value::String(training_job_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(training_job_name.clone()),
            );
        }

        if let Some(training_job_arn) = &response.training_job_arn {
            json.insert(
                "TrainingJobArn".to_string(),
                serde_json::Value::String(training_job_arn.clone()),
            );
        }

        if let Some(training_job_status) = &response.training_job_status {
            json.insert(
                "TrainingJobStatus".to_string(),
                serde_json::Value::String(training_job_status.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(training_job_status.as_str().to_string()),
            );
        }

        if let Some(failure_reason) = &response.failure_reason {
            json.insert(
                "FailureReason".to_string(),
                serde_json::Value::String(failure_reason.clone()),
            );
        }

        if let Some(model_artifacts) = &response.model_artifacts {
            if let Some(s3_model_artifacts) = &model_artifacts.s3_model_artifacts {
                json.insert(
                    "ModelArtifacts".to_string(),
                    serde_json::Value::String(s3_model_artifacts.clone()),
                );
            }
        }

        if let Some(role_arn) = &response.role_arn {
            json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(role_arn.clone()),
            );
        }

        if let Some(algorithm_specification) = &response.algorithm_specification {
            let mut algo_json = serde_json::Map::new();
            if let Some(training_image) = &algorithm_specification.training_image {
                algo_json.insert(
                    "TrainingImage".to_string(),
                    serde_json::Value::String(training_image.clone()),
                );
            }
            if let Some(training_input_mode) = &algorithm_specification.training_input_mode {
                algo_json.insert(
                    "TrainingInputMode".to_string(),
                    serde_json::Value::String(training_input_mode.as_str().to_string()),
                );
            }
            if !algo_json.is_empty() {
                json.insert(
                    "AlgorithmSpecification".to_string(),
                    serde_json::Value::Object(algo_json),
                );
            }
        }

        if let Some(creation_time) = response.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(training_start_time) = response.training_start_time {
            json.insert(
                "TrainingStartTime".to_string(),
                serde_json::Value::String(training_start_time.to_string()),
            );
        }

        if let Some(training_end_time) = response.training_end_time {
            json.insert(
                "TrainingEndTime".to_string(),
                serde_json::Value::String(training_end_time.to_string()),
            );
        }

        if let Some(last_modified_time) = response.last_modified_time {
            json.insert(
                "LastModifiedTime".to_string(),
                serde_json::Value::String(last_modified_time.to_string()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn model_summary_to_json(&self, model: &sagemaker::types::ModelSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(model_name) = &model.model_name {
            json.insert(
                "ModelName".to_string(),
                serde_json::Value::String(model_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(model_name.clone()),
            );
        }

        if let Some(model_arn) = &model.model_arn {
            json.insert(
                "ModelArn".to_string(),
                serde_json::Value::String(model_arn.clone()),
            );
        }

        if let Some(creation_time) = model.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Available".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn model_description_to_json(
        &self,
        response: &sagemaker::operation::describe_model::DescribeModelOutput,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(model_name) = &response.model_name {
            json.insert(
                "ModelName".to_string(),
                serde_json::Value::String(model_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(model_name.clone()),
            );
        }

        if let Some(model_arn) = &response.model_arn {
            json.insert(
                "ModelArn".to_string(),
                serde_json::Value::String(model_arn.clone()),
            );
        }

        if let Some(execution_role_arn) = &response.execution_role_arn {
            json.insert(
                "ExecutionRoleArn".to_string(),
                serde_json::Value::String(execution_role_arn.clone()),
            );
        }

        if let Some(primary_container) = &response.primary_container {
            let mut container_json = serde_json::Map::new();
            if let Some(image) = &primary_container.image {
                container_json.insert(
                    "Image".to_string(),
                    serde_json::Value::String(image.clone()),
                );
            }
            if let Some(model_data_url) = &primary_container.model_data_url {
                container_json.insert(
                    "ModelDataUrl".to_string(),
                    serde_json::Value::String(model_data_url.clone()),
                );
            }
            if !container_json.is_empty() {
                json.insert(
                    "PrimaryContainer".to_string(),
                    serde_json::Value::Object(container_json),
                );
            }
        }

        if let Some(vpc_config) = &response.vpc_config {
            let mut vpc_json = serde_json::Map::new();
            if let Some(security_group_ids) = &vpc_config.security_group_ids {
                let security_groups: Vec<serde_json::Value> = security_group_ids
                    .iter()
                    .map(|sg| serde_json::Value::String(sg.clone()))
                    .collect();
                vpc_json.insert(
                    "SecurityGroupIds".to_string(),
                    serde_json::Value::Array(security_groups),
                );
            }
            if let Some(subnets) = &vpc_config.subnets {
                let subnets_json: Vec<serde_json::Value> = subnets
                    .iter()
                    .map(|subnet| serde_json::Value::String(subnet.clone()))
                    .collect();
                vpc_json.insert(
                    "Subnets".to_string(),
                    serde_json::Value::Array(subnets_json),
                );
            }
            if !vpc_json.is_empty() {
                json.insert("VpcConfig".to_string(), serde_json::Value::Object(vpc_json));
            }
        }

        if let Some(creation_time) = response.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Available".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
