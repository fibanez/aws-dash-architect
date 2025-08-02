use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for RDS DB Instances
pub struct RDSDBInstanceNormalizer;

impl ResourceNormalizer for RDSDBInstanceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let db_instance_identifier = raw_response
            .get("DBInstanceIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-db-instance")
            .to_string();

        let display_name = extract_display_name(&raw_response, &db_instance_identifier);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

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

impl ResourceNormalizer for RDSDBClusterNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let db_cluster_identifier = raw_response
            .get("DBClusterIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-db-cluster")
            .to_string();

        let display_name = extract_display_name(&raw_response, &db_cluster_identifier);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

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

impl ResourceNormalizer for RDSDBSnapshotNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let snapshot_identifier = raw_response
            .get("DBSnapshotIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-snapshot")
            .to_string();

        let display_name = extract_display_name(&raw_response, &snapshot_identifier);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);

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

impl ResourceNormalizer for RDSDBParameterGroupNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let parameter_group_name = raw_response
            .get("DBParameterGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-parameter-group")
            .to_string();

        let display_name = extract_display_name(&raw_response, &parameter_group_name);
        let status = Some("Available".to_string()); // Parameter groups don't have status
        let tags = extract_tags(&raw_response);
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

impl ResourceNormalizer for RDSDBSubnetGroupNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
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
        let tags = extract_tags(&raw_response);
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
