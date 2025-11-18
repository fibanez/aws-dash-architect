use super::*;
use super::utils::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Lex Resources
pub struct LexBotNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for LexBotNormalizer {
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
            .or_else(|| raw_response.get("BotId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-bot")
            .to_string();

        let display_name = raw_response
            .get("BotName")
            .or_else(|| raw_response.get("Name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("BotStatus")
            .or_else(|| raw_response.get("Status"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Fetch tags asynchronously from AWS API with caching


        let tags = aws_client


            .fetch_tags_for_resource("AWS::Lex::Bot", &resource_id, account, region)


            .await


            .unwrap_or_else(|e| {


                tracing::warn!("Failed to fetch tags for AWS::Lex::Bot {}: {}", resource_id, e);


                Vec::new()


            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Lex::Bot".to_string(),
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

        // Lex bots can be associated with various AWS resources
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::IAM::Role" => {
                    // Lex bots use IAM service roles
                    if let Some(role_arn) = entry.raw_properties.get("RoleArn") {
                        if let Some(role_arn_str) = role_arn.as_str() {
                            if role_arn_str.contains(&resource.resource_id) {
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
                    // Lex bots can integrate with Lambda functions for fulfillment
                    if resource.account_id == entry.account_id 
                        && resource.region == entry.region {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                "AWS::Logs::LogGroup" => {
                    // Lex bots can log to CloudWatch
                    if resource.account_id == entry.account_id 
                        && resource.region == entry.region 
                        && (resource.resource_id.contains("lex") || resource.resource_id.contains(&entry.resource_id)) {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                "AWS::Cognito::UserPool" => {
                    // Lex bots can integrate with Cognito for user authentication
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
        "AWS::Lex::Bot"
    }
}

