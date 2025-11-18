//! CloudWatch Logs Data Types
//!
//! Data structures for CloudWatch Logs queries, results, and configuration.

#![warn(clippy::all, rust_2018_idioms)]

use serde::{Deserialize, Serialize};

/// Query options for CloudWatch Logs
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// Start time (Unix timestamp in milliseconds)
    pub start_time: Option<i64>,
    /// End time (Unix timestamp in milliseconds)
    pub end_time: Option<i64>,
    /// Filter pattern (CloudWatch Logs filter syntax)
    pub filter_pattern: Option<String>,
    /// Maximum number of events to return
    pub limit: Option<i32>,
    /// Log stream names to query (empty = all streams)
    pub log_stream_names: Vec<String>,
    /// Whether to query in reverse chronological order (false = most recent first)
    pub start_from_head: bool,
}

impl QueryOptions {
    /// Create new QueryOptions with default values
    pub fn new() -> Self {
        Self {
            start_time: None,
            end_time: None,
            filter_pattern: None,
            limit: Some(100), // Default to 100 events
            log_stream_names: Vec::new(),
            start_from_head: false, // Default to most recent first
        }
    }

    /// Set start time
    pub fn with_start_time(mut self, start_time: i64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    /// Set end time
    pub fn with_end_time(mut self, end_time: i64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    /// Set filter pattern
    pub fn with_filter_pattern(mut self, pattern: String) -> Self {
        self.filter_pattern = Some(pattern);
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set log stream names
    pub fn with_log_stream_names(mut self, streams: Vec<String>) -> Self {
        self.log_stream_names = streams;
        self
    }

    /// Set start from head
    pub fn with_start_from_head(mut self, start_from_head: bool) -> Self {
        self.start_from_head = start_from_head;
        self
    }
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a CloudWatch Logs query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryResult {
    /// Log events returned by the query
    pub events: Vec<LogEvent>,
    /// Token for pagination (if more results available)
    pub next_token: Option<String>,
    /// Total number of events in this result
    pub total_events: usize,
    /// Query statistics (bytes scanned, records matched, etc.)
    pub query_statistics: QueryStatistics,
}

impl LogQueryResult {
    /// Create a new empty result
    pub fn empty() -> Self {
        Self {
            events: Vec::new(),
            next_token: None,
            total_events: 0,
            query_statistics: QueryStatistics::default(),
        }
    }

    /// Create a new result with events
    pub fn new(events: Vec<LogEvent>, next_token: Option<String>) -> Self {
        let total_events = events.len();
        Self {
            events,
            next_token,
            total_events,
            query_statistics: QueryStatistics::default(),
        }
    }

    /// Create result with statistics
    pub fn with_statistics(
        events: Vec<LogEvent>,
        next_token: Option<String>,
        statistics: QueryStatistics,
    ) -> Self {
        let total_events = events.len();
        Self {
            events,
            next_token,
            total_events,
            query_statistics: statistics,
        }
    }
}

/// A single log event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    /// Event timestamp (Unix milliseconds)
    pub timestamp: i64,
    /// Log message content
    pub message: String,
    /// Time when the event was ingested (Unix milliseconds)
    pub ingestion_time: i64,
    /// Name of the log stream this event belongs to
    pub log_stream_name: String,
}

impl LogEvent {
    /// Create a new log event
    pub fn new(timestamp: i64, message: String, log_stream_name: String) -> Self {
        Self {
            timestamp,
            message,
            ingestion_time: timestamp, // Default to same as timestamp
            log_stream_name,
        }
    }

    /// Create log event with ingestion time
    pub fn with_ingestion_time(
        timestamp: i64,
        message: String,
        ingestion_time: i64,
        log_stream_name: String,
    ) -> Self {
        Self {
            timestamp,
            message,
            ingestion_time,
            log_stream_name,
        }
    }
}

/// Statistics about a CloudWatch Logs query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatistics {
    /// Bytes scanned during the query
    pub bytes_scanned: f64,
    /// Number of records that matched the filter
    pub records_matched: f64,
    /// Total number of records scanned
    pub records_scanned: f64,
}

impl QueryStatistics {
    /// Create new query statistics
    pub fn new(bytes_scanned: f64, records_matched: f64, records_scanned: f64) -> Self {
        Self {
            bytes_scanned,
            records_matched,
            records_scanned,
        }
    }
}

impl Default for QueryStatistics {
    fn default() -> Self {
        Self {
            bytes_scanned: 0.0,
            records_matched: 0.0,
            records_scanned: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_options_builder() {
        let options = QueryOptions::new()
            .with_limit(500)
            .with_start_from_head(true)
            .with_filter_pattern("ERROR".to_string());

        assert_eq!(options.limit, Some(500));
        assert!(options.start_from_head);
        assert_eq!(options.filter_pattern, Some("ERROR".to_string()));
    }

    #[test]
    fn test_query_options_defaults() {
        let options = QueryOptions::default();

        assert_eq!(options.limit, Some(100));
        assert!(!options.start_from_head);
        assert!(options.filter_pattern.is_none());
        assert!(options.log_stream_names.is_empty());
    }

    #[test]
    fn test_log_event_creation() {
        let event = LogEvent::new(
            1234567890000,
            "Test message".to_string(),
            "test-stream".to_string(),
        );

        assert_eq!(event.timestamp, 1234567890000);
        assert_eq!(event.message, "Test message");
        assert_eq!(event.log_stream_name, "test-stream");
        assert_eq!(event.ingestion_time, event.timestamp);
    }

    #[test]
    fn test_log_query_result_empty() {
        let result = LogQueryResult::empty();

        assert_eq!(result.events.len(), 0);
        assert_eq!(result.total_events, 0);
        assert!(result.next_token.is_none());
    }

    #[test]
    fn test_log_query_result_serialization() {
        let events = vec![
            LogEvent::new(1000, "msg1".to_string(), "stream1".to_string()),
            LogEvent::new(2000, "msg2".to_string(), "stream1".to_string()),
        ];

        let result = LogQueryResult::new(events, None);

        // Should be serializable to JSON
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("msg1"));
        assert!(json.contains("stream1"));

        // Should be deserializable from JSON
        let deserialized: LogQueryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.events.len(), 2);
        assert_eq!(deserialized.total_events, 2);
    }
}
