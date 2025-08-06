#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::app::resource_explorer::state::*;
use super::ResourceNormalizer;


pub struct DataBrewJobNormalizer;

impl ResourceNormalizer for DataBrewJobNormalizer {
    fn normalize(
        &self,
        raw_response: Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let binding = raw_response.clone();
        let job_obj = binding.as_object().ok_or_else(|| anyhow::anyhow!("Job is not an object"))?;
        
        let name = job_obj.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let _arn = format!("arn:aws:databrew:{}:{}:job/{}", region, account, name);

        let resource_entry = ResourceEntry {
            resource_type: "AWS::DataBrew::Job".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: name.clone(),
            display_name: name,
            status: job_obj.get("state").and_then(|v| v.as_str()).map(|s| s.to_string()),
            properties: raw_response.clone(),
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags: extract_tags_from_job(job_obj),
            relationships: Vec::new(),
            account_color: egui::Color32::from_rgb(100, 150, 200),
            region_color: egui::Color32::from_rgb(200, 150, 100),
            query_timestamp,
        };

        Ok(resource_entry)
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        if let Some(job_obj) = entry.properties.as_object() {
            // Dataset relationship
            if let Some(dataset_name) = job_obj.get("dataset_name").and_then(|v| v.as_str()) {
                relationships.push(ResourceRelationship {
                    relationship_type: RelationshipType::Uses,
                    target_resource_id: dataset_name.to_string(),
                    target_resource_type: "AWS::DataBrew::Dataset".to_string(),
                });
            }

            // Role relationship
            if let Some(role_arn) = job_obj.get("role_arn").and_then(|v| v.as_str()) {
                // Extract role name from ARN
                if let Some(role_name) = role_arn.split('/').next_back() {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: role_name.to_string(),
                        target_resource_type: "AWS::IAM::Role".to_string(),
                    });
                }
            }

            // S3 output relationships
            if let Some(outputs) = job_obj.get("outputs").and_then(|v| v.as_array()) {
                for output in outputs {
                    if let Some(output_obj) = output.as_object() {
                        if let Some(location) = output_obj.get("location").and_then(|v| v.as_object()) {
                            if let Some(bucket) = location.get("bucket").and_then(|v| v.as_str()) {
                                relationships.push(ResourceRelationship {
                                    relationship_type: RelationshipType::Uses,
                                    target_resource_id: bucket.to_string(),
                                    target_resource_type: "AWS::S3::Bucket".to_string(),
                                });
                            }
                        }
                    }
                }
            }

            // KMS key relationship
            if let Some(encryption_key_arn) = job_obj.get("encryption_key_arn").and_then(|v| v.as_str()) {
                // Extract key ID from ARN
                if let Some(key_id) = encryption_key_arn.split('/').next_back() {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: key_id.to_string(),
                        target_resource_type: "AWS::KMS::Key".to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::DataBrew::Job"
    }
}

pub struct DataBrewDatasetNormalizer;

impl ResourceNormalizer for DataBrewDatasetNormalizer {
    fn normalize(
        &self,
        raw_response: Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let binding = raw_response.clone();
        let dataset_obj = binding.as_object().ok_or_else(|| anyhow::anyhow!("Dataset is not an object"))?;
        
        let name = dataset_obj.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let _arn = format!("arn:aws:databrew:{}:{}:dataset/{}", region, account, name);

        let resource_entry = ResourceEntry {
            resource_type: "AWS::DataBrew::Dataset".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: name.clone(),
            display_name: name,
            status: None,
            properties: raw_response.clone(),
            raw_properties: raw_response,
            detailed_properties: None,
            detailed_timestamp: None,
            tags: extract_tags_from_dataset(dataset_obj),
            relationships: Vec::new(),
            account_color: egui::Color32::from_rgb(100, 150, 200),
            region_color: egui::Color32::from_rgb(200, 150, 100),
            query_timestamp,
        };

        Ok(resource_entry)
    }

    fn extract_relationships(
        &self,
        entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        let mut relationships = Vec::new();

        if let Some(dataset_obj) = entry.properties.as_object() {
            // S3 source relationship
            if let Some(input) = dataset_obj.get("input").and_then(|v| v.as_object()) {
                if let Some(s3_input_definition) = input.get("s3_input_definition").and_then(|v| v.as_object()) {
                    if let Some(bucket) = s3_input_definition.get("bucket").and_then(|v| v.as_str()) {
                        relationships.push(ResourceRelationship {
                            relationship_type: RelationshipType::Uses,
                            target_resource_id: bucket.to_string(),
                            target_resource_type: "AWS::S3::Bucket".to_string(),
                        });
                    }
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::DataBrew::Dataset"
    }
}

fn extract_tags_from_job(job_obj: &Map<String, Value>) -> Vec<ResourceTag> {
    let mut tags = Vec::new();
    if let Some(tag_map) = job_obj.get("tags").and_then(|v| v.as_object()) {
        for (key, value) in tag_map {
            if let Some(tag_value) = value.as_str() {
                tags.push(ResourceTag {
                    key: key.clone(),
                    value: tag_value.to_string(),
                });
            }
        }
    }
    tags
}

fn extract_tags_from_dataset(dataset_obj: &Map<String, Value>) -> Vec<ResourceTag> {
    let mut tags = Vec::new();
    if let Some(tag_map) = dataset_obj.get("tags").and_then(|v| v.as_object()) {
        for (key, value) in tag_map {
            if let Some(tag_value) = value.as_str() {
                tags.push(ResourceTag {
                    key: key.clone(),
                    value: tag_value.to_string(),
                });
            }
        }
    }
    tags
}