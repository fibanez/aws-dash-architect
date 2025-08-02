use super::{utils::*, ResourceNormalizer};
use crate::app::resource_explorer::state::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct EC2InstanceNormalizer;

impl ResourceNormalizer for EC2InstanceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let instance_id = raw_response
            .get("InstanceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-instance")
            .to_string();

        let display_name = extract_display_name(&raw_response, &instance_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::Instance".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: instance_id,
            display_name,
            status,
            properties,
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags,
            relationships: Vec::new(), // Will be populated later
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Find security groups this instance uses
        if let Some(security_groups) = entry
            .raw_properties
            .get("SecurityGroups")
            .and_then(|sg| sg.as_array())
        {
            for sg in security_groups {
                if let Some(group_id) = sg.get("GroupId").and_then(|id| id.as_str()) {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: group_id.to_string(),
                        target_resource_type: "AWS::EC2::SecurityGroup".to_string(),
                    });
                }
            }
        }

        // Find VPC this instance belongs to
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|id| id.as_str()) {
            relationships.push(ResourceRelationship {
                relationship_type: RelationshipType::MemberOf,
                target_resource_id: vpc_id.to_string(),
                target_resource_type: "AWS::EC2::VPC".to_string(),
            });
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::Instance"
    }
}

pub struct EC2SecurityGroupNormalizer;

impl ResourceNormalizer for EC2SecurityGroupNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let group_id = raw_response
            .get("GroupId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-sg")
            .to_string();

        let base_name = raw_response
            .get("GroupName")
            .and_then(|v| v.as_str())
            .unwrap_or(&group_id);

        let display_name = if let Some(vpc_id) = raw_response.get("VpcId").and_then(|v| v.as_str())
        {
            format!("{} ({})", base_name, vpc_id)
        } else {
            base_name.to_string()
        };

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::SecurityGroup".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: group_id,
            display_name,
            status: None,
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
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Find VPC this security group belongs to
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|id| id.as_str()) {
            relationships.push(ResourceRelationship {
                relationship_type: RelationshipType::MemberOf,
                target_resource_id: vpc_id.to_string(),
                target_resource_type: "AWS::EC2::VPC".to_string(),
            });
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::SecurityGroup"
    }
}

pub struct EC2VPCNormalizer;

impl ResourceNormalizer for EC2VPCNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let vpc_id = raw_response
            .get("VpcId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-vpc")
            .to_string();

        let display_name = extract_display_name(&raw_response, &vpc_id);
        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::VPC".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: vpc_id,
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
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // VPCs are top-level resources, no relationships to extract
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::VPC"
    }
}

/// Normalizer for EBS Volumes
pub struct EC2VolumeNormalizer;

impl ResourceNormalizer for EC2VolumeNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let volume_id = raw_response
            .get("VolumeId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-volume")
            .to_string();

        let display_name = extract_display_name(&raw_response, &volume_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::Volume".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: volume_id,
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

        // Find attached instances
        if let Some(attachments) = entry.raw_properties.get("Attachments") {
            if let Some(attachment_array) = attachments.as_array() {
                for attachment in attachment_array {
                    if let Some(instance_id) = attachment.get("InstanceId").and_then(|v| v.as_str())
                    {
                        // Find the instance resource
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::Instance"
                                && resource.resource_id == instance_id
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::AttachedTo,
                                    target_resource_id: instance_id.to_string(),
                                    target_resource_type: "AWS::EC2::Instance".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::Volume"
    }
}

/// Normalizer for EBS Snapshots
pub struct EC2SnapshotNormalizer;

impl ResourceNormalizer for EC2SnapshotNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let snapshot_id = raw_response
            .get("SnapshotId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-snapshot")
            .to_string();

        let display_name = extract_display_name(&raw_response, &snapshot_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

        // Extract start time
        let _creation_date = raw_response
            .get("StartTime")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::Snapshot".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: snapshot_id,
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

        // Find the source volume
        if let Some(volume_id) = entry
            .raw_properties
            .get("VolumeId")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::Volume" && resource.resource_id == volume_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: volume_id.to_string(),
                        target_resource_type: "AWS::EC2::Volume".to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::Snapshot"
    }
}

/// Normalizer for AMIs
pub struct EC2ImageNormalizer;

impl ResourceNormalizer for EC2ImageNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let image_id = raw_response
            .get("ImageId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-ami")
            .to_string();

        let display_name = extract_display_name(&raw_response, &image_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

        // Extract creation date
        let _creation_date = raw_response
            .get("CreationDate")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::Image".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: image_id,
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
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // AMIs typically don't have direct relationships in this context
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::Image"
    }
}

/// Normalizer for Subnets
pub struct EC2SubnetNormalizer;

impl ResourceNormalizer for EC2SubnetNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let subnet_id = raw_response
            .get("SubnetId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-subnet")
            .to_string();

        let display_name = extract_display_name(&raw_response, &subnet_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::Subnet".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: subnet_id,
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

        // Find the parent VPC
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|v| v.as_str()) {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::VPC" && resource.resource_id == vpc_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: vpc_id.to_string(),
                        target_resource_type: "AWS::EC2::VPC".to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::Subnet"
    }
}

