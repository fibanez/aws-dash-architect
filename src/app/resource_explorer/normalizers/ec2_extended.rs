use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for EC2 Transit Gateway
pub struct EC2TransitGatewayNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2TransitGatewayNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("TransitGatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-transit-gateway")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::EC2::TransitGateway", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::EC2::TransitGateway {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
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

#[async_trait]
impl AsyncResourceNormalizer for EC2VPCPeeringConnectionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("VpcPeeringConnectionId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-peering-connection")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::EC2::VPCPeeringConnection",
                &resource_id,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::EC2::VPCPeeringConnection {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
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

        // Extract VPC IDs from the peering connection
        let accepter_vpc_id = entry
            .raw_properties
            .get("AccepterVpcId")
            .and_then(|v| v.as_str());
        let requester_vpc_id = entry
            .raw_properties
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

#[async_trait]
impl AsyncResourceNormalizer for EC2FlowLogNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("FlowLogId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-flow-log")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::EC2::FlowLog", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::EC2::FlowLog {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
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

        // Flow logs can be attached to VPCs, subnets, or network interfaces
        if let Some(attached_resource_id) = entry
            .raw_properties
            .get("AttachedResourceId")
            .and_then(|v| v.as_str())
        {
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

#[async_trait]
impl AsyncResourceNormalizer for EC2VolumeAttachmentNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("AttachmentId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-attachment")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::EC2::VolumeAttachment", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::EC2::VolumeAttachment {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
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

        // Extract volume and instance IDs from the attachment
        let volume_id = entry
            .raw_properties
            .get("VolumeId")
            .and_then(|v| v.as_str());
        let instance_id = entry
            .raw_properties
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

async fn normalize_ec2_simple_resource(
    resource_type: &str,
    resource_id: String,
    raw_response: serde_json::Value,
    account: &str,
    region: &str,
    query_timestamp: DateTime<Utc>,
    aws_client: &AWSResourceClient,
) -> Result<ResourceEntry> {
    let display_name = extract_display_name(&raw_response, &resource_id);
    let status = extract_status(&raw_response);

    let tags = aws_client
        .fetch_tags_for_resource(resource_type, &resource_id, account, region)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to fetch tags for {} {}: {}",
                resource_type,
                resource_id,
                e
            );

            Vec::new()
        });

    let properties = create_normalized_properties(&raw_response);

    Ok(ResourceEntry {
        resource_type: resource_type.to_string(),
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

/// Normalizer for EC2 Elastic IP
pub struct EC2ElasticIPNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2ElasticIPNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("AllocationId")
            .and_then(|v| v.as_str())
            .or_else(|| raw_response.get("PublicIp").and_then(|v| v.as_str()))
            .unwrap_or("unknown-elastic-ip")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::ElasticIP",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::ElasticIP"
    }
}

/// Normalizer for EC2 Launch Template
pub struct EC2LaunchTemplateNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2LaunchTemplateNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("LaunchTemplateId")
            .and_then(|v| v.as_str())
            .or_else(|| raw_response.get("LaunchTemplateName").and_then(|v| v.as_str()))
            .unwrap_or("unknown-launch-template")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::LaunchTemplate",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::LaunchTemplate"
    }
}

/// Normalizer for EC2 Placement Group
pub struct EC2PlacementGroupNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2PlacementGroupNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("GroupId")
            .and_then(|v| v.as_str())
            .or_else(|| raw_response.get("GroupName").and_then(|v| v.as_str()))
            .unwrap_or("unknown-placement-group")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::PlacementGroup",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::PlacementGroup"
    }
}

/// Normalizer for EC2 Reserved Instance
pub struct EC2ReservedInstanceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2ReservedInstanceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ReservedInstancesId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-reserved-instance")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::ReservedInstance",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::ReservedInstance"
    }
}

/// Normalizer for EC2 Spot Instance Request
pub struct EC2SpotInstanceRequestNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2SpotInstanceRequestNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("SpotInstanceRequestId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-spot-request")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::SpotInstanceRequest",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::SpotInstanceRequest"
    }
}

/// Normalizer for EC2 DHCP Options Set
pub struct EC2DHCPOptionsNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2DHCPOptionsNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("DhcpOptionsId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-dhcp-options")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::DHCPOptions",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::DHCPOptions"
    }
}

/// Normalizer for EC2 Egress-Only Internet Gateway
pub struct EC2EgressOnlyInternetGatewayNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2EgressOnlyInternetGatewayNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("EgressOnlyInternetGatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-egress-only-igw")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::EgressOnlyInternetGateway",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::EgressOnlyInternetGateway"
    }
}

/// Normalizer for EC2 VPN Connection
pub struct EC2VPNConnectionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2VPNConnectionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("VpnConnectionId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-vpn-connection")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::VPNConnection",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::VPNConnection"
    }
}

/// Normalizer for EC2 VPN Gateway
pub struct EC2VPNGatewayNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2VPNGatewayNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("VpnGatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-vpn-gateway")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::VPNGateway",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::VPNGateway"
    }
}

/// Normalizer for EC2 Customer Gateway
pub struct EC2CustomerGatewayNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2CustomerGatewayNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("CustomerGatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-customer-gateway")
            .to_string();

        normalize_ec2_simple_resource(
            "AWS::EC2::CustomerGateway",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::CustomerGateway"
    }
}
