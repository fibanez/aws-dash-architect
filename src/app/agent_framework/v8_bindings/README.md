# V8 JavaScript Engine Bindings

This module provides V8 JavaScript execution capabilities for the agent framework, enabling agents to execute JavaScript code in isolated environments with configurable resource limits.

## Architecture

The V8 bindings follow a two-level architecture:

1. **Global Platform** (`platform.rs`) - One-time initialization at application startup
2. **Per-Execution Runtime** (`runtime.rs`) - Fresh isolate for each JavaScript execution

## Quick Start

### Initialize V8 Platform

Call once at application startup (already integrated in `main.rs`):

```rust
use awsdash::app::agent_framework::initialize_v8_platform;

fn main() {
    initialize_v8_platform().expect("Failed to initialize V8");
    // ... rest of application
}
```

### Execute JavaScript Code

```rust
use awsdash::app::agent_framework::v8_bindings::{V8Runtime, RuntimeConfig};
use std::time::Duration;

// Create runtime with default configuration (256MB, 30s timeout)
let runtime = V8Runtime::new();

// Execute JavaScript
let result = runtime.execute("2 + 2").expect("Execution failed");

if result.success {
    println!("Result: {}", result.result.unwrap()); // "4"
    println!("Execution time: {}ms", result.execution_time_ms);
} else {
    eprintln!("Error: {}", result.stderr);
}
```

### Custom Configuration

```rust
let config = RuntimeConfig {
    max_heap_size_bytes: 128 * 1024 * 1024, // 128MB
    timeout: Duration::from_secs(10),        // 10 second timeout
    capture_console: true,                   // Enable console capture
};

let runtime = V8Runtime::with_config(config);
```

## API Reference

### Platform Management (`platform.rs`)

#### `initialize_v8_platform() -> Result<(), String>`

Initializes the global V8 platform. Thread-safe and idempotent - multiple calls are safe.

**Call this once at application startup before creating any V8 runtimes.**

#### `is_v8_initialized() -> bool`

Returns `true` if V8 platform has been initialized.

#### `unsafe fn dispose_v8_platform()`

Optionally dispose the V8 platform on shutdown. Only call when no isolates are active.

### Runtime Execution (`runtime.rs`)

#### `struct V8Runtime`

JavaScript runtime with configurable resource limits.

**Methods:**
- `new()` - Create runtime with default configuration
- `with_config(config: RuntimeConfig)` - Create with custom configuration
- `execute(&self, code: &str) -> Result<ExecutionResult>` - Execute JavaScript code

#### `struct RuntimeConfig`

Configuration for V8 runtime execution.

**Fields:**
- `max_heap_size_bytes: usize` - Maximum heap size (default: 256MB)
- `timeout: Duration` - Execution timeout (default: 30 seconds)
- `capture_console: bool` - Enable console output capture (default: true, not yet implemented)

#### `struct ExecutionResult`

Result of JavaScript execution.

**Fields:**
- `success: bool` - Whether execution succeeded
- `result: Option<String>` - Return value as string (None if error)
- `stdout: String` - Captured console output (not yet implemented)
- `stderr: String` - Error messages and exceptions
- `execution_time_ms: u64` - Execution time in milliseconds

## Resource Limits

### Memory Limits

Each V8 isolate has a configurable heap size limit:

```rust
let config = RuntimeConfig {
    max_heap_size_bytes: 512 * 1024 * 1024, // 512MB
    ..Default::default()
};
```

**Note:** V8 memory limits are advisory. The engine may allocate slightly more memory for internal structures.

### Timeout Enforcement

Infinite loops and long-running code are terminated after the configured timeout:

```rust
let config = RuntimeConfig {
    timeout: Duration::from_millis(100), // 100ms timeout
    ..Default::default()
};

let runtime = V8Runtime::with_config(config);
let result = runtime.execute("while(true) {}").unwrap();

assert!(!result.success);
assert!(result.stderr.contains("timeout") || result.stderr.contains("terminated"));
```

