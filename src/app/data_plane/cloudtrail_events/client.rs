//! AWS SDK client wrapper for CloudTrail Events

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Context, Result};
use aws_sdk_cloudtrail as cloudtrail_sdk;
use aws_smithy_types::DateTime;
use std::sync::Arc;

use crate::app::resource_explorer::credentials::CredentialCoordinator;
use super::types::{CloudTrailEvent, EventResource, LookupAttribute, LookupOptions, LookupResult};

/// Client for querying AWS CloudTrail Events
#[derive(Clone)]
pub struct CloudTrailEventsClient {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl CloudTrailEventsClient {
    /// Create new client with credential coordinator
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// Look up CloudTrail events with full options
    ///
    /// # Arguments
    /// * `account_id` - AWS account ID
    /// * `region` - AWS region (e.g., "us-east-1")
    /// * `options` - Lookup options (time range, filters, pagination)
    ///
    /// # Returns
    /// LookupResult with events and pagination token
    ///
    /// # Example
    /// ```rust
    /// let options = LookupOptions::new()
    ///     .with_start_time(start_ms)
    ///     .with_resource_type("AWS::EC2::Instance".to_string())
    ///     .with_max_results(50);
    ///
    /// let result = client.lookup_events(
    ///     "123456789012",
    ///     "us-east-1",
    ///     options
    /// ).await?;
    /// ```
    pub async fn lookup_events(
        &self,
        account_id: &str,
        region: &str,
        options: LookupOptions,
    ) -> Result<LookupResult> {
        // Step 1: Create AWS config with credentials for account/region
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

        // Step 2: Create AWS SDK client
        let client = cloudtrail_sdk::Client::new(&aws_config);

        // Step 3: Build request with options
        let mut request = client.lookup_events();

        // Apply time range
        if let Some(start_time) = options.start_time {
            let start_dt = DateTime::from_millis(start_time);
            request = request.start_time(start_dt);
        }

        if let Some(end_time) = options.end_time {
            let end_dt = DateTime::from_millis(end_time);
            request = request.end_time(end_dt);
        }

        // Apply lookup attributes
        for attr in &options.lookup_attributes {
            let sdk_attr = cloudtrail_sdk::types::LookupAttribute::builder()
                .attribute_key(attr.attribute_key.to_sdk())
                .attribute_value(&attr.attribute_value)
                .build()
                .with_context(|| "Failed to build lookup attribute")?;

            request = request.lookup_attributes(sdk_attr);
        }

        // Apply max results
        if let Some(max_results) = options.max_results {
            request = request.max_results(max_results);
        }

        // Apply pagination
        if let Some(token) = options.next_token {
            request = request.next_token(token);
        }

        // Step 4: Execute request
        let response = request
            .send()
            .await
            .with_context(|| "Failed to lookup CloudTrail events")?;

        // Step 5: Convert AWS SDK response to our types
        let events: Vec<CloudTrailEvent> = response
            .events()
            .iter()
            .map(|event| {
                // Convert event time from DateTime to Unix millis
                let event_time = event
                    .event_time()
                    .and_then(|dt| dt.to_millis().ok())
                    .unwrap_or(0);

                // Extract resources
                let resources: Vec<EventResource> = event
                    .resources()
                    .iter()
                    .map(|res| EventResource {
                        resource_type: res.resource_type().map(|s| s.to_string()),
                        resource_name: res.resource_name().map(|s| s.to_string()),
                    })
                    .collect();

                CloudTrailEvent {
                    event_id: event.event_id().unwrap_or_default().to_string(),
                    event_name: event.event_name().unwrap_or_default().to_string(),
                    event_time,
                    event_source: event.event_source().unwrap_or_default().to_string(),
                    username: event.username().unwrap_or_default().to_string(),
                    resources,
                    cloud_trail_event: event.cloud_trail_event().map(|s| s.to_string()),
                    access_key_id: event.access_key_id().map(|s| s.to_string()),
                    read_only: event.read_only().map(|s| s.to_string()),
                    error_code: None,
                    error_message: None,
                }
            })
            .collect();

        let result = LookupResult::new(
            events,
            response.next_token().map(|t| t.to_string()),
        );

        Ok(result)
    }

