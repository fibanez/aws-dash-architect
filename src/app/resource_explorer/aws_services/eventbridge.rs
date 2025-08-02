use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_eventbridge as eventbridge;
use std::sync::Arc;

pub struct EventBridgeService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl EventBridgeService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List EventBridge event buses
    pub async fn list_event_buses(
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

        let client = eventbridge::Client::new(&aws_config);
        let response = client.list_event_buses().send().await?;

        let mut event_buses = Vec::new();
        if let Some(event_bus_list) = response.event_buses {
            for event_bus in event_bus_list {
                let event_bus_json = self.event_bus_to_json(&event_bus);
                event_buses.push(event_bus_json);
            }
        }

        Ok(event_buses)
    }

    /// Get detailed information for specific EventBridge event bus
    pub async fn describe_event_bus(
        &self,
        account_id: &str,
        region: &str,
        event_bus_name: &str,
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

        let client = eventbridge::Client::new(&aws_config);
        let response = client
            .describe_event_bus()
            .name(event_bus_name)
            .send()
            .await?;

        let mut event_bus_details = serde_json::Map::new();

        if let Some(name) = response.name {
            event_bus_details.insert("Name".to_string(), serde_json::Value::String(name));
        }

        if let Some(arn) = response.arn {
            event_bus_details.insert("Arn".to_string(), serde_json::Value::String(arn));
        }

        if let Some(policy) = response.policy {
            event_bus_details.insert("Policy".to_string(), serde_json::Value::String(policy));
        }

        // Note: DescribeEventBus response doesn't have kms_key_identifier field in the SDK

        if let Some(description) = response.description {
            event_bus_details.insert(
                "Description".to_string(),
                serde_json::Value::String(description),
            );
        }

        if let Some(creation_time) = response.creation_time {
            event_bus_details.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(last_modified_time) = response.last_modified_time {
            event_bus_details.insert(
                "LastModifiedTime".to_string(),
                serde_json::Value::String(last_modified_time.to_string()),
            );
        }

        Ok(serde_json::Value::Object(event_bus_details))
    }

    /// List EventBridge rules
    pub async fn list_rules(
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

        let client = eventbridge::Client::new(&aws_config);
        let response = client.list_rules().send().await?;

        let mut rules = Vec::new();
        if let Some(rules_list) = response.rules {
            for rule in rules_list {
                let rule_json = self.rule_to_json(&rule);
                rules.push(rule_json);
            }
        }

        Ok(rules)
    }

    /// Get detailed information for specific EventBridge rule
    pub async fn describe_rule(
        &self,
        account_id: &str,
        region: &str,
        rule_name: &str,
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

        let client = eventbridge::Client::new(&aws_config);
        let response = client.describe_rule().name(rule_name).send().await?;

        let mut rule_details = serde_json::Map::new();

        if let Some(name) = response.name {
            rule_details.insert("Name".to_string(), serde_json::Value::String(name));
        }

        if let Some(arn) = response.arn {
            rule_details.insert("Arn".to_string(), serde_json::Value::String(arn));
        }

        if let Some(event_pattern) = response.event_pattern {
            rule_details.insert(
                "EventPattern".to_string(),
                serde_json::Value::String(event_pattern),
            );
        }

        if let Some(schedule_expression) = response.schedule_expression {
            rule_details.insert(
                "ScheduleExpression".to_string(),
                serde_json::Value::String(schedule_expression),
            );
        }

        if let Some(state) = response.state {
            rule_details.insert(
                "State".to_string(),
                serde_json::Value::String(state.as_str().to_string()),
            );
        }

        if let Some(description) = response.description {
            rule_details.insert(
                "Description".to_string(),
                serde_json::Value::String(description),
            );
        }

        if let Some(role_arn) = response.role_arn {
            rule_details.insert("RoleArn".to_string(), serde_json::Value::String(role_arn));
        }

        if let Some(managed_by) = response.managed_by {
            rule_details.insert(
                "ManagedBy".to_string(),
                serde_json::Value::String(managed_by),
            );
        }

        if let Some(event_bus_name) = response.event_bus_name {
            rule_details.insert(
                "EventBusName".to_string(),
                serde_json::Value::String(event_bus_name),
            );
        }

        if let Some(created_by) = response.created_by {
            rule_details.insert(
                "CreatedBy".to_string(),
                serde_json::Value::String(created_by),
            );
        }

        Ok(serde_json::Value::Object(rule_details))
    }

    fn event_bus_to_json(&self, event_bus: &eventbridge::types::EventBus) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &event_bus.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "EventBusName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &event_bus.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(policy) = &event_bus.policy {
            json.insert(
                "Policy".to_string(),
                serde_json::Value::String(policy.clone()),
            );
        }

        // Note: EventBus doesn't have kms_key_identifier field in the SDK

        if let Some(description) = &event_bus.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(creation_time) = event_bus.creation_time {
            json.insert(
                "CreationTime".to_string(),
                serde_json::Value::String(creation_time.to_string()),
            );
        }

        if let Some(last_modified_time) = event_bus.last_modified_time {
            json.insert(
                "LastModifiedTime".to_string(),
                serde_json::Value::String(last_modified_time.to_string()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn rule_to_json(&self, rule: &eventbridge::types::Rule) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &rule.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "RuleName".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(arn) = &rule.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(event_pattern) = &rule.event_pattern {
            json.insert(
                "EventPattern".to_string(),
                serde_json::Value::String(event_pattern.clone()),
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

        if let Some(description) = &rule.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(schedule_expression) = &rule.schedule_expression {
            json.insert(
                "ScheduleExpression".to_string(),
                serde_json::Value::String(schedule_expression.clone()),
            );
        }

        if let Some(role_arn) = &rule.role_arn {
            json.insert(
                "RoleArn".to_string(),
                serde_json::Value::String(role_arn.clone()),
            );
        }

        if let Some(managed_by) = &rule.managed_by {
            json.insert(
                "ManagedBy".to_string(),
                serde_json::Value::String(managed_by.clone()),
            );
        }

        if let Some(event_bus_name) = &rule.event_bus_name {
            json.insert(
                "EventBusName".to_string(),
                serde_json::Value::String(event_bus_name.clone()),
            );
        }

        serde_json::Value::Object(json)
    }
}
