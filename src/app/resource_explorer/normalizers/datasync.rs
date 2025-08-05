use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for AWS DataSync Task Resources
pub struct DataSyncResourceNormalizer;

impl ResourceNormalizer for DataSyncResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("TaskArn"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-datasync-task")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::DataSync::Task".to_string(),
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
        
        // DataSync tasks relate to S3 buckets, EFS file systems, and other storage services
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::S3::Bucket" => {
                    // DataSync tasks often transfer data to/from S3 buckets
                    if let Some(source_location) = entry.raw_properties.get("SourceLocationArn").and_then(|v| v.as_str()) {
                        if source_location.contains("s3") && source_location.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                    
                    if let Some(destination_location) = entry.raw_properties.get("DestinationLocationArn").and_then(|v| v.as_str()) {
                        if destination_location.contains("s3") && destination_location.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::EFS::FileSystem" => {
                    // DataSync tasks can transfer data to/from EFS file systems
                    if let Some(source_location) = entry.raw_properties.get("SourceLocationArn").and_then(|v| v.as_str()) {
                        if source_location.contains("efs") && source_location.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                    
                    if let Some(destination_location) = entry.raw_properties.get("DestinationLocationArn").and_then(|v| v.as_str()) {
                        if destination_location.contains("efs") && destination_location.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::FSx::FileSystem" => {
                    // DataSync tasks can transfer data to/from FSx file systems
                    if let Some(source_location) = entry.raw_properties.get("SourceLocationArn").and_then(|v| v.as_str()) {
                        if source_location.contains("fsx") && source_location.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                    
                    if let Some(destination_location) = entry.raw_properties.get("DestinationLocationArn").and_then(|v| v.as_str()) {
                        if destination_location.contains("fsx") && destination_location.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::EC2::Subnet" => {
                    // DataSync agents need subnets for VPC connectivity
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::EC2::SecurityGroup" => {
                    // DataSync agents use security groups for network access control
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::IAM::Role" => {
                    // DataSync uses IAM roles for permissions to access source and destination locations
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::CloudWatch::LogGroup" => {
                    // DataSync tasks can log to CloudWatch
                    if let Some(log_group_arn) = entry.raw_properties.get("CloudWatchLogGroupArn").and_then(|v| v.as_str()) {
                        if log_group_arn.contains(&resource.resource_id) {
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
        "AWS::DataSync::Task"
    }
}

/// Normalizer for AWS DataSync Location Resources
pub struct DataSyncLocationResourceNormalizer;

impl ResourceNormalizer for DataSyncLocationResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("LocationArn"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-datasync-location")
            .to_string();

        let display_name = raw_response
            .get("LocationUri")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::DataSync::Location".to_string(),
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
        
        // DataSync locations relate to the actual storage services they represent
        if let Some(location_uri) = entry.raw_properties.get("LocationUri").and_then(|v| v.as_str()) {
            for resource in all_resources {
                match resource.resource_type.as_str() {
                    "AWS::S3::Bucket" => {
                        if location_uri.contains("s3://") && location_uri.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                    "AWS::EFS::FileSystem" => {
                        if location_uri.contains("efs") && location_uri.contains(&resource.resource_id) {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                    "AWS::FSx::FileSystem" => {
                        if location_uri.contains("fsx") && location_uri.contains(&resource.resource_id) {
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
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::DataSync::Location"
    }
}