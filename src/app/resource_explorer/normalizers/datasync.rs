use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for AWS DataSync Task Resources
pub struct DataSyncResourceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for DataSyncResourceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("TaskArn"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-datasync-task")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

        let mut entry = ResourceEntry {
            resource_type: "AWS::DataSync::Task".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: raw_response,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        };

        // Fetch tags (will be empty for resources that don't support tagging)
        entry.tags = aws_client
            .fetch_tags_for_resource(&entry.resource_type, &entry.resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for {} {}: {:?}",
                    entry.resource_type,
                    entry.resource_id,
                    e
                );
                Vec::new()
            });

        Ok(entry)
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::DataSync::Task"
    }
}

/// Normalizer for AWS DataSync Location Resources
pub struct DataSyncLocationResourceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for DataSyncLocationResourceNormalizer {
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
            .or_else(|| raw_response.get("LocationArn"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-datasync-location")
            .to_string();

        let display_name = raw_response
            .get("LocationUri")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::DataSync::Location", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::DataSync::Location {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::DataSync::Location".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: raw_response,
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

        // DataSync locations relate to the actual storage services they represent
        if let Some(location_uri) = entry
            .properties
            .get("LocationUri")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                match resource.resource_type.as_str() {
                    "AWS::S3::Bucket" => {
                        if location_uri.contains("s3://")
                            && location_uri.contains(&resource.resource_id)
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                    "AWS::EFS::FileSystem" => {
                        if location_uri.contains("efs")
                            && location_uri.contains(&resource.resource_id)
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                    "AWS::FSx::FileSystem" => {
                        if location_uri.contains("fsx")
                            && location_uri.contains(&resource.resource_id)
                        {
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
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::DataSync::Location"
    }
}
