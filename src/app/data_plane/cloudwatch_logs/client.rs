//! CloudWatch Logs Client Wrapper
//!
//! Provides a simplified interface to AWS CloudWatch Logs with credential management.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Context, Result};
use aws_sdk_cloudwatchlogs as cloudwatchlogs;
use std::sync::Arc;

use crate::app::resource_explorer::credentials::CredentialCoordinator;

use super::types::{LogEvent, LogQueryResult, QueryOptions, QueryStatistics};

/// CloudWatch Logs client wrapper
#[derive(Clone)]
pub struct CloudWatchLogsClient {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CloudWatchLogsClient {
    /// Create a new CloudWatch Logs client wrapper
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// Query log events from a log group
    pub async fn query_log_events(
        &self,
        account_id: &str,
        region: &str,
        log_group_name: &str,
        options: QueryOptions,
    ) -> Result<LogQueryResult> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = cloudwatchlogs::Client::new(&aws_config);

        // Build the request
        let mut request = client.filter_log_events().log_group_name(log_group_name);

        // Apply query options
        if let Some(start_time) = options.start_time {
            request = request.start_time(start_time);
        }

        if let Some(end_time) = options.end_time {
            request = request.end_time(end_time);
        }

        if let Some(filter_pattern) = options.filter_pattern {
            request = request.filter_pattern(filter_pattern);
        }

        if let Some(limit) = options.limit {
            request = request.limit(limit);
        }

        // Add log stream names if specified
        for stream_name in &options.log_stream_names {
            request = request.log_stream_names(stream_name.clone());
        }

        // Execute the query
        let response = request.send().await.with_context(|| {
            format!(
                "Failed to query log events from log group: {}",
                log_group_name
            )
        })?;

        // Convert response to our types
        let mut events = Vec::new();

        if let Some(aws_events) = response.events {
            for event in aws_events {
                let log_event = LogEvent::with_ingestion_time(
                    event.timestamp.unwrap_or(0),
                    event.message.unwrap_or_default(),
                    event.ingestion_time.unwrap_or(0),
                    event.log_stream_name.unwrap_or_default(),
                );
                events.push(log_event);
            }
        }

        // Basic statistics - bytes scanned not available from filter_log_events API
        let statistics = QueryStatistics::new(
            0.0,                 // bytes_scanned not available
            events.len() as f64, // records matched
            events.len() as f64, // records scanned
        );

        Ok(LogQueryResult::with_statistics(
            events,
            response.next_token,
            statistics,
        ))
    }

    /// Get the latest log events from a log group
    pub async fn get_latest_log_events(
        &self,
        account_id: &str,
        region: &str,
        log_group_name: &str,
        limit: i32,
    ) -> Result<LogQueryResult> {
        let options = QueryOptions::new()
            .with_limit(limit)
            .with_start_from_head(false); // Most recent first

        self.query_log_events(account_id, region, log_group_name, options)
            .await
    }

    /// List log groups in a region
    pub async fn list_log_groups(
        &self,
        account_id: &str,
        region: &str,
        prefix: Option<String>,
    ) -> Result<Vec<String>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = cloudwatchlogs::Client::new(&aws_config);

        let mut request = client.describe_log_groups();

        if let Some(prefix) = prefix {
            request = request.log_group_name_prefix(prefix);
        }

        let response = request
            .send()
            .await
            .with_context(|| "Failed to list log groups")?;

        let mut log_groups = Vec::new();

        if let Some(groups) = response.log_groups {
            for group in groups {
                if let Some(name) = group.log_group_name {
                    log_groups.push(name);
                }
            }
        }

        Ok(log_groups)
    }

    /// List log streams in a log group
    pub async fn list_log_streams(
        &self,
        account_id: &str,
        region: &str,
        log_group_name: &str,
    ) -> Result<Vec<String>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = cloudwatchlogs::Client::new(&aws_config);

        let response = client
            .describe_log_streams()
            .log_group_name(log_group_name)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to list log streams for log group: {}",
                    log_group_name
                )
            })?;

        let mut log_streams = Vec::new();

        if let Some(streams) = response.log_streams {
            for stream in streams {
                if let Some(name) = stream.log_stream_name {
                    log_streams.push(name);
                }
            }
        }

        Ok(log_streams)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // This is a basic smoke test - actual integration tests will need real AWS credentials
        // For now, we just verify the client can be created
        // Real tests will be in integration tests
    }
}
