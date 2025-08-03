//! AWS Find Region Tool
//!
//! This tool allows AI agents to search for AWS regions using fuzzy matching
//! on region codes and display names without making API calls.

use crate::app::{
    cfn_resources::AWS_REGIONS,
    resource_explorer::dialogs::get_default_regions,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::info;

/// Region search result with match scoring
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegionSearchResult {
    /// AWS region code (e.g., "us-east-1")
    pub region_code: String,
    /// Human-readable display name (e.g., "US East (N. Virginia)")
    pub display_name: String,
    /// Match confidence score (0.0 to 1.0)
    pub match_score: f64,
    /// What field(s) matched the search
    pub matched_fields: Vec<String>,
    /// Source of the region data
    pub source: String,
    /// Whether this is a commonly used region
    pub is_default: bool,
}

/// AWS Find Region Tool
#[derive(Clone, Debug)]
pub struct AwsFindRegionTool {}

impl AwsFindRegionTool {
    pub fn new_uninitialized() -> Self {
        Self {}
    }

    /// Get human-readable display name for a region
    fn format_region_display_name(region_code: &str) -> String {
        match region_code {
            "us-east-1" => "US East (N. Virginia)".to_string(),
            "us-east-2" => "US East (Ohio)".to_string(),
            "us-west-1" => "US West (N. California)".to_string(),
            "us-west-2" => "US West (Oregon)".to_string(),
            "af-south-1" => "Africa (Cape Town)".to_string(),
            "ap-east-1" => "Asia Pacific (Hong Kong)".to_string(),
            "ap-south-1" => "Asia Pacific (Mumbai)".to_string(),
            "ap-southeast-1" => "Asia Pacific (Singapore)".to_string(),
            "ap-southeast-2" => "Asia Pacific (Sydney)".to_string(),
            "ap-southeast-3" => "Asia Pacific (Jakarta)".to_string(),
            "ap-northeast-1" => "Asia Pacific (Tokyo)".to_string(),
            "ap-northeast-2" => "Asia Pacific (Seoul)".to_string(),
            "ap-northeast-3" => "Asia Pacific (Osaka)".to_string(),
            "ca-central-1" => "Canada (Central)".to_string(),
            "eu-central-1" => "Europe (Frankfurt)".to_string(),
            "eu-west-1" => "Europe (Ireland)".to_string(),
            "eu-west-2" => "Europe (London)".to_string(),
            "eu-west-3" => "Europe (Paris)".to_string(),
            "eu-north-1" => "Europe (Stockholm)".to_string(),
            "eu-south-1" => "Europe (Milan)".to_string(),
            "me-south-1" => "Middle East (Bahrain)".to_string(),
            "sa-east-1" => "South America (São Paulo)".to_string(),
            "us-gov-east-1" => "AWS GovCloud (US-East)".to_string(),
            "us-gov-west-1" => "AWS GovCloud (US-West)".to_string(),
            _ => region_code.to_string(),
        }
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

        // Word boundary matching (for multi-word display names)
        if target_lower.split_whitespace().any(|word| word.starts_with(&query_lower)) {
            return 0.8;
        }

        // Subsequence matching (for partial region codes)
        if query_lower.len() >= 2 && Self::is_subsequence(&query_lower, &target_lower) {
            return 0.5;
        }

        0.0
    }

    /// Check if query is a subsequence of target
    fn is_subsequence(query: &str, target: &str) -> bool {
        let mut target_chars = target.chars();
        query.chars().all(|qc| target_chars.any(|tc| tc == qc))
    }

    /// Search regions from static AWS_REGIONS list
    fn search_static_regions(&self, query: &str) -> Vec<RegionSearchResult> {
        let mut results = Vec::new();
        let default_regions = get_default_regions();

        for &region_code in AWS_REGIONS {
            let display_name = Self::format_region_display_name(region_code);
            let mut matched_fields = Vec::new();
            let mut max_score = 0.0f64;

            // Match against region code
            let code_score = Self::fuzzy_match(query, region_code);
            if code_score > 0.0 {
                matched_fields.push("region_code".to_string());
                max_score = max_score.max(code_score);
            }

            // Match against display name
            let name_score = Self::fuzzy_match(query, &display_name);
            if name_score > 0.0 {
                matched_fields.push("display_name".to_string());
                max_score = max_score.max(name_score);
            }

            // Include regions with any match
            if max_score > 0.0 {
                results.push(RegionSearchResult {
                    region_code: region_code.to_string(),
                    display_name,
                    match_score: max_score,
                    matched_fields,
                    source: "AWS Static List".to_string(),
                    is_default: default_regions.contains(&region_code.to_string()),
                });
            }
        }

        results
    }

    /// Filter to show only default regions if requested
    fn filter_default_only(&self, results: Vec<RegionSearchResult>) -> Vec<RegionSearchResult> {
        results.into_iter().filter(|r| r.is_default).collect()
    }
}

impl Default for AwsFindRegionTool {
    fn default() -> Self {
        Self::new_uninitialized()
    }
}

#[async_trait]
impl Tool for AwsFindRegionTool {
    fn name(&self) -> &str {
        "aws_find_region"
    }

    fn description(&self) -> &str {
        r#"Search for AWS regions using fuzzy matching on region codes and display names.

This tool searches through AWS regions without making API calls, using data from:
- Static AWS region list (32 regions including GovCloud)
- Default common regions (9 most popular regions)

Search supports:
- Exact region code matching: "us-east-1", "eu-west-1"
- Partial region code matching: "us-east", "eu-", "east-1"
- Display name fuzzy matching: "Virginia", "Ohio", "Frankfurt", "Ireland"
- Geographic area matching: "Europe", "Asia", "US", "Gov"
- Case-insensitive matching

Examples:
- Find US regions: {"query": "us"}
- Find European regions: {"query": "europe"}
- Find Virginia region: {"query": "virginia"}
- Find by partial code: {"query": "east-1"}
- List only common regions: {"query": "", "defaults_only": true}
- List all regions: {"query": ""}
- Limit results: {"query": "us", "limit": 5}"#
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for region code or display name. Empty string returns all regions.",
                    "examples": ["us-east-1", "virginia", "europe", "us", "east-1", ""]
                },
                "defaults_only": {
                    "type": "boolean",
                    "description": "Return only the 9 default/common regions (default: false)",
                    "default": false
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of results to return (default: 20)",
                    "default": 20,
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
        info!("🔍 aws_find_region executing with parameters: {:?}", parameters);

        // Parse parameters
        let params = parameters.unwrap_or_else(|| serde_json::json!({}));
        
        let query = params.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        
        let defaults_only = params.get("defaults_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let limit = params.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .min(100) as usize;

        info!("🔍 Searching for regions matching: '{}' (defaults_only: {})", query, defaults_only);

        // Search from static AWS regions
        let mut all_results = self.search_static_regions(query);
        info!("📊 Found {} matches from static AWS regions", all_results.len());

        // Filter to defaults only if requested
        if defaults_only {
            all_results = self.filter_default_only(all_results);
        }

        // Sort by match score (highest first), then by default status
        all_results.sort_by(|a, b| {
            b.match_score.partial_cmp(&a.match_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.is_default.cmp(&a.is_default))
        });

        let total_before_limit = all_results.len();

        // Apply limit
        let final_results: Vec<_> = all_results.into_iter().take(limit).collect();

        let duration = start_time.elapsed();

        let execution_summary = if query.is_empty() {
            format!(
                "Listed {} AWS regions{} (showing {} of {}) in {:.2}s",
                final_results.len(),
                if defaults_only { " (defaults only)" } else { "" },
                final_results.len(),
                total_before_limit,
                duration.as_secs_f64()
            )
        } else {
            format!(
                "Found {} AWS regions matching '{}'{} (showing {} of {}) in {:.2}s",
                total_before_limit,
                query,
                if defaults_only { " (defaults only)" } else { "" },
                final_results.len(),
                total_before_limit,
                duration.as_secs_f64()
            )
        };

        info!("📊 aws_find_region completed: {}", execution_summary);

        // Create response JSON
        let response_data = serde_json::json!({
            "regions": final_results,
            "total_matches": total_before_limit,
            "showing_count": final_results.len(),
            "query": query,
            "defaults_only": defaults_only,
            "limit": limit,
            "execution_summary": execution_summary,
            "duration_seconds": duration.as_secs_f64()
        });

        Ok(ToolResult::success(response_data))
    }
}