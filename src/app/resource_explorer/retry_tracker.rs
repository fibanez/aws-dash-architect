//! Global retry state tracking for AWS SDK operations.
//!
//! This module provides visibility into AWS SDK retry behavior without implementing
//! additional application-level retry logic. The SDK handles retries internally;
//! this module tracks transient errors for user feedback in the status bar.

use super::sdk_errors::ErrorCategory;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

/// Global retry tracker instance
static RETRY_TRACKER: Lazy<RetryTracker> = Lazy::new(RetryTracker::new);

/// Get the global retry tracker
pub fn retry_tracker() -> &'static RetryTracker {
    &RETRY_TRACKER
}

/// State of a single query's retry attempts
#[derive(Debug, Clone)]
pub struct QueryRetryState {
    /// Query identifier (e.g., "638876637120:us-east-1:AWS::Lambda::Function")
    pub query_key: String,
    /// Number of transient errors encountered
    pub transient_errors: u32,
    /// Most recent error category
    pub last_error: Option<ErrorCategory>,
    /// When the first error occurred
    pub first_error_time: Instant,
    /// When the most recent error occurred
    pub last_error_time: Instant,
    /// Whether this query has completed (successfully or with final failure)
    pub completed: bool,
    /// Whether the query ultimately succeeded after retries
    pub succeeded: bool,
}

impl QueryRetryState {
    fn new(query_key: &str, error: ErrorCategory) -> Self {
        let now = Instant::now();
        Self {
            query_key: query_key.to_string(),
            transient_errors: 1,
            last_error: Some(error),
            first_error_time: now,
            last_error_time: now,
            completed: false,
            succeeded: false,
        }
    }
}

/// Summary statistics for status bar display
#[derive(Debug, Clone, Default)]
pub struct QueryRetrySummary {
    /// Number of queries currently experiencing transient errors
    pub active_retries: u32,
    /// Number of queries throttled this session
    pub throttled_count: u32,
    /// Number of queries with timeout errors
    pub timeout_count: u32,
    /// Number of queries with network errors
    pub network_error_count: u32,
    /// Number of queries with service unavailable errors
    pub service_unavailable_count: u32,
    /// Total transient errors across all queries
    pub total_transient_errors: u32,
    /// Queries that recovered after transient errors
    pub recovered_count: u32,
    /// Queries currently retrying (with their states)
    pub active_retry_queries: Vec<QueryRetryState>,
}

/// Session-wide statistics
#[derive(Debug, Default)]
struct SessionStats {
    /// Total queries that had throttling errors
    throttled_queries: u32,
    /// Total queries that had timeout errors
    timeout_queries: u32,
    /// Total queries that had network errors
    network_error_queries: u32,
    /// Total queries that had service unavailable errors
    service_unavailable_queries: u32,
    /// Queries that recovered after transient errors
    recovered_queries: u32,
}

/// Global retry state tracker
pub struct RetryTracker {
    /// Per-query retry state
    queries: RwLock<HashMap<String, QueryRetryState>>,
    /// Session-wide statistics
    stats: RwLock<SessionStats>,
}

impl RetryTracker {
    fn new() -> Self {
        Self {
            queries: RwLock::new(HashMap::new()),
            stats: RwLock::new(SessionStats::default()),
        }
    }

    /// Record a transient error for a query
    ///
    /// Call this when a retryable error is detected. The SDK will handle
    /// the actual retry; this just tracks state for user visibility.
    pub fn record_transient_error(&self, query_key: &str, error: ErrorCategory) {
        if !error.is_retryable() {
            return;
        }

        // Update session stats based on error type
        if let Ok(mut stats) = self.stats.write() {
            match &error {
                ErrorCategory::Throttled { .. } => stats.throttled_queries += 1,
                ErrorCategory::Timeout { .. } => stats.timeout_queries += 1,
                ErrorCategory::NetworkError { .. } => stats.network_error_queries += 1,
                ErrorCategory::ServiceUnavailable { .. } => stats.service_unavailable_queries += 1,
                ErrorCategory::NonRetryable { .. } => {} // Should not happen, filtered above
            }
        }

        // Update query state
        if let Ok(mut queries) = self.queries.write() {
            if let Some(state) = queries.get_mut(query_key) {
                state.transient_errors += 1;
                state.last_error = Some(error);
                state.last_error_time = Instant::now();
            } else {
                queries.insert(query_key.to_string(), QueryRetryState::new(query_key, error));
            }
        }

        // Log to query timing
        super::query_timing::log_retry_event(query_key, &self.get_query_state(query_key));
    }

    /// Record successful completion of a query
    ///
    /// Call this when a query completes successfully. If it had transient errors,
    /// it will be marked as recovered.
    pub fn record_success(&self, query_key: &str) {
        if let Ok(mut queries) = self.queries.write() {
            if let Some(state) = queries.get_mut(query_key) {
                if state.transient_errors > 0 {
                    // Query had errors but recovered
                    if let Ok(mut stats) = self.stats.write() {
                        stats.recovered_queries += 1;
                    }
                }
                state.completed = true;
                state.succeeded = true;
            }
        }
    }

    /// Record final failure of a query
    ///
    /// Call this when a query fails definitively (all SDK retries exhausted
    /// or non-retryable error).
    pub fn record_failure(&self, query_key: &str, error: ErrorCategory) {
        if let Ok(mut queries) = self.queries.write() {
            if let Some(state) = queries.get_mut(query_key) {
                state.completed = true;
                state.succeeded = false;
                state.last_error = Some(error);
            } else {
                // First time seeing this query and it failed
                let mut state = QueryRetryState::new(query_key, error);
                state.completed = true;
                state.succeeded = false;
                queries.insert(query_key.to_string(), state);
            }
        }
    }

