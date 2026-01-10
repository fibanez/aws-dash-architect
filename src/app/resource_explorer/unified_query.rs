//! Unified query types for sharing between AWS Explorer and Agent Framework
//!
//! This module provides common data structures for resource querying that are
//! used by both the UI-based AWS Explorer and the V8-based Agent Framework,
//! enabling a single caching mechanism and consistent API.

use serde::{Deserialize, Serialize};

use super::bookmarks::Bookmark;
use super::state::{ResourceEntry, ResourceTag};

// ============================================================================
// Detail Level
// ============================================================================

/// Detail level for resource queries - controls how much data is returned
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    /// Just totals, no resource data - useful for "how many instances do I have?"
    Count,
    /// Minimal fields: id, name, type, account, region, status (DEFAULT)
    #[default]
    Summary,
    /// Summary fields plus tags
    Tags,
    /// Complete JSON properties including raw and detailed
    Full,
}

impl DetailLevel {
    /// Parse from string (case-insensitive)
    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s.map(|s| s.to_lowercase()).as_deref() {
            Some("count") => DetailLevel::Count,
            Some("summary") => DetailLevel::Summary,
            Some("tags") => DetailLevel::Tags,
            Some("full") => DetailLevel::Full,
            _ => DetailLevel::default(),
        }
    }
}

// ============================================================================
// Query Result Status
// ============================================================================

/// Query result status - indicates success, partial success, or failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryResultStatus {
    /// All queries succeeded
    Success,
    /// Some queries succeeded, some failed (multi-account/region scenarios)
    Partial,
    /// All queries failed
    Error,
}

// ============================================================================
// Warnings and Errors
// ============================================================================

/// Warning for a specific account/region combination
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryWarning {
    pub account: String,
    pub region: String,
    pub message: String,
}

/// Error for a specific account/region combination
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryError {
    pub account: String,
    pub region: String,
    pub code: String,
    pub message: String,
}

// ============================================================================
// Unified Query Result
// ============================================================================

/// Unified query result with status, warnings, and errors
///
/// This structure provides clear distinction between:
/// - Success with data
/// - Partial success (some accounts/regions failed)
/// - Complete failure
/// - Empty results (success with count=0)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedQueryResult<T> {
    /// Overall status of the query
    pub status: QueryResultStatus,
    /// Query results (empty for Count detail level or on error)
    pub data: T,
    /// Total count of resources found
    pub count: usize,
    /// Non-fatal warnings (e.g., rate limiting, partial data)
    pub warnings: Vec<QueryWarning>,
    /// Errors that occurred during query (per account/region)
    pub errors: Vec<QueryError>,
    /// True if Phase 2 enrichment has completed and detailed_properties are available
    #[serde(default)]
    pub details_loaded: bool,
    /// True if Phase 2 enrichment is currently in progress
    #[serde(default)]
    pub details_pending: bool,
}

impl<T: Default> UnifiedQueryResult<T> {
    /// Create a successful result with data
    pub fn success(data: T, count: usize) -> Self {
        Self {
            status: QueryResultStatus::Success,
            data,
            count,
            warnings: Vec::new(),
            errors: Vec::new(),
            details_loaded: false,
            details_pending: false,
        }
    }

    /// Create a successful empty result (no resources found, but query succeeded)
    pub fn empty() -> Self {
        Self {
            status: QueryResultStatus::Success,
            data: T::default(),
            count: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
            details_loaded: false,
            details_pending: false,
        }
    }

    /// Create an error result
    pub fn error(errors: Vec<QueryError>) -> Self {
        Self {
            status: QueryResultStatus::Error,
            data: T::default(),
            count: 0,
            warnings: Vec::new(),
            errors,
            details_loaded: false,
            details_pending: false,
        }
    }

    /// Create a partial success result (some data, some errors)
    pub fn partial(
        data: T,
        count: usize,
        warnings: Vec<QueryWarning>,
        errors: Vec<QueryError>,
    ) -> Self {
        Self {
            status: QueryResultStatus::Partial,
            data,
            count,
            warnings,
            errors,
            details_loaded: false,
            details_pending: false,
        }
    }

    /// Determine status based on success/error counts
    pub fn from_results(
        data: T,
        count: usize,
        success_count: usize,
        error_count: usize,
        warnings: Vec<QueryWarning>,
        errors: Vec<QueryError>,
    ) -> Self {
        let status = if error_count == 0 {
            QueryResultStatus::Success
        } else if success_count == 0 {
            QueryResultStatus::Error
        } else {
            QueryResultStatus::Partial
        };

        Self {
            status,
            data,
            count,
            warnings,
            errors,
            details_loaded: false,
            details_pending: false,
        }
    }

    /// Determine status based on success/error counts with Phase 2 status
    pub fn from_results_with_phase2_status(
        data: T,
        count: usize,
        success_count: usize,
        error_count: usize,
        warnings: Vec<QueryWarning>,
        errors: Vec<QueryError>,
        details_loaded: bool,
        details_pending: bool,
    ) -> Self {
        let status = if error_count == 0 {
            QueryResultStatus::Success
        } else if success_count == 0 {
            QueryResultStatus::Error
        } else {
            QueryResultStatus::Partial
        };

        Self {
            status,
            data,
            count,
            warnings,
            errors,
            details_loaded,
            details_pending,
        }
    }
}

// ============================================================================
// Resource Representations at Different Detail Levels
// ============================================================================

/// Summary resource - minimal fields for list views and counts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSummary {
    pub resource_id: String,
    pub display_name: String,
    pub resource_type: String,
    pub account_id: String,
    pub region: String,
    pub status: Option<String>,
}

impl From<&ResourceEntry> for ResourceSummary {
    fn from(entry: &ResourceEntry) -> Self {
        Self {
            resource_id: entry.resource_id.clone(),
            display_name: entry.display_name.clone(),
            resource_type: entry.resource_type.clone(),
            account_id: entry.account_id.clone(),
            region: entry.region.clone(),
            status: entry.status.clone(),
        }
    }
}

/// Resource with tags - summary fields plus tags
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceWithTags {
    pub resource_id: String,
    pub display_name: String,
    pub resource_type: String,
    pub account_id: String,
    pub region: String,
    pub status: Option<String>,
    pub tags: Vec<ResourceTag>,
}

impl From<&ResourceEntry> for ResourceWithTags {
    fn from(entry: &ResourceEntry) -> Self {
        Self {
            resource_id: entry.resource_id.clone(),
            display_name: entry.display_name.clone(),
            resource_type: entry.resource_type.clone(),
            account_id: entry.account_id.clone(),
            region: entry.region.clone(),
            status: entry.status.clone(),
            tags: entry.tags.clone(),
        }
    }
}

/// Full resource - complete data including all properties
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFull {
    pub resource_id: String,
    pub display_name: String,
    pub resource_type: String,
    pub account_id: String,
    pub region: String,
    pub status: Option<String>,
    pub tags: Vec<ResourceTag>,
    pub properties: serde_json::Value,
    pub detailed_properties: Option<serde_json::Value>,
}

impl From<&ResourceEntry> for ResourceFull {
    fn from(entry: &ResourceEntry) -> Self {
        Self {
            resource_id: entry.resource_id.clone(),
            display_name: entry.display_name.clone(),
            resource_type: entry.resource_type.clone(),
            account_id: entry.account_id.clone(),
            region: entry.region.clone(),
            status: entry.status.clone(),
            tags: entry.tags.clone(),
            properties: entry.properties.clone(),
            // detailed_properties now merged into properties, expose as Some if enriched
            detailed_properties: entry.detailed_timestamp.map(|_| entry.properties.clone()),
        }
    }
}

// ============================================================================
// Bookmark Information for V8
// ============================================================================

/// Bookmark info exposed to V8 - flat list without folder hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub account_ids: Vec<String>,
    pub region_codes: Vec<String>,
    pub resource_types: Vec<String>,
    pub has_tag_filters: bool,
    pub has_search_filter: bool,
    pub access_count: usize,
    pub last_accessed: Option<String>,
}

impl From<&Bookmark> for BookmarkInfo {
    fn from(bookmark: &Bookmark) -> Self {
        Self {
            id: bookmark.id.clone(),
            name: bookmark.name.clone(),
            description: bookmark.description.clone(),
            account_ids: bookmark.account_ids.clone(),
            region_codes: bookmark.region_codes.clone(),
            resource_types: bookmark.resource_type_ids.clone(),
            has_tag_filters: !bookmark.tag_filters.filters.is_empty()
                || !bookmark.tag_filters.sub_groups.is_empty(),
            has_search_filter: !bookmark.search_filter.is_empty(),
            access_count: bookmark.access_count,
            last_accessed: bookmark.last_accessed.map(|dt| dt.to_rfc3339()),
        }
    }
}

// ============================================================================
// Conversion Utilities
// ============================================================================

/// Convert resources to the appropriate detail level representation
pub enum DetailedResources {
    Count(usize),
    Summary(Vec<ResourceSummary>),
    Tags(Vec<ResourceWithTags>),
    Full(Vec<ResourceFull>),
}

impl DetailedResources {
    /// Convert a slice of ResourceEntry to the specified detail level
    pub fn from_entries(entries: &[ResourceEntry], level: DetailLevel) -> Self {
        match level {
            DetailLevel::Count => DetailedResources::Count(entries.len()),
            DetailLevel::Summary => {
                DetailedResources::Summary(entries.iter().map(ResourceSummary::from).collect())
            }
            DetailLevel::Tags => {
                DetailedResources::Tags(entries.iter().map(ResourceWithTags::from).collect())
            }
            DetailLevel::Full => {
                DetailedResources::Full(entries.iter().map(ResourceFull::from).collect())
            }
        }
    }

