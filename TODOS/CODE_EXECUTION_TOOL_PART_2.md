# CODE EXECUTION TOOL IMPLEMENTATION - PART 2
# MILESTONE 1: V8 INFRASTRUCTURE SETUP

**Created**: 2025-11-13
**Last Updated**: 2025-01-14
**Status**: ✅ COMPLETE - All Sub-Milestones Implemented and Tested
**Test Results**: 25/25 tests passing (3 platform + 22 runtime)
**Parent Document**: [CODE_EXECUTION_TOOL.md](./CODE_EXECUTION_TOOL.md)

---

## Overview

This document provides detailed specifications and implementation steps for **Milestone 1: V8 Infrastructure Setup**, breaking it down into sub-milestones with TDD approach.

**Reference**: See [CODE_EXECUTION_TOOL.md](./CODE_EXECUTION_TOOL.md) for:
- Architecture decisions (Decisions 1-14)
- Overall milestone overview
- Tool interface specifications
- Security and resource limits

---

## ✅ IMPLEMENTATION COMPLETE

**Completion Date**: 2025-01-14
**Implementation Files**:
- `src/app/agent_framework/v8_bindings/platform.rs` - Global V8 platform initialization
- `src/app/agent_framework/v8_bindings/runtime.rs` - V8Runtime with isolates, timeouts, memory limits

**Test Results**: 25/25 tests passing
- Platform tests: 3/3 ✅
  - `test_v8_platform_initialization`
  - `test_v8_double_initialization`
  - `test_v8_initialized_before_init`
- Runtime tests: 22/22 ✅
  - Execution: 5 tests (simple arithmetic, variables, strings, etc.)
  - Timeout mechanism: 3 tests (configurable, infinite loop, quick execution)
  - Memory limits: 5 tests (enforcement, small/moderate/large allocations)
  - Console integration: 4 tests (output capture, multiple calls, mixed output)
  - Error handling: 5 tests (compilation, runtime, recovery, undefined variables)

**Sub-Milestones Completed**:
- ✅ 1.1: Add rusty_v8 Dependency & Binary Cache Setup
- ✅ 1.2: Global V8 Platform Initialization
- ✅ 1.3: V8Runtime - Isolate Creation & Basic Execution
- ✅ 1.4: Timeout Mechanism with IsolateHandle
- ✅ 1.5: Memory Limit Enforcement
- ✅ 1.6: Integration & Documentation

**Key Features Implemented**:
- One-time global V8 platform initialization with `OnceCell`
- Per-execution isolate creation with configurable limits
- Timeout mechanism using `IsolateHandle` and `TerminateExecution()`
- Memory limit enforcement with near-heap-limit callbacks
- Comprehensive error handling and recovery
- Integration with console output capture (see Part 3)

---

## Development Approach: Test-Driven Development (TDD)

**All sub-milestones follow TDD**:
1. **Write failing test** - Define expected behavior
2. **Implement functionality** - Make test pass
3. **Verify test passes** - Confirm implementation works
4. **Refactor if needed** - Clean up while keeping tests green

**Testing from Scratch**:
- Include script to clean V8 binary cache
- Test from "naked start" to ensure reproducibility
- Validate binary download and caching behavior

---

## Research Summary: rusty_v8 Setup

### Binary Distribution Model

**IMPORTANT**: The V8 C++ JavaScript engine is **statically linked** into the aws-dash executable at build time. End users do NOT download anything at runtime.

## Understanding V8 vs rusty_v8

**V8 Engine**:
- Google's JavaScript engine written in C++ (600,000+ lines of code)
- The actual JavaScript interpreter/compiler
- Must be compiled from C++ source code

**rusty_v8**:
- Rust bindings/wrapper around the V8 C++ API
- Provides safe Rust interface to call V8 functions
- Depends on compiled V8 library

**What Actually Gets Downloaded**:
- Pre-compiled **static library** files (`.a` on Linux/Mac, `.lib` on Windows)
- Example: `librusty_v8_release_x86_64-unknown-linux-gnu.a.gz`
- **Size**: 30-80MB compressed (larger uncompressed)
- **Contents**: The entire V8 C++ engine (already compiled) + rusty_v8 Rust bindings

## During Development (cargo build)

When you run `cargo build`:
1. Build script downloads platform-specific static library from GitHub
2. URL: `https://github.com/denoland/rusty_v8/releases`
3. Files like `librusty_v8_release_x86_64-unknown-linux-gnu.a.gz`
4. Cached in `target/debug/build/rusty_v8-*/` directory
5. Rust linker links this static library into aws-dash executable
6. Final executable contains: Your Rust code + rusty_v8 + V8 C++ engine (all in one)

