use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_s3 as s3;
use std::sync::Arc;

pub struct S3Service {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl S3Service {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List S3 buckets (using list_buckets for listing)
    pub async fn list_buckets(
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

        let client = s3::Client::new(&aws_config);
        let response = client.list_buckets().send().await?;

        let mut buckets = Vec::new();
        if let Some(bucket_list) = response.buckets {
            for bucket in bucket_list {
                let bucket_json = self.bucket_to_json(&bucket, account_id, region).await?;
                buckets.push(bucket_json);
            }
        }

        Ok(buckets)
    }

    /// Get detailed bucket information
    pub async fn describe_bucket(
        &self,
        account_id: &str,
        region: &str,
        bucket_name: &str,
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

        let client = s3::Client::new(&aws_config);

        // Get bucket location
        let location_response = client
            .get_bucket_location()
            .bucket(bucket_name)
            .send()
            .await;

        // Get bucket versioning
        let versioning_response = client
            .get_bucket_versioning()
            .bucket(bucket_name)
            .send()
            .await;

        // Get bucket encryption
        let encryption_response = client
            .get_bucket_encryption()
            .bucket(bucket_name)
            .send()
            .await;

        // Get bucket policy status
        let policy_status_response = client
            .get_bucket_policy_status()
            .bucket(bucket_name)
            .send()
            .await;

        // Get bucket logging
        let logging_response = client.get_bucket_logging().bucket(bucket_name).send().await;

        let mut bucket_details = serde_json::Map::new();
        bucket_details.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );
        bucket_details.insert(
            "AccountId".to_string(),
            serde_json::Value::String(account_id.to_string()),
        );

        // Add location
        if let Ok(location) = location_response {
            if let Some(constraint) = location.location_constraint {
                bucket_details.insert(
                    "LocationConstraint".to_string(),
                    serde_json::Value::String(constraint.as_str().to_string()),
                );
            }
        }

        // Add versioning
        if let Ok(versioning) = versioning_response {
            if let Some(status) = versioning.status {
                bucket_details.insert(
                    "VersioningStatus".to_string(),
                    serde_json::Value::String(status.as_str().to_string()),
                );
            }
        }

        // Add encryption
        if let Ok(encryption) = encryption_response {
            bucket_details.insert(
                "EncryptionEnabled".to_string(),
                serde_json::Value::Bool(true),
            );
            if let Some(_config) = encryption.server_side_encryption_configuration {
                // Manual conversion for AWS SDK type
                let mut config_json = serde_json::Map::new();
                config_json.insert(
                    "HasConfiguration".to_string(),
                    serde_json::Value::Bool(true),
                );
                bucket_details.insert(
                    "EncryptionConfiguration".to_string(),
                    serde_json::Value::Object(config_json),
                );
            }
        } else {
            bucket_details.insert(
                "EncryptionEnabled".to_string(),
                serde_json::Value::Bool(false),
            );
        }

        // Add policy status
        if let Ok(policy_status) = policy_status_response {
            if let Some(status) = policy_status.policy_status {
                bucket_details.insert(
                    "PublicAccessBlockEnabled".to_string(),
                    serde_json::Value::Bool(!status.is_public.unwrap_or(false)),
                );
            }
        }

        // Add logging
        if let Ok(logging) = logging_response {
            if let Some(_config) = logging.logging_enabled {
                bucket_details.insert("LoggingEnabled".to_string(), serde_json::Value::Bool(true));
                // Manual conversion for AWS SDK type
                let mut config_json = serde_json::Map::new();
                config_json.insert("HasLogging".to_string(), serde_json::Value::Bool(true));
                bucket_details.insert(
                    "LoggingConfiguration".to_string(),
                    serde_json::Value::Object(config_json),
                );
            } else {
                bucket_details.insert("LoggingEnabled".to_string(), serde_json::Value::Bool(false));
            }
        }

        Ok(serde_json::Value::Object(bucket_details))
    }

    /// Get bucket policy
    pub async fn get_bucket_policy(
        &self,
        account_id: &str,
        region: &str,
        bucket_name: &str,
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

        let client = s3::Client::new(&aws_config);
        let response = client.get_bucket_policy().bucket(bucket_name).send().await;

        let mut policy_json = serde_json::Map::new();
        policy_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(policy_response) => {
                policy_json.insert("HasPolicy".to_string(), serde_json::Value::Bool(true));
                if let Some(policy) = policy_response.policy {
                    policy_json.insert("Policy".to_string(), serde_json::Value::String(policy));
                }
            }
            Err(_) => {
                policy_json.insert("HasPolicy".to_string(), serde_json::Value::Bool(false));
            }
        }

