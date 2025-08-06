use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for Auto Scaling Groups
pub struct AutoScalingGroupNormalizer;

impl ResourceNormalizer for AutoScalingGroupNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("AutoScalingGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-asg")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::AutoScaling::AutoScalingGroup".to_string(),
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
        
        // Auto Scaling Groups relate to Load Balancers
        if let Some(load_balancer_names) = entry.raw_properties.get("LoadBalancerNames") {
            if let Some(lb_array) = load_balancer_names.as_array() {
                for resource in all_resources {
                    if resource.resource_type == "AWS::ElasticLoadBalancing::LoadBalancer" {
                        for lb_name in lb_array {
                            if let Some(lb_name_str) = lb_name.as_str() {
                                if resource.display_name == lb_name_str || resource.resource_id == lb_name_str {
                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::Uses,
                                        target_resource_id: resource.resource_id.clone(),
                                        target_resource_type: resource.resource_type.clone(),
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Auto Scaling Groups relate to Target Groups (ALB/NLB)
        if let Some(target_group_arns) = entry.raw_properties.get("TargetGroupARNs") {
            if let Some(tg_array) = target_group_arns.as_array() {
                for resource in all_resources {
                    if resource.resource_type == "AWS::ElasticLoadBalancingV2::TargetGroup" {
                        for tg_arn in tg_array {
                            if let Some(tg_arn_str) = tg_arn.as_str() {
                                if resource.raw_properties.get("TargetGroupArn")
                                    .and_then(|v| v.as_str()) == Some(tg_arn_str) {
                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::Uses,
                                        target_resource_id: resource.resource_id.clone(),
                                        target_resource_type: resource.resource_type.clone(),
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Auto Scaling Groups relate to Subnets through VPCZoneIdentifier
        if let Some(vpc_zone_identifier) = entry.raw_properties.get("VPCZoneIdentifier") {
            if let Some(vpc_zone_str) = vpc_zone_identifier.as_str() {
                let subnet_ids: Vec<&str> = vpc_zone_str.split(',').map(|s| s.trim()).collect();
                for resource in all_resources {
                    if resource.resource_type == "AWS::EC2::Subnet" && subnet_ids.contains(&resource.resource_id.as_str()) {
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
        "AWS::AutoScaling::AutoScalingGroup"
    }
}

/// Normalizer for Auto Scaling Policies
pub struct AutoScalingPolicyNormalizer;

impl ResourceNormalizer for AutoScalingPolicyNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("PolicyName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-policy")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::AutoScaling::ScalingPolicy".to_string(),
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
        
        // Scaling Policies relate to their Auto Scaling Groups
        if let Some(asg_name) = entry.raw_properties.get("AutoScalingGroupName") {
            if let Some(asg_name_str) = asg_name.as_str() {
                for resource in all_resources {
                    if resource.resource_type == "AWS::AutoScaling::AutoScalingGroup" 
                        && resource.resource_id == asg_name_str {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::AttachedTo,
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                        });
                        break;
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::AutoScaling::ScalingPolicy"
    }
}