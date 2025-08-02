use super::super::state::{RelationshipType, ResourceEntry, ResourceRelationship};
use super::{utils, ResourceNormalizer};
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct BatchJobQueueNormalizer;
pub struct BatchComputeEnvironmentNormalizer;

impl ResourceNormalizer for BatchJobQueueNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("JobQueueName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing JobQueueName"))?
            .to_string();

        let display_name = raw_response
            .get("JobQueueName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = utils::extract_status(&raw_response);
        let tags = utils::extract_tags(&raw_response);
        let normalized_properties = utils::create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Batch::JobQueue".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: normalized_properties,
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            account_color: egui::Color32::PLACEHOLDER,
            region_color: egui::Color32::PLACEHOLDER,
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Extract compute environment relationships
        if let Some(ce_order) = entry.raw_properties.get("ComputeEnvironmentOrder") {
            if let Some(ce_array) = ce_order.as_array() {
                for ce in ce_array {
                    if let Some(ce_name) = ce.get("ComputeEnvironment").and_then(|v| v.as_str()) {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: ce_name.to_string(),
                            target_resource_type: "AWS::Batch::ComputeEnvironment".to_string(),
                        });
                    }
                }
            }
        }

        // Extract scheduling policy relationship
        if let Some(scheduling_policy_arn) = entry
            .raw_properties
            .get("SchedulingPolicyArn")
            .and_then(|v| v.as_str())
        {
            relationships.push(ResourceRelationship {
                relationship_type: RelationshipType::Uses,
                target_resource_id: scheduling_policy_arn.to_string(),
                target_resource_type: "AWS::Batch::SchedulingPolicy".to_string(),
            });
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Batch::JobQueue"
    }
}

impl ResourceNormalizer for BatchComputeEnvironmentNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ComputeEnvironmentName")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing ComputeEnvironmentName"))?
            .to_string();

        let display_name = raw_response
            .get("ComputeEnvironmentName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = utils::extract_status(&raw_response);
        let tags = utils::extract_tags(&raw_response);
        let normalized_properties = utils::create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Batch::ComputeEnvironment".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: normalized_properties,
            raw_properties: raw_response.clone(),
            detailed_properties: Some(raw_response),
            detailed_timestamp: Some(query_timestamp),
            tags,
            relationships: Vec::new(),
            account_color: egui::Color32::PLACEHOLDER,
            region_color: egui::Color32::PLACEHOLDER,
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Extract ECS cluster relationship
        if let Some(ecs_cluster_arn) = entry
            .raw_properties
            .get("EcsClusterArn")
            .and_then(|v| v.as_str())
        {
            relationships.push(ResourceRelationship {
                relationship_type: RelationshipType::Uses,
                target_resource_id: ecs_cluster_arn.to_string(),
                target_resource_type: "AWS::ECS::Cluster".to_string(),
            });
        }

        // Extract service role relationship
        if let Some(service_role) = entry
            .raw_properties
            .get("ServiceRole")
            .and_then(|v| v.as_str())
        {
            relationships.push(ResourceRelationship {
                relationship_type: RelationshipType::Uses,
                target_resource_id: service_role.to_string(),
                target_resource_type: "AWS::IAM::Role".to_string(),
            });
        }

        // Extract compute resources relationships
        if let Some(compute_resources) = entry.raw_properties.get("ComputeResources") {
            // Instance role
            if let Some(instance_role) = compute_resources
                .get("InstanceRole")
                .and_then(|v| v.as_str())
            {
                relationships.push(ResourceRelationship {
                    relationship_type: RelationshipType::Uses,
                    target_resource_id: instance_role.to_string(),
                    target_resource_type: "AWS::IAM::InstanceProfile".to_string(),
                });
            }

            // Subnets
            if let Some(subnets) = compute_resources.get("Subnets").and_then(|v| v.as_array()) {
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

            // Security Groups
            if let Some(security_groups) = compute_resources
                .get("SecurityGroupIds")
                .and_then(|v| v.as_array())
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
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Batch::ComputeEnvironment"
    }
}
