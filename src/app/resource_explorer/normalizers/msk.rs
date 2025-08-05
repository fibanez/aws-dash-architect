use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for MSK (Managed Streaming for Kafka) Clusters
pub struct MskNormalizer;

impl ResourceNormalizer for MskNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("ClusterArn"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-msk-cluster")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::MSK::Cluster".to_string(),
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

        // MSK clusters can be related to VPC subnets and security groups
        if let Some(provisioned) = entry.raw_properties.get("Provisioned") {
            if let Some(broker_info) = provisioned.get("BrokerNodeGroupInfo") {
                if let Some(client_subnets) = broker_info.get("ClientSubnets") {
                    if let Some(subnets_array) = client_subnets.as_array() {
                        for subnet in subnets_array {
                            if let Some(subnet_id) = subnet.as_str() {
                                // Find VPC subnets this cluster is deployed in
                                for resource in all_resources {
                                    if resource.resource_type == "AWS::EC2::Subnet" 
                                        && resource.resource_id == subnet_id {
                                        relationships.push(ResourceRelationship {
                                            relationship_type: RelationshipType::Uses,
                                            target_resource_id: resource.resource_id.clone(),
                                            target_resource_type: "AWS::EC2::Subnet".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for serverless VPC configuration
        if let Some(serverless) = entry.raw_properties.get("Serverless") {
            if let Some(vpc_configs) = serverless.get("VpcConfigs") {
                if let Some(configs_array) = vpc_configs.as_array() {
                    for config in configs_array {
                        // Subnet relationships
                        if let Some(subnet_ids) = config.get("SubnetIds") {
                            if let Some(subnets_array) = subnet_ids.as_array() {
                                for subnet in subnets_array {
                                    if let Some(subnet_id) = subnet.as_str() {
                                        for resource in all_resources {
                                            if resource.resource_type == "AWS::EC2::Subnet" 
                                                && resource.resource_id == subnet_id {
                                                relationships.push(ResourceRelationship {
                                                    relationship_type: RelationshipType::Uses,
                                                    target_resource_id: resource.resource_id.clone(),
                                                    target_resource_type: "AWS::EC2::Subnet".to_string(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Security group relationships
                        if let Some(sg_ids) = config.get("SecurityGroupIds") {
                            if let Some(sgs_array) = sg_ids.as_array() {
                                for sg in sgs_array {
                                    if let Some(sg_id) = sg.as_str() {
                                        for resource in all_resources {
                                            if resource.resource_type == "AWS::EC2::SecurityGroup" 
                                                && resource.resource_id == sg_id {
                                                relationships.push(ResourceRelationship {
                                                    relationship_type: RelationshipType::Uses,
                                                    target_resource_id: resource.resource_id.clone(),
                                                    target_resource_type: "AWS::EC2::SecurityGroup".to_string(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::MSK::Cluster"
    }
}