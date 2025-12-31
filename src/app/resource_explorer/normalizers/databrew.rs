#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::AsyncResourceNormalizer;
use crate::app::resource_explorer::state::*;
use crate::app::resource_explorer::{assign_account_color, assign_region_color};

pub struct DataBrewJobNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for DataBrewJobNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let binding = raw_response.clone();
        let job_obj = binding
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Job is not an object"))?;

        let name = job_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let _arn = format!("arn:aws:databrew:{}:{}:job/{}", region, account, name);

        let mut entry = ResourceEntry {
            resource_type: "AWS::DataBrew::Job".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: name.clone(),
            display_name: name,
            status: job_obj
                .get("state")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            properties: raw_response.clone(),
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags: Vec::new(), // Will be filled below
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
        "AWS::DataBrew::Job"
    }
}

pub struct DataBrewDatasetNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for DataBrewDatasetNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let binding = raw_response.clone();
        let job_obj = binding
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Job is not an object"))?;

        let name = job_obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let _arn = format!("arn:aws:databrew:{}:{}:job/{}", region, account, name);

        let mut entry = ResourceEntry {
            resource_type: "AWS::DataBrew::Job".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: name.clone(),
            display_name: name,
            status: job_obj
                .get("state")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            properties: raw_response.clone(),
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags: Vec::new(), // Will be filled below
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
        "AWS::DataBrew::Dataset"
    }
}
