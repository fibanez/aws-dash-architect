use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for CloudWatch Logs Resources
pub struct LogsResourceNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for LogsResourceNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("LogGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-log-group")
            .to_string();

        let display_name = extract_display_name(&raw_response, &resource_id);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::Logs::LogGroup", &resource_id, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::Logs::LogGroup {}: {}",
                    resource_id,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::Logs::LogGroup".to_string(),
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
        __entry: &ResourceEntry,
        __all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        // Log groups can have relationships with Lambda functions, API Gateway, etc.
        // but these would need to be discovered through other means
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Logs::LogGroup"
    }
}

async fn normalize_logs_simple_resource(
    resource_type: &str,
    resource_id: String,
    raw_response: serde_json::Value,
    account: &str,
    region: &str,
    query_timestamp: DateTime<Utc>,
    aws_client: &AWSResourceClient,
) -> Result<ResourceEntry> {
    let display_name = extract_display_name(&raw_response, &resource_id);
    let status = extract_status(&raw_response);

    let tags = aws_client
        .fetch_tags_for_resource(resource_type, &resource_id, account, region)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to fetch tags for {} {}: {}",
                resource_type,
                resource_id,
                e
            );

            Vec::new()
        });

    let properties = create_normalized_properties(&raw_response);

    Ok(ResourceEntry {
        resource_type: resource_type.to_string(),
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
        parent_resource_id: None,
        parent_resource_type: None,
        is_child_resource: false,
        account_color: assign_account_color(account),
        region_color: assign_region_color(region),
        query_timestamp,
    })
}

/// Normalizer for CloudWatch Log Streams
pub struct LogsLogStreamNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for LogsLogStreamNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let log_group = raw_response
            .get("LogGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-log-group");
        let stream_name = raw_response
            .get("LogStreamName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-log-stream");
        let resource_id = format!("{}:{}", log_group, stream_name);

        normalize_logs_simple_resource(
            "AWS::Logs::LogStream",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Logs::LogStream"
    }
}

/// Normalizer for CloudWatch Logs Metric Filters
pub struct LogsMetricFilterNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for LogsMetricFilterNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let log_group = raw_response
            .get("LogGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-log-group");
        let filter_name = raw_response
            .get("FilterName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-metric-filter");
        let resource_id = format!("{}:{}", log_group, filter_name);

        normalize_logs_simple_resource(
            "AWS::Logs::MetricFilter",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Logs::MetricFilter"
    }
}

/// Normalizer for CloudWatch Logs Subscription Filters
pub struct LogsSubscriptionFilterNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for LogsSubscriptionFilterNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let log_group = raw_response
            .get("LogGroupName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-log-group");
        let filter_name = raw_response
            .get("FilterName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-subscription-filter");
        let resource_id = format!("{}:{}", log_group, filter_name);

        normalize_logs_simple_resource(
            "AWS::Logs::SubscriptionFilter",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Logs::SubscriptionFilter"
    }
}

/// Normalizer for CloudWatch Logs Resource Policies
pub struct LogsResourcePolicyNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for LogsResourcePolicyNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("PolicyName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-resource-policy")
            .to_string();

        normalize_logs_simple_resource(
            "AWS::Logs::ResourcePolicy",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Logs::ResourcePolicy"
    }
}

/// Normalizer for CloudWatch Logs Query Definitions
pub struct LogsQueryDefinitionNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for LogsQueryDefinitionNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("QueryDefinitionId")
            .and_then(|v| v.as_str())
            .or_else(|| raw_response.get("Name").and_then(|v| v.as_str()))
            .unwrap_or("unknown-query-definition")
            .to_string();

        normalize_logs_simple_resource(
            "AWS::Logs::QueryDefinition",
            resource_id,
            raw_response,
            account,
            region,
            query_timestamp,
            aws_client,
        )
        .await
    }

    fn extract_relationships(
        &self,
        _entry: &ResourceEntry,
        _all_resources: &[ResourceEntry],
    ) -> Vec<ResourceRelationship> {
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::Logs::QueryDefinition"
    }
}
