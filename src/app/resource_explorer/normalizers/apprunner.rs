use super::*;
use super::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for AWS App Runner Service Resources
pub struct AppRunnerResourceNormalizer;

impl ResourceNormalizer for AppRunnerResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("ServiceArn"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-apprunner-service")
            .to_string();

        let display_name = raw_response
            .get("ServiceName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::AppRunner::Service".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status: Some(status),
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
        
        // App Runner services relate to various AWS resources
        for resource in all_resources {
            match resource.resource_type.as_str() {
                "AWS::IAM::Role" => {
                    // App Runner services use IAM roles for instance permissions
                    if let Some(instance_config) = entry.raw_properties.get("InstanceConfiguration") {
                        if let Some(instance_role_arn) = instance_config.get("InstanceRoleArn").and_then(|v| v.as_str()) {
                            if instance_role_arn.contains(&resource.resource_id) {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                });
                            }
                        }
                    }
                }
                "AWS::ECR::Repository" => {
                    // App Runner services can use ECR repositories as image sources
                    if let Some(source_config) = entry.raw_properties.get("SourceConfiguration") {
                        if let Some(image_repo) = source_config.get("ImageRepository") {
                            if let Some(image_identifier) = image_repo.get("ImageIdentifier").and_then(|v| v.as_str()) {
                                if image_identifier.contains("ecr") && image_identifier.contains(&resource.resource_id) {
                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::Uses,
                                        target_resource_id: resource.resource_id.clone(),
                                        target_resource_type: resource.resource_type.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
                "AWS::CodeCommit::Repository" => {
                    // App Runner services can use CodeCommit repositories as code sources
                    if let Some(source_config) = entry.raw_properties.get("SourceConfiguration") {
                        if let Some(code_repo) = source_config.get("CodeRepository") {
                            if let Some(repo_url) = code_repo.get("RepositoryUrl").and_then(|v| v.as_str()) {
                                if repo_url.contains("codecommit") && repo_url.contains(&resource.resource_id) {
                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::Uses,
                                        target_resource_id: resource.resource_id.clone(),
                                        target_resource_type: resource.resource_type.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
                "AWS::AppRunner::Connection" => {
                    // App Runner services use connections for external code repositories
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::CloudWatch::LogGroup" => {
                    // App Runner services send logs to CloudWatch
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::CertificateManager::Certificate" => {
                    // App Runner services can use custom domains with SSL certificates
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                "AWS::EC2::VPC" => {
                    // App Runner services can connect to VPCs through VPC connectors
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: resource.resource_id.clone(),
                        target_resource_type: resource.resource_type.clone(),
                    });
                }
                _ => {}
            }
        }
        
        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::AppRunner::Service"
    }
}

/// Normalizer for AWS App Runner Connection Resources
pub struct AppRunnerConnectionResourceNormalizer;

impl ResourceNormalizer for AppRunnerConnectionResourceNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ResourceId")
            .or_else(|| raw_response.get("ConnectionArn"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-apprunner-connection")
            .to_string();

        let display_name = raw_response
            .get("ConnectionName")
            .and_then(|v| v.as_str())
            .unwrap_or(&resource_id)
            .to_string();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::AppRunner::Connection".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status: Some(status),
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // App Runner connections are primarily used by App Runner services
        // The relationship is established from the service side
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::AppRunner::Connection"
    }
}