use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for AWS Organizations CreateAccountStatus
pub struct OrganizationsCreateAccountStatusNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for OrganizationsCreateAccountStatusNormalizer {
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
            .unwrap_or("unknown-request")
            .to_string();

        // Use account name as display name, fall back to ID
        let display_name = raw_response
            .get("AccountName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| resource_id.clone());

        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                raw_response
                    .get("Status")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        // Extract basic properties for normalized view
        let mut properties = serde_json::Map::new();

        if let Some(status) = &status {
            properties.insert(
                "status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(state) = raw_response.get("State") {
            properties.insert("state".to_string(), state.clone());
        }

        if let Some(account_name) = raw_response.get("AccountName") {
            properties.insert("account_name".to_string(), account_name.clone());
        }

        if let Some(account_id) = raw_response.get("AccountId") {
            properties.insert("account_id".to_string(), account_id.clone());
        }

        if let Some(requested_timestamp) = raw_response.get("RequestedTimestamp") {
            properties.insert(
                "requested_timestamp".to_string(),
                requested_timestamp.clone(),
            );
        }

        if let Some(completed_timestamp) = raw_response.get("CompletedTimestamp") {
            properties.insert(
                "completed_timestamp".to_string(),
                completed_timestamp.clone(),
            );
        }

        if let Some(gov_cloud_account_id) = raw_response.get("GovCloudAccountId") {
            properties.insert(
                "gov_cloud_account_id".to_string(),
                gov_cloud_account_id.clone(),
            );
        }

        if let Some(failure_reason) = raw_response.get("FailureReason") {
            properties.insert("failure_reason".to_string(), failure_reason.clone());
        }

        let account_color = assign_account_color(account);
        let region_color = assign_region_color(region);

        let mut entry = ResourceEntry {
            resource_type: "AWS::Organizations::CreateAccountStatus".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: serde_json::Value::Object(properties),
            detailed_timestamp: None,
            tags: Vec::new(),
            relationships: Vec::new(), // Will be populated by extract_relationships
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false, // Independent resource tracking account creation
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
        "AWS::Organizations::CreateAccountStatus"
    }
}
