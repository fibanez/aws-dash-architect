use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Polly Voice Resources
pub struct PollyVoiceNormalizer;

impl ResourceNormalizer for PollyVoiceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
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
        let tags = extract_tags(&raw_response);
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

impl ResourceNormalizer for PollyLexiconNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-lexicon")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
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
            match resource.resource_type.as_str() {
                "AWS::Polly::SynthesisTask" => {
                    // Synthesis tasks can use lexicons
                    if resource.account_id == entry.account_id 
                        && resource.region == entry.region {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                _ => {}
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

impl ResourceNormalizer for PollySynthesisTaskNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("TaskId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-task")
            .to_string();

        let display_name = resource_id.clone();

        let status = raw_response
            .get("TaskStatus")
            .or_else(|| raw_response.get("Status"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Polly::SynthesisTask".to_string(),
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

        // Synthesis tasks can be associated with various AWS resources
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::SNS::Topic" => {
                    // Synthesis tasks can notify via SNS
                    if let Some(sns_topic_arn) = entry.raw_properties.get("SnsTopicArn") {
                        if let Some(sns_topic_arn_str) = sns_topic_arn.as_str() {
                            if sns_topic_arn_str.contains(&resource.resource_id) {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::S3::Bucket" => {
                    // Synthesis tasks output to S3
                    if let Some(output_uri) = entry.raw_properties.get("OutputUri") {
                        if let Some(output_uri_str) = output_uri.as_str() {
                            if output_uri_str.contains(&resource.resource_id) {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::Polly::Voice" => {
                    // Synthesis tasks use voices
                    if let Some(voice_id) = entry.raw_properties.get("VoiceId") {
                        if let Some(voice_id_str) = voice_id.as_str() {
                            if voice_id_str == resource.resource_id {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::Polly::Lexicon" => {
                    // Synthesis tasks can use lexicons
                    if resource.account_id == entry.account_id 
                        && resource.region == entry.region {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Polly::SynthesisTask"
    }
}