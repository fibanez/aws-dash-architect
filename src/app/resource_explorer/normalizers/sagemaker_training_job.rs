use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for SageMaker Training Jobs
pub struct SageMakerTrainingJobNormalizer;

impl ResourceNormalizer for SageMakerTrainingJobNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("TrainingJobName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-training-job")
            .to_string();

        let display_name = resource_id.clone();

        let status = raw_response
            .get("TrainingJobStatus")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract basic properties for normalized view
        let mut properties = serde_json::Map::new();

        if let Some(status) = &status {
            properties.insert(
                "status".to_string(),
                serde_json::Value::String(status.clone()),
            );
        }

        if let Some(creation_time) = raw_response.get("CreationTime") {
            properties.insert("creation_time".to_string(), creation_time.clone());
        }

        if let Some(training_end_time) = raw_response.get("TrainingEndTime") {
            properties.insert("training_end_time".to_string(), training_end_time.clone());
        }

        if let Some(arn) = raw_response.get("TrainingJobArn") {
            properties.insert("arn".to_string(), arn.clone());
        }

        let account_color = assign_account_color(account);
        let region_color = assign_region_color(region);

        Ok(ResourceEntry {
            resource_type: "AWS::SageMaker::TrainingJob".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id,
            display_name,
            status,
            properties: serde_json::Value::Object(properties),
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags: Vec::new(),
            relationships: Vec::new(), // Will be populated by extract_relationships
            account_color,
            region_color,
            query_timestamp,
        })
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        // Map to IAM role if present
        if let Some(role_arn) = entry.raw_properties.get("RoleArn").and_then(|v| v.as_str()) {
            // Extract role name from ARN: arn:aws:iam::account:role/role-name
            if let Some(role_name) = role_arn.split('/').nth(1) {
                for resource in all_resources {
                    if resource.resource_type == "AWS::IAM::Role"
                        && resource.resource_id == role_name
                    {
                        relationships.push(ResourceRelationship {
                            target_resource_id: resource.resource_id.clone(),
                            target_resource_type: resource.resource_type.clone(),
                            relationship_type: RelationshipType::Uses,
                        });
                        break;
                    }
                }
            }
        }

        // Map to S3 buckets for model artifacts
        if let Some(model_artifacts) = entry
            .raw_properties
            .get("ModelArtifacts")
            .and_then(|v| v.as_str())
        {
            // Extract bucket name from S3 URI: s3://bucket-name/path/to/artifacts
            if let Some(stripped) = model_artifacts.strip_prefix("s3://") {
                if let Some(bucket_name) = stripped.split('/').next() {
                    for resource in all_resources {
                        if resource.resource_type == "AWS::S3::Bucket"
                            && resource.resource_id == bucket_name
                        {
                            relationships.push(ResourceRelationship {
                                target_resource_id: resource.resource_id.clone(),
                                target_resource_type: resource.resource_type.clone(),
                                relationship_type: RelationshipType::Uses,
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
        "AWS::SageMaker::TrainingJob"
    }
}