## Error Handling

JavaScript errors are captured and returned in `ExecutionResult`:

```rust
let runtime = V8Runtime::new();

// Compilation error
let result = runtime.execute("const x = ;").unwrap();
assert!(!result.success);
assert!(result.stderr.contains("Unexpected token"));

// Runtime error
let result = runtime.execute("throw new Error('test');").unwrap();
assert!(!result.success);
assert!(result.stderr.contains("test"));

// Undefined variable
let result = runtime.execute("nonExistent").unwrap();
assert!(!result.success);
assert!(result.stderr.contains("not defined"));
```

## Usage Examples

### Simple Arithmetic

```rust
let runtime = V8Runtime::new();
let result = runtime.execute("2 + 2").unwrap();
assert_eq!(result.result.unwrap(), "4");
```

### Variables and Functions

```rust
let code = r#"
    function fibonacci(n) {
        if (n <= 1) return n;
        return fibonacci(n - 1) + fibonacci(n - 2);
    }
    fibonacci(10);
"#;

let result = runtime.execute(code).unwrap();
assert_eq!(result.result.unwrap(), "55");
```

### Error Recovery

Each execution uses a fresh isolate, so errors don't affect subsequent executions:

```rust
let runtime = V8Runtime::new();

// Execute valid code
assert!(runtime.execute("42").unwrap().success);

// Execute invalid code
assert!(!runtime.execute("invalid {").unwrap().success);

// Execute valid code again (works fine)
assert!(runtime.execute("100").unwrap().success);
```

## Implementation Details

### V8 142.x Scope Management

This implementation uses V8 142.x which requires proper scope pinning:

```rust
use std::pin::pin;

let scope = pin!(v8::HandleScope::new(&mut isolate));
let scope = &mut scope.init();
```

### Timeout Mechanism

Timeouts are enforced via a watchdog thread:

1. Get thread-safe `IsolateHandle` before execution
2. Spawn watchdog thread that sleeps for timeout duration
3. Watchdog calls `isolate_handle.terminate_execution()`
4. Execution checks `scope.has_terminated()` and returns timeout error

### Static Linking

The V8 library (~154MB) is statically linked into the executable at build time via the `rusty_v8` crate. No separate V8 installation is required.

## Testing

The module includes comprehensive tests:

- **Basic execution** - Arithmetic, variables, strings
- **Error handling** - Compilation errors, runtime errors, undefined variables
- **Timeout enforcement** - Infinite loop termination
- **Memory limits** - Heap size configuration
- **Integration tests** - Complete workflows, multiple executions, error recovery

Run tests:
```bash
cargo test --lib v8_bindings::runtime::tests
cargo test --lib v8_bindings::platform::tests
```

## Future Enhancements

Planned features for upcoming milestones:

1. **Console Output Capture** - Implement `capture_console` functionality
2. **Rust Function Bindings** - Bind Rust functions to JavaScript global scope
3. **AWS SDK Integration** - Bind AWS operations for infrastructure code execution
4. **Enhanced Error Context** - Stack traces and line numbers for errors

## Dependencies

- `v8 = "142.0.0"` - Rust bindings to V8 JavaScript engine
- `anyhow` - Error handling
- `log` - Logging
- `once_cell` - Thread-safe one-time initialization

## Thread Safety

- **Platform initialization** - Thread-safe via `OnceCell`, idempotent
- **Runtime execution** - Each execution creates a fresh isolate
- **Timeout watchdog** - Uses thread-safe `IsolateHandle`

## Performance

- **Initialization overhead** - Platform initialization is one-time (~10ms)
- **Per-execution overhead** - Fresh isolate creation (~1-2ms)
- **Execution speed** - V8 JIT compilation provides near-native performance
- **Memory footprint** - Configurable per-isolate (default 256MB)

## Logging

V8 platform initialization is logged to the application log file:

```
[timestamp] INFO V8 platform initialized successfully
```

Log file location: `$HOME/.local/share/awsdash/logs/awsdash.log`
