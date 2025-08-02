use super::super::state::{RelationshipType, ResourceEntry, ResourceRelationship};
use super::{utils, ResourceNormalizer};
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct CognitoNormalizer;

impl ResourceNormalizer for CognitoNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
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

        Ok(ResourceEntry {
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
            account_color: egui::Color32::PLACEHOLDER,
            region_color: egui::Color32::PLACEHOLDER,
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Extract relationships based on resource type
        if entry.resource_type == "AWS::Cognito::UserPool" {
            // User Pool relationships
            if let Some(email_config) = entry.raw_properties.get("EmailConfiguration") {
                if let Some(source_arn) = email_config.get("SourceArn").and_then(|v| v.as_str()) {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: source_arn.to_string(),
                        target_resource_type: "AWS::SES::ConfigurationSet".to_string(),
                    });
                }
            }

            if let Some(sms_config) = entry.raw_properties.get("SmsConfiguration") {
                if let Some(sns_caller_arn) =
                    sms_config.get("SnsCallerArn").and_then(|v| v.as_str())
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: sns_caller_arn.to_string(),
                        target_resource_type: "AWS::SNS::Topic".to_string(),
                    });
                }
            }
        } else if entry.resource_type == "AWS::Cognito::IdentityPool" {
            // Identity Pool relationships
            if let Some(cognito_providers) = entry.raw_properties.get("CognitoIdentityProviders") {
                if let Some(providers_array) = cognito_providers.as_array() {
                    for provider in providers_array {
                        if let Some(provider_name) =
                            provider.get("ProviderName").and_then(|v| v.as_str())
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: provider_name.to_string(),
                                target_resource_type: "AWS::Cognito::UserPool".to_string(),
                            });
                        }
                    }
                }
            }
        } else if entry.resource_type == "AWS::Cognito::UserPoolClient" {
            // User Pool Client relationships
            if let Some(user_pool_id) = entry
                .raw_properties
                .get("UserPoolId")
                .and_then(|v| v.as_str())
            {
                relationships.push(ResourceRelationship {
                    relationship_type: RelationshipType::AttachedTo,
                    target_resource_id: user_pool_id.to_string(),
                    target_resource_type: "AWS::Cognito::UserPool".to_string(),
                });
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Cognito::*" // Handles multiple Cognito resource types
    }
}
