use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for ECS Clusters
pub struct ECSClusterNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ECSClusterNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let cluster_name = raw_response
            .get("ClusterName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-cluster")
            .to_string();

        let display_name = extract_display_name(&raw_response, &cluster_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::ECS::Cluster", &cluster_name, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::ECS::Cluster {}: {}",
                    cluster_name,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::ECS::Cluster".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: cluster_name,
            display_name,
            status,
            properties: raw_response,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // ECS clusters can have relationships with services, tasks, etc.
        // but we'd need to list services/tasks for that
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ECS::Cluster"
    }
}

/// Normalizer for ECS Services
pub struct ECSServiceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ECSServiceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let service_name = raw_response
            .get("ServiceName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-service")
            .to_string();

        let display_name = extract_display_name(&raw_response, &service_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::ECS::Service", &service_name, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::ECS::Service {}: {}",
                    service_name,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::ECS::Service".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: service_name,
            display_name,
            status,
            properties: raw_response,
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

        // Map to cluster
        if let Some(cluster_arn) = entry
            .properties
            .get("ClusterArn")
            .and_then(|v| v.as_str())
        {
            let cluster_name = cluster_arn.split('/').next_back().unwrap_or(cluster_arn);
            for resource in all_resources {
                if resource.resource_type == "AWS::ECS::Cluster"
                    && resource.resource_id == cluster_name
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: cluster_name.to_string(),
                        target_resource_type: "AWS::ECS::Cluster".to_string(),
                    });
                }
            }
        }

        // Map to task definition
        if let Some(task_def_arn) = entry
            .properties
            .get("TaskDefinition")
            .and_then(|v| v.as_str())
        {
            let task_def_family = task_def_arn
                .split('/')
                .next_back()
                .and_then(|s| s.split(':').next())
                .unwrap_or(task_def_arn);
            for resource in all_resources {
                if resource.resource_type == "AWS::ECS::TaskDefinition"
                    && resource.resource_id == task_def_family
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: task_def_family.to_string(),
                        target_resource_type: "AWS::ECS::TaskDefinition".to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ECS::Service"
    }
}

/// Normalizer for ECS Tasks
pub struct ECSTaskNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ECSTaskNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let task_arn = raw_response
            .get("TaskArn")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-task")
            .to_string();

        // Extract task ID from ARN for resource_id
        let task_id = task_arn
            .split('/')
            .next_back()
            .unwrap_or(&task_arn)
            .to_string();

        let display_name = raw_response
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or(&task_id)
            .to_string();

        let status = raw_response
            .get("LastStatus")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::ECS::Task", &task_arn, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::ECS::Task {}: {}",
                    task_arn,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::ECS::Task".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: task_id,
            display_name,
            status,
            properties: raw_response,
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

        // Map to cluster
        if let Some(cluster_arn) = entry
            .properties
            .get("ClusterArn")
            .and_then(|v| v.as_str())
        {
            let cluster_name = cluster_arn.split('/').next_back().unwrap_or(cluster_arn);
            for resource in all_resources {
                if resource.resource_type == "AWS::ECS::Cluster"
                    && resource.resource_id == cluster_name
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::MemberOf,
                        target_resource_id: cluster_name.to_string(),
                        target_resource_type: "AWS::ECS::Cluster".to_string(),
                    });
                }
            }
        }

        // Map to task definition
        if let Some(task_def_arn) = entry
            .properties
            .get("TaskDefinitionArn")
            .and_then(|v| v.as_str())
        {
            let task_def_family = task_def_arn
                .split('/')
                .next_back()
                .and_then(|s| s.split(':').next())
                .unwrap_or(task_def_arn);
            for resource in all_resources {
                if resource.resource_type == "AWS::ECS::TaskDefinition"
                    && resource.resource_id == task_def_family
                {
                    relationships.push(ResourceRelationship {
                        relationship_type: RelationshipType::Uses,
                        target_resource_id: task_def_family.to_string(),
                        target_resource_type: "AWS::ECS::TaskDefinition".to_string(),
                    });
                }
            }
        }

        relationships
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ECS::Task"
    }
}

/// Normalizer for ECS Task Definitions
pub struct ECSTaskDefinitionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for ECSTaskDefinitionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let family = raw_response
            .get("Family")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-task-definition")
            .to_string();

        let display_name = extract_display_name(&raw_response, &family);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::ECS::TaskDefinition", &family, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::ECS::TaskDefinition {}: {}",
                    family,
                    e
                );

                Vec::new()
            });

        Ok(ResourceEntry {
            resource_type: "AWS::ECS::TaskDefinition".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: family,
            display_name,
            status,
            properties: raw_response,
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
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Task definitions are used by services and tasks but don't depend on other resources
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::ECS::TaskDefinition"
    }
}
