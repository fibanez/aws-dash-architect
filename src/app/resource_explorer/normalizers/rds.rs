use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for RDS DB Instances
pub struct RDSDBInstanceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for RDSDBInstanceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let db_instance_identifier = raw_response
            .get("DBInstanceIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-db-instance")
            .to_string();

        let display_name = extract_display_name(&raw_response, &db_instance_identifier);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::RDS::DBInstance",
                &db_instance_identifier,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::RDS::DBInstance {}: {}",
                    db_instance_identifier,
                    e
                );

                Vec::new()
            });

        // Extract creation time
        let _creation_date = raw_response
            .get("InstanceCreateTime")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::RDS::DBInstance".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: db_instance_identifier.clone(),
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

        // Find related VPCs and security groups
        if let Some(vpc_security_groups) = entry.raw_properties.get("VpcSecurityGroups") {
            if let Some(security_groups) = vpc_security_groups.as_array() {
                for sg in security_groups {
                    if let Some(sg_id) = sg.get("VpcSecurityGroupId").and_then(|v| v.as_str()) {
                        // Find the security group resource
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::SecurityGroup"
                                && resource.resource_id == sg_id
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: sg_id.to_string(),
                                    target_resource_type: "AWS::EC2::SecurityGroup".to_string(),
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
        "AWS::RDS::DBInstance"
    }
}

/// Normalizer for RDS DB Clusters
pub struct RDSDBClusterNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for RDSDBClusterNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let db_cluster_identifier = raw_response
            .get("DBClusterIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-db-cluster")
            .to_string();

        let display_name = extract_display_name(&raw_response, &db_cluster_identifier);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::RDS::DBCluster",
                &db_cluster_identifier,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::RDS::DBCluster {}: {}",
                    db_cluster_identifier,
                    e
                );

                Vec::new()
            });

        // Extract creation time
        let _creation_date = raw_response
            .get("ClusterCreateTime")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::RDS::DBCluster".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: db_cluster_identifier.clone(),
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

        // Find related security groups
        if let Some(vpc_security_groups) = entry.raw_properties.get("VpcSecurityGroups") {
            if let Some(security_groups) = vpc_security_groups.as_array() {
                for sg in security_groups {
                    if let Some(sg_id) = sg.get("VpcSecurityGroupId").and_then(|v| v.as_str()) {
                        // Find the security group resource
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::SecurityGroup"
                                && resource.resource_id == sg_id
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: sg_id.to_string(),
                                    target_resource_type: "AWS::EC2::SecurityGroup".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Find related DB cluster members
        if let Some(members) = entry.raw_properties.get("DBClusterMembers") {
            if let Some(member_array) = members.as_array() {
                for member in member_array {
                    if let Some(instance_id) =
                        member.get("DBInstanceIdentifier").and_then(|v| v.as_str())
                    {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Contains,
                            target_resource_id: instance_id.to_string(),
                            target_resource_type: "AWS::RDS::DBInstance".to_string(),
                        });
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::RDS::DBCluster"
    }
}

/// Normalizer for RDS DB Snapshots
pub struct RDSDBSnapshotNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for RDSDBSnapshotNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let snapshot_identifier = raw_response
            .get("DBSnapshotIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-snapshot")
            .to_string();

        let display_name = extract_display_name(&raw_response, &snapshot_identifier);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::RDS::DBSnapshot",
                &snapshot_identifier,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::RDS::DBSnapshot {}: {}",
                    snapshot_identifier,
                    e
                );

                Vec::new()
            });

        // Extract creation time
        let _creation_date = raw_response
            .get("SnapshotCreateTime")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::RDS::DBSnapshot".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: snapshot_identifier.clone(),
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

        // Find the source DB instance
        if let Some(db_instance_id) = entry
            .raw_properties
            .get("DBInstanceIdentifier")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::RDS::DBInstance"
                    && resource.resource_id == db_instance_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: db_instance_id.to_string(),
                        target_resource_type: "AWS::RDS::DBInstance".to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::RDS::DBSnapshot"
    }
}

/// Normalizer for RDS DB Parameter Groups
pub struct RDSDBParameterGroupNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for RDSDBParameterGroupNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let parameter_group_name = raw_response
            .get("DBParameterGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-parameter-group")
            .to_string();

        let display_name = extract_display_name(&raw_response, &parameter_group_name);
        let status = Some("Available".to_string()); // Parameter groups don't have status
                                                    // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::RDS::DBParameterGroup",
                &parameter_group_name,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::RDS::DBParameterGroup {}: {}",
                    parameter_group_name,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::RDS::DBParameterGroup".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: parameter_group_name.clone(),
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
        // Parameter groups can be used by DB instances and clusters
        // but this requires cross-referencing which is complex
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::RDS::DBParameterGroup"
    }
}

/// Normalizer for RDS DB Subnet Groups
pub struct RDSDBSubnetGroupNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for RDSDBSubnetGroupNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let subnet_group_name = raw_response
            .get("DBSubnetGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-subnet-group")
            .to_string();

        let display_name = extract_display_name(&raw_response, &subnet_group_name);
        let status = raw_response
            .get("SubnetGroupStatus")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::RDS::DBSubnetGroup",
                &subnet_group_name,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::RDS::DBSubnetGroup {}: {}",
                    subnet_group_name,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::RDS::DBSubnetGroup".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: subnet_group_name.clone(),
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

        // Map to VPC if present
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

        // Map to subnets if present
        if let Some(subnets) = entry
            .raw_properties
            .get("Subnets")
            .and_then(|v| v.as_array())
        {
            for subnet in subnets {
                if let Some(subnet_id) = subnet.get("SubnetIdentifier").and_then(|v| v.as_str()) {
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
        "AWS::RDS::DBSubnetGroup"
    }
}
