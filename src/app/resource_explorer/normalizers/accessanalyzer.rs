use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for AWS Access Analyzer
pub struct AccessAnalyzerNormalizer;

impl ResourceNormalizer for AccessAnalyzerNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("AnalyzerName"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-access-analyzer")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::AccessAnalyzer::Analyzer".to_string(),
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
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Access Analyzer analyzes IAM policies and resources for external access
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::IAM::Role" | "AWS::IAM::User" | "AWS::IAM::Policy" => {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::S3::Bucket" => {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::KMS::Key" => {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::SQS::Queue" => {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                _ => {}
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::AccessAnalyzer::Analyzer"
    }
}