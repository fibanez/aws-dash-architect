//! CloudWatch Logs Integration Module
//!
//! Provides functionality for querying and displaying AWS CloudWatch Logs within the application.
//!
//! ## Features
//!
//! - Query log events with flexible filtering options
//! - Time range and pattern-based filtering
//! - Pagination support for large result sets
//! - Integration with Resource Explorer and Agent V2
//!
//! ## Usage
//!
//! ```rust,no_run
//! use awsdash::app::cloudwatch_logs::{CloudWatchLogsClient, QueryOptions};
//! # use std::sync::Arc;
//! # use awsdash::app::resource_explorer::credentials::CredentialCoordinator;
//!
//! # async fn example(credential_coordinator: Arc<CredentialCoordinator>) -> anyhow::Result<()> {
//! let client = CloudWatchLogsClient::new(credential_coordinator);
//!
//! // Get latest 100 log events
//! let result = client.get_latest_log_events(
//!     "123456789012",           // account_id
//!     "us-east-1",              // region
//!     "/aws/lambda/my-function", // log_group_name
//!     100,                       // limit
//! ).await?;
//!
//! for event in result.events {
//!     println!("{}: {}", event.timestamp, event.message);
//! }
//! # Ok(())
//! # }
//! ```

#![warn(clippy::all, rust_2018_idioms)]

pub mod client;
pub mod resource_mapping;
pub mod types;

// Re-export commonly used types
pub use client::CloudWatchLogsClient;
pub use resource_mapping::{get_log_group_name, has_cloudwatch_logs, get_all_log_group_patterns};
pub use types::{LogEvent, LogQueryResult, QueryOptions, QueryStatistics};
