use super::super::credentials::CredentialCoordinator;
use super::super::query_timing;
use super::super::state::ResourceTag;
use anyhow::{Context, Result};
use aws_sdk_acm as acm;
use aws_sdk_amplify as amplify;
use aws_sdk_appsync as appsync;
use aws_sdk_batch as batch;
use aws_sdk_cloudfront as cloudfront;
use aws_sdk_config as config;
use aws_sdk_dynamodb as dynamodb;
use aws_sdk_ec2 as ec2;
use aws_sdk_ecs as ecs;
use aws_sdk_eks as eks;
use aws_sdk_iam as iam;
use aws_sdk_kms as kms;
use aws_sdk_lambda as lambda;
use aws_sdk_lexmodelsv2 as lex;
use aws_sdk_organizations as organizations;
use aws_sdk_quicksight as quicksight;
use aws_sdk_rds as rds;
use aws_sdk_resourcegroupstagging as tagging;
use aws_sdk_route53 as route53;
use aws_sdk_s3 as s3;
use aws_sdk_sns as sns;
use aws_sdk_sqs as sqs;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{timeout, Duration};
use tokio::sync::RwLock;

/// Service for fetching tags from AWS using multiple strategies
///
/// This service provides tag fetching capabilities using:
/// 1. AWS Resource Groups Tagging API (universal, works for most services)
/// 2. Service-specific tag APIs (S3, IAM, etc.)
///
/// # Tag Caching
///
/// Results are cached with a 5-minute TTL to minimize API calls. The cache
/// is automatically managed and thread-safe.
///
/// # Example
///
/// ```rust,ignore
/// let service = ResourceTaggingService::new(credential_coordinator);
///
/// // Get all tag keys across resources
/// let tag_keys = service.get_tag_keys("123456789012", "us-east-1").await?;
///
/// // Get values for a specific tag key
/// let values = service.get_tag_values("123456789012", "us-east-1", "Environment").await?;
///
/// // Get tags for a specific resource using ARN
/// let tags = service.get_tags_for_arn("123456789012", "us-east-1", "arn:aws:...").await?;
/// ```
pub struct ResourceTaggingService {
    credential_coordinator: Arc<CredentialCoordinator>,
    tag_keys_cache: Arc<RwLock<HashMap<String, CachedTagKeys>>>,
    tag_values_cache: Arc<RwLock<HashMap<String, CachedTagValues>>>,
}

struct CachedTagKeys {
    keys: Vec<String>,
    timestamp: DateTime<Utc>,
}

struct CachedTagValues {
    values: Vec<String>,
    timestamp: DateTime<Utc>,
}

