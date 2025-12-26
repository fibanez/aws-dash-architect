//! Data Plane Services Module
//!
//! This module contains AWS data plane service integrations.
//! Data plane services query, analyze, or manipulate data within AWS resources,
//! as opposed to control plane operations (resource discovery and management).
//!
//! ## Available Services
//!
//! - **CloudWatch Logs**: Query log events from Lambda, API Gateway, RDS, and other resources
//! - **CloudTrail Events**: Query API call history and governance/compliance events
//!
//! ## Future Services
//!
//! - CloudWatch Metrics: Query metrics and statistics
//! - AWS Config: Query resource configuration history
//! - Athena: Run SQL queries against data in S3
//! - X-Ray: Trace distributed application requests
//!
//! ## Architecture
//!
//! Each data plane service follows a consistent 5-layer architecture:
//!
//! ```text
//! 1. AWS SDK Client Layer (src/app/data_plane/{service}/)
//!    └─ Rust client with credential management
//!
//! 2. V8 JavaScript Binding Layer
//!    └─ Expose to agents
//!
//! 3. Agent Tool Integration
//!    └─ LLM documentation
//!
//! 4. UI Viewer Window
//!    └─ Visual interface for data exploration
//!
//! 5. Resource Explorer Integration
//!    └─ Convenient "View X" buttons
//! ```
//!
//! ## Usage
//!
//! Data plane services can be used in two ways:
//!
//! 1. **Agent Access** - Via JavaScript V8 bindings:
//!    ```javascript
//!    const logs = queryCloudWatchLogs({
//!      resourceId: "/aws/lambda/my-function",
//!      accountId: "123456789012",
//!      region: "us-east-1",
//!      limit: 100
//!    });
//!    ```
//!
//! 2. **UI Access** - Via Resource Explorer buttons:
//!    - Click "View Logs" or "View Events" on any resource
//!    - Opens dedicated viewer window
//!    - Supports search, filtering, and pagination
//!
//! ## Adding New Services
//!
//! See `docs/technical/aws-data-plane-integration-guide.md` for the complete
//! integration pattern and step-by-step guide.

pub mod cloudtrail_events;
pub mod cloudwatch_logs;

// Re-export commonly used types from each service
pub use cloudwatch_logs::{
    CloudWatchLogsClient, LogQueryResult as CloudWatchLogsQueryResult,
    QueryOptions as CloudWatchLogsQueryOptions,
};

pub use cloudtrail_events::{
    CloudTrailEventsClient, LookupOptions as CloudTrailLookupOptions,
    LookupResult as CloudTrailLookupResult,
};
