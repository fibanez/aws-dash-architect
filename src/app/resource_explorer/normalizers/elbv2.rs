use super::{utils::*, AWSResourceClient, AsyncResourceNormalizer};
use crate::app::resource_explorer::state::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for Application/Network Load Balancers (ELBv2)
pub struct ELBv2LoadBalancerNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ELBv2LoadBalancerNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let lb_arn = raw_response
            .get("LoadBalancerArn")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-elbv2")
            .to_string();

        // Extract load balancer name from ARN for display
        let display_name = raw_response
            .get("LoadBalancerName")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                // Extract name from ARN if not directly available
                lb_arn.split('/').next_back().unwrap_or("unknown-elbv2")
            })
            .to_string();

        let status = raw_response
            .get("State")
            .and_then(|s| s.get("Code"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::ElasticLoadBalancingV2::LoadBalancer",
                &lb_arn,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::ElasticLoadBalancingV2::LoadBalancer {}: {}",
                    lb_arn,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::ElasticLoadBalancingV2::LoadBalancer".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: lb_arn.clone(),
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

        // Relationships to subnets (from availability zones)
        if let Some(azs) = entry
            .raw_properties
            .get("AvailabilityZones")
            .and_then(|az| az.as_array())
        {
            for az in azs {
                if let Some(subnet_id) = az.get("SubnetId").and_then(|s| s.as_str()) {
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

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ElasticLoadBalancingV2::LoadBalancer"
    }
}

/// Normalizer for Target Groups (ELBv2)
pub struct ELBv2TargetGroupNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ELBv2TargetGroupNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let tg_arn = raw_response
            .get("TargetGroupArn")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-target-group")
            .to_string();

        // Extract target group name from ARN for display
        let display_name = raw_response
            .get("TargetGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                // Extract name from ARN if not directly available
                tg_arn.split('/').nth(1).unwrap_or("unknown-target-group")
            })
            .to_string();

        // Target groups don't have a traditional status, use health check enabled as indicator
        let status = if raw_response
            .get("HealthCheckEnabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            Some("active".to_string())
        } else {
            Some("inactive".to_string())
        };

        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::ElasticLoadBalancingV2::TargetGroup",
                &tg_arn,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::ElasticLoadBalancingV2::TargetGroup {}: {}",
                    tg_arn,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::ElasticLoadBalancingV2::TargetGroup".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: tg_arn.clone(),
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

        // Relationships to load balancers
        if let Some(lb_arns) = entry
            .raw_properties
            .get("LoadBalancerArns")
            .and_then(|arns| arns.as_array())
        {
            for lb_arn in lb_arns {
                if let Some(arn) = lb_arn.as_str() {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::AttachedTo,
                        target_resource_id: arn.to_string(),
                        target_resource_type: "AWS::ElasticLoadBalancingV2::LoadBalancer"
                            .to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ElasticLoadBalancingV2::TargetGroup"
    }
}
