use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for ECS Fargate Service
pub struct ECSFargateServiceNormalizer;

impl ResourceNormalizer for ECSFargateServiceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ServiceArn")
            .and_then(|v| v.as_str())
            .and_then(|arn| arn.split('/').nth_back(0))
            .unwrap_or("unknown-fargate-service")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::ECS::FargateService".to_string(),
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
        
        // Fargate services relate to clusters
        if let Some(cluster_arn) = entry.raw_properties
            .get("ClusterArn")
            .and_then(|v| v.as_str()) {
            
            for resource in all_resources {
                if resource.resource_type == "AWS::ECS::Cluster" {
                    if let Some(resource_arn) = resource.raw_properties
                        .get("ClusterArn")
                        .and_then(|v| v.as_str()) {
                        if resource_arn == cluster_arn {
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
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ECS::FargateService"
    }
}

/// Normalizer for ECS Fargate Task
pub struct ECSFargateTaskNormalizer;

impl ResourceNormalizer for ECSFargateTaskNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("TaskArn")
            .and_then(|v| v.as_str())
            .and_then(|arn| arn.split('/').nth_back(0))
            .unwrap_or("unknown-fargate-task")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::ECS::FargateTask".to_string(),
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
        
        // Fargate tasks relate to clusters and services
        if let Some(cluster_arn) = entry.raw_properties
            .get("ClusterArn")
            .and_then(|v| v.as_str()) {
            
            for resource in all_resources {
                match resource.resource_type.as_str() {
                    "AWS::ECS::Cluster" => {
                        if let Some(resource_arn) = resource.raw_properties
                            .get("ClusterArn")
                            .and_then(|v| v.as_str()) {
                            if resource_arn == cluster_arn {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                    "AWS::ECS::Service" | "AWS::ECS::FargateService" => {
                        if let Some(service_arn) = entry.raw_properties
                            .get("ServiceArn")
                            .and_then(|v| v.as_str()) {
                            if let Some(resource_arn) = resource.raw_properties
                                .get("ServiceArn")
                                .and_then(|v| v.as_str()) {
                                if resource_arn == service_arn {
                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::Uses,
                                        target_resource_id: resource.resource_id.clone(),
                                        target_resource_type: resource.resource_type.clone(),
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ECS::FargateTask"
    }
}

/// Normalizer for EKS Fargate Profile
pub struct EKSFargateProfileNormalizer;

impl ResourceNormalizer for EKSFargateProfileNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-fargate-profile")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::EKS::FargateProfile".to_string(),
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
        
        // Fargate profiles relate to EKS clusters
        if let Some(cluster_name) = entry.raw_properties
            .get("ClusterName")
            .and_then(|v| v.as_str()) {
            
            for resource in all_resources {
                if resource.resource_type == "AWS::EKS::Cluster" && (resource.display_name == cluster_name || resource.resource_id == cluster_name) {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                    break;
                }
            }
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::EKS::FargateProfile"
    }
}