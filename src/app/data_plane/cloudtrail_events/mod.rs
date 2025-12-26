//! CloudTrail Events Integration Module
//!
//! Provides functionality for querying and displaying AWS CloudTrail events
//! within the application.
//!
//! ## Features
//!
//! - **Multi-Account/Multi-Region**: Query CloudTrail across all configured AWS accounts and regions
//! - **Flexible Filtering**: Filter by resource type, event name, username, time ranges
//! - **Automatic Pagination**: Fetches at least 2 pages (100 events) automatically
//! - **Agent Integration**: Available to agents via JavaScript V8 bindings (`getCloudTrailEvents()`)
//! - **UI Integration**: Visual viewer window for exploring events
//! - **Explorer Integration**: "View Events" buttons in Resource Explorer for all resources
//!
//! ## Architecture
//!
//! ```text
//! Agent (JavaScript) → V8 Binding → CloudTrailEventsClient → AWS SDK → AWS CloudTrail
//!                                          ↓
//! UI Window ← Resource Explorer ← Event Results
//! ```
//!
//! ## What is CloudTrail?
//!
//! AWS CloudTrail records API calls made in your AWS account, providing:
//! - **Governance**: Track who did what, when, and from where
//! - **Compliance**: Audit trail for regulatory requirements
//! - **Security**: Detect unusual activity or unauthorized access
//! - **Troubleshooting**: Understand the sequence of events leading to issues
//!
//! ## Usage Example
//!
//! ```rust
//! use crate::app::data_plane::cloudtrail_events::{CloudTrailEventsClient, LookupOptions};
//! use std::sync::Arc;
//!
//! // Create client with credential coordinator
//! let client = CloudTrailEventsClient::new(credential_coordinator);
//!
//! // Build lookup options
//! let options = LookupOptions::new()
//!     .with_start_time(start_ms)
//!     .with_resource_type("AWS::EC2::Instance".to_string())
//!     .with_max_results(50);
//!
//! // Lookup CloudTrail events
//! let result = client.lookup_events(
//!     "123456789012",  // account_id
//!     "us-east-1",     // region
//!     options
//! ).await?;
//!
//! // Process results
//! for event in result.events {
//!     println!("{}: {} by {}", event.event_time, event.event_name, event.username);
//! }
//! ```
//!
//! ## Agent JavaScript API
//!
//! The agent can query CloudTrail using JavaScript:
//!
//! ```javascript
//! const events = getCloudTrailEvents({
//!   accountId: "123456789012",
//!   region: "us-east-1",
//!   startTime: Date.now() - (24 * 60 * 60 * 1000), // Last 24 hours
//!   lookupAttributes: [
//!     { attributeKey: "ResourceType", attributeValue: "AWS::EC2::Instance" }
//!   ],
//!   maxResults: 100
//! });
//!
//! events.events.forEach(event => {
//!   console.log(event.eventName + " by " + event.username);
//! });
//! ```
//!
//! ## Common Use Cases
//!
//! ### 1. Track Resource Changes
//! ```javascript
//! // Find who created/modified/deleted an EC2 instance
//! const events = getCloudTrailEvents({
//!   accountId: "123456789012",
//!   region: "us-east-1",
//!   lookupAttributes: [
//!     { attributeKey: "ResourceName", attributeValue: "i-1234567890abcdef0" }
//!   ]
//! });
//! ```
//!
//! ### 2. Security Audit
//! ```javascript
//! // Find all failed API calls (potential unauthorized access)
//! const events = getCloudTrailEvents({
//!   accountId: "123456789012",
//!   region: "us-east-1",
//!   startTime: Date.now() - (7 * 24 * 60 * 60 * 1000) // Last 7 days
//! });
//!
//! const failures = events.events.filter(e => e.errorCode);
//! ```
//!
//! ### 3. Compliance Reporting
//! ```javascript
//! // Track IAM changes for compliance
//! const events = getCloudTrailEvents({
//!   accountId: "123456789012",
//!   region: "us-east-1",
//!   lookupAttributes: [
//!     { attributeKey: "ResourceType", attributeValue: "AWS::IAM::Role" }
//!   ]
//! });
//! ```
//!
//! ## Supported Resource Types
//!
//! CloudTrail supports **ALL AWS resource types** because it logs all API calls.
//! Common resource types include:
//!
//! - `AWS::EC2::Instance` - EC2 instances
//! - `AWS::S3::Bucket` - S3 buckets
//! - `AWS::Lambda::Function` - Lambda functions
//! - `AWS::RDS::DBInstance` - RDS databases
//! - `AWS::DynamoDB::Table` - DynamoDB tables
//! - `AWS::IAM::Role` - IAM roles
//! - `AWS::IAM::User` - IAM users
//! - And hundreds more...
//!
//! ## Lookup Attributes
//!
//! You can filter events using these lookup attributes:
//!
//! - **EventId**: Unique event identifier
//! - **EventName**: API operation (e.g., "RunInstances", "CreateBucket")
//! - **ResourceType**: CloudFormation resource type
//! - **ResourceName**: Resource identifier
//! - **Username**: IAM user or role that made the call
//! - **EventSource**: AWS service (e.g., "ec2.amazonaws.com")
//! - **AccessKeyId**: Access key used for the call
//! - **ReadOnly**: "true" for read-only calls, "false" for write operations
//!
//! ## Pagination
//!
//! CloudTrail limits results to 50 events per API call. This module automatically:
//! - Fetches at least 2 pages (100 events minimum) in `get_recent_events()`
//! - Provides pagination tokens for fetching more events
//! - Limits to 500 events maximum (10 pages) for safety
//!
//! ## Performance Considerations
//!
//! - **Time Ranges**: Narrow time ranges are faster than broad queries
//! - **Filtering**: Use lookup attributes to reduce results
//! - **Pagination**: Be aware that large queries may require multiple API calls
//! - **CloudTrail Delay**: Events may take 5-15 minutes to appear in CloudTrail

#![warn(clippy::all, rust_2018_idioms)]

pub mod client;
pub mod resource_mapping;
pub mod types;

// Re-export commonly used types
pub use client::CloudTrailEventsClient;
pub use resource_mapping::{
    get_cloudtrail_lookup_value, get_common_event_names, has_cloudtrail_support,
};
pub use types::{
    CloudTrailEvent, EventResource, LookupAttribute, LookupAttributeKey, LookupOptions,
    LookupResult,
};
