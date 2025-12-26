use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_rekognition as rekognition;
use std::sync::Arc;

pub struct RekognitionService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl RekognitionService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Rekognition collections (basic list data)
    pub async fn list_collections(
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

        let client = rekognition::Client::new(&aws_config);

        let mut paginator = client.list_collections().into_paginator().send();

        let mut collections = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(collection_ids) = page.collection_ids {
                for collection_id in collection_ids {
                    let collection_json = self.collection_to_json(&collection_id);
                    collections.push(collection_json);
                }
            }
        }

        Ok(collections)
    }

    /// List Rekognition stream processors (basic list data)
    pub async fn list_stream_processors(
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

        let client = rekognition::Client::new(&aws_config);

        let mut paginator = client.list_stream_processors().into_paginator().send();

        let mut processors = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(stream_processors) = page.stream_processors {
                for processor in stream_processors {
                    let processor_json = self.stream_processor_to_json(&processor);
                    processors.push(processor_json);
                }
            }
        }

        Ok(processors)
    }

    /// Get detailed information for specific Rekognition collection (for describe functionality)
    pub async fn describe_collection(
        &self,
        account_id: &str,
        region: &str,
        collection_id: &str,
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

        let client = rekognition::Client::new(&aws_config);
        self.describe_collection_internal(&client, collection_id)
            .await
    }

    /// Get detailed information for specific stream processor (for describe functionality)
    pub async fn describe_stream_processor(
        &self,
        account_id: &str,
        region: &str,
        stream_processor_name: &str,
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

        let client = rekognition::Client::new(&aws_config);
        self.describe_stream_processor_internal(&client, stream_processor_name)
            .await
    }

    async fn describe_collection_internal(
        &self,
        client: &rekognition::Client,
        collection_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_collection()
            .collection_id(collection_id)
            .send()
            .await?;

        let mut json = serde_json::Map::new();

        json.insert(
            "CollectionId".to_string(),
            serde_json::Value::String(collection_id.to_string()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(collection_id.to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(collection_id.to_string()),
        );

        if let Some(collection_arn) = response.collection_arn {
            json.insert(
                "CollectionARN".to_string(),
                serde_json::Value::String(collection_arn),
            );
        }

        if let Some(face_count) = response.face_count {
            json.insert(
                "FaceCount".to_string(),
                serde_json::Value::Number(serde_json::Number::from(face_count)),
            );
        }

        if let Some(face_model_version) = response.face_model_version {
            json.insert(
                "FaceModelVersion".to_string(),
                serde_json::Value::String(face_model_version),
            );
        }

        if let Some(creation_timestamp) = response.creation_timestamp {
            json.insert(
                "CreationTimestamp".to_string(),
                serde_json::Value::String(creation_timestamp.to_string()),
            );
        }

        Ok(serde_json::Value::Object(json))
    }

    async fn describe_stream_processor_internal(
        &self,
        client: &rekognition::Client,
        stream_processor_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_stream_processor()
            .name(stream_processor_name)
            .send()
            .await?;

        let mut json = serde_json::Map::new();

        if let Some(name) = response.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert("ResourceId".to_string(), serde_json::Value::String(name));
        }

        if let Some(status) = response.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(status_message) = response.status_message {
            json.insert(
                "StatusMessage".to_string(),
                serde_json::Value::String(status_message),
            );
        }

        if let Some(creation_timestamp) = response.creation_timestamp {
            json.insert(
                "CreationTimestamp".to_string(),
                serde_json::Value::String(creation_timestamp.to_string()),
            );
        }

        if let Some(last_update_timestamp) = response.last_update_timestamp {
            json.insert(
                "LastUpdateTimestamp".to_string(),
                serde_json::Value::String(last_update_timestamp.to_string()),
            );
        }

        if let Some(input) = response.input {
            let mut input_json = serde_json::Map::new();
            if let Some(kinesis_video_stream) = input.kinesis_video_stream {
                input_json.insert(
                    "KinesisVideoStreamArn".to_string(),
                    serde_json::Value::String(kinesis_video_stream.arn.unwrap_or_default()),
                );
            }
            json.insert("Input".to_string(), serde_json::Value::Object(input_json));
        }

        if let Some(output) = response.output {
            let mut output_json = serde_json::Map::new();
            if let Some(kinesis_data_stream) = output.kinesis_data_stream {
                output_json.insert(
                    "KinesisDataStreamArn".to_string(),
                    serde_json::Value::String(kinesis_data_stream.arn.unwrap_or_default()),
                );
            }
            if let Some(s3_destination) = output.s3_destination {
                output_json.insert(
                    "S3Bucket".to_string(),
                    serde_json::Value::String(s3_destination.bucket.unwrap_or_default()),
                );
                if let Some(key_prefix) = s3_destination.key_prefix {
                    output_json.insert(
                        "S3KeyPrefix".to_string(),
                        serde_json::Value::String(key_prefix),
                    );
                }
            }
            json.insert("Output".to_string(), serde_json::Value::Object(output_json));
        }

        if let Some(role_arn) = response.role_arn {
            json.insert("RoleArn".to_string(), serde_json::Value::String(role_arn));
        }

        Ok(serde_json::Value::Object(json))
    }

    fn collection_to_json(&self, collection_id: &str) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "CollectionId".to_string(),
            serde_json::Value::String(collection_id.to_string()),
        );
        json.insert(
            "ResourceId".to_string(),
            serde_json::Value::String(collection_id.to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(collection_id.to_string()),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn stream_processor_to_json(
        &self,
        processor: &rekognition::types::StreamProcessor,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &processor.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(status) = &processor.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        serde_json::Value::Object(json)
    }
}
