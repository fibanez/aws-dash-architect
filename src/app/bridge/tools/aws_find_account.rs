//! AWS Find Account Tool
//!
//! This tool allows AI agents to search for AWS accounts using fuzzy matching
//! on account IDs, names, and email addresses without making API calls.

use crate::app::aws_identity::AwsIdentityCenter;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::{Arc, Mutex, RwLock};
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{info, warn};

/// Global access to AwsIdentityCenter for account lookups
static GLOBAL_AWS_IDENTITY: RwLock<Option<Arc<Mutex<AwsIdentityCenter>>>> = RwLock::new(None);

/// Set the global AwsIdentityCenter for account lookups
pub fn set_global_aws_identity(identity: Option<Arc<Mutex<AwsIdentityCenter>>>) {
    match GLOBAL_AWS_IDENTITY.write() {
        Ok(mut guard) => {
            *guard = identity;
            info!("🔧 Global AwsIdentityCenter updated for account search");
        }
        Err(e) => {
            warn!("Failed to update global AwsIdentityCenter: {}", e);
        }
    }
}

/// Get the global AwsIdentityCenter for account lookups
fn get_global_aws_identity() -> Option<Arc<Mutex<AwsIdentityCenter>>> {
    match GLOBAL_AWS_IDENTITY.read() {
        Ok(guard) => guard.clone(),
        Err(e) => {
            warn!("Failed to read global AwsIdentityCenter: {}", e);
            None
        }
    }
}

/// Account search result with match scoring
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccountSearchResult {
    /// AWS account ID
    pub account_id: String,
    /// Human-readable account name
    pub account_name: String,
    /// Account email address (if available)
    pub account_email: Option<String>,
    /// Match confidence score (0.0 to 1.0)
    pub match_score: f64,
    /// What field(s) matched the search
    pub matched_fields: Vec<String>,
    /// Source of the account data
    pub source: String,
}

/// AWS Find Account Tool
#[derive(Clone, Debug)]
pub struct AwsFindAccountTool {
    aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>,
}

impl AwsFindAccountTool {
    pub fn new(aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>) -> Self {
        Self { aws_identity }
    }

    pub fn new_uninitialized() -> Self {
        Self {
            aws_identity: None,
        }
    }

    pub fn set_aws_identity(&mut self, aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>) {
        self.aws_identity = aws_identity;
    }

    /// Perform fuzzy matching on a string
    fn fuzzy_match(query: &str, target: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let target_lower = target.to_lowercase();

        // Exact match
        if query_lower == target_lower {
            return 1.0;
        }

        // Starts with
        if target_lower.starts_with(&query_lower) {
            return 0.9;
        }

        // Contains
        if target_lower.contains(&query_lower) {
            return 0.7;
        }

        // Subsequence matching (for partial IDs)
        if query_lower.len() >= 3 && Self::is_subsequence(&query_lower, &target_lower) {
            return 0.5;
        }

        0.0
    }

    /// Check if query is a subsequence of target
    fn is_subsequence(query: &str, target: &str) -> bool {
        let mut target_chars = target.chars();
        query.chars().all(|qc| target_chars.any(|tc| tc == qc))
    }