impl ResourceTaggingService {
    const CACHE_TTL_MINUTES: i64 = 5;

    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
            tag_keys_cache: Arc::new(RwLock::new(HashMap::new())),
            tag_values_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get all resources with their tags, optionally filtered by resource type and/or tags
    ///
    /// Uses the AWS Resource Groups Tagging API which works across most AWS services.
    ///
    /// # Arguments
    ///
    /// * `account_id` - The AWS account ID
    /// * `region` - The AWS region
    /// * `resource_type_filter` - Optional filter like "ec2:instance" or "s3:bucket"
    /// * `tag_filters` - Optional map of tag keys to values for filtering
    ///
    /// # Returns
    ///
    /// Vector of (resource_arn, tags) tuples
    pub async fn get_resources_with_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_type_filter: Option<&str>,
        tag_filters: Option<HashMap<String, Vec<String>>>,
    ) -> Result<Vec<(String, Vec<ResourceTag>)>> {
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

        let client = tagging::Client::new(&aws_config);
        let mut resources = Vec::new();

        // Build request
        let mut request = client.get_resources();

        if let Some(resource_type) = resource_type_filter {
            request = request.resource_type_filters(resource_type);
        }

        if let Some(tag_filters) = tag_filters {
            for (key, values) in tag_filters {
                let tag_filter = tagging::types::TagFilter::builder()
                    .key(key)
                    .set_values(Some(values))
                    .build();
                request = request.tag_filters(tag_filter);
            }
        }

        // Paginate through results
        let mut paginator = request.into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page.context("Failed to fetch page of resources with tags")?;

            if let Some(resource_tag_mapping_list) = page.resource_tag_mapping_list {
                for mapping in resource_tag_mapping_list {
                    let arn = mapping.resource_arn.unwrap_or_default();
                    let tags: Vec<ResourceTag> = mapping
                        .tags
                        .unwrap_or_default()
                        .into_iter()
                        .map(|tag| ResourceTag {
                            key: tag.key,
                            value: tag.value,
                        })
                        .collect();

                    resources.push((arn, tags));
                }
            }
        }

        tracing::debug!(
            "Fetched {} resources with tags from Resource Groups API in {}/{}",
            resources.len(),
            account_id,
            region
        );

        Ok(resources)
    }

    /// Get all tag keys in use across resources in an account/region
    ///
    /// Results are cached for 5 minutes to reduce API calls.
    /// Useful for tag discovery and autocomplete in tag filter UI.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let keys = service.get_tag_keys("123456789012", "us-east-1").await?;
    /// // Returns: ["Environment", "Team", "Project", "CostCenter", ...]
    /// ```
    pub async fn get_tag_keys(&self, account_id: &str, region: &str) -> Result<Vec<String>> {
        let cache_key = format!("{}:{}", account_id, region);

        // Check cache
        {
            let cache = self.tag_keys_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                let age = Utc::now() - cached.timestamp;
                if age < ChronoDuration::minutes(Self::CACHE_TTL_MINUTES) {
                    tracing::debug!("Tag keys cache hit for {}/{}", account_id, region);
                    return Ok(cached.keys.clone());
                }
            }
        }

        // Fetch from API
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

        let client = tagging::Client::new(&aws_config);
        let mut tag_keys = Vec::new();

        let mut paginator = client.get_tag_keys().into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page.context("Failed to fetch tag keys")?;
            if let Some(keys) = page.tag_keys {
                tag_keys.extend(keys);
            }
        }

        // Sort for consistent ordering
        tag_keys.sort();
        tag_keys.dedup();

        tracing::debug!(
            "Discovered {} unique tag keys in {}/{}",
            tag_keys.len(),
            account_id,
            region
        );

        // Update cache
        {
            let mut cache = self.tag_keys_cache.write().await;
            cache.insert(
                cache_key,
                CachedTagKeys {
                    keys: tag_keys.clone(),
                    timestamp: Utc::now(),
                },
            );
        }

        Ok(tag_keys)
    }

    /// Get all values in use for a specific tag key
    ///
    /// Results are cached for 5 minutes to reduce API calls.
    /// Useful for autocomplete in tag filter UI.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let values = service.get_tag_values("123456789012", "us-east-1", "Environment").await?;
    /// // Returns: ["Production", "Staging", "Development"]
    /// ```
    pub async fn get_tag_values(
        &self,
        account_id: &str,
        region: &str,
        tag_key: &str,
    ) -> Result<Vec<String>> {
        let cache_key = format!("{}:{}:{}", account_id, region, tag_key);

        // Check cache
        {
            let cache = self.tag_values_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                let age = Utc::now() - cached.timestamp;
                if age < ChronoDuration::minutes(Self::CACHE_TTL_MINUTES) {
                    tracing::debug!(
                        "Tag values cache hit for {}/{}/{}",
                        account_id,
                        region,
                        tag_key
                    );
                    return Ok(cached.values.clone());
                }
            }
        }

        // Fetch from API
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

        let client = tagging::Client::new(&aws_config);
        let mut tag_values = Vec::new();

        let mut paginator = client.get_tag_values().key(tag_key).into_paginator().send();

        while let Some(page) = paginator.next().await {
            let page = page.context("Failed to fetch tag values")?;
            if let Some(values) = page.tag_values {
                tag_values.extend(values);
            }
        }

        // Sort for consistent ordering
        tag_values.sort();
        tag_values.dedup();

        tracing::debug!(
            "Discovered {} unique values for tag key '{}' in {}/{}",
            tag_values.len(),
            tag_key,
            account_id,
            region
        );

        // Update cache
        {
            let mut cache = self.tag_values_cache.write().await;
            cache.insert(
                cache_key,
                CachedTagValues {
                    values: tag_values.clone(),
                    timestamp: Utc::now(),
                },
            );
        }

        Ok(tag_values)
    }

    /// Get tags for a specific resource using its ARN
    ///
    /// Uses the AWS Resource Groups Tagging API.
    /// For services that don't support this API, use service-specific methods.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let arn = "arn:aws:ec2:us-east-1:123456789012:instance/i-1234567890abcdef0";
    /// let tags = service.get_tags_for_arn("123456789012", "us-east-1", arn).await?;
    /// ```
    pub async fn get_tags_for_arn(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let start = Instant::now();

        // Detect service from ARN for tracking (arn:aws:SERVICE:region:account:...)
        let service = resource_arn
            .split(':')
            .nth(2)
            .map(|s| match s {
                "logs" => "Logs",
                "lambda" => "Lambda",
                "ec2" => "EC2",
                "iam" => "IAM",
                "s3" => "S3",
                _ => "Other",
            })
            .unwrap_or("Other");

        // Track tag fetch start - returns unique operation ID for tracking
        let op_id = query_timing::tag_fetch_start(service, resource_arn, region, account_id);

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .inspect_err(|_e| {
                query_timing::tag_fetch_end(op_id, service, resource_arn, region, start.elapsed().as_millis(), 0, false);
            })
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = tagging::Client::new(&aws_config);

        let response = match client
            .get_resources()
            .resource_arn_list(resource_arn)
            .send()
            .await {
            Ok(resp) => resp,
            Err(e) => {
                query_timing::tag_fetch_end(op_id, service, resource_arn, region, start.elapsed().as_millis(), 0, false);
                return Err(e).context("Failed to fetch tags for resource ARN");
            }
        };

        if let Some(mappings) = response.resource_tag_mapping_list {
            if let Some(mapping) = mappings.first() {
                let tags: Vec<ResourceTag> = mapping
                    .tags
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|tag| ResourceTag {
                        key: tag.key,
                        value: tag.value,
                    })
                    .collect();

                // Log success
                query_timing::tag_fetch_end(op_id, service, resource_arn, region, start.elapsed().as_millis(), tags.len(), true);

                tracing::debug!("Fetched {} tags for ARN {}", tags.len(), resource_arn);
                return Ok(tags);
            }
        }

        // No tags found (success with 0 tags)
        query_timing::tag_fetch_end(op_id, service, resource_arn, region, start.elapsed().as_millis(), 0, true);

        Ok(Vec::new())
    }

    //
    // Service-Specific Tag Fetching Methods
    //
    // These methods are used when the Resource Groups Tagging API doesn't work
    // or when we need more detailed control over tag fetching.
    //

    /// Fetch tags for EC2 resources using EC2-specific API
    ///
    /// Works for: instances, volumes, VPCs, subnets, security groups, etc.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tags = service.get_ec2_tags("123456789012", "us-east-1", "i-1234567890abcdef0").await?;
    /// ```
    pub async fn get_ec2_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_id: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = ec2::Client::new(&aws_config);

        let response = client
            .describe_tags()
            .filters(
                ec2::types::Filter::builder()
                    .name("resource-id")
                    .values(resource_id)
                    .build(),
            )
            .send()
            .await
            .context("Failed to fetch EC2 tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tag| {
                let key = tag.key?;
                let value = tag.value?;
                Some(ResourceTag { key, value })
            })
            .collect();

        tracing::debug!(
            "Fetched {} EC2 tags for resource {}",
            tags.len(),
            resource_id
        );
        Ok(tags)
    }

    /// Fetch tags for S3 buckets
    ///
    /// S3 uses bucket name instead of ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tags = service.get_s3_bucket_tags("123456789012", "us-east-1", "my-bucket").await?;
    /// ```
    pub async fn get_s3_bucket_tags(
        &self,
        account_id: &str,
        region: &str,
        bucket_name: &str,
    ) -> Result<Vec<ResourceTag>> {
        let start = Instant::now();

        // Track tag fetch start - returns unique operation ID for tracking
        let op_id = query_timing::tag_fetch_start("S3", bucket_name, region, account_id);

        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .inspect_err(|_e| {
                query_timing::tag_fetch_end(op_id, "S3", bucket_name, region, start.elapsed().as_millis(), 0, false);
            })?;

        let client = s3::Client::new(&aws_config);

        // Time the actual S3 API call
        let api_start = Instant::now();
        let response = match client.get_bucket_tagging().bucket(bucket_name).send().await {
            Ok(resp) => resp,
            Err(e) => {
                // Handle NoSuchTagSet error - bucket simply has no tags
                let error_str = format!("{:?}", e);
                if error_str.contains("NoSuchTagSet") || error_str.contains("NoSuchTagSetError") {
                    tracing::debug!(
                        "S3 bucket {} has no tags (NoSuchTagSet)",
                        bucket_name
                    );
                    // Log success with 0 tags (NoSuchTagSet is not an error)
                    query_timing::tag_fetch_end(op_id, "S3", bucket_name, region, start.elapsed().as_millis(), 0, true);
                    return Ok(Vec::new());
                }
                // For other errors, log failure and propagate
                query_timing::tag_fetch_end(op_id, "S3", bucket_name, region, start.elapsed().as_millis(), 0, false);
                return Err(e).context(format!(
                    "Failed to fetch S3 bucket tags for {} (API call took {}ms)",
                    bucket_name, api_start.elapsed().as_millis()
                ));
            }
        };

        let tags: Vec<ResourceTag> = response
            .tag_set
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        // Log success
        query_timing::tag_fetch_end(op_id, "S3", bucket_name, region, start.elapsed().as_millis(), tags.len(), true);

        tracing::debug!("Fetched {} S3 tags for bucket {}", tags.len(), bucket_name);
        Ok(tags)
    }

    /// Fetch tags for Lambda functions
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let arn = "arn:aws:lambda:us-east-1:123456789012:function:my-function";
    /// let tags = service.get_lambda_tags("123456789012", "us-east-1", arn).await?;
    /// ```
    pub async fn get_lambda_tags(
        &self,
        account_id: &str,
        region: &str,
        function_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = lambda::Client::new(&aws_config);

        // Use timeout to prevent hanging on unresponsive Lambda API calls
        let response = timeout(
            Duration::from_secs(10),
            client.list_tags().resource(function_arn).send(),
        )
        .await
        .context("Lambda list_tags timed out after 10s")?
        .context("Failed to fetch Lambda tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| ResourceTag { key, value })
            .collect();

        tracing::debug!(
            "Fetched {} Lambda tags for function {}",
            tags.len(),
            function_arn
        );
        Ok(tags)
    }

    /// Fetch tags for IAM users
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tags = service.get_iam_user_tags("123456789012", "us-east-1", "john-doe").await?;
    /// ```
    pub async fn get_iam_user_tags(
        &self,
        account_id: &str,
        _region: &str, // IAM is global
        user_name: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, "us-east-1")
            .await?;

        let client = iam::Client::new(&aws_config);

        // Use timeout to prevent hanging on unresponsive IAM API calls
        let response = timeout(
            Duration::from_secs(10),
            client.list_user_tags().user_name(user_name).send(),
        )
        .await
        .context("IAM list_user_tags timed out after 10s")?
        .context("Failed to fetch IAM user tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        tracing::debug!("Fetched {} IAM user tags for {}", tags.len(), user_name);
        Ok(tags)
    }

    /// Fetch tags for IAM roles
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tags = service.get_iam_role_tags("123456789012", "us-east-1", "MyRole").await?;
    /// ```
    pub async fn get_iam_role_tags(
        &self,
        account_id: &str,
        _region: &str, // IAM is global
        role_name: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, "us-east-1")
            .await?;

        let client = iam::Client::new(&aws_config);

        // Use timeout to prevent hanging on unresponsive IAM API calls
        let response = timeout(
            Duration::from_secs(10),
            client.list_role_tags().role_name(role_name).send(),
        )
        .await
        .context("IAM list_role_tags timed out after 10s")?
        .context("Failed to fetch IAM role tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        tracing::debug!("Fetched {} IAM role tags for {}", tags.len(), role_name);
        Ok(tags)
    }

    /// Fetch tags for IAM policies
    ///
    /// IAM policies use ARN instead of name for tagging.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let policy_arn = "arn:aws:iam::123456789012:policy/MyPolicy";
    /// let tags = service.get_iam_policy_tags("123456789012", "us-east-1", policy_arn).await?;
    /// ```
    pub async fn get_iam_policy_tags(
        &self,
        account_id: &str,
        _region: &str, // IAM is global
        policy_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, "us-east-1")
            .await?;

        let client = iam::Client::new(&aws_config);

        // Use timeout to prevent hanging on unresponsive IAM API calls
        let response = timeout(
            Duration::from_secs(10),
            client.list_policy_tags().policy_arn(policy_arn).send(),
        )
        .await
        .context("IAM list_policy_tags timed out after 10s")?
        .context("Failed to fetch IAM policy tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        tracing::debug!("Fetched {} IAM policy tags for {}", tags.len(), policy_arn);
        Ok(tags)
    }

    /// Fetch tags for IAM server certificates
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tags = service.get_iam_server_certificate_tags("123456789012", "us-east-1", "MyCertificate").await?;
    /// ```
    pub async fn get_iam_server_certificate_tags(
        &self,
        account_id: &str,
        _region: &str, // IAM is global
        certificate_name: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, "us-east-1")
            .await?;

        let client = iam::Client::new(&aws_config);

        let response = client
            .list_server_certificate_tags()
            .server_certificate_name(certificate_name)
            .send()
            .await
            .context("Failed to fetch IAM server certificate tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        tracing::debug!(
            "Fetched {} IAM server certificate tags for {}",
            tags.len(),
            certificate_name
        );
        Ok(tags)
    }

    /// Fetch tags for Organizations resources
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tags = service.get_organizations_tags("123456789012", "us-east-1", "o-1234567890").await?;
    /// ```
    pub async fn get_organizations_tags(
        &self,
        account_id: &str,
        _region: &str, // Organizations is global
        resource_id: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, "us-east-1")
            .await?;

        let client = organizations::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_id(resource_id)
            .send()
            .await
            .context("Failed to fetch Organizations tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        tracing::debug!(
            "Fetched {} Organizations tags for {}",
            tags.len(),
            resource_id
        );
        Ok(tags)
    }

    /// Fetch tags for RDS resources
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let arn = "arn:aws:rds:us-east-1:123456789012:db:my-database";
    /// let tags = service.get_rds_tags("123456789012", "us-east-1", arn).await?;
    /// ```
    pub async fn get_rds_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = rds::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_name(resource_arn)
            .send()
            .await
            .context("Failed to fetch RDS tags")?;

        let tags: Vec<ResourceTag> = response
            .tag_list
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tag| {
                let key = tag.key?;
                let value = tag.value?;
                Some(ResourceTag { key, value })
            })
            .collect();

        tracing::debug!(
            "Fetched {} RDS tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for DynamoDB tables
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let arn = "arn:aws:dynamodb:us-east-1:123456789012:table/MyTable";
    /// let tags = service.get_dynamodb_tags("123456789012", "us-east-1", arn).await?;
    /// ```
    pub async fn get_dynamodb_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = dynamodb::Client::new(&aws_config);

        let response = client
            .list_tags_of_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch DynamoDB tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        tracing::debug!(
            "Fetched {} DynamoDB tags for table {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for SQS queues
    ///
    /// SQS uses queue URL instead of ARN for tag operations.
    /// Queue URL format: https://sqs.{region}.amazonaws.com/{account}/{queue-name}
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tags = service.get_sqs_queue_tags("123456789012", "us-east-1", "my-queue").await?;
    /// ```
    pub async fn get_sqs_queue_tags(
        &self,
        account_id: &str,
        region: &str,
        queue_name: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sqs::Client::new(&aws_config);

        // Construct queue URL from queue name
        // Format: https://sqs.{region}.amazonaws.com/{account}/{queue-name}
        let queue_url = format!(
            "https://sqs.{}.amazonaws.com/{}/{}",
            region, account_id, queue_name
        );

        let response = client
            .list_queue_tags()
            .queue_url(&queue_url)
            .send()
            .await
            .context("Failed to fetch SQS queue tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| ResourceTag { key, value })
            .collect();

        tracing::debug!(
            "Fetched {} SQS tags for queue {} (url: {})",
            tags.len(),
            queue_name,
            queue_url
        );
        Ok(tags)
    }

    /// Fetch tags for SNS topics
    ///
    /// SNS uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let topic_arn = "arn:aws:sns:us-east-1:123456789012:my-topic";
    /// let tags = service.get_sns_topic_tags("123456789012", "us-east-1", topic_arn).await?;
    /// ```
    pub async fn get_sns_topic_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = sns::Client::new(&aws_config);

        // Use timeout to prevent hanging on unresponsive SNS API calls
        let response = timeout(
            Duration::from_secs(10),
            client.list_tags_for_resource().resource_arn(resource_arn).send(),
        )
        .await
        .context("SNS list_tags_for_resource timed out after 10s")?
        .context("Failed to fetch SNS topic tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        tracing::debug!("Fetched {} SNS tags for topic {}", tags.len(), resource_arn);
        Ok(tags)
    }

    /// Fetch tags for KMS keys
    ///
    /// KMS accepts key ID, ARN, or alias for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let key_id = "1234abcd-12ab-34cd-56ef-1234567890ab";
    /// let tags = service.get_kms_key_tags("123456789012", "us-east-1", key_id).await?;
    /// ```
    pub async fn get_kms_key_tags(
        &self,
        account_id: &str,
        region: &str,
        key_id: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = kms::Client::new(&aws_config);

        // Use timeout to prevent hanging on unresponsive KMS API calls
        let response = timeout(
            Duration::from_secs(10),
            client.list_resource_tags().key_id(key_id).send(),
        )
        .await
        .context("KMS list_resource_tags timed out after 10s")?
        .context("Failed to fetch KMS key tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.tag_key,
                value: tag.tag_value,
            })
            .collect();

        tracing::debug!("Fetched {} KMS tags for key {}", tags.len(), key_id);
        Ok(tags)
    }

    /// Fetch tags for CloudFront distributions
    ///
    /// CloudFront uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let distribution_arn = "arn:aws:cloudfront::123456789012:distribution/E1234567890ABC";
    /// let tags = service.get_cloudfront_distribution_tags("123456789012", "us-east-1", distribution_arn).await?;
    /// ```
    pub async fn get_cloudfront_distribution_tags(
        &self,
        account_id: &str,
        _region: &str, // CloudFront is global
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, "us-east-1")
            .await?;

        let client = cloudfront::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource(resource_arn)
            .send()
            .await
            .context("Failed to fetch CloudFront distribution tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .and_then(|tags| tags.items)
            .unwrap_or_default()
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value.unwrap_or_default(),
            })
            .collect();

        tracing::debug!(
            "Fetched {} CloudFront tags for distribution {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for EKS resources (clusters and Fargate profiles)
    ///
    /// EKS uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cluster_arn = "arn:aws:eks:us-east-1:123456789012:cluster/my-cluster";
    /// let tags = service.get_eks_resource_tags("123456789012", "us-east-1", cluster_arn).await?;
    /// ```
    pub async fn get_eks_resource_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = eks::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch EKS resource tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| ResourceTag { key, value })
            .collect();

        tracing::debug!(
            "Fetched {} EKS tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for ECS resources (clusters, services, tasks, task definitions)
    ///
    /// ECS uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cluster_arn = "arn:aws:ecs:us-east-1:123456789012:cluster/my-cluster";
    /// let tags = service.get_ecs_resource_tags("123456789012", "us-east-1", cluster_arn).await?;
    /// ```
    pub async fn get_ecs_resource_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = ecs::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch ECS resource tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key.unwrap_or_default(),
                value: tag.value.unwrap_or_default(),
            })
            .collect();

        tracing::debug!(
            "Fetched {} ECS tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for Route53 hosted zones
    ///
    /// Route53 uses a dedicated tag API with resource type and ID.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let zone_id = "/hostedzone/Z1234567890ABC";
    /// let tags = service.get_route53_hosted_zone_tags("123456789012", "us-east-1", zone_id).await?;
    /// ```
    pub async fn get_route53_hosted_zone_tags(
        &self,
        account_id: &str,
        _region: &str, // Route53 is global
        resource_id: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, "us-east-1")
            .await?;

        let client = route53::Client::new(&aws_config);

        // Strip /hostedzone/ prefix if present
        let zone_id = resource_id.trim_start_matches("/hostedzone/");

        let response = client
            .list_tags_for_resource()
            .resource_type(route53::types::TagResourceType::Hostedzone)
            .resource_id(zone_id)
            .send()
            .await
            .context("Failed to fetch Route53 hosted zone tags")?;

        let tags: Vec<ResourceTag> = response
            .resource_tag_set
            .and_then(|set| set.tags)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tag| {
                let key = tag.key?;
                let value = tag.value;
                Some(ResourceTag {
                    key,
                    value: value.unwrap_or_default(),
                })
            })
            .collect();

        tracing::debug!(
            "Fetched {} Route53 tags for hosted zone {}",
            tags.len(),
            zone_id
        );
        Ok(tags)
    }

    /// Fetch tags for Lex v2 resources (bots, bot aliases)
    ///
    /// Lex v2 uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bot_arn = "arn:aws:lex:us-east-1:123456789012:bot/MYBOT123";
    /// let tags = service.get_lex_tags("123456789012", "us-east-1", bot_arn).await?;
    /// ```
    pub async fn get_lex_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = lex::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch Lex tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| ResourceTag { key, value })
            .collect();

        tracing::debug!(
            "Fetched {} Lex tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for QuickSight resources (dashboards, datasets, data sources)
    ///
    /// QuickSight uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let dashboard_arn = "arn:aws:quicksight:us-east-1:123456789012:dashboard/dashboard-id";
    /// let tags = service.get_quicksight_tags("123456789012", "us-east-1", dashboard_arn).await?;
    /// ```
    pub async fn get_quicksight_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = quicksight::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch QuickSight tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value,
            })
            .collect();

        tracing::debug!(
            "Fetched {} QuickSight tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for Batch resources (compute environments, job queues)
    ///
    /// Batch uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let env_arn = "arn:aws:batch:us-east-1:123456789012:compute-environment/my-env";
    /// let tags = service.get_batch_tags("123456789012", "us-east-1", env_arn).await?;
    /// ```
    pub async fn get_batch_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = batch::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch Batch tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| ResourceTag { key, value })
            .collect();

        tracing::debug!(
            "Fetched {} Batch tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for ACM certificates
    ///
    /// ACM uses certificate ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let cert_arn = "arn:aws:acm:us-east-1:123456789012:certificate/12345678-1234-1234-1234-123456789012";
    /// let tags = service.get_acm_certificate_tags("123456789012", "us-east-1", cert_arn).await?;
    /// ```
    pub async fn get_acm_certificate_tags(
        &self,
        account_id: &str,
        region: &str,
        certificate_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = acm::Client::new(&aws_config);

        let response = client
            .list_tags_for_certificate()
            .certificate_arn(certificate_arn)
            .send()
            .await
            .context("Failed to fetch ACM certificate tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| ResourceTag {
                key: tag.key,
                value: tag.value.unwrap_or_default(),
            })
            .collect();

        tracing::debug!(
            "Fetched {} ACM tags for certificate {}",
            tags.len(),
            certificate_arn
        );
        Ok(tags)
    }

    /// Fetch tags for Amplify apps
    ///
    /// Amplify uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let app_arn = "arn:aws:amplify:us-east-1:123456789012:apps/d1234567890";
    /// let tags = service.get_amplify_tags("123456789012", "us-east-1", app_arn).await?;
    /// ```
    pub async fn get_amplify_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = amplify::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch Amplify tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| ResourceTag { key, value })
            .collect();

        tracing::debug!(
            "Fetched {} Amplify tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for AppSync GraphQL APIs
    ///
    /// AppSync uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let api_arn = "arn:aws:appsync:us-east-1:123456789012:apis/abcd1234efgh5678ijkl9012";
    /// let tags = service.get_appsync_tags("123456789012", "us-east-1", api_arn).await?;
    /// ```
    pub async fn get_appsync_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = appsync::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch AppSync tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|(key, value)| ResourceTag { key, value })
            .collect();

        tracing::debug!(
            "Fetched {} AppSync tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }

    /// Fetch tags for AWS Config resources (config rules, configuration recorders)
    ///
    /// Config uses ARN for tag operations.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let rule_arn = "arn:aws:config:us-east-1:123456789012:config-rule/my-rule";
    /// let tags = service.get_config_tags("123456789012", "us-east-1", rule_arn).await?;
    /// ```
    pub async fn get_config_tags(
        &self,
        account_id: &str,
        region: &str,
        resource_arn: &str,
    ) -> Result<Vec<ResourceTag>> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await?;

        let client = config::Client::new(&aws_config);

        let response = client
            .list_tags_for_resource()
            .resource_arn(resource_arn)
            .send()
            .await
            .context("Failed to fetch Config tags")?;

        let tags: Vec<ResourceTag> = response
            .tags
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tag| {
                let key = tag.key?;
                let value = tag.value;
                Some(ResourceTag {
                    key,
                    value: value.unwrap_or_default(),
                })
            })
            .collect();

        tracing::debug!(
            "Fetched {} Config tags for resource {}",
            tags.len(),
            resource_arn
        );
        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_tagging_service_creation() {
        // This test just ensures the struct can be created
        // Real tests would require mocking AWS API calls
        let creds = Arc::new(CredentialCoordinator::new_mock());
        let _service = ResourceTaggingService::new(creds);
        // Service created successfully
    }
}
