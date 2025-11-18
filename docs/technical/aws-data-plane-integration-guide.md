# AWS Data Plane Service Integration Guide

**Purpose**: This guide documents the complete pattern for integrating AWS data plane services (CloudWatch Logs, CloudWatch Metrics, CloudTrail, Athena, etc.) into the application.

**Scope**: Data plane services are operations that query, analyze, or manipulate data within AWS resources, as opposed to control plane operations (resource discovery and management).

**Reference Implementation**: CloudWatch Logs (commits: 770adbe, 3819148, 972ed97)

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [**CRITICAL: Async Runtime Handling in egui Windows**](#critical-async-runtime-handling-in-egui-windows) ⚠️
4. [Layer 1: AWS SDK Client](#layer-1-aws-sdk-client)
5. [Layer 2: V8 JavaScript Binding](#layer-2-v8-javascript-binding)
6. [Layer 3: Agent Tool Integration](#layer-3-agent-tool-integration)
7. [Layer 4: UI Viewer Window](#layer-4-ui-viewer-window)
8. [Layer 5: Explorer Integration](#layer-5-explorer-integration)
9. [Step-by-Step Checklist](#step-by-step-checklist)
10. [Naming Conventions](#naming-conventions)
11. [Common Patterns](#common-patterns)
12. [Testing Strategy](#testing-strategy)
13. [Service-Specific Examples](#service-specific-examples)

---

## Overview

### What is Data Plane Integration?

Data plane services operate on the **data** within AWS resources:
- **CloudWatch Logs**: Query log events from Lambda, API Gateway, RDS, etc.
- **CloudWatch Metrics**: Query metrics and statistics
- **CloudTrail**: Query API call history and events
- **Athena**: Run SQL queries against data in S3

This is distinct from **control plane** operations (resource discovery in the Resource Explorer).

### Integration Goals

1. **Agent Access**: Enable agents to query AWS data via JavaScript
2. **UI Access**: Provide visual interface for users to explore data
3. **Explorer Integration**: Add convenient "View X" buttons for relevant resources
4. **Multi-Account/Multi-Region**: Support querying across all configured accounts and regions

### Time Estimate

- **Minimal (Agent-only)**: ~2-3 hours (Client + V8 binding)
- **Complete (with UI)**: ~4-6 hours (All layers)
- **With Explorer**: Add ~1-2 hours (Resource mapping + buttons)

---

## Architecture

### 5-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    DATA PLANE INTEGRATION                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Layer 1: AWS SDK Client (REQUIRED)                             │
│  ├─ Location: src/app/{service}/                                │
│  ├─ Files: client.rs, types.rs, mod.rs, resource_mapping.rs    │
│  └─ Purpose: Wrap AWS SDK with credential management           │
│                                                                  │
│  Layer 2: V8 JavaScript Binding (REQUIRED)                      │
│  ├─ Location: src/app/agent_framework/v8_bindings/bindings/    │
│  ├─ File: {service}.rs                                          │
│  └─ Purpose: Expose Rust client to JavaScript for agent        │
│                                                                  │
│  Layer 3: Agent Tool Integration (REQUIRED)                     │
│  ├─ Location: src/app/agent_framework/tools/                    │
│  ├─ File: execute_javascript.rs                                 │
│  └─ Purpose: Document function for LLM discovery               │
│                                                                  │
│  Layer 4: UI Viewer Window (REQUIRED)                           │
│  ├─ Location: src/app/dashui/                                   │
│  ├─ File: {service}_window.rs                                   │
│  └─ Purpose: Visual interface for exploring data               │
│                                                                  │
│  Layer 5: Explorer Integration (REQUIRED)                       │
│  ├─ Location: src/app/resource_explorer/                        │
│  ├─ Files: tree.rs, mod.rs                                      │
│  └─ Purpose: "View X" buttons in Resource Explorer             │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User/Agent → V8 JavaScript → Rust Client → AWS SDK → AWS Service
                                                         ↓
User ← UI Window ← Resource Explorer Button ← Query Results
```

### Key Principles

1. **Separation of Concerns**: Each layer has a single responsibility
2. **Reusability**: Client layer can be used by both agent and UI
3. **Type Safety**: Strong typing at Rust layer, proper conversion to JavaScript
4. **Multi-Account**: All operations use CredentialCoordinator for account/region selection
5. **Async Everywhere**: All AWS operations are async with proper runtime handling

---

## CRITICAL: Async Runtime Handling in egui Windows

**⚠️ IMPORTANT**: egui runs on a **blocking thread** and does **NOT** have a tokio runtime available.

### The Problem

If you try to use `tokio::spawn()` directly from an egui window's refresh method, you'll get:

```
thread 'main' panicked at src/app/dashui/{service}_window.rs:XXX:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

### The Solution

You **MUST** create a new thread with its own tokio runtime:

```rust
// ❌ WRONG - This will panic!
tokio::spawn(async move {
    let result = client.query_data(&account_id, &region).await;
    let _ = sender.send(result);
});

// ✅ CORRECT - Create new thread with tokio runtime
std::thread::spawn(move || {
    // Create a new tokio runtime for this thread
    let runtime = tokio::runtime::Runtime::new()
        .expect("Failed to create tokio runtime");

    // Run the async operation
    runtime.block_on(async move {
        let result = match client.query_data(&account_id, &region).await {
            Ok(data) => {
                log::info!("Successfully loaded data");
                Ok(data)
            }
            Err(e) => {
                log::error!("Failed to load data: {}", e);
                Err(e.to_string())
            }
        };

        let _ = sender.send(result);
    });
});
```

### Reference Implementation

See `src/app/dashui/cloudwatch_logs_window.rs:104` for the complete pattern.

### Logging

- **Application logs**: `$HOME/.local/share/awsdash/logs/awsdash.log`
- Use `log::info!()` and `log::error!()` for operational logging
- All logs from both logging and tracing go to the same file

---

## Layer 1: AWS SDK Client

### Purpose

Create a Rust client that wraps the AWS SDK and provides:
- Type-safe query operations
- Credential management via CredentialCoordinator
- Error handling with context
- Convenient builder patterns

### File Structure

```
src/app/{service}/
├── mod.rs                    # Module definition and public API
├── types.rs                  # Data structures with Serde support
├── client.rs                 # AWS SDK wrapper
└── resource_mapping.rs       # Map AWS resources to service identifiers
```

### 1.1 Create Module Directory

```bash
mkdir -p src/app/{service}
```

Example: `mkdir -p src/app/cloudwatch_logs`

---

### 1.2 Implement `types.rs`

**Purpose**: Define data structures for queries and results with Serde support for JSON serialization.

**Template**:

```rust
//! Data types for {Service} operations

use serde::{Deserialize, Serialize};

/// Options for querying {Service}
#[derive(Debug, Clone)]
pub struct QueryOptions {
    // Time range
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,

    // Filtering
    pub filter_pattern: Option<String>,

    // Pagination
    pub limit: Option<i32>,
    pub next_token: Option<String>,

    // Service-specific options
    // Add as needed...
}

impl QueryOptions {
    /// Create new query options with sensible defaults
    pub fn new() -> Self {
        Self {
            start_time: None,
            end_time: None,
            filter_pattern: None,
            limit: Some(100),
            next_token: None,
        }
    }

    /// Builder pattern: set start time (Unix milliseconds)
    pub fn with_start_time(mut self, start_time: i64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    /// Builder pattern: set end time (Unix milliseconds)
    pub fn with_end_time(mut self, end_time: i64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    /// Builder pattern: set filter pattern
    pub fn with_filter_pattern(mut self, pattern: String) -> Self {
        self.filter_pattern = Some(pattern);
        self
    }

    /// Builder pattern: set result limit
    pub fn with_limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Builder pattern: set pagination token
    pub fn with_next_token(mut self, token: String) -> Self {
        self.next_token = Some(token);
        self
    }
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a {Service} query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Items returned from the query
    pub items: Vec<ResultItem>,

    /// Token for fetching next page of results
    pub next_token: Option<String>,

    /// Total number of items in this result
    pub total_items: usize,

    /// Query performance statistics
    pub query_statistics: QueryStatistics,
}

impl QueryResult {
    /// Create empty result
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            next_token: None,
            total_items: 0,
            query_statistics: QueryStatistics::default(),
        }
    }

    /// Create new result
    pub fn new(
        items: Vec<ResultItem>,
        next_token: Option<String>,
    ) -> Self {
        let total_items = items.len();
        Self {
            items,
            next_token,
            total_items,
            query_statistics: QueryStatistics::default(),
        }
    }
}

/// Individual result item from {Service}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultItem {
    /// Timestamp in Unix milliseconds
    pub timestamp: i64,

    /// Item data (service-specific format)
    pub data: String,

    // Add service-specific fields as needed
}

/// Query performance statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryStatistics {
    /// Number of records matched
    pub records_matched: Option<i64>,

    /// Number of records scanned
    pub records_scanned: Option<i64>,

    /// Bytes scanned
    pub bytes_scanned: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_options_builder() {
        let options = QueryOptions::new()
            .with_start_time(1000)
            .with_end_time(2000)
            .with_limit(50);

        assert_eq!(options.start_time, Some(1000));
        assert_eq!(options.end_time, Some(2000));
        assert_eq!(options.limit, Some(50));
    }

    #[test]
    fn test_query_result_serialization() {
        let result = QueryResult::new(
            vec![ResultItem {
                timestamp: 1234567890,
                data: "test data".to_string(),
            }],
            Some("next-token".to_string()),
        );

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: QueryResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.items.len(), 1);
        assert_eq!(deserialized.next_token, Some("next-token".to_string()));
    }
}
```

**Reference**: `src/app/cloudwatch_logs/types.rs` (lines 1-273)

**Critical Points**:
- ✅ Always derive `Serialize` and `Deserialize` for types exposed to JavaScript
- ✅ Use builder pattern for QueryOptions (ergonomic API)
- ✅ Provide sensible defaults
- ✅ Use `Option<T>` for optional fields
- ✅ Include unit tests for serialization

---

### 1.3 Implement `client.rs`

**Purpose**: Wrap AWS SDK client with credential management and multi-account support.

**Template**:

```rust
//! AWS SDK client wrapper for {Service}

use anyhow::{Context, Result};
use aws_sdk_{service} as {service}_sdk;
use std::sync::Arc;

use crate::app::resource_explorer::credentials::CredentialCoordinator;
use super::types::{QueryOptions, QueryResult, ResultItem, QueryStatistics};

/// Client for querying AWS {Service}
#[derive(Clone)]
pub struct {Service}Client {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl {Service}Client {
    /// Create new client with credential coordinator
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }

    /// Query {Service} with full options
    ///
    /// # Arguments
    /// * `account_id` - AWS account ID
    /// * `region` - AWS region (e.g., "us-east-1")
    /// * `resource_identifier` - Service-specific resource identifier
    /// * `options` - Query options (time range, filters, pagination)
    ///
    /// # Returns
    /// QueryResult with items and pagination token
    ///
    /// # Example
    /// ```rust
    /// let options = QueryOptions::new()
    ///     .with_start_time(start_ms)
    ///     .with_limit(100);
    ///
    /// let result = client.query_items(
    ///     "123456789012",
    ///     "us-east-1",
    ///     "my-resource-id",
    ///     options
    /// ).await?;
    /// ```
    pub async fn query_items(
        &self,
        account_id: &str,
        region: &str,
        resource_identifier: &str,
        options: QueryOptions,
    ) -> Result<QueryResult> {
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
        let client = {service}_sdk::Client::new(&aws_config);

        // Step 3: Build request with options
        let mut request = client
            .{sdk_operation}()  // Replace with actual SDK operation
            .{resource_parameter}(resource_identifier);  // Replace with actual parameter

        // Apply time range
        if let Some(start_time) = options.start_time {
            request = request.start_time(start_time);
        }

        if let Some(end_time) = options.end_time {
            request = request.end_time(end_time);
        }

        // Apply filter
        if let Some(filter) = options.filter_pattern {
            request = request.filter_pattern(filter);
        }

        // Apply limit
        if let Some(limit) = options.limit {
            request = request.limit(limit);
        }

        // Apply pagination
        if let Some(token) = options.next_token {
            request = request.next_token(token);
        }

        // Step 4: Execute request
        let response = request
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to query {Service} for resource: {}",
                    resource_identifier
                )
            })?;

        // Step 5: Convert AWS SDK response to our types
        let items: Vec<ResultItem> = response
            .items()  // Replace with actual response field
            .iter()
            .map(|item| ResultItem {
                timestamp: item.timestamp().unwrap_or(0),
                data: item.message().unwrap_or_default().to_string(),
            })
            .collect();

        let statistics = QueryStatistics {
            records_matched: response.statistics().and_then(|s| s.records_matched()),
            records_scanned: response.statistics().and_then(|s| s.records_scanned()),
            bytes_scanned: response.statistics().and_then(|s| s.bytes_scanned()),
        };

        let mut result = QueryResult::new(
            items,
            response.next_token().map(|t| t.to_string()),
        );
        result.query_statistics = statistics;

        Ok(result)
    }

    /// Convenience method: Get recent items with simple parameters
    ///
    /// # Arguments
    /// * `account_id` - AWS account ID
    /// * `region` - AWS region
    /// * `resource_identifier` - Resource identifier
    /// * `limit` - Maximum number of items to return
    ///
    /// # Example
    /// ```rust
    /// let result = client.get_recent_items(
    ///     "123456789012",
    ///     "us-east-1",
    ///     "my-resource",
    ///     100
    /// ).await?;
    /// ```
    pub async fn get_recent_items(
        &self,
        account_id: &str,
        region: &str,
        resource_identifier: &str,
        limit: i32,
    ) -> Result<QueryResult> {
        let options = QueryOptions::new()
            .with_limit(limit);

        self.query_items(account_id, region, resource_identifier, options)
            .await
    }

    /// List available resources (if applicable to service)
    ///
    /// Example: List CloudWatch log groups, list Athena databases, etc.
    pub async fn list_resources(
        &self,
        account_id: &str,
        region: &str,
        prefix: Option<String>,
    ) -> Result<Vec<String>> {
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

        let client = {service}_sdk::Client::new(&aws_config);

        let mut request = client.{list_operation}();

        if let Some(prefix) = prefix {
            request = request.prefix(prefix);
        }

        let response = request
            .send()
            .await
            .with_context(|| "Failed to list resources")?;

        let resources = response
            .items()  // Replace with actual response field
            .iter()
            .map(|item| item.name().unwrap_or_default().to_string())
            .collect();

        Ok(resources)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests with real AWS should be in tests/ directory
    // Unit tests here should test logic, not AWS API calls

    #[test]
    fn test_client_creation() {
        // Test that client can be created
        // (CredentialCoordinator creation would be mocked in real tests)
    }
}
```

**Reference**: `src/app/cloudwatch_logs/client.rs` (lines 1-224)

**Critical Points**:
- ✅ Use `CredentialCoordinator` for ALL AWS API calls (multi-account support)
- ✅ Always use `.with_context()` for error handling (better error messages)
- ✅ Convert AWS SDK types to your own types (don't expose SDK types in public API)
- ✅ All operations are `async`
- ✅ Provide convenience methods for common use cases

---

### 1.4 Implement `resource_mapping.rs`

**Purpose**: Map AWS resource types to service-specific identifiers for Explorer integration.

**Template**:

```rust
//! Resource type mapping for {Service}
//!
//! Maps AWS CloudFormation resource types to {Service} identifiers.

/// Check if a resource type is supported by {Service}
pub fn has_{service}_support(resource_type: &str) -> bool {
    matches!(
        resource_type,
        "AWS::Lambda::Function"
            | "AWS::ApiGateway::RestApi"
            | "AWS::ECS::Service"
            | "AWS::RDS::DBInstance"
            // Add all supported resource types
    )
}

/// Get {Service} identifier for a resource
///
/// Returns the service-specific identifier (e.g., log group name, metric namespace)
/// based on the resource type and name.
///
/// # Arguments
/// * `resource_type` - CloudFormation resource type
/// * `resource_name` - Resource name/physical ID
/// * `resource_arn` - Resource ARN (if available)
///
/// # Returns
/// Service identifier if resource type is supported
pub fn get_{service}_identifier(
    resource_type: &str,
    resource_name: &str,
    _resource_arn: Option<&str>,
) -> Option<String> {
    match resource_type {
        "AWS::Lambda::Function" => {
            // Lambda logs go to /aws/lambda/{function-name}
            Some(format!("/aws/lambda/{}", resource_name))
        }
        "AWS::ApiGateway::RestApi" => {
            // API Gateway logs go to /aws/apigateway/{api-id}
            Some(format!("/aws/apigateway/{}", resource_name))
        }
        "AWS::ECS::Service" => {
            // ECS logs are service-specific (would need cluster name too)
            Some(format!("/aws/ecs/{}", resource_name))
        }
        "AWS::RDS::DBInstance" => {
            // RDS has multiple log streams - return default
            Some(format!("/aws/rds/instance/{}/error", resource_name))
        }
        // Add more resource type mappings
        _ => None,
    }
}

/// Get all possible {Service} identifiers for a resource
///
/// Some resources (like RDS) have multiple log streams.
/// This function returns all possible identifiers.
pub fn get_all_{service}_identifiers(
    resource_type: &str,
    resource_name: &str,
) -> Vec<String> {
    match resource_type {
        "AWS::RDS::DBInstance" => {
            // RDS has multiple log types
            vec![
                format!("/aws/rds/instance/{}/error", resource_name),
                format!("/aws/rds/instance/{}/slowquery", resource_name),
                format!("/aws/rds/instance/{}/general", resource_name),
                format!("/aws/rds/instance/{}/audit", resource_name),
            ]
        }
        _ => {
            // Single identifier resources
            if let Some(id) = get_{service}_identifier(resource_type, resource_name, None) {
                vec![id]
            } else {
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lambda_support() {
        assert!(has_{service}_support("AWS::Lambda::Function"));

        let id = get_{service}_identifier(
            "AWS::Lambda::Function",
            "my-function",
            None,
        );
        assert_eq!(id, Some("/aws/lambda/my-function".to_string()));
    }

    #[test]
    fn test_unsupported_resource() {
        assert!(!has_{service}_support("AWS::S3::Bucket"));

        let id = get_{service}_identifier(
            "AWS::S3::Bucket",
            "my-bucket",
            None,
        );
        assert_eq!(id, None);
    }

    #[test]
    fn test_rds_multiple_identifiers() {
        let ids = get_all_{service}_identifiers(
            "AWS::RDS::DBInstance",
            "my-db",
        );

        assert!(ids.len() > 1);
        assert!(ids.contains(&"/aws/rds/instance/my-db/error".to_string()));
        assert!(ids.contains(&"/aws/rds/instance/my-db/slowquery".to_string()));
    }
}
```

**Reference**: `src/app/cloudwatch_logs/resource_mapping.rs` (lines 1-292)

**Critical Points**:
- ✅ Use `matches!` macro for cleaner support checks
- ✅ Handle resources with multiple identifiers (e.g., RDS log streams)
- ✅ Return `Option<String>` for single identifier lookup
- ✅ Return `Vec<String>` for multi-identifier lookup
- ✅ Add comprehensive unit tests

---

### 1.5 Implement `mod.rs`

**Purpose**: Define module public API and documentation.

**Template**:

```rust
//! {Service} Integration Module
//!
//! Provides functionality for querying and displaying AWS {Service} data
//! within the application.
//!
//! ## Features
//!
//! - **Multi-Account/Multi-Region**: Query {Service} across all configured AWS accounts and regions
//! - **Flexible Filtering**: Time ranges, filter patterns, and pagination support
//! - **Agent Integration**: Available to agents via JavaScript V8 bindings
//! - **UI Integration**: Visual viewer window for exploring data
//! - **Explorer Integration**: "View {Service}" buttons in Resource Explorer for supported resources
//!
//! ## Architecture
//!
//! ```text
//! Agent (JavaScript) → V8 Binding → {Service}Client → AWS SDK → AWS {Service}
//!                                        ↓
//! UI Window ← Resource Explorer ← Query Results
//! ```
//!
//! ## Usage Example
//!
//! ```rust
//! use crate::app::{service}::{Service}Client, QueryOptions};
//! use std::sync::Arc;
//!
//! // Create client with credential coordinator
//! let client = {Service}Client::new(credential_coordinator);
//!
//! // Build query options
//! let options = QueryOptions::new()
//!     .with_start_time(start_ms)
//!     .with_end_time(end_ms)
//!     .with_limit(100);
//!
//! // Query {Service}
//! let result = client.query_items(
//!     "123456789012",  // account_id
//!     "us-east-1",     // region
//!     "my-resource",   // resource_identifier
//!     options
//! ).await?;
//!
//! // Process results
//! for item in result.items {
//!     println!("{}: {}", item.timestamp, item.data);
//! }
//! ```
//!
//! ## Supported Resource Types
//!
//! The following CloudFormation resource types are supported:
//! - `AWS::Lambda::Function` - Lambda function logs/metrics
//! - `AWS::ApiGateway::RestApi` - API Gateway logs/metrics
//! - `AWS::ECS::Service` - ECS service logs/metrics
//! - `AWS::RDS::DBInstance` - RDS database logs/metrics
//! // List all supported types
//!
//! ## Agent JavaScript API
//!
//! The agent can query {Service} using JavaScript:
//!
//! ```javascript
//! const result = query{Service}({
//!   resourceId: "my-resource",
//!   accountId: "123456789012",
//!   region: "us-east-1",
//!   startTime: Date.now() - (60 * 60 * 1000), // Last hour
//!   limit: 100
//! });
//!
//! result.items.forEach(item => {
//!   console.log(item.timestamp + ": " + item.data);
//! });
//! ```

pub mod client;
pub mod resource_mapping;
pub mod types;

// Re-export commonly used types
pub use client::{Service}Client;
pub use resource_mapping::{get_{service}_identifier, get_all_{service}_identifiers, has_{service}_support};
pub use types::{QueryOptions, QueryResult, QueryStatistics, ResultItem};
```

**Reference**: `src/app/cloudwatch_logs/mod.rs` (lines 1-47)

**Critical Points**:
- ✅ Comprehensive module documentation with examples
- ✅ Document architecture and data flow
- ✅ List supported resource types
- ✅ Show both Rust and JavaScript usage examples
- ✅ Re-export commonly used types

---

### 1.6 Add to Cargo.toml

**File**: `Cargo.toml`

```toml
[dependencies]
# Existing dependencies...

# AWS SDK for {Service}
aws-sdk-{service} = "1.x"  # Use latest version
```

**Example**:
```toml
aws-sdk-cloudwatchlogs = "1.52.0"
aws-sdk-cloudwatch = "1.52.0"
aws-sdk-cloudtrail = "1.52.0"
aws-sdk-athena = "1.52.0"
```

---

### 1.7 Export Module

**File**: `src/app/mod.rs`

Add your module to the exports:

```rust
pub mod cloudformation;
pub mod cloudwatch_logs;  // ADD THIS LINE
pub mod identity_center;
// ... other modules
```

---

## Layer 2: V8 JavaScript Binding

### Purpose

Expose the Rust client to JavaScript so agents can query the service.

### File Location

```
src/app/agent_framework/v8_bindings/bindings/{service}.rs
```

Example: `src/app/agent_framework/v8_bindings/bindings/cloudwatch_logs.rs`

---

### 2.1 Create V8 Binding File

**Template**:

```rust
//! V8 JavaScript bindings for {Service}
//!
//! Exposes {Service} querying functionality to the agent's JavaScript environment.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::app::{service}::{Service}Client, QueryOptions};

/// JavaScript function arguments for query{Service}()
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // CRITICAL: JavaScript uses camelCase
pub struct Query{Service}Args {
    /// Resource identifier (required)
    /// Example: "/aws/lambda/my-function" for CloudWatch Logs
    pub resource_id: String,

    /// AWS account ID (required)
    pub account_id: String,

    /// AWS region (required)
    /// Example: "us-east-1"
    pub region: String,

    /// Start time in Unix milliseconds (optional)
    /// Example: Date.now() - (60 * 60 * 1000) for 1 hour ago
    pub start_time: Option<i64>,

    /// End time in Unix milliseconds (optional)
    pub end_time: Option<i64>,

    /// Filter pattern (optional, service-specific)
    pub filter_pattern: Option<String>,

    /// Maximum number of items to return (optional, default 100, max 10000)
    pub limit: Option<i32>,
}

/// Result item exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultItemInfo {
    /// Timestamp in Unix milliseconds
    pub timestamp: i64,

    /// Item data
    pub data: String,
}

/// Query result exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct {Service}QueryResult {
    /// Array of result items
    pub items: Vec<ResultItemInfo>,

    /// Pagination token for next page (null if no more results)
    pub next_token: Option<String>,

    /// Total number of items in this result
    pub total_items: usize,

    /// Query performance statistics
    pub statistics: QueryStatisticsInfo,
}

/// Query statistics exposed to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryStatisticsInfo {
    pub records_matched: Option<i64>,
    pub records_scanned: Option<i64>,
    pub bytes_scanned: Option<i64>,
}

/// Register {Service} functions into V8 JavaScript context
///
/// This is called during agent initialization to make functions available.
pub fn register(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Register query{Service}() function
    let query_fn = v8::Function::new(scope, query_{service}_callback)
        .expect("Failed to create query{Service} function");

    let fn_name = v8::String::new(scope, "query{Service}")
        .expect("Failed to create function name string");

    global.set(scope, fn_name.into(), query_fn.into());

    Ok(())
}

/// V8 callback for query{Service}() JavaScript function
///
/// This function is called when JavaScript code executes query{Service}({...}).
fn query_{service}_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    mut rv: v8::ReturnValue<'_>,
) {
    // Step 1: Parse JavaScript arguments
    let args_obj = match args.get(0).to_object(scope) {
        Some(obj) => obj,
        None => {
            let msg = v8::String::new(
                scope,
                "query{Service}() requires an object argument with { resourceId, accountId, region, ... }",
            )
            .unwrap();
            let error = v8::Exception::type_error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 2: Convert V8 object to JSON string
    let json_str = match v8::json::stringify(scope, args_obj.into()) {
        Some(s) => s.to_rust_string_lossy(scope),
        None => {
            let msg = v8::String::new(scope, "Failed to stringify arguments").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 3: Parse JSON into typed Args struct
    let query_args: Query{Service}Args = match serde_json::from_str(&json_str) {
        Ok(args) => args,
        Err(e) => {
            let msg = v8::String::new(
                scope,
                &format!("Failed to parse query{Service} arguments: {}. Expected {{ resourceId: string, accountId: string, region: string, ... }}", e),
            )
            .unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 4: Execute query (async operation in blocking context)
    let result = match execute_query(query_args) {
        Ok(result) => result,
        Err(e) => {
            let msg = v8::String::new(
                scope,
                &format!("{Service} query failed: {}", e),
            )
            .unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 5: Serialize result to JSON
    let result_json = match serde_json::to_string(&result) {
        Ok(json) => json,
        Err(e) => {
            let msg = v8::String::new(
                scope,
                &format!("Failed to serialize query result: {}", e),
            )
            .unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    // Step 6: Parse JSON string back to V8 value and return
    let result_value = match v8::json::parse(
        scope,
        v8::String::new(scope, &result_json).unwrap().into(),
    ) {
        Some(val) => val,
        None => {
            let msg = v8::String::new(scope, "Failed to parse result JSON").unwrap();
            let error = v8::Exception::error(scope, msg);
            scope.throw_exception(error);
            return;
        }
    };

    rv.set(result_value);
}

/// Execute query using tokio runtime
///
/// CRITICAL: Use block_in_place to avoid "Cannot start a runtime from within a runtime" error
fn execute_query(args: Query{Service}Args) -> Result<{Service}QueryResult> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            query_{service}_internal(args).await
        })
    })
}

/// Internal async implementation of {Service} query
async fn query_{service}_internal(
    args: Query{Service}Args,
) -> Result<{Service}QueryResult> {
    info!(
        "Querying {Service}: account={}, region={}, resource={}",
        args.account_id, args.region, args.resource_id
    );

    // Get global AWS client for credential coordinator access
    let aws_client = crate::app::agent_framework::tools_registry::get_global_aws_client()
        .ok_or_else(|| antml::anyhow!("AWS client not initialized"))?;

    let credential_coordinator = aws_client.get_credential_coordinator();

    // Create {Service} client
    let service_client = {Service}Client::new(credential_coordinator);

    // Build query options
    let mut options = QueryOptions::new();

    if let Some(start_time) = args.start_time {
        options = options.with_start_time(start_time);
    }

    if let Some(end_time) = args.end_time {
        options = options.with_end_time(end_time);
    }

    if let Some(filter) = args.filter_pattern {
        options = options.with_filter_pattern(filter);
    }

    // Set limit with safety cap
    if let Some(limit) = args.limit {
        let safe_limit = limit.min(10000);  // Cap at 10k items
        options = options.with_limit(safe_limit);
    } else {
        options = options.with_limit(100);  // Default to 100
    }

    // Execute query
    let result = service_client
        .query_items(
            &args.account_id,
            &args.region,
            &args.resource_id,
            options,
        )
        .await
        .map_err(|e| anyhow!("Failed to query {Service}: {}", e))?;

    // Convert to V8-friendly format
    let items: Vec<ResultItemInfo> = result
        .items
        .into_iter()
        .map(|item| ResultItemInfo {
            timestamp: item.timestamp,
            data: item.data,
        })
        .collect();

    let statistics = QueryStatisticsInfo {
        records_matched: result.query_statistics.records_matched,
        records_scanned: result.query_statistics.records_scanned,
        bytes_scanned: result.query_statistics.bytes_scanned,
    };

    Ok({Service}QueryResult {
        items,
        next_token: result.next_token,
        total_items: result.total_items,
        statistics,
    })
}

/// Get LLM documentation for {Service} functions
///
/// This documentation is included in the execute_javascript tool description
/// so the LLM knows how to use the function.
pub fn get_documentation() -> String {
    r#"### query{Service}(params)

Query AWS {Service} for data analysis and monitoring.

**Parameters** (object):
- `resourceId` (string, required): Service-specific resource identifier
  - Example for CloudWatch Logs: "/aws/lambda/my-function"
  - Example for CloudWatch Metrics: "AWS/Lambda"
- `accountId` (string, required): AWS account ID (12 digits)
- `region` (string, required): AWS region code (e.g., "us-east-1", "eu-west-1")
- `startTime` (number, optional): Start time in Unix milliseconds timestamp
  - Example: `Date.now() - (60 * 60 * 1000)` for 1 hour ago
- `endTime` (number, optional): End time in Unix milliseconds timestamp
  - Example: `Date.now()` for current time
- `filterPattern` (string, optional): Service-specific filter pattern
- `limit` (number, optional): Maximum items to return (default: 100, max: 10000)

**Returns** (object):
- `items` (array): Result items matching the query
  - `timestamp` (number): Item timestamp in Unix milliseconds
  - `data` (string): Item data (service-specific format)
- `nextToken` (string|null): Pagination token for fetching more results (null if no more)
- `totalItems` (number): Number of items in this response
- `statistics` (object): Query performance statistics
  - `recordsMatched` (number|null): Number of records matched
  - `recordsScanned` (number|null): Number of records scanned
  - `bytesScanned` (number|null): Bytes scanned during query

**Example 1 - Query recent data:**
```javascript
const result = query{Service}({
  resourceId: "my-resource-id",
  accountId: "123456789012",
  region: "us-east-1",
  startTime: Date.now() - (60 * 60 * 1000), // Last hour
  limit: 100
});

console.log("Found " + result.items.length + " items");
result.items.forEach(item => {
  const date = new Date(item.timestamp).toISOString();
  console.log(date + ": " + item.data);
});
```

**Example 2 - Query with filter:**
```javascript
const result = query{Service}({
  resourceId: "/aws/lambda/my-function",
  accountId: "123456789012",
  region: "us-east-1",
  startTime: Date.now() - (24 * 60 * 60 * 1000), // Last 24 hours
  filterPattern: "ERROR",  // Service-specific filter
  limit: 500
});

if (result.items.length > 0) {
  console.log("Found " + result.items.length + " errors in the last 24 hours");
}
```

**Example 3 - Pagination:**
```javascript
let allItems = [];
let nextToken = null;

do {
  const result = query{Service}({
    resourceId: "my-resource",
    accountId: "123456789012",
    region: "us-east-1",
    limit: 1000,
    nextToken: nextToken  // Pass token from previous response
  });

  allItems = allItems.concat(result.items);
  nextToken = result.nextToken;
} while (nextToken !== null);

console.log("Total items: " + allItems.length);
```

**Important Notes:**
- Timestamps must be in Unix milliseconds (use `Date.now()` or `new Date().getTime()`)
- Queries are limited to 10,000 items maximum per request (use pagination for more)
- Large time ranges may be slow - use specific time windows when possible
- Filter patterns are service-specific (refer to AWS documentation)
- Always check `nextToken` for pagination - large result sets require multiple requests
"#
    .to_string()
}
```

**Reference**: `src/app/agent_framework/v8_bindings/bindings/cloudwatch_logs.rs` (lines 1-379)

**Critical Points**:
- ✅ **camelCase**: Use `#[serde(rename_all = "camelCase")]` for JavaScript compatibility
- ✅ **block_in_place**: Use `tokio::task::block_in_place()` to avoid runtime errors
- ✅ **Error Handling**: Throw V8 exceptions, never panic
- ✅ **Type Conversion**: Parse via JSON for type safety
- ✅ **Documentation**: Comprehensive LLM-friendly docs with examples

---

### 2.2 Register in Bindings Module

**File**: `src/app/agent_framework/v8_bindings/bindings/mod.rs`

**Add module declaration**:

```rust
pub mod accounts;
pub mod cloudwatch_logs;  // ADD THIS
pub mod regions;
pub mod resources;
// Add new services here
```

**Update `register_bindings()` function**:

```rust
/// Register all bound functions into a V8 context
pub fn register_bindings(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
) -> Result<()> {
    accounts::register(scope)?;
    regions::register(scope)?;
    resources::register(scope)?;
    cloudwatch_logs::register(scope)?;  // ADD THIS
    // Add new service registrations here
    Ok(())
}
```

**Update `get_api_documentation()` function**:

```rust
/// Get the LLM documentation for all bound functions
pub fn get_api_documentation() -> String {
    let mut docs = String::new();

    docs.push_str("# Available JavaScript APIs\n\n");

    // Existing sections...
    docs.push_str("## Account Management\n\n");
    docs.push_str(&accounts::get_documentation());

    docs.push_str("\n## Region Information\n\n");
    docs.push_str(&regions::get_documentation());

    docs.push_str("\n## Resource Queries\n\n");
    docs.push_str(&resources::get_documentation());

    // ADD THIS SECTION
    docs.push_str("\n## CloudWatch Logs\n\n");
    docs.push_str(&cloudwatch_logs::get_documentation());

    // Add new service documentation sections here

    docs
}
```

**Reference**: `src/app/agent_framework/v8_bindings/bindings/mod.rs` (lines 1-130)

---

## Layer 3: Agent Tool Integration

### Purpose

Update the `execute_javascript` tool description so the LLM knows about the new function.

### File Location

```
src/app/agent_framework/tools/execute_javascript.rs
```

---

### 3.1 Update Tool Description

**Find the `description()` method** (around line 174) and add your function to the documentation.

**Add to function list** (around line 194-199):

```rust
Available JavaScript APIs:
- listAccounts(): List all configured AWS accounts
- listRegions(): List all AWS regions
- queryResources(options): Query AWS resources across accounts/regions
- query{Service}(params): Query {Service} data for analysis and monitoring  // ADD THIS
  Parameters: { resourceId: string, accountId: string, region: string, startTime?: number, endTime?: number, filterPattern?: string, limit?: number }
  Returns: { items: Array<{timestamp: number, data: string}>, nextToken: string|null, totalItems: number, statistics: {...} }
```

**Add to examples list** (around line 237+):

```rust
9. Query {Service} for recent data:  // ADD THIS
   {"code": "const results = query{Service}({ resourceId: 'my-resource', accountId: '123456789012', region: 'us-east-1', startTime: Date.now() - (60 * 60 * 1000), limit: 100 }); results.items.forEach(item => console.log(new Date(item.timestamp).toISOString() + ': ' + item.data));"}
```

**Reference**: `src/app/agent_framework/tools/execute_javascript.rs` (lines 174-246)

**Critical Points**:
- ✅ Add function to the "Available JavaScript APIs" list
- ✅ Include parameter types and return type
- ✅ Add concrete usage example to examples list
- ✅ Keep description concise but informative

---

## Layer 4: UI Viewer Window

### Purpose

Create a visual window for users to explore service data with search/filtering capabilities.

### File Location

```
src/app/dashui/{service}_window.rs
```

Example: `src/app/dashui/cloudwatch_logs_window.rs`

---

### 4.1 Create Window File

**Template**:

```rust
//! {Service} Viewer Window
//!
//! Displays {Service} data for AWS resources with search filtering and pagination.

use super::window_focus::FocusableWindow;
use crate::app::{service}::{Service}Client, ResultItem, QueryResult};
use crate::app::resource_explorer::credentials::CredentialCoordinator;
use chrono::{DateTime, Utc};
use eframe::egui;
use egui::{Color32, Context, RichText, Ui};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::sync::Arc;
use std::sync::mpsc;

/// Maximum number of items to display (performance limit)
const MAX_DISPLAY_ITEMS: usize = 1000;

/// Parameters for opening the window
#[derive(Clone)]
pub struct {Service}ShowParams {
    /// Service-specific identifier
    pub service_identifier: String,

    /// Human-readable resource name (for window title)
    pub resource_name: String,

    /// AWS account ID
    pub account_id: String,

    /// AWS region
    pub region: String,
}

/// Result type for background loading
type LoadResult = Result<QueryResult, String>;

/// {Service} viewer window with async data loading
pub struct {Service}Window {
    /// Window open state
    pub open: bool,

    // Display parameters
    service_identifier: String,
    resource_name: String,
    account_id: String,
    region: String,

    // State
    items: Vec<ResultItem>,
    search_filter: String,
    loading: bool,
    error_message: Option<String>,

    // Services
    client: Arc<{Service}Client>,
    fuzzy_matcher: SkimMatcherV2,

    // Channel for receiving results from background thread
    receiver: mpsc::Receiver<LoadResult>,
    sender: mpsc::Sender<LoadResult>,
}

impl {Service}Window {
    /// Create new window
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            open: false,
            service_identifier: String::new(),
            resource_name: String::new(),
            account_id: String::new(),
            region: String::new(),
            items: Vec::new(),
            search_filter: String::new(),
            loading: false,
            error_message: None,
            client: Arc::new({Service}Client::new(credential_coordinator)),
            fuzzy_matcher: SkimMatcherV2::default(),
            receiver,
            sender,
        }
    }

    /// Open window for a specific resource
    pub fn open_for_resource(&mut self, params: {Service}ShowParams) {
        self.service_identifier = params.service_identifier;
        self.resource_name = params.resource_name;
        self.account_id = params.account_id;
        self.region = params.region;
        self.search_filter.clear();
        self.error_message = None;
        self.open = true;

        // Load initial data
        self.refresh_items();
    }

    /// Refresh data from AWS
    fn refresh_items(&mut self) {
        self.loading = true;
        self.error_message = None;

        let client = Arc::clone(&self.client);
        let account_id = self.account_id.clone();
        let region = self.region.clone();
        let identifier = self.service_identifier.clone();
        let sender = self.sender.clone();

        // Spawn async task to fetch data
        tokio::spawn(async move {
            let result = client
                .get_recent_items(&account_id, &region, &identifier, 100)
                .await
                .map_err(|e| e.to_string());

            let _ = sender.send(result);
        });
    }

    /// Show window with optional focus
    fn show_with_focus(&mut self, ctx: &Context, bring_to_front: bool) {
        // Check for results from background thread
        if let Ok(result) = self.receiver.try_recv() {
            self.loading = false;
            match result {
                Ok(query_result) => {
                    self.items = query_result.items;
                }
                Err(error) => {
                    self.error_message = Some(error);
                }
            }
        }

        // Create window
        let mut window = egui::Window::new(format!("{Service}: {}", self.resource_name))
            .id(egui::Id::new("{service}_window"))
            .open(&mut self.open)
            .default_width(900.0)
            .default_height(700.0)
            .resizable(true)
            .collapsible(false);

        if bring_to_front {
            window = window.current_pos([100.0, 100.0]);
        }

        window.show(ctx, |ui| {
            self.render_ui(ui);
        });
    }

    /// Render window UI
    fn render_ui(&mut self, ui: &mut Ui) {
        // Header with resource info
        ui.horizontal(|ui| {
            ui.label(RichText::new("Identifier:").strong());
            ui.label(&self.service_identifier);

            ui.separator();

            ui.label(RichText::new("Account:").strong());
            ui.label(&self.account_id);

            ui.separator();

            ui.label(RichText::new("Region:").strong());
            ui.label(&self.region);
        });

        ui.separator();

        // Search and refresh controls
        ui.horizontal(|ui| {
            ui.label("Search:");
            let search_response = ui.text_edit_singleline(&mut self.search_filter);

            if search_response.changed() {
                // Search filter changed - UI will update on next frame
            }

            if ui.button("Refresh").clicked() {
                self.refresh_items();
            }

            if ui.button("Clear").clicked() {
                self.search_filter.clear();
            }
        });

        ui.separator();

        // Loading indicator
        if self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading data...");
            });
        }

        // Error message
        if let Some(error) = &self.error_message {
            ui.colored_label(
                Color32::RED,
                format!("Error: {}", error)
            );
        }

        // Filter items based on search
        let filtered_items: Vec<_> = if self.search_filter.is_empty() {
            self.items.iter().collect()
        } else {
            self.items
                .iter()
                .filter(|item| {
                    self.fuzzy_matcher
                        .fuzzy_match(&item.data, &self.search_filter)
                        .is_some()
                })
                .collect()
        };

        // Items display with scrolling
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 40.0)
            .show(ui, |ui| {
                for item in filtered_items.iter().take(MAX_DISPLAY_ITEMS) {
                    ui.horizontal(|ui| {
                        // Format timestamp
                        let timestamp = DateTime::from_timestamp(
                            item.timestamp / 1000,
                            ((item.timestamp % 1000) * 1_000_000) as u32,
                        )
                        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());

                        ui.label(
                            RichText::new(
                                timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
                            )
                            .color(Color32::GRAY)
                            .monospace()
                        );

                        // Item data
                        ui.label(&item.data);
                    });

                    ui.separator();
                }

                // Warning if too many items
                if filtered_items.len() > MAX_DISPLAY_ITEMS {
                    ui.colored_label(
                        Color32::YELLOW,
                        format!(
                            "Showing first {} of {} items (refine your search)",
                            MAX_DISPLAY_ITEMS,
                            filtered_items.len()
                        )
                    );
                }
            });

        // Footer with item count
        ui.separator();
        ui.horizontal(|ui| {
            let showing = filtered_items.len().min(MAX_DISPLAY_ITEMS);
            let total = self.items.len();

            if self.search_filter.is_empty() {
                ui.label(format!("Showing {} items", showing));
            } else {
                ui.label(format!(
                    "Showing {} of {} items (filtered from {} total)",
                    showing,
                    filtered_items.len(),
                    total
                ));
            }
        });
    }
}

impl FocusableWindow for {Service}Window {
    type ShowParams = {Service}ShowParams;

    fn window_id(&self) -> &'static str {
        "{service}_window"
    }

    fn window_title(&self) -> String {
        format!("{Service}: {}", self.resource_name)
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    fn show(&mut self, ctx: &Context) {
        self.show_with_focus(ctx, false);
    }

    fn show_with_params(
        &mut self,
        ctx: &Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        // First open for resource
        self.open_for_resource(params);

        // Then show with optional focus
        self.show_with_focus(ctx, bring_to_front);
    }
}
```

**Reference**: `src/app/dashui/cloudwatch_logs_window.rs` (lines 1-543)

**Critical Points**:
- ✅ Use `mpsc::channel` for async data loading (non-blocking UI)
- ✅ Use `tokio::spawn` for background tasks
- ✅ Use `try_recv()` not `recv()` (would block UI thread)
- ✅ Implement fuzzy search filtering
- ✅ Show loading states and error messages
- ✅ Limit display items for performance (MAX_DISPLAY_ITEMS)
- ✅ Implement `FocusableWindow` trait

---

### 4.2 Register Window in DashUI Module

**File**: `src/app/dashui/mod.rs`

**Add module** (around line 85):

```rust
pub mod cloudwatch_logs_window;  // ADD THIS
// Add other service windows here
```

**Export types** (around line 105):

```rust
pub use cloudwatch_logs_window::{CloudWatchLogsShowParams, CloudWatchLogsWindow};  // ADD THIS
// Export other service window types here
```

---

### 4.3 Add Window to DashApp

**File**: `src/app/dashui/app/mod.rs`

**Import** (around line 13):

```rust
use super::cloudwatch_logs_window::CloudWatchLogsWindow;  // ADD THIS
```

**Add field to DashApp struct** (around line 126):

```rust
pub struct DashApp {
    // Existing fields...

    pub cloudwatch_logs_windows: Vec<CloudWatchLogsWindow>,  // ADD THIS

    // Other fields...
}
```

**Initialize in `DashApp::new()`** (around line 206):

```rust
Self {
    // Existing initializations...

    cloudwatch_logs_windows: Vec::new(),  // ADD THIS

    // Other initializations...
}
```

---

## Layer 5: Explorer Integration

### Purpose

Add "View {Service}" buttons in the Resource Explorer for supported resources.

---

### 5.1 Define Resource Explorer Action

**File**: `src/app/resource_explorer/mod.rs`

**Add variant to `ResourceExplorerAction` enum** (around lines 8-15):

```rust
#[derive(Debug, Clone)]
pub enum ResourceExplorerAction {
    // Existing actions...

    /// Request to open {Service} viewer for a resource
    Open{Service} {
        service_identifier: String,
        resource_name: String,
        account_id: String,
        region: String,
    },
}
```

Example:
```rust
OpenCloudWatchLogs {
    service_identifier: String,
    resource_name: String,
    account_id: String,
    region: String,
},
```

---

### 5.2 Add Button in Resource Explorer Tree

**File**: `src/app/resource_explorer/tree.rs`

**Import resource mapping functions** (around line 2):

```rust
use crate::app::{service}::{get_{service}_identifier, has_{service}_support};
```

Example:
```rust
use crate::app::cloudwatch_logs::{get_cloudwatch_logs_identifier, has_cloudwatch_logs_support};
```

**Add button in resource detail rendering** (around lines 1288-1307):

Find the section where resource details are rendered (inside the expanded resource JSON tree), and add:

```rust
// Check if resource supports {Service}
if has_{service}_support(&resource.resource_type) {
    if let Some(service_id) = get_{service}_identifier(
        &resource.resource_type,
        &resource.display_name,
        Some(&resource.resource_id),
    ) {
        if ui.button("View {Service}").clicked() {
            // Queue action to open {Service} window
            self.pending_explorer_actions.push(
                super::ResourceExplorerAction::Open{Service} {
                    service_identifier: service_id,
                    resource_name: resource.display_name.clone(),
                    account_id: account_id.to_string(),
                    region: region.to_string(),
                }
            );
        }
    }
}
```

Example for CloudWatch Logs:
```rust
// CloudWatch Logs button for supported resources
if has_cloudwatch_logs_support(&resource.resource_type) {
    if let Some(log_group) = get_cloudwatch_logs_identifier(
        &resource.resource_type,
        &resource.display_name,
        Some(&resource.resource_id),
    ) {
        if ui.button("View Logs").clicked() {
            self.pending_explorer_actions.push(
                super::ResourceExplorerAction::OpenCloudWatchLogs {
                    service_identifier: log_group,
                    resource_name: resource.display_name.clone(),
                    account_id: account_id.to_string(),
                    region: region.to_string(),
                }
            );
        }
    }
}
```

**Reference**: `src/app/resource_explorer/tree.rs` (lines 1288-1307)

---

### 5.3 Handle Action in Window Rendering

**File**: `src/app/dashui/app/window_rendering.rs`

**Handle action in `handle_resource_explorer()` method** (around lines 210-233):

```rust
// In handle_resource_explorer method
for action in actions {
    match action {
        // Existing actions...

        ResourceExplorerAction::Open{Service} {
            service_identifier,
            resource_name,
            account_id,
            region,
        } => {
            if let Some(aws_client) = self.resource_explorer.get_aws_client() {
                let credential_coordinator = aws_client.get_credential_coordinator();
                let mut new_window = {Service}Window::new(credential_coordinator);

                new_window.open_for_resource({Service}ShowParams {
                    service_identifier,
                    resource_name,
                    account_id,
                    region,
                });

                self.{service}_windows.push(new_window);
            }
        }
    }
}
```

**Render all windows** (after action handling):

```rust
// Render {Service} windows
for window in &mut self.{service}_windows {
    if window.is_open() {
        window.show(ctx);
    }
}

// Remove closed windows (optional optimization)
self.{service}_windows.retain(|w| w.is_open());
```

Example:
```rust
ResourceExplorerAction::OpenCloudWatchLogs {
    service_identifier,
    resource_name,
    account_id,
    region,
} => {
    if let Some(aws_client) = self.resource_explorer.get_aws_client() {
        let credential_coordinator = aws_client.get_credential_coordinator();
        let mut new_window = CloudWatchLogsWindow::new(credential_coordinator);

        new_window.open_for_resource(CloudWatchLogsShowParams {
            service_identifier,
            resource_name,
            account_id,
            region,
        });

        self.cloudwatch_logs_windows.push(new_window);
    }
}

// Later in rendering:
for window in &mut self.cloudwatch_logs_windows {
    if window.is_open() {
        window.show(ctx);
    }
}
```

**Reference**: `src/app/dashui/app/window_rendering.rs` (lines 210-242)

---

## Step-by-Step Checklist

Use this checklist when integrating a new data plane service:

### Phase 1: AWS SDK Client Layer (2-3 hours)

- [ ] **1.1** Create module directory `src/app/{service}/`
- [ ] **1.2** Implement `types.rs`
  - [ ] Define `QueryOptions` with builder pattern
  - [ ] Define `QueryResult` with `Serialize/Deserialize`
  - [ ] Define `ResultItem` with `Serialize/Deserialize`
  - [ ] Define `QueryStatistics` (if applicable)
  - [ ] Add `Default` implementations
  - [ ] Add unit tests for serialization
- [ ] **1.3** Implement `client.rs`
  - [ ] Define `{Service}Client` struct with `CredentialCoordinator`
  - [ ] Implement `new()` constructor
  - [ ] Implement `query_items()` with full options
  - [ ] Implement `get_recent_items()` convenience method
  - [ ] Implement `list_resources()` (if applicable)
  - [ ] Use `.with_context()` for all error handling
  - [ ] Convert AWS SDK types to your own types
- [ ] **1.4** Implement `resource_mapping.rs`
  - [ ] Implement `has_{service}_support()`
  - [ ] Implement `get_{service}_identifier()`
  - [ ] Implement `get_all_{service}_identifiers()` if needed
  - [ ] Add comprehensive unit tests
- [ ] **1.5** Implement `mod.rs`
  - [ ] Add module documentation with examples
  - [ ] Document architecture and data flow
  - [ ] List supported resource types
  - [ ] Show Rust and JavaScript usage examples
  - [ ] Re-export commonly used types
- [ ] **1.6** Add AWS SDK dependency to `Cargo.toml`
  - [ ] Add `aws-sdk-{service} = "1.x"`
- [ ] **1.7** Export module in `src/app/mod.rs`
  - [ ] Add `pub mod {service};`
- [ ] **1.8** Test compilation: `cargo build`

### Phase 2: V8 JavaScript Binding (1-2 hours)

- [ ] **2.1** Create `src/app/agent_framework/v8_bindings/bindings/{service}.rs`
  - [ ] Define `Query{Service}Args` with `#[serde(rename_all = "camelCase")]`
  - [ ] Define `ResultItemInfo` with `#[serde(rename_all = "camelCase")]`
  - [ ] Define `{Service}QueryResult` with `#[serde(rename_all = "camelCase")]`
  - [ ] Define `QueryStatisticsInfo` with `#[serde(rename_all = "camelCase")]`
  - [ ] Implement `register()` function
  - [ ] Implement `query_{service}_callback()` V8 callback
  - [ ] Implement `execute_query()` with `tokio::task::block_in_place()`
  - [ ] Implement `query_{service}_internal()` async function
  - [ ] Implement `get_documentation()` with comprehensive LLM docs
- [ ] **2.2** Register in `v8_bindings/bindings/mod.rs`
  - [ ] Add `pub mod {service};`
  - [ ] Call `{service}::register(scope)?;` in `register_bindings()`
  - [ ] Add `{service}::get_documentation()` to `get_api_documentation()`
- [ ] **2.3** Test compilation: `cargo build`

### Phase 3: Agent Tool Integration (15 minutes)

- [ ] **3.1** Update `tools/execute_javascript.rs`
  - [ ] Add function to "Available JavaScript APIs" list
  - [ ] Add parameter and return type documentation
  - [ ] Add usage example to examples list
- [ ] **3.2** Test compilation: `cargo build`

### Phase 4: UI Viewer Window (2-3 hours)

- [ ] **4.1** Create `src/app/dashui/{service}_window.rs`
  - [ ] Define `{Service}ShowParams` struct
  - [ ] Define `{Service}Window` struct
  - [ ] Implement `new()` constructor with `mpsc::channel`
  - [ ] Implement `open_for_resource()` method
  - [ ] Implement `refresh_items()` with `tokio::spawn`
  - [ ] Implement `show_with_focus()` method
  - [ ] Implement `render_ui()` method with:
    - [ ] Resource info header
    - [ ] Search/filter controls
    - [ ] Refresh button
    - [ ] Loading indicator
    - [ ] Error display
    - [ ] Scrollable items list
    - [ ] Item count footer
  - [ ] Implement `FocusableWindow` trait
  - [ ] Add fuzzy search filtering
- [ ] **4.2** Register in `dashui/mod.rs`
  - [ ] Add `pub mod {service}_window;`
  - [ ] Export `{Service}ShowParams` and `{Service}Window`
- [ ] **4.3** Add to `dashui/app/mod.rs`
  - [ ] Import window type
  - [ ] Add `{service}_windows: Vec<{Service}Window>` field
  - [ ] Initialize to `Vec::new()` in `DashApp::new()`
- [ ] **4.4** Test compilation: `cargo build`

### Phase 5: Explorer Integration (1 hour)

- [ ] **5.1** Define action in `resource_explorer/mod.rs`
  - [ ] Add `Open{Service}` variant to `ResourceExplorerAction` enum
  - [ ] Include `service_identifier`, `resource_name`, `account_id`, `region` fields
- [ ] **5.2** Add button in `resource_explorer/tree.rs`
  - [ ] Import `has_{service}_support` and `get_{service}_identifier`
  - [ ] Add button in resource detail rendering (around line 1288)
  - [ ] Check support with `has_{service}_support()`
  - [ ] Get identifier with `get_{service}_identifier()`
  - [ ] Push action to `pending_explorer_actions`
- [ ] **5.3** Handle action in `dashui/app/window_rendering.rs`
  - [ ] Match `Open{Service}` action
  - [ ] Create window instance
  - [ ] Call `open_for_resource()` with params
  - [ ] Push to `{service}_windows` vector
  - [ ] Render all windows in loop
- [ ] **5.4** Test compilation: `cargo build`

### Phase 6: Testing (1-2 hours)

- [ ] **6.1** Unit tests
  - [ ] Test types serialization/deserialization
  - [ ] Test query options builder pattern
  - [ ] Test resource mapping functions
- [ ] **6.2** Integration tests (if safe with AWS)
  - [ ] Test end-to-end query with real AWS
  - [ ] Test pagination
  - [ ] Test error handling
- [ ] **6.3** UI tests (optional)
  - [ ] Test window opening
  - [ ] Test search filtering
  - [ ] Test refresh
- [ ] **6.4** Agent tests
  - [ ] Test LLM can discover function
  - [ ] Test LLM can call function correctly
  - [ ] Test results are usable
- [ ] **6.5** Run test suite: `./scripts/test-chunks.sh fast`

### Phase 7: Documentation (30 minutes)

- [ ] **7.1** Update project documentation
  - [ ] Add entry to technical documentation
  - [ ] Document any service-specific patterns
  - [ ] Add usage examples
- [ ] **7.2** Commit changes
  - [ ] Create descriptive commit message
  - [ ] Reference any related issues

---

## Naming Conventions

### File Names (snake_case)
- **Module directory**: `{service}` (lowercase with underscores)
  - Examples: `cloudwatch_logs`, `cloudwatch_metrics`, `cloudtrail`, `athena`
- **Client file**: `{service}/client.rs`
- **Types file**: `{service}/types.rs`
- **Mapping file**: `{service}/resource_mapping.rs`
- **V8 binding**: `v8_bindings/bindings/{service}.rs`
- **UI window**: `dashui/{service}_window.rs`

### Type Names (PascalCase)
- **Client**: `{Service}Client`
  - Examples: `CloudWatchLogsClient`, `CloudWatchMetricsClient`, `CloudTrailClient`
- **Query args (Rust)**: `QueryOptions`
- **Query args (V8)**: `Query{Service}Args`
  - Examples: `QueryCloudWatchLogsArgs`, `QueryCloudWatchMetricsArgs`
- **Result**: `{Service}QueryResult`
  - Examples: `CloudWatchLogsQueryResult`, `CloudWatchMetricsQueryResult`
- **Result item**: `ResultItem` (Rust), `ResultItemInfo` (V8)
- **Window**: `{Service}Window`
  - Examples: `CloudWatchLogsWindow`, `CloudWatchMetricsWindow`
- **Show params**: `{Service}ShowParams`
  - Examples: `CloudWatchLogsShowParams`, `CloudWatchMetricsShowParams`
- **Action enum**: `Open{Service}`
  - Examples: `OpenCloudWatchLogs`, `OpenCloudWatchMetrics`

### Function Names
- **JavaScript (camelCase)**: `query{Service}()`
  - Examples: `queryCloudWatchLogs()`, `queryCloudWatchMetrics()`, `queryCloudTrail()`
- **Rust (snake_case)**: `query_{items}()`, `get_recent_{items}()`
  - Examples: `query_log_events()`, `query_metrics()`, `query_trail_events()`
- **Resource mapping**: `has_{service}_support()`, `get_{service}_identifier()`
  - Examples: `has_cloudwatch_logs_support()`, `get_cloudwatch_logs_identifier()`
- **V8 callback**: `query_{service}_callback()`
  - Examples: `query_cloudwatch_logs_callback()`
- **V8 internal**: `query_{service}_internal()`

### Variable Names
- **Client instance**: `{service}_client` or `client`
- **Credential coordinator**: `credential_coordinator`
- **Query options**: `options`
- **Query result**: `result`
- **Service identifier**: `service_identifier` or `resource_identifier`
- **Account/Region**: `account_id`, `region`

---

## Common Patterns

### 1. Error Handling

**Always use `.with_context()` for AWS SDK errors:**

```rust
let response = client
    .some_operation()
    .send()
    .await
    .with_context(|| format!(
        "Failed to perform operation for resource: {}",
        resource_id
    ))?;
```

**Throw V8 exceptions in V8 bindings:**

```rust
let msg = v8::String::new(scope, "Operation failed").unwrap();
let error = v8::Exception::error(scope, msg);
scope.throw_exception(error);
return;
```

---

### 2. Async/Sync Boundaries

**V8 bindings - Use `block_in_place`:**

```rust
fn execute_query(args: QueryArgs) -> Result<QueryResult> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            query_internal(args).await
        })
    })
}
```

**UI windows - Use `tokio::spawn` with channels:**

```rust
let (sender, receiver) = mpsc::channel();

// In refresh method:
tokio::spawn(async move {
    let result = client.query().await;
    let _ = sender.send(result);
});

// In render method:
if let Ok(result) = self.receiver.try_recv() {
    // Update UI state
}
```

---

### 3. Type Conversions

**JavaScript-facing types use camelCase:**

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // CRITICAL
pub struct QueryArgs {
    pub resource_id: String,     // → resourceId in JSON
    pub account_id: String,      // → accountId in JSON
    pub start_time: Option<i64>, // → startTime in JSON
}
```

**Convert via JSON for type safety:**

```rust
// V8 object → JSON string → Rust struct
let json_str = v8::json::stringify(scope, args_obj.into())?;
let query_args: QueryArgs = serde_json::from_str(&json_str)?;

// Rust struct → JSON string → V8 object
let result_json = serde_json::to_string(&result)?;
let result_value = v8::json::parse(scope, v8::String::new(scope, &result_json)?)?;
```

---

### 4. Credential Management

**Always use CredentialCoordinator for AWS operations:**

```rust
let aws_config = self
    .credential_coordinator
    .create_aws_config_for_account(account_id, region)
    .await?;

let client = aws_sdk_service::Client::new(&aws_config);
```

**Get coordinator from global AWS client in V8 bindings:**

```rust
let aws_client = crate::app::agent_framework::tools_registry::get_global_aws_client()
    .ok_or_else(|| anyhow!("AWS client not initialized"))?;

let credential_coordinator = aws_client.get_credential_coordinator();
```

---

### 5. Builder Patterns

**Provide ergonomic builders for options:**

```rust
impl QueryOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_start_time(mut self, start_time: i64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    pub fn with_limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }
}

// Usage:
let options = QueryOptions::new()
    .with_start_time(1234567890)
    .with_limit(100);
```

---

### 6. Fuzzy Search in UI

**Use SkimMatcherV2 for fuzzy filtering:**

```rust
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

// In struct:
fuzzy_matcher: SkimMatcherV2,

// In init:
fuzzy_matcher: SkimMatcherV2::default(),

// In filtering:
let filtered_items: Vec<_> = items
    .iter()
    .filter(|item| {
        self.fuzzy_matcher
            .fuzzy_match(&item.data, &self.search_filter)
            .is_some()
    })
    .collect();
```

---

## Testing Strategy

### Unit Tests

Test individual components in isolation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_options_builder() {
        let options = QueryOptions::new()
            .with_start_time(1000)
            .with_limit(50);

        assert_eq!(options.start_time, Some(1000));
        assert_eq!(options.limit, Some(50));
    }

    #[test]
    fn test_serialization() {
        let result = QueryResult::new(
            vec![ResultItem {
                timestamp: 123,
                data: "test".to_string(),
            }],
            None,
        );

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: QueryResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.items.len(), 1);
    }

    #[test]
    fn test_resource_mapping() {
        assert!(has_service_support("AWS::Lambda::Function"));

        let id = get_service_identifier(
            "AWS::Lambda::Function",
            "my-function",
            None,
        );

        assert_eq!(id, Some("/aws/lambda/my-function".to_string()));
    }
}
```

### Integration Tests

Test end-to-end with real AWS (if safe and cost-effective):

```rust
#[tokio::test]
#[ignore]  // Run explicitly with --ignored flag
async fn test_real_aws_query() {
    // This test requires real AWS credentials and resources
    // Run with: cargo test test_real_aws_query -- --ignored

    let credential_coordinator = /* setup */;
    let client = ServiceClient::new(credential_coordinator);

    let result = client
        .get_recent_items(
            "123456789012",
            "us-east-1",
            "test-resource",
            10,
        )
        .await;

    assert!(result.is_ok());
}
```

### UI Tests (Optional)

Test UI components with egui_kittest:

```rust
#[test]
fn test_window_opening() {
    // Create test harness
    let mut harness = TestHarness::new();

    // Open window
    let params = ServiceShowParams {
        service_identifier: "test-id".to_string(),
        resource_name: "Test Resource".to_string(),
        account_id: "123456789012".to_string(),
        region: "us-east-1".to_string(),
    };

    harness.window.open_for_resource(params);

    // Verify window is open
    assert!(harness.window.is_open());
}
```

### Agent Integration Tests

Test that the agent can discover and use the function:

```rust
#[tokio::test]
async fn test_agent_can_call_function() {
    // Create agent with execute_javascript tool
    let agent = /* setup */;

    // Provide prompt that should use the function
    let prompt = "Query CloudWatch Logs for Lambda function my-func in us-east-1";

    let response = agent.execute(prompt).await;

    // Verify function was called
    assert!(response.contains("queryCloudWatchLogs"));
}
```

---

## Service-Specific Examples

### CloudWatch Metrics

**Resource Identifier Format**: `{Namespace}/{MetricName}`

**Example identifiers**:
- `AWS/Lambda/Duration` - Lambda duration metric
- `AWS/EC2/CPUUtilization` - EC2 CPU metric
- `AWS/RDS/DatabaseConnections` - RDS connections

**Service-Specific Fields**:
```rust
pub struct QueryOptions {
    pub namespace: String,         // AWS/Lambda, AWS/EC2, etc.
    pub metric_name: String,       // Duration, CPUUtilization, etc.
    pub dimensions: Vec<Dimension>, // [{Name: "FunctionName", Value: "my-func"}]
    pub statistics: Vec<String>,   // ["Average", "Sum", "Maximum"]
    pub period: i32,               // 60, 300, 3600 (seconds)
    pub start_time: i64,
    pub end_time: i64,
}

pub struct ResultItem {
    pub timestamp: i64,
    pub average: Option<f64>,
    pub sum: Option<f64>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
}
```

**SDK Operation**: `get_metric_statistics()`

---

### CloudTrail

**Resource Identifier Format**: `{EventName}` or `all`

**Example identifiers**:
- `RunInstances` - EC2 instance launches
- `CreateBucket` - S3 bucket creations
- `all` - All events

**Service-Specific Fields**:
```rust
pub struct QueryOptions {
    pub start_time: i64,
    pub end_time: i64,
    pub lookup_attributes: Vec<LookupAttribute>, // [{AttributeKey: "ResourceType", AttributeValue: "AWS::EC2::Instance"}]
    pub max_results: Option<i32>,
    pub next_token: Option<String>,
}

pub struct ResultItem {
    pub event_id: String,
    pub event_name: String,
    pub event_time: i64,
    pub username: String,
    pub resources: Vec<Resource>,
    pub cloud_trail_event: String, // JSON string of full event
}
```

**SDK Operation**: `lookup_events()`

---

### Athena

**Resource Identifier Format**: `{Database}/{Table}` or custom SQL

**Example identifiers**:
- `mydatabase/mytable` - Query specific table
- `custom` - Custom SQL query

**Service-Specific Fields**:
```rust
pub struct QueryOptions {
    pub query_string: String,      // SQL query
    pub database: String,          // Database name
    pub output_location: String,   // S3 location for results
    pub max_results: Option<i32>,
    pub next_token: Option<String>,
}

pub struct ResultItem {
    pub row: Vec<String>,          // Column values
}

pub struct QueryResult {
    pub items: Vec<ResultItem>,
    pub column_info: Vec<ColumnInfo>, // Column names and types
    pub next_token: Option<String>,
    pub query_execution_id: String,   // For checking query status
}
```

**SDK Operations**: `start_query_execution()`, `get_query_results()`

**Special Considerations**: Athena queries are asynchronous - need to poll for completion

---

## File Path Quick Reference

```
src/app/{service}/
├── mod.rs                       # Module definition and public API
├── types.rs                     # Data structures
├── client.rs                    # AWS SDK wrapper
└── resource_mapping.rs          # Resource type → identifier mapping

src/app/agent_framework/v8_bindings/bindings/
├── mod.rs                       # Register all bindings (MODIFY)
└── {service}.rs                 # V8 JavaScript binding (CREATE)

src/app/agent_framework/tools/
└── execute_javascript.rs        # Tool description (MODIFY)

src/app/resource_explorer/
├── mod.rs                       # ResourceExplorerAction enum (MODIFY)
└── tree.rs                      # "View X" button (MODIFY)

src/app/dashui/
├── mod.rs                       # Window exports (MODIFY)
├── {service}_window.rs          # UI viewer window (CREATE)
└── app/
    ├── mod.rs                   # DashApp struct (MODIFY)
    └── window_rendering.rs      # Action handler (MODIFY)

Cargo.toml                       # Add aws-sdk-{service} dependency (MODIFY)
src/app/mod.rs                   # Export {service} module (MODIFY)
```

---

## Summary

This guide provides a complete, repeatable pattern for integrating AWS data plane services into the application. The CloudWatch Logs implementation (commits 770adbe, 3819148, 972ed97) demonstrates this pattern successfully.

### Key Takeaways

1. **5-Layer Architecture**: Client → V8 Binding → Agent Integration → UI Window → Explorer Integration
2. **Complete Integration**: All 4 components (Client, V8, UI, Explorer) are part of the standard pattern
3. **Type Safety**: Strong typing in Rust, proper conversion to JavaScript via Serde
4. **Multi-Account**: Always use CredentialCoordinator for AWS operations
5. **Async Everywhere**: Proper async handling with tokio at all layers
6. **Reusable Pattern**: Same structure for CloudWatch Metrics, CloudTrail, Athena, etc.

### Time Investment

- **Minimal (Agent-only)**: ~2-3 hours (Client + V8 binding)
- **Complete (with UI)**: ~4-6 hours (All 4 layers)
- **With Testing**: Add ~1-2 hours

### Next Services to Integrate

Following this pattern, you can rapidly add:
- **CloudWatch Metrics**: Query and visualize metrics
- **CloudTrail**: View API call history
- **Athena**: Run SQL queries on S3 data
- **X-Ray**: Trace distributed application requests
- **AWS Config**: View resource configuration history

Each follows the exact same pattern documented here.