## For End Users (distributed executable)

- User downloads single `awsdash` executable
- V8 C++ engine is **embedded inside** that executable
- Binary size increase: ~40-60MB (V8 engine + bindings)
- No separate V8 "runtime" or "binary" needed
- No downloads at runtime - completely self-contained
- User runs `./awsdash` - V8 is already there, ready to execute JavaScript

**Building from Source** (Alternative for development):
- Environment variable: `V8_FROM_SOURCE=1 cargo build -vv`
- Requires: Python 3, curl, gn, ninja, clang (auto-downloaded if missing)
- Platform-specific requirements:
  - Linux: `libglib2.0-dev` (Ubuntu: `sudo apt install libglib2.0-dev`)
  - Windows: 64-bit toolchain only
  - macOS: Xcode + Xcode Command Line Tools
- Compile time: Much longer (~30-60 minutes vs ~5 minutes for binary download)

**Recommendation for MVP**: Use automatic binary download (simpler, faster)

**Binary Size Impact**:
- Without V8: ~20-30MB
- With V8: ~60-90MB
- Acceptable trade-off for self-contained distribution

### V8 Initialization Pattern

**From rusty_v8 hello_world.rs example**:

```rust
// 1. Platform creation and initialization
let platform = v8::new_default_platform(0, false).make_shared();
v8::V8::initialize_platform(platform);
v8::V8::initialize();

// 2. Isolate creation
let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

// 3. Context setup
let handle_scope = &mut v8::HandleScope::new(isolate);
let context = v8::Context::new(handle_scope);
let scope = &mut v8::ContextScope::new(handle_scope, context);

// 4. JavaScript execution
let code = v8::String::new(scope, "'Hello' + ' World!'").unwrap();
let script = v8::Script::compile(scope, code, None).unwrap();
let result = script.run(scope).unwrap();
let result = result.to_string(scope).unwrap();

// 5. Extract result to Rust
println!("{}", result.to_rust_string_lossy(scope));
```

**Key Insights**:
- Platform is global, initialized once at app startup
- Isolate is per-execution (in our case, per tool call)
- Handle scopes manage V8 object lifetime (GC safety)
- Context scopes provide execution environment

### Memory Limits

**CreateParams Configuration**:
```rust
let mut params = v8::CreateParams::default();
params = params.heap_limits(
    0,                    // initial heap size (0 = V8 decides)
    256 * 1024 * 1024     // max heap size in bytes (256MB)
);
let isolate = &mut v8::Isolate::new(params);
```

**How Memory Limits Work**:
- V8 starts with small heap, grows dynamically
- When approaching max, performs GC
- Can register `NearHeapLimitCallback` for warnings
- If GC fails to free memory and callback doesn't increase limit → crash with `FatalProcessOutOfMemory`

**Our Approach**: Set 256MB max, let V8 manage initial size

### Timeout/Termination

**IsolateHandle** (thread-safe reference):
```rust
// Get handle before spawning timeout thread
let isolate_handle = isolate.thread_safe_handle();

// From another thread (e.g., timeout watchdog)
isolate_handle.terminate_execution();

// Check if terminating
if isolate_handle.is_execution_terminating() {
    // Handle termination
}

// Optionally resume (if needed)
isolate_handle.cancel_terminate_execution();
```

**Termination Behavior**:
- Throws uncatchable JavaScript exception
- Propagates through all JS frames
- Can be detected in TryCatch blocks
- Requires all JS frames to exit before isolate can be reused

**Our Approach**:
- Spawn timeout thread with `isolate_handle`
- After 30s (configurable), call `terminate_execution()`
- Don't resume - we create fresh isolate per execution anyway

---

## Sub-Milestone Breakdown

### Sub-Milestone 1.1: Add rusty_v8 Dependency & Binary Cache Setup

**Duration**: 2-4 hours

**Objective**: Add rusty_v8 to Cargo.toml, verify binary download, create cache management script

#### Tasks

**1.1.1: Add Dependency**
- Add `rusty_v8` to `Cargo.toml`
- Research latest stable version on crates.io
- Add to dependencies with appropriate features

**1.1.2: Verify Binary Download**
- Run `cargo build` and observe binary download
- Confirm binary cached in `target/` directory
- Document binary location and size

**1.1.3: Verify Static Linking**
- Confirm V8 is statically linked into executable
- Check binary size increase (~40-60MB)
- Verify no runtime dependencies on V8 libraries

**1.1.4: Create Cleanup Script (Development Only)**
```bash
#!/bin/bash
# scripts/clean-v8.sh
# Cleans V8 build artifacts for testing from scratch
# NOTE: This is for DEVELOPMENT ONLY - end users don't need this

echo "Cleaning V8 build artifacts (development only)..."

# Remove rusty_v8 build artifacts from target/
cargo clean -p rusty_v8

# Remove downloaded static libraries from target/
rm -rf target/debug/gn_root
rm -rf target/debug/build/v8-*
rm -rf target/debug/build/rusty_v8-*

echo "V8 build cache cleaned. Next build will download fresh static library."
echo "NOTE: End users don't need this - V8 is embedded in distributed executable."
```

**1.1.5: Test Script**
```bash
# Test 1: Build normally
cargo build

# Test 2: Clean and rebuild from scratch
./scripts/clean-v8.sh
cargo build

# Verify: Second build should re-download V8 binary
```

#### TDD Approach

**Test (Cargo.toml validation)**:
- Verify `rusty_v8` in dependencies
- Verify `cargo build` succeeds

**Implementation**:
- Add dependency
- Create cleanup script

**Verification**:
- Run cleanup script
- Build from scratch
- Confirm binary download happens

#### Acceptance Criteria
- [ ] rusty_v8 added to Cargo.toml
- [ ] Project compiles successfully (V8 static library downloaded)
- [ ] Binary size increased by ~40-60MB (confirms V8 is linked)
- [ ] Cleanup script created and tested (development only)
- [ ] Can rebuild from "naked start" reliably
- [ ] Understand V8 is embedded in final executable (no runtime download)

---

### Sub-Milestone 1.2: Global V8 Platform Initialization

**Duration**: 4-6 hours

**Objective**: Initialize V8 platform at application startup, create singleton for global access

#### Tasks

**1.2.1: Create V8 Platform Manager**

File: `src/app/agent_framework/v8_bindings/platform.rs`

```rust
//! Global V8 Platform Management
//!
//! The V8 platform must be initialized once at application startup
//! and remain alive for the entire application lifetime.

use once_cell::sync::OnceCell;
use std::sync::Arc;
use v8::{Platform, V8};

static GLOBAL_V8_PLATFORM: OnceCell<Arc<dyn Platform>> = OnceCell::new();

/// Initialize the V8 platform
///
/// Must be called exactly once at application startup.
/// Thread-safe via OnceCell.
///
/// # Errors
/// Returns error if V8 platform already initialized
pub fn initialize_v8_platform() -> Result<(), String> {
    GLOBAL_V8_PLATFORM.get_or_try_init(|| {
        // Create platform with default parameters
        // Parameters: (thread_pool_size, idle_task_support)
        // 0 = use default thread pool size
        // false = no idle task support (not needed for our use case)
        let platform = v8::new_default_platform(0, false).make_shared();

        // Initialize V8 with platform
        V8::initialize_platform(platform.clone());
        V8::initialize();

        info!("✅ V8 platform initialized successfully");

        Ok(platform)
    })?;

    Ok(())
}

/// Check if V8 platform is initialized
pub fn is_v8_initialized() -> bool {
    GLOBAL_V8_PLATFORM.get().is_some()
}

/// Shutdown V8 platform
///
/// Should be called on application exit.
/// Only for clean shutdown, not required in most cases.
pub unsafe fn dispose_v8_platform() {
    if GLOBAL_V8_PLATFORM.get().is_some() {
        // V8::dispose() is unsafe and should only be called on shutdown
        V8::dispose();
        info!("🛑 V8 platform disposed");
    }
}
```

**1.2.2: Add to Module Exports**

File: `src/app/agent_framework/v8_bindings/mod.rs`

```rust
pub mod platform;

pub use platform::{initialize_v8_platform, is_v8_initialized};
```

**1.2.3: Initialize at Application Startup**

File: `src/main.rs` (modify existing startup sequence)

```rust
// After logging initialization, before UI setup
info!("Initializing V8 JavaScript engine...");
if let Err(e) = awsdash::app::agent_framework::v8_bindings::initialize_v8_platform() {
    error!("Failed to initialize V8 platform: {}", e);
    std::process::exit(1);
}
```

**1.2.4: Add Shutdown Hook** (Optional for clean exit)

```rust
// In main.rs, before final return
unsafe {
    awsdash::app::agent_framework::v8_bindings::dispose_v8_platform();
}
```

#### TDD Approach

**Test 1: Initialization Test**

File: `src/app/agent_framework/v8_bindings/platform_test.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v8_platform_initialization() {
        // Test: Platform should initialize successfully
        let result = initialize_v8_platform();
        assert!(result.is_ok(), "V8 platform initialization failed");

        // Verify platform is initialized
        assert!(is_v8_initialized(), "V8 platform not marked as initialized");
    }

    #[test]
    fn test_v8_double_initialization() {
        // Test: Second initialization should succeed (idempotent)
        let result1 = initialize_v8_platform();
        let result2 = initialize_v8_platform();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        // OnceCell ensures only one actual initialization
    }
}
```

**Implementation**: Create platform.rs as specified above

**Verification**:
```bash
cargo test test_v8_platform
```

#### Acceptance Criteria
- [ ] V8 platform initializes at app startup
- [ ] Initialization is idempotent (safe to call multiple times)
- [ ] Application fails fast if V8 init fails
- [ ] Tests pass for initialization
- [ ] Platform remains alive for app lifetime

---

### Sub-Milestone 1.3: V8Runtime - Isolate Creation & Basic Execution

**Duration**: 8-12 hours

**Objective**: Create V8Runtime struct that manages isolate lifecycle and executes JavaScript code

#### Tasks

**1.3.1: Define V8Runtime Structure**

File: `src/app/agent_framework/v8_bindings/runtime.rs`

```rust
//! V8 Runtime for JavaScript Execution
//!
//! Manages V8 isolate lifecycle, execution, and result extraction.

use anyhow::{anyhow, Context, Result};
use std::time::{Duration, Instant};
use v8::{HandleScope, Isolate, TryCatch};

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
    pub fn execute(&self, code: &str) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        // Create isolate with memory limits
        let mut params = v8::CreateParams::default();
        params = params.heap_limits(0, self.config.max_heap_size_bytes);

        let isolate = &mut v8::Isolate::new(params);

        // Create handle scope for GC management
        let handle_scope = &mut v8::HandleScope::new(isolate);

        // Create context
        let context = v8::Context::new(handle_scope);
        let scope = &mut v8::ContextScope::new(handle_scope, context);

        // TODO: Register bound functions (console, AWS functions)
        // Will be implemented in later sub-milestones

        // Compile JavaScript
        let code_str = v8::String::new(scope, code)
            .ok_or_else(|| anyhow!("Failed to create V8 string from code"))?;

        let try_catch = &mut v8::TryCatch::new(scope);

        let script = match v8::Script::compile(try_catch, code_str, None) {
            Some(script) => script,
            None => {
                // Compilation error
                let exception = try_catch.exception().unwrap();
                let exception_str = exception.to_string(try_catch).unwrap();
                let stderr = exception_str.to_rust_string_lossy(try_catch);

                return Ok(ExecutionResult {
                    success: false,
                    result: None,
                    stdout: String::new(),
                    stderr,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }
        };

        // Execute JavaScript
        let result = match script.run(try_catch) {
            Some(result) => result,
            None => {
                // Runtime error
                let exception = try_catch.exception().unwrap();
                let exception_str = exception.to_string(try_catch).unwrap();
                let stderr = exception_str.to_rust_string_lossy(try_catch);

                return Ok(ExecutionResult {
                    success: false,
                    result: None,
                    stdout: String::new(),
                    stderr,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }
        };

        // Extract result as JSON
        let result_str = result.to_string(try_catch).unwrap();
        let result_json = result_str.to_rust_string_lossy(try_catch);

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            success: true,
            result: Some(result_json),
            stdout: String::new(), // TODO: Capture console output
            stderr: String::new(),
            execution_time_ms,
        })
    }
}

impl Default for V8Runtime {
    fn default() -> Self {
        Self::new()
    }
}
```

**1.3.2: Add Module Exports**

File: `src/app/agent_framework/v8_bindings/mod.rs`

```rust
pub mod platform;
pub mod runtime;

pub use platform::{initialize_v8_platform, is_v8_initialized};
pub use runtime::{ExecutionResult, RuntimeConfig, V8Runtime};
```

**1.3.3: Create Module Root**

File: `src/app/agent_framework/v8_bindings/mod.rs` (if not exists)

#### TDD Approach

**Test 1: Basic Execution**

File: `tests/v8_runtime_basic_test.rs`

