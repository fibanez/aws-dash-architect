use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for CodePipeline Pipelines
pub struct CodePipelinePipelineNormalizer;

impl ResourceNormalizer for CodePipelinePipelineNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let pipeline_name = raw_response
            .get("PipelineName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-pipeline")
            .to_string();

        let display_name = extract_display_name(&raw_response, &pipeline_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CodePipeline::Pipeline".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: pipeline_name,
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

        // Check for CodeBuild project relationships in pipeline stages
        if let Some(stages) = entry.raw_properties.get("Stages") {
            if let Some(stages_array) = stages.as_array() {
                for stage in stages_array {
                    if let Some(actions) = stage.get("Actions") {
                        if let Some(actions_array) = actions.as_array() {
                            for action in actions_array {
                                if let Some(action_type) =
                                    action.get("ActionTypeId").and_then(|v| v.as_str())
                                {
                                    // Check if this is a CodeBuild action
                                    if action_type.contains("Build:CodeBuild") {
                                        if let Some(config) = action.get("Configuration") {
                                            if let Some(project_name) =
                                                config.get("ProjectName").and_then(|v| v.as_str())
                                            {
                                                // Find matching CodeBuild project in all_resources
                                                for resource in all_resources {
                                                    if resource.resource_type
                                                        == "AWS::CodeBuild::Project"
                                                        && resource.resource_id == project_name
                                                    {
                                                        relationships.push(ResourceRelationship {
                                                            relationship_type:
                                                                RelationshipType::Uses,
                                                            target_resource_type:
                                                                "AWS::CodeBuild::Project"
                                                                    .to_string(),
                                                            target_resource_id: project_name
                                                                .to_string(),
                                                        });
                                                        break;
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
            }
        }

        // Check for CodeCommit repository relationships in source actions
        if let Some(stages) = entry.raw_properties.get("Stages") {
            if let Some(stages_array) = stages.as_array() {
                for stage in stages_array {
                    if let Some(actions) = stage.get("Actions") {
                        if let Some(actions_array) = actions.as_array() {
                            for action in actions_array {
                                if let Some(action_type) =
                                    action.get("ActionTypeId").and_then(|v| v.as_str())
                                {
                                    // Check if this is a CodeCommit source action
                                    if action_type.contains("Source:CodeCommit") {
                                        if let Some(config) = action.get("Configuration") {
                                            if let Some(repo_name) = config
                                                .get("RepositoryName")
                                                .and_then(|v| v.as_str())
                                            {
                                                // Find matching CodeCommit repository in all_resources
                                                for resource in all_resources {
                                                    if resource.resource_type
                                                        == "AWS::CodeCommit::Repository"
                                                        && resource.resource_id == repo_name
                                                    {
                                                        relationships.push(ResourceRelationship {
                                                            relationship_type:
                                                                RelationshipType::Uses,
                                                            target_resource_type:
                                                                "AWS::CodeCommit::Repository"
                                                                    .to_string(),
                                                            target_resource_id: repo_name
                                                                .to_string(),
                                                        });
                                                        break;
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
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CodePipeline::Pipeline"
    }
}
