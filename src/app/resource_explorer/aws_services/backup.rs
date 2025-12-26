use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_backup as backup;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct BackupService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl BackupService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List backup plans
    pub async fn list_backup_plans(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
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

        let client = backup::Client::new(&aws_config);
        let mut paginator = client.list_backup_plans().into_paginator().send();

        let mut backup_plans = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(backup_plans_list) = page.backup_plans_list {
                for backup_plan in backup_plans_list {
                    let mut backup_plan_json = self.backup_plan_list_member_to_json(&backup_plan);

                    if include_details {
                        if let Some(backup_plan_id) = &backup_plan.backup_plan_id {
                            report_status(
                                "Backup",
                                "get_backup_plan_details",
                                Some(backup_plan_id),
                            );

                            // Get detailed plan info
                            if let Ok(plan_detail) =
                                self.get_backup_plan_internal(&client, backup_plan_id).await
                            {
                                if let serde_json::Value::Object(ref mut map) = backup_plan_json {
                                    if let serde_json::Value::Object(details) = plan_detail {
                                        for (k, v) in details {
                                            map.insert(k, v);
                                        }
                                    }
                                }
                            }

                            // Get backup selections
                            if let Ok(selections) = self
                                .get_backup_selections_internal(&client, backup_plan_id)
                                .await
                            {
                                if let serde_json::Value::Object(ref mut map) = backup_plan_json {
                                    map.insert("BackupSelections".to_string(), selections);
                                }
                            }

                            report_status_done(
                                "Backup",
                                "get_backup_plan_details",
                                Some(backup_plan_id),
                            );
                        }
                    }

                    backup_plans.push(backup_plan_json);
                }
            }
        }

        Ok(backup_plans)
    }

    /// Get detailed information for specific backup plan
    pub async fn describe_backup_plan(
        &self,
        account_id: &str,
        region: &str,
        backup_plan_id: &str,
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

        let client = backup::Client::new(&aws_config);
        let response = client
            .get_backup_plan()
            .backup_plan_id(backup_plan_id)
            .send()
            .await?;

        let mut backup_plan_details = serde_json::Map::new();

        if let Some(backup_plan) = response.backup_plan {
            let plan_json = self.backup_plan_to_json(&backup_plan);
            backup_plan_details.insert("BackupPlan".to_string(), plan_json);
        }

        if let Some(backup_plan_id) = response.backup_plan_id {
            backup_plan_details.insert(
                "BackupPlanId".to_string(),
                serde_json::Value::String(backup_plan_id),
            );
        }

        if let Some(backup_plan_arn) = response.backup_plan_arn {
            backup_plan_details.insert(
                "BackupPlanArn".to_string(),
                serde_json::Value::String(backup_plan_arn),
            );
        }

        if let Some(version_id) = response.version_id {
            backup_plan_details.insert(
                "VersionId".to_string(),
                serde_json::Value::String(version_id),
            );
        }

        if let Some(creator_request_id) = response.creator_request_id {
            backup_plan_details.insert(
                "CreatorRequestId".to_string(),
                serde_json::Value::String(creator_request_id),
            );
        }

        if let Some(creation_date) = response.creation_date {
            backup_plan_details.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(last_execution_date) = response.last_execution_date {
            backup_plan_details.insert(
                "LastExecutionDate".to_string(),
                serde_json::Value::String(last_execution_date.to_string()),
            );
        }

        Ok(serde_json::Value::Object(backup_plan_details))
    }

    /// List backup vaults
    pub async fn list_backup_vaults(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
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

        let client = backup::Client::new(&aws_config);
        let mut paginator = client.list_backup_vaults().into_paginator().send();

        let mut backup_vaults = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(backup_vault_list) = page.backup_vault_list {
                for backup_vault in backup_vault_list {
                    let mut backup_vault_json =
                        self.backup_vault_list_member_to_json(&backup_vault);

                    if include_details {
                        if let Some(vault_name) = &backup_vault.backup_vault_name {
                            report_status("Backup", "get_backup_vault_details", Some(vault_name));

                            // Get vault access policy
                            if let Ok(policy) = self
                                .get_vault_access_policy_internal(&client, vault_name)
                                .await
                            {
                                if let serde_json::Value::Object(ref mut map) = backup_vault_json {
                                    map.insert("AccessPolicy".to_string(), policy);
                                }
                            }

                            // Get recovery points (limit to 20)
                            if let Ok(recovery_points) = self
                                .list_recovery_points_internal(&client, vault_name)
                                .await
                            {
                                if let serde_json::Value::Object(ref mut map) = backup_vault_json {
                                    map.insert("RecoveryPoints".to_string(), recovery_points);
                                }
                            }

                            report_status_done(
                                "Backup",
                                "get_backup_vault_details",
                                Some(vault_name),
                            );
                        }
                    }

                    backup_vaults.push(backup_vault_json);
                }
            }
        }

        Ok(backup_vaults)
    }

    /// Get detailed information for specific backup vault
    pub async fn describe_backup_vault(
        &self,
        account_id: &str,
        region: &str,
        backup_vault_name: &str,
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

        let client = backup::Client::new(&aws_config);
        let response = client
            .describe_backup_vault()
            .backup_vault_name(backup_vault_name)
            .send()
            .await?;

        let mut vault_details = serde_json::Map::new();

        if let Some(backup_vault_name) = response.backup_vault_name {
            vault_details.insert(
                "BackupVaultName".to_string(),
                serde_json::Value::String(backup_vault_name),
            );
        }

        if let Some(backup_vault_arn) = response.backup_vault_arn {
            vault_details.insert(
                "BackupVaultArn".to_string(),
                serde_json::Value::String(backup_vault_arn),
            );
        }

        if let Some(encryption_key_arn) = response.encryption_key_arn {
            vault_details.insert(
                "EncryptionKeyArn".to_string(),
                serde_json::Value::String(encryption_key_arn),
            );
        }

        if let Some(creation_date) = response.creation_date {
            vault_details.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(creator_request_id) = response.creator_request_id {
            vault_details.insert(
                "CreatorRequestId".to_string(),
                serde_json::Value::String(creator_request_id),
            );
        }

        let number_of_recovery_points = response.number_of_recovery_points;
        if number_of_recovery_points > 0 {
            vault_details.insert(
                "NumberOfRecoveryPoints".to_string(),
                serde_json::Value::Number(number_of_recovery_points.into()),
            );
        }

        if let Some(locked) = response.locked {
            vault_details.insert("Locked".to_string(), serde_json::Value::Bool(locked));
        }

        if let Some(min_retention_days) = response.min_retention_days {
            vault_details.insert(
                "MinRetentionDays".to_string(),
                serde_json::Value::Number(min_retention_days.into()),
            );
        }

        if let Some(max_retention_days) = response.max_retention_days {
            vault_details.insert(
                "MaxRetentionDays".to_string(),
                serde_json::Value::Number(max_retention_days.into()),
            );
        }

        if let Some(lock_date) = response.lock_date {
            vault_details.insert(
                "LockDate".to_string(),
                serde_json::Value::String(lock_date.to_string()),
            );
        }

        Ok(serde_json::Value::Object(vault_details))
    }

    fn backup_plan_list_member_to_json(
        &self,
        backup_plan: &backup::types::BackupPlansListMember,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(backup_plan_arn) = &backup_plan.backup_plan_arn {
            json.insert(
                "BackupPlanArn".to_string(),
                serde_json::Value::String(backup_plan_arn.clone()),
            );
        }

        if let Some(backup_plan_id) = &backup_plan.backup_plan_id {
            json.insert(
                "BackupPlanId".to_string(),
                serde_json::Value::String(backup_plan_id.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(backup_plan_id.clone()),
            );
        }

        if let Some(backup_plan_name) = &backup_plan.backup_plan_name {
            json.insert(
                "BackupPlanName".to_string(),
                serde_json::Value::String(backup_plan_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(backup_plan_name.clone()),
            );
        }

        if let Some(creation_date) = backup_plan.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(last_execution_date) = backup_plan.last_execution_date {
            json.insert(
                "LastExecutionDate".to_string(),
                serde_json::Value::String(last_execution_date.to_string()),
            );
        }

        if let Some(version_id) = &backup_plan.version_id {
            json.insert(
                "VersionId".to_string(),
                serde_json::Value::String(version_id.clone()),
            );
        }

        if let Some(creator_request_id) = &backup_plan.creator_request_id {
            json.insert(
                "CreatorRequestId".to_string(),
                serde_json::Value::String(creator_request_id.clone()),
            );
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Active".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn backup_plan_to_json(&self, backup_plan: &backup::types::BackupPlan) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        let backup_plan_name = &backup_plan.backup_plan_name;
        if !backup_plan_name.is_empty() {
            json.insert(
                "BackupPlanName".to_string(),
                serde_json::Value::String(backup_plan_name.clone()),
            );
        }

        let rules = &backup_plan.rules;
        if !rules.is_empty() {
            let rules_array: Vec<serde_json::Value> = rules
                .iter()
                .map(|rule| {
                    let mut rule_json = serde_json::Map::new();
                    let rule_name = &rule.rule_name;
                    if !rule_name.is_empty() {
                        rule_json.insert(
                            "RuleName".to_string(),
                            serde_json::Value::String(rule_name.clone()),
                        );
                    }
                    let target_backup_vault = &rule.target_backup_vault_name;
                    if !target_backup_vault.is_empty() {
                        rule_json.insert(
                            "TargetBackupVault".to_string(),
                            serde_json::Value::String(target_backup_vault.clone()),
                        );
                    }
                    if let Some(schedule_expression) = &rule.schedule_expression {
                        rule_json.insert(
                            "ScheduleExpression".to_string(),
                            serde_json::Value::String(schedule_expression.clone()),
                        );
                    }
                    if let Some(start_window_minutes) = rule.start_window_minutes {
                        rule_json.insert(
                            "StartWindowMinutes".to_string(),
                            serde_json::Value::Number(start_window_minutes.into()),
                        );
                    }
                    if let Some(completion_window_minutes) = rule.completion_window_minutes {
                        rule_json.insert(
                            "CompletionWindowMinutes".to_string(),
                            serde_json::Value::Number(completion_window_minutes.into()),
                        );
                    }
                    if let Some(lifecycle) = &rule.lifecycle {
                        let mut lifecycle_json = serde_json::Map::new();
                        if let Some(move_to_cold_storage_after_days) =
                            lifecycle.move_to_cold_storage_after_days
                        {
                            lifecycle_json.insert(
                                "MoveToColdStorageAfterDays".to_string(),
                                serde_json::Value::Number(move_to_cold_storage_after_days.into()),
                            );
                        }
                        if let Some(delete_after_days) = lifecycle.delete_after_days {
                            lifecycle_json.insert(
                                "DeleteAfterDays".to_string(),
                                serde_json::Value::Number(delete_after_days.into()),
                            );
                        }
                        rule_json.insert(
                            "Lifecycle".to_string(),
                            serde_json::Value::Object(lifecycle_json),
                        );
                    }
                    serde_json::Value::Object(rule_json)
                })
                .collect();
            json.insert("Rules".to_string(), serde_json::Value::Array(rules_array));
        }

        if let Some(advanced_backup_settings) = &backup_plan.advanced_backup_settings {
            let settings_array: Vec<serde_json::Value> = advanced_backup_settings
                .iter()
                .map(|setting| {
                    let mut setting_json = serde_json::Map::new();
                    if let Some(resource_type) = &setting.resource_type {
                        setting_json.insert(
                            "ResourceType".to_string(),
                            serde_json::Value::String(resource_type.clone()),
                        );
                    }
                    if let Some(backup_options) = &setting.backup_options {
                        let options_map: serde_json::Map<String, serde_json::Value> =
                            backup_options
                                .iter()
                                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                                .collect();
                        setting_json.insert(
                            "BackupOptions".to_string(),
                            serde_json::Value::Object(options_map),
                        );
                    }
                    serde_json::Value::Object(setting_json)
                })
                .collect();
            json.insert(
                "AdvancedBackupSettings".to_string(),
                serde_json::Value::Array(settings_array),
            );
        }

        serde_json::Value::Object(json)
    }

    fn backup_vault_list_member_to_json(
        &self,
        backup_vault: &backup::types::BackupVaultListMember,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(backup_vault_name) = &backup_vault.backup_vault_name {
            json.insert(
                "BackupVaultName".to_string(),
                serde_json::Value::String(backup_vault_name.clone()),
            );
            json.insert(
                "Name".to_string(),
                serde_json::Value::String(backup_vault_name.clone()),
            );
        }

        if let Some(backup_vault_arn) = &backup_vault.backup_vault_arn {
            json.insert(
                "BackupVaultArn".to_string(),
                serde_json::Value::String(backup_vault_arn.clone()),
            );
        }

        if let Some(encryption_key_arn) = &backup_vault.encryption_key_arn {
            json.insert(
                "EncryptionKeyArn".to_string(),
                serde_json::Value::String(encryption_key_arn.clone()),
            );
        }

        if let Some(creation_date) = backup_vault.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(creator_request_id) = &backup_vault.creator_request_id {
            json.insert(
                "CreatorRequestId".to_string(),
                serde_json::Value::String(creator_request_id.clone()),
            );
        }

        let number_of_recovery_points = backup_vault.number_of_recovery_points;
        if number_of_recovery_points > 0 {
            json.insert(
                "NumberOfRecoveryPoints".to_string(),
                serde_json::Value::Number(number_of_recovery_points.into()),
            );
        }

        if let Some(locked) = backup_vault.locked {
            json.insert("Locked".to_string(), serde_json::Value::Bool(locked));
        }

        // Set default status
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("Available".to_string()),
        );

        serde_json::Value::Object(json)
    }

    /// Get detailed information for a backup plan (Phase 2 enrichment)
    pub async fn get_backup_plan_details(
        &self,
        account_id: &str,
        region: &str,
        backup_plan_id: &str,
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

        let client = backup::Client::new(&aws_config);

        let mut details = serde_json::Map::new();

        // Get detailed plan info
        if let Ok(serde_json::Value::Object(plan_obj)) =
            self.get_backup_plan_internal(&client, backup_plan_id).await
        {
            for (k, v) in plan_obj {
                details.insert(k, v);
            }
        }

        // Get backup selections
        if let Ok(selections) = self
            .get_backup_selections_internal(&client, backup_plan_id)
            .await
        {
            details.insert("BackupSelections".to_string(), selections);
        }

        Ok(serde_json::Value::Object(details))
    }

    /// Get detailed information for a backup vault (Phase 2 enrichment)
    pub async fn get_backup_vault_details(
        &self,
        account_id: &str,
        region: &str,
        backup_vault_name: &str,
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

        let client = backup::Client::new(&aws_config);

        let mut details = serde_json::Map::new();

        // Get vault access policy
        if let Ok(policy) = self
            .get_vault_access_policy_internal(&client, backup_vault_name)
            .await
        {
            details.insert("AccessPolicy".to_string(), policy);
        }

        // Get recovery points (limit to 20)
        if let Ok(recovery_points) = self
            .list_recovery_points_internal(&client, backup_vault_name)
            .await
        {
            details.insert("RecoveryPoints".to_string(), recovery_points);
        }

        Ok(serde_json::Value::Object(details))
    }

    /// Internal helper to get backup plan details
    async fn get_backup_plan_internal(
        &self,
        client: &backup::Client,
        backup_plan_id: &str,
    ) -> Result<serde_json::Value> {
        let timeout_duration = Duration::from_secs(30);

        let result = timeout(
            timeout_duration,
            client
                .get_backup_plan()
                .backup_plan_id(backup_plan_id)
                .send(),
        )
        .await
        .with_context(|| format!("Timeout getting backup plan {}", backup_plan_id))?
        .with_context(|| format!("Failed to get backup plan {}", backup_plan_id))?;

        let mut details = serde_json::Map::new();

        if let Some(backup_plan) = result.backup_plan {
            let plan_json = self.backup_plan_to_json(&backup_plan);
            details.insert("BackupPlan".to_string(), plan_json);
        }

        if let Some(backup_plan_arn) = result.backup_plan_arn {
            details.insert(
                "BackupPlanArn".to_string(),
                serde_json::Value::String(backup_plan_arn),
            );
        }

        if let Some(version_id) = result.version_id {
            details.insert(
                "VersionId".to_string(),
                serde_json::Value::String(version_id),
            );
        }

        if let Some(creation_date) = result.creation_date {
            details.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(last_execution_date) = result.last_execution_date {
            details.insert(
                "LastExecutionDate".to_string(),
                serde_json::Value::String(last_execution_date.to_string()),
            );
        }

        Ok(serde_json::Value::Object(details))
    }

    /// Internal helper to get backup selections
    async fn get_backup_selections_internal(
        &self,
        client: &backup::Client,
        backup_plan_id: &str,
    ) -> Result<serde_json::Value> {
        let timeout_duration = Duration::from_secs(30);

        let result = timeout(
            timeout_duration,
            client
                .list_backup_selections()
                .backup_plan_id(backup_plan_id)
                .send(),
        )
        .await
        .with_context(|| format!("Timeout listing backup selections for {}", backup_plan_id))?
        .with_context(|| format!("Failed to list backup selections for {}", backup_plan_id))?;

        let selections: Vec<serde_json::Value> = result
            .backup_selections_list
            .unwrap_or_default()
            .into_iter()
            .map(|selection| {
                let mut sel_json = serde_json::Map::new();
                if let Some(selection_id) = selection.selection_id {
                    sel_json.insert(
                        "SelectionId".to_string(),
                        serde_json::Value::String(selection_id),
                    );
                }
                if let Some(selection_name) = selection.selection_name {
                    sel_json.insert(
                        "SelectionName".to_string(),
                        serde_json::Value::String(selection_name),
                    );
                }
                if let Some(iam_role_arn) = selection.iam_role_arn {
                    sel_json.insert(
                        "IamRoleArn".to_string(),
                        serde_json::Value::String(iam_role_arn),
                    );
                }
                if let Some(creation_date) = selection.creation_date {
                    sel_json.insert(
                        "CreationDate".to_string(),
                        serde_json::Value::String(creation_date.to_string()),
                    );
                }
                serde_json::Value::Object(sel_json)
            })
            .collect();

        Ok(serde_json::Value::Array(selections))
    }

    /// Internal helper to get vault access policy
    async fn get_vault_access_policy_internal(
        &self,
        client: &backup::Client,
        backup_vault_name: &str,
    ) -> Result<serde_json::Value> {
        let timeout_duration = Duration::from_secs(30);

        let result = timeout(
            timeout_duration,
            client
                .get_backup_vault_access_policy()
                .backup_vault_name(backup_vault_name)
                .send(),
        )
        .await
        .with_context(|| format!("Timeout getting access policy for {}", backup_vault_name))?
        .with_context(|| format!("Failed to get access policy for {}", backup_vault_name))?;

        let mut policy_json = serde_json::Map::new();

        if let Some(policy) = result.policy {
            // Try to parse as JSON
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&policy) {
                policy_json.insert("Policy".to_string(), parsed);
            } else {
                policy_json.insert("Policy".to_string(), serde_json::Value::String(policy));
            }
        }

        Ok(serde_json::Value::Object(policy_json))
    }

    /// Internal helper to list recovery points
    async fn list_recovery_points_internal(
        &self,
        client: &backup::Client,
        backup_vault_name: &str,
    ) -> Result<serde_json::Value> {
        let timeout_duration = Duration::from_secs(30);

        let result = timeout(
            timeout_duration,
            client
                .list_recovery_points_by_backup_vault()
                .backup_vault_name(backup_vault_name)
                .max_results(20)
                .send(),
        )
        .await
        .with_context(|| format!("Timeout listing recovery points for {}", backup_vault_name))?
        .with_context(|| format!("Failed to list recovery points for {}", backup_vault_name))?;

        let recovery_points: Vec<serde_json::Value> = result
            .recovery_points
            .unwrap_or_default()
            .into_iter()
            .map(|rp| self.recovery_point_to_json(&rp))
            .collect();

        Ok(serde_json::Value::Array(recovery_points))
    }

    /// Convert recovery point to JSON
    fn recovery_point_to_json(
        &self,
        recovery_point: &backup::types::RecoveryPointByBackupVault,
    ) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(recovery_point_arn) = &recovery_point.recovery_point_arn {
            json.insert(
                "RecoveryPointArn".to_string(),
                serde_json::Value::String(recovery_point_arn.clone()),
            );
        }

        if let Some(backup_vault_name) = &recovery_point.backup_vault_name {
            json.insert(
                "BackupVaultName".to_string(),
                serde_json::Value::String(backup_vault_name.clone()),
            );
        }

        if let Some(resource_arn) = &recovery_point.resource_arn {
            json.insert(
                "ResourceArn".to_string(),
                serde_json::Value::String(resource_arn.clone()),
            );
        }

        if let Some(resource_type) = &recovery_point.resource_type {
            json.insert(
                "ResourceType".to_string(),
                serde_json::Value::String(resource_type.clone()),
            );
        }

        if let Some(creation_date) = recovery_point.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        if let Some(status) = &recovery_point.status {
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(status.as_str().to_string()),
            );
        }

        if let Some(status_message) = &recovery_point.status_message {
            json.insert(
                "StatusMessage".to_string(),
                serde_json::Value::String(status_message.clone()),
            );
        }

        if let Some(backup_size_in_bytes) = recovery_point.backup_size_in_bytes {
            json.insert(
                "BackupSizeInBytes".to_string(),
                serde_json::Value::Number(backup_size_in_bytes.into()),
            );
        }

        if let Some(iam_role_arn) = &recovery_point.iam_role_arn {
            json.insert(
                "IamRoleArn".to_string(),
                serde_json::Value::String(iam_role_arn.clone()),
            );
        }

        if let Some(encryption_key_arn) = &recovery_point.encryption_key_arn {
            json.insert(
                "EncryptionKeyArn".to_string(),
                serde_json::Value::String(encryption_key_arn.clone()),
            );
        }

        json.insert(
            "IsEncrypted".to_string(),
            serde_json::Value::Bool(recovery_point.is_encrypted),
        );

        serde_json::Value::Object(json)
    }
}