```rust
//! Basic V8 Runtime Tests
//!
//! Tests fundamental JavaScript execution capabilities

use awsdash::app::agent_framework::v8_bindings::{initialize_v8_platform, V8Runtime};

#[test]
fn test_simple_arithmetic() {
    // Ensure V8 is initialized
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let result = runtime.execute("2 + 2").unwrap();

    assert!(result.success, "Execution should succeed");
    assert_eq!(result.result, Some("4".to_string()));
    assert!(result.execution_time_ms > 0);
}

#[test]
fn test_string_concatenation() {
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let result = runtime.execute("'Hello' + ' ' + 'World'").unwrap();

    assert!(result.success);
    assert_eq!(result.result, Some("Hello World".to_string()));
}

#[test]
fn test_object_return() {
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let result = runtime.execute("const obj = {name: 'test', value: 42}; obj").unwrap();

    assert!(result.success);
    // Result should be JSON representation of object
    assert!(result.result.unwrap().contains("test"));
    assert!(result.result.unwrap().contains("42"));
}

#[test]
fn test_array_return() {
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let result = runtime.execute("[1, 2, 3, 4, 5]").unwrap();

    assert!(result.success);
    assert!(result.result.unwrap().contains("1,2,3,4,5"));
}
```

**Test 2: Error Handling**

```rust
#[test]
fn test_syntax_error() {
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let result = runtime.execute("const x = {").unwrap(); // Syntax error

    assert!(!result.success, "Should fail on syntax error");
    assert!(result.result.is_none());
    assert!(!result.stderr.is_empty(), "Should have error message");
    assert!(result.stderr.contains("SyntaxError") || result.stderr.contains("Unexpected"));
}

#[test]
fn test_runtime_error() {
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let result = runtime.execute("const x = null; x.property").unwrap(); // Runtime error

    assert!(!result.success);
    assert!(result.result.is_none());
    assert!(!result.stderr.is_empty());
    assert!(result.stderr.contains("TypeError") || result.stderr.contains("null"));
}

#[test]
fn test_reference_error() {
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let result = runtime.execute("nonExistentVariable").unwrap();

    assert!(!result.success);
    assert!(result.stderr.contains("ReferenceError") || result.stderr.contains("not defined"));
}
```

**Test 3: Multi-line Code**

```rust
#[test]
fn test_multiline_code() {
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let code = r#"
        const numbers = [1, 2, 3, 4, 5];
        const sum = numbers.reduce((a, b) => a + b, 0);
        sum;
    "#;
    let result = runtime.execute(code).unwrap();

    assert!(result.success);
    assert_eq!(result.result, Some("15".to_string()));
}

#[test]
fn test_function_definition_and_call() {
    let _ = initialize_v8_platform();

    let runtime = V8Runtime::new();
    let code = r#"
        function add(a, b) {
            return a + b;
        }
        add(10, 20);
    "#;
    let result = runtime.execute(code).unwrap();

    assert!(result.success);
    assert_eq!(result.result, Some("30".to_string()));
}
```

**Implementation**: Create runtime.rs as specified above

**Verification**:
```bash
cargo test --test v8_runtime_basic_test
```

#### Acceptance Criteria
- [ ] V8Runtime struct created
- [ ] Can execute simple JavaScript expressions
- [ ] Returns correct results
- [ ] Handles syntax errors gracefully
- [ ] Handles runtime errors gracefully
- [ ] Captures execution time
- [ ] All basic tests pass

---

### Sub-Milestone 1.4: Timeout Mechanism with IsolateHandle

**Duration**: 4-6 hours

**Objective**: Implement execution timeout using thread-safe IsolateHandle

#### Tasks

**1.4.1: Add Timeout Logic to V8Runtime**

Update `src/app/agent_framework/v8_bindings/runtime.rs`:

```rust
use std::sync::{Arc, Mutex};
use std::thread;

impl V8Runtime {
    /// Execute JavaScript code with timeout enforcement
    pub fn execute(&self, code: &str) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        // Create isolate with memory limits
        let mut params = v8::CreateParams::default();
        params = params.heap_limits(0, self.config.max_heap_size_bytes);

        let isolate = &mut v8::Isolate::new(params);

        // Get thread-safe handle for timeout enforcement
        let isolate_handle = isolate.thread_safe_handle();
        let timeout = self.config.timeout;

        // Spawn timeout watchdog thread
        let timeout_thread = thread::spawn(move || {
            thread::sleep(timeout);
            // Terminate execution after timeout
            isolate_handle.terminate_execution();
        });

        // ... rest of execution code ...

        // Check if execution was terminated by timeout
        if isolate_handle.is_execution_terminating() {
            return Ok(ExecutionResult {
                success: false,
                result: None,
                stdout: String::new(),
                stderr: format!("Execution timeout after {:?}", timeout),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Normal execution continues...
        // (existing code from 1.3.1)

        // Note: timeout_thread will be cleaned up automatically
        // We don't join() because execution completed before timeout
    }
}
```

