use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_shield as shield;
use std::sync::Arc;

pub struct ShieldService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl ShieldService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List Shield Protections
    pub async fn list_protections(
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

        let client = shield::Client::new(&aws_config);

        // Shield operates globally but resources may be region-specific
        let mut paginator = client.list_protections().into_paginator().send();

        let mut protections = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(protection_list) = page.protections {
                for protection in protection_list {
                    let protection_json = self.protection_to_json(&protection);
                    protections.push(protection_json);
                }
            }
        }

        Ok(protections)
    }

    /// List Shield Subscriptions
    pub async fn list_subscriptions(
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

        let client = shield::Client::new(&aws_config);

        // Get subscription information
        match client.describe_subscription().send().await {
            Ok(response) => {
                let mut subscriptions = Vec::new();
                if let Some(subscription) = response.subscription {
                    let subscription_json = self.subscription_to_json(&subscription);
                    subscriptions.push(subscription_json);
                }
                Ok(subscriptions)
            }
            Err(e) => {
                // If no subscription exists, return empty list
                log::debug!("No Shield subscription found: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Get detailed information for specific protection
    pub async fn describe_protection(
        &self,
        account_id: &str,
        region: &str,
        protection_id: &str,
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

        let client = shield::Client::new(&aws_config);
        let response = client
            .describe_protection()
            .protection_id(protection_id)
            .send()
            .await?;

        if let Some(protection) = response.protection {
            Ok(self.protection_to_json(&protection))
        } else {
            Err(anyhow::anyhow!("Protection {} not found", protection_id))
        }
    }

    /// List Shield attacks (for Advanced subscribers)
    pub async fn list_attacks(
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

        let client = shield::Client::new(&aws_config);

        // Get recent attacks (last 1 year by default)
        let end_time = chrono::Utc::now();
        let start_time = end_time - chrono::Duration::days(365);

        // Create TimeRange structs for Shield's API requirements
        let start_time_range = shield::types::TimeRange::builder()
            .from_inclusive(aws_smithy_types::DateTime::from_secs(
                start_time.timestamp(),
            ))
            .build();

        let end_time_range = shield::types::TimeRange::builder()
            .to_exclusive(aws_smithy_types::DateTime::from_secs(end_time.timestamp()))
            .build();

        let mut paginator = client
            .list_attacks()
            .start_time(start_time_range)
            .end_time(end_time_range)
            .into_paginator()
            .send();

        let mut attacks = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(attack_summaries) = page.attack_summaries {
                for attack in attack_summaries {
                    let attack_json = self.attack_to_json(&attack);
                    attacks.push(attack_json);
                }
            }
        }

        Ok(attacks)
    }

    fn protection_to_json(&self, protection: &shield::types::Protection) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(id) = &protection.id {
            json.insert("Id".to_string(), serde_json::Value::String(id.clone()));
            json.insert(
                "ProtectionId".to_string(),
                serde_json::Value::String(id.clone()),
            );
            json.insert("Name".to_string(), serde_json::Value::String(id.clone()));
        }

        if let Some(name) = &protection.name {
            json.insert(
                "ProtectionName".to_string(),
                serde_json::Value::String(name.clone()),
            );
            // Override the Id-based name with actual protection name if available
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
        }

        if let Some(resource_arn) = &protection.resource_arn {
            json.insert(
                "ResourceArn".to_string(),
                serde_json::Value::String(resource_arn.clone()),
            );

            // Extract resource type from ARN for better categorization
            if let Some(resource_type) = Self::extract_resource_type_from_arn(resource_arn) {
                json.insert(
                    "ResourceType".to_string(),
                    serde_json::Value::String(resource_type),
                );
            }
        }

        if let Some(health_check_ids) = &protection.health_check_ids {
            if !health_check_ids.is_empty() {
                let health_checks: Vec<serde_json::Value> = health_check_ids
                    .iter()
                    .map(|id| serde_json::Value::String(id.clone()))
                    .collect();
                json.insert(
                    "HealthCheckIds".to_string(),
                    serde_json::Value::Array(health_checks),
                );
            }
        }

        if let Some(protection_arn) = &protection.protection_arn {
            json.insert(
                "ProtectionArn".to_string(),
                serde_json::Value::String(protection_arn.clone()),
            );
        }

        // Default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn subscription_to_json(
        &self,
        subscription: &shield::types::Subscription,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(start_time) = subscription.start_time {
            json.insert(
                "StartTime".to_string(),
                serde_json::Value::String(start_time.to_string()),
            );
        }

        if let Some(end_time) = subscription.end_time {
            json.insert(
                "EndTime".to_string(),
                serde_json::Value::String(end_time.to_string()),
            );
        }

        json.insert(
            "TimeCommitmentInSeconds".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                subscription.time_commitment_in_seconds,
            )),
        );

        if let Some(auto_renew) = &subscription.auto_renew {
            json.insert(
                "AutoRenew".to_string(),
                serde_json::Value::String(auto_renew.as_str().to_string()),
            );
        }

        if let Some(limits) = &subscription.limits {
            if !limits.is_empty() {
                let limits_json: Vec<serde_json::Value> = limits
                    .iter()
                    .map(|limit| {
                        let mut limit_obj = serde_json::Map::new();
                        if let Some(limit_type) = &limit.r#type {
                            limit_obj.insert(
                                "Type".to_string(),
                                serde_json::Value::String(limit_type.clone()),
                            );
                        }
                        limit_obj.insert(
                            "Max".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(limit.max)),
                        );
                        serde_json::Value::Object(limit_obj)
                    })
                    .collect();
                json.insert("Limits".to_string(), serde_json::Value::Array(limits_json));
            }
        }

        // Generate a unique identifier for the subscription
        json.insert(
            "SubscriptionId".to_string(),
            serde_json::Value::String("shield-advanced-subscription".to_string()),
        );
        json.insert(
            "Name".to_string(),
            serde_json::Value::String("Shield Advanced Subscription".to_string()),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn attack_to_json(&self, attack: &shield::types::AttackSummary) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(attack_id) = &attack.attack_id {
            json.insert(
                "AttackId".to_string(),
                serde_json::Value::String(attack_id.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(format!("Attack {}", attack_id)),
            );
        }

        if let Some(resource_arn) = &attack.resource_arn {
            json.insert(
                "ResourceArn".to_string(),
                serde_json::Value::String(resource_arn.clone()),
            );
        }

        if let Some(start_time) = attack.start_time {
            json.insert(
                "StartTime".to_string(),
                serde_json::Value::String(start_time.to_string()),
            );
        }

        if let Some(end_time) = attack.end_time {
            json.insert(
                "EndTime".to_string(),
                serde_json::Value::String(end_time.to_string()),
            );
        }

        if let Some(attack_vectors) = &attack.attack_vectors {
            if !attack_vectors.is_empty() {
                let vectors_json: Vec<serde_json::Value> = attack_vectors
                    .iter()
                    .map(|vector| {
                        let mut vector_obj = serde_json::Map::new();
                        vector_obj.insert(
                            "VectorType".to_string(),
                            serde_json::Value::String(vector.vector_type.clone()),
                        );
                        serde_json::Value::Object(vector_obj)
                    })
                    .collect();
                json.insert(
                    "AttackVectors".to_string(),
                    serde_json::Value::Array(vectors_json),
                );
            }
        }

        // Default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Completed".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn extract_resource_type_from_arn(arn: &str) -> Option<String> {
        // Parse ARN format: arn:partition:service:region:account:resource-type/resource-id
        let parts: Vec<&str> = arn.split(':').collect();
        if parts.len() >= 6 {
            let service = parts[2];
            let resource_part = parts[5];

            // Extract resource type from the resource part
            if let Some(slash_pos) = resource_part.find('/') {
                let resource_type = &resource_part[..slash_pos];
                return Some(format!("{}/{}", service, resource_type));
            }
        }
        None
    }
}
