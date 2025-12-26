use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Amazon FSx File System Resources
pub struct FsxResourceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for FsxResourceNormalizer {
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
            .or_else(|| raw_response.get("FileSystemId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-fsx-filesystem")
            .to_string();

        let display_name = raw_response
            .get("DNSName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Lifecycle")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
            resource_type: "AWS::FSx::FileSystem".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status: Some(status),
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
        "AWS::FSx::FileSystem"
    }
}

/// Normalizer for Amazon FSx Backup Resources
pub struct FsxBackupResourceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for FsxBackupResourceNormalizer {
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
            .or_else(|| raw_response.get("BackupId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-fsx-backup")
            .to_string();

        let display_name = resource_id.clone();

        let status = raw_response
            .get("Lifecycle")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::FSx::Backup", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::FSx::Backup {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::FSx::Backup".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status: Some(status),
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

        // FSx backups relate to their source file systems
        if let Some(file_system_id) = entry
            .raw_properties
            .get("FileSystemId")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::FSx::FileSystem"
                    && resource.resource_id == file_system_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                    break;
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::FSx::Backup"
    }
}
