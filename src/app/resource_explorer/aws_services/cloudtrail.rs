use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudtrail as cloudtrail;
use std::sync::Arc;

pub struct CloudTrailService {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CloudTrailService {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// List CloudTrail Trails
    pub async fn list_trails(
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

        let client = cloudtrail::Client::new(&aws_config);
        let response = client.list_trails().send().await?;

        let mut trails = Vec::new();
        if let Some(trails_list) = response.trails {
            for trail in trails_list {
                let trail_json = self.trail_info_to_json(&trail);
                trails.push(trail_json);
            }
        }

        Ok(trails)
    }

    /// Get detailed information for specific trail
    pub async fn describe_trail(
        &self,
        account_id: &str,
        region: &str,
        trail_name: &str,
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

        let client = cloudtrail::Client::new(&aws_config);
        self.describe_trail_internal(&client, trail_name).await
    }

    async fn describe_trail_internal(
        &self,
        client: &cloudtrail::Client,
        trail_name: &str,
    ) -> Result<serde_json::Value> {
        let response = client
            .describe_trails()
            .trail_name_list(trail_name)
            .send()
            .await?;

        if let Some(trail_list) = response.trail_list {
            if let Some(trail) = trail_list.first() {
                Ok(self.trail_details_to_json(trail))
            } else {
                Err(anyhow::anyhow!("Trail {} not found", trail_name))
            }
        } else {
            Err(anyhow::anyhow!("Trail {} not found", trail_name))
        }
    }

    fn trail_info_to_json(&self, trail: &cloudtrail::types::TrailInfo) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(trail_arn) = &trail.trail_arn {
            json.insert(
                "TrailArn".to_string(),
                serde_json::Value::String(trail_arn.clone()),
            );
        }

        if let Some(name) = &trail.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(home_region) = &trail.home_region {
            json.insert(
                "HomeRegion".to_string(),
                serde_json::Value::String(home_region.clone()),
            );
        }

        // Add a default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn trail_to_json(&self, trail: &cloudtrail::types::Trail) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(name) = &trail.name {
            json.insert("Name".to_string(), serde_json::Value::String(name.clone()));
            json.insert(
                "ResourceId".to_string(),
                serde_json::Value::String(name.clone()),
            );
        }

        if let Some(s3_bucket_name) = &trail.s3_bucket_name {
            json.insert(
                "S3BucketName".to_string(),
                serde_json::Value::String(s3_bucket_name.clone()),
            );
        }

        if let Some(s3_key_prefix) = &trail.s3_key_prefix {
            json.insert(
                "S3KeyPrefix".to_string(),
                serde_json::Value::String(s3_key_prefix.clone()),
            );
        }

        // sns_topic_name field is deprecated, use sns_topic_arn instead
        // if let Some(sns_topic_name) = &trail.sns_topic_name {
        //     json.insert("SnsTopicName".to_string(), serde_json::Value::String(sns_topic_name.clone()));
        // }

        if let Some(sns_topic_arn) = &trail.sns_topic_arn {
            json.insert(
                "SnsTopicArn".to_string(),
                serde_json::Value::String(sns_topic_arn.clone()),
            );
        }

        json.insert(
            "IncludeGlobalServiceEvents".to_string(),
            serde_json::Value::Bool(trail.include_global_service_events.unwrap_or(false)),
        );

        json.insert(
            "IsMultiRegionTrail".to_string(),
            serde_json::Value::Bool(trail.is_multi_region_trail.unwrap_or(false)),
        );

        if let Some(home_region) = &trail.home_region {
            json.insert(
                "HomeRegion".to_string(),
                serde_json::Value::String(home_region.clone()),
            );
        }

        if let Some(trail_arn) = &trail.trail_arn {
            json.insert(
                "TrailArn".to_string(),
                serde_json::Value::String(trail_arn.clone()),
            );
        }

        json.insert(
            "LogFileValidationEnabled".to_string(),
            serde_json::Value::Bool(trail.log_file_validation_enabled.unwrap_or(false)),
        );

        if let Some(cloud_watch_logs_log_group_arn) = &trail.cloud_watch_logs_log_group_arn {
            json.insert(
                "CloudWatchLogsLogGroupArn".to_string(),
                serde_json::Value::String(cloud_watch_logs_log_group_arn.clone()),
            );
        }

        if let Some(cloud_watch_logs_role_arn) = &trail.cloud_watch_logs_role_arn {
            json.insert(
                "CloudWatchLogsRoleArn".to_string(),
                serde_json::Value::String(cloud_watch_logs_role_arn.clone()),
            );
        }

        if let Some(kms_key_id) = &trail.kms_key_id {
            json.insert(
                "KmsKeyId".to_string(),
                serde_json::Value::String(kms_key_id.clone()),
            );
        }

        json.insert(
            "HasCustomEventSelectors".to_string(),
            serde_json::Value::Bool(trail.has_custom_event_selectors.unwrap_or(false)),
        );

        json.insert(
            "HasInsightSelectors".to_string(),
            serde_json::Value::Bool(trail.has_insight_selectors.unwrap_or(false)),
        );

        json.insert(
            "IsOrganizationTrail".to_string(),
            serde_json::Value::Bool(trail.is_organization_trail.unwrap_or(false)),
        );

        // Add a default status for consistency
        json.insert(
            "Status".to_string(),
            serde_json::Value::String("ACTIVE".to_string()),
        );

        serde_json::Value::Object(json)
    }

    fn trail_details_to_json(&self, trail: &cloudtrail::types::Trail) -> serde_json::Value {
        // For detailed view, we use the same conversion as the basic one
        // In a real implementation, we might fetch additional details like event selectors
        self.trail_to_json(trail)
    }
}
