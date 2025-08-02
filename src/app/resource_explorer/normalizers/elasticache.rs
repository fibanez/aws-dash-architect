use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for ElastiCache Cache Clusters
pub struct ElastiCacheCacheClusterNormalizer;

impl ResourceNormalizer for ElastiCacheCacheClusterNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("CacheClusterId")
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
            resource_type: "AWS::ElastiCache::CacheCluster".to_string(),
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

        // ElastiCache clusters are associated with security groups
        if let Some(security_groups) = entry
            .raw_properties
            .get("SecurityGroups")
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

        // ElastiCache clusters are deployed in subnet groups
        if let Some(subnet_group_name) = entry
            .raw_properties
            .get("CacheSubnetGroupName")
            .and_then(|v| v.as_str())
        {
            for resource in all_resources {
                if resource.resource_type == "AWS::ElastiCache::SubnetGroup"
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
        "AWS::ElastiCache::CacheCluster"
    }
}

/// Normalizer for ElastiCache Replication Groups
pub struct ElastiCacheReplicationGroupNormalizer;

impl ResourceNormalizer for ElastiCacheReplicationGroupNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ReplicationGroupId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-replication-group")
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
            resource_type: "AWS::ElastiCache::ReplicationGroup".to_string(),
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

        // Replication groups contain member clusters
        if let Some(member_clusters) = entry
            .raw_properties
            .get("MemberClusters")
            .and_then(|v| v.as_array())
        {
            for cluster_id_value in member_clusters {
                if let Some(cluster_id) = cluster_id_value.as_str() {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::ElastiCache::CacheCluster"
                            && resource.resource_id == cluster_id
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

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ElastiCache::ReplicationGroup"
    }
}
