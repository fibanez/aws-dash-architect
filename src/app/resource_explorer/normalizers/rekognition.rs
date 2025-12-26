use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Rekognition Collection Resources
pub struct RekognitionCollectionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for RekognitionCollectionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("CollectionId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-collection")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::Rekognition::Collection",
                &resource_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Rekognition::Collection {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Rekognition::Collection".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Rekognition collections can be associated with various AWS resources
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::S3::Bucket" => {
                    // Rekognition can analyze images stored in S3
                    if resource.account_id == entry.account_id {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                "AWS::Lambda::Function" => {
                    // Lambda functions often trigger Rekognition analysis
                    if resource.account_id == entry.account_id && resource.region == entry.region {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Rekognition::Collection"
    }
}

/// Normalizer for Rekognition Stream Processor Resources
pub struct RekognitionStreamProcessorNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for RekognitionStreamProcessorNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-processor")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::Rekognition::StreamProcessor",
                &resource_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Rekognition::StreamProcessor {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Rekognition::StreamProcessor".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Stream processors can be associated with various AWS resources
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::IAM::Role" => {
                    // Stream processors use IAM roles for permissions
                    if let Some(role_arn) = entry.raw_properties.get("RoleArn") {
                        if let Some(role_arn_str) = role_arn.as_str() {
                            if role_arn_str.contains(&resource.resource_id) {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::Kinesis::Stream" => {
                    // Stream processors can output to Kinesis streams
                    if resource.account_id == entry.account_id && resource.region == entry.region {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                "AWS::S3::Bucket" => {
                    // Stream processors can output to S3
                    if let Some(output) = entry.raw_properties.get("Output") {
                        if let Some(s3_bucket) = output.get("S3Bucket") {
                            if let Some(s3_bucket_str) = s3_bucket.as_str() {
                                if s3_bucket_str == resource.resource_id {
                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::Uses,
                                        target_resource_id: resource.resource_id.clone(),
                                        target_resource_type: resource.resource_type.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Rekognition::StreamProcessor"
    }
}