    /// Get the count regardless of detail level
    pub fn count(&self) -> usize {
        match self {
            DetailedResources::Count(n) => *n,
            DetailedResources::Summary(v) => v.len(),
            DetailedResources::Tags(v) => v.len(),
            DetailedResources::Full(v) => v.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_entry() -> ResourceEntry {
        ResourceEntry {
            resource_type: "AWS::EC2::Instance".to_string(),
            account_id: "123456789012".to_string(),
            region: "us-east-1".to_string(),
            resource_id: "i-1234567890abcdef0".to_string(),
            display_name: "test-instance".to_string(),
            status: Some("running".to_string()),
            properties: serde_json::json!({"InstanceId": "i-1234567890abcdef0", "instanceType": "t2.micro"}),
            detailed_properties: None,
            detailed_timestamp: None,
            tags: vec![
                ResourceTag {
                    key: "Name".to_string(),
                    value: "test-instance".to_string(),
                },
                ResourceTag {
                    key: "Environment".to_string(),
                    value: "dev".to_string(),
                },
            ],
            relationships: vec![],
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: egui::Color32::WHITE,
            region_color: egui::Color32::WHITE,
            query_timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_detail_level_parsing() {
        assert_eq!(DetailLevel::from_str_opt(Some("count")), DetailLevel::Count);
        assert_eq!(DetailLevel::from_str_opt(Some("COUNT")), DetailLevel::Count);
        assert_eq!(
            DetailLevel::from_str_opt(Some("summary")),
            DetailLevel::Summary
        );
        assert_eq!(DetailLevel::from_str_opt(Some("tags")), DetailLevel::Tags);
        assert_eq!(DetailLevel::from_str_opt(Some("full")), DetailLevel::Full);
        assert_eq!(DetailLevel::from_str_opt(None), DetailLevel::Summary);
        assert_eq!(
            DetailLevel::from_str_opt(Some("invalid")),
            DetailLevel::Summary
        );
    }

    #[test]
    fn test_resource_summary_conversion() {
        let entry = create_test_entry();
        let summary = ResourceSummary::from(&entry);

        assert_eq!(summary.resource_id, "i-1234567890abcdef0");
        assert_eq!(summary.display_name, "test-instance");
        assert_eq!(summary.resource_type, "AWS::EC2::Instance");
        assert_eq!(summary.account_id, "123456789012");
        assert_eq!(summary.region, "us-east-1");
        assert_eq!(summary.status, Some("running".to_string()));
    }

    #[test]
    fn test_resource_with_tags_conversion() {
        let entry = create_test_entry();
        let with_tags = ResourceWithTags::from(&entry);

        assert_eq!(with_tags.resource_id, "i-1234567890abcdef0");
        assert_eq!(with_tags.tags.len(), 2);
        assert_eq!(with_tags.tags[0].key, "Name");
    }

    #[test]
    fn test_unified_result_success() {
        let result: UnifiedQueryResult<Vec<String>> =
            UnifiedQueryResult::success(vec!["a".to_string(), "b".to_string()], 2);

        assert_eq!(result.status, QueryResultStatus::Success);
        assert_eq!(result.count, 2);
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_unified_result_empty() {
        let result: UnifiedQueryResult<Vec<String>> = UnifiedQueryResult::empty();

        assert_eq!(result.status, QueryResultStatus::Success);
        assert_eq!(result.count, 0);
        assert!(result.data.is_empty());
    }

    #[test]
    fn test_unified_result_from_results() {
        // All success
        let result: UnifiedQueryResult<Vec<String>> =
            UnifiedQueryResult::from_results(vec!["a".to_string()], 1, 3, 0, vec![], vec![]);
        assert_eq!(result.status, QueryResultStatus::Success);

        // Partial
        let result: UnifiedQueryResult<Vec<String>> = UnifiedQueryResult::from_results(
            vec!["a".to_string()],
            1,
            2,
            1,
            vec![],
            vec![QueryError {
                account: "123".to_string(),
                region: "us-east-1".to_string(),
                code: "AccessDenied".to_string(),
                message: "No permission".to_string(),
            }],
        );
        assert_eq!(result.status, QueryResultStatus::Partial);

        // All error
        let result: UnifiedQueryResult<Vec<String>> =
            UnifiedQueryResult::from_results(vec![], 0, 0, 2, vec![], vec![]);
        assert_eq!(result.status, QueryResultStatus::Error);
    }

    #[test]
    fn test_detailed_resources_conversion() {
        let entries = vec![create_test_entry()];

        let count = DetailedResources::from_entries(&entries, DetailLevel::Count);
        assert_eq!(count.count(), 1);

        let summary = DetailedResources::from_entries(&entries, DetailLevel::Summary);
        assert_eq!(summary.count(), 1);

        let tags = DetailedResources::from_entries(&entries, DetailLevel::Tags);
        assert_eq!(tags.count(), 1);

        let full = DetailedResources::from_entries(&entries, DetailLevel::Full);
        assert_eq!(full.count(), 1);
    }
}
