use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_xray as xray;
use std::sync::Arc;

pub struct XRayService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl XRayService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List X-Ray Sampling Rules
    pub async fn list_sampling_rules(
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

        let client = xray::Client::new(&aws_config);
        
        // Note: X-Ray GetSamplingRules doesn't have pagination
        let response = client.get_sampling_rules().send().await?;

        let mut rules = Vec::new();
        if let Some(sampling_rule_records) = response.sampling_rule_records {
            for rule_record in sampling_rule_records {
                let rule_json = self.sampling_rule_to_json(&rule_record);
                rules.push(rule_json);
            }
        }

        Ok(rules)
    }

    /// List X-Ray Service Map services
    pub async fn list_services(
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

        let client = xray::Client::new(&aws_config);
        
        // Get services from the last hour by default
        let end_time = chrono::Utc::now();
        let start_time = end_time - chrono::Duration::hours(1);

        let mut paginator = client
            .get_service_graph()
            .start_time(aws_smithy_types::DateTime::from_secs(start_time.timestamp()))
            .end_time(aws_smithy_types::DateTime::from_secs(end_time.timestamp()))
            .into_paginator()
            .send();

        let mut services = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(service_list) = page.services {
                for service in service_list {
                    let service_json = self.service_to_json(&service);
                    services.push(service_json);
                }
            }
        }

        Ok(services)
    }

    /// Get detailed information for specific sampling rule
    pub async fn describe_sampling_rule(
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

        let client = xray::Client::new(&aws_config);
        let response = client
            .get_sampling_rules()
            .send()
            .await?;

        if let Some(sampling_rule_records) = response.sampling_rule_records {
            for rule_record in sampling_rule_records {
                if let Some(sampling_rule) = &rule_record.sampling_rule {
                    if let Some(rule_name_field) = &sampling_rule.rule_name {
                        if rule_name_field == rule_name {
                            return Ok(self.sampling_rule_to_json(&rule_record));
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Sampling rule {} not found", rule_name))
    }

    fn sampling_rule_to_json(&self, rule_record: &xray::types::SamplingRuleRecord) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(sampling_rule) = &rule_record.sampling_rule {
            if let Some(rule_name) = &sampling_rule.rule_name {
                json.insert(
                    "RuleName".to_string(),
                    serde_json::Value::String(rule_name.clone()),
                );
                json.insert(
                    "Name".to_string(),
                    serde_json::Value::String(rule_name.clone()),
                );
            }

            if let Some(rule_arn) = &sampling_rule.rule_arn {
                json.insert(
                    "RuleARN".to_string(),
                    serde_json::Value::String(rule_arn.clone()),
                );
            }

            json.insert(
                "ServiceName".to_string(),
                serde_json::Value::String(sampling_rule.service_name.clone()),
            );

            json.insert(
                "ServiceType".to_string(),
                serde_json::Value::String(sampling_rule.service_type.clone()),
            );

            json.insert(
                "Host".to_string(),
                serde_json::Value::String(sampling_rule.host.clone()),
            );

            json.insert(
                "HTTPMethod".to_string(),
                serde_json::Value::String(sampling_rule.http_method.clone()),
            );

            json.insert(
                "URLPath".to_string(),
                serde_json::Value::String(sampling_rule.url_path.clone()),
            );

            json.insert(
                "Version".to_string(),
                serde_json::Value::Number(serde_json::Number::from(sampling_rule.version)),
            );

            json.insert(
                "Priority".to_string(),
                serde_json::Value::Number(serde_json::Number::from(sampling_rule.priority)),
            );

            json.insert(
                "FixedRate".to_string(),
                serde_json::Value::Number(
                    serde_json::Number::from_f64(sampling_rule.fixed_rate).unwrap_or(serde_json::Number::from(0))
                ),
            );

            json.insert(
                "ReservoirSize".to_string(),
                serde_json::Value::Number(serde_json::Number::from(sampling_rule.reservoir_size)),
            );

            json.insert(
                "ResourceARN".to_string(),
                serde_json::Value::String(sampling_rule.resource_arn.clone()),
            );

            if let Some(attributes) = &sampling_rule.attributes {
                if !attributes.is_empty() {
                    let attrs_json: serde_json::Map<String, serde_json::Value> = attributes
                        .iter()
                        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                        .collect();
                    json.insert(
                        "Attributes".to_string(),
                        serde_json::Value::Object(attrs_json),
                    );
                }
            }
        }

        if let Some(created_at) = rule_record.created_at {
            json.insert(
                "CreatedAt".to_string(),
                serde_json::Value::String(created_at.to_string()),
            );
        }

        if let Some(modified_at) = rule_record.modified_at {
            json.insert(
                "ModifiedAt".to_string(),
                serde_json::Value::String(modified_at.to_string()),
            );
        }

        // Default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn service_to_json(&self, service: &xray::types::Service) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(reference_id) = service.reference_id {
            json.insert(
                "ReferenceId".to_string(),
                serde_json::Value::Number(serde_json::Number::from(reference_id)),
            );
        }

        if let Some(name) = &service.name {
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(service_names) = &service.names {
            if !service_names.is_empty() {
                let names_json: Vec<serde_json::Value> = service_names
                    .iter()
                    .map(|name| serde_json::Value::String(name.clone()))
                    .collect();
                json.insert(
                    "Names".to_string(),
                    serde_json::Value::Array(names_json),
                );
            }
        }

        if let Some(r#type) = &service.r#type {
            json.insert(
                "Type".to_string(),
                serde_json::Value::String(r#type.clone()),
            );
        }

        if let Some(state) = &service.state {
            json.insert(
                "State".to_string(),
                serde_json::Value::String(state.clone()),
            );
        }

        if let Some(start_time) = service.start_time {
            json.insert(
                "StartTime".to_string(),
                serde_json::Value::String(start_time.to_string()),
            );
        }

        if let Some(end_time) = service.end_time {
            json.insert(
                "EndTime".to_string(),
                serde_json::Value::String(end_time.to_string()),
            );
        }

        if let Some(root) = service.root {
            json.insert(
                "Root".to_string(),
                serde_json::Value::Bool(root),
            );
        }

        // Default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }
}