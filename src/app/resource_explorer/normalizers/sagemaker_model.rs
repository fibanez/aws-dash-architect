use super::*;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Normalizer for SageMaker Models
pub struct SageMakerModelNormalizer;

impl ResourceNormalizer for SageMakerModelNormalizer {
    fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("ModelName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-model")
            .to_string();

        let display_name = resource_id.clone();

        let status = raw_response
            .get("Status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some("Available".to_string()));

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

        if let Some(arn) = raw_response.get("ModelArn") {
            properties.insert("arn".to_string(), arn.clone());
        }

        if let Some(execution_role_arn) = raw_response.get("ExecutionRoleArn") {
            properties.insert("execution_role_arn".to_string(), execution_role_arn.clone());
        }

        let account_color = assign_account_color(account);
        let region_color = assign_region_color(region);

        Ok(ResourceEntry {
            resource_type: "AWS::SageMaker::Model".to_string(),
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
        if let Some(role_arn) = entry
            .raw_properties
            .get("ExecutionRoleArn")
            .and_then(|v| v.as_str())
        {
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

        // Map to S3 buckets for model data
        if let Some(primary_container) = entry.raw_properties.get("PrimaryContainer") {
            if let Some(model_data_url) = primary_container
                .get("ModelDataUrl")
                .and_then(|v| v.as_str())
            {
                // Extract bucket name from S3 URI: s3://bucket-name/path/to/model
                if let Some(stripped) = model_data_url.strip_prefix("s3://") {
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
        }

        // Map to VPC resources if VPC config is present
        if let Some(vpc_config) = entry.raw_properties.get("VpcConfig") {
            // Map to subnets
            if let Some(subnets) = vpc_config.get("Subnets").and_then(|v| v.as_array()) {
                for subnet_value in subnets {
                    if let Some(subnet_id) = subnet_value.as_str() {
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::Subnet"
                                && resource.resource_id == subnet_id
                            {
                                relationships.push(ResourceRelationship {
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                    relationship_type: RelationshipType::DeployedIn,
                                });
                                break;
                            }
                        }
                    }
                }
            }

            // Map to security groups
            if let Some(security_groups) = vpc_config
                .get("SecurityGroupIds")
                .and_then(|v| v.as_array())
            {
                for sg_value in security_groups {
                    if let Some(sg_id) = sg_value.as_str() {
                        for resource in all_resources {
                            if resource.resource_type == "AWS::EC2::SecurityGroup"
                                && resource.resource_id == sg_id
                            {
                                relationships.push(ResourceRelationship {
                                    target_resource_id: resource.resource_id.clone(),
                                    target_resource_type: resource.resource_type.clone(),
                                    relationship_type: RelationshipType::ProtectedBy,
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
        "AWS::SageMaker::Model"
    }
}
