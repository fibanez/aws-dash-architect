use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudwatch as cloudwatch;
use std::collections::HashSet;
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

    /// List CloudWatch composite alarms
    pub async fn list_composite_alarms(
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
        let mut alarms = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client
                .describe_alarms()
                .alarm_types(cloudwatch::types::AlarmType::CompositeAlarm);

            if let Some(token) = next_token.take() {
                request = request.next_token(token);
            }

            let response = request.send().await?;
            if let Some(composite_alarms) = response.composite_alarms {
                for alarm in composite_alarms {
                    alarms.push(self.composite_alarm_to_json(&alarm));
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(alarms)
    }

    /// List CloudWatch metrics
    pub async fn list_metrics(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let alarm_metrics = self.list_alarm_metric_keys(account_id, region).await?;
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
        let mut metrics = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.list_metrics();
            if let Some(token) = next_token.take() {
                request = request.next_token(token);
            }

            let response = request.send().await?;
            if let Some(metric_list) = response.metrics {
                for metric in metric_list {
                    if self.should_include_metric(&metric, &alarm_metrics) {
                        metrics.push(self.metric_to_json(&metric));
                    }
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(metrics)
    }

    async fn list_alarm_metric_keys(
        &self,
        account_id: &str,
        region: &str,
    ) -> Result<HashSet<(String, String)>> {
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
        let mut keys = HashSet::new();

        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(metric_alarms) = page.metric_alarms {
                for alarm in metric_alarms {
                    if let (Some(namespace), Some(metric_name)) =
                        (alarm.namespace.clone(), alarm.metric_name.clone())
                    {
                        keys.insert((namespace, metric_name));
                    }

                    if let Some(metric_queries) = alarm.metrics {
                        for query in metric_queries {
                            if let Some(metric_stat) = query.metric_stat {
                                if let Some(metric) = metric_stat.metric {
                                    if let (Some(namespace), Some(metric_name)) =
                                        (metric.namespace, metric.metric_name)
                                    {
                                        keys.insert((namespace, metric_name));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(keys)
    }

    fn should_include_metric(
        &self,
        metric: &cloudwatch::types::Metric,
        alarm_metrics: &HashSet<(String, String)>,
    ) -> bool {
        let namespace = metric.namespace.as_deref().unwrap_or("");
        let metric_name = metric.metric_name.as_deref().unwrap_or("");

        if namespace.is_empty() || metric_name.is_empty() {
            return false;
        }

        if !namespace.starts_with("AWS/") {
            return true;
        }

        alarm_metrics.contains(&(namespace.to_string(), metric_name.to_string()))
    }

    /// List CloudWatch insight rules
    pub async fn list_insight_rules(
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
        let mut rules = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.describe_insight_rules();
            if let Some(token) = next_token.take() {
                request = request.next_token(token);
            }

            let response = request.send().await?;
            if let Some(rule_list) = response.insight_rules {
                for rule in rule_list {
                    rules.push(self.insight_rule_to_json(&rule));
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(rules)
    }

    /// List CloudWatch anomaly detectors
    pub async fn list_anomaly_detectors(
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
        let mut detectors = Vec::new();
        let mut next_token = None;

        loop {
            let mut request = client.describe_anomaly_detectors();
            if let Some(token) = next_token.take() {
                request = request.next_token(token);
            }

            let response = request.send().await?;
            if let Some(detector_list) = response.anomaly_detectors {
                for detector in detector_list {
                    detectors.push(self.anomaly_detector_to_json(&detector));
                }
            }

            next_token = response.next_token;
            if next_token.is_none() {
                break;
            }
        }

        Ok(detectors)
    }

    fn composite_alarm_to_json(
        &self,
        alarm: &cloudwatch::types::CompositeAlarm,
    ) -> serde_json::Value {
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

        if let Some(alarm_rule) = &alarm.alarm_rule {
            json.insert(
                "AlarmRule".to_string(),
                serde_json::Value::String(alarm_rule.clone()),
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

        serde_json::Value::Object(json)
    }

    fn metric_to_json(&self, metric: &cloudwatch::types::Metric) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        let metric_name = metric.metric_name.clone().unwrap_or_default();
        let namespace = metric.namespace.clone().unwrap_or_default();

        json.insert(
            "MetricName".to_string(),
            serde_json::Value::String(metric_name.clone()),
        );
        json.insert(
            "Namespace".to_string(),
            serde_json::Value::String(namespace.clone()),
        );

        if let Some(dimensions) = &metric.dimensions {
            let dims_json: Vec<serde_json::Value> = dimensions
                .iter()
                .map(|dim| {
                    let mut dim_json = serde_json::Map::new();
                    if let Some(name) = &dim.name {
                        dim_json
                            .insert("Name".to_string(), serde_json::Value::String(name.clone()));
                    }
                    if let Some(value) = &dim.value {
                        dim_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                    }
                    serde_json::Value::Object(dim_json)
                })
                .collect();
            json.insert(
                "Dimensions".to_string(),
                serde_json::Value::Array(dims_json),
            );
        }

        let mut identifier = format!("{}/{}", namespace, metric_name);
        if let Some(dimensions) = &metric.dimensions {
            if !dimensions.is_empty() {
                let mut dim_parts: Vec<String> = dimensions
                    .iter()
                    .filter_map(|dim| {
                        let name = dim.name.as_ref()?;
                        let value = dim.value.as_ref()?;
                        Some(format!("{}={}", name, value))
                    })
                    .collect();
                dim_parts.sort();
                identifier.push(':');
                identifier.push_str(&dim_parts.join(","));
            }
        }

        json.insert(
            "MetricId".to_string(),
            serde_json::Value::String(identifier.clone()),
        );
        json.insert("Name".to_string(), serde_json::Value::String(identifier));
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Available".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn insight_rule_to_json(&self, rule: &cloudwatch::types::InsightRule) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(rule_name) = &rule.name {
            json.insert(
                "RuleName".to_string(),
                serde_json::Value::String(rule_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(rule_name.clone()),
            );
        }

        if let Some(state) = &rule.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(schema) = &rule.schema {
            json.insert(
                "Schema".to_string(),
                serde_json::Value::String(schema.clone()),
            );
        }

        if let Some(definition) = &rule.definition {
            json.insert(
                "Definition".to_string(),
                serde_json::Value::String(definition.clone()),
            );
        }

        if let Some(managed_rule) = rule.managed_rule {
            json.insert(
                "ManagedRule".to_string(),
                serde_json::Value::Bool(managed_rule),
            );
        }

        serde_json::Value::Object(json)
    }

    fn anomaly_detector_to_json(
        &self,
        detector: &cloudwatch::types::AnomalyDetector,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        let (metric_name, namespace, stat, dimensions, detector_type, metric_math_queries) =
            if let Some(single) = &detector.single_metric_anomaly_detector {
                (
                    single.metric_name.clone(),
                    single.namespace.clone(),
                    single.stat.clone(),
                    single.dimensions.clone(),
                    "SingleMetric",
                    None,
                )
            } else if let Some(metric_math) = &detector.metric_math_anomaly_detector {
                let query_count = metric_math
                    .metric_data_queries
                    .as_ref()
                    .map(|queries| queries.len())
                    .unwrap_or(0);
                json.insert(
                    "MetricDataQueryCount".to_string(),
                    serde_json::Value::Number(query_count.into()),
                );
                (
                    None,
                    None,
                    None,
                    None,
                    "MetricMath",
                    metric_math.metric_data_queries.clone(),
                )
            } else {
                (None, None, None, None, "Unknown", None)
            };

        json.insert(
            "DetectorType".to_string(),
            serde_json::Value::String(detector_type.to_string()),
        );

        if let Some(metric_name) = &metric_name {
            json.insert(
                "MetricName".to_string(),
                serde_json::Value::String(metric_name.clone()),
            );
        }

        if let Some(namespace) = &namespace {
            json.insert(
                "Namespace".to_string(),
                serde_json::Value::String(namespace.clone()),
            );
        }

        if let Some(stat) = &stat {
            json.insert("Stat".to_string(), serde_json::Value::String(stat.clone()));
        }

        if let Some(state_value) = &detector.state_value {
            json.insert(
                "StateValue".to_string(),
                serde_json::Value::String(state_value.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(state_value.as_str().to_string()),
            );
        }

        if let Some(dimensions) = &dimensions {
            let dims_json: Vec<serde_json::Value> = dimensions
                .iter()
                .map(|dim| {
                    let mut dim_json = serde_json::Map::new();
                    if let Some(name) = &dim.name {
                        dim_json
                            .insert("Name".to_string(), serde_json::Value::String(name.clone()));
                    }
                    if let Some(value) = &dim.value {
                        dim_json.insert(
                            "Value".to_string(),
                            serde_json::Value::String(value.clone()),
                        );
                    }
                    serde_json::Value::Object(dim_json)
                })
                .collect();
            json.insert(
                "Dimensions".to_string(),
                serde_json::Value::Array(dims_json),
            );
        }

        let metric_name = metric_name.clone().unwrap_or_default();
        let namespace = namespace.clone().unwrap_or_default();
        let stat_value = stat.clone().unwrap_or_default();
        let dimension_suffix = dimensions
            .as_ref()
            .filter(|dims| !dims.is_empty())
            .map(|dims| {
                let mut parts: Vec<String> = dims
                    .iter()
                    .filter_map(|dim| {
                        let name = dim.name.as_ref()?;
                        let value = dim.value.as_ref()?;
                        Some(format!("{}={}", name, value))
                    })
                    .collect();
                parts.sort();
                parts.join(",")
            })
            .unwrap_or_default();

        let identifier = if detector_type == "MetricMath" {
            let query_id = metric_math_queries
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(idx, query)| {
                    query
                        .id
                        .clone()
                        .or_else(|| query.label.clone())
                        .or_else(|| query.expression.clone())
                        .unwrap_or_else(|| format!("query-{}", idx))
                })
                .collect::<Vec<String>>()
                .join("|");
            format!("metric-math:{}", query_id)
        } else if metric_name.is_empty() && namespace.is_empty() {
            format!("anomaly-detector:{}", detector_type)
        } else if dimension_suffix.is_empty() && stat_value.is_empty() {
            format!("{}/{}", namespace, metric_name)
        } else if dimension_suffix.is_empty() {
            format!("{}/{}:{}", namespace, metric_name, stat_value)
        } else if stat_value.is_empty() {
            format!("{}/{}:{}", namespace, metric_name, dimension_suffix)
        } else {
            format!(
                "{}/{}:{}:{}",
                namespace, metric_name, stat_value, dimension_suffix
            )
        };

        json.insert(
            "DetectorId".to_string(),
            serde_json::Value::String(identifier.clone()),
        );
        json.insert("Name".to_string(), serde_json::Value::String(identifier));

        serde_json::Value::Object(json)
    }
}
