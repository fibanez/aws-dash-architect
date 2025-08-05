use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Amazon WorkSpaces Resources
pub struct WorkSpacesResourceNormalizer;

impl ResourceNormalizer for WorkSpacesResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("WorkspaceId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-workspace")
            .to_string();

        let display_name = raw_response
            .get("UserName")
            .and_then(|v| v.as_str())
            .map(|username| format!("{} ({})", username, resource_id))
            .unwrap_or_else(|| resource_id.clone());

        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::WorkSpaces::Workspace".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status: Some(status),
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
        
        // WorkSpaces relate to directories, subnets, and security groups
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::WorkSpaces::Directory" => {
                    // WorkSpaces belong to directories
                    if let Some(directory_id) = entry.raw_properties.get("DirectoryId").and_then(|v| v.as_str()) {
                        if directory_id == resource.resource_id {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::EC2::Subnet" => {
                    // WorkSpaces use subnets
                    if let Some(subnet_id) = entry.raw_properties.get("SubnetId").and_then(|v| v.as_str()) {
                        if subnet_id == resource.resource_id {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::KMS::Key" => {
                    // WorkSpaces can use KMS keys for volume encryption
                    if let Some(volume_encryption_key) = entry.raw_properties.get("VolumeEncryptionKey").and_then(|v| v.as_str()) {
                        if volume_encryption_key.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::DirectoryService::Directory" => {
                    // WorkSpaces directories can be backed by Directory Service
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
        "AWS::WorkSpaces::Workspace"
    }
}

/// Normalizer for Amazon WorkSpaces Directory Resources
pub struct WorkSpacesDirectoryResourceNormalizer;

impl ResourceNormalizer for WorkSpacesDirectoryResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("DirectoryId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-workspace-directory")
            .to_string();

        let display_name = raw_response
            .get("DirectoryName")
            .or_else(|| raw_response.get("Alias"))
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::WorkSpaces::Directory".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status: Some(status),
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
        
        // WorkSpaces directories relate to subnets and Directory Service
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::EC2::Subnet" => {
                    // WorkSpaces directories use subnets
                    if let Some(subnet_ids) = entry.raw_properties.get("SubnetIds") {
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
                "AWS::DirectoryService::Directory" => {
                    // WorkSpaces directories are often backed by Directory Service
                    if let Some(directory_type) = entry.raw_properties.get("DirectoryType").and_then(|v| v.as_str()) {
                        if directory_type == "AD_CONNECTOR" || directory_type == "SIMPLE_AD" {
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
        "AWS::WorkSpaces::Directory"
    }
}