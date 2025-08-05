use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for AWS Transfer Family Resources
pub struct TransferResourceNormalizer;

impl ResourceNormalizer for TransferResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("ServerId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-transfer-server")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Transfer::Server".to_string(),
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
        
        // Transfer Family servers relate to VPCs, subnets, security groups, and S3/EFS storage
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::EC2::VPC" => {
                    // Transfer servers can be deployed in VPCs
                    if let Some(endpoint_details) = entry.raw_properties.get("EndpointDetails") {
                        if let Some(vpc_id) = endpoint_details.get("VpcId").and_then(|v| v.as_str()) {
                            if vpc_id == resource.resource_id {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::EC2::Subnet" => {
                    // Transfer servers use subnets for VPC endpoints
                    if let Some(endpoint_details) = entry.raw_properties.get("EndpointDetails") {
                        if let Some(subnet_ids) = endpoint_details.get("SubnetIds") {
                            if let Some(subnets) = subnet_ids.as_array() {
                                for subnet in subnets {
                                    if let Some(subnet_id) = subnet.as_str() {
                                        if subnet_id == resource.resource_id {
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
                }
                "AWS::EC2::SecurityGroup" => {
                    // Transfer servers use security groups for network access control
                    if let Some(endpoint_details) = entry.raw_properties.get("EndpointDetails") {
                        if let Some(security_group_ids) = endpoint_details.get("SecurityGroupIds") {
                            if let Some(groups) = security_group_ids.as_array() {
                                for group in groups {
                                    if let Some(group_id) = group.as_str() {
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
                }
                "AWS::S3::Bucket" => {
                    // Transfer servers often use S3 buckets as storage backends
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::EFS::FileSystem" => {
                    // Transfer servers can use EFS for file storage
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::IAM::Role" => {
                    // Transfer servers use IAM roles for authentication and access
                    if let Some(logging_role) = entry.raw_properties.get("LoggingRole").and_then(|v| v.as_str()) {
                        if logging_role.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                    
                    if let Some(identity_provider_details) = entry.raw_properties.get("IdentityProviderDetails") {
                        if let Some(invocation_role) = identity_provider_details.get("InvocationRole").and_then(|v| v.as_str()) {
                            if invocation_role.contains(&resource.resource_id) {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::CertificateManager::Certificate" => {
                    // Transfer servers can use ACM certificates for TLS
                    if let Some(certificate) = entry.raw_properties.get("Certificate").and_then(|v| v.as_str()) {
                        if certificate.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::DirectoryService::Directory" => {
                    // Transfer servers can integrate with AWS Directory Service
                    if let Some(identity_provider_details) = entry.raw_properties.get("IdentityProviderDetails") {
                        if let Some(directory_id) = identity_provider_details.get("DirectoryId").and_then(|v| v.as_str()) {
                            if directory_id == resource.resource_id {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
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
        "AWS::Transfer::Server"
    }
}

/// Normalizer for AWS Transfer Family User Resources
pub struct TransferUserResourceNormalizer;

impl ResourceNormalizer for TransferUserResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("UserName"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-transfer-user")
            .to_string();

        let display_name = raw_response
            .get("UserName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Transfer::User".to_string(),
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
        
        // Transfer users relate to Transfer servers and IAM roles
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::Transfer::Server" => {
                    // Users belong to Transfer servers
                    if let Some(server_id) = entry.raw_properties.get("ServerId").and_then(|v| v.as_str()) {
                        if server_id == resource.resource_id {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::IAM::Role" => {
                    // Transfer users use IAM roles for access permissions
                    if let Some(role) = entry.raw_properties.get("Role").and_then(|v| v.as_str()) {
                        if role.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Transfer::User"
    }
}