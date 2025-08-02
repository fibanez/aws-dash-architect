use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudwatch as cloudwatch;
use std::sync::Arc;

pub struct CloudWatchService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CloudWatchService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CloudWatch alarms
    pub async fn list_alarms(
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

        let client = cloudwatch::Client::new(&aws_config);
        let mut paginator = client.describe_alarms().into_paginator().send();

        let mut alarms = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(metric_alarms) = page.metric_alarms {
                for alarm in metric_alarms {
                    let alarm_json = self.alarm_to_json(&alarm);
                    alarms.push(alarm_json);
                }
            }
        }

        Ok(alarms)
    }

    /// Get detailed information for specific CloudWatch alarm
    pub async fn describe_alarm(
        &self,
        account_id: &str,
        region: &str,
        alarm_name: &str,
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

        let client = cloudwatch::Client::new(&aws_config);
        let response = client
            .describe_alarms()
            .alarm_names(alarm_name)
            .send()
            .await?;

        if let Some(metric_alarms) = response.metric_alarms {
            if let Some(alarm) = metric_alarms.first() {
                return Ok(self.alarm_to_json(alarm));
            }
        }

        Err(anyhow::anyhow!("Alarm {} not found", alarm_name))
    }

    fn alarm_to_json(&self, alarm: &cloudwatch::types::MetricAlarm) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(alarm_name) = &alarm.alarm_name {
            json.insert(
                "AlarmName".to_string(),
                serde_json::Value::String(alarm_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(alarm_name.clone()),
            );
        }

        if let Some(alarm_arn) = &alarm.alarm_arn {
            json.insert(
                "AlarmArn".to_string(),
                serde_json::Value::String(alarm_arn.clone()),
            );
        }

        if let Some(alarm_description) = &alarm.alarm_description {
            json.insert(
                "AlarmDescription".to_string(),
                serde_json::Value::String(alarm_description.clone()),
            );
        }

        if let Some(alarm_configuration_updated_timestamp) =
            alarm.alarm_configuration_updated_timestamp
        {
            json.insert(
                "AlarmConfigurationUpdatedTimestamp".to_string(),
                serde_json::Value::String(alarm_configuration_updated_timestamp.to_string()),
            );
        }

        if let Some(actions_enabled) = alarm.actions_enabled {
            json.insert(
                "ActionsEnabled".to_string(),
                serde_json::Value::Bool(actions_enabled),
            );
        }

        if let Some(state_value) = &alarm.state_value {
            json.insert(
                "StateValue".to_string(),
                serde_json::Value::String(state_value.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state_value.as_str().to_string()),
            );
        }

        if let Some(state_reason) = &alarm.state_reason {
            json.insert(
                "StateReason".to_string(),
                serde_json::Value::String(state_reason.clone()),
            );
        }

        if let Some(state_updated_timestamp) = alarm.state_updated_timestamp {
            json.insert(
                "StateUpdatedTimestamp".to_string(),
                serde_json::Value::String(state_updated_timestamp.to_string()),
            );
        }

        if let Some(metric_name) = &alarm.metric_name {
            json.insert(
                "MetricName".to_string(),
                serde_json::Value::String(metric_name.clone()),
            );
        }

        if let Some(namespace) = &alarm.namespace {
            json.insert(
                "Namespace".to_string(),
                serde_json::Value::String(namespace.clone()),
            );
        }

        if let Some(statistic) = &alarm.statistic {
            json.insert(
                "Statistic".to_string(),
                serde_json::Value::String(statistic.as_str().to_string()),
            );
        }

        if let Some(comparison_operator) = &alarm.comparison_operator {
            json.insert(
                "ComparisonOperator".to_string(),
                serde_json::Value::String(comparison_operator.as_str().to_string()),
            );
        }

        if let Some(threshold) = alarm.threshold {
            json.insert(
                "Threshold".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(threshold).unwrap_or(serde_json::Number::from(0)),
                ),
            );
        }

        if let Some(period) = alarm.period {
            json.insert(
                "Period".to_string(),
                serde_json::Value::Number(period.into()),
            );
        }

        if let Some(evaluation_periods) = alarm.evaluation_periods {
            json.insert(
                "EvaluationPeriods".to_string(),
                serde_json::Value::Number(evaluation_periods.into()),
            );
        }

        serde_json::Value::Object(json)
    }

    /// List CloudWatch dashboards
    pub async fn list_dashboards(
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

        let client = cloudwatch::Client::new(&aws_config);
        let mut paginator = client.list_dashboards().into_paginator().send();

        let mut dashboards = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(dashboard_entries) = page.dashboard_entries {
                for dashboard in dashboard_entries {
                    let dashboard_json = self.dashboard_to_json(&dashboard);
                    dashboards.push(dashboard_json);
                }
            }
        }

        Ok(dashboards)
    }

    /// Get detailed information for specific CloudWatch dashboard
    pub async fn describe_dashboard(
        &self,
        account_id: &str,
        region: &str,
        dashboard_name: &str,
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

        let client = cloudwatch::Client::new(&aws_config);
        let response = client
            .get_dashboard()
            .dashboard_name(dashboard_name)
            .send()
            .await?;

        let mut dashboard_details = serde_json::Map::new();

        if let Some(dashboard_name) = response.dashboard_name {
            dashboard_details.insert(
                "DashboardName".to_string(),
                serde_json::Value::String(dashboard_name),
            );
        }

        if let Some(dashboard_arn) = response.dashboard_arn {
            dashboard_details.insert(
                "DashboardArn".to_string(),
                serde_json::Value::String(dashboard_arn),
            );
        }

        if let Some(dashboard_body) = response.dashboard_body {
            dashboard_details.insert(
                "DashboardBody".to_string(),
                serde_json::Value::String(dashboard_body),
            );
        }

        Ok(serde_json::Value::Object(dashboard_details))
    }

    fn dashboard_to_json(
        &self,
        dashboard: &cloudwatch::types::DashboardEntry,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(dashboard_name) = &dashboard.dashboard_name {
            json.insert(
                "DashboardName".to_string(),
                serde_json::Value::String(dashboard_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(dashboard_name.clone()),
            );
        }

        if let Some(dashboard_arn) = &dashboard.dashboard_arn {
            json.insert(
                "DashboardArn".to_string(),
                serde_json::Value::String(dashboard_arn.clone()),
            );
        }

        if let Some(last_modified) = dashboard.last_modified {
            json.insert(
                "LastModified".to_string(),
                serde_json::Value::String(last_modified.to_string()),
            );
        }

        if let Some(size) = dashboard.size {
            json.insert("Size".to_string(), serde_json::Value::Number(size.into()));
        }

        // Set a default status for dashboards
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Available".to_string()),
        );

        serde_json::Value::Object(json)
    }
}
