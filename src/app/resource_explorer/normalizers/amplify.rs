use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Amplify Resources
pub struct AmplifyNormalizer;

impl ResourceNormalizer for AmplifyNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("AppId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-app")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        
        // Amplify apps don't have a traditional status field, use platform or "active"
        let status = raw_response
            .get("Platform")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Amplify::App".to_string(),
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

        // Amplify apps can be associated with various AWS resources
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::IAM::Role" => {
                    // Amplify apps can use IAM service roles
                    if let Some(service_role) = entry.raw_properties.get("IamServiceRoleArn") {
                        if let Some(service_role_str) = service_role.as_str() {
                            if service_role_str.contains(&resource.resource_id) {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::Lambda::Function" => {
                    // Amplify can integrate with Lambda functions for serverless functions
                    if resource.account_id == entry.account_id 
                        && resource.region == entry.region {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                "AWS::S3::Bucket" => {
                    // Amplify apps store build artifacts and host content in S3
                    if resource.account_id == entry.account_id 
                        && (resource.resource_id.contains("amplify") 
                            || resource.resource_id.contains(&entry.resource_id)) {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                "AWS::CloudFront::Distribution" => {
                    // Amplify uses CloudFront for CDN
                    if resource.account_id == entry.account_id {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                "AWS::Cognito::UserPool" => {
                    // Amplify apps often integrate with Cognito for authentication
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
        "AWS::Amplify::App"
    }
}