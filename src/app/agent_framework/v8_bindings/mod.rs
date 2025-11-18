//! V8 JavaScript Engine Bindings
//!
//! This module provides V8 JavaScript execution capabilities for the agent framework.
//!
//! # Architecture
//!
//! - **Platform**: Global V8 platform initialized once at app startup
//! - **Runtime**: Per-execution isolate with configurable limits
//! - **Bindings**: Rust functions bound to JavaScript for infrastructure operations
//!
//! # Usage
//!
//! ```no_run
//! use awsdash::app::agent_framework::v8_bindings::initialize_v8_platform;
//!
//! // Initialize at app startup
//! initialize_v8_platform().expect("Failed to initialize V8");
//! ```

#![warn(clippy::all, rust_2018_idioms)]

pub mod bindings;
pub mod console;
pub mod platform;
pub mod runtime;
pub mod types;

pub use bindings::{get_api_documentation, register_bindings, set_global_aws_identity};
pub use console::{register_console, ConsoleBuffers};
pub use platform::{initialize_v8_platform, is_v8_initialized};
pub use runtime::{ExecutionResult, RuntimeConfig, V8Runtime};
pub use types::{from_v8_value, to_v8_value};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::pin::pin;

    #[test]
    fn test_full_integration() {
        // Initialize V8 platform
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // Register console
        let console_buffers = ConsoleBuffers::new();
        register_console(scope, console_buffers.clone());

        // Register bindings
        register_bindings(scope).unwrap();

        // Execute JavaScript using both console and bindings
        let code = r#"
            console.log("Getting accounts...");
            const accounts = listAccounts();
            console.log(`Found ${accounts.length} accounts`);

            const accountNames = accounts.map(a => a.name).join(', ');
            console.log(`Account names: ${accountNames}`);

            JSON.stringify({
                count: accounts.length,
                names: accounts.map(a => a.name)
            })
        "#;

        let code_str = v8::String::new(scope, code).unwrap();
        let script = v8::Script::compile(scope, code_str, None).unwrap();
        let result = script.run(scope).unwrap();

        // Verify result
        let result_str = result.to_string(scope).unwrap();
        let result_json = result_str.to_rust_string_lossy(scope);
        assert!(result_json.contains("count"));
        assert!(result_json.contains("names"));

        // Verify console output
        let stdout = console_buffers.get_stdout();
        assert!(stdout.contains("Getting accounts"));
        assert!(stdout.contains("Found"));
        assert!(stdout.contains("Account names"));
    }
}
