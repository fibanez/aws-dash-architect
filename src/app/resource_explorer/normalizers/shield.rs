use super::*;
use super::utils::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Shield Protections
pub struct ShieldProtectionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ShieldProtectionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ProtectionId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-protection")
            .to_string();

        let display_name = raw_response
            .get("ProtectionName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::Shield::Protection", &resource_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::Shield::Protection {}: {}", resource_id, e);

                Vec::new()

            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Shield::Protection".to_string(),
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

        // Shield protections protect specific AWS resources
        if let Some(resource_arn) = entry.raw_properties.get("ResourceArn").and_then(|v| v.as_str()) {
            // Find the protected resource
            for resource in all_resources {
                // Check if this resource matches the ARN protected by Shield
                if let Some(arn) = resource.raw_properties.get("Arn").and_then(|v| v.as_str()) {
                    if arn == resource_arn {
                        relationships.push(ResourceRelationship {
                            relationship_type: crate::app::resource_explorer::state::RelationshipType::ProtectedBy,
                            target_resource_type: resource.resource_type.clone(),
                            target_resource_id: resource.resource_id.clone(),
                        });
                        break;
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Shield::Protection"
    }
}

/// Normalizer for Shield Subscription (Advanced)
pub struct ShieldSubscriptionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ShieldSubscriptionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("SubscriptionId")
            .and_then(|v| v.as_str())
            .unwrap_or("shield-advanced-subscription")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::Shield::Subscription", &resource_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::Shield::Subscription {}: {}", resource_id, e);

                Vec::new()

            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Shield::Subscription".to_string(),
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Shield subscription doesn't have direct relationships with other resources
        // It's a service-level configuration
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Shield::Subscription"
    }
}