    /// Search accounts from AwsIdentityCenter
    fn search_identity_accounts(&self, query: &str) -> Vec<AccountSearchResult> {
        let mut results = Vec::new();

        // Try instance AwsIdentityCenter first, then global
        let global_identity = get_global_aws_identity();
        let identity = self.aws_identity.as_ref().or(global_identity.as_ref());

        if let Some(identity) = identity {
            if let Ok(identity_guard) = identity.lock() {
                for account in &identity_guard.accounts {
                    let mut matched_fields = Vec::new();
                    let mut max_score = 0.0f64;

                    // Match against account ID
                    let id_score = Self::fuzzy_match(query, &account.account_id);
                    if id_score > 0.0 {
                        matched_fields.push("account_id".to_string());
                        max_score = max_score.max(id_score);
                    }

                    // Match against account name
                    let name_score = Self::fuzzy_match(query, &account.account_name);
                    if name_score > 0.0 {
                        matched_fields.push("account_name".to_string());
                        max_score = max_score.max(name_score);
                    }

                    // Match against account email if available
                    if let Some(ref email) = account.account_email {
                        let email_score = Self::fuzzy_match(query, email);
                        if email_score > 0.0 {
                            matched_fields.push("account_email".to_string());
                            max_score = max_score.max(email_score);
                        }
                    }

                    // Include accounts with any match
                    if max_score > 0.0 {
                        results.push(AccountSearchResult {
                            account_id: account.account_id.clone(),
                            account_name: account.account_name.clone(),
                            account_email: account.account_email.clone(),
                            match_score: max_score,
                            matched_fields,
                            source: "AwsIdentityCenter".to_string(),
                        });
                    }
                }
            }
        }

        results
    }
}

impl Default for AwsFindAccountTool {
    fn default() -> Self {
        Self::new_uninitialized()
    }
}

#[async_trait]
impl Tool for AwsFindAccountTool {
    fn name(&self) -> &str {
        "aws_find_account"
    }

    fn description(&self) -> &str {
        r#"Search for AWS accounts using fuzzy matching on account IDs, names, and email addresses.

This tool searches through available AWS accounts without making API calls, using cached data from:
- AwsIdentityCenter system (with detailed account info)

Search supports:
- Exact account ID matching: "123456789012"
- Partial account ID matching: "123456", "789012"
- Account name fuzzy matching: "production", "prod", "dev"
- Email address matching: "admin@company.com", "company.com"
- Case-insensitive matching

Examples:
- Find production accounts: {"query": "production"}
- Find account by partial ID: {"query": "123456"}
- Find accounts by email domain: {"query": "company.com"}
- List all available accounts: {"query": ""}
- Limit results: {"query": "dev", "limit": 5}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for account ID, name, or email. Empty string returns all accounts.",
                    "examples": ["production", "123456789012", "dev", "admin@company.com", ""]
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of results to return (default: 10)",
                    "default": 10,
                    "minimum": 1,
                    "maximum": 100
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        info!("🔍 aws_find_account executing with parameters: {:?}", parameters);

        // Parse parameters
        let params = parameters.unwrap_or_else(|| serde_json::json!({}));
        
        let query = params.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        
        let limit = params.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(100) as usize;

        info!("🔍 Searching for accounts matching: '{}'", query);

        // Search from AwsIdentityCenter
        let mut all_results = self.search_identity_accounts(query);
        info!("📊 Found {} matches from AwsIdentityCenter", all_results.len());

        // Sort by match score (highest first)
        all_results.sort_by(|a, b| b.match_score.partial_cmp(&a.match_score).unwrap_or(std::cmp::Ordering::Equal));

        let total_before_limit = all_results.len();

        // Apply limit
        let final_results: Vec<_> = all_results.into_iter().take(limit).collect();

        let duration = start_time.elapsed();

        let execution_summary = if query.is_empty() {
            format!(
                "Listed {} AWS accounts (showing {} of {}) in {:.2}s",
                final_results.len(),
                final_results.len(),
                total_before_limit,
                duration.as_secs_f64()
            )
        } else {
            format!(
                "Found {} AWS accounts matching '{}' (showing {} of {}) in {:.2}s",
                total_before_limit,
                query,
                final_results.len(),
                total_before_limit,
                duration.as_secs_f64()
            )
        };

        info!("📊 aws_find_account completed: {}", execution_summary);

        // Create response JSON
        let response_data = serde_json::json!({
            "accounts": final_results,
            "total_matches": total_before_limit,
            "showing_count": final_results.len(),
            "query": query,
            "limit": limit,
            "execution_summary": execution_summary,
            "duration_seconds": duration.as_secs_f64()
        });

        Ok(ToolResult::success(response_data))
    }
}