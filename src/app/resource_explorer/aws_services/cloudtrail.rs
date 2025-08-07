use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudtrail as cloudtrail;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::info;

/// Parameters for CloudTrail LookupEvents API
#[derive(Debug, Clone)]
pub struct LookupEventsParams {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub lookup_attribute: Option<LookupAttribute>,
    pub max_results: usize,
    pub event_category: Option<String>, // "insight" for Insights events
}

/// Lookup attribute for filtering CloudTrail events
#[derive(Debug, Clone)]
pub struct LookupAttribute {
    pub attribute_key: String,   // EventId, EventName, ReadOnly, Username, ResourceType, ResourceName, EventSource, AccessKeyId
    pub attribute_value: String,
}

impl Default for LookupEventsParams {
    fn default() -> Self {
        Self {
            start_time: None,
            end_time: None,
            lookup_attribute: None,
            max_results: 50,
            event_category: None,
        }
    }
}

#[derive(Clone)]
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

    // ===== DATA PLANE OPERATIONS ===== //
    
    /// Lookup CloudTrail events from the 90-day event history (data plane operation)
    /// This queries events without requiring a trail to be configured
    pub async fn lookup_events(
        &self,
        account_id: &str,
        region: &str,
        params: LookupEventsParams,
    ) -> Result<Vec<serde_json::Value>> {
        info!(
            "🔍 Looking up CloudTrail events for account {} in region {}",
            account_id, region
        );

        // Get credentials for this account/region
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
        let mut all_events = Vec::new();
        let mut next_token: Option<String> = None;
        let mut total_fetched = 0;

        // Paginate through results
        loop {
            let mut request = client.lookup_events();

            // Add lookup attribute filter if provided
            if let Some(ref attr) = params.lookup_attribute {
                let lookup_attr = cloudtrail::types::LookupAttribute::builder()
                    .attribute_key(cloudtrail::types::LookupAttributeKey::from(attr.attribute_key.as_str()))
                    .attribute_value(&attr.attribute_value)
                    .build()?;
                request = request.lookup_attributes(lookup_attr);
            }

            // Add time range
            if let Some(start) = params.start_time {
                request = request.start_time(aws_sdk_cloudtrail::primitives::DateTime::from_millis(
                    start.timestamp_millis(),
                ));
            }

            if let Some(end) = params.end_time {
                request = request.end_time(aws_sdk_cloudtrail::primitives::DateTime::from_millis(
                    end.timestamp_millis(),
                ));
            }

            // Add event category if specified (e.g., "insight")
            if let Some(ref category) = params.event_category {
                if category == "insight" {
                    request = request.event_category(cloudtrail::types::EventCategory::Insight);
                }
            }

            // Add pagination token if we have one
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            // Set max results per request (API limit is 50)
            let remaining = params.max_results - total_fetched;
            let page_size = remaining.min(50) as i32;
            request = request.max_results(page_size);

            // Execute the request
            let response = request.send().await?;

            // Process events
            if let Some(events) = response.events {
                info!("📊 Retrieved {} events from CloudTrail", events.len());
                for event in events {
                    let event_json = self.event_to_json(&event);
                    all_events.push(event_json);
                    total_fetched += 1;
                }
            }

            // Check if we should continue paginating
            next_token = response.next_token;
            if next_token.is_none() || total_fetched >= params.max_results {
                break;
            }
        }

        info!(
            "✅ Successfully retrieved {} CloudTrail events for account {} in region {}",
            all_events.len(),
            account_id,
            region
        );

        Ok(all_events)
    }

    /// Convert CloudTrail event to JSON
    fn event_to_json(&self, event: &cloudtrail::types::Event) -> serde_json::Value {
        let mut json = serde_json::Map::new();

        if let Some(event_id) = &event.event_id {
            json.insert(
                "EventId".to_string(),
                serde_json::Value::String(event_id.clone()),
            );
        }

        if let Some(event_name) = &event.event_name {
            json.insert(
                "EventName".to_string(),
                serde_json::Value::String(event_name.clone()),
            );
        }

        if let Some(read_only) = &event.read_only {
            json.insert(
                "ReadOnly".to_string(),
                serde_json::Value::String(read_only.clone()),
            );
        }

        if let Some(access_key_id) = &event.access_key_id {
            json.insert(
                "AccessKeyId".to_string(),
                serde_json::Value::String(access_key_id.clone()),
            );
        }

        if let Some(event_time) = event.event_time {
            json.insert(
                "EventTime".to_string(),
                serde_json::Value::String(event_time.to_string()),
            );
        }

        if let Some(event_source) = &event.event_source {
            json.insert(
                "EventSource".to_string(),
                serde_json::Value::String(event_source.clone()),
            );
        }

        if let Some(username) = &event.username {
            json.insert(
                "Username".to_string(),
                serde_json::Value::String(username.clone()),
            );
        }

        // Add resources if present
        if let Some(resources) = &event.resources {
            let resources_json: Vec<serde_json::Value> = resources
                .iter()
                .map(|r| {
                    let mut res_map = serde_json::Map::new();
                    if let Some(rt) = &r.resource_type {
                        res_map.insert(
                            "ResourceType".to_string(),
                            serde_json::Value::String(rt.clone()),
                        );
                    }
                    if let Some(rn) = &r.resource_name {
                        res_map.insert(
                            "ResourceName".to_string(),
                            serde_json::Value::String(rn.clone()),
                        );
                    }
                    serde_json::Value::Object(res_map)
                })
                .collect();
            json.insert("Resources".to_string(), serde_json::Value::Array(resources_json));
        }

        // Include the full CloudTrail event JSON if available
        if let Some(cloud_trail_event) = &event.cloud_trail_event {
            // Parse the CloudTrail event JSON string
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(cloud_trail_event) {
                json.insert("CloudTrailEvent".to_string(), parsed);
            } else {
                // If parsing fails, include as string
                json.insert(
                    "CloudTrailEvent".to_string(),
                    serde_json::Value::String(cloud_trail_event.clone()),
                );
            }
        }

        serde_json::Value::Object(json)
    }
}
