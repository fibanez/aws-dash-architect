use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudwatchlogs as logs;
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub struct LogsService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl LogsService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CloudWatch Log Groups
    pub async fn list_log_groups(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
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

        let client = logs::Client::new(&aws_config);
        let mut paginator = client.describe_log_groups().into_paginator().send();

        let mut log_groups = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(log_group_list) = page.log_groups {
                for log_group in log_group_list {
                    let log_group_json = self.log_group_to_json(&log_group);
                    log_groups.push(log_group_json);
                }
            }
        }

        Ok(log_groups)
    }

    /// Get detailed information for specific Log Group
    pub async fn describe_log_group(
        &self,
        account_id: &str,
        region: &str,
        log_group_name: &str,
    ) -> Result<serde_json::Value> {
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

        let client = logs::Client::new(&aws_config);
        let response = client
            .describe_log_groups()
            .log_group_name_prefix(log_group_name)
            .send()
            .await?;

        if let Some(log_groups) = response.log_groups {
            if let Some(log_group) = log_groups.into_iter().find(|lg| {
                lg.log_group_name
                    .as_ref()
                    .is_some_and(|name| name == log_group_name)
            }) {
                Ok(self.log_group_to_json(&log_group))
            } else {
                Err(anyhow::anyhow!("Log group {} not found", log_group_name))
            }
        } else {
            Err(anyhow::anyhow!("Log group {} not found", log_group_name))
        }
    }

    fn log_group_to_json(&self, log_group: &logs::types::LogGroup) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(log_group_name) = &log_group.log_group_name {
            json.insert(
                "LogGroupName".to_string(),
                serde_json::Value::String(log_group_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(log_group_name.clone()),
            );
        }

        if let Some(creation_time) = log_group.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::Number(creation_time.into()),
            );
        }

        if let Some(retention_in_days) = log_group.retention_in_days {
            json.insert(
                "RetentionInDays".to_string(),
                serde_json::Value::Number(retention_in_days.into()),
            );
        }

        if let Some(metric_filter_count) = log_group.metric_filter_count {
            json.insert(
                "MetricFilterCount".to_string(),
                serde_json::Value::Number(metric_filter_count.into()),
            );
        }

        if let Some(arn) = &log_group.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(stored_bytes) = log_group.stored_bytes {
            json.insert(
                "StoredBytes".to_string(),
                serde_json::Value::Number(stored_bytes.into()),
            );
        }

        if let Some(kms_key_id) = &log_group.kms_key_id {
            json.insert(
                "KmsKeyId".to_string(),
                serde_json::Value::String(kms_key_id.clone()),
            );
        }

        if let Some(data_protection_status) = &log_group.data_protection_status {
            json.insert(
                "DataProtectionStatus".to_string(),
                serde_json::Value::String(data_protection_status.as_str().to_string()),
            );
        }

        // Add a status field for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// Describe log groups with optional filtering
    pub async fn describe_log_groups(
        &self,
        account_id: &str,
        region: &str,
        name_prefix: Option<&str>,
        limit: Option<i32>,
    ) -> Result<Vec<serde_json::Value>> {
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

        let client = logs::Client::new(&aws_config);
        let mut request = client.describe_log_groups();
        
        if let Some(prefix) = name_prefix {
            request = request.log_group_name_prefix(prefix);
        }
        
        if let Some(limit_val) = limit {
            request = request.limit(limit_val);
        }

        let mut paginator = request.into_paginator().send();
        let mut log_groups = Vec::new();
        
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(log_group_list) = page.log_groups {
                for log_group in log_group_list {
                    let log_group_json = self.log_group_to_json(&log_group);
                    log_groups.push(log_group_json);
                }
            }
        }

        Ok(log_groups)
    }

    /// Get log events from a specific log group and stream
    pub async fn get_log_events(
        &self,
        account_id: &str,
        region: &str,
        log_group_name: &str,
        log_stream_name: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        filter_pattern: Option<&str>,
        limit: Option<i32>,
    ) -> Result<Vec<serde_json::Value>> {
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

        let client = logs::Client::new(&aws_config);
        
        // If no specific log stream is provided, get all log streams and their events
        let log_streams = if let Some(stream_name) = log_stream_name {
            vec![stream_name.to_string()]
        } else {
            self.get_log_stream_names(&client, log_group_name, limit.map(|l| l / 10)).await?
        };

        let mut all_events = Vec::new();
        
        for stream_name in log_streams {
            let events = self.get_log_events_from_stream(
                &client,
                log_group_name,
                &stream_name,
                start_time,
                end_time,
                filter_pattern,
                limit,
            ).await?;
            
            all_events.extend(events);
            
            // Break if we have enough events
            if let Some(limit_val) = limit {
                if all_events.len() >= limit_val as usize {
                    all_events.truncate(limit_val as usize);
                    break;
                }
            }
        }

        // Sort by timestamp (most recent first)
        all_events.sort_by(|a, b| {
            let timestamp_a = a.get("Timestamp").and_then(|t| t.as_i64()).unwrap_or(0);
            let timestamp_b = b.get("Timestamp").and_then(|t| t.as_i64()).unwrap_or(0);
            timestamp_b.cmp(&timestamp_a)
        });

        Ok(all_events)
    }

    /// Get log stream names from a log group
    async fn get_log_stream_names(
        &self,
        client: &logs::Client,
        log_group_name: &str,
        limit: Option<i32>,
    ) -> Result<Vec<String>> {
        let mut request = client
            .describe_log_streams()
            .log_group_name(log_group_name)
            .order_by(logs::types::OrderBy::LastEventTime)
            .descending(true);
            
        if let Some(limit_val) = limit {
            request = request.limit(limit_val);
        }

        let response = request.send().await?;
        
        let stream_names = response
            .log_streams
            .unwrap_or_default()
            .into_iter()
            .filter_map(|stream| stream.log_stream_name)
            .collect();
            
        Ok(stream_names)
    }

    /// Get log events from a specific stream
    async fn get_log_events_from_stream(
        &self,
        client: &logs::Client,
        log_group_name: &str,
        log_stream_name: &str,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        filter_pattern: Option<&str>,
        limit: Option<i32>,
    ) -> Result<Vec<serde_json::Value>> {
        let mut request = client
            .get_log_events()
            .log_group_name(log_group_name)
            .log_stream_name(log_stream_name);

        if let Some(start) = start_time {
            request = request.start_time(start.timestamp_millis());
        }
        
        if let Some(end) = end_time {
            request = request.end_time(end.timestamp_millis());
        }

        if let Some(limit_val) = limit {
            request = request.limit(limit_val);
        }

        let response = request.send().await?;
        
        let mut events = Vec::new();
        if let Some(log_events) = response.events {
            for event in log_events {
                // Apply filter pattern if specified
                if let Some(pattern) = filter_pattern {
                    if let Some(message) = &event.message {
                        if !self.matches_filter_pattern(message, pattern) {
                            continue;
                        }
                    }
                }
                
                let event_json = self.log_event_to_json(&event, log_stream_name);
                events.push(event_json);
            }
        }

        Ok(events)
    }

    /// Simple pattern matching for CloudWatch filter patterns
    fn matches_filter_pattern(&self, message: &str, pattern: &str) -> bool {
        // Simple implementation - just check if pattern exists in message
        // For full CloudWatch filter pattern support, this would need more sophisticated parsing
        message.contains(pattern)
    }

    /// Convert log event to JSON
    fn log_event_to_json(&self, event: &logs::types::OutputLogEvent, stream_name: &str) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(timestamp) = event.timestamp {
            json.insert(
                "Timestamp".to_string(),
                serde_json::Value::Number(timestamp.into()),
            );
            
            // Also add human-readable timestamp
            if let Some(dt) = DateTime::from_timestamp_millis(timestamp) {
                json.insert(
                    "TimestampISO".to_string(),
                    serde_json::Value::String(dt.to_rfc3339()),
                );
            }
        }

        if let Some(message) = &event.message {
            json.insert(
                "Message".to_string(),
                serde_json::Value::String(message.clone()),
            );
        }

        if let Some(ingestion_time) = event.ingestion_time {
            json.insert(
                "IngestionTime".to_string(),
                serde_json::Value::Number(ingestion_time.into()),
            );
        }

        json.insert(
            "LogStreamName".to_string(),
            serde_json::Value::String(stream_name.to_string()),
        );

        serde_json::Value::Object(json)
    }
}
