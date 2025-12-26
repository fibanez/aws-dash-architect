use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for AWS Organizations Handshakes
pub struct OrganizationsHandshakeNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for OrganizationsHandshakeNormalizer {
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
            .get("Id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-handshake")
            .to_string();

        // Use handshake ID as display name
        let display_name = resource_id.clone();

        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                raw_response
                    .get("Status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| Some("REQUESTED".to_string()));

        // Extract basic properties for normalized view
        let mut properties = serde_json::Map::new();

        if let Some(status) = &status {
            properties.insert(
                "status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(arn) = raw_response.get("Arn") {
            properties.insert("arn".to_string(), arn.clone());
        }

        if let Some(state) = raw_response.get("State") {
            properties.insert("state".to_string(), state.clone());
        }

        if let Some(action) = raw_response.get("Action") {
            properties.insert("action".to_string(), action.clone());
        }

        if let Some(requested_timestamp) = raw_response.get("RequestedTimestamp") {
            properties.insert(
                "requested_timestamp".to_string(),
                requested_timestamp.clone(),
            );
        }

        if let Some(expiration_timestamp) = raw_response.get("ExpirationTimestamp") {
            properties.insert(
                "expiration_timestamp".to_string(),
                expiration_timestamp.clone(),
            );
        }

        let account_color = assign_account_color(account);
        let region_color = assign_region_color(region);

        let mut entry = ResourceEntry {
            resource_type: "AWS::Organizations::Handshake".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: serde_json::Value::Object(properties),
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags: Vec::new(),
            relationships: Vec::new(), // Will be populated by extract_relationships
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false, // Independent resource
            account_color,
            region_color,
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
        "AWS::Organizations::Handshake"
    }
}
