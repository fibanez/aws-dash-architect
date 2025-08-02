use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudwatchlogs as logs;
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
}