    /// Get the current retry state for a specific query
    pub fn get_query_state(&self, query_key: &str) -> Option<QueryRetryState> {
        self.queries
            .read()
            .ok()
            .and_then(|q| q.get(query_key).cloned())
    }

    /// Get summary statistics for status bar display
    pub fn get_summary(&self) -> QueryRetrySummary {
        let mut summary = QueryRetrySummary::default();

        if let Ok(queries) = self.queries.read() {
            for state in queries.values() {
                if !state.completed {
                    // Active retry
                    summary.active_retries += 1;
                    summary.total_transient_errors += state.transient_errors;
                    summary.active_retry_queries.push(state.clone());

                    // Count by error type
                    if let Some(ref error) = state.last_error {
                        match error {
                            ErrorCategory::Throttled { .. } => summary.throttled_count += 1,
                            ErrorCategory::Timeout { .. } => summary.timeout_count += 1,
                            ErrorCategory::NetworkError { .. } => summary.network_error_count += 1,
                            ErrorCategory::ServiceUnavailable { .. } => {
                                summary.service_unavailable_count += 1
                            }
                            ErrorCategory::NonRetryable { .. } => {}
                        }
                    }
                }
            }
        }

        if let Ok(stats) = self.stats.read() {
            summary.recovered_count = stats.recovered_queries;
        }

        // Sort active queries by error time (most recent first)
        summary
            .active_retry_queries
            .sort_by(|a, b| b.last_error_time.cmp(&a.last_error_time));

        summary
    }

    /// Get queries that are currently retrying
    pub fn get_active_retries(&self) -> Vec<QueryRetryState> {
        self.queries
            .read()
            .ok()
            .map(|q| q.values().filter(|s| !s.completed).cloned().collect())
            .unwrap_or_default()
    }

    /// Clear tracking state for a new query session
    ///
    /// Call this at the start of a new query phase to reset per-query tracking
    /// while preserving session statistics.
    pub fn clear_query_state(&self) {
        if let Ok(mut queries) = self.queries.write() {
            queries.clear();
        }
    }

    /// Clear all tracking state including session statistics
    ///
    /// Call this when the user logs out or starts a completely new session.
    pub fn reset(&self) {
        if let Ok(mut queries) = self.queries.write() {
            queries.clear();
        }
        if let Ok(mut stats) = self.stats.write() {
            *stats = SessionStats::default();
        }
    }

    /// Check if there are any active transient errors
    pub fn has_active_errors(&self) -> bool {
        self.queries
            .read()
            .ok()
            .is_some_and(|q| q.values().any(|s| !s.completed && s.transient_errors > 0))
    }

    /// Get count of queries with recent throttling (for status bar)
    pub fn get_throttle_count(&self) -> u32 {
        self.get_summary().throttled_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_throttle_error() -> ErrorCategory {
        ErrorCategory::Throttled {
            service: "Lambda".to_string(),
            error_code: "ThrottlingException".to_string(),
            retry_after: None,
        }
    }

    fn make_timeout_error() -> ErrorCategory {
        ErrorCategory::Timeout {
            operation: "ListFunctions".to_string(),
            duration: None,
        }
    }

    #[test]
    fn test_record_transient_error() {
        let tracker = RetryTracker::new();

        tracker.record_transient_error("test:us-east-1:Lambda", make_throttle_error());

        let state = tracker.get_query_state("test:us-east-1:Lambda");
        assert!(state.is_some());

        let state = state.unwrap();
        assert_eq!(state.transient_errors, 1);
        assert!(!state.completed);
    }

    #[test]
    fn test_record_multiple_errors() {
        let tracker = RetryTracker::new();

        tracker.record_transient_error("test:us-east-1:Lambda", make_throttle_error());
        tracker.record_transient_error("test:us-east-1:Lambda", make_timeout_error());

        let state = tracker.get_query_state("test:us-east-1:Lambda").unwrap();
        assert_eq!(state.transient_errors, 2);
    }

    #[test]
    fn test_record_success_with_recovery() {
        let tracker = RetryTracker::new();

        tracker.record_transient_error("test:us-east-1:Lambda", make_throttle_error());
        tracker.record_success("test:us-east-1:Lambda");

        let state = tracker.get_query_state("test:us-east-1:Lambda").unwrap();
        assert!(state.completed);
        assert!(state.succeeded);

        let summary = tracker.get_summary();
        assert_eq!(summary.recovered_count, 1);
    }

    #[test]
    fn test_summary() {
        let tracker = RetryTracker::new();

        // Add some active errors
        tracker.record_transient_error("test:us-east-1:Lambda", make_throttle_error());
        tracker.record_transient_error("test:us-west-2:S3", make_timeout_error());

        let summary = tracker.get_summary();
        assert_eq!(summary.active_retries, 2);
        assert_eq!(summary.throttled_count, 1);
        assert_eq!(summary.timeout_count, 1);
    }

    #[test]
    fn test_non_retryable_not_tracked() {
        let tracker = RetryTracker::new();

        let error = ErrorCategory::NonRetryable {
            code: "AccessDenied".to_string(),
            message: "Access denied".to_string(),
            is_permission_error: true,
        };

        tracker.record_transient_error("test:us-east-1:Lambda", error);

        // Non-retryable errors should not be tracked as transient
        let state = tracker.get_query_state("test:us-east-1:Lambda");
        assert!(state.is_none());
    }

    #[test]
    fn test_clear_query_state() {
        let tracker = RetryTracker::new();

        tracker.record_transient_error("test:us-east-1:Lambda", make_throttle_error());
        tracker.clear_query_state();

        assert!(tracker.get_query_state("test:us-east-1:Lambda").is_none());
    }
}
