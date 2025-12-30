//! Verification result types and file generation.
//!
//! This module provides types for storing verification results and functions
//! for writing results to files that can be used by coding agents for debugging.
//!
//! CRITICAL: This performs FIELD-BY-FIELD comparison of Dash cache vs CLI output.

#![cfg(debug_assertions)]

use super::cli_commands::{CliExecution, ComparisonType, get_field_mappings, get_json_value};
use super::state::ResourceEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tracing::info;

// ============================================================================
// Field-Level Comparison Results
// ============================================================================

/// Result of comparing a single field between Dash and CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldComparison {
    /// Field name
    pub field_name: String,
    /// Value from Dash cache (as string for display)
    pub dash_value: Option<String>,
    /// Value from CLI response (as string for display)
    pub cli_value: Option<String>,
    /// Whether the values matched
    pub matched: bool,
    /// How the comparison was performed
    pub comparison_type: ComparisonType,
    /// Was this field skipped (e.g., Ignore type)
    pub skipped: bool,
}

/// Result of comparing a single resource between Dash and CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceComparison {
    /// Resource ID
    pub resource_id: String,
    /// Whether the resource was found in Dash cache
    pub found_in_dash: bool,
    /// Whether the resource was found in CLI output
    pub found_in_cli: bool,
    /// Field-by-field comparison results
    pub field_comparisons: Vec<FieldComparison>,
    /// Count of matched fields
    pub matched_count: usize,
    /// Count of mismatched fields
    pub mismatched_count: usize,
    /// Count of skipped fields (Ignore type)
    pub skipped_count: usize,
}

impl ResourceComparison {
    /// Check if all compared fields matched
    pub fn all_fields_match(&self) -> bool {
        self.mismatched_count == 0 && self.found_in_dash && self.found_in_cli
    }
}

// ============================================================================
// Resource Type Results
// ============================================================================

/// Result of verifying a single resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTypeResult {
    pub resource_type: String,
    pub dash_count: usize,
    pub cli_count: usize,
    /// Overall match status (all resources and fields match)
    pub matched: bool,
    /// Resources in CLI but not in Dash
    pub missing_in_dash: Vec<String>,
    /// Resources in Dash but not in CLI
    pub missing_in_cli: Vec<String>,
    /// Per-resource field comparisons
    pub resource_comparisons: Vec<ResourceComparison>,
    /// CLI execution details
    pub cli_execution: Option<CliExecution>,
    /// Error if verification failed
    pub error: Option<String>,
    // Summary stats
    pub total_fields_compared: usize,
    pub total_fields_matched: usize,
    pub total_fields_mismatched: usize,
}

impl ResourceTypeResult {
    /// Calculate match percentage
    pub fn match_percentage(&self) -> f64 {
        if self.total_fields_compared == 0 {
            return 100.0;
        }
        (self.total_fields_matched as f64 / self.total_fields_compared as f64) * 100.0
    }
}

// ============================================================================
// Complete Verification Results
// ============================================================================

/// Complete verification results for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResults {
    pub timestamp: DateTime<Utc>,
    pub account_id: String,
    pub region: String,
    pub results: Vec<ResourceTypeResult>,
    /// All CLI executions for raw output
    pub cli_executions: Vec<CliExecution>,
}

impl VerificationResults {
    /// Create a new empty verification results container.
    pub fn new(account_id: String, region: String) -> Self {
        Self {
            timestamp: Utc::now(),
            account_id,
            region,
            results: Vec::new(),
            cli_executions: Vec::new(),
        }
    }

    /// Add a result for a resource type.
    pub fn add_result(&mut self, result: ResourceTypeResult) {
        if let Some(ref exec) = result.cli_execution {
            self.cli_executions.push(exec.clone());
        }
        self.results.push(result);
    }

