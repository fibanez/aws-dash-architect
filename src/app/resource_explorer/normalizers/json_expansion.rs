//! JSON Expansion Utility
//!
//! This module provides post-processing for AWS resource JSON to detect and expand
//! embedded JSON strings. AWS APIs often return policy documents and other JSON
//! content as URL-encoded or stringified JSON, which makes the output hard to read.
//!
//! This utility:
//! - Detects URL-encoded JSON strings and decodes them
//! - Detects stringified JSON and parses it into proper JSON objects
//! - Recursively processes all values in the JSON tree
//!
//! # Example
//!
//! Input (before expansion):
//! ```json
//! {
//!   "PolicyDocument": "%7B%22Version%22%3A%222012-10-17%22%7D",
//!   "AssumeRolePolicyDocument": "{\"Version\":\"2012-10-17\"}"
//! }
//! ```
//!
//! Output (after expansion):
//! ```json
//! {
//!   "PolicyDocument": {
//!     "Version": "2012-10-17"
//!   },
//!   "AssumeRolePolicyDocument": {
//!     "Version": "2012-10-17"
//!   }
//! }
//! ```

use percent_encoding::percent_decode;
use serde_json::Value;

/// Known field names that commonly contain embedded JSON
const JSON_FIELD_HINTS: &[&str] = &[
    "PolicyDocument",
    "AssumeRolePolicyDocument",
    "Document",
    "Policy",
    "Configuration",
    "Definition",
    "Schema",
    "Template",
    "Manifest",
    "Metadata",
    "Parameters",
    "Environment",
    "Statement",
    "Principal",
    "Condition",
    "Resource",
    "Action",
    "NotAction",
    "NotResource",
    "NotPrincipal",
];

/// Recursively expand embedded JSON strings in a JSON value
///
/// This function walks the JSON tree and expands any string values that
/// appear to contain JSON (either URL-encoded or plain stringified JSON).
pub fn expand_embedded_json(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, val) in map {
                let expanded_val = expand_value_with_context(&key, val);
                new_map.insert(key, expanded_val);
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            let expanded_arr: Vec<Value> = arr.into_iter().map(expand_embedded_json).collect();
            Value::Array(expanded_arr)
        }
        other => other,
    }
}

/// Expand a value with knowledge of its field name for better detection
fn expand_value_with_context(field_name: &str, value: Value) -> Value {
    match value {
        Value::String(s) => {
            // Check if this field is known to contain JSON
            let is_json_field = JSON_FIELD_HINTS
                .iter()
                .any(|hint| field_name.contains(hint));

            // Try to expand the string
            if let Some(expanded) = try_expand_json_string(&s, is_json_field) {
                // Recursively expand in case of nested JSON
                expand_embedded_json(expanded)
            } else {
                Value::String(s)
            }
        }
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, val) in map {
                let expanded_val = expand_value_with_context(&key, val);
                new_map.insert(key, expanded_val);
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            let expanded_arr: Vec<Value> = arr.into_iter().map(expand_embedded_json).collect();
            Value::Array(expanded_arr)
        }
        other => other,
    }
}

/// Try to expand a string that might contain JSON
///
/// Returns Some(Value) if the string was successfully expanded, None otherwise.
fn try_expand_json_string(s: &str, is_json_field: bool) -> Option<Value> {
    let trimmed = s.trim();

    // Skip empty strings
    if trimmed.is_empty() {
        return None;
    }

    // First, try URL decoding if it looks URL-encoded
    let decoded = if looks_url_encoded(trimmed) {
        percent_decode(trimmed.as_bytes())
            .decode_utf8()
            .ok()
            .map(|cow| cow.to_string())
    } else {
        None
    };

    let to_parse = decoded.as_deref().unwrap_or(trimmed);

    // Check if it looks like JSON
    if looks_like_json(to_parse) || is_json_field {
        // Try to parse as JSON
        if let Ok(parsed) = serde_json::from_str::<Value>(to_parse) {
            // Only expand if it's a complex type (object or array)
            // Don't expand simple strings/numbers/booleans as that would be wrong
            match &parsed {
                Value::Object(_) | Value::Array(_) => return Some(parsed),
                _ => {}
            }
        }
    }

    None
}

