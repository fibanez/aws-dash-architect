use super::super::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use aws_sdk_cloudtrail as cloudtrail;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tracing::info;

/// Parameters for CloudTrail LookupEvents API
#[derive(Debug, Clone)]
pub struct LookupEventsParams {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub lookup_attribute: Option<LookupAttribute>,
    pub max_results: usize,
    pub event_category: Option<String>, // "insight" for Insights events (Data events not supported by LookupEvents API)
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
            max_results: 100,
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

            // Add event category if specified (only "insight" is supported)
            if let Some(ref category) = params.event_category {
                if category == "insight" {
                    request = request.event_category(cloudtrail::types::EventCategory::Insight);
                } else if category != "management" {
                    // Log unsupported category
                    info!("⚠️  Event category '{}' not supported by LookupEvents API - using default (management)", category);
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

    // ===== DEDICATED LOOKUP METHODS ===== //
    
    /// Lookup CloudTrail events for a specific resource ID
    pub async fn lookup_events_by_resource_id(
        &self,
        account_id: &str,
        region: &str,
        resource_id: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        let params = LookupEventsParams {
            lookup_attribute: Some(LookupAttribute {
                attribute_key: "ResourceName".to_string(),
                attribute_value: resource_id.to_string(),
            }),
            max_results: max_results.unwrap_or(100),
            ..Default::default()
        };
        
        info!(
            "🔍 Looking up CloudTrail events for resource {} in account {} region {}",
            resource_id, account_id, region
        );
        
        self.lookup_events(account_id, region, params).await
    }

    /// Lookup CloudTrail events for a specific resource type
    pub async fn lookup_events_by_resource_type(
        &self,
        account_id: &str,
        region: &str,
        resource_type: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        let params = LookupEventsParams {
            lookup_attribute: Some(LookupAttribute {
                attribute_key: "ResourceType".to_string(),
                attribute_value: resource_type.to_string(),
            }),
            max_results: max_results.unwrap_or(100),
            ..Default::default()
        };
        
        info!(
            "🔍 Looking up CloudTrail events for resource type {} in account {} region {}",
            resource_type, account_id, region
        );
        
        self.lookup_events(account_id, region, params).await
    }

    /// Lookup CloudTrail events for a specific AWS service
    pub async fn lookup_events_by_service(
        &self,
        account_id: &str,
        region: &str,
        service_name: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        // Map common service names to their EventSource values
        let service_name_lower = service_name.to_lowercase();
        let event_source = match service_name_lower.as_str() {
            // Core compute & storage services
            "s3" => "s3.amazonaws.com",
            "ec2" => "ec2.amazonaws.com", 
            "lambda" => "lambda.amazonaws.com",
            "efs" => "elasticfilesystem.amazonaws.com",
            "ecs" => "ecs.amazonaws.com",
            "eks" => "eks.amazonaws.com",
            "batch" => "batch.amazonaws.com",
            
            // Database services
            "rds" => "rds.amazonaws.com",
            "dynamodb" => "dynamodb.amazonaws.com",
            "elasticache" => "elasticache.amazonaws.com",
            "neptune" => "neptune.amazonaws.com",
            "redshift" => "redshift.amazonaws.com",
            
            // Security & Identity services
            "iam" => "iam.amazonaws.com",
            "sts" => "sts.amazonaws.com",
            "kms" => "kms.amazonaws.com",
            "secretsmanager" | "secrets" => "secretsmanager.amazonaws.com",
            "ssm" => "ssm.amazonaws.com",
            "guardduty" => "guardduty.amazonaws.com",
            "securityhub" => "securityhub.amazonaws.com",
            "acm" => "acm.amazonaws.com",
            "organizations" => "organizations.amazonaws.com",
            
            // Networking services
            "elb" | "elasticloadbalancing" => "elasticloadbalancing.amazonaws.com",
            "elbv2" | "elasticloadbalancingv2" => "elasticloadbalancingv2.amazonaws.com",
            "route53" => "route53.amazonaws.com",
            "apigateway" => "apigateway.amazonaws.com",
            "apigatewayv2" => "apigatewayv2.amazonaws.com",
            "cloudfront" => "cloudfront.amazonaws.com",
            "waf" | "wafv2" => "wafv2.amazonaws.com",
            
            // Messaging & Events
            "sns" => "sns.amazonaws.com",
            "sqs" => "sqs.amazonaws.com",
            "eventbridge" | "events" => "events.amazonaws.com",
            "kinesis" => "kinesis.amazonaws.com",
            "firehose" | "kinesisfirehose" => "firehose.amazonaws.com",
            
            // Developer Tools
            "codecommit" => "codecommit.amazonaws.com",
            "codebuild" => "codebuild.amazonaws.com",
            "codedeploy" => "codedeploy.amazonaws.com",
            "codepipeline" => "codepipeline.amazonaws.com",
            
            // Analytics & ML
            "athena" => "athena.amazonaws.com",
            "glue" => "glue.amazonaws.com",
            "emr" => "elasticmapreduce.amazonaws.com",
            "sagemaker" => "sagemaker.amazonaws.com",
            "opensearch" | "elasticsearch" => "opensearch.amazonaws.com",
            "quicksight" => "quicksight.amazonaws.com",
            
            // Application Integration
            "stepfunctions" | "states" => "states.amazonaws.com",
            "appsync" => "appsync.amazonaws.com",
            "cognito" => "cognito-idp.amazonaws.com",
            
            // Management & Governance
            "cloudformation" | "cfn" => "cloudformation.amazonaws.com",
            "cloudtrail" => "cloudtrail.amazonaws.com",
            "logs" | "cloudwatch-logs" => "logs.amazonaws.com",
            "cloudwatch" => "monitoring.amazonaws.com",
            "config" | "configservice" => "config.amazonaws.com",
            "backup" => "backup.amazonaws.com",
            "transfer" => "transfer.amazonaws.com",
            
            // If it already looks like an event source, use as-is
            name if name.contains(".amazonaws.com") => name,
            // Otherwise assume it's already in the correct format
            _ => service_name,
        };

        let params = LookupEventsParams {
            lookup_attribute: Some(LookupAttribute {
                attribute_key: "EventSource".to_string(),
                attribute_value: event_source.to_string(),
            }),
            max_results: max_results.unwrap_or(100),
            ..Default::default()
        };
        
        info!(
            "🔍 Looking up CloudTrail events for service {} (EventSource: {}) in account {} region {}",
            service_name, event_source, account_id, region
        );
        
        self.lookup_events(account_id, region, params).await
    }

    /// Lookup CloudTrail events for a specific event name
    pub async fn lookup_events_by_event_name(
        &self,
        account_id: &str,
        region: &str,
        event_name: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        let params = LookupEventsParams {
            lookup_attribute: Some(LookupAttribute {
                attribute_key: "EventName".to_string(),
                attribute_value: event_name.to_string(),
            }),
            max_results: max_results.unwrap_or(100),
            ..Default::default()
        };
        
        info!(
            "🔍 Looking up CloudTrail events for event name {} in account {} region {}",
            event_name, account_id, region
        );
        
        self.lookup_events(account_id, region, params).await
    }

    /// Lookup CloudTrail events for a specific username
    pub async fn lookup_events_by_username(
        &self,
        account_id: &str,
        region: &str,
        username: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        let params = LookupEventsParams {
            lookup_attribute: Some(LookupAttribute {
                attribute_key: "Username".to_string(),
                attribute_value: username.to_string(),
            }),
            max_results: max_results.unwrap_or(100),
            ..Default::default()
        };
        
        info!(
            "🔍 Looking up CloudTrail events for username {} in account {} region {}",
            username, account_id, region
        );
        
        self.lookup_events(account_id, region, params).await
    }

    // ===== TIME-BASED CONVENIENCE METHODS ===== //
    
    /// Lookup recent CloudTrail events (last N hours)
    pub async fn lookup_recent_events(
        &self,
        account_id: &str,
        region: &str,
        hours_back: i64,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        let now = Utc::now();
        let start_time = now - Duration::hours(hours_back);
        
        let params = LookupEventsParams {
            start_time: Some(start_time),
            end_time: Some(now),
            max_results: max_results.unwrap_or(100),
            ..Default::default()
        };
        
        info!(
            "🔍 Looking up CloudTrail events from last {} hours in account {} region {}",
            hours_back, account_id, region
        );
        
        self.lookup_events(account_id, region, params).await
    }

    /// Lookup CloudTrail events from the last hour
    pub async fn lookup_events_last_hour(
        &self,
        account_id: &str,
        region: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        self.lookup_recent_events(account_id, region, 1, max_results).await
    }

    /// Lookup CloudTrail events from the last 24 hours
    pub async fn lookup_events_last_24_hours(
        &self,
        account_id: &str,
        region: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        self.lookup_recent_events(account_id, region, 24, max_results).await
    }

    /// Lookup CloudTrail events from the last week
    pub async fn lookup_events_last_week(
        &self,
        account_id: &str,
        region: &str,
        max_results: Option<usize>,
    ) -> Result<Vec<serde_json::Value>> {
        let now = Utc::now();
        let start_time = now - Duration::weeks(1);
        
        let params = LookupEventsParams {
            start_time: Some(start_time),
            end_time: Some(now),
            max_results: max_results.unwrap_or(100),
            ..Default::default()
        };
        
        info!(
            "🔍 Looking up CloudTrail events from last week in account {} region {}",
            account_id, region
        );
        
        self.lookup_events(account_id, region, params).await
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
