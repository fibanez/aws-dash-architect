//! Type Conversion Utilities for V8 Bindings
//!
//! Provides bidirectional conversion between Rust types and V8 JavaScript values
//! using JSON as the interchange format. This ensures type safety and simplifies
//! the binding implementation.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Convert a Rust value (via JSON) to a V8 JavaScript value
///
/// This function serializes any Rust type implementing Serialize to JSON,
/// then parses that JSON in V8 to create a JavaScript value.
///
/// # Example
/// ```no_run
/// # use awsdash::app::agent_framework::v8_bindings::types::to_v8_value;
/// # use serde::Serialize;
/// #[derive(Serialize)]
/// struct Account {
///     id: String,
///     name: String,
/// }
///
/// # fn example(scope: &mut v8::HandleScope) {
/// let account = Account { id: "123".into(), name: "Prod".into() };
/// let js_value = to_v8_value(scope, &account).unwrap();
/// # }
/// ```
pub fn to_v8_value<'a, T: Serialize>(
    scope: &mut v8::ContextScope<'_, 'a, v8::HandleScope<'a>>,
    value: &T,
) -> Result<v8::Local<'a, v8::Value>> {
    // Serialize Rust value to JSON string
    let json_str = serde_json::to_string(value)
        .map_err(|e| anyhow!("Failed to serialize to JSON: {}", e))?;

    // Create V8 string from JSON
    let v8_str = v8::String::new(scope, &json_str)
        .ok_or_else(|| anyhow!("Failed to create V8 string"))?;

    // Parse JSON in V8 context to create JavaScript value
    let json = v8::json::parse(scope, v8_str)
        .ok_or_else(|| anyhow!("Failed to parse JSON in V8"))?;

    Ok(json)
}

/// Convert a V8 JavaScript value to a Rust value (via JSON)
///
/// This function serializes a V8 value to JSON, then deserializes
/// that JSON into a Rust type implementing Deserialize.
///
/// # Example
/// ```no_run
/// # use awsdash::app::agent_framework::v8_bindings::types::from_v8_value;
/// # use serde::Deserialize;
/// #[derive(Deserialize)]
/// struct Query {
///     region: String,
///     limit: usize,
/// }
///
/// # fn example(scope: &mut v8::HandleScope, js_value: v8::Local<v8::Value>) {
/// let query: Query = from_v8_value(scope, js_value).unwrap();
/// # }
/// ```
pub fn from_v8_value<T: for<'de> Deserialize<'de>>(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
    value: v8::Local<'_, v8::Value>,
) -> Result<T> {
    // Stringify V8 value to JSON
    let json_str = v8::json::stringify(scope, value)
        .ok_or_else(|| anyhow!("Failed to stringify V8 value"))?;

    // Convert V8 string to Rust string
    let json_rust_str = json_str.to_rust_string_lossy(scope);

    // Deserialize JSON to Rust type
    let rust_value: T = serde_json::from_str(&json_rust_str)
        .map_err(|e| anyhow!("Failed to deserialize JSON: {}", e))?;

    Ok(rust_value)
}

// These functions are tested through integration tests in runtime.rs and binding tests.
// Unit tests are not included due to Rust lifetime+macro interaction issues with v8::scope!.
