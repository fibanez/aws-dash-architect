use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
use anyhow::{Context, Result};
use aws_sdk_kms as kms;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct KmsService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl KmsService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List KMS Keys with optional detailed security information
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic key info quickly.
    ///   If true (Phase 2), includes policy, rotation status, grants, and aliases.
    pub async fn list_keys(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("KMS", "list_keys", Some(region));

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

        let client = kms::Client::new(&aws_config);
        let mut paginator = client.list_keys().into_paginator().send();

        let mut keys = Vec::new();
        while let Some(page) = paginator.next().await {
            let page = page?;
            if let Some(key_list) = page.keys {
                for key in key_list {
                    // Get detailed key information
                    if let Some(key_id) = &key.key_id {
                        let mut key_details = if let Ok(details) =
                            self.describe_key_internal(&client, key_id).await
                        {
                            details
                        } else {
                            // Fallback to basic key info if describe fails
                            self.key_list_entry_to_json(&key)
                        };

                        // Only fetch security details if requested (Phase 2)
                        if include_details {
                            if let serde_json::Value::Object(ref mut details) = key_details {
                                // Get key policy
                                report_status("KMS", "get_key_policy", Some(key_id));
                                match self.get_key_policy_internal(&client, key_id).await {
                                    Ok(policy) => {
                                        details.insert("KeyPolicy".to_string(), policy);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get key policy for {}: {}",
                                            key_id,
                                            e
                                        );
                                    }
                                }

                                // Get rotation status
                                report_status("KMS", "get_rotation_status", Some(key_id));
                                match self.get_key_rotation_status_internal(&client, key_id).await {
                                    Ok(rotation) => {
                                        details.insert("RotationStatus".to_string(), rotation);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not get rotation status for {}: {}",
                                            key_id,
                                            e
                                        );
                                    }
                                }

                                // List grants
                                report_status("KMS", "list_grants", Some(key_id));
                                match self.list_grants_internal(&client, key_id).await {
                                    Ok(grants) => {
                                        details.insert("Grants".to_string(), grants);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not list grants for {}: {}",
                                            key_id,
                                            e
                                        );
                                    }
                                }

                                // List aliases
                                report_status("KMS", "list_aliases", Some(key_id));
                                match self.list_aliases_internal(&client, key_id).await {
                                    Ok(aliases) => {
                                        details.insert("Aliases".to_string(), aliases);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            "Could not list aliases for {}: {}",
                                            key_id,
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        keys.push(key_details);
                    } else {
                        // Fallback to basic key info if no ID
                        let key_json = self.key_list_entry_to_json(&key);
                        keys.push(key_json);
                    }
                }
            }
        }

        report_status_done("KMS", "list_keys", Some(region));
        Ok(keys)
    }

    /// Get security details for a single KMS key (Phase 2 enrichment)
    ///
    /// This function fetches detailed security information for a single key,
    /// including key policy, rotation status, grants, and aliases.
    /// Used for incremental detail fetching after the initial fast list.
    pub async fn get_key_details(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
    ) -> Result<serde_json::Value> {
        report_status("KMS", "get_key_details", Some(key_id));

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

        let client = kms::Client::new(&aws_config);
        let mut details = serde_json::Map::new();

        // Get key policy
        report_status("KMS", "get_key_policy", Some(key_id));
        match self.get_key_policy_internal(&client, key_id).await {
            Ok(policy) => {
                details.insert("KeyPolicy".to_string(), policy);
            }
            Err(e) => {
                tracing::debug!("Could not get key policy for {}: {}", key_id, e);
            }
        }

        // Get rotation status
        report_status("KMS", "get_rotation_status", Some(key_id));
        match self.get_key_rotation_status_internal(&client, key_id).await {
            Ok(rotation) => {
                details.insert("RotationStatus".to_string(), rotation);
            }
            Err(e) => {
                tracing::debug!("Could not get rotation status for {}: {}", key_id, e);
            }
        }

        // List grants
        report_status("KMS", "list_grants", Some(key_id));
        match self.list_grants_internal(&client, key_id).await {
            Ok(grants) => {
                details.insert("Grants".to_string(), grants);
            }
            Err(e) => {
                tracing::debug!("Could not list grants for {}: {}", key_id, e);
            }
        }

        // List aliases
        report_status("KMS", "list_aliases", Some(key_id));
        match self.list_aliases_internal(&client, key_id).await {
            Ok(aliases) => {
                details.insert("Aliases".to_string(), aliases);
            }
            Err(e) => {
                tracing::debug!("Could not list aliases for {}: {}", key_id, e);
            }
        }

        report_status_done("KMS", "get_key_details", Some(key_id));
        Ok(serde_json::Value::Object(details))
    }

    /// Get detailed information for specific KMS key
    pub async fn describe_key(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
    ) -> Result<serde_json::Value> {
        report_status("KMS", "describe_key", Some(key_id));

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

        let client = kms::Client::new(&aws_config);
        let mut key_details = self.describe_key_internal(&client, key_id).await?;

        // Add security details to the key info
        if let serde_json::Value::Object(ref mut details) = key_details {
            // Get key policy
            report_status("KMS", "get_key_policy", Some(key_id));
            match self.get_key_policy(account_id, region, key_id).await {
                Ok(policy) => {
                    details.insert("KeyPolicy".to_string(), policy);
                }
                Err(e) => {
                    tracing::debug!("Could not get key policy: {}", e);
                }
            }

            // Get rotation status
            report_status("KMS", "get_rotation_status", Some(key_id));
            match self
                .get_key_rotation_status(account_id, region, key_id)
                .await
            {
                Ok(rotation) => {
                    details.insert("RotationStatus".to_string(), rotation);
                }
                Err(e) => {
                    tracing::debug!("Could not get rotation status: {}", e);
                }
            }

            // List grants
            report_status("KMS", "list_grants", Some(key_id));
            match self.list_grants(account_id, region, key_id).await {
                Ok(grants) => {
                    details.insert("Grants".to_string(), grants);
                }
                Err(e) => {
                    tracing::debug!("Could not list grants: {}", e);
                }
            }

            // List aliases
            report_status("KMS", "list_aliases", Some(key_id));
            match self.list_aliases(account_id, region, key_id).await {
                Ok(aliases) => {
                    details.insert("Aliases".to_string(), aliases);
                }
                Err(e) => {
                    tracing::debug!("Could not list aliases: {}", e);
                }
            }
        }

        report_status_done("KMS", "describe_key", Some(key_id));
        Ok(key_details)
    }

    async fn describe_key_internal(
        &self,
        client: &kms::Client,
        key_id: &str,
    ) -> Result<serde_json::Value> {
        let response = client.describe_key().key_id(key_id).send().await?;

        if let Some(key_metadata) = response.key_metadata {
            Ok(self.key_metadata_to_json(&key_metadata))
        } else {
            Err(anyhow::anyhow!("Key {} not found", key_id))
        }
    }

    /// Get key policy document
    pub async fn get_key_policy(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
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

        let client = kms::Client::new(&aws_config);
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_key_policy()
                .key_id(key_id)
                .policy_name("default")
                .send(),
        )
        .await
        .with_context(|| "get_key_policy timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();
                if let Some(policy) = result.policy {
                    // Try to parse the policy as JSON
                    if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&policy) {
                        json.insert("Policy".to_string(), policy_json);
                    } else {
                        json.insert("Policy".to_string(), serde_json::Value::String(policy));
                    }
                }
                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("NotFoundException") || error_str.contains("AccessDenied") {
                    Ok(serde_json::json!({
                        "Policy": null,
                        "Note": "Unable to retrieve key policy"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get key policy: {}", e))
                }
            }
        }
    }

    /// Get key rotation status
    pub async fn get_key_rotation_status(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
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

        let client = kms::Client::new(&aws_config);
        let response = timeout(
            Duration::from_secs(10),
            client.get_key_rotation_status().key_id(key_id).send(),
        )
        .await
        .with_context(|| "get_key_rotation_status timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();
                json.insert(
                    "KeyRotationEnabled".to_string(),
                    serde_json::Value::Bool(result.key_rotation_enabled),
                );
                if let Some(rotation_period) = result.rotation_period_in_days {
                    json.insert(
                        "RotationPeriodInDays".to_string(),
                        serde_json::Value::Number(rotation_period.into()),
                    );
                }
                if let Some(next_rotation) = result.next_rotation_date {
                    json.insert(
                        "NextRotationDate".to_string(),
                        serde_json::Value::String(next_rotation.to_string()),
                    );
                }
                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                // AWS-managed keys don't support rotation status queries
                if error_str.contains("UnsupportedOperation") || error_str.contains("AccessDenied")
                {
                    Ok(serde_json::json!({
                        "KeyRotationEnabled": null,
                        "Note": "Rotation status not available for this key type"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get key rotation status: {}", e))
                }
            }
        }
    }

    /// List key policies (usually just "default")
    pub async fn list_key_policies(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
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

        let client = kms::Client::new(&aws_config);
        let response = timeout(
            Duration::from_secs(10),
            client.list_key_policies().key_id(key_id).send(),
        )
        .await
        .with_context(|| "list_key_policies timed out")?
        .with_context(|| format!("Failed to list policies for key {}", key_id))?;

        Ok(serde_json::json!({
            "PolicyNames": response.policy_names
        }))
    }

    /// List grants for key access audit
    pub async fn list_grants(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
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

        let client = kms::Client::new(&aws_config);
        let response = timeout(
            Duration::from_secs(10),
            client.list_grants().key_id(key_id).send(),
        )
        .await
        .with_context(|| "list_grants timed out")?;

        match response {
            Ok(result) => {
                let mut grants = Vec::new();
                if let Some(grant_list) = result.grants {
                    for grant in grant_list {
                        let mut grant_json = serde_json::Map::new();
                        if let Some(grant_id) = &grant.grant_id {
                            grant_json.insert(
                                "GrantId".to_string(),
                                serde_json::Value::String(grant_id.clone()),
                            );
                        }
                        if let Some(grantee_principal) = &grant.grantee_principal {
                            grant_json.insert(
                                "GranteePrincipal".to_string(),
                                serde_json::Value::String(grantee_principal.clone()),
                            );
                        }
                        if let Some(retiring_principal) = &grant.retiring_principal {
                            grant_json.insert(
                                "RetiringPrincipal".to_string(),
                                serde_json::Value::String(retiring_principal.clone()),
                            );
                        }
                        if let Some(issuing_account) = &grant.issuing_account {
                            grant_json.insert(
                                "IssuingAccount".to_string(),
                                serde_json::Value::String(issuing_account.clone()),
                            );
                        }
                        if let Some(operations) = &grant.operations {
                            let ops: Vec<serde_json::Value> = operations
                                .iter()
                                .map(|op| serde_json::Value::String(op.as_str().to_string()))
                                .collect();
                            grant_json
                                .insert("Operations".to_string(), serde_json::Value::Array(ops));
                        }
                        if let Some(constraints) = &grant.constraints {
                            let mut constraints_json = serde_json::Map::new();
                            if let Some(enc_context_subset) = &constraints.encryption_context_subset
                            {
                                let context: serde_json::Map<String, serde_json::Value> =
                                    enc_context_subset
                                        .iter()
                                        .map(|(k, v)| {
                                            (k.clone(), serde_json::Value::String(v.clone()))
                                        })
                                        .collect();
                                constraints_json.insert(
                                    "EncryptionContextSubset".to_string(),
                                    serde_json::Value::Object(context),
                                );
                            }
                            if let Some(enc_context_equals) = &constraints.encryption_context_equals
                            {
                                let context: serde_json::Map<String, serde_json::Value> =
                                    enc_context_equals
                                        .iter()
                                        .map(|(k, v)| {
                                            (k.clone(), serde_json::Value::String(v.clone()))
                                        })
                                        .collect();
                                constraints_json.insert(
                                    "EncryptionContextEquals".to_string(),
                                    serde_json::Value::Object(context),
                                );
                            }
                            if !constraints_json.is_empty() {
                                grant_json.insert(
                                    "Constraints".to_string(),
                                    serde_json::Value::Object(constraints_json),
                                );
                            }
                        }
                        if let Some(name) = &grant.name {
                            grant_json.insert(
                                "Name".to_string(),
                                serde_json::Value::String(name.clone()),
                            );
                        }
                        if let Some(creation_date) = grant.creation_date {
                            grant_json.insert(
                                "CreationDate".to_string(),
                                serde_json::Value::String(creation_date.to_string()),
                            );
                        }
                        grants.push(serde_json::Value::Object(grant_json));
                    }
                }
                Ok(serde_json::json!({ "Grants": grants }))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("AccessDenied") {
                    Ok(serde_json::json!({
                        "Grants": [],
                        "Note": "Unable to list grants - access denied"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to list grants: {}", e))
                }
            }
        }
    }

    /// List aliases for a key
    pub async fn list_aliases(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
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

        let client = kms::Client::new(&aws_config);
        let response = timeout(
            Duration::from_secs(10),
            client.list_aliases().key_id(key_id).send(),
        )
        .await
        .with_context(|| "list_aliases timed out")?
        .with_context(|| format!("Failed to list aliases for key {}", key_id))?;

        let mut aliases = Vec::new();
        if let Some(alias_list) = response.aliases {
            for alias in alias_list {
                let mut alias_json = serde_json::Map::new();
                if let Some(alias_name) = &alias.alias_name {
                    alias_json.insert(
                        "AliasName".to_string(),
                        serde_json::Value::String(alias_name.clone()),
                    );
                }
                if let Some(alias_arn) = &alias.alias_arn {
                    alias_json.insert(
                        "AliasArn".to_string(),
                        serde_json::Value::String(alias_arn.clone()),
                    );
                }
                if let Some(target_key_id) = &alias.target_key_id {
                    alias_json.insert(
                        "TargetKeyId".to_string(),
                        serde_json::Value::String(target_key_id.clone()),
                    );
                }
                if let Some(creation_date) = alias.creation_date {
                    alias_json.insert(
                        "CreationDate".to_string(),
                        serde_json::Value::String(creation_date.to_string()),
                    );
                }
                if let Some(last_updated) = alias.last_updated_date {
                    alias_json.insert(
                        "LastUpdatedDate".to_string(),
                        serde_json::Value::String(last_updated.to_string()),
                    );
                }
                aliases.push(serde_json::Value::Object(alias_json));
            }
        }

        Ok(serde_json::json!({ "Aliases": aliases }))
    }

    // Internal versions that take a client reference for use in list_keys
    async fn get_key_policy_internal(
        &self,
        client: &kms::Client,
        key_id: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client
                .get_key_policy()
                .key_id(key_id)
                .policy_name("default")
                .send(),
        )
        .await
        .with_context(|| "get_key_policy timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();
                if let Some(policy) = result.policy {
                    if let Ok(policy_json) = serde_json::from_str::<serde_json::Value>(&policy) {
                        json.insert("Policy".to_string(), policy_json);
                    } else {
                        json.insert("Policy".to_string(), serde_json::Value::String(policy));
                    }
                }
                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("NotFoundException") || error_str.contains("AccessDenied") {
                    Ok(serde_json::json!({
                        "Policy": null,
                        "Note": "Unable to retrieve key policy"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get key policy: {}", e))
                }
            }
        }
    }

    async fn get_key_rotation_status_internal(
        &self,
        client: &kms::Client,
        key_id: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.get_key_rotation_status().key_id(key_id).send(),
        )
        .await
        .with_context(|| "get_key_rotation_status timed out")?;

        match response {
            Ok(result) => {
                let mut json = serde_json::Map::new();
                json.insert(
                    "KeyRotationEnabled".to_string(),
                    serde_json::Value::Bool(result.key_rotation_enabled),
                );
                if let Some(rotation_period) = result.rotation_period_in_days {
                    json.insert(
                        "RotationPeriodInDays".to_string(),
                        serde_json::Value::Number(rotation_period.into()),
                    );
                }
                Ok(serde_json::Value::Object(json))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("UnsupportedOperation") || error_str.contains("AccessDenied")
                {
                    Ok(serde_json::json!({
                        "KeyRotationEnabled": null,
                        "Note": "Rotation status not available for this key type"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to get key rotation status: {}", e))
                }
            }
        }
    }

    async fn list_grants_internal(
        &self,
        client: &kms::Client,
        key_id: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.list_grants().key_id(key_id).send(),
        )
        .await
        .with_context(|| "list_grants timed out")?;

        match response {
            Ok(result) => {
                let mut grants = Vec::new();
                if let Some(grant_list) = result.grants {
                    for grant in grant_list {
                        let mut grant_json = serde_json::Map::new();
                        if let Some(grant_id) = &grant.grant_id {
                            grant_json.insert(
                                "GrantId".to_string(),
                                serde_json::Value::String(grant_id.clone()),
                            );
                        }
                        if let Some(grantee_principal) = &grant.grantee_principal {
                            grant_json.insert(
                                "GranteePrincipal".to_string(),
                                serde_json::Value::String(grantee_principal.clone()),
                            );
                        }
                        if let Some(operations) = &grant.operations {
                            let ops: Vec<serde_json::Value> = operations
                                .iter()
                                .map(|op| serde_json::Value::String(op.as_str().to_string()))
                                .collect();
                            grant_json
                                .insert("Operations".to_string(), serde_json::Value::Array(ops));
                        }
                        grants.push(serde_json::Value::Object(grant_json));
                    }
                }
                Ok(serde_json::json!({ "Grants": grants }))
            }
            Err(e) => {
                let error_str = format!("{:?}", e);
                if error_str.contains("AccessDenied") {
                    Ok(serde_json::json!({
                        "Grants": [],
                        "Note": "Unable to list grants - access denied"
                    }))
                } else {
                    Err(anyhow::anyhow!("Failed to list grants: {}", e))
                }
            }
        }
    }

    async fn list_aliases_internal(
        &self,
        client: &kms::Client,
        key_id: &str,
    ) -> Result<serde_json::Value> {
        let response = timeout(
            Duration::from_secs(10),
            client.list_aliases().key_id(key_id).send(),
        )
        .await
        .with_context(|| "list_aliases timed out")?
        .with_context(|| format!("Failed to list aliases for key {}", key_id))?;

        let mut aliases = Vec::new();
        if let Some(alias_list) = response.aliases {
            for alias in alias_list {
                let mut alias_json = serde_json::Map::new();
                if let Some(alias_name) = &alias.alias_name {
                    alias_json.insert(
                        "AliasName".to_string(),
                        serde_json::Value::String(alias_name.clone()),
                    );
                }
                if let Some(target_key_id) = &alias.target_key_id {
                    alias_json.insert(
                        "TargetKeyId".to_string(),
                        serde_json::Value::String(target_key_id.clone()),
                    );
                }
                aliases.push(serde_json::Value::Object(alias_json));
            }
        }

        Ok(serde_json::json!({ "Aliases": aliases }))
    }

    fn key_list_entry_to_json(&self, key: &kms::types::KeyListEntry) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(key_id) = &key.key_id {
            json.insert(
                "KeyId".to_string(),
                serde_json::Value::String(key_id.clone()),
            );
        }

        if let Some(key_arn) = &key.key_arn {
            json.insert(
                "Arn".to_string(),
                serde_json::Value::String(key_arn.clone()),
            );
        }

        // Add default fields for consistency
        json.insert(
            "Name".to_string(),
            serde_json::Value::String(key.key_id.as_deref().unwrap_or("unknown-key").to_string()),
        );
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("UNKNOWN".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn key_metadata_to_json(&self, key_metadata: &kms::types::KeyMetadata) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(aws_account_id) = &key_metadata.aws_account_id {
            json.insert(
                "AwsAccountId".to_string(),
                serde_json::Value::String(aws_account_id.clone()),
            );
        }

        json.insert(
            "KeyId".to_string(),
            serde_json::Value::String(key_metadata.key_id.clone()),
        );

        if let Some(arn) = &key_metadata.arn {
            json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
        }

        if let Some(creation_date) = key_metadata.creation_date {
            json.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        json.insert(
            "Enabled".to_string(),
            serde_json::Value::Bool(key_metadata.enabled),
        );

        if let Some(description) = &key_metadata.description {
            json.insert(
                "Description".to_string(),
                serde_json::Value::String(description.clone()),
            );
        }

        if let Some(key_usage) = &key_metadata.key_usage {
            json.insert(
                "KeyUsage".to_string(),
                serde_json::Value::String(key_usage.as_str().to_string()),
            );
        }

        if let Some(key_state) = &key_metadata.key_state {
            json.insert(
                "KeyState".to_string(),
                serde_json::Value::String(key_state.as_str().to_string()),
            );
            json.insert(
                "Status".to_string(),
                serde_json::Value::String(key_state.as_str().to_string()),
            );
        }

        if let Some(deletion_date) = key_metadata.deletion_date {
            json.insert(
                "DeletionDate".to_string(),
                serde_json::Value::String(deletion_date.to_string()),
            );
        }

        if let Some(valid_to) = key_metadata.valid_to {
            json.insert(
                "ValidTo".to_string(),
                serde_json::Value::String(valid_to.to_string()),
            );
        }

        if let Some(origin) = &key_metadata.origin {
            json.insert(
                "Origin".to_string(),
                serde_json::Value::String(origin.as_str().to_string()),
            );
        }

        if let Some(custom_key_store_id) = &key_metadata.custom_key_store_id {
            json.insert(
                "CustomKeyStoreId".to_string(),
                serde_json::Value::String(custom_key_store_id.clone()),
            );
        }

        if let Some(cloud_hsm_cluster_id) = &key_metadata.cloud_hsm_cluster_id {
            json.insert(
                "CloudHsmClusterId".to_string(),
                serde_json::Value::String(cloud_hsm_cluster_id.clone()),
            );
        }

        if let Some(expiration_model) = &key_metadata.expiration_model {
            json.insert(
                "ExpirationModel".to_string(),
                serde_json::Value::String(expiration_model.as_str().to_string()),
            );
        }

        if let Some(key_manager) = &key_metadata.key_manager {
            json.insert(
                "KeyManager".to_string(),
                serde_json::Value::String(key_manager.as_str().to_string()),
            );
        }

        // Use the newer KeySpec field instead of deprecated customer_master_key_spec
        // if let Some(customer_master_key_spec) = &key_metadata.customer_master_key_spec {
        //     json.insert("CustomerMasterKeySpec".to_string(), serde_json::Value::String(customer_master_key_spec.as_str().to_string()));
        // }

        if let Some(key_spec) = &key_metadata.key_spec {
            json.insert(
                "KeySpec".to_string(),
                serde_json::Value::String(key_spec.as_str().to_string()),
            );
        }

        if let Some(encryption_algorithms) = &key_metadata.encryption_algorithms {
            if !encryption_algorithms.is_empty() {
                let algorithms_json: Vec<serde_json::Value> = encryption_algorithms
                    .iter()
                    .map(|alg| serde_json::Value::String(alg.as_str().to_string()))
                    .collect();
                json.insert(
                    "EncryptionAlgorithms".to_string(),
                    serde_json::Value::Array(algorithms_json),
                );
            }
        }

        if let Some(signing_algorithms) = &key_metadata.signing_algorithms {
            if !signing_algorithms.is_empty() {
                let algorithms_json: Vec<serde_json::Value> = signing_algorithms
                    .iter()
                    .map(|alg| serde_json::Value::String(alg.as_str().to_string()))
                    .collect();
                json.insert(
                    "SigningAlgorithms".to_string(),
                    serde_json::Value::Array(algorithms_json),
                );
            }
        }

        json.insert(
            "MultiRegion".to_string(),
            serde_json::Value::Bool(key_metadata.multi_region.unwrap_or(false)),
        );

        if let Some(multi_region_configuration) = &key_metadata.multi_region_configuration {
            let mut config_json = serde_json::Map::new();
            if let Some(multi_region_key_type) = &multi_region_configuration.multi_region_key_type {
                config_json.insert(
                    "MultiRegionKeyType".to_string(),
                    serde_json::Value::String(multi_region_key_type.as_str().to_string()),
                );
            }
            if let Some(primary_key) = &multi_region_configuration.primary_key {
                let mut primary_json = serde_json::Map::new();
                if let Some(arn) = &primary_key.arn {
                    primary_json.insert("Arn".to_string(), serde_json::Value::String(arn.clone()));
                }
                if let Some(region) = &primary_key.region {
                    primary_json.insert(
                        "Region".to_string(),
                        serde_json::Value::String(region.clone()),
                    );
                }
                config_json.insert(
                    "PrimaryKey".to_string(),
                    serde_json::Value::Object(primary_json),
                );
            }
            json.insert(
                "MultiRegionConfiguration".to_string(),
                serde_json::Value::Object(config_json),
            );
        }

        // Use the description or key ID as name
        let name = key_metadata
            .description
            .as_deref()
            .unwrap_or(&key_metadata.key_id)
            .to_string();
        json.insert("Name".to_string(), serde_json::Value::String(name));

        serde_json::Value::Object(json)
    }
}