    /// Check if all verifications passed.
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.matched && r.error.is_none())
    }

    /// Get count of passed verifications.
    pub fn passed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.matched && r.error.is_none())
            .count()
    }

    /// Get count of failed verifications.
    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !r.matched || r.error.is_some())
            .count()
    }

    /// Get total fields compared across all resources
    pub fn total_fields_compared(&self) -> usize {
        self.results.iter().map(|r| r.total_fields_compared).sum()
    }

    /// Get total fields matched
    pub fn total_fields_matched(&self) -> usize {
        self.results.iter().map(|r| r.total_fields_matched).sum()
    }

    /// Get the output directory path.
    fn get_output_dir() -> PathBuf {
        PathBuf::from("target/verification")
    }

    /// Ensure the output directory exists.
    fn ensure_output_dir() -> std::io::Result<PathBuf> {
        let dir = Self::get_output_dir();
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Write results to files.
    pub fn write_to_files(&self) -> std::io::Result<(PathBuf, PathBuf)> {
        let dir = Self::ensure_output_dir()?;
        let summary_path = dir.join("verification_summary.txt");
        let details_path = dir.join("verification_details.txt");
        let raw_path = dir.join("verification_cli_raw.json");

        // Write summary
        self.write_summary(&summary_path)?;

        // Write details
        self.write_details(&details_path)?;

        // Write raw CLI responses
        self.write_raw_responses(&raw_path)?;

        info!(
            "Verification results written to {:?}",
            dir
        );

        Ok((summary_path, details_path))
    }

    /// Write concise summary file.
    fn write_summary(&self, path: &PathBuf) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        writeln!(file, "# AWS Dash Verification Results")?;
        writeln!(file, "Timestamp: {}", self.timestamp.to_rfc3339())?;
        writeln!(file, "Account: {}", self.account_id)?;
        writeln!(file, "Region: {}", self.region)?;
        writeln!(file)?;

        let total_compared = self.total_fields_compared();
        let total_matched = self.total_fields_matched();
        let match_pct = if total_compared > 0 {
            (total_matched as f64 / total_compared as f64) * 100.0
        } else {
            100.0
        };

        writeln!(file, "OVERALL SUMMARY")?;
        writeln!(file, "===============")?;
        writeln!(file, "Resource Types Verified: {}", self.results.len())?;
        writeln!(file, "Total Fields Compared: {}", total_compared)?;
        writeln!(file, "Total Fields Matched: {} ({:.1}%)", total_matched, match_pct)?;
        writeln!(file, "Total Fields Mismatched: {}", total_compared - total_matched)?;
        writeln!(file)?;

        writeln!(file, "RESOURCE TYPE SUMMARY")?;
        writeln!(file, "=====================")?;

        for result in &self.results {
            if let Some(ref err) = result.error {
                writeln!(file, "ERROR: {} - {}", result.resource_type, err)?;
            } else if result.matched {
                writeln!(
                    file,
                    "OK: {} - {} resources, {} fields compared, {:.1}% match",
                    result.resource_type,
                    result.dash_count,
                    result.total_fields_compared,
                    result.match_percentage()
                )?;
            } else {
                writeln!(
                    file,
                    "FAIL: {} - {} resources, {} fields compared, {} mismatches",
                    result.resource_type,
                    result.dash_count,
                    result.total_fields_compared,
                    result.total_fields_mismatched
                )?;
                if !result.missing_in_dash.is_empty() {
                    writeln!(file, "  Missing in Dash: {:?}", result.missing_in_dash)?;
                }
                if !result.missing_in_cli.is_empty() {
                    writeln!(file, "  Missing in CLI: {:?}", result.missing_in_cli)?;
                }
            }
        }

        Ok(())
    }

    /// Write detailed results file.
    fn write_details(&self, path: &PathBuf) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        writeln!(file, "# AWS Dash Verification Details")?;
        writeln!(file, "Timestamp: {}", self.timestamp.to_rfc3339())?;
        writeln!(file, "Account: {}", self.account_id)?;
        writeln!(file, "Region: {}", self.region)?;
        writeln!(file)?;

        for result in &self.results {
            writeln!(file, "===============================================================================")?;
            writeln!(file, "=== {} ===", result.resource_type)?;
            writeln!(file, "===============================================================================")?;

            // CLI execution details
            if let Some(ref exec) = result.cli_execution {
                writeln!(file, "CLI Command: {}", exec.command)?;
                writeln!(file, "Execution Time: {}ms", exec.duration_ms)?;
                writeln!(file, "Response Size: {} bytes", exec.response_size_bytes)?;
                writeln!(file, "Resources Returned: {}", exec.resource_count)?;
                writeln!(file)?;
            }

            if let Some(ref err) = result.error {
                writeln!(file, "ERROR: {}", err)?;
                writeln!(file)?;
                continue;
            }

            writeln!(file, "Dash count: {}", result.dash_count)?;
            writeln!(file, "CLI count: {}", result.cli_count)?;
            writeln!(file, "Fields compared: {}", result.total_fields_compared)?;
            writeln!(file, "Fields matched: {} ({:.1}%)",
                result.total_fields_matched, result.match_percentage())?;
            writeln!(file, "Fields mismatched: {}", result.total_fields_mismatched)?;
            writeln!(file)?;

            // Missing resources
            if !result.missing_in_dash.is_empty() {
                writeln!(file, "MISSING IN DASH (found in CLI but not in Dash cache):")?;
                for id in &result.missing_in_dash {
                    writeln!(file, "  - {}", id)?;
                }
                writeln!(file)?;
            }

            if !result.missing_in_cli.is_empty() {
                writeln!(file, "MISSING IN CLI (found in Dash but not in CLI output):")?;
                for id in &result.missing_in_cli {
                    writeln!(file, "  - {}", id)?;
                }
                writeln!(file)?;
            }

            // Per-resource field comparisons
            for resource in &result.resource_comparisons {
                if !resource.found_in_dash || !resource.found_in_cli {
                    continue; // Skip resources that weren't in both
                }

                writeln!(file, "--- Resource: {} ---", resource.resource_id)?;

                // Show mismatches first
                let mismatches: Vec<_> = resource.field_comparisons.iter()
                    .filter(|f| !f.matched && !f.skipped)
                    .collect();

                if !mismatches.is_empty() {
                    for field in mismatches {
                        writeln!(file, "MISMATCH: {}", field.field_name)?;
                        writeln!(file, "  Dash: {}", field.dash_value.as_deref().unwrap_or("null"))?;
                        writeln!(file, "  CLI:  {}", field.cli_value.as_deref().unwrap_or("null"))?;
                    }
                }

                // Then show matches
                let matches: Vec<_> = resource.field_comparisons.iter()
                    .filter(|f| f.matched && !f.skipped)
                    .collect();

                for field in matches {
                    let value = field.dash_value.as_deref().unwrap_or("null");
                    // Truncate long values for readability
                    let display_value = if value.len() > 80 {
                        format!("{}...", &value[..80])
                    } else {
                        value.to_string()
                    };
                    writeln!(file, "MATCH: {} = {}", field.field_name, display_value)?;
                }

                // Show skipped fields
                let skipped: Vec<_> = resource.field_comparisons.iter()
                    .filter(|f| f.skipped)
                    .collect();

                if !skipped.is_empty() {
                    writeln!(file, "SKIPPED: {} (dynamic fields)",
                        skipped.iter().map(|f| f.field_name.as_str()).collect::<Vec<_>>().join(", "))?;
                }

                writeln!(file, "Summary: {} matched, {} mismatched, {} skipped",
                    resource.matched_count, resource.mismatched_count, resource.skipped_count)?;
                writeln!(file)?;
            }

            writeln!(file)?;
        }

        Ok(())
    }

    /// Write raw CLI responses to JSON file.
    fn write_raw_responses(&self, path: &PathBuf) -> std::io::Result<()> {
        let json = serde_json::json!({
            "timestamp": self.timestamp.to_rfc3339(),
            "account_id": self.account_id,
            "region": self.region,
            "executions": self.cli_executions
        });

        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &json)?;

        Ok(())
    }
}

