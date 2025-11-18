use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for CodeBuild Projects
pub struct CodeBuildProjectNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CodeBuildProjectNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let project_name = raw_response
            .get("ProjectName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-project")
            .to_string();

        let display_name = extract_display_name(&raw_response, &project_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client

            .fetch_tags_for_resource("AWS::CodeBuild::Project", &project_name, account, region)

            .await

            .unwrap_or_else(|e| {

                tracing::warn!("Failed to fetch tags for AWS::CodeBuild::Project {}: {}", project_name, e);

                Vec::new()

            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CodeBuild::Project".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: project_name,
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
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Check for CodeCommit repository relationships in source configuration
        if let Some(source) = entry.raw_properties.get("Source") {
            if let Some(source_type) = source.get("Type").and_then(|v| v.as_str()) {
                if source_type == "CODECOMMIT" {
                    if let Some(location) = source.get("Location").and_then(|v| v.as_str()) {
                        // CodeCommit location format: https://git-codecommit.region.amazonaws.com/v1/repos/repo-name
                        if let Some(repo_name) = location.split('/').next_back() {
                            // Find matching CodeCommit repository in all_resources
                            for resource in all_resources {
                                if resource.resource_type == "AWS::CodeCommit::Repository"
                                    && resource.resource_id == repo_name
                                {
                                    relationships.push(ResourceRelationship {
                                        relationship_type: RelationshipType::Uses,
                                        target_resource_type: "AWS::CodeCommit::Repository"
                                            .to_string(),
                                        target_resource_id: repo_name.to_string(),
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for S3 bucket relationships in artifacts configuration
        if let Some(artifacts) = entry.raw_properties.get("Artifacts") {
            if let Some(artifacts_type) = artifacts.get("Type").and_then(|v| v.as_str()) {
                if artifacts_type == "S3" {
                    if let Some(location) = artifacts.get("Location").and_then(|v| v.as_str()) {
                        // S3 location is the bucket name for CodeBuild artifacts
                        // Find matching S3 bucket in all_resources
                        for resource in all_resources {
                            if resource.resource_type == "AWS::S3::Bucket"
                                && resource.resource_id == location
                            {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_type: "AWS::S3::Bucket".to_string(),
                                    target_resource_id: location.to_string(),
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CodeBuild::Project"
    }
}

