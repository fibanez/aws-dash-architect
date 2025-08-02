use super::super::state::{RelationshipType, ResourceEntry, ResourceRelationship};
use super::{utils, ResourceNormalizer};
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct KinesisFirehoseDeliveryStreamNormalizer;

impl ResourceNormalizer for KinesisFirehoseDeliveryStreamNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("DeliveryStreamName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing DeliveryStreamName"))?
            .to_string();

        let display_name = raw_response
            .get("DeliveryStreamName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = utils::extract_status(&raw_response);
        let tags = utils::extract_tags(&raw_response);
        let normalized_properties = utils::create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::KinesisFirehose::DeliveryStream".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: normalized_properties,
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            account_color: egui::Color32::PLACEHOLDER,
            region_color: egui::Color32::PLACEHOLDER,
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Extract source relationships
        if let Some(source) = entry.raw_properties.get("Source") {
            if let Some(kinesis_stream_arn) =
                source.get("KinesisStreamArn").and_then(|v| v.as_str())
            {
                relationships.push(ResourceRelationship {
                    relationship_type: RelationshipType::Uses,
                    target_resource_id: kinesis_stream_arn.to_string(),
                    target_resource_type: "AWS::Kinesis::Stream".to_string(),
                });
            }

            if let Some(role_arn) = source.get("RoleArn").and_then(|v| v.as_str()) {
                relationships.push(ResourceRelationship {
                    relationship_type: RelationshipType::Uses,
                    target_resource_id: role_arn.to_string(),
                    target_resource_type: "AWS::IAM::Role".to_string(),
                });
            }
        }

        // Extract destination relationships
        if let Some(destinations) = entry
            .raw_properties
            .get("Destinations")
            .and_then(|v| v.as_array())
        {
            for destination in destinations {
                // Role ARN for all destination types
                if let Some(role_arn) = destination.get("RoleArn").and_then(|v| v.as_str()) {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: role_arn.to_string(),
                        target_resource_type: "AWS::IAM::Role".to_string(),
                    });
                }

                // S3 bucket relationships
                if let Some(bucket_arn) = destination.get("BucketArn").and_then(|v| v.as_str()) {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: bucket_arn.to_string(),
                        target_resource_type: "AWS::S3::Bucket".to_string(),
                    });
                }

                // Redshift cluster relationships
                if let Some(cluster_jdbc_url) =
                    destination.get("ClusterJDBCURL").and_then(|v| v.as_str())
                {
                    // Extract cluster identifier from JDBC URL if possible
                    if let Some(cluster_id) =
                        Self::extract_redshift_cluster_from_jdbc(cluster_jdbc_url)
                    {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: cluster_id,
                            target_resource_type: "AWS::Redshift::Cluster".to_string(),
                        });
                    }
                }

                // OpenSearch collection relationships
                if let Some(collection_endpoint) = destination
                    .get("CollectionEndpoint")
                    .and_then(|v| v.as_str())
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: collection_endpoint.to_string(),
                        target_resource_type: "AWS::OpenSearchServerless::Collection".to_string(),
                    });
                }
            }
        }

        // Extract encryption key relationships
        if let Some(encryption_config) = entry
            .raw_properties
            .get("DeliveryStreamEncryptionConfiguration")
        {
            if let Some(key_arn) = encryption_config.get("KeyArn").and_then(|v| v.as_str()) {
                relationships.push(ResourceRelationship {
                    relationship_type: RelationshipType::Uses,
                    target_resource_id: key_arn.to_string(),
                    target_resource_type: "AWS::KMS::Key".to_string(),
                });
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::KinesisFirehose::DeliveryStream"
    }
}

impl KinesisFirehoseDeliveryStreamNormalizer {
    fn extract_redshift_cluster_from_jdbc(jdbc_url: &str) -> Option<String> {
        // Extract cluster identifier from JDBC URL format:
        // jdbc:redshift://cluster-identifier.region.redshift.amazonaws.com:5439/database
        if let Some(start) = jdbc_url.find("://") {
            if let Some(end) = jdbc_url[start + 3..].find('.') {
                let cluster_id = &jdbc_url[start + 3..start + 3 + end];
                return Some(cluster_id.to_string());
            }
        }
        None
    }
}
