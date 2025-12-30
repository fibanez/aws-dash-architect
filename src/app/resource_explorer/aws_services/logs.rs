use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudwatchlogs as logs;
use chrono::{DateTime, Utc};
use std::sync::Arc;

/// Parameters for getting log events
#[derive(Default)]
pub struct LogEventsParams<'a> {
    pub log_group_name: &'a str,
    pub log_stream_name: Option<&'a str>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub filter_pattern: Option<&'a str>,
    pub limit: Option<i32>,
}

/// Parameters for getting log events from a specific stream
#[derive(Default)]
pub struct LogStreamEventsParams<'a> {
    pub log_group_name: &'a str,
    pub log_stream_name: &'a str,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub filter_pattern: Option<&'a str>,
    pub limit: Option<i32>,
}

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
        params: LogEventsParams<'_>,
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
        let log_streams = if let Some(stream_name) = params.log_stream_name {
            vec![stream_name.to_string()]
        } else {
            self.get_log_stream_names(&client, params.log_group_name, params.limit.map(|l| l / 10))
                .await?
        };

        let mut all_events = Vec::new();

        for stream_name in log_streams {
            let stream_params = LogStreamEventsParams {
                log_group_name: params.log_group_name,
                log_stream_name: &stream_name,
                start_time: params.start_time,
                end_time: params.end_time,
                filter_pattern: params.filter_pattern,
                limit: params.limit,
            };

            let events = self
                .get_log_events_from_stream(&client, stream_params)
                .await?;

            all_events.extend(events);

            // Break if we have enough events
            if let Some(limit_val) = params.limit {
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
        params: LogStreamEventsParams<'_>,
    ) -> Result<Vec<serde_json::Value>> {
        let mut request = client
            .get_log_events()
            .log_group_name(params.log_group_name)
            .log_stream_name(params.log_stream_name);

        if let Some(start) = params.start_time {
            request = request.start_time(start.timestamp_millis());
        }

        if let Some(end) = params.end_time {
            request = request.end_time(end.timestamp_millis());
        }

        if let Some(limit_val) = params.limit {
            request = request.limit(limit_val);
        }

        let response = request.send().await?;

        let mut events = Vec::new();
        if let Some(log_events) = response.events {
            for event in log_events {
                // Apply filter pattern if specified
                if let Some(pattern) = params.filter_pattern {
                    if let Some(message) = &event.message {
                        if !self.matches_filter_pattern(message, pattern) {
                            continue;
                        }
                    }
                }

                let event_json = self.log_event_to_json(&event, params.log_stream_name);
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
    fn log_event_to_json(
        &self,
        event: &logs::types::OutputLogEvent,
        stream_name: &str,
    ) -> serde_json::Value {
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

    /// List CloudWatch Log Streams across log groups
    pub async fn list_log_streams(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let log_groups = self.list_log_groups(account_id, region).await?;
        let log_group_names: Vec<String> = log_groups
            .into_iter()
            .filter_map(|group| {
                group
                    .get("LogGroupName")
                    .and_then(|name| name.as_str())
                    .map(|name| name.to_string())
            })
            .collect();

        if log_group_names.is_empty() {
            return Ok(Vec::new());
        }

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
        let mut log_streams = Vec::new();

        for log_group_name in log_group_names {
            let mut next_token = None;
            loop {
                let mut request = client
                    .describe_log_streams()
                    .log_group_name(&log_group_name);

                if let Some(token) = next_token.take() {
                    request = request.next_token(token);
                }

                let response = match request.send().await {
                    Ok(response) => response,
                    Err(error) => {
                        tracing::warn!(
                            "Failed to describe log streams for {}: {}",
                            log_group_name,
                            error
                        );
                        break;
                    }
                };
                if let Some(stream_list) = response.log_streams {
                    for stream in stream_list {
                        let stream_json = self.log_stream_to_json(&log_group_name, &stream);
                        log_streams.push(stream_json);
                    }
                }

                next_token = response.next_token;
                if next_token.is_none() {
                    break;
                }
            }
        }

        Ok(log_streams)
    }

    /// List CloudWatch Logs metric filters
    pub async fn list_metric_filters(
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
        let mut filters = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.describe_metric_filters();
            if let Some(token) = next_token.take() {
                request = request.next_token(token);
            }

            let response = request.send().await?;
            if let Some(filter_list) = response.metric_filters {
                for filter in filter_list {
                    filters.push(self.metric_filter_to_json(&filter));
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(filters)
    }

    /// List CloudWatch Logs subscription filters across log groups
    pub async fn list_subscription_filters(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let log_groups = self.list_log_groups(account_id, region).await?;
        let log_group_names: Vec<String> = log_groups
            .into_iter()
            .filter_map(|group| {
                group
                    .get("LogGroupName")
                    .and_then(|name| name.as_str())
                    .map(|name| name.to_string())
            })
            .collect();

        if log_group_names.is_empty() {
            return Ok(Vec::new());
        }

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
        let mut filters = Vec::new();

        for log_group_name in log_group_names {
            let mut next_token = None;
            loop {
                let mut request = client
                    .describe_subscription_filters()
                    .log_group_name(&log_group_name);

                if let Some(token) = next_token.take() {
                    request = request.next_token(token);
                }

                let response = match request.send().await {
                    Ok(response) => response,
                    Err(error) => {
                        tracing::warn!(
                            "Failed to describe subscription filters for {}: {}",
                            log_group_name,
                            error
                        );
                        break;
                    }
                };
                if let Some(filter_list) = response.subscription_filters {
                    for filter in filter_list {
                        filters.push(self.subscription_filter_to_json(&log_group_name, &filter));
                    }
                }

                next_token = response.next_token;
                if next_token.is_none() {
                    break;
                }
            }
        }

        Ok(filters)
    }

    /// List CloudWatch Logs resource policies
    pub async fn list_resource_policies(
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
        let mut policies = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.describe_resource_policies();
            if let Some(token) = next_token.take() {
                request = request.next_token(token);
            }

            let response = request.send().await?;
            if let Some(policy_list) = response.resource_policies {
                for policy in policy_list {
                    policies.push(self.resource_policy_to_json(&policy));
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(policies)
    }

    /// List CloudWatch Logs query definitions
    pub async fn list_query_definitions(
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
        let mut definitions = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.describe_query_definitions();
            if let Some(token) = next_token.take() {
                request = request.next_token(token);
            }

            let response = request.send().await?;
            if let Some(def_list) = response.query_definitions {
                for definition in def_list {
                    definitions.push(self.query_definition_to_json(&definition));
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(definitions)
    }

    fn log_stream_to_json(
        &self,
        log_group_name: &str,
        stream: &logs::types::LogStream,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(log_stream_name) = &stream.log_stream_name {
            json.insert(
                "LogStreamName".to_string(),
                serde_json::Value::String(log_stream_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(log_stream_name.clone()),
            );
        }

        json.insert(
            "LogGroupName".to_string(),
            serde_json::Value::String(log_group_name.to_string()),
        );

        if let Some(arn) = &stream.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(first_event) = stream.first_event_timestamp {
            json.insert(
                "FirstEventTimestamp".to_string(),
                serde_json::Value::Number(first_event.into()),
            );
        }

        if let Some(last_event) = stream.last_event_timestamp {
            json.insert(
                "LastEventTimestamp".to_string(),
                serde_json::Value::Number(last_event.into()),
            );
        }

        if let Some(last_ingestion) = stream.last_ingestion_time {
            json.insert(
                "LastIngestionTime".to_string(),
                serde_json::Value::Number(last_ingestion.into()),
            );
        }

        if let Some(upload_sequence) = &stream.upload_sequence_token {
            json.insert(
                "UploadSequenceToken".to_string(),
                serde_json::Value::String(upload_sequence.clone()),
            );
        }

        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn metric_filter_to_json(&self, filter: &logs::types::MetricFilter) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(filter_name) = &filter.filter_name {
            json.insert(
                "FilterName".to_string(),
                serde_json::Value::String(filter_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(filter_name.clone()),
            );
        }

        if let Some(log_group_name) = &filter.log_group_name {
            json.insert(
                "LogGroupName".to_string(),
                serde_json::Value::String(log_group_name.clone()),
            );
        }

        if let Some(filter_pattern) = &filter.filter_pattern {
            json.insert(
                "FilterPattern".to_string(),
                serde_json::Value::String(filter_pattern.clone()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn subscription_filter_to_json(
        &self,
        log_group_name: &str,
        filter: &logs::types::SubscriptionFilter,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(filter_name) = &filter.filter_name {
            json.insert(
                "FilterName".to_string(),
                serde_json::Value::String(filter_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(filter_name.clone()),
            );
        }

        json.insert(
            "LogGroupName".to_string(),
            serde_json::Value::String(log_group_name.to_string()),
        );

        if let Some(filter_pattern) = &filter.filter_pattern {
            json.insert(
                "FilterPattern".to_string(),
                serde_json::Value::String(filter_pattern.clone()),
            );
        }

        if let Some(destination_arn) = &filter.destination_arn {
            json.insert(
                "DestinationArn".to_string(),
                serde_json::Value::String(destination_arn.clone()),
            );
        }

        if let Some(role_arn) = &filter.role_arn {
            json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(role_arn.clone()),
            );
        }

        if let Some(distribution) = &filter.distribution {
            json.insert(
                "Distribution".to_string(),
                serde_json::Value::String(distribution.as_str().to_string()),
            );
        }

        if let Some(creation_time) = filter.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::Number(creation_time.into()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn resource_policy_to_json(
        &self,
        policy: &logs::types::ResourcePolicy,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(policy_name) = &policy.policy_name {
            json.insert(
                "PolicyName".to_string(),
                serde_json::Value::String(policy_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(policy_name.clone()),
            );
        }

        if let Some(policy_document) = &policy.policy_document {
            json.insert(
                "PolicyDocument".to_string(),
                serde_json::Value::String(policy_document.clone()),
            );
        }

        if let Some(last_updated_time) = policy.last_updated_time {
            json.insert(
                "LastUpdatedTime".to_string(),
                serde_json::Value::Number(last_updated_time.into()),
            );
        }

        serde_json::Value::Object(json)
    }

    fn query_definition_to_json(
        &self,
        definition: &logs::types::QueryDefinition,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(definition_id) = &definition.query_definition_id {
            json.insert(
                "QueryDefinitionId".to_string(),
                serde_json::Value::String(definition_id.clone()),
            );
        }

        if let Some(name) = &definition.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(query_string) = &definition.query_string {
            json.insert(
                "QueryString".to_string(),
                serde_json::Value::String(query_string.clone()),
            );
        }

        if let Some(log_group_names) = &definition.log_group_names {
            let names_json: Vec<serde_json::Value> = log_group_names
                .iter()
                .map(|name| serde_json::Value::String(name.clone()))
                .collect();
            json.insert("LogGroupNames".to_string(), serde_json::Value::Array(names_json));
        }

        if let Some(last_modified) = definition.last_modified {
            json.insert(
                "LastModified".to_string(),
                serde_json::Value::Number(last_modified.into()),
            );
        }

        serde_json::Value::Object(json)
    }
}
