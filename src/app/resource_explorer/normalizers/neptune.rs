use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Neptune DB Clusters
pub struct NeptuneDBClusterNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for NeptuneDBClusterNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &crate::app::resource_explorer::aws_client::AWSResourceClient,
    ) -> Result<ResourceEntry> {
        // Inline normalization logic
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

        let mut entry = ResourceEntry {
            resource_type: "AWS::Neptune::DBCluster".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: raw_response,
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
                tracing::warn!(
                    "Failed to fetch tags for {} {}: {:?}",
                    entry.resource_type,
                    entry.resource_id,
                    e
                );
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
        "AWS::Neptune::DBCluster"
    }
}

/// Normalizer for Neptune DB Instances
pub struct NeptuneDBInstanceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for NeptuneDBInstanceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
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

        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Neptune::DBInstance", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Neptune::DBInstance {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::Neptune::DBInstance".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: raw_response,
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

        // Neptune instances belong to clusters
        if let Some(cluster_id) = entry
            .properties
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
            .properties
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
            .properties
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