/// Check if a string looks like it might be URL-encoded
fn looks_url_encoded(s: &str) -> bool {
    // URL-encoded strings contain %XX patterns
    // Check for common encoded characters like %7B ({), %22 ("), %3A (:)
    s.contains("%7B") || s.contains("%22") || s.contains("%3A") || s.contains("%2C")
}

/// Check if a string looks like it might be JSON
fn looks_like_json(s: &str) -> bool {
    let trimmed = s.trim();
    // JSON objects start with { and end with }
    // JSON arrays start with [ and end with ]
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_expand_url_encoded_json() {
        // URL-encoded policy document
        let input = json!({
            "PolicyDocument": "%7B%22Version%22%3A%222012-10-17%22%2C%22Statement%22%3A%5B%5D%7D"
        });

        let expanded = expand_embedded_json(input);

        assert!(expanded["PolicyDocument"].is_object());
        assert_eq!(expanded["PolicyDocument"]["Version"], "2012-10-17");
    }

    #[test]
    fn test_expand_stringified_json() {
        // Plain stringified JSON
        let input = json!({
            "AssumeRolePolicyDocument": "{\"Version\":\"2012-10-17\",\"Statement\":[]}"
        });

        let expanded = expand_embedded_json(input);

        assert!(expanded["AssumeRolePolicyDocument"].is_object());
        assert_eq!(
            expanded["AssumeRolePolicyDocument"]["Version"],
            "2012-10-17"
        );
    }

    #[test]
    fn test_expand_nested_json() {
        // JSON with nested objects containing embedded JSON
        let input = json!({
            "Role": {
                "RoleName": "test-role",
                "AssumeRolePolicyDocument": "{\"Version\":\"2012-10-17\"}"
            }
        });

        let expanded = expand_embedded_json(input);

        assert!(expanded["Role"]["AssumeRolePolicyDocument"].is_object());
        assert_eq!(
            expanded["Role"]["AssumeRolePolicyDocument"]["Version"],
            "2012-10-17"
        );
    }

    #[test]
    fn test_expand_array_with_embedded_json() {
        let input = json!({
            "InlinePolicies": [
                {
                    "PolicyName": "policy1",
                    "PolicyDocument": "{\"Version\":\"2012-10-17\"}"
                }
            ]
        });

        let expanded = expand_embedded_json(input);

        assert!(expanded["InlinePolicies"][0]["PolicyDocument"].is_object());
    }

    #[test]
    fn test_no_expansion_for_regular_strings() {
        let input = json!({
            "RoleName": "test-role",
            "Description": "This is a test role"
        });

        let expanded = expand_embedded_json(input);

        // Regular strings should remain as strings
        assert!(expanded["RoleName"].is_string());
        assert!(expanded["Description"].is_string());
        assert_eq!(expanded["RoleName"], "test-role");
    }

    #[test]
    fn test_looks_url_encoded() {
        assert!(looks_url_encoded("%7B%22Version%22%3A%222012-10-17%22%7D"));
        assert!(!looks_url_encoded("{\"Version\":\"2012-10-17\"}"));
        assert!(!looks_url_encoded("plain text"));
    }

    #[test]
    fn test_looks_like_json() {
        assert!(looks_like_json("{\"key\": \"value\"}"));
        assert!(looks_like_json("[1, 2, 3]"));
        assert!(looks_like_json("  { \"spaced\": true }  "));
        assert!(!looks_like_json("plain text"));
        assert!(!looks_like_json("{ incomplete"));
    }

    #[test]
    fn test_real_world_assume_role_policy() {
        // Realistic AWS AssumeRolePolicyDocument
        let policy = r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"lambda.amazonaws.com"},"Action":"sts:AssumeRole"}]}"#;

        let input = json!({
            "AssumeRolePolicyDocument": policy
        });

        let expanded = expand_embedded_json(input);

        assert!(expanded["AssumeRolePolicyDocument"].is_object());
        assert_eq!(
            expanded["AssumeRolePolicyDocument"]["Version"],
            "2012-10-17"
        );
        assert!(expanded["AssumeRolePolicyDocument"]["Statement"].is_array());
    }
}
