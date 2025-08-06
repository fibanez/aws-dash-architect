use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for EC2 Transit Gateway
pub struct EC2TransitGatewayNormalizer;

impl ResourceNormalizer for EC2TransitGatewayNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("TransitGatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-transit-gateway")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::TransitGateway".to_string(),
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
        _entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();
        
        // Transit Gateways connect to VPCs through attachments
        for resource in all_resources {
            if resource.resource_type == "AWS::EC2::VPC" {
                // Note: This is a simplification - actual TGW attachments would be separate resources
                relationships.push(ResourceRelationship {
                    relationship_type: RelationshipType::Uses,
                    target_resource_id: resource.resource_id.clone(),
                    target_resource_type: resource.resource_type.clone(),
                });
            }
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::TransitGateway"
    }
}

/// Normalizer for EC2 VPC Peering Connection
pub struct EC2VPCPeeringConnectionNormalizer;

impl ResourceNormalizer for EC2VPCPeeringConnectionNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("VpcPeeringConnectionId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-peering-connection")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::VPCPeeringConnection".to_string(),
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
        
        // Extract VPC IDs from the peering connection
        let accepter_vpc_id = entry.raw_properties
            .get("AccepterVpcId")
            .and_then(|v| v.as_str());
        let requester_vpc_id = entry.raw_properties
            .get("RequesterVpcId")
            .and_then(|v| v.as_str());
        
        // Find related VPCs
        for resource in all_resources {
            if resource.resource_type == "AWS::EC2::VPC" {
                if let Some(accepter_id) = accepter_vpc_id {
                    if resource.resource_id == accepter_id {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                    }
                }
                if let Some(requester_id) = requester_vpc_id {
                    if resource.resource_id == requester_id {
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
        "AWS::EC2::VPCPeeringConnection"
    }
}

/// Normalizer for EC2 VPC Flow Log
pub struct EC2FlowLogNormalizer;

impl ResourceNormalizer for EC2FlowLogNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("FlowLogId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-flow-log")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::FlowLog".to_string(),
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
        
        // Flow logs can be attached to VPCs, subnets, or network interfaces
        if let Some(attached_resource_id) = entry.raw_properties
            .get("AttachedResourceId")
            .and_then(|v| v.as_str()) {
            
            for resource in all_resources {
                if resource.resource_id == attached_resource_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::AttachedTo,
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
        "AWS::EC2::FlowLog"
    }
}

/// Normalizer for EC2 EBS Volume Attachment
pub struct EC2VolumeAttachmentNormalizer;

impl ResourceNormalizer for EC2VolumeAttachmentNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("AttachmentId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-attachment")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::VolumeAttachment".to_string(),
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
        
        // Extract volume and instance IDs from the attachment
        let volume_id = entry.raw_properties
            .get("VolumeId")
            .and_then(|v| v.as_str());
        let instance_id = entry.raw_properties
            .get("InstanceId")
            .and_then(|v| v.as_str());
        
        // Find related EC2 instances and EBS volumes
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::EC2::Volume" => {
                    if let Some(vol_id) = volume_id {
                        if resource.resource_id == vol_id {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
                "AWS::EC2::Instance" => {
                    if let Some(inst_id) = instance_id {
                        if resource.resource_id == inst_id {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::AttachedTo,
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
        "AWS::EC2::VolumeAttachment"
    }
}