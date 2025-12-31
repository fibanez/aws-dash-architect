use super::*;
use crate::app::resource_explorer::{assign_account_color, assign_region_color};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for QuickSight Data Source resources
pub struct QuickSightDataSourceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for QuickSightDataSourceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let data_source_id = raw_response
            .get("DataSourceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&data_source_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut tags = Vec::new();

        // Add data source type as a tag
        if let Some(data_source_type) = raw_response.get("Type").and_then(|v| v.as_str()) {
            tags.push(ResourceTag {
                key: "DataSourceType".to_string(),
                value: data_source_type.to_string(),
            });
        }

        let mut entry = ResourceEntry {
            resource_id: data_source_id.clone(),
            resource_type: "AWS::QuickSight::DataSource".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            display_name: name,
            status,
            properties: serde_json::Value::Object(serde_json::Map::new()),
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            query_timestamp,
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
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
        "AWS::QuickSight::DataSource"
    }
}

/// Normalizer for QuickSight Dashboard resources
pub struct QuickSightDashboardNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for QuickSightDashboardNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let data_source_id = raw_response
            .get("DataSourceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&data_source_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut tags = Vec::new();

        // Add data source type as a tag
        if let Some(data_source_type) = raw_response.get("Type").and_then(|v| v.as_str()) {
            tags.push(ResourceTag {
                key: "DataSourceType".to_string(),
                value: data_source_type.to_string(),
            });
        }

        let mut entry = ResourceEntry {
            resource_id: data_source_id.clone(),
            resource_type: "AWS::QuickSight::DataSource".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            display_name: name,
            status,
            properties: serde_json::Value::Object(serde_json::Map::new()),
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            query_timestamp,
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
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
        "AWS::QuickSight::Dashboard"
    }
}

/// Normalizer for QuickSight Data Set resources
pub struct QuickSightDataSetNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for QuickSightDataSetNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let data_source_id = raw_response
            .get("DataSourceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&data_source_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut tags = Vec::new();

        // Add data source type as a tag
        if let Some(data_source_type) = raw_response.get("Type").and_then(|v| v.as_str()) {
            tags.push(ResourceTag {
                key: "DataSourceType".to_string(),
                value: data_source_type.to_string(),
            });
        }

        let mut entry = ResourceEntry {
            resource_id: data_source_id.clone(),
            resource_type: "AWS::QuickSight::DataSource".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            display_name: name,
            status,
            properties: serde_json::Value::Object(serde_json::Map::new()),
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            query_timestamp,
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
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
        "AWS::QuickSight::DataSet"
    }
}
