//! Data types for CloudTrail Events operations

#![warn(clippy::all, rust_2018_idioms)]

use serde::{Deserialize, Serialize};

/// Lookup attribute key for filtering CloudTrail events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LookupAttributeKey {
    /// Event ID
    EventId,
    /// Event name (e.g., "RunInstances", "CreateBucket")
    EventName,
    /// Read-only event (true/false)
    ReadOnly,
    /// Username (IAM user or role)
    Username,
    /// Resource type (e.g., "AWS::EC2::Instance")
    ResourceType,
    /// Resource name
    ResourceName,
    /// Event source (e.g., "ec2.amazonaws.com")
    EventSource,
    /// Access key ID
    AccessKeyId,
}

impl LookupAttributeKey {
    /// Convert to AWS SDK string
    pub fn as_str(&self) -> &'static str {
        match self {
            LookupAttributeKey::EventId => "EventId",
            LookupAttributeKey::EventName => "EventName",
            LookupAttributeKey::ReadOnly => "ReadOnly",
            LookupAttributeKey::Username => "Username",
            LookupAttributeKey::ResourceType => "ResourceType",
            LookupAttributeKey::ResourceName => "ResourceName",
            LookupAttributeKey::EventSource => "EventSource",
            LookupAttributeKey::AccessKeyId => "AccessKeyId",
        }
    }

    /// Convert to AWS SDK LookupAttributeKey type
    pub fn to_sdk(&self) -> aws_sdk_cloudtrail::types::LookupAttributeKey {
        match self {
            LookupAttributeKey::EventId => aws_sdk_cloudtrail::types::LookupAttributeKey::EventId,
            LookupAttributeKey::EventName => aws_sdk_cloudtrail::types::LookupAttributeKey::EventName,
            LookupAttributeKey::ReadOnly => aws_sdk_cloudtrail::types::LookupAttributeKey::ReadOnly,
            LookupAttributeKey::Username => aws_sdk_cloudtrail::types::LookupAttributeKey::Username,
            LookupAttributeKey::ResourceType => aws_sdk_cloudtrail::types::LookupAttributeKey::ResourceType,
            LookupAttributeKey::ResourceName => aws_sdk_cloudtrail::types::LookupAttributeKey::ResourceName,
            LookupAttributeKey::EventSource => aws_sdk_cloudtrail::types::LookupAttributeKey::EventSource,
            LookupAttributeKey::AccessKeyId => aws_sdk_cloudtrail::types::LookupAttributeKey::AccessKeyId,
        }
    }
}

/// Lookup attribute for filtering CloudTrail events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupAttribute {
    /// Attribute key
    pub attribute_key: LookupAttributeKey,
    /// Attribute value
    pub attribute_value: String,
}

impl LookupAttribute {
    /// Create new lookup attribute
    pub fn new(attribute_key: LookupAttributeKey, attribute_value: String) -> Self {
        Self {
            attribute_key,
            attribute_value,
        }
    }

    /// Create lookup attribute for resource type
    pub fn resource_type(resource_type: String) -> Self {
        Self::new(LookupAttributeKey::ResourceType, resource_type)
    }

    /// Create lookup attribute for event name
    pub fn event_name(event_name: String) -> Self {
        Self::new(LookupAttributeKey::EventName, event_name)
    }

    /// Create lookup attribute for username
    pub fn username(username: String) -> Self {
        Self::new(LookupAttributeKey::Username, username)
    }

    /// Create lookup attribute for resource name
    pub fn resource_name(resource_name: String) -> Self {
        Self::new(LookupAttributeKey::ResourceName, resource_name)
    }
}

/// Options for looking up CloudTrail events
#[derive(Debug, Clone)]
pub struct LookupOptions {
    /// Start time (Unix milliseconds timestamp)
    pub start_time: Option<i64>,

    /// End time (Unix milliseconds timestamp)
    pub end_time: Option<i64>,

    /// Lookup attributes for filtering
    pub lookup_attributes: Vec<LookupAttribute>,

    /// Maximum number of results per request (max 50)
    pub max_results: Option<i32>,

    /// Pagination token
    pub next_token: Option<String>,
}

impl LookupOptions {
    /// Create new lookup options with sensible defaults
    pub fn new() -> Self {
        Self {
            start_time: None,
            end_time: None,
            lookup_attributes: Vec::new(),
            max_results: Some(50), // CloudTrail max per request
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

    /// Builder pattern: add lookup attribute
    pub fn with_lookup_attribute(mut self, attribute: LookupAttribute) -> Self {
        self.lookup_attributes.push(attribute);
        self
    }

    /// Builder pattern: add multiple lookup attributes
    pub fn with_lookup_attributes(mut self, attributes: Vec<LookupAttribute>) -> Self {
        self.lookup_attributes.extend(attributes);
        self
    }

    /// Builder pattern: filter by resource type
    pub fn with_resource_type(self, resource_type: String) -> Self {
        self.with_lookup_attribute(LookupAttribute::resource_type(resource_type))
    }

    /// Builder pattern: filter by event name
    pub fn with_event_name(self, event_name: String) -> Self {
        self.with_lookup_attribute(LookupAttribute::event_name(event_name))
    }

    /// Builder pattern: filter by username
    pub fn with_username(self, username: String) -> Self {
        self.with_lookup_attribute(LookupAttribute::username(username))
    }

    /// Builder pattern: set max results (capped at 50)
    pub fn with_max_results(mut self, max_results: i32) -> Self {
        self.max_results = Some(max_results.min(50));
        self
    }

    /// Builder pattern: set pagination token
    pub fn with_next_token(mut self, token: String) -> Self {
        self.next_token = Some(token);
        self
    }
}

impl Default for LookupOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// CloudTrail event resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResource {
    /// Resource type (e.g., "AWS::EC2::Instance")
    pub resource_type: Option<String>,

    /// Resource name/identifier
    pub resource_name: Option<String>,
}

/// CloudTrail event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudTrailEvent {
    /// Event ID (unique identifier)
    pub event_id: String,

