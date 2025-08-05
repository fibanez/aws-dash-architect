use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Amazon FSx File System Resources
pub struct FsxResourceNormalizer;

impl ResourceNormalizer for FsxResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("FileSystemId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-fsx-filesystem")
            .to_string();

        let display_name = raw_response
            .get("DNSName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Lifecycle")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::FSx::FileSystem".to_string(),
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
        
        // FSx file systems relate to VPCs, subnets, security groups, and other AWS services
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::EC2::VPC" => {
                    // FSx file systems are deployed in VPCs
                    if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|v| v.as_str()) {
                        if vpc_id == resource.resource_id {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::EC2::Subnet" => {
                    // FSx file systems use subnets
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
                "AWS::EC2::NetworkInterface" => {
                    // FSx file systems create network interfaces
                    if let Some(network_interface_ids) = entry.raw_properties.get("NetworkInterfaceIds") {
                        if let Some(interfaces) = network_interface_ids.as_array() {
                            for interface in interfaces {
                                if let Some(interface_id) = interface.as_str() {
                                    if interface_id == resource.resource_id {
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
                "AWS::KMS::Key" => {
                    // FSx file systems can use KMS keys for encryption
                    if let Some(kms_key_id) = entry.raw_properties.get("KmsKeyId").and_then(|v| v.as_str()) {
                        if kms_key_id.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::S3::Bucket" => {
                    // FSx Lustre can integrate with S3 buckets for data repository
                    if let Some(lustre_config) = entry.raw_properties.get("LustreConfiguration") {
                        if let Some(data_repo_config) = lustre_config.get("DataRepositoryConfiguration") {
                            if let Some(import_path) = data_repo_config.get("ImportPath").and_then(|v| v.as_str()) {
                                if import_path.contains("s3://") && import_path.contains(&resource.resource_id) {
                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::Uses,
                                        target_resource_id: resource.resource_id.clone(),
                                        target_resource_type: resource.resource_type.clone(),
                                    });
                                }
                            }
                            
                            if let Some(export_path) = data_repo_config.get("ExportPath").and_then(|v| v.as_str()) {
                                if export_path.contains("s3://") && export_path.contains(&resource.resource_id) {
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
                "AWS::DirectoryService::Directory" => {
                    // FSx Windows file systems can integrate with Active Directory
                    if let Some(windows_config) = entry.raw_properties.get("WindowsConfiguration") {
                        if let Some(active_directory_id) = windows_config.get("ActiveDirectoryId").and_then(|v| v.as_str()) {
                            if active_directory_id == resource.resource_id {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::DataSync::Task" => {
                    // DataSync tasks often use FSx as source or destination
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
        "AWS::FSx::FileSystem"
    }
}

/// Normalizer for Amazon FSx Backup Resources
pub struct FsxBackupResourceNormalizer;

impl ResourceNormalizer for FsxBackupResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("BackupId"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-fsx-backup")
            .to_string();

        let display_name = resource_id.clone();

        let status = raw_response
            .get("Lifecycle")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::FSx::Backup".to_string(),
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
        
        // FSx backups relate to their source file systems
        if let Some(file_system_id) = entry.raw_properties.get("FileSystemId").and_then(|v| v.as_str()) {
            for resource in all_resources {
                if resource.resource_type == "AWS::FSx::FileSystem" && resource.resource_id == file_system_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                    break;
                }
            }
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::FSx::Backup"
    }
}