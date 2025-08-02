use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for EKS Clusters
pub struct EKSClusterNormalizer;

impl ResourceNormalizer for EKSClusterNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let cluster_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-cluster")
            .to_string();

        let display_name = extract_display_name(&raw_response, &cluster_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EKS::Cluster".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: cluster_name,
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

        // Find related VPC
        if let Some(vpc_config) = entry.raw_properties.get("ResourcesVpcConfig") {
            if let Some(vpc_id) = vpc_config.get("VpcId").and_then(|v| v.as_str()) {
                for resource in all_resources {
                    if resource.resource_type == "AWS::EC2::VPC" && resource.resource_id == vpc_id {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: vpc_id.to_string(),
                            target_resource_type: "AWS::EC2::VPC".to_string(),
                        });
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EKS::Cluster"
    }
}