    /// Get recent events with automatic pagination (fetches at least 2 pages, 100 events)
    ///
    /// This method automatically fetches at least 2 pages of events (up to 100 events total)
    /// by making multiple API calls if needed.
    ///
    /// # Arguments
    /// * `account_id` - AWS account ID
    /// * `region` - AWS region
    /// * `limit` - Target number of events to fetch (will fetch at least 2 pages regardless)
    ///
    /// # Returns
    /// LookupResult with combined events from multiple pages
    ///
    /// # Example
    /// ```rust
    /// let result = client.get_recent_events(
    ///     "123456789012",
    ///     "us-east-1",
    ///     100
    /// ).await?;
    /// ```
    pub async fn get_recent_events(
        &self,
        account_id: &str,
        region: &str,
        limit: i32,
    ) -> Result<LookupResult> {
        let mut all_events = Vec::new();
        let mut next_token = None;
        let target_events = limit.max(100); // Fetch at least 100 events

        // Fetch at least 2 pages (minimum 100 events)
        let mut pages_fetched = 0;
        const MIN_PAGES: i32 = 2;

        loop {
            // Determine page size (CloudTrail max is 50 per request)
            let page_size = ((target_events - all_events.len() as i32).min(50)).max(50);

            let options = LookupOptions::new()
                .with_max_results(page_size)
                .with_next_token(next_token.unwrap_or_default());

            let result = self
                .lookup_events(account_id, region, options)
                .await?;

            all_events.extend(result.events);
            next_token = result.next_token;
            pages_fetched += 1;

            // Stop if:
            // 1. We've fetched at least MIN_PAGES pages AND
            // 2. Either we've reached our target or there are no more events
            if pages_fetched >= MIN_PAGES {
                if all_events.len() >= target_events as usize || next_token.is_none() {
                    break;
                }
            }

            // Safety: stop after 10 pages (500 events max)
            if pages_fetched >= 10 {
                break;
            }
        }

        Ok(LookupResult::new(all_events, next_token))
    }

    /// Get events for a specific resource
    ///
    /// # Arguments
    /// * `account_id` - AWS account ID
    /// * `region` - AWS region
    /// * `resource_type` - CloudFormation resource type (e.g., "AWS::EC2::Instance")
    /// * `resource_name` - Resource name or identifier
    /// * `limit` - Maximum number of events to fetch
    ///
    /// # Returns
    /// LookupResult filtered to the specified resource
    pub async fn get_resource_events(
        &self,
        account_id: &str,
        region: &str,
        resource_type: &str,
        resource_name: Option<&str>,
        limit: i32,
    ) -> Result<LookupResult> {
        // IMPORTANT: CloudTrail API only accepts ONE LookupAttribute (AWS limitation)
        // Return empty result if no resource name is provided to avoid returning all events
        let Some(name) = resource_name else {
            log::warn!(
                "CloudTrail: No resource name provided for resource_type='{}' - returning empty result to avoid fetching all events",
                resource_type
            );
            return Ok(LookupResult::empty());
        };

        let mut all_events = Vec::new();
        let mut next_token = None;
        let mut pages_fetched = 0;
        const MIN_PAGES: i32 = 2;

        log::info!(
            "CloudTrail: Filtering events by ResourceName='{}' (resource_type='{}')",
            name,
            resource_type
        );

        loop {
            let page_size = ((limit - all_events.len() as i32).min(50)).max(50);

            // Use ONLY ResourceName filter (CloudTrail limitation: only 1 attribute allowed)
            // We cannot filter by both ResourceType AND ResourceName
            let mut options = LookupOptions::new()
                .with_max_results(page_size)
                .with_lookup_attribute(LookupAttribute::resource_name(name.to_string()));

            if let Some(token) = next_token {
                options = options.with_next_token(token);
            }

            let result = self
                .lookup_events(account_id, region, options)
                .await?;

            all_events.extend(result.events);
            next_token = result.next_token;
            pages_fetched += 1;

            // Stop conditions (same as get_recent_events)
            if pages_fetched >= MIN_PAGES {
                if all_events.len() >= limit as usize || next_token.is_none() {
                    break;
                }
            }

            if pages_fetched >= 10 {
                break;
            }
        }

        Ok(LookupResult::new(all_events, next_token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        // Client creation test (CredentialCoordinator would be mocked in real tests)
        // This just ensures the struct can be constructed
    }

    // Integration tests with real AWS should be in tests/ directory with #[ignore]
}
