use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for EventBridge Event Bus
pub struct EventBridgeEventBusNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EventBridgeEventBusNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let event_bus_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-event-bus")
            .to_string();

        let display_name = extract_display_name(&raw_response, &event_bus_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Events::EventBus", &event_bus_name, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Events::EventBus {}: {}",
                    event_bus_name,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::Events::EventBus".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: event_bus_name,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // EventBridge event buses can have relationships with rules
        // but we'd need to analyze rule configurations for specific targeting
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Events::EventBus"
    }
}

/// Normalizer for EventBridge Rules
pub struct EventBridgeRuleNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EventBridgeRuleNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let rule_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-rule")
            .to_string();

        let display_name = extract_display_name(&raw_response, &rule_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Events::Rule", &rule_name, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Events::Rule {}: {}",
                    rule_name,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::Events::Rule".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: rule_name,
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

        // Map to event bus if specified
        if let Some(event_bus_name) = entry
            .properties
            .get("EventBusName")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::Events::EventBus"
                    && resource.resource_id == event_bus_name
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: event_bus_name.to_string(),
                        target_resource_type: "AWS::Events::EventBus".to_string(),
                    });
                }
            }
        }

        // Could potentially analyze targets from rule configuration
        // but that would require additional API calls or parsing rule target configurations

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Events::Rule"
    }
}
