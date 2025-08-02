use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_firehose as firehose;
use std::sync::Arc;

pub struct KinesisFirehoseService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl KinesisFirehoseService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Kinesis Data Firehose Delivery Streams
    pub async fn list_delivery_streams(
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

        let client = firehose::Client::new(&aws_config);

        let mut delivery_streams = Vec::new();
        let mut exclusive_start_delivery_stream_name = None;

        loop {
            let mut request = client.list_delivery_streams().limit(25);
            if let Some(ref start_name) = exclusive_start_delivery_stream_name {
                request = request.exclusive_start_delivery_stream_name(start_name);
            }

            let response = request.send().await?;

            if !response.delivery_stream_names.is_empty() {
                for stream_name in &response.delivery_stream_names {
                    // Get detailed stream information
                    if let Ok(stream_details) = self
                        .get_delivery_stream_internal(&client, stream_name)
                        .await
                    {
                        delivery_streams.push(stream_details);
                    } else {
                        // Fallback to basic stream info if describe fails
                        let mut stream_json = serde_json::Map::new();
                        stream_json.insert(
                            "DeliveryStreamName".to_string(),
                            serde_json::Value::String(stream_name.clone()),
                        );
                        stream_json.insert(
                            "Name".to_string(),
                            serde_json::Value::String(stream_name.clone()),
                        );
                        delivery_streams.push(serde_json::Value::Object(stream_json));
                    }
                }
            }

            if response.has_more_delivery_streams && !response.delivery_stream_names.is_empty() {
                if let Some(last_name) = response.delivery_stream_names.last() {
                    exclusive_start_delivery_stream_name = Some(last_name.clone());
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(delivery_streams)
    }

    /// Get detailed information for specific Delivery Stream
    pub async fn describe_delivery_stream(
        &self,
        account_id: &str,
        region: &str,
        delivery_stream_name: &str,
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

        let client = firehose::Client::new(&aws_config);
        self.get_delivery_stream_internal(&client, delivery_stream_name)
            .await
    }

    async fn get_delivery_stream_internal(
        &self,
        client: &firehose::Client,
        delivery_stream_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_delivery_stream()
            .delivery_stream_name(delivery_stream_name)
            .send()
            .await?;

        if let Some(delivery_stream_description) = response.delivery_stream_description {
            Ok(self.delivery_stream_to_json(&delivery_stream_description))
        } else {
            Err(anyhow::anyhow!(
                "Delivery Stream {} not found",
                delivery_stream_name
            ))
        }
    }

    fn delivery_stream_to_json(
        &self,
        stream: &firehose::types::DeliveryStreamDescription,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        json.insert(
            "DeliveryStreamName".to_string(),
            serde_json::Value::String(stream.delivery_stream_name.clone()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(stream.delivery_stream_name.clone()),
        );

        json.insert(
            "DeliveryStreamArn".to_string(),
            serde_json::Value::String(stream.delivery_stream_arn.clone()),
        );

        json.insert(
            "DeliveryStreamStatus".to_string(),
            serde_json::Value::String(format!("{:?}", stream.delivery_stream_status)),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String(format!("{:?}", stream.delivery_stream_status)),
        );

        json.insert(
            "DeliveryStreamType".to_string(),
            serde_json::Value::String(format!("{:?}", stream.delivery_stream_type)),
        );

        json.insert(
            "VersionId".to_string(),
            serde_json::Value::String(stream.version_id.clone()),
        );

        if let Some(create_timestamp) = &stream.create_timestamp {
            json.insert(
                "CreateTimestamp".to_string(),
                serde_json::Value::String(create_timestamp.to_string()),
            );
        }

        if let Some(last_update_timestamp) = &stream.last_update_timestamp {
            json.insert(
                "LastUpdateTimestamp".to_string(),
                serde_json::Value::String(last_update_timestamp.to_string()),
            );
        }

        // Source
        if let Some(source) = &stream.source {
            let mut source_json = serde_json::Map::new();
            if let Some(kinesis_stream_source_description) =
                &source.kinesis_stream_source_description
            {
                source_json.insert(
                    "SourceType".to_string(),
                    serde_json::Value::String("KinesisStream".to_string()),
                );
                if let Some(kinesis_stream_arn) =
                    &kinesis_stream_source_description.kinesis_stream_arn
                {
                    source_json.insert(
                        "KinesisStreamArn".to_string(),
                        serde_json::Value::String(kinesis_stream_arn.clone()),
                    );
                }
                if let Some(role_arn) = &kinesis_stream_source_description.role_arn {
                    source_json.insert(
                        "RoleArn".to_string(),
                        serde_json::Value::String(role_arn.clone()),
                    );
                }
            }
            json.insert("Source".to_string(), serde_json::Value::Object(source_json));
        }

        // Destinations
        let destinations_array: Vec<serde_json::Value> = stream
            .destinations
            .iter()
            .map(|dest| self.destination_to_json(dest))
            .collect();
        json.insert(
            "Destinations".to_string(),
            serde_json::Value::Array(destinations_array),
        );

        // Delivery Stream Encryption Configuration
        if let Some(delivery_stream_encryption_configuration) =
            &stream.delivery_stream_encryption_configuration
        {
            let mut encryption_json = serde_json::Map::new();
            if let Some(key_arn) = &delivery_stream_encryption_configuration.key_arn {
                encryption_json.insert(
                    "KeyArn".to_string(),
                    serde_json::Value::String(key_arn.clone()),
                );
            }
            if let Some(key_type) = &delivery_stream_encryption_configuration.key_type {
                encryption_json.insert(
                    "KeyType".to_string(),
                    serde_json::Value::String(format!("{:?}", key_type)),
                );
            }
            if let Some(status) = &delivery_stream_encryption_configuration.status {
                encryption_json.insert(
                    "Status".to_string(),
                    serde_json::Value::String(format!("{:?}", status)),
                );
            }
            json.insert(
                "DeliveryStreamEncryptionConfiguration".to_string(),
                serde_json::Value::Object(encryption_json),
            );
        }

        // Failure Description
        if let Some(failure_description) = &stream.failure_description {
            let mut failure_json = serde_json::Map::new();
            failure_json.insert(
                "Type".to_string(),
                serde_json::Value::String(format!("{:?}", &failure_description.r#type)),
            );
            failure_json.insert(
                "Details".to_string(),
                serde_json::Value::String(failure_description.details.clone()),
            );
            json.insert(
                "FailureDescription".to_string(),
                serde_json::Value::Object(failure_json),
            );
        }

        serde_json::Value::Object(json)
    }

    fn destination_to_json(
        &self,
        destination: &firehose::types::DestinationDescription,
    ) -> serde_json::Value {
        let mut dest_json = serde_json::Map::new();

        dest_json.insert(
            "DestinationId".to_string(),
            serde_json::Value::String(destination.destination_id.clone()),
        );

        // S3 Destination
        if let Some(s3_destination_description) = &destination.s3_destination_description {
            dest_json.insert(
                "DestinationType".to_string(),
                serde_json::Value::String("S3".to_string()),
            );

            dest_json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(s3_destination_description.role_arn.clone()),
            );

            dest_json.insert(
                "BucketArn".to_string(),
                serde_json::Value::String(s3_destination_description.bucket_arn.clone()),
            );

            if let Some(prefix) = &s3_destination_description.prefix {
                dest_json.insert(
                    "Prefix".to_string(),
                    serde_json::Value::String(prefix.clone()),
                );
            }

            if let Some(error_output_prefix) = &s3_destination_description.error_output_prefix {
                dest_json.insert(
                    "ErrorOutputPrefix".to_string(),
                    serde_json::Value::String(error_output_prefix.clone()),
                );
            }

            if let Some(buffering_hints) = &s3_destination_description.buffering_hints {
                let mut buffering_json = serde_json::Map::new();
                if let Some(size_in_mbs) = buffering_hints.size_in_mbs {
                    buffering_json.insert(
                        "SizeInMBs".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(size_in_mbs)),
                    );
                }
                if let Some(interval_in_seconds) = buffering_hints.interval_in_seconds {
                    buffering_json.insert(
                        "IntervalInSeconds".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(interval_in_seconds)),
                    );
                }
                dest_json.insert(
                    "BufferingHints".to_string(),
                    serde_json::Value::Object(buffering_json),
                );
            }

            dest_json.insert(
                "CompressionFormat".to_string(),
                serde_json::Value::String(format!(
                    "{:?}",
                    s3_destination_description.compression_format
                )),
            );
        }

        // Extended S3 Destination
        if let Some(extended_s3_destination_description) =
            &destination.extended_s3_destination_description
        {
            dest_json.insert(
                "DestinationType".to_string(),
                serde_json::Value::String("ExtendedS3".to_string()),
            );

            dest_json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(extended_s3_destination_description.role_arn.clone()),
            );

            dest_json.insert(
                "BucketArn".to_string(),
                serde_json::Value::String(extended_s3_destination_description.bucket_arn.clone()),
            );

            if let Some(prefix) = &extended_s3_destination_description.prefix {
                dest_json.insert(
                    "Prefix".to_string(),
                    serde_json::Value::String(prefix.clone()),
                );
            }

            if let Some(error_output_prefix) =
                &extended_s3_destination_description.error_output_prefix
            {
                dest_json.insert(
                    "ErrorOutputPrefix".to_string(),
                    serde_json::Value::String(error_output_prefix.clone()),
                );
            }
        }

        // Redshift Destination
        if let Some(redshift_destination_description) =
            &destination.redshift_destination_description
        {
            dest_json.insert(
                "DestinationType".to_string(),
                serde_json::Value::String("Redshift".to_string()),
            );

            dest_json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(redshift_destination_description.role_arn.clone()),
            );

            dest_json.insert(
                "ClusterJDBCURL".to_string(),
                serde_json::Value::String(redshift_destination_description.cluster_jdbcurl.clone()),
            );

            if let Some(copy_command) = &redshift_destination_description.copy_command {
                let mut copy_json = serde_json::Map::new();
                copy_json.insert(
                    "DataTableName".to_string(),
                    serde_json::Value::String(copy_command.data_table_name.clone()),
                );
                if let Some(data_table_columns) = &copy_command.data_table_columns {
                    copy_json.insert(
                        "DataTableColumns".to_string(),
                        serde_json::Value::String(data_table_columns.clone()),
                    );
                }
                if let Some(copy_options) = &copy_command.copy_options {
                    copy_json.insert(
                        "CopyOptions".to_string(),
                        serde_json::Value::String(copy_options.clone()),
                    );
                }
                dest_json.insert(
                    "CopyCommand".to_string(),
                    serde_json::Value::Object(copy_json),
                );
            }

            if let Some(username) = &redshift_destination_description.username {
                dest_json.insert(
                    "Username".to_string(),
                    serde_json::Value::String(username.clone()),
                );
            }
        }

        // OpenSearch/Elasticsearch Destination
        if let Some(amazon_open_search_serverless_destination_description) =
            &destination.amazon_open_search_serverless_destination_description
        {
            dest_json.insert(
                "DestinationType".to_string(),
                serde_json::Value::String("OpenSearchServerless".to_string()),
            );

            if let Some(role_arn) = &amazon_open_search_serverless_destination_description.role_arn
            {
                dest_json.insert(
                    "RoleArn".to_string(),
                    serde_json::Value::String(role_arn.clone()),
                );
            }

            if let Some(collection_endpoint) =
                &amazon_open_search_serverless_destination_description.collection_endpoint
            {
                dest_json.insert(
                    "CollectionEndpoint".to_string(),
                    serde_json::Value::String(collection_endpoint.clone()),
                );
            }

            if let Some(index_name) =
                &amazon_open_search_serverless_destination_description.index_name
            {
                dest_json.insert(
                    "IndexName".to_string(),
                    serde_json::Value::String(index_name.clone()),
                );
            }
        }

        serde_json::Value::Object(dest_json)
    }
}