        Ok(serde_json::Value::Object(policy_json))
    }

    /// Get bucket encryption configuration
    pub async fn get_bucket_encryption(
        &self,
        account_id: &str,
        region: &str,
        bucket_name: &str,
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

        let client = s3::Client::new(&aws_config);
        let response = client
            .get_bucket_encryption()
            .bucket(bucket_name)
            .send()
            .await;

        let mut encryption_json = serde_json::Map::new();
        encryption_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(encryption_response) => {
                encryption_json.insert(
                    "EncryptionEnabled".to_string(),
                    serde_json::Value::Bool(true),
                );
                if let Some(config) = encryption_response.server_side_encryption_configuration {
                    let mut config_json = serde_json::Map::new();
                    if !config.rules.is_empty() {
                        let mut rules = Vec::new();
                        for rule in &config.rules {
                            let mut rule_json = serde_json::Map::new();
                            if let Some(default_encryption) =
                                &rule.apply_server_side_encryption_by_default
                            {
                                rule_json.insert(
                                    "SSEAlgorithm".to_string(),
                                    serde_json::Value::String(
                                        default_encryption.sse_algorithm.as_str().to_string(),
                                    ),
                                );
                                if let Some(kms_key_id) = &default_encryption.kms_master_key_id {
                                    rule_json.insert(
                                        "KMSMasterKeyID".to_string(),
                                        serde_json::Value::String(kms_key_id.clone()),
                                    );
                                }
                            }
                            rules.push(serde_json::Value::Object(rule_json));
                        }
                        config_json.insert("Rules".to_string(), serde_json::Value::Array(rules));
                    }
                    encryption_json.insert(
                        "Configuration".to_string(),
                        serde_json::Value::Object(config_json),
                    );
                }
            }
            Err(_) => {
                encryption_json.insert(
                    "EncryptionEnabled".to_string(),
                    serde_json::Value::Bool(false),
                );
            }
        }

        Ok(serde_json::Value::Object(encryption_json))
    }

    /// Get bucket versioning configuration
    pub async fn get_bucket_versioning(
        &self,
        account_id: &str,
        region: &str,
        bucket_name: &str,
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

        let client = s3::Client::new(&aws_config);
        let response = client
            .get_bucket_versioning()
            .bucket(bucket_name)
            .send()
            .await;

        let mut versioning_json = serde_json::Map::new();
        versioning_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(versioning_response) => {
                if let Some(status) = versioning_response.status {
                    versioning_json.insert(
                        "Status".to_string(),
                        serde_json::Value::String(status.as_str().to_string()),
                    );
                } else {
                    versioning_json.insert(
                        "Status".to_string(),
                        serde_json::Value::String("Disabled".to_string()),
                    );
                }

                if let Some(mfa_delete) = versioning_response.mfa_delete {
                    versioning_json.insert(
                        "MfaDelete".to_string(),
                        serde_json::Value::String(mfa_delete.as_str().to_string()),
                    );
                }
            }
            Err(_) => {
                versioning_json.insert(
                    "Status".to_string(),
                    serde_json::Value::String("Unknown".to_string()),
                );
            }
        }

        Ok(serde_json::Value::Object(versioning_json))
    }

    /// Get bucket lifecycle configuration
    pub async fn get_bucket_lifecycle(
        &self,
        account_id: &str,
        region: &str,
        bucket_name: &str,
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

        let client = s3::Client::new(&aws_config);
        let response = client
            .get_bucket_lifecycle_configuration()
            .bucket(bucket_name)
            .send()
            .await;

        let mut lifecycle_json = serde_json::Map::new();
        lifecycle_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(lifecycle_response) => {
                lifecycle_json.insert(
                    "HasLifecycleConfiguration".to_string(),
                    serde_json::Value::Bool(true),
                );
                if let Some(rules) = lifecycle_response.rules {
                    let mut rules_array = Vec::new();
                    for rule in &rules {
                        let mut rule_json = serde_json::Map::new();

                        if let Some(id) = &rule.id {
                            rule_json
                                .insert("Id".to_string(), serde_json::Value::String(id.clone()));
                        }

                        rule_json.insert(
                            "Status".to_string(),
                            serde_json::Value::String(rule.status.as_str().to_string()),
                        );

                        if let Some(filter) = &rule.filter {
                            let mut filter_json = serde_json::Map::new();
                            if let Some(prefix) = &filter.prefix {
                                filter_json.insert(
                                    "Prefix".to_string(),
                                    serde_json::Value::String(prefix.clone()),
                                );
                            }
                            rule_json.insert(
                                "Filter".to_string(),
                                serde_json::Value::Object(filter_json),
                            );
                        }

                        if let Some(expiration) = &rule.expiration {
                            let mut exp_json = serde_json::Map::new();
                            if let Some(days) = expiration.days {
                                exp_json.insert(
                                    "Days".to_string(),
                                    serde_json::Value::Number(serde_json::Number::from(days)),
                                );
                            }
                            rule_json.insert(
                                "Expiration".to_string(),
                                serde_json::Value::Object(exp_json),
                            );
                        }

                        rules_array.push(serde_json::Value::Object(rule_json));
                    }
                    lifecycle_json
                        .insert("Rules".to_string(), serde_json::Value::Array(rules_array));
                }
            }
            Err(_) => {
                lifecycle_json.insert(
                    "HasLifecycleConfiguration".to_string(),
                    serde_json::Value::Bool(false),
                );
            }
        }

        Ok(serde_json::Value::Object(lifecycle_json))
    }

    /// Convert bucket to JSON format with enhanced configuration
    async fn bucket_to_json(
        &self,
        bucket: &s3::types::Bucket,
        account_id: &str,
        region: &str,
    ) -> Result<serde_json::Value> {
        let mut bucket_map = serde_json::Map::new();

        let bucket_name = if let Some(name) = &bucket.name {
            bucket_map.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            bucket_map.insert(
                "BucketName".to_string(),
                serde_json::Value::String(name.clone()),
            );
            name.clone()
        } else {
            return Ok(serde_json::Value::Object(bucket_map));
        };

        if let Some(creation_date) = bucket.creation_date {
            bucket_map.insert(
                "CreationDate".to_string(),
                serde_json::Value::String(creation_date.to_string()),
            );
        }

        bucket_map.insert(
            "AccountId".to_string(),
            serde_json::Value::String(account_id.to_string()),
        );

        // S3 buckets are global but we track them by region for UI purposes
        bucket_map.insert("GlobalResource".to_string(), serde_json::Value::Bool(true));

        // Add enhanced configuration data - use timeouts and ignore errors for listing
        if let Ok(Ok(policy_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_policy(account_id, region, &bucket_name),
        )
        .await
        {
            if let Some(has_policy) = policy_data.get("HasPolicy").and_then(|v| v.as_bool()) {
                bucket_map.insert("HasPolicy".to_string(), serde_json::Value::Bool(has_policy));
            }
        }

        if let Ok(Ok(encryption_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_encryption(account_id, region, &bucket_name),
        )
        .await
        {
            if let Some(encrypted) = encryption_data
                .get("EncryptionEnabled")
                .and_then(|v| v.as_bool())
            {
                bucket_map.insert(
                    "EncryptionEnabled".to_string(),
                    serde_json::Value::Bool(encrypted),
                );
            }
            if let Some(config) = encryption_data.get("Configuration") {
                bucket_map.insert("EncryptionConfiguration".to_string(), config.clone());
            }
        }

        if let Ok(Ok(versioning_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_versioning(account_id, region, &bucket_name),
        )
        .await
        {
            if let Some(status) = versioning_data.get("Status").and_then(|v| v.as_str()) {
                bucket_map.insert(
                    "VersioningStatus".to_string(),
                    serde_json::Value::String(status.to_string()),
                );
            }
        }

        if let Ok(Ok(lifecycle_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_lifecycle(account_id, region, &bucket_name),
        )
        .await
        {
            if let Some(has_lifecycle) = lifecycle_data
                .get("HasLifecycleConfiguration")
                .and_then(|v| v.as_bool())
            {
                bucket_map.insert(
                    "HasLifecycleConfiguration".to_string(),
                    serde_json::Value::Bool(has_lifecycle),
                );
            }
            if let Some(rules) = lifecycle_data.get("Rules") {
                bucket_map.insert("LifecycleRules".to_string(), rules.clone());
            }
        }

        Ok(serde_json::Value::Object(bucket_map))
    }
}