**1.4.2: Handle Termination in TryCatch**

```rust
// After script.run() attempt:
if try_catch.has_terminated() {
    return Ok(ExecutionResult {
        success: false,
        result: None,
        stdout: String::new(), // TODO: Capture partial output
        stderr: format!("Execution terminated (timeout: {:?})", self.config.timeout),
        execution_time_ms: start_time.elapsed().as_millis() as u64,
    });
}
```

#### TDD Approach

**Test 1: Quick Execution (No Timeout)**

File: `tests/v8_runtime_timeout_test.rs`

```rust
use awsdash::app::agent_framework::v8_bindings::{initialize_v8_platform, RuntimeConfig, V8Runtime};
use std::time::Duration;

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
    assert_eq!(result.result, Some("4".to_string()));
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
    assert!(result.stderr.contains("timeout") || result.stderr.contains("terminated"));
    assert!(result.execution_time_ms >= 100); // Should hit timeout
    assert!(result.execution_time_ms < 200); // Should terminate quickly after timeout
}

#[test]
fn test_long_computation_timeout() {
    let _ = initialize_v8_platform();

    let config = RuntimeConfig {
        timeout: Duration::from_millis(200),
        ..Default::default()
    };

    let runtime = V8Runtime::with_config(config);
    let code = r#"
        let sum = 0;
        for (let i = 0; i < 100000000; i++) {
            sum += i;
        }
        sum;
    "#;
    let result = runtime.execute(code).unwrap();

    // May or may not timeout depending on machine speed
    // But should not crash or hang
    assert!(result.execution_time_ms < 500);
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
```

**Implementation**: Update runtime.rs with timeout logic

**Verification**:
```bash
cargo test --test v8_runtime_timeout_test
```

#### Acceptance Criteria
- [ ] Timeout mechanism implemented with IsolateHandle
- [ ] Quick executions complete without timeout
- [ ] Infinite loops are terminated after timeout
- [ ] Timeout duration is configurable
- [ ] Timeout errors are reported clearly
- [ ] All timeout tests pass

---

### Sub-Milestone 1.5: Memory Limit Enforcement

**Duration**: 2-4 hours

**Objective**: Verify memory limits work correctly and handle OOM gracefully

#### Tasks

**1.5.1: Verify CreateParams Memory Configuration**

Already implemented in 1.3.1, verify it's working:

```rust
let mut params = v8::CreateParams::default();
params = params.heap_limits(0, self.config.max_heap_size_bytes);
let isolate = &mut v8::Isolate::new(params);
```

**1.5.2: Add Memory Limit to RuntimeConfig**

Already done in 1.3.1:
```rust
pub max_heap_size_bytes: usize, // default: 256MB
```

**1.5.3: Handle Out-of-Memory Errors**

V8 will throw exception when OOM occurs. The existing TryCatch logic should handle it.

#### TDD Approach

**Test 1: Small Memory Allocation (Within Limits)**

File: `tests/v8_runtime_memory_test.rs`

```rust
use awsdash::app::agent_framework::v8_bindings::{initialize_v8_platform, RuntimeConfig, V8Runtime};

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
    assert_eq!(result.result, Some("1000".to_string()));
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
    assert_eq!(result.result, Some("100000".to_string()));
}

#[test]
#[ignore] // This test may be slow or cause system issues
fn test_excessive_allocation_fails() {
    let _ = initialize_v8_platform();

    // Use smaller memory limit for testing OOM
    let config = RuntimeConfig {
        max_heap_size_bytes: 10 * 1024 * 1024, // 10MB limit
        ..Default::default()
    };

    let runtime = V8Runtime::with_config(config);
    let code = r#"
        // Try to allocate more than 10MB
        const arr = new Array(1000000).fill({
            data: 'x'.repeat(1000) // 1KB per element
        });
        arr.length;
    "#;
    let result = runtime.execute(code).unwrap();

    // Should fail with OOM or similar error
    assert!(!result.success);
    assert!(result.stderr.contains("memory") || result.stderr.contains("out of"));
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
    let result1 = runtime_small.execute("new Array(1000).fill(42)").unwrap();
    let result2 = runtime_large.execute("new Array(1000).fill(42)").unwrap();

    assert!(result1.success);
    assert!(result2.success);
}
```

