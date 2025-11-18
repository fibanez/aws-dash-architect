//! Rust Function Bindings for JavaScript Execution
//!
//! This module provides the registry system for binding Rust functions
//! into V8 JavaScript contexts. All bound functions follow a consistent
//! pattern and are automatically registered.

#![warn(clippy::all, rust_2018_idioms)]

pub mod accounts;
pub mod cloudwatch_logs;
pub mod cloudtrail_events;
pub mod regions;
pub mod resources;

use anyhow::Result;

// Re-export the global identity setter for application initialization
pub use accounts::set_global_aws_identity;

/// Register all bound functions into a V8 context
///
/// This function is called during V8Runtime initialization to make
/// all Rust functions available to JavaScript code.
///
/// # Example
/// ```no_run
/// # use awsdash::app::agent_framework::v8_bindings::bindings::register_bindings;
/// # fn example(scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>) {
/// register_bindings(scope).unwrap();
///
/// // Now JavaScript can call: listAccounts(), etc.
/// # }
/// ```
pub fn register_bindings(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
) -> Result<()> {
    // Register account-related functions
    accounts::register(scope)?;

    // Register region-related functions
    regions::register(scope)?;

    // Register resource query functions
    resources::register(scope)?;

    // Register CloudWatch Logs functions
    cloudwatch_logs::register(scope)?;

    // Register CloudTrail Events functions
    cloudtrail_events::register(scope)?;

    // Future: Register other function categories here
    // files::register(scope)?;
    // aws_services::register(scope)?;

    Ok(())
}

/// Get the LLM documentation for all bound functions
///
/// Returns a formatted string containing TypeScript-style function
/// signatures, JSON schemas, and usage examples for every bound function.
///
/// This documentation is included in the agent's system prompt so the
/// LLM knows what APIs are available and how to use them.
///
/// # Example
/// ```
/// # use awsdash::app::agent_framework::v8_bindings::bindings::get_api_documentation;
/// let docs = get_api_documentation();
/// assert!(docs.contains("Available JavaScript APIs"));
/// ```
pub fn get_api_documentation() -> String {
    let mut docs = String::new();

    docs.push_str("# Available JavaScript APIs\n\n");
    docs.push_str("The following functions are available in your JavaScript execution environment.\n");
    docs.push_str("All functions are synchronous and return data immediately.\n\n");

    docs.push_str("## Account Management\n\n");
    docs.push_str(&accounts::get_documentation());

    docs.push_str("\n## Region Management\n\n");
    docs.push_str(&regions::get_documentation());

    docs.push_str("\n## Resource Queries\n\n");
    docs.push_str(&resources::get_documentation());

    docs.push_str("\n## CloudWatch Logs\n\n");
    docs.push_str(&cloudwatch_logs::get_documentation());

    docs.push_str("\n## CloudTrail Events\n\n");
    docs.push_str(&cloudtrail_events::get_documentation());

    docs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
    use std::pin::pin;

    #[test]
    fn test_register_bindings_no_crash() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Should not panic
        let result = register_bindings(scope);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_api_documentation() {
        let docs = get_api_documentation();

        // Should contain expected sections
        assert!(docs.contains("Available JavaScript APIs"));
        assert!(docs.contains("Account Management"));

        // Should not be empty
        assert!(!docs.is_empty());
    }
}