/// Normalizer for Internet Gateways
pub struct EC2InternetGatewayNormalizer;

impl ResourceNormalizer for EC2InternetGatewayNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let igw_id = raw_response
            .get("InternetGatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-igw")
            .to_string();

        let display_name = extract_display_name(&raw_response, &igw_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::InternetGateway".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: igw_id,
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

        // Find attached VPCs
        if let Some(attachments) = entry.raw_properties.get("Attachments") {
            if let Some(attachment_array) = attachments.as_array() {
                for attachment in attachment_array {
                    if let Some(vpc_id) = attachment.get("VpcId").and_then(|v| v.as_str()) {
                        // Find the VPC resource
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::VPC"
                                && resource.resource_id == vpc_id
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::AttachedTo,
                                    target_resource_id: vpc_id.to_string(),
                                    target_resource_type: "AWS::EC2::VPC".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::InternetGateway"
    }
}

pub struct EC2RouteTableNormalizer;

impl ResourceNormalizer for EC2RouteTableNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let route_table_id = raw_response
            .get("RouteTableId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-route-table")
            .to_string();

        let display_name = extract_display_name(&raw_response, &route_table_id);
        let status = Some("available".to_string()); // Route tables don't have status in the same way as instances
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::RouteTable".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: route_table_id,
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

        // Find the parent VPC
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|v| v.as_str()) {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::VPC" && resource.resource_id == vpc_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: vpc_id.to_string(),
                        target_resource_type: "AWS::EC2::VPC".to_string(),
                    });
                }
            }
        }

        // Find associated subnets from the associations array
        if let Some(associations) = entry
            .raw_properties
            .get("Associations")
            .and_then(|v| v.as_array())
        {
            for association in associations {
                if let Some(subnet_id) = association.get("SubnetId").and_then(|v| v.as_str()) {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::Subnet"
                            && resource.resource_id == subnet_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: subnet_id.to_string(),
                                target_resource_type: "AWS::EC2::Subnet".to_string(),
                            });
                        }
                    }
                }

                // Also check for gateway associations
                if let Some(gateway_id) = association.get("GatewayId").and_then(|v| v.as_str()) {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::InternetGateway"
                            && resource.resource_id == gateway_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: gateway_id.to_string(),
                                target_resource_type: "AWS::EC2::InternetGateway".to_string(),
                            });
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::RouteTable"
    }
}

pub struct EC2NatGatewayNormalizer;

impl ResourceNormalizer for EC2NatGatewayNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let nat_gateway_id = raw_response
            .get("NatGatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-nat-gateway")
            .to_string();

        let display_name = extract_display_name(&raw_response, &nat_gateway_id);
        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::NatGateway".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: nat_gateway_id,
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

        // Find the parent VPC
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|v| v.as_str()) {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::VPC" && resource.resource_id == vpc_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: vpc_id.to_string(),
                        target_resource_type: "AWS::EC2::VPC".to_string(),
                    });
                }
            }
        }

        // Find the associated subnet
        if let Some(subnet_id) = entry
            .raw_properties
            .get("SubnetId")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::Subnet" && resource.resource_id == subnet_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: subnet_id.to_string(),
                        target_resource_type: "AWS::EC2::Subnet".to_string(),
                    });
                }
            }
        }

        // Find network interfaces from NAT gateway addresses
        if let Some(addresses) = entry
            .raw_properties
            .get("NatGatewayAddresses")
            .and_then(|v| v.as_array())
        {
            for address in addresses {
                if let Some(network_interface_id) =
                    address.get("NetworkInterfaceId").and_then(|v| v.as_str())
                {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::NetworkInterface"
                            && resource.resource_id == network_interface_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: network_interface_id.to_string(),
                                target_resource_type: "AWS::EC2::NetworkInterface".to_string(),
                            });
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::NatGateway"
    }
}

pub struct EC2NetworkInterfaceNormalizer;

impl ResourceNormalizer for EC2NetworkInterfaceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let network_interface_id = raw_response
            .get("NetworkInterfaceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-network-interface")
            .to_string();

        let display_name = extract_display_name(&raw_response, &network_interface_id);
        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::NetworkInterface".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: network_interface_id,
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

        // Find the parent VPC
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|v| v.as_str()) {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::VPC" && resource.resource_id == vpc_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: vpc_id.to_string(),
                        target_resource_type: "AWS::EC2::VPC".to_string(),
                    });
                }
            }
        }

        // Find the associated subnet
        if let Some(subnet_id) = entry
            .raw_properties
            .get("SubnetId")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::Subnet" && resource.resource_id == subnet_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: subnet_id.to_string(),
                        target_resource_type: "AWS::EC2::Subnet".to_string(),
                    });
                }
            }
        }

        // Find attached instance from attachment information
        if let Some(attachment) = entry
            .raw_properties
            .get("Attachment")
            .and_then(|v| v.as_object())
        {
            if let Some(instance_id) = attachment.get("InstanceId").and_then(|v| v.as_str()) {
                for resource in all_resources {
                    if resource.resource_type == "AWS::EC2::Instance"
                        && resource.resource_id == instance_id
                    {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::AttachedTo,
                            target_resource_id: instance_id.to_string(),
                            target_resource_type: "AWS::EC2::Instance".to_string(),
                        });
                    }
                }
            }
        }

        // Find associated security groups
        if let Some(groups) = entry
            .raw_properties
            .get("Groups")
            .and_then(|v| v.as_array())
        {
            for group in groups {
                if let Some(group_id) = group.get("GroupId").and_then(|v| v.as_str()) {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::SecurityGroup"
                            && resource.resource_id == group_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: group_id.to_string(),
                                target_resource_type: "AWS::EC2::SecurityGroup".to_string(),
                            });
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::NetworkInterface"
    }
}

