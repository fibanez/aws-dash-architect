use super::super::state::{ResourceEntry, ResourceRelationship};
use super::{utils, AsyncResourceNormalizer};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub struct CognitoNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CognitoNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        // Determine resource type and extract fields
        let (resource_id, resource_type, display_name) =
            if let Some(user_pool_id) = raw_response.get("Id").and_then(|v| v.as_str()) {
                // This is a User Pool
                let name = raw_response
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(user_pool_id);
                (
                    user_pool_id.to_string(),
                    "AWS::Cognito::UserPool",
                    name.to_string(),
                )
            } else if let Some(identity_pool_id) =
                raw_response.get("IdentityPoolId").and_then(|v| v.as_str())
            {
                // This is an Identity Pool
                let name = raw_response
                    .get("IdentityPoolName")
                    .and_then(|v| v.as_str())
                    .unwrap_or(identity_pool_id);
                (
                    identity_pool_id.to_string(),
                    "AWS::Cognito::IdentityPool",
                    name.to_string(),
                )
            } else if let Some(client_id) = raw_response.get("ClientId").and_then(|v| v.as_str()) {
                // This is a User Pool Client
                let name = raw_response
                    .get("ClientName")
                    .and_then(|v| v.as_str())
                    .unwrap_or(client_id);
                (
                    client_id.to_string(),
                    "AWS::Cognito::UserPoolClient",
                    name.to_string(),
                )
            } else {
                return Err(anyhow::anyhow!("Unable to determine Cognito resource type"));
            };

        // Extract status
        let status = utils::extract_status(&raw_response);

        // Extract tags (Cognito resources don't typically have tags, but we'll check)
        let tags = utils::extract_tags(&raw_response);

        // Create normalized properties
        let normalized_properties = utils::create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
            resource_type: resource_type.to_string(),
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
            relationships: Vec::new(), // Will be filled by extract_relationships
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: egui::Color32::PLACEHOLDER,
            region_color: egui::Color32::PLACEHOLDER,
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
        "AWS::Cognito::*"
    }
}