// ============================================================================
// Comparison Functions
// ============================================================================

/// Compare resources between Dash cache and CLI output with field-level detail.
pub fn compare_resources_detailed(
    resource_type: &str,
    dash_resources: &[ResourceEntry],
    cli_resources_by_id: &HashMap<String, Value>,
    cli_resource_ids: &[String],
    cli_execution: CliExecution,
) -> ResourceTypeResult {
    let field_mappings = get_field_mappings(resource_type);

    // Build set of Dash resource IDs
    let dash_ids: HashSet<String> = dash_resources.iter()
        .map(|r| r.resource_id.clone())
        .collect();
    let cli_ids: HashSet<String> = cli_resource_ids.iter().cloned().collect();

    // Find missing resources
    let missing_in_dash: Vec<String> = cli_ids.difference(&dash_ids)
        .cloned()
        .collect();
    let missing_in_cli: Vec<String> = dash_ids.difference(&cli_ids)
        .cloned()
        .collect();

    // Compare each resource that exists in both
    let mut resource_comparisons = Vec::new();
    let mut total_fields_compared = 0;
    let mut total_fields_matched = 0;
    let mut total_fields_mismatched = 0;

    for dash_resource in dash_resources {
        let resource_id = &dash_resource.resource_id;

        // Check if CLI has this resource
        let cli_resource = cli_resources_by_id.get(resource_id);

        if cli_resource.is_none() {
            // Resource not in CLI - already tracked in missing_in_cli
            resource_comparisons.push(ResourceComparison {
                resource_id: resource_id.clone(),
                found_in_dash: true,
                found_in_cli: false,
                field_comparisons: Vec::new(),
                matched_count: 0,
                mismatched_count: 0,
                skipped_count: 0,
            });
            continue;
        }

        let cli_json = cli_resource.unwrap();

        // Get Dash JSON data - try detailed_properties first, then raw_properties, then properties
        let dash_json = dash_resource.detailed_properties.as_ref()
            .or(Some(&dash_resource.raw_properties))
            .unwrap_or(&dash_resource.properties);

        // Compare fields
        let mut field_comparisons = Vec::new();
        let mut matched_count = 0;
        let mut mismatched_count = 0;
        let mut skipped_count = 0;

        // If we have field mappings, use them
        if !field_mappings.is_empty() {
            for mapping in &field_mappings {
                let comparison = compare_field(
                    dash_json,
                    cli_json,
                    mapping.dash_field,
                    mapping.cli_field,
                    mapping.comparison_type,
                );

                if comparison.skipped {
                    skipped_count += 1;
                } else if comparison.matched {
                    matched_count += 1;
                    total_fields_compared += 1;
                    total_fields_matched += 1;
                } else {
                    mismatched_count += 1;
                    total_fields_compared += 1;
                    total_fields_mismatched += 1;
                }

                field_comparisons.push(comparison);
            }
        } else {
            // No mappings - compare all common top-level fields
            let dash_obj = dash_json.as_object();
            let cli_obj = cli_json.as_object();

            if let (Some(dash_map), Some(cli_map)) = (dash_obj, cli_obj) {
                // Get all keys from both
                let all_keys: HashSet<&String> = dash_map.keys().chain(cli_map.keys()).collect();

                for key in all_keys {
                    let comparison = compare_field(
                        dash_json,
                        cli_json,
                        key,
                        key,
                        ComparisonType::Exact,
                    );

                    if comparison.matched {
                        matched_count += 1;
                        total_fields_compared += 1;
                        total_fields_matched += 1;
                    } else {
                        mismatched_count += 1;
                        total_fields_compared += 1;
                        total_fields_mismatched += 1;
                    }

                    field_comparisons.push(comparison);
                }
            }
        }

        resource_comparisons.push(ResourceComparison {
            resource_id: resource_id.clone(),
            found_in_dash: true,
            found_in_cli: true,
            field_comparisons,
            matched_count,
            mismatched_count,
            skipped_count,
        });
    }

    // Determine overall match status
    let all_resources_present = missing_in_dash.is_empty() && missing_in_cli.is_empty();
    let all_fields_match = total_fields_mismatched == 0;
    let matched = all_resources_present && all_fields_match;

    ResourceTypeResult {
        resource_type: resource_type.to_string(),
        dash_count: dash_resources.len(),
        cli_count: cli_resource_ids.len(),
        matched,
        missing_in_dash,
        missing_in_cli,
        resource_comparisons,
        cli_execution: Some(cli_execution),
        error: None,
        total_fields_compared,
        total_fields_matched,
        total_fields_mismatched,
    }
}