    /// Event name (API operation, e.g., "RunInstances")
    pub event_name: String,

    /// Event time (Unix milliseconds timestamp)
    pub event_time: i64,

    /// Event source (AWS service, e.g., "ec2.amazonaws.com")
    pub event_source: String,

    /// Username (IAM user/role that made the call)
    pub username: String,

    /// Resources affected by this event
    pub resources: Vec<EventResource>,

    /// Full CloudTrail event JSON (optional, can be large)
    pub cloud_trail_event: Option<String>,

    /// Access key ID used for the call
    pub access_key_id: Option<String>,

    /// Read-only event (true if it doesn't modify resources)
    pub read_only: Option<String>,

    /// Error code (if the API call failed)
    pub error_code: Option<String>,

    /// Error message (if the API call failed)
    pub error_message: Option<String>,
}

/// Result of a CloudTrail lookup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResult {
    /// Events returned from the lookup
    pub events: Vec<CloudTrailEvent>,

    /// Token for fetching next page of results
    pub next_token: Option<String>,

    /// Total number of events in this result
    pub total_events: usize,
}

impl LookupResult {
    /// Create empty result
    pub fn empty() -> Self {
        Self {
            events: Vec::new(),
            next_token: None,
            total_events: 0,
        }
    }

    /// Create new result
    pub fn new(events: Vec<CloudTrailEvent>, next_token: Option<String>) -> Self {
        let total_events = events.len();
        Self {
            events,
            next_token,
            total_events,
        }
    }

    /// Merge another result into this one (for pagination)
    pub fn merge(&mut self, other: LookupResult) {
        self.events.extend(other.events);
        self.next_token = other.next_token;
        self.total_events = self.events.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_options_builder() {
        let options = LookupOptions::new()
            .with_start_time(1000)
            .with_end_time(2000)
            .with_resource_type("AWS::EC2::Instance".to_string())
            .with_max_results(25);

        assert_eq!(options.start_time, Some(1000));
        assert_eq!(options.end_time, Some(2000));
        assert_eq!(options.lookup_attributes.len(), 1);
        assert_eq!(options.max_results, Some(25));
    }

    #[test]
    fn test_lookup_attribute_creation() {
        let attr = LookupAttribute::resource_type("AWS::Lambda::Function".to_string());
        assert_eq!(attr.attribute_key, LookupAttributeKey::ResourceType);
        assert_eq!(attr.attribute_value, "AWS::Lambda::Function");
    }

    #[test]
    fn test_max_results_capped_at_50() {
        let options = LookupOptions::new().with_max_results(100);
        assert_eq!(options.max_results, Some(50));
    }

    #[test]
    fn test_lookup_result_serialization() {
        let event = CloudTrailEvent {
            event_id: "test-id".to_string(),
            event_name: "RunInstances".to_string(),
            event_time: 1234567890,
            event_source: "ec2.amazonaws.com".to_string(),
            username: "test-user".to_string(),
            resources: vec![EventResource {
                resource_type: Some("AWS::EC2::Instance".to_string()),
                resource_name: Some("i-1234567890abcdef0".to_string()),
            }],
            cloud_trail_event: None,
            access_key_id: None,
            read_only: Some("false".to_string()),
            error_code: None,
            error_message: None,
        };

        let result = LookupResult::new(vec![event], Some("next-token".to_string()));

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: LookupResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.events.len(), 1);
        assert_eq!(deserialized.next_token, Some("next-token".to_string()));
        assert_eq!(deserialized.events[0].event_name, "RunInstances");
    }

    #[test]
    fn test_lookup_result_merge() {
        let event1 = CloudTrailEvent {
            event_id: "test-id-1".to_string(),
            event_name: "RunInstances".to_string(),
            event_time: 1234567890,
            event_source: "ec2.amazonaws.com".to_string(),
            username: "test-user".to_string(),
            resources: vec![],
            cloud_trail_event: None,
            access_key_id: None,
            read_only: Some("false".to_string()),
            error_code: None,
            error_message: None,
        };

        let event2 = CloudTrailEvent {
            event_id: "test-id-2".to_string(),
            event_name: "TerminateInstances".to_string(),
            event_time: 1234567900,
            event_source: "ec2.amazonaws.com".to_string(),
            username: "test-user".to_string(),
            resources: vec![],
            cloud_trail_event: None,
            access_key_id: None,
            read_only: Some("false".to_string()),
            error_code: None,
            error_message: None,
        };

        let mut result1 = LookupResult::new(vec![event1], Some("token1".to_string()));
        let result2 = LookupResult::new(vec![event2], None);

        result1.merge(result2);

        assert_eq!(result1.events.len(), 2);
        assert_eq!(result1.next_token, None);
        assert_eq!(result1.total_events, 2);
    }
}
