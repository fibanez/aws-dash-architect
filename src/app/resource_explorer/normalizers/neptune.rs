use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Neptune DB Clusters
pub struct NeptuneDBClusterNormalizer;

impl ResourceNormalizer for NeptuneDBClusterNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("DBClusterIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-cluster")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Neptune::DBCluster".to_string(),
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

        // Neptune clusters contain DB instances
        if let Some(cluster_members) = entry
            .raw_properties
            .get("DBClusterMembers")
            .and_then(|v| v.as_array())
        {
            for member in cluster_members {
                if let Some(instance_id) =
                    member.get("DBInstanceIdentifier").and_then(|v| v.as_str())
                {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::Neptune::DBInstance"
                            && resource.resource_id == instance_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::Contains,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Neptune clusters are protected by security groups
        if let Some(security_groups) = entry
            .raw_properties
            .get("VpcSecurityGroups")
            .and_then(|v| v.as_array())
        {
            for sg_id_value in security_groups {
                if let Some(sg_id) = sg_id_value.as_str() {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::SecurityGroup"
                            && resource.resource_id == sg_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::ProtectedBy,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Neptune clusters use subnet groups (similar to RDS)
        if let Some(subnet_group_name) = entry
            .raw_properties
            .get("DBSubnetGroup")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::RDS::DBSubnetGroup"
                    && resource.resource_id == subnet_group_name
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::DeployedIn,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        // Neptune clusters can use KMS keys for encryption
        if let Some(kms_key_id) = entry
            .raw_properties
            .get("KmsKeyId")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::KMS::Key" && resource.resource_id == kms_key_id {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Neptune::DBCluster"
    }
}

/// Normalizer for Neptune DB Instances
pub struct NeptuneDBInstanceNormalizer;

impl ResourceNormalizer for NeptuneDBInstanceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("DBInstanceIdentifier")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-instance")
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Neptune::DBInstance".to_string(),
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

        // Neptune instances belong to clusters
        if let Some(cluster_id) = entry
            .raw_properties
            .get("DBClusterIdentifier")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::Neptune::DBCluster"
                    && resource.resource_id == cluster_id
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        // Neptune instances are protected by security groups
        if let Some(security_groups) = entry
            .raw_properties
            .get("VpcSecurityGroups")
            .and_then(|v| v.as_array())
        {
            for sg_id_value in security_groups {
                if let Some(sg_id) = sg_id_value.as_str() {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::EC2::SecurityGroup"
                            && resource.resource_id == sg_id
                        {
                            relationships.push(ResourceRelationship {
                                relationship_type: RelationshipType::ProtectedBy,
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                            });
                        }
                    }
                }
            }
        }

        // Neptune instances use subnet groups
        if let Some(subnet_group_name) = entry
            .raw_properties
            .get("DBSubnetGroupName")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::RDS::DBSubnetGroup"
                    && resource.resource_id == subnet_group_name
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::DeployedIn,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Neptune::DBInstance"
    }
}