/// Compare a single field between Dash and CLI JSON
fn compare_field(
    dash_json: &Value,
    cli_json: &Value,
    dash_field: &str,
    cli_field: &str,
    comparison_type: ComparisonType,
) -> FieldComparison {
    // Skip ignored fields
    if comparison_type == ComparisonType::Ignore {
        return FieldComparison {
            field_name: dash_field.to_string(),
            dash_value: None,
            cli_value: None,
            matched: true,
            comparison_type,
            skipped: true,
        };
    }

    // Extract values
    let dash_value = get_json_value(dash_json, dash_field);
    let cli_value = get_json_value(cli_json, cli_field);

    // Convert to strings for display and comparison
    let dash_str = value_to_string(&dash_value);
    let cli_str = value_to_string(&cli_value);

    // Compare based on type
    let matched = match comparison_type {
        ComparisonType::Exact => dash_str == cli_str,
        ComparisonType::CaseInsensitive => {
            dash_str.as_ref().map(|s| s.to_lowercase()) == cli_str.as_ref().map(|s| s.to_lowercase())
        }
        ComparisonType::Numeric => {
            compare_numeric(&dash_value, &cli_value)
        }
        ComparisonType::Ignore => true, // Already handled above
    };

    FieldComparison {
        field_name: dash_field.to_string(),
        dash_value: dash_str,
        cli_value: cli_str,
        matched,
        comparison_type,
        skipped: false,
    }
}

/// Convert a JSON value to a string for display
fn value_to_string(value: &Option<Value>) -> Option<String> {
    match value {
        None => None,
        Some(Value::Null) => Some("null".to_string()),
        Some(Value::Bool(b)) => Some(b.to_string()),
        Some(Value::Number(n)) => Some(n.to_string()),
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Array(arr)) => Some(serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string())),
        Some(Value::Object(obj)) => Some(serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string())),
    }
}

