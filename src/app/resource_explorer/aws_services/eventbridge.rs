use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_eventbridge as eventbridge;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

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
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("EventBridge", "list_event_buses", Some(region));

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
                let mut event_bus_json = self.event_bus_to_json(&event_bus);

                if include_details {
                    if let Some(name) = &event_bus.name {
                        // Get rules for this event bus
                        if let Ok(rules) = self.list_rules_for_bus_internal(&client, name).await {
                            if let Some(obj) = event_bus_json.as_object_mut() {
                                obj.insert("Rules".to_string(), rules);
                            }
                        }
                    }
                }

                event_buses.push(event_bus_json);
            }
        }

        report_status_done("EventBridge", "list_event_buses", Some(region));
        Ok(event_buses)
    }

    /// Get detailed information for a single EventBridge event bus (Phase 2 enrichment)
    pub async fn get_event_bus_details(
        &self,
        account_id: &str,
        region: &str,
        event_bus_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("EventBridge", "get_event_bus_details", Some(event_bus_name));

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
        let mut details = serde_json::Map::new();

        // Get event bus details
        report_status("EventBridge", "describe_event_bus", Some(event_bus_name));
        if let Ok(Ok(resp)) = timeout(
            Duration::from_secs(10),
            client.describe_event_bus().name(event_bus_name).send(),
        )
        .await
        {
            if let Some(name) = resp.name {
                details.insert("Name".to_string(), serde_json::Value::String(name));
            }
            if let Some(arn) = resp.arn {
                details.insert("Arn".to_string(), serde_json::Value::String(arn));
            }
            if let Some(policy) = resp.policy {
                details.insert("Policy".to_string(), serde_json::Value::String(policy));
            }
            if let Some(description) = resp.description {
                details.insert(
                    "Description".to_string(),
                    serde_json::Value::String(description),
                );
            }
        }

        // Get rules for this event bus
        report_status("EventBridge", "list_rules", Some(event_bus_name));
        if let Ok(rules) = self
            .list_rules_for_bus_internal(&client, event_bus_name)
            .await
        {
            // Also get targets for each rule
            let mut rules_with_targets = Vec::new();
            if let Some(rules_array) = rules.as_array() {
                for rule in rules_array {
                    let mut rule_obj = rule.clone();
                    if let Some(rule_name) = rule.get("Name").and_then(|v| v.as_str()) {
                        if let Ok(targets) = self
                            .list_targets_by_rule_internal(&client, rule_name, event_bus_name)
                            .await
                        {
                            if let Some(obj) = rule_obj.as_object_mut() {
                                obj.insert("Targets".to_string(), targets);
                            }
                        }
                    }
                    rules_with_targets.push(rule_obj);
                }
            }
            details.insert(
                "Rules".to_string(),
                serde_json::Value::Array(rules_with_targets),
            );
        }

        // Get archives associated with this event bus
        report_status("EventBridge", "list_archives", Some(event_bus_name));
        if let Ok(archives) = self.list_archives_internal(&client, event_bus_name).await {
            details.insert("Archives".to_string(), archives);
        }

        report_status_done("EventBridge", "get_event_bus_details", Some(event_bus_name));
        Ok(serde_json::Value::Object(details))
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

    // ============= Internal Helper Functions for Detail Fetching =============

    /// Internal: List rules for a specific event bus
    async fn list_rules_for_bus_internal(
        &self,
        client: &eventbridge::Client,
        event_bus_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.list_rules().event_bus_name(event_bus_name).send(),
        )
        .await
        .with_context(|| "list_rules timed out")?
        .with_context(|| "Failed to list rules")?;

        let mut rules = Vec::new();
        if let Some(rule_list) = response.rules {
            for rule in rule_list {
                rules.push(self.rule_to_json(&rule));
            }
        }

        Ok(serde_json::Value::Array(rules))
    }

    /// Internal: List targets for a specific rule
    async fn list_targets_by_rule_internal(
        &self,
        client: &eventbridge::Client,
        rule_name: &str,
        event_bus_name: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .list_targets_by_rule()
                .rule(rule_name)
                .event_bus_name(event_bus_name)
                .send(),
        )
        .await
        .with_context(|| "list_targets_by_rule timed out")?
        .with_context(|| "Failed to list targets")?;

        let mut targets = Vec::new();
        if let Some(target_list) = response.targets {
            for target in target_list {
                let mut t_json = serde_json::Map::new();

                // Required fields (not Optional in SDK)
                t_json.insert(
                    "Id".to_string(),
                    serde_json::Value::String(target.id.clone()),
                );
                t_json.insert(
                    "Arn".to_string(),
                    serde_json::Value::String(target.arn.clone()),
                );

                if let Some(role_arn) = &target.role_arn {
                    t_json.insert(
                        "RoleArn".to_string(),
                        serde_json::Value::String(role_arn.clone()),
                    );
                }

                if let Some(input) = &target.input {
                    t_json.insert(
                        "Input".to_string(),
                        serde_json::Value::String(input.clone()),
                    );
                }

                if let Some(input_path) = &target.input_path {
                    t_json.insert(
                        "InputPath".to_string(),
                        serde_json::Value::String(input_path.clone()),
                    );
                }

                // Input transformer
                if let Some(input_transformer) = &target.input_transformer {
                    let mut it_json = serde_json::Map::new();
                    // input_template is required in InputTransformer
                    it_json.insert(
                        "InputTemplate".to_string(),
                        serde_json::Value::String(input_transformer.input_template.clone()),
                    );
                    if let Some(input_paths_map) = &input_transformer.input_paths_map {
                        let paths: serde_json::Map<String, serde_json::Value> = input_paths_map
                            .iter()
                            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                            .collect();
                        it_json.insert(
                            "InputPathsMap".to_string(),
                            serde_json::Value::Object(paths),
                        );
                    }
                    t_json.insert(
                        "InputTransformer".to_string(),
                        serde_json::Value::Object(it_json),
                    );
                }

                // Retry policy
                if let Some(retry_policy) = &target.retry_policy {
                    let mut rp_json = serde_json::Map::new();
                    if let Some(max_age) = retry_policy.maximum_event_age_in_seconds {
                        rp_json.insert(
                            "MaximumEventAgeInSeconds".to_string(),
                            serde_json::Value::Number(max_age.into()),
                        );
                    }
                    if let Some(max_retry) = retry_policy.maximum_retry_attempts {
                        rp_json.insert(
                            "MaximumRetryAttempts".to_string(),
                            serde_json::Value::Number(max_retry.into()),
                        );
                    }
                    t_json.insert(
                        "RetryPolicy".to_string(),
                        serde_json::Value::Object(rp_json),
                    );
                }

                // Dead letter config
                if let Some(dlc) = &target.dead_letter_config {
                    if let Some(arn) = &dlc.arn {
                        let mut dlc_json = serde_json::Map::new();
                        dlc_json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
                        t_json.insert(
                            "DeadLetterConfig".to_string(),
                            serde_json::Value::Object(dlc_json),
                        );
                    }
                }

                targets.push(serde_json::Value::Object(t_json));
            }
        }

        Ok(serde_json::Value::Array(targets))
    }

    /// Internal: List archives for an event bus
    async fn list_archives_internal(
        &self,
        client: &eventbridge::Client,
        event_source_arn: &str,
    ) -> Result<serde_json::Value> {
        // Note: list_archives uses event_source_arn not event_bus_name
        // For the default event bus, we may need to construct the ARN
        let response = timeout(Duration::from_secs(10), client.list_archives().send())
            .await
            .with_context(|| "list_archives timed out")?
            .with_context(|| "Failed to list archives")?;

        let mut archives = Vec::new();
        if let Some(archive_list) = response.archives {
            for archive in archive_list {
                // Filter by event source if not default bus
                if event_source_arn != "default" {
                    if let Some(source_arn) = &archive.event_source_arn {
                        if !source_arn.contains(event_source_arn) {
                            continue;
                        }
                    }
                }

                let mut a_json = serde_json::Map::new();

                if let Some(name) = &archive.archive_name {
                    a_json.insert(
                        "ArchiveName".to_string(),
                        serde_json::Value::String(name.clone()),
                    );
                }

                if let Some(source_arn) = &archive.event_source_arn {
                    a_json.insert(
                        "EventSourceArn".to_string(),
                        serde_json::Value::String(source_arn.clone()),
                    );
                }

                if let Some(state) = &archive.state {
                    a_json.insert(
                        "State".to_string(),
                        serde_json::Value::String(state.as_str().to_string()),
                    );
                }

                if let Some(retention_days) = archive.retention_days {
                    a_json.insert(
                        "RetentionDays".to_string(),
                        serde_json::Value::Number(retention_days.into()),
                    );
                }

                a_json.insert(
                    "EventCount".to_string(),
                    serde_json::Value::Number(archive.event_count.into()),
                );

                a_json.insert(
                    "SizeBytes".to_string(),
                    serde_json::Value::Number(archive.size_bytes.into()),
                );

                if let Some(creation_time) = archive.creation_time {
                    a_json.insert(
                        "CreationTime".to_string(),
                        serde_json::Value::String(creation_time.to_string()),
                    );
                }

                archives.push(serde_json::Value::Object(a_json));
            }
        }

        Ok(serde_json::Value::Array(archives))
    }
}
