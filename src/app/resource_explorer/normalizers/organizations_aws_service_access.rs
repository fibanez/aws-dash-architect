use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for AWS Organizations Service Access
pub struct OrganizationsAwsServiceAccessNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for OrganizationsAwsServiceAccessNormalizer {
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
            .unwrap_or("unknown-service")
            .to_string();

        // Use service principal as display name
        let display_name = raw_response
            .get("ServicePrincipal")
            .and_then(|v| v.as_str())
            .map(|s| {
                // Convert service principal to friendly name
                // e.g., "cloudtrail.amazonaws.com" -> "CloudTrail"
                if let Some(service_name) = s.split('.').next() {
                    service_name
                        .split('-')
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("")
                } else {
                    s.to_string()
                }
            })
            .unwrap_or_else(|| resource_id.clone());

        let status = Some("ENABLED".to_string());

        // Extract basic properties for normalized view
        let mut properties = serde_json::Map::new();

        properties.insert(
            "status".to_string(),
            serde_json::Value::String("ENABLED".to_string()),
        );

        if let Some(service_principal) = raw_response.get("ServicePrincipal") {
            properties.insert("service_principal".to_string(), service_principal.clone());
        }

        if let Some(date_enabled) = raw_response.get("DateEnabled") {
            properties.insert("date_enabled".to_string(), date_enabled.clone());
        }

        let account_color = assign_account_color(account);
        let region_color = assign_region_color(region);

        let mut entry = ResourceEntry {
            resource_type: "AWS::Organizations::AwsServiceAccess".to_string(),
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
        "AWS::Organizations::AwsServiceAccess"
    }
}
