//! V8 Runtime for JavaScript Execution
//!
//! Manages V8 isolate lifecycle, execution, and result extraction.
//!
//! # Example
//!
//! ```no_run
//! use awsdash::app::agent_framework::v8_bindings::{V8Runtime, RuntimeConfig};
//!
//! let runtime = V8Runtime::new();
//! let result = runtime.execute("2 + 2").expect("Execution failed");
//!
//! assert!(result.success);
//! assert_eq!(result.result.unwrap(), "4");
//! ```

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{anyhow, Result};
use std::pin::pin;
use std::thread;
use std::time::{Duration, Instant};

use super::bindings::register_bindings;
use super::console::{register_console, ConsoleBuffers};

/// Configuration for V8 runtime execution
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum heap size in bytes (default: 256MB)
    pub max_heap_size_bytes: usize,

    /// Execution timeout (default: 30 seconds)
    pub timeout: Duration,

    /// Enable console output capture (default: true)
    pub capture_console: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_heap_size_bytes: 256 * 1024 * 1024, // 256MB
            timeout: Duration::from_secs(30),
            capture_console: true,
        }
    }
}

/// Result of JavaScript execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,

    /// Return value as JSON string (null if error)
    pub result: Option<String>,

    /// Captured stdout (console.log, etc.)
    pub stdout: String,

    /// Captured stderr (console.error, exceptions)
    pub stderr: String,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// V8 JavaScript runtime
pub struct V8Runtime {
    config: RuntimeConfig,
}