/// Compare two values as numbers
fn compare_numeric(dash_value: &Option<Value>, cli_value: &Option<Value>) -> bool {
    let dash_num = dash_value.as_ref().and_then(|v| {
        if let Some(n) = v.as_f64() {
            Some(n)
        } else if let Some(s) = v.as_str() {
            s.parse::<f64>().ok()
        } else {
            None
        }
    });

    let cli_num = cli_value.as_ref().and_then(|v| {
        if let Some(n) = v.as_f64() {
            Some(n)
        } else if let Some(s) = v.as_str() {
            s.parse::<f64>().ok()
        } else {
            None
        }
    });

    match (dash_num, cli_num) {
        (Some(d), Some(c)) => (d - c).abs() < f64::EPSILON,
        (None, None) => true,
        _ => false,
    }
}

/// Legacy function for simple resource comparison (ID only)
/// Kept for backward compatibility
pub fn compare_resources(
    resource_type: &str,
    dash_ids: &[String],
    cli_ids: &[String],
) -> ResourceTypeResult {
    let dash_set: HashSet<_> = dash_ids.iter().collect();
    let cli_set: HashSet<_> = cli_ids.iter().collect();

    let missing_in_dash: Vec<String> = cli_set
        .difference(&dash_set)
        .map(|s| (*s).clone())
        .collect();

    let missing_in_cli: Vec<String> = dash_set
        .difference(&cli_set)
        .map(|s| (*s).clone())
        .collect();

    let matched =
        dash_ids.len() == cli_ids.len() && missing_in_dash.is_empty() && missing_in_cli.is_empty();

    ResourceTypeResult {
        resource_type: resource_type.to_string(),
        dash_count: dash_ids.len(),
        cli_count: cli_ids.len(),
        matched,
        missing_in_dash,
        missing_in_cli,
        resource_comparisons: Vec::new(),
        cli_execution: None,
        error: None,
        total_fields_compared: 0,
        total_fields_matched: 0,
        total_fields_mismatched: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_field_exact_match() {
        let dash = serde_json::json!({"Name": "test-function"});
        let cli = serde_json::json!({"Name": "test-function"});

        let result = compare_field(&dash, &cli, "Name", "Name", ComparisonType::Exact);
        assert!(result.matched);
        assert!(!result.skipped);
    }

    #[test]
    fn test_compare_field_mismatch() {
        let dash = serde_json::json!({"MemorySize": 128});
        let cli = serde_json::json!({"MemorySize": 256});

        let result = compare_field(&dash, &cli, "MemorySize", "MemorySize", ComparisonType::Numeric);
        assert!(!result.matched);
        assert_eq!(result.dash_value, Some("128".to_string()));
        assert_eq!(result.cli_value, Some("256".to_string()));
    }

    #[test]
    fn test_compare_field_ignored() {
        let dash = serde_json::json!({"LastModified": "2024-01-01"});
        let cli = serde_json::json!({"LastModified": "2024-12-01"});

        let result = compare_field(&dash, &cli, "LastModified", "LastModified", ComparisonType::Ignore);
        assert!(result.matched);
        assert!(result.skipped);
    }

    #[test]
    fn test_compare_field_nested_path() {
        let dash = serde_json::json!({"State": "running"});
        let cli = serde_json::json!({"State": {"Name": "running", "Code": 16}});

        let result = compare_field(&dash, &cli, "State", "State.Name", ComparisonType::Exact);
        assert!(result.matched);
    }

    #[test]
    fn test_value_to_string() {
        assert_eq!(value_to_string(&Some(serde_json::json!("test"))), Some("test".to_string()));
        assert_eq!(value_to_string(&Some(serde_json::json!(123))), Some("123".to_string()));
        assert_eq!(value_to_string(&Some(serde_json::json!(true))), Some("true".to_string()));
        assert_eq!(value_to_string(&None), None);
    }

    #[test]
    fn test_resource_type_result_match_percentage() {
        let result = ResourceTypeResult {
            resource_type: "test".to_string(),
            dash_count: 10,
            cli_count: 10,
            matched: true,
            missing_in_dash: Vec::new(),
            missing_in_cli: Vec::new(),
            resource_comparisons: Vec::new(),
            cli_execution: None,
            error: None,
            total_fields_compared: 100,
            total_fields_matched: 95,
            total_fields_mismatched: 5,
        };

        assert!((result.match_percentage() - 95.0).abs() < 0.01);
    }
}