**Implementation**: Memory limits already implemented, just verify with tests

**Verification**:
```bash
cargo test --test v8_runtime_memory_test
```

#### Acceptance Criteria
- [ ] Memory limits enforced via CreateParams
- [ ] Small allocations succeed
- [ ] Moderate allocations succeed
- [ ] Excessive allocations fail gracefully (for test with small limit)
- [ ] Memory limit is configurable
- [ ] All memory tests pass

---

### Sub-Milestone 1.6: Integration & Documentation

**Duration**: 2-3 hours

**Objective**: Integrate all sub-milestones, document usage, create comprehensive test suite

#### Tasks

**1.6.1: Create Comprehensive Integration Test**

File: `tests/v8_integration_test.rs`

```rust
//! V8 Integration Tests
//!
//! Tests complete V8 infrastructure setup and execution

use awsdash::app::agent_framework::v8_bindings::{
    initialize_v8_platform, is_v8_initialized, RuntimeConfig, V8Runtime
};
use std::time::Duration;

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

        // Return as object
        ({
            input: 10,
            output: result,
            method: 'recursive'
        });
    "#;

    let result = runtime.execute(code).unwrap();

    // 4. Verify result
    assert!(result.success);
    assert!(result.result.is_some());

    let result_str = result.result.unwrap();
    assert!(result_str.contains("55")); // fibonacci(10) = 55

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
        assert_eq!(result.result, Some(format!("{}", i * 2)));
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
```

**1.6.2: Create Usage Documentation**

File: `src/app/agent_framework/v8_bindings/README.md`

```markdown
# V8 Bindings Module

JavaScript execution engine for agent framework using rusty_v8.

## Architecture

- **Platform**: Global V8 platform initialized at app startup
- **Runtime**: Per-execution isolate with configurable limits
- **Isolation**: Fresh isolate created for each execution (no state persistence)

## Usage

### Initialize Platform (at app startup)

```rust
use awsdash::app::agent_framework::v8_bindings::initialize_v8_platform;

fn main() {
    // Initialize V8 platform (required once at startup)
    initialize_v8_platform().expect("Failed to initialize V8");

    // ... rest of application
}
```

### Execute JavaScript

```rust
use awsdash::app::agent_framework::v8_bindings::{V8Runtime, RuntimeConfig};
use std::time::Duration;

// Create runtime with default config (256MB, 30s timeout)
let runtime = V8Runtime::new();

// Execute code
let result = runtime.execute("2 + 2").unwrap();

if result.success {
    println!("Result: {:?}", result.result);
} else {
    eprintln!("Error: {}", result.stderr);
}
```

### Custom Configuration

```rust
let config = RuntimeConfig {
    max_heap_size_bytes: 512 * 1024 * 1024, // 512MB
    timeout: Duration::from_secs(60), // 60 seconds
    capture_console: true,
};

let runtime = V8Runtime::with_config(config);
```

## Security

- **Memory Limit**: Enforced at V8 level (default 256MB)
- **Timeout**: Enforced via IsolateHandle termination (default 30s)
- **Isolation**: Fresh isolate per execution, no state leakage
- **Sandboxing**: No Node.js APIs, only bound Rust functions (added later)

## Testing

```bash
# Run all V8 tests
cargo test --workspace -- v8

# Run specific test suites
cargo test --test v8_runtime_basic_test
cargo test --test v8_runtime_timeout_test
cargo test --test v8_runtime_memory_test
cargo test --test v8_integration_test

# Clean V8 cache and test from scratch
./scripts/clean-v8.sh
cargo test --workspace
```

## Troubleshooting

### V8 Binary Download Fails

```bash
# Clean and rebuild
./scripts/clean-v8.sh
cargo clean
cargo build
```

### Memory Limit Errors

Increase `max_heap_size_bytes` in RuntimeConfig.

### Timeout Errors

Increase `timeout` duration in RuntimeConfig.
```

**1.6.3: Update Main Documentation**

Add to `TODOS/CODE_EXECUTION_TOOL.md`:

```markdown
## Milestone 1: V8 Infrastructure Setup - ✅ COMPLETE

**Detailed Implementation**: See [CODE_EXECUTION_TOOL_PART_2.md](./CODE_EXECUTION_TOOL_PART_2.md)

**Summary**:
- rusty_v8 dependency added
- Global V8 platform initialized at app startup
- V8Runtime created with configurable memory/timeout
- Execution with error handling
- Timeout mechanism via IsolateHandle
- Memory limits enforced
- Comprehensive test suite
- Cleanup script for testing from scratch

**Key Files**:
- `src/app/agent_framework/v8_bindings/platform.rs` - Global platform
- `src/app/agent_framework/v8_bindings/runtime.rs` - Execution runtime
- `scripts/clean-v8.sh` - Cache cleanup script
- `tests/v8_*_test.rs` - Test suites
```

**1.6.4: Final Verification Checklist**

```bash
#!/bin/bash
# scripts/verify-milestone-1.sh

echo "🔍 Verifying Milestone 1: V8 Infrastructure Setup"
echo ""

echo "1. Cleaning V8 cache..."
./scripts/clean-v8.sh

echo ""
echo "2. Building from scratch..."
cargo build || exit 1

echo ""
echo "3. Running platform tests..."
cargo test test_v8_platform || exit 1

echo ""
echo "4. Running basic execution tests..."
cargo test --test v8_runtime_basic_test || exit 1

echo ""
echo "5. Running timeout tests..."
cargo test --test v8_runtime_timeout_test || exit 1

echo ""
echo "6. Running memory tests..."
cargo test --test v8_runtime_memory_test || exit 1

echo ""
echo "7. Running integration tests..."
cargo test --test v8_integration_test || exit 1

echo ""
echo "✅ Milestone 1 verification complete!"
echo ""
echo "Next: Milestone 2 - Console Binding"
```

#### Acceptance Criteria
- [ ] Integration test passes
- [ ] Usage documentation created
- [ ] Main doc updated with references
- [ ] Verification script created and passes
- [ ] All sub-milestones complete
- [ ] Ready for Milestone 2

---

## Milestone 1 Complete!

### Deliverables
1. ✅ rusty_v8 dependency configured
2. ✅ V8 platform initialization (global, at startup)
3. ✅ V8Runtime with isolate management
4. ✅ JavaScript execution with error handling
5. ✅ Timeout enforcement (30s default, configurable)
6. ✅ Memory limits (256MB default, configurable)
7. ✅ Comprehensive test suite (basic, timeout, memory, integration)
8. ✅ Cleanup script for testing from scratch
9. ✅ Documentation (usage, troubleshooting)

### Test Coverage
- Platform initialization and lifecycle
- Basic JavaScript execution (arithmetic, strings, objects, arrays)
- Error handling (syntax, runtime, reference errors)
- Timeout enforcement (infinite loops, long computations)
- Memory limits (small, moderate, excessive allocations)
- Integration (complete workflow, multiple executions, error recovery)

### Estimated vs Actual
- **Estimated**: 2-3 days
- **Actual**: (To be measured during implementation)

### Next Steps
**Milestone 2**: Console Binding - See [CODE_EXECUTION_TOOL_PART_3.md](./CODE_EXECUTION_TOOL_PART_3.md)

---

## References

### Internal Documentation
- **Main Planning**: [CODE_EXECUTION_TOOL.md](./CODE_EXECUTION_TOOL.md)
- **Milestone 2**: [CODE_EXECUTION_TOOL_PART_3.md](./CODE_EXECUTION_TOOL_PART_3.md)
- **🎯 V8 API Reference**: [RUSTY_V8_142_API_REFERENCE.md](./RUSTY_V8_142_API_REFERENCE.md) *(Critical for all V8 implementation - Read this first!)*
- **Decisions Reference**:
  - Decision 12: Security & Resource Limits
  - Decision 13: JavaScript Execution Mechanics
  - Decision 14: Integration with Existing Agent System

### External Documentation
- **rusty_v8 Crate**: https://crates.io/crates/v8
- **rusty_v8 Docs**: https://docs.rs/v8/142.0.0/v8/
- **rusty_v8 GitHub**: https://github.com/denoland/rusty_v8
- **rusty_v8 Tests**: https://github.com/denoland/rusty_v8/blob/v142.1.0/tests/test_api.rs
- **V8 C++ Docs**: https://v8.dev/docs
- **rusty_v8 Examples**: https://github.com/denoland/rusty_v8/tree/main/examples

---

**End of Part 2 - Milestone 1: V8 Infrastructure Setup**
