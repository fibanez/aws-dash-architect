use super::{utils::*, AsyncResourceNormalizer, AWSResourceClient};
use crate::app::resource_explorer::state::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub struct EC2InstanceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2InstanceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let instance_id = raw_response
            .get("InstanceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-instance")
            .to_string();

        let display_name = extract_display_name(&raw_response, &instance_id);
        let status = extract_status(&raw_response);

        // Fetch tags asynchronously from AWS API with caching
        let tags = aws_client
            .fetch_tags_for_resource("AWS::EC2::Instance", &instance_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch tags for EC2 instance {}: {}", instance_id, e);
                Vec::new() // Graceful degradation
            });

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

#[async_trait]
impl AsyncResourceNormalizer for EC2SecurityGroupNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
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

        // Fetch tags asynchronously from AWS API with caching


        let tags = aws_client


            .fetch_tags_for_resource("AWS::EC2::SecurityGroup", &group_id, account, region)


            .await


            .unwrap_or_else(|e| {


                tracing::warn!("Failed to fetch tags for AWS::EC2::SecurityGroup {}: {}", group_id, e);


                Vec::new()


            });
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

#[async_trait]
impl AsyncResourceNormalizer for EC2VPCNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let vpc_id = raw_response
            .get("VpcId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-vpc")
            .to_string();

        tracing::debug!("🔍 EC2VPCNormalizer: Starting normalization for VPC {} in account {} region {}", vpc_id, account, region);

        let display_name = extract_display_name(&raw_response, &vpc_id);
        let status = raw_response
            .get("State")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        tracing::debug!("🔍 EC2VPCNormalizer: VPC {} - display_name={}, status={:?}", vpc_id, display_name, status);

        // Fetch tags asynchronously from AWS API with caching
        tracing::debug!("🔍 EC2VPCNormalizer: Fetching tags for VPC {} using async AWS client", vpc_id);

        let tags = aws_client
            .fetch_tags_for_resource("AWS::EC2::VPC", &vpc_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch tags for AWS::EC2::VPC {}: {}", vpc_id, e);
                Vec::new()
            });

        tracing::debug!("🔍 EC2VPCNormalizer: VPC {} - fetched {} tags", vpc_id, tags.len());
        for tag in &tags {
            tracing::debug!("🔍 EC2VPCNormalizer: VPC {} - tag: {}={}", vpc_id, tag.key, tag.value);
        }

        let properties = create_normalized_properties(&raw_response);

        let entry = ResourceEntry {
            resource_type: "AWS::EC2::VPC".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: vpc_id.clone(),
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
        };

        tracing::debug!("🔍 EC2VPCNormalizer: Successfully normalized VPC {} - resource_type={}", vpc_id, entry.resource_type);

        Ok(entry)
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

#[async_trait]
impl AsyncResourceNormalizer for EC2VolumeNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let volume_id = raw_response
            .get("VolumeId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-volume")
            .to_string();

        let display_name = extract_display_name(&raw_response, &volume_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::Volume", &volume_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::Volume {}: {}", volume_id, e);

                Vec::new()

            });
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

#[async_trait]
impl AsyncResourceNormalizer for EC2SnapshotNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let snapshot_id = raw_response
            .get("SnapshotId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-snapshot")
            .to_string();

        let display_name = extract_display_name(&raw_response, &snapshot_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::Snapshot", &snapshot_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::Snapshot {}: {}", snapshot_id, e);

                Vec::new()

            });

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

#[async_trait]
impl AsyncResourceNormalizer for EC2ImageNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let image_id = raw_response
            .get("ImageId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-ami")
            .to_string();

        let display_name = extract_display_name(&raw_response, &image_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::Image", &image_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::Image {}: {}", image_id, e);

                Vec::new()

            });

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

#[async_trait]
impl AsyncResourceNormalizer for EC2SubnetNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let subnet_id = raw_response
            .get("SubnetId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-subnet")
            .to_string();

        let display_name = extract_display_name(&raw_response, &subnet_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::Subnet", &subnet_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::Subnet {}: {}", subnet_id, e);

                Vec::new()

            });

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

#[async_trait]
impl AsyncResourceNormalizer for EC2InternetGatewayNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let igw_id = raw_response
            .get("InternetGatewayId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-igw")
            .to_string();

        let display_name = extract_display_name(&raw_response, &igw_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::InternetGateway", &igw_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::InternetGateway {}: {}", igw_id, e);

                Vec::new()

            });

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

#[async_trait]
impl AsyncResourceNormalizer for EC2RouteTableNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let route_table_id = raw_response
            .get("RouteTableId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-route-table")
            .to_string();

        let display_name = extract_display_name(&raw_response, &route_table_id);
        let status = Some("available".to_string()); // Route tables don't have status in the same way as instances
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::RouteTable", &route_table_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::RouteTable {}: {}", route_table_id, e);

                Vec::new()

            });
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

#[async_trait]
impl AsyncResourceNormalizer for EC2NatGatewayNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
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
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::NatGateway", &nat_gateway_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::NatGateway {}: {}", nat_gateway_id, e);

                Vec::new()

            });
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


#[async_trait]
impl AsyncResourceNormalizer for EC2NetworkInterfaceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let instance_id = raw_response
            .get("InstanceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-instance")
            .to_string();

        let display_name = extract_display_name(&raw_response, &instance_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);


        let mut entry = ResourceEntry {
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
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        };

        // Fetch tags (will be empty for resources that don't support tagging)
        entry.tags = aws_client
            .fetch_tags_for_resource(&entry.resource_type, &entry.resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch tags for {} {}: {:?}", entry.resource_type, entry.resource_id, e);
                Vec::new()
            });

        Ok(entry)
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::NetworkInterface"
    }
}

pub struct EC2NetworkInterfaceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2VPCEndpointNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
        let instance_id = raw_response
            .get("InstanceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-instance")
            .to_string();

        let display_name = extract_display_name(&raw_response, &instance_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response); // Fallback to local extraction for sync path // Fallback to local extraction for sync path // Fallback to local extraction for sync path
        let properties = create_normalized_properties(&raw_response);


        let mut entry = ResourceEntry {
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
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: assign_account_color(account),
            region_color: assign_region_color(region),
            query_timestamp,
        };

        // Fetch tags (will be empty for resources that don't support tagging)
        entry.tags = aws_client
            .fetch_tags_for_resource(&entry.resource_type, &entry.resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch tags for {} {}: {:?}", entry.resource_type, entry.resource_id, e);
                Vec::new()
            });

        Ok(entry)
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EC2::VPCEndpoint"
    }
}

pub struct EC2VPCEndpointNormalizer;

pub struct EC2NetworkAclNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for EC2NetworkAclNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let network_acl_id = raw_response
            .get("NetworkAclId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-network-acl")
            .to_string();

        let display_name = extract_display_name(&raw_response, &network_acl_id);
        let status = Some("available".to_string()); // Network ACLs don't have a status field like instances
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::NetworkAcl", &network_acl_id, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::NetworkAcl {}: {}", network_acl_id, e);

                Vec::new()

            });
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

#[async_trait]
impl AsyncResourceNormalizer for EC2KeyPairNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let key_name = raw_response
            .get("KeyName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-key")
            .to_string();

        let display_name = extract_display_name(&raw_response, &key_name);
        let status = Some("available".to_string()); // Key pairs don't have status like instances
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::EC2::KeyPair", &key_name, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::EC2::KeyPair {}: {}", key_name, e);

                Vec::new()

            });
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