pub struct EC2VPCEndpointNormalizer;

impl ResourceNormalizer for EC2VPCEndpointNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let vpc_endpoint_id = raw_response
            .get("VpcEndpointId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-vpc-endpoint")
            .to_string();

        let display_name = extract_display_name(&raw_response, &vpc_endpoint_id);
        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::VPCEndpoint".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: vpc_endpoint_id,
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

        // Find the parent VPC
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|v| v.as_str()) {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::VPC" && resource.resource_id == vpc_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: vpc_id.to_string(),
                        target_resource_type: "AWS::EC2::VPC".to_string(),
                    });
                }
            }
        }

        // Find associated route tables
        if let Some(route_table_ids) = entry
            .raw_properties
            .get("RouteTableIds")
            .and_then(|v| v.as_array())
        {
            for route_table_id in route_table_ids {
                if let Some(rt_id) = route_table_id.as_str() {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::RouteTable"
                            && resource.resource_id == rt_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: rt_id.to_string(),
                                target_resource_type: "AWS::EC2::RouteTable".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Find associated subnets
        if let Some(subnet_ids) = entry
            .raw_properties
            .get("SubnetIds")
            .and_then(|v| v.as_array())
        {
            for subnet_id in subnet_ids {
                if let Some(sub_id) = subnet_id.as_str() {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::Subnet"
                            && resource.resource_id == sub_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: sub_id.to_string(),
                                target_resource_type: "AWS::EC2::Subnet".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Find associated security groups
        if let Some(groups) = entry
            .raw_properties
            .get("Groups")
            .and_then(|v| v.as_array())
        {
            for group in groups {
                if let Some(group_id) = group.get("GroupId").and_then(|v| v.as_str()) {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::SecurityGroup"
                            && resource.resource_id == group_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: group_id.to_string(),
                                target_resource_type: "AWS::EC2::SecurityGroup".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Find associated network interfaces
        if let Some(network_interface_ids) = entry
            .raw_properties
            .get("NetworkInterfaceIds")
            .and_then(|v| v.as_array())
        {
            for eni_id in network_interface_ids {
                if let Some(eni_id_str) = eni_id.as_str() {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::NetworkInterface"
                            && resource.resource_id == eni_id_str
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: eni_id_str.to_string(),
                                target_resource_type: "AWS::EC2::NetworkInterface".to_string(),
                            });
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::VPCEndpoint"
    }
}

pub struct EC2NetworkAclNormalizer;

impl ResourceNormalizer for EC2NetworkAclNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let network_acl_id = raw_response
            .get("NetworkAclId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-network-acl")
            .to_string();

        let display_name = extract_display_name(&raw_response, &network_acl_id);
        let status = Some("available".to_string()); // Network ACLs don't have a status field like instances
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::NetworkAcl".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: network_acl_id,
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

        // Find the parent VPC
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|v| v.as_str()) {
            for resource in all_resources {
                if resource.resource_type == "AWS::EC2::VPC" && resource.resource_id == vpc_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: vpc_id.to_string(),
                        target_resource_type: "AWS::EC2::VPC".to_string(),
                    });
                }
            }
        }

        // Find associated subnets from associations
        if let Some(associations) = entry
            .raw_properties
            .get("Associations")
            .and_then(|v| v.as_array())
        {
            for association in associations {
                if let Some(subnet_id) = association.get("SubnetId").and_then(|v| v.as_str()) {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::Subnet"
                            && resource.resource_id == subnet_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Uses,
                                target_resource_id: subnet_id.to_string(),
                                target_resource_type: "AWS::EC2::Subnet".to_string(),
                            });
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::NetworkAcl"
    }
}

pub struct EC2KeyPairNormalizer;

impl ResourceNormalizer for EC2KeyPairNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let key_name = raw_response
            .get("KeyName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-key")
            .to_string();

        let display_name = extract_display_name(&raw_response, &key_name);
        let status = Some("available".to_string()); // Key pairs don't have status like instances
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EC2::KeyPair".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: key_name,
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
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Key pairs are regional resources that don't have direct relationships
        // They are used by EC2 instances but this relationship is tracked from the instance side
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::KeyPair"
    }
}
