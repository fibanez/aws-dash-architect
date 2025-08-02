use super::{utils::*, ResourceNormalizer};
use crate::app::resource_explorer::state::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Classic Load Balancers (ELB)
pub struct ELBLoadBalancerNormalizer;

impl ResourceNormalizer for ELBLoadBalancerNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let lb_name = raw_response
            .get("LoadBalancerName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-elb")
            .to_string();

        let display_name = lb_name.clone();
        let status = raw_response
            .get("State")
            .and_then(|s| s.get("Code"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::ElasticLoadBalancing::LoadBalancer".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: lb_name.clone(),
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
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Relationship to VPC
        if let Some(vpc_id) = entry.raw_properties.get("VpcId").and_then(|id| id.as_str()) {
            relationships.push(ResourceRelationship {
                relationship_type: RelationshipType::MemberOf,
                target_resource_id: vpc_id.to_string(),
                target_resource_type: "AWS::EC2::VPC".to_string(),
            });
        }

        // Relationships to subnets
        if let Some(subnets) = entry
            .raw_properties
            .get("Subnets")
            .and_then(|s| s.as_array())
        {
            for subnet in subnets {
                if let Some(subnet_id) = subnet.as_str() {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: subnet_id.to_string(),
                        target_resource_type: "AWS::EC2::Subnet".to_string(),
                    });
                }
            }
        }

        // Relationships to security groups
        if let Some(security_groups) = entry
            .raw_properties
            .get("SecurityGroups")
            .and_then(|sg| sg.as_array())
        {
            for sg in security_groups {
                if let Some(sg_id) = sg.as_str() {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: sg_id.to_string(),
                        target_resource_type: "AWS::EC2::SecurityGroup".to_string(),
                    });
                }
            }
        }

        // Relationships to EC2 instances
        if let Some(instances) = entry
            .raw_properties
            .get("Instances")
            .and_then(|i| i.as_array())
        {
            for instance in instances {
                if let Some(instance_id) = instance.as_str() {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: instance_id.to_string(),
                        target_resource_type: "AWS::EC2::Instance".to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ElasticLoadBalancing::LoadBalancer"
    }
}
