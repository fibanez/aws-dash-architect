use super::utils::*;
use super::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Normalizer for CloudWatch Alarms
pub struct CloudWatchAlarmNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CloudWatchAlarmNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let alarm_name = raw_response
            .get("AlarmName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-alarm")
            .to_string();

        let display_name = extract_display_name(&raw_response, &alarm_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource("AWS::CloudWatch::Alarm", &alarm_name, account, region)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::CloudWatch::Alarm {}: {}",
                    alarm_name,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CloudWatch::Alarm".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: alarm_name,
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
        // CloudWatch alarms can monitor other AWS resources
        // but we'd need to parse the metric configuration for relationships
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CloudWatch::Alarm"
    }
}

/// Normalizer for CloudWatch Dashboards
pub struct CloudWatchDashboardNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CloudWatchDashboardNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let dashboard_name = raw_response
            .get("DashboardName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-dashboard")
            .to_string();

        let display_name = extract_display_name(&raw_response, &dashboard_name);
        let status = extract_status(&raw_response);
        // Fetch tags asynchronously from AWS API with caching

        let tags = aws_client
            .fetch_tags_for_resource(
                "AWS::CloudWatch::Dashboard",
                &dashboard_name,
                account,
                region,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch tags for AWS::CloudWatch::Dashboard {}: {}",
                    dashboard_name,
                    e
                );

                Vec::new()
            });
        let properties = create_normalized_properties(&raw_response);

        Ok(ResourceEntry {
            resource_type: "AWS::CloudWatch::Dashboard".to_string(),
            account_id: account.to_string(),
            region: region.to_string(),
            resource_id: dashboard_name,
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
        // CloudWatch dashboards can display metrics from various AWS resources
        // but we'd need to parse the dashboard body JSON for relationships
        Vec::new()
    }

    fn resource_type(&self) -> &'static str {
        "AWS::CloudWatch::Dashboard"
    }
}

async fn normalize_cloudwatch_simple_resource(
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

/// Normalizer for CloudWatch Composite Alarms
pub struct CloudWatchCompositeAlarmNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CloudWatchCompositeAlarmNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let alarm_name = raw_response
            .get("AlarmName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown-composite-alarm")
            .to_string();

        normalize_cloudwatch_simple_resource(
            "AWS::CloudWatch::CompositeAlarm",
            alarm_name,
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
        "AWS::CloudWatch::CompositeAlarm"
    }
}

/// Normalizer for CloudWatch Metrics
pub struct CloudWatchMetricNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CloudWatchMetricNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("MetricId")
            .and_then(|v| v.as_str())
            .or_else(|| raw_response.get("Name").and_then(|v| v.as_str()))
            .unwrap_or("unknown-metric")
            .to_string();

        normalize_cloudwatch_simple_resource(
            "AWS::CloudWatch::Metric",
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
        "AWS::CloudWatch::Metric"
    }
}

/// Normalizer for CloudWatch Insight Rules
pub struct CloudWatchInsightRuleNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CloudWatchInsightRuleNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("RuleName")
            .and_then(|v| v.as_str())
            .or_else(|| raw_response.get("Name").and_then(|v| v.as_str()))
            .unwrap_or("unknown-insight-rule")
            .to_string();

        normalize_cloudwatch_simple_resource(
            "AWS::CloudWatch::InsightRule",
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
        "AWS::CloudWatch::InsightRule"
    }
}

/// Normalizer for CloudWatch Anomaly Detectors
pub struct CloudWatchAnomalyDetectorNormalizer;

#[async_trait]
impl AsyncResourceNormalizer for CloudWatchAnomalyDetectorNormalizer {
    async fn normalize(
        &self,
        raw_response: serde_json::Value,
        account: &str,
        region: &str,
        query_timestamp: DateTime<Utc>,
        aws_client: &AWSResourceClient,
    ) -> Result<ResourceEntry> {
        let resource_id = raw_response
            .get("DetectorId")
            .and_then(|v| v.as_str())
            .or_else(|| raw_response.get("Name").and_then(|v| v.as_str()))
            .unwrap_or("unknown-anomaly-detector")
            .to_string();

        normalize_cloudwatch_simple_resource(
            "AWS::CloudWatch::AnomalyDetector",
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
        "AWS::CloudWatch::AnomalyDetector"
    }
}
