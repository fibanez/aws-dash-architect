use super::super::credentials::CredentialCoordinator;
use super::super::status::{report_status, report_status_done};
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

    /// List S3 buckets with optional detailed configuration
    ///
    /// # Arguments
    /// * `include_details` - If false (Phase 1), returns basic bucket info quickly.
    ///   If true (Phase 2), includes encryption, versioning, policy, etc.
    pub async fn list_buckets(
        &self,
        account_id: &str,
        region: &str,
        include_details: bool,
    ) -> Result<Vec<serde_json::Value>> {
        report_status("S3", "list_buckets", Some(account_id));

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
                let bucket_json = self
                    .bucket_to_json(&bucket, account_id, region, include_details)
                    .await?;
                buckets.push(bucket_json);
            }
        }

        report_status_done("S3", "list_buckets", Some(account_id));
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

    /// Get bucket ACL (2.1)
    pub async fn get_bucket_acl(
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
        let response = client.get_bucket_acl().bucket(bucket_name).send().await;

        let mut acl_json = serde_json::Map::new();
        acl_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(acl_response) => {
                // Add owner information
                if let Some(owner) = acl_response.owner {
                    let mut owner_json = serde_json::Map::new();
                    if let Some(id) = owner.id {
                        owner_json.insert("Id".to_string(), serde_json::Value::String(id));
                    }
                    if let Some(display_name) = owner.display_name {
                        owner_json.insert(
                            "DisplayName".to_string(),
                            serde_json::Value::String(display_name),
                        );
                    }
                    acl_json.insert("Owner".to_string(), serde_json::Value::Object(owner_json));
                }

                // Add grants
                if let Some(grants) = acl_response.grants {
                    let mut grants_array = Vec::new();
                    for grant in grants {
                        let mut grant_json = serde_json::Map::new();

                        if let Some(grantee) = grant.grantee {
                            let mut grantee_json = serde_json::Map::new();
                            grantee_json.insert(
                                "Type".to_string(),
                                serde_json::Value::String(grantee.r#type.as_str().to_string()),
                            );
                            if let Some(id) = grantee.id {
                                grantee_json
                                    .insert("Id".to_string(), serde_json::Value::String(id));
                            }
                            if let Some(display_name) = grantee.display_name {
                                grantee_json.insert(
                                    "DisplayName".to_string(),
                                    serde_json::Value::String(display_name),
                                );
                            }
                            if let Some(uri) = grantee.uri {
                                grantee_json
                                    .insert("URI".to_string(), serde_json::Value::String(uri));
                            }
                            grant_json.insert(
                                "Grantee".to_string(),
                                serde_json::Value::Object(grantee_json),
                            );
                        }

                        if let Some(permission) = grant.permission {
                            grant_json.insert(
                                "Permission".to_string(),
                                serde_json::Value::String(permission.as_str().to_string()),
                            );
                        }

                        grants_array.push(serde_json::Value::Object(grant_json));
                    }
                    acl_json.insert("Grants".to_string(), serde_json::Value::Array(grants_array));
                }
            }
            Err(e) => {
                acl_json.insert(
                    "Error".to_string(),
                    serde_json::Value::String(e.to_string()),
                );
            }
        }

        Ok(serde_json::Value::Object(acl_json))
    }

    /// Get public access block configuration (2.2)
    pub async fn get_public_access_block(
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
            .get_public_access_block()
            .bucket(bucket_name)
            .send()
            .await;

        let mut pab_json = serde_json::Map::new();
        pab_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(pab_response) => {
                pab_json.insert(
                    "HasPublicAccessBlock".to_string(),
                    serde_json::Value::Bool(true),
                );
                if let Some(config) = pab_response.public_access_block_configuration {
                    pab_json.insert(
                        "BlockPublicAcls".to_string(),
                        serde_json::Value::Bool(config.block_public_acls.unwrap_or(false)),
                    );
                    pab_json.insert(
                        "IgnorePublicAcls".to_string(),
                        serde_json::Value::Bool(config.ignore_public_acls.unwrap_or(false)),
                    );
                    pab_json.insert(
                        "BlockPublicPolicy".to_string(),
                        serde_json::Value::Bool(config.block_public_policy.unwrap_or(false)),
                    );
                    pab_json.insert(
                        "RestrictPublicBuckets".to_string(),
                        serde_json::Value::Bool(config.restrict_public_buckets.unwrap_or(false)),
                    );
                }
            }
            Err(_) => {
                // No public access block configured
                pab_json.insert(
                    "HasPublicAccessBlock".to_string(),
                    serde_json::Value::Bool(false),
                );
            }
        }

        Ok(serde_json::Value::Object(pab_json))
    }

    /// Get bucket replication configuration (2.3)
    pub async fn get_bucket_replication(
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
            .get_bucket_replication()
            .bucket(bucket_name)
            .send()
            .await;

        let mut replication_json = serde_json::Map::new();
        replication_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(replication_response) => {
                replication_json
                    .insert("HasReplication".to_string(), serde_json::Value::Bool(true));
                if let Some(config) = replication_response.replication_configuration {
                    replication_json.insert(
                        "Role".to_string(),
                        serde_json::Value::String(config.role.clone()),
                    );
                    let mut rules_array = Vec::new();
                    for rule in &config.rules {
                        let mut rule_json = serde_json::Map::new();
                        if let Some(id) = &rule.id {
                            rule_json
                                .insert("Id".to_string(), serde_json::Value::String(id.clone()));
                        }
                        rule_json.insert(
                            "Status".to_string(),
                            serde_json::Value::String(rule.status.as_str().to_string()),
                        );
                        if let Some(priority) = rule.priority {
                            rule_json.insert(
                                "Priority".to_string(),
                                serde_json::Value::Number(serde_json::Number::from(priority)),
                            );
                        }
                        if let Some(dest) = &rule.destination {
                            let mut dest_json = serde_json::Map::new();
                            dest_json.insert(
                                "Bucket".to_string(),
                                serde_json::Value::String(dest.bucket.clone()),
                            );
                            if let Some(account) = &dest.account {
                                dest_json.insert(
                                    "Account".to_string(),
                                    serde_json::Value::String(account.clone()),
                                );
                            }
                            if let Some(storage_class) = &dest.storage_class {
                                dest_json.insert(
                                    "StorageClass".to_string(),
                                    serde_json::Value::String(storage_class.as_str().to_string()),
                                );
                            }
                            rule_json.insert(
                                "Destination".to_string(),
                                serde_json::Value::Object(dest_json),
                            );
                        }
                        rules_array.push(serde_json::Value::Object(rule_json));
                    }
                    replication_json
                        .insert("Rules".to_string(), serde_json::Value::Array(rules_array));
                }
            }
            Err(_) => {
                replication_json
                    .insert("HasReplication".to_string(), serde_json::Value::Bool(false));
            }
        }

        Ok(serde_json::Value::Object(replication_json))
    }

    /// Get bucket CORS configuration (2.4)
    pub async fn get_bucket_cors(
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
        let response = client.get_bucket_cors().bucket(bucket_name).send().await;

        let mut cors_json = serde_json::Map::new();
        cors_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(cors_response) => {
                cors_json.insert("HasCORS".to_string(), serde_json::Value::Bool(true));
                if let Some(rules) = cors_response.cors_rules {
                    let mut rules_array = Vec::new();
                    for rule in rules {
                        let mut rule_json = serde_json::Map::new();

                        if let Some(id) = rule.id {
                            rule_json.insert("Id".to_string(), serde_json::Value::String(id));
                        }

                        let origins: Vec<serde_json::Value> = rule
                            .allowed_origins
                            .iter()
                            .map(|o| serde_json::Value::String(o.clone()))
                            .collect();
                        rule_json.insert(
                            "AllowedOrigins".to_string(),
                            serde_json::Value::Array(origins),
                        );

                        let methods: Vec<serde_json::Value> = rule
                            .allowed_methods
                            .iter()
                            .map(|m| serde_json::Value::String(m.clone()))
                            .collect();
                        rule_json.insert(
                            "AllowedMethods".to_string(),
                            serde_json::Value::Array(methods),
                        );

                        if let Some(headers) = rule.allowed_headers {
                            let headers_array: Vec<serde_json::Value> = headers
                                .iter()
                                .map(|h| serde_json::Value::String(h.clone()))
                                .collect();
                            rule_json.insert(
                                "AllowedHeaders".to_string(),
                                serde_json::Value::Array(headers_array),
                            );
                        }

                        if let Some(expose_headers) = rule.expose_headers {
                            let expose_array: Vec<serde_json::Value> = expose_headers
                                .iter()
                                .map(|h| serde_json::Value::String(h.clone()))
                                .collect();
                            rule_json.insert(
                                "ExposeHeaders".to_string(),
                                serde_json::Value::Array(expose_array),
                            );
                        }

                        if let Some(max_age) = rule.max_age_seconds {
                            rule_json.insert(
                                "MaxAgeSeconds".to_string(),
                                serde_json::Value::Number(serde_json::Number::from(max_age)),
                            );
                        }

                        rules_array.push(serde_json::Value::Object(rule_json));
                    }
                    cors_json.insert(
                        "CORSRules".to_string(),
                        serde_json::Value::Array(rules_array),
                    );
                }
            }
            Err(_) => {
                cors_json.insert("HasCORS".to_string(), serde_json::Value::Bool(false));
            }
        }

        Ok(serde_json::Value::Object(cors_json))
    }

    /// Get bucket website configuration (2.5)
    pub async fn get_bucket_website(
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
        let response = client.get_bucket_website().bucket(bucket_name).send().await;

        let mut website_json = serde_json::Map::new();
        website_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(website_response) => {
                website_json.insert("WebsiteEnabled".to_string(), serde_json::Value::Bool(true));

                if let Some(index_doc) = website_response.index_document {
                    website_json.insert(
                        "IndexDocument".to_string(),
                        serde_json::Value::String(index_doc.suffix),
                    );
                }

                if let Some(error_doc) = website_response.error_document {
                    website_json.insert(
                        "ErrorDocument".to_string(),
                        serde_json::Value::String(error_doc.key),
                    );
                }

                if let Some(redirect) = website_response.redirect_all_requests_to {
                    let mut redirect_json = serde_json::Map::new();
                    redirect_json.insert(
                        "HostName".to_string(),
                        serde_json::Value::String(redirect.host_name),
                    );
                    if let Some(protocol) = redirect.protocol {
                        redirect_json.insert(
                            "Protocol".to_string(),
                            serde_json::Value::String(protocol.as_str().to_string()),
                        );
                    }
                    website_json.insert(
                        "RedirectAllRequestsTo".to_string(),
                        serde_json::Value::Object(redirect_json),
                    );
                }

                if let Some(routing_rules) = website_response.routing_rules {
                    let mut rules_array = Vec::new();
                    for rule in routing_rules {
                        let mut rule_json = serde_json::Map::new();

                        if let Some(condition) = rule.condition {
                            let mut cond_json = serde_json::Map::new();
                            if let Some(key_prefix) = condition.key_prefix_equals {
                                cond_json.insert(
                                    "KeyPrefixEquals".to_string(),
                                    serde_json::Value::String(key_prefix),
                                );
                            }
                            if let Some(error_code) = condition.http_error_code_returned_equals {
                                cond_json.insert(
                                    "HttpErrorCodeReturnedEquals".to_string(),
                                    serde_json::Value::String(error_code),
                                );
                            }
                            rule_json.insert(
                                "Condition".to_string(),
                                serde_json::Value::Object(cond_json),
                            );
                        }

                        if let Some(redirect) = rule.redirect {
                            let mut redir_json = serde_json::Map::new();
                            if let Some(host) = redirect.host_name {
                                redir_json.insert(
                                    "HostName".to_string(),
                                    serde_json::Value::String(host),
                                );
                            }
                            if let Some(protocol) = redirect.protocol {
                                redir_json.insert(
                                    "Protocol".to_string(),
                                    serde_json::Value::String(protocol.as_str().to_string()),
                                );
                            }
                            if let Some(replace_key) = redirect.replace_key_with {
                                redir_json.insert(
                                    "ReplaceKeyWith".to_string(),
                                    serde_json::Value::String(replace_key),
                                );
                            }
                            if let Some(replace_prefix) = redirect.replace_key_prefix_with {
                                redir_json.insert(
                                    "ReplaceKeyPrefixWith".to_string(),
                                    serde_json::Value::String(replace_prefix),
                                );
                            }
                            if let Some(code) = redirect.http_redirect_code {
                                redir_json.insert(
                                    "HttpRedirectCode".to_string(),
                                    serde_json::Value::String(code),
                                );
                            }
                            rule_json.insert(
                                "Redirect".to_string(),
                                serde_json::Value::Object(redir_json),
                            );
                        }

                        rules_array.push(serde_json::Value::Object(rule_json));
                    }
                    website_json.insert(
                        "RoutingRules".to_string(),
                        serde_json::Value::Array(rules_array),
                    );
                }
            }
            Err(_) => {
                website_json.insert("WebsiteEnabled".to_string(), serde_json::Value::Bool(false));
            }
        }

        Ok(serde_json::Value::Object(website_json))
    }

    /// Get bucket notification configuration (2.6)
    pub async fn get_bucket_notification(
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
            .get_bucket_notification_configuration()
            .bucket(bucket_name)
            .send()
            .await;

        let mut notification_json = serde_json::Map::new();
        notification_json.insert(
            "BucketName".to_string(),
            serde_json::Value::String(bucket_name.to_string()),
        );

        match response {
            Ok(notification_response) => {
                let mut has_notifications = false;

                // Lambda configurations
                if let Some(lambda_configs) = notification_response.lambda_function_configurations {
                    if !lambda_configs.is_empty() {
                        has_notifications = true;
                        let mut lambda_array = Vec::new();
                        for config in lambda_configs {
                            let mut config_json = serde_json::Map::new();
                            if let Some(id) = config.id {
                                config_json.insert("Id".to_string(), serde_json::Value::String(id));
                            }
                            config_json.insert(
                                "LambdaFunctionArn".to_string(),
                                serde_json::Value::String(config.lambda_function_arn),
                            );
                            let events: Vec<serde_json::Value> = config
                                .events
                                .iter()
                                .map(|e| serde_json::Value::String(e.as_str().to_string()))
                                .collect();
                            config_json
                                .insert("Events".to_string(), serde_json::Value::Array(events));
                            lambda_array.push(serde_json::Value::Object(config_json));
                        }
                        notification_json.insert(
                            "LambdaFunctionConfigurations".to_string(),
                            serde_json::Value::Array(lambda_array),
                        );
                    }
                }

                // SQS configurations
                if let Some(queue_configs) = notification_response.queue_configurations {
                    if !queue_configs.is_empty() {
                        has_notifications = true;
                        let mut queue_array = Vec::new();
                        for config in queue_configs {
                            let mut config_json = serde_json::Map::new();
                            if let Some(id) = config.id {
                                config_json.insert("Id".to_string(), serde_json::Value::String(id));
                            }
                            config_json.insert(
                                "QueueArn".to_string(),
                                serde_json::Value::String(config.queue_arn),
                            );
                            let events: Vec<serde_json::Value> = config
                                .events
                                .iter()
                                .map(|e| serde_json::Value::String(e.as_str().to_string()))
                                .collect();
                            config_json
                                .insert("Events".to_string(), serde_json::Value::Array(events));
                            queue_array.push(serde_json::Value::Object(config_json));
                        }
                        notification_json.insert(
                            "QueueConfigurations".to_string(),
                            serde_json::Value::Array(queue_array),
                        );
                    }
                }

                // SNS configurations
                if let Some(topic_configs) = notification_response.topic_configurations {
                    if !topic_configs.is_empty() {
                        has_notifications = true;
                        let mut topic_array = Vec::new();
                        for config in topic_configs {
                            let mut config_json = serde_json::Map::new();
                            if let Some(id) = config.id {
                                config_json.insert("Id".to_string(), serde_json::Value::String(id));
                            }
                            config_json.insert(
                                "TopicArn".to_string(),
                                serde_json::Value::String(config.topic_arn),
                            );
                            let events: Vec<serde_json::Value> = config
                                .events
                                .iter()
                                .map(|e| serde_json::Value::String(e.as_str().to_string()))
                                .collect();
                            config_json
                                .insert("Events".to_string(), serde_json::Value::Array(events));
                            topic_array.push(serde_json::Value::Object(config_json));
                        }
                        notification_json.insert(
                            "TopicConfigurations".to_string(),
                            serde_json::Value::Array(topic_array),
                        );
                    }
                }

                notification_json.insert(
                    "HasNotifications".to_string(),
                    serde_json::Value::Bool(has_notifications),
                );
            }
            Err(_) => {
                notification_json.insert(
                    "HasNotifications".to_string(),
                    serde_json::Value::Bool(false),
                );
            }
        }

        Ok(serde_json::Value::Object(notification_json))
    }

    /// Convert bucket to JSON format with optional enhanced configuration
    async fn bucket_to_json(
        &self,
        bucket: &s3::types::Bucket,
        account_id: &str,
        region: &str,
        include_details: bool,
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

        // Only fetch details if requested (Phase 2)
        if include_details {
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

            // Get ACL (2.1)
            report_status("S3", "bucket_acl", Some(&bucket_name));
            if let Ok(Ok(acl_data)) = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.get_bucket_acl(account_id, region, &bucket_name),
            )
            .await
            {
                if let Some(owner) = acl_data.get("Owner") {
                    bucket_map.insert("Owner".to_string(), owner.clone());
                }
                if let Some(grants) = acl_data.get("Grants") {
                    bucket_map.insert("Grants".to_string(), grants.clone());
                }
            }

            // Get Public Access Block (2.2)
            report_status("S3", "public_access_block", Some(&bucket_name));
            if let Ok(Ok(pab_data)) = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.get_public_access_block(account_id, region, &bucket_name),
            )
            .await
            {
                if let Some(has_pab) = pab_data
                    .get("HasPublicAccessBlock")
                    .and_then(|v| v.as_bool())
                {
                    bucket_map.insert(
                        "HasPublicAccessBlock".to_string(),
                        serde_json::Value::Bool(has_pab),
                    );
                }
                if let Some(block_acls) = pab_data.get("BlockPublicAcls") {
                    bucket_map.insert("BlockPublicAcls".to_string(), block_acls.clone());
                }
                if let Some(ignore_acls) = pab_data.get("IgnorePublicAcls") {
                    bucket_map.insert("IgnorePublicAcls".to_string(), ignore_acls.clone());
                }
                if let Some(block_policy) = pab_data.get("BlockPublicPolicy") {
                    bucket_map.insert("BlockPublicPolicy".to_string(), block_policy.clone());
                }
                if let Some(restrict) = pab_data.get("RestrictPublicBuckets") {
                    bucket_map.insert("RestrictPublicBuckets".to_string(), restrict.clone());
                }
            }

            // Get Replication (2.3)
            report_status("S3", "bucket_replication", Some(&bucket_name));
            if let Ok(Ok(replication_data)) = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.get_bucket_replication(account_id, region, &bucket_name),
            )
            .await
            {
                if let Some(has_replication) = replication_data
                    .get("HasReplication")
                    .and_then(|v| v.as_bool())
                {
                    bucket_map.insert(
                        "HasReplication".to_string(),
                        serde_json::Value::Bool(has_replication),
                    );
                }
                if let Some(rules) = replication_data.get("Rules") {
                    bucket_map.insert("ReplicationRules".to_string(), rules.clone());
                }
                if let Some(role) = replication_data.get("Role") {
                    bucket_map.insert("ReplicationRole".to_string(), role.clone());
                }
            }

            // Get CORS (2.4)
            report_status("S3", "bucket_cors", Some(&bucket_name));
            if let Ok(Ok(cors_data)) = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.get_bucket_cors(account_id, region, &bucket_name),
            )
            .await
            {
                if let Some(has_cors) = cors_data.get("HasCORS").and_then(|v| v.as_bool()) {
                    bucket_map.insert("HasCORS".to_string(), serde_json::Value::Bool(has_cors));
                }
                if let Some(rules) = cors_data.get("CORSRules") {
                    bucket_map.insert("CORSRules".to_string(), rules.clone());
                }
            }

            // Get Website (2.5)
            report_status("S3", "bucket_website", Some(&bucket_name));
            if let Ok(Ok(website_data)) = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.get_bucket_website(account_id, region, &bucket_name),
            )
            .await
            {
                if let Some(enabled) = website_data.get("WebsiteEnabled").and_then(|v| v.as_bool())
                {
                    bucket_map.insert(
                        "WebsiteEnabled".to_string(),
                        serde_json::Value::Bool(enabled),
                    );
                }
                if let Some(index_doc) = website_data.get("IndexDocument") {
                    bucket_map.insert("WebsiteIndexDocument".to_string(), index_doc.clone());
                }
                if let Some(error_doc) = website_data.get("ErrorDocument") {
                    bucket_map.insert("WebsiteErrorDocument".to_string(), error_doc.clone());
                }
                if let Some(redirect) = website_data.get("RedirectAllRequestsTo") {
                    bucket_map.insert("WebsiteRedirect".to_string(), redirect.clone());
                }
            }

            // Get Notifications (2.6)
            report_status("S3", "bucket_notifications", Some(&bucket_name));
            if let Ok(Ok(notification_data)) = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                self.get_bucket_notification(account_id, region, &bucket_name),
            )
            .await
            {
                if let Some(has_notifications) = notification_data
                    .get("HasNotifications")
                    .and_then(|v| v.as_bool())
                {
                    bucket_map.insert(
                        "HasNotifications".to_string(),
                        serde_json::Value::Bool(has_notifications),
                    );
                }
                if let Some(lambda_configs) = notification_data.get("LambdaFunctionConfigurations")
                {
                    bucket_map.insert(
                        "LambdaFunctionConfigurations".to_string(),
                        lambda_configs.clone(),
                    );
                }
                if let Some(queue_configs) = notification_data.get("QueueConfigurations") {
                    bucket_map.insert("QueueConfigurations".to_string(), queue_configs.clone());
                }
                if let Some(topic_configs) = notification_data.get("TopicConfigurations") {
                    bucket_map.insert("TopicConfigurations".to_string(), topic_configs.clone());
                }
            }
        } // end if include_details

        Ok(serde_json::Value::Object(bucket_map))
    }

    /// Get details for a specific S3 bucket (Phase 2 enrichment)
    /// Returns only the detail fields to be merged into existing resource data
    pub async fn get_bucket_details(
        &self,
        account_id: &str,
        region: &str,
        bucket_name: &str,
    ) -> Result<serde_json::Value> {
        report_status("S3", "get_bucket_details", Some(bucket_name));
        let mut details = serde_json::Map::new();

        // Get bucket policy
        if let Ok(Ok(policy_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_policy(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(has_policy) = policy_data.get("HasPolicy").and_then(|v| v.as_bool()) {
                details.insert("HasPolicy".to_string(), serde_json::Value::Bool(has_policy));
            }
            if let Some(policy) = policy_data.get("Policy") {
                details.insert("Policy".to_string(), policy.clone());
            }
        }

        // Get encryption
        if let Ok(Ok(encryption_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_encryption(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(encrypted) = encryption_data
                .get("EncryptionEnabled")
                .and_then(|v| v.as_bool())
            {
                details.insert(
                    "EncryptionEnabled".to_string(),
                    serde_json::Value::Bool(encrypted),
                );
            }
            if let Some(config) = encryption_data.get("Configuration") {
                details.insert("EncryptionConfiguration".to_string(), config.clone());
            }
        }

        // Get versioning
        if let Ok(Ok(versioning_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_versioning(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(status) = versioning_data.get("Status").and_then(|v| v.as_str()) {
                details.insert(
                    "VersioningStatus".to_string(),
                    serde_json::Value::String(status.to_string()),
                );
            }
        }

        // Get lifecycle
        if let Ok(Ok(lifecycle_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_lifecycle(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(has_lifecycle) = lifecycle_data
                .get("HasLifecycleConfiguration")
                .and_then(|v| v.as_bool())
            {
                details.insert(
                    "HasLifecycleConfiguration".to_string(),
                    serde_json::Value::Bool(has_lifecycle),
                );
            }
            if let Some(rules) = lifecycle_data.get("Rules") {
                details.insert("LifecycleRules".to_string(), rules.clone());
            }
        }

        // Get ACL
        if let Ok(Ok(acl_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_acl(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(owner) = acl_data.get("Owner") {
                details.insert("Owner".to_string(), owner.clone());
            }
            if let Some(grants) = acl_data.get("Grants") {
                details.insert("Grants".to_string(), grants.clone());
            }
        }

        // Get Public Access Block
        if let Ok(Ok(pab_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_public_access_block(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(has_pab) = pab_data
                .get("HasPublicAccessBlock")
                .and_then(|v| v.as_bool())
            {
                details.insert(
                    "HasPublicAccessBlock".to_string(),
                    serde_json::Value::Bool(has_pab),
                );
            }
            if let Some(block_acls) = pab_data.get("BlockPublicAcls") {
                details.insert("BlockPublicAcls".to_string(), block_acls.clone());
            }
            if let Some(ignore_acls) = pab_data.get("IgnorePublicAcls") {
                details.insert("IgnorePublicAcls".to_string(), ignore_acls.clone());
            }
            if let Some(block_policy) = pab_data.get("BlockPublicPolicy") {
                details.insert("BlockPublicPolicy".to_string(), block_policy.clone());
            }
            if let Some(restrict) = pab_data.get("RestrictPublicBuckets") {
                details.insert("RestrictPublicBuckets".to_string(), restrict.clone());
            }
        }

        // Get Replication
        if let Ok(Ok(replication_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_replication(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(has_replication) = replication_data
                .get("HasReplication")
                .and_then(|v| v.as_bool())
            {
                details.insert(
                    "HasReplication".to_string(),
                    serde_json::Value::Bool(has_replication),
                );
            }
            if let Some(rules) = replication_data.get("Rules") {
                details.insert("ReplicationRules".to_string(), rules.clone());
            }
            if let Some(role) = replication_data.get("Role") {
                details.insert("ReplicationRole".to_string(), role.clone());
            }
        }

        // Get CORS
        if let Ok(Ok(cors_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_cors(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(has_cors) = cors_data.get("HasCORS").and_then(|v| v.as_bool()) {
                details.insert("HasCORS".to_string(), serde_json::Value::Bool(has_cors));
            }
            if let Some(rules) = cors_data.get("CORSRules") {
                details.insert("CORSRules".to_string(), rules.clone());
            }
        }

        // Get Website
        if let Ok(Ok(website_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_website(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(enabled) = website_data.get("WebsiteEnabled").and_then(|v| v.as_bool()) {
                details.insert(
                    "WebsiteEnabled".to_string(),
                    serde_json::Value::Bool(enabled),
                );
            }
            if let Some(index_doc) = website_data.get("IndexDocument") {
                details.insert("WebsiteIndexDocument".to_string(), index_doc.clone());
            }
            if let Some(error_doc) = website_data.get("ErrorDocument") {
                details.insert("WebsiteErrorDocument".to_string(), error_doc.clone());
            }
        }

        // Get Notifications
        if let Ok(Ok(notification_data)) = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.get_bucket_notification(account_id, region, bucket_name),
        )
        .await
        {
            if let Some(has_notifications) = notification_data
                .get("HasNotifications")
                .and_then(|v| v.as_bool())
            {
                details.insert(
                    "HasNotifications".to_string(),
                    serde_json::Value::Bool(has_notifications),
                );
            }
            if let Some(lambda_configs) = notification_data.get("LambdaFunctionConfigurations") {
                details.insert(
                    "LambdaFunctionConfigurations".to_string(),
                    lambda_configs.clone(),
                );
            }
            if let Some(queue_configs) = notification_data.get("QueueConfigurations") {
                details.insert("QueueConfigurations".to_string(), queue_configs.clone());
            }
            if let Some(topic_configs) = notification_data.get("TopicConfigurations") {
                details.insert("TopicConfigurations".to_string(), topic_configs.clone());
            }
        }

        report_status_done("S3", "get_bucket_details", Some(bucket_name));
        Ok(serde_json::Value::Object(details))
    }
}
