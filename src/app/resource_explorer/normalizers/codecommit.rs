use super::utils::*;
use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for CodeCommit Repositories
pub struct CodeCommitRepositoryNormalizer;

impl ResourceNormalizer for CodeCommitRepositoryNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let repository_name = raw_response
            .get("RepositoryName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-repository")
            .to_string();

        let display_name = extract_display_name(&raw_response, &repository_name);
        let status = extract_status(&raw_response);
        let tags = extract_tags(&raw_response);
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CodeCommit::Repository".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: repository_name,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // CodeCommit repositories are typically source repositories
        // Relationships would be determined by CodePipeline and CodeBuild projects
        // that reference this repository, which is handled in their respective normalizers
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CodeCommit::Repository"
    }
}
