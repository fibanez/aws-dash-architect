use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for OpenSearch Domains
pub struct OpenSearchDomainNormalizer;

impl ResourceNormalizer for OpenSearchDomainNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("DomainName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-domain")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        // OpenSearch domains don't have a simple status field like other services
        let status = if raw_response
            .get("Processing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("Processing".to_string())
        } else if raw_response
            .get("UpgradeProcessing")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("UpgradeProcessing".to_string())
        } else if raw_response
            .get("Deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("Deleted".to_string())
        } else if raw_response
            .get("Created")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("Available".to_string())
        } else {
            Some("Unknown".to_string())
        };

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::OpenSearchService::Domain".to_string(),
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

        // OpenSearch domains can be deployed in VPC with security groups
        if let Some(vpc_options) = entry
            .raw_properties
            .get("VPCOptions")
            .and_then(|v| v.as_object())
        {
            // Relationship with VPC
            if let Some(vpc_id) = vpc_options.get("VPCId").and_then(|v| v.as_str()) {
                for resource in all_resources {
                    if resource.resource_type == "AWS::EC2::VPC" && resource.resource_id == vpc_id {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::DeployedIn,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
            }

            // Relationships with security groups
            if let Some(security_group_ids) = vpc_options
                .get("SecurityGroupIds")
                .and_then(|v| v.as_array())
            {
                for sg_id_value in security_group_ids {
                    if let Some(sg_id) = sg_id_value.as_str() {
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::SecurityGroup"
                                && resource.resource_id == sg_id
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::ProtectedBy,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
            }

            // Relationships with subnets
            if let Some(subnet_ids) = vpc_options.get("SubnetIds").and_then(|v| v.as_array()) {
                for subnet_id_value in subnet_ids {
                    if let Some(subnet_id) = subnet_id_value.as_str() {
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::Subnet"
                                && resource.resource_id == subnet_id
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::DeployedIn,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // OpenSearch domains can use KMS keys for encryption
        if let Some(encryption_options) = entry
            .raw_properties
            .get("EncryptionAtRestOptions")
            .and_then(|v| v.as_object())
        {
            if let Some(kms_key_id) = encryption_options.get("KmsKeyId").and_then(|v| v.as_str()) {
                for resource in all_resources {
                    if resource.resource_type == "AWS::KMS::Key"
                        && resource.resource_id == kms_key_id
                    {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
            }
        }

        // OpenSearch domains can use Cognito for authentication
        if let Some(cognito_options) = entry
            .raw_properties
            .get("CognitoOptions")
            .and_then(|v| v.as_object())
        {
            if let Some(user_pool_id) = cognito_options.get("UserPoolId").and_then(|v| v.as_str()) {
                for resource in all_resources {
                    if resource.resource_type == "AWS::Cognito::UserPool"
                        && resource.resource_id == user_pool_id
                    {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
            }

            if let Some(identity_pool_id) = cognito_options
                .get("IdentityPoolId")
                .and_then(|v| v.as_str())
            {
                for resource in all_resources {
                    if resource.resource_type == "AWS::Cognito::IdentityPool"
                        && resource.resource_id == identity_pool_id
                    {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::OpenSearchService::Domain"
    }
}