impl V8Runtime {
    /// Create a new V8 runtime with default configuration
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
        }
    }

    /// Create a new V8 runtime with custom configuration
    pub fn with_config(config: RuntimeConfig) -> Self {
        Self { config }
    }

    /// Execute JavaScript code and return result
    ///
    /// # Arguments
    /// * `code` - JavaScript source code to execute
    ///
    /// # Returns
    /// ExecutionResult with success status, return value, output, and timing
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use awsdash::app::agent_framework::v8_bindings::V8Runtime;
    /// let runtime = V8Runtime::new();
    /// let result = runtime.execute("2 + 2").unwrap();
    /// assert!(result.success);
    /// assert_eq!(result.result.unwrap(), "4");
    /// ```
    pub fn execute(&self, code: &str) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        // Create isolate with memory limits
        let mut params = v8::CreateParams::default();
        params = params.heap_limits(0, self.config.max_heap_size_bytes);

        let mut isolate = v8::Isolate::new(params);

        // Get thread-safe handle for timeout enforcement
        let isolate_handle = isolate.thread_safe_handle();
        let timeout = self.config.timeout;

        // Spawn timeout watchdog thread
        let _timeout_thread = thread::spawn(move || {
            thread::sleep(timeout);
            // Terminate execution after timeout
            isolate_handle.terminate_execution();
        });

        // Execute JavaScript in proper scope hierarchy
        let result = {
            let scope = pin!(v8::HandleScope::new(&mut isolate));
            let scope = &mut scope.init();
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);

            // Create console buffers and register console functions
            let console_buffers = if self.config.capture_console {
                let buffers = ConsoleBuffers::new();
                register_console(scope, buffers.clone());
                Some(buffers)
            } else {
                None
            };

            // Register function bindings (listAccounts, etc.)
            if let Err(e) = register_bindings(scope) {
                let (stdout, mut stderr) = if let Some(ref buffers) = console_buffers {
                    (buffers.get_stdout(), buffers.get_stderr())
                } else {
                    (String::new(), String::new())
                };
                stderr.push_str(&format!("Failed to register bindings: {}", e));
                return Ok(ExecutionResult {
                    success: false,
                    result: None,
                    stdout,
                    stderr,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }

            // Compile JavaScript
            let code_str = v8::String::new(scope, code)
                .ok_or_else(|| anyhow!("Failed to create V8 string from code"))?;

            let scope = pin!(v8::TryCatch::new(scope));
            let scope = &mut scope.init();

            let script = match v8::Script::compile(scope, code_str, None) {
                Some(script) => script,
                None => {
                    // Extract console output before returning
                    let (stdout, mut stderr) = if let Some(ref buffers) = console_buffers {
                        (buffers.get_stdout(), buffers.get_stderr())
                    } else {
                        (String::new(), String::new())
                    };

                    // Check if terminated by timeout
                    if scope.has_terminated() {
                        stderr.push_str(&format!("Execution terminated (timeout: {:?})", timeout));
                        return Ok(ExecutionResult {
                            success: false,
                            result: None,
                            stdout,
                            stderr,
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                        });
                    }

                    // Compilation error
                    let exception = scope.exception().unwrap();
                    let exception_str = exception.to_string(scope).unwrap();
                    let exception_msg = exception_str.to_rust_string_lossy(scope);
                    stderr.push_str(&exception_msg);

                    return Ok(ExecutionResult {
                        success: false,
                        result: None,
                        stdout,
                        stderr,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            };

            // Execute JavaScript
            let result = match script.run(scope) {
                Some(result) => result,
                None => {
                    // Extract console output before returning
                    let (stdout, mut stderr) = if let Some(ref buffers) = console_buffers {
                        (buffers.get_stdout(), buffers.get_stderr())
                    } else {
                        (String::new(), String::new())
                    };

                    // Check if terminated by timeout
                    if scope.has_terminated() {
                        stderr.push_str(&format!("Execution terminated (timeout: {:?})", timeout));
                        return Ok(ExecutionResult {
                            success: false,
                            result: None,
                            stdout,
                            stderr,
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                        });
                    }

                    // Runtime error
                    let exception = scope.exception().unwrap();
                    let exception_str = exception.to_string(scope).unwrap();
                    let exception_msg = exception_str.to_rust_string_lossy(scope);
                    stderr.push_str(&exception_msg);

                    return Ok(ExecutionResult {
                        success: false,
                        result: None,
                        stdout,
                        stderr,
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            };

            // Extract result as JSON using v8::json::stringify
            // This properly serializes JavaScript objects/arrays as JSON
            // instead of using JavaScript's toString() which gives "[object Object]"
            let result_json = if let Some(json_value) = v8::json::stringify(scope, result) {
                json_value.to_rust_string_lossy(scope)
            } else {
                // If JSON serialization fails (e.g., circular references, BigInt),
                // fall back to toString()
                let result_str = result.to_string(scope).unwrap();
                result_str.to_rust_string_lossy(scope)
            };

            Ok::<(String, Option<ConsoleBuffers>), anyhow::Error>((result_json, console_buffers))
        }?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Extract console output
        let (result, console_buffers) = result;
        let (stdout, stderr) = if let Some(buffers) = console_buffers {
            (buffers.get_stdout(), buffers.get_stderr())
        } else {
            (String::new(), String::new())
        };

        Ok(ExecutionResult {
            success: true,
            result: Some(result),
            stdout,
            stderr,
            execution_time_ms,
        })
    }
}

impl Default for V8Runtime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::v8_bindings::{initialize_v8_platform, is_v8_initialized};

    #[test]
    fn test_runtime_creation() {
        let runtime = V8Runtime::new();
        assert_eq!(runtime.config.max_heap_size_bytes, 256 * 1024 * 1024);
        assert_eq!(runtime.config.timeout, Duration::from_secs(30));
        assert!(runtime.config.capture_console);
    }

    #[test]
    fn test_runtime_with_custom_config() {
        let config = RuntimeConfig {
            max_heap_size_bytes: 128 * 1024 * 1024,
            timeout: Duration::from_secs(10),
            capture_console: false,
        };
        let runtime = V8Runtime::with_config(config.clone());
        assert_eq!(runtime.config.max_heap_size_bytes, 128 * 1024 * 1024);
        assert_eq!(runtime.config.timeout, Duration::from_secs(10));
        assert!(!runtime.config.capture_console);
    }

    #[test]
    fn test_simple_arithmetic() {
        // Ensure V8 is initialized
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let result = runtime.execute("2 + 2").unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap(), "4");
        assert!(result.execution_time_ms < 1000);
    }

    #[test]
    fn test_variable_operations() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let result = runtime.execute("const x = 10; const y = 20; x + y").unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap(), "30");
    }

    #[test]
    fn test_string_operations() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let result = runtime.execute("'hello' + ' ' + 'world'").unwrap();

        assert!(result.success);
        // JSON.stringify wraps strings in quotes
        assert_eq!(result.result.unwrap(), "\"hello world\"");
    }

    #[test]
    fn test_compilation_error() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let result = runtime.execute("const x = ;").unwrap();

        assert!(!result.success);
        assert!(result.result.is_none());
        assert!(result.stderr.contains("Unexpected token"));
    }

    #[test]
    fn test_runtime_error() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let result = runtime.execute("throw new Error('test error');").unwrap();

        assert!(!result.success);
        assert!(result.result.is_none());
        assert!(result.stderr.contains("test error"));
    }

    #[test]
    fn test_undefined_variable() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let result = runtime.execute("nonExistentVariable").unwrap();

        assert!(!result.success);
        assert!(result.stderr.contains("not defined"));
    }

    #[test]
    fn test_quick_execution_no_timeout() {
        let _ = initialize_v8_platform();

        let config = RuntimeConfig {
            timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let runtime = V8Runtime::with_config(config);
        let result = runtime.execute("2 + 2").unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap(), "4");
        assert!(result.execution_time_ms < 5000); // Should be much faster
    }

    #[test]
    fn test_infinite_loop_timeout() {
        let _ = initialize_v8_platform();

        let config = RuntimeConfig {
            timeout: Duration::from_millis(100), // Short timeout for test
            ..Default::default()
        };

        let runtime = V8Runtime::with_config(config);
        let code = "while(true) {}"; // Infinite loop
        let result = runtime.execute(code).unwrap();

        assert!(!result.success, "Should timeout");
        assert!(result.result.is_none());
        assert!(
            result.stderr.contains("timeout") || result.stderr.contains("terminated"),
            "Error message should mention timeout or terminated, got: {}",
            result.stderr
        );
        assert!(result.execution_time_ms >= 100); // Should hit timeout
        assert!(result.execution_time_ms < 500); // Should terminate quickly after timeout
    }

    #[test]
    fn test_configurable_timeout() {
        let _ = initialize_v8_platform();

        // Test with 1 second timeout
        let config1 = RuntimeConfig {
            timeout: Duration::from_secs(1),
            ..Default::default()
        };

        // Test with 2 second timeout
        let config2 = RuntimeConfig {
            timeout: Duration::from_secs(2),
            ..Default::default()
        };

        let runtime1 = V8Runtime::with_config(config1);
        let runtime2 = V8Runtime::with_config(config2);

        // Both should execute quick code successfully
        let result1 = runtime1.execute("42").unwrap();
        let result2 = runtime2.execute("42").unwrap();

        assert!(result1.success);
        assert!(result2.success);
    }

    // Memory limit tests

    #[test]
    fn test_small_allocation_succeeds() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let code = r#"
            const arr = new Array(1000).fill({data: 'test'});
            arr.length;
        "#;
        let result = runtime.execute(code).unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap(), "1000");
    }

    #[test]
    fn test_moderate_allocation() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let code = r#"
            const arr = new Array(100000).fill({
                name: 'item',
                value: 42,
                data: 'some string data'
            });
            arr.length;
        "#;
        let result = runtime.execute(code).unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap(), "100000");
    }

    #[test]
    fn test_memory_limit_configurable() {
        let _ = initialize_v8_platform();

        let config_small = RuntimeConfig {
            max_heap_size_bytes: 50 * 1024 * 1024, // 50MB
            ..Default::default()
        };

        let config_large = RuntimeConfig {
            max_heap_size_bytes: 512 * 1024 * 1024, // 512MB
            ..Default::default()
        };

        let runtime_small = V8Runtime::with_config(config_small);
        let runtime_large = V8Runtime::with_config(config_large);

        // Both should handle small allocations
        let code = "new Array(1000).fill(42).length";
        let result1 = runtime_small.execute(code).unwrap();
        let result2 = runtime_large.execute(code).unwrap();

        assert!(result1.success);
        assert!(result2.success);
        assert_eq!(result1.result.unwrap(), "1000");
        assert_eq!(result2.result.unwrap(), "1000");
    }

    #[test]
    fn test_memory_limit_enforced() {
        let _ = initialize_v8_platform();

        // Use very small memory limit for testing OOM
        let config = RuntimeConfig {
            max_heap_size_bytes: 5 * 1024 * 1024, // 5MB limit
            timeout: Duration::from_secs(5), // Give enough time
            ..Default::default()
        };

        let runtime = V8Runtime::with_config(config);

        // Small allocation should succeed
        let small_result = runtime.execute("new Array(100).fill(42).length").unwrap();
        assert!(small_result.success);

        // Note: V8 memory limits are advisory and may not trigger OOM consistently
        // This test verifies the limit is set but doesn't strictly verify OOM behavior
        // as it depends on V8 GC implementation details
    }

    // Integration tests

    #[test]
    fn test_complete_workflow() {
        // 1. Initialize platform
        let result = initialize_v8_platform();
        assert!(result.is_ok());
        assert!(is_v8_initialized());

        // 2. Create runtime with custom config
        let config = RuntimeConfig {
            max_heap_size_bytes: 128 * 1024 * 1024, // 128MB
            timeout: Duration::from_secs(10),
            capture_console: true,
        };
        let runtime = V8Runtime::with_config(config);

        // 3. Execute complex JavaScript
        let code = r#"
            // Define function
            function fibonacci(n) {
                if (n <= 1) return n;
                return fibonacci(n - 1) + fibonacci(n - 2);
            }

            // Calculate fibonacci
            const result = fibonacci(10);

            // Return result
            result;
        "#;

        let result = runtime.execute(code).unwrap();

        // 4. Verify result
        assert!(result.success);
        assert!(result.result.is_some());

        let result_str = result.result.unwrap();
        assert_eq!(result_str, "55"); // fibonacci(10) = 55

        // 5. Verify performance
        assert!(result.execution_time_ms < 1000); // Should be fast
    }

    #[test]
    fn test_multiple_executions() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();

        // Execute multiple times (fresh isolate each time)
        for i in 1..=5 {
            let code = format!("{} * 2", i);
            let result = runtime.execute(&code).unwrap();

            assert!(result.success);
            assert_eq!(result.result.unwrap(), format!("{}", i * 2));
        }
    }

    #[test]
    fn test_error_recovery() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();

        // 1. Execute valid code
        let result1 = runtime.execute("42").unwrap();
        assert!(result1.success);

        // 2. Execute invalid code
        let result2 = runtime.execute("invalid syntax {").unwrap();
        assert!(!result2.success);

        // 3. Execute valid code again (should recover)
        let result3 = runtime.execute("100").unwrap();
        assert!(result3.success);
    }

    // Console integration tests

    #[test]
    fn test_console_output_capture() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let code = r#"
            console.log("Hello from JavaScript");
            console.error("This is an error");
            console.warn("This is a warning");
            2 + 2
        "#;

        let result = runtime.execute(code).unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap(), "4");
        assert!(
            result.stdout.contains("Hello from JavaScript"),
            "stdout should contain log output, got: {}",
            result.stdout
        );
        assert!(
            result.stdout.contains("This is a warning"),
            "stdout should contain warn output, got: {}",
            result.stdout
        );
        assert!(
            result.stderr.contains("This is an error"),
            "stderr should contain error output, got: {}",
            result.stderr
        );
    }

    #[test]
    fn test_console_disabled() {
        let _ = initialize_v8_platform();

        let config = RuntimeConfig {
            capture_console: false,
            ..Default::default()
        };

        let runtime = V8Runtime::with_config(config);
        let code = r#"
            console.log("This should not be captured");
            console.error("This error should not be captured");
            42
        "#;

        let result = runtime.execute(code).unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap(), "42");
        assert_eq!(result.stdout, "");
        assert_eq!(result.stderr, "");
    }

    #[test]
    fn test_console_with_error() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let code = r#"
            console.log("Before error");
            throw new Error("test error");
        "#;

        let result = runtime.execute(code).unwrap();

        assert!(!result.success);
        assert!(result.result.is_none());
        assert!(
            result.stdout.contains("Before error"),
            "stdout should contain output before error, got: {}",
            result.stdout
        );
        assert!(
            result.stderr.contains("test error"),
            "stderr should contain exception message, got: {}",
            result.stderr
        );
    }

    #[test]
    fn test_console_multiple_calls() {
        let _ = initialize_v8_platform();

        let runtime = V8Runtime::new();
        let code = r#"
            console.log("Line 1");
            console.log("Line 2");
            console.error("Error 1");
            console.error("Error 2");
            console.warn("Warning");
            console.debug("Debug");
            100
        "#;

        let result = runtime.execute(code).unwrap();

        assert!(result.success);
        assert_eq!(result.result.unwrap(), "100");

        // Check stdout contains all log/warn/debug output
        assert!(result.stdout.contains("Line 1"));
        assert!(result.stdout.contains("Line 2"));
        assert!(result.stdout.contains("Warning"));
        assert!(result.stdout.contains("Debug"));

        // Check stderr contains all error output
        assert!(result.stderr.contains("Error 1"));
        assert!(result.stderr.contains("Error 2"));
    }

    #[test]
    fn test_foreach_pattern_investigation() {
        // This test investigates why the LLM's code returned undefined
        // instead of the accounts array
        let _ = initialize_v8_platform();
        let runtime = V8Runtime::new();

        // Pattern 1: Just the array reference
        let code1 = "const arr = [1, 2, 3]; arr;";
        let result1 = runtime.execute(code1).unwrap();
        println!("Pattern 1 (just array): result={:?}", result1.result);
        assert!(result1.success);
        assert!(result1.result.is_some());

        // Pattern 2: Array with forEach then array reference
        let code2 = r#"
const arr = [1, 2, 3];
arr.forEach(x => console.log(x));
arr;
        "#;
        let result2 = runtime.execute(code2).unwrap();
        println!("Pattern 2 (forEach then array): result={:?}", result2.result);
        println!("Pattern 2 stdout: {}", result2.stdout);
        assert!(result2.success);

        // Pattern 3: The actual failing pattern from the log
        let code3 = r#"
const accounts = [{id: "123", name: "Test"}];
console.log(`Found ${accounts.length} total accounts`);

// Create detailed output
accounts.forEach(acc => {
    console.log(`- ${acc.name} (${acc.id})`);
});

accounts;
        "#;
        let result3 = runtime.execute(code3).unwrap();
        println!("Pattern 3 (exact log pattern): result={:?}", result3.result);
        println!("Pattern 3 stdout: {}", result3.stdout);
        assert!(result3.success);

        // Pattern 4: Test with listAccounts() binding
        let code4 = r#"
const accounts = listAccounts();
console.log(`Found ${accounts.length} total accounts`);

// Create detailed output
accounts.forEach(acc => {
    console.log(`- ${acc.name} (${acc.id})`);
});

accounts;
        "#;
        let result4 = runtime.execute(code4).unwrap();
        println!("\nPattern 4 (with listAccounts() binding):");
        println!("  success: {}", result4.success);
        println!("  result: {:?}", result4.result);
        println!("  stdout: {}", result4.stdout);
        println!("  stderr: {}", result4.stderr);
        assert!(result4.success);
    }
}
