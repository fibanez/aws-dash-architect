use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_kinesis as kinesis;
use std::sync::Arc;

pub struct KinesisService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl KinesisService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Kinesis Data Streams
    pub async fn list_streams(
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

        let client = kinesis::Client::new(&aws_config);
        let mut streams = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_streams();
            if let Some(token) = &next_token {
                request = request.next_token(token);
            }

            let response = request.send().await?;

            if !response.stream_names.is_empty() {
                for stream_name in response.stream_names {
                    // Get detailed stream information
                    if let Ok(stream_details) =
                        self.describe_stream_internal(&client, &stream_name).await
                    {
                        streams.push(stream_details);
                    } else {
                        // Fallback to basic stream info if describe fails
                        let mut basic_stream = serde_json::Map::new();
                        basic_stream.insert(
                            "StreamName".to_string(),
                            serde_json::Value::String(stream_name.clone()),
                        );
                        basic_stream
                            .insert("Name".to_string(), serde_json::Value::String(stream_name));
                        basic_stream.insert(
                            "Status".to_string(),
                            serde_json::Value::String("UNKNOWN".to_string()),
                        );
                        streams.push(serde_json::Value::Object(basic_stream));
                    }
                }
            }

            if response.has_more_streams && response.next_token.is_some() {
                next_token = response.next_token;
            } else {
                break;
            }
        }

        Ok(streams)
    }

    /// Get detailed information for specific Kinesis stream
    pub async fn describe_stream(
        &self,
        account_id: &str,
        region: &str,
        stream_name: &str,
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

        let client = kinesis::Client::new(&aws_config);
        self.describe_stream_internal(&client, stream_name).await
    }

    async fn describe_stream_internal(
        &self,
        client: &kinesis::Client,
        stream_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_stream()
            .stream_name(stream_name)
            .send()
            .await?;

        if let Some(stream_description) = response.stream_description {
            Ok(self.stream_to_json(&stream_description))
        } else {
            Err(anyhow::anyhow!("Stream {} not found", stream_name))
        }
    }

    fn stream_to_json(&self, stream: &kinesis::types::StreamDescription) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "StreamName".to_string(),
            serde_json::Value::String(stream.stream_name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(stream.stream_name.clone()),
        );

        json.insert(
            "StreamArn".to_string(),
            serde_json::Value::String(stream.stream_arn.clone()),
        );

        json.insert(
            "StreamStatus".to_string(),
            serde_json::Value::String(stream.stream_status.as_str().to_string()),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String(stream.stream_status.as_str().to_string()),
        );

        if let Some(stream_mode_details) = &stream.stream_mode_details {
            json.insert(
                "StreamMode".to_string(),
                serde_json::Value::String(stream_mode_details.stream_mode.as_str().to_string()),
            );
        }

        json.insert(
            "StreamCreationTimestamp".to_string(),
            serde_json::Value::String(stream.stream_creation_timestamp.to_string()),
        );

        if !stream.shards.is_empty() {
            json.insert(
                "ShardCount".to_string(),
                serde_json::Value::Number(stream.shards.len().into()),
            );

            let shards_json: Vec<serde_json::Value> = stream
                .shards
                .iter()
                .map(|shard| {
                    let mut shard_json = serde_json::Map::new();
                    shard_json.insert(
                        "ShardId".to_string(),
                        serde_json::Value::String(shard.shard_id.clone()),
                    );

                    if let Some(hash_key_range) = &shard.hash_key_range {
                        let mut range_json = serde_json::Map::new();
                        range_json.insert(
                            "StartingHashKey".to_string(),
                            serde_json::Value::String(hash_key_range.starting_hash_key.clone()),
                        );
                        range_json.insert(
                            "EndingHashKey".to_string(),
                            serde_json::Value::String(hash_key_range.ending_hash_key.clone()),
                        );
                        shard_json.insert(
                            "HashKeyRange".to_string(),
                            serde_json::Value::Object(range_json),
                        );
                    }
                    serde_json::Value::Object(shard_json)
                })
                .collect();
            json.insert("Shards".to_string(), serde_json::Value::Array(shards_json));
        }

        json.insert(
            "RetentionPeriodHours".to_string(),
            serde_json::Value::Number(stream.retention_period_hours.into()),
        );

        if let Some(encryption_type) = &stream.encryption_type {
            json.insert(
                "EncryptionType".to_string(),
                serde_json::Value::String(encryption_type.as_str().to_string()),
            );
        }

        if let Some(key_id) = &stream.key_id {
            json.insert(
                "KeyId".to_string(),
                serde_json::Value::String(key_id.clone()),
            );
        }

        serde_json::Value::Object(json)
    }
}
