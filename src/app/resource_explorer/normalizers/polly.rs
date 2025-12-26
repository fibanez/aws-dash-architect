use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Polly Voice Resources
pub struct PollyVoiceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for PollyVoiceNormalizer {
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
            .or_else(|| raw_response.get("VoiceId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-voice")
            .to_string();

        let display_name = raw_response
            .get("VoiceName")
            .or_else(|| raw_response.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Polly::Voice", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Polly::Voice {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Polly::Voice".to_string(),
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
        // Voices are AWS-managed resources with minimal relationships
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Polly::Voice"
    }
}

/// Normalizer for Polly Lexicon Resources
pub struct PollyLexiconNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for PollyLexiconNormalizer {
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
            .or_else(|| raw_response.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-lexicon")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Polly::Lexicon", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Polly::Lexicon {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Polly::Lexicon".to_string(),
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

        // Lexicons can be used by synthesis tasks and applications
        for resource in all_resources {
            if resource.resource_type.as_str() == "AWS::Polly::SynthesisTask" {
                // Synthesis tasks can use lexicons
                if resource.account_id == entry.account_id && resource.region == entry.region {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Polly::Lexicon"
    }
}

/// Normalizer for Polly Synthesis Task Resources
pub struct PollySynthesisTaskNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for PollySynthesisTaskNormalizer {
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
            .or_else(|| raw_response.get("VoiceId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-voice")
            .to_string();

        let display_name = raw_response
            .get("VoiceName")
            .or_else(|| raw_response.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);

        let mut entry = ResourceEntry {
            resource_type: "AWS::Polly::Voice".to_string(),
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
        "AWS::Polly::SynthesisTask"
    }
}
