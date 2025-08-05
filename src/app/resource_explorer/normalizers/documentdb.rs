use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for DocumentDB Resources
pub struct DocumentDbResourceNormalizer;

impl ResourceNormalizer for DocumentDbResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("Id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-documentdb-cluster")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::DocumentDB::Cluster".to_string(),
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
        
        // DocumentDB relates to VPCs, subnets, and security groups
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::EC2::VPC" => {
                    // DocumentDB clusters are deployed in VPCs
                    if let Some(vpc_security_groups) = entry.raw_properties.get("VpcSecurityGroups") {
                        if vpc_security_groups.is_array() {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::EC2::SecurityGroup" => {
                    // DocumentDB uses security groups for network access control
                    if let Some(vpc_security_groups) = entry.raw_properties.get("VpcSecurityGroups") {
                        if let Some(groups) = vpc_security_groups.as_array() {
                            for group in groups {
                                if let Some(group_id) = group.get("VpcSecurityGroupId").and_then(|v| v.as_str()) {
                                    if group_id == resource.resource_id {
                                        relationships.push(ResourceRelationship {
                                            relationship_type: RelationshipType::Uses,
                                            target_resource_id: resource.resource_id.clone(),
                                            target_resource_type: resource.resource_type.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                "AWS::Lambda::Function" => {
                    // Lambda functions often connect to DocumentDB
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::IAM::Role" => {
                    // DocumentDB uses IAM roles for authentication and permissions
                    if let Some(associated_roles) = entry.raw_properties.get("AssociatedRoles") {
                        if let Some(roles) = associated_roles.as_array() {
                            for role in roles {
                                if let Some(role_arn) = role.get("RoleArn").and_then(|v| v.as_str()) {
                                    if role_arn.contains(&resource.resource_id) {
                                        relationships.push(ResourceRelationship {
                                            relationship_type: RelationshipType::Uses,
                                            target_resource_id: resource.resource_id.clone(),
                                            target_resource_type: resource.resource_type.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::DocumentDB::Cluster"
    }
}