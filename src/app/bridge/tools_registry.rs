//! AWS Resource Tools for AI Agent
//!
//! This module provides a registry and common utilities for AWS bridge tools.
//! Individual tool implementations are in the tools/ subfolder.

use crate::app::resource_explorer::aws_client::AWSResourceClient;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use stood::tools::Tool;
use tracing::{info, warn};

// Import individual tools for registry functions
use super::tools::*;

/// Global tool context for accessing AWS client at runtime
static GLOBAL_AWS_CLIENT: RwLock<Option<Arc<AWSResourceClient>>> = RwLock::new(None);

/// Set the global AWS client for all tools to use
pub fn set_global_aws_client(client: Option<Arc<AWSResourceClient>>) {
    match GLOBAL_AWS_CLIENT.write() {
        Ok(mut guard) => {
            let client_status = if client.is_some() { "✅ Set" } else { "❌ Cleared" };
            info!("🔧 Global AWS client updated for bridge tools: {}", client_status);
            *guard = client;
        }
        Err(e) => {
            warn!("Failed to update global AWS client: {}", e);
        }
    }
}

/// Get the global AWS client for tool execution
pub fn get_global_aws_client() -> Option<Arc<AWSResourceClient>> {
    match GLOBAL_AWS_CLIENT.read() {
        Ok(guard) => {
            let client = guard.clone();
            info!("🔍 Global AWS client access: {}", if client.is_some() { "✅ Available" } else { "❌ Not set" });
            client
        }
        Err(e) => {
            warn!("Failed to read global AWS client: {}", e);
            None
        }
    }
}

/// Simplified resource summary for tool responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceSummary {
    /// AWS resource type
    pub resource_type: String,
    /// AWS account ID
    pub account_id: String,
    /// AWS region
    pub region: String,
    /// Resource identifier
    pub resource_id: String,
    /// Human-readable display name
    pub display_name: String,
    /// Resource status if available
    pub status: Option<String>,
    /// Key properties as JSON
    pub properties: serde_json::Value,
    /// Resource tags
    pub tags: Vec<String>,
}


/// Registry for AWS tools that can be added to an Agent
pub fn get_aws_tools(aws_client: Option<Arc<AWSResourceClient>>) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(AwsListResourcesTool::new(aws_client.clone())),
        Box::new(AwsDescribeResourceTool::new(aws_client.clone())),
        Box::new(AwsFindAccountTool::new_uninitialized()),
        Box::new(AwsFindRegionTool::new_uninitialized()),
    ]
}