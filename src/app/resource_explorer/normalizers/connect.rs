use super::*;
use super::utils::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Connect Resources
pub struct ConnectNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ConnectNormalizer {
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
            .or_else(|| raw_response.get("Id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-instance")
            .to_string();

        let display_name = raw_response
            .get("InstanceAlias")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("InstanceStatus")
            .or_else(|| raw_response.get("Status"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Fetch tags asynchronously from AWS API with caching


        let tags = aws_client


            .fetch_tags_for_resource("AWS::Connect::Instance", &resource_id, account, region)


            .await


            .unwrap_or_else(|e| {


                tracing::warn!("Failed to fetch tags for AWS::Connect::Instance {}: {}", resource_id, e);


                Vec::new()


            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Connect::Instance".to_string(),
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

        // Amazon Connect instances can be associated with various AWS resources
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::IAM::Role" => {
                    // Connect instances use IAM service roles
                    if let Some(service_role) = entry.raw_properties.get("ServiceRole") {
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
                    // Connect can integrate with Lambda functions for call flows
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
                    // Connect instances can log to CloudWatch
                    if resource.account_id == entry.account_id 
                        && resource.region == entry.region 
                        && resource.resource_id.contains("connect") {
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
        "AWS::Connect::Instance"
    }
}

