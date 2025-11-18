# CODE EXECUTION TOOL IMPLEMENTATION - PART 3
# MILESTONE 2: CONSOLE BINDING

**Created**: 2025-11-14
**Last Updated**: 2025-01-14
**Status**: ✅ COMPLETE - All Sub-Milestones Implemented and Tested
**Test Results**: 9/9 tests passing
**Parent Document**: [CODE_EXECUTION_TOOL.md](./CODE_EXECUTION_TOOL.md)

---

## Overview

This document provides detailed specifications and implementation steps for **Milestone 2: Console Binding**, implementing console.log/error/warn/debug as Rust-bound functions that capture output for agent inspection.

**Reference Documents**:
- **Parent Planning**: [CODE_EXECUTION_TOOL.md](./CODE_EXECUTION_TOOL.md)
- **Milestone 1**: [CODE_EXECUTION_TOOL_PART_2.md](./CODE_EXECUTION_TOOL_PART_2.md)
- **🎯 V8 API Reference**: [RUSTY_V8_142_API_REFERENCE.md](./RUSTY_V8_142_API_REFERENCE.md) *(CRITICAL - Read before implementation)*

---

## ⚠️ IMPORTANT: Required Reading for LLM Implementers

Before implementing console bindings, **you MUST read and understand**:

### Primary Reference: RUSTY_V8_142_API_REFERENCE.md

**Location**: `TODOS/RUSTY_V8_142_API_REFERENCE.md`

**Why This is Critical**:
- rusty_v8 v142.x has **significant API changes** from earlier versions
- Function callbacks use **`&mut v8::PinScope`**, not `&mut v8::HandleScope`
- Scopes require **pinning with `pin!()` macro** and `.init()` calls
- External data pattern has specific lifetime requirements

**Key Sections to Reference**:
1. **Function Callback API** (Section 2) - Correct callback signatures
2. **Scope Management** (Section 3) - HandleScope, PinScope, ContextScope patterns
3. **External Data Pattern** (Section 6) - Passing Rust data to JavaScript callbacks
4. **rusty_v8 Test Suite Examples** (Section 10) - Real implementation examples:
   - Example 2: "Callback with External Data" - Shows console.log pattern
   - Example 4: "Reading Arguments" - Shows how to read JS arguments
5. **Common Pitfalls** (Section 11) - Avoid known mistakes

**When to Reference**:
- ✅ Before writing any callback function signature
- ✅ Before creating External data for ConsoleBuffers
- ✅ When encountering compilation errors with scopes
- ✅ When converting JS values to Rust strings
- ✅ If unsure about any V8 API usage

---

## Development Approach: Test-Driven Development (TDD)

**All sub-milestones follow TDD**:
1. **Write failing test** - Define expected behavior
2. **Implement functionality** - Make test pass
3. **Verify test passes** - Confirm implementation works
4. **Refactor if needed** - Clean up while keeping tests green

---

## ✅ IMPLEMENTATION COMPLETE

**Completion Date**: 2025-01-14
**Implementation Files**:
- `src/app/agent_framework/v8_bindings/console.rs` - Complete console binding implementation

**Test Results**: 9/9 tests passing ✅
- `test_console_buffers_creation` - ConsoleBuffers creation and accessors
- `test_console_buffers_clear` - Buffer clearing functionality
- `test_console_log_binding` - console.log() captures to stdout
- `test_console_error_binding` - console.error() captures to stderr
- `test_console_warn_binding` - console.warn() captures to stdout
- `test_console_debug_binding` - console.debug() captures to stderr
- `test_console_multiple_arguments` - Multiple argument formatting
- `test_console_mixed_output` - Mixed stdout/stderr capture
- `test_v8_javascript_demo` - Full integration test

**Sub-Milestones Completed**:
- ✅ 2.1: ConsoleBuffers Structure
- ✅ 2.2: Console Function Callbacks
- ✅ 2.3: Integration with V8Runtime
- ✅ 2.4: Update Module Exports
- ✅ 2.5: Documentation and Verification

**Key Features Implemented**:
- `ConsoleBuffers` with Rc<RefCell<String>> for shared mutable access
- Four console functions: log, error, warn, debug
- Proper V8 External data pattern for passing Rust data to callbacks
- Multiple argument support with space-separated formatting
- Stdout/stderr separation (log/warn → stdout, error/debug → stderr)
- Integration with V8Runtime via `register_console()` function
- Comprehensive tests covering all functionality

---

## Milestone 2: Console Binding (~1 day)

### Goal

Implement console.log/error/warn/debug as Rust-bound functions that capture output to in-memory buffers, enabling the agent to inspect JavaScript console output.

### Why This Matters

Console functions are the primary debugging and output mechanism in JavaScript. By capturing console output:
- Agents can see what their JavaScript code is doing
- Error messages become visible for troubleshooting
- Debugging becomes possible without external I/O

### Architecture

```
JavaScript Code                   Rust Side
└─ console.log("Hello")   ─────>  ConsoleBuffers.stdout.push("Hello\n")
└─ console.error("Fail")  ─────>  ConsoleBuffers.stderr.push("Fail\n")
└─ console.warn("Warning")─────>  ConsoleBuffers.stdout.push("Warning\n")
└─ console.debug("Debug") ─────>  ConsoleBuffers.stderr.push("Debug\n")
```

**Key Insight**: Console functions are Rust callbacks bound into JavaScript global scope. When JavaScript calls `console.log()`, it directly calls Rust code that appends to buffers.

---

## Sub-Milestone 2.1: ConsoleBuffers Structure

**Estimated Effort**: 1 hour
**Priority**: P0 - Blocking

### Goal

Create the data structure to hold captured console output.

### Implementation

**File**: `src/app/agent_framework/v8_bindings/console.rs`

```rust
use std::cell::RefCell;
use std::rc::Rc;

/// Console output buffers
///
/// Holds stdout and stderr buffers that capture console output
/// during JavaScript execution. Uses Rc<RefCell<>> for shared
/// mutable access from V8 callback functions.
#[derive(Debug, Clone)]
pub struct ConsoleBuffers {
    /// Stdout buffer (console.log, console.warn, console.debug)
    pub stdout: Rc<RefCell<String>>,

    /// Stderr buffer (console.error)
    pub stderr: Rc<RefCell<String>>,
}

impl ConsoleBuffers {
    /// Create new empty console buffers
    pub fn new() -> Self {
        Self {
            stdout: Rc::new(RefCell::new(String::new())),
            stderr: Rc::new(RefCell::new(String::new())),
        }
    }

    /// Get stdout contents
    pub fn get_stdout(&self) -> String {
        self.stdout.borrow().clone()
    }

    /// Get stderr contents
    pub fn get_stderr(&self) -> String {
        self.stderr.borrow().clone()
    }

    /// Clear both buffers
    pub fn clear(&self) {
        self.stdout.borrow_mut().clear();
        self.stderr.borrow_mut().clear();
    }
}

impl Default for ConsoleBuffers {
    fn default() -> Self {
        Self::new()
    }
}
```

### Design Decisions

**Why `Rc<RefCell<String>>`?**
- `Rc<>` allows multiple owners (ConsoleBuffers + V8 callbacks)
- `RefCell<>` allows interior mutability (append during callbacks)
- `String` is the buffer for text output

**Why separate stdout/stderr?**
- Mirrors standard Unix streams
- Allows agents to distinguish errors from regular output
- Follows JavaScript convention (console.error vs console.log)

### Tests

**File**: `src/app/agent_framework/v8_bindings/console.rs` (test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_buffers_creation() {
        let buffers = ConsoleBuffers::new();
        assert_eq!(buffers.get_stdout(), "");
        assert_eq!(buffers.get_stderr(), "");
    }

    #[test]
    fn test_console_buffers_clear() {
        let buffers = ConsoleBuffers::new();
        buffers.stdout.borrow_mut().push_str("test");
        buffers.stderr.borrow_mut().push_str("error");

        assert_eq!(buffers.get_stdout(), "test");
        assert_eq!(buffers.get_stderr(), "error");

        buffers.clear();
        assert_eq!(buffers.get_stdout(), "");
        assert_eq!(buffers.get_stderr(), "");
    }
}
```

### Acceptance Criteria

- [ ] ConsoleBuffers struct compiles
- [ ] Can create new empty buffers
- [ ] Can read stdout/stderr contents
- [ ] Can clear buffers
- [ ] Tests pass

---

## Sub-Milestone 2.2: Console Function Callbacks

**Estimated Effort**: 3 hours
**Priority**: P0 - Blocking

**⚠️ CRITICAL**: Before implementing, read:
- RUSTY_V8_142_API_REFERENCE.md Section 2 (Function Callback API)
- RUSTY_V8_142_API_REFERENCE.md Section 10 (Test Suite Examples #2 and #4)

### Goal

Implement the four console functions as Rust callbacks that append to ConsoleBuffers.

### Implementation Pattern

**Reference**: See RUSTY_V8_142_API_REFERENCE.md, Example 2 (Line 3643) for the exact pattern.

**Callback Signature (v142.x)**:
```rust
fn callback_name(
    scope: &mut v8::PinScope,  // ✅ NOT &mut v8::HandleScope!
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
)
```

**File**: `src/app/agent_framework/v8_bindings/console.rs`

#### External Data Pattern

**Reference**: RUSTY_V8_142_API_REFERENCE.md Section 6 (External Data Pattern)

ConsoleBuffers need to be accessible from callbacks. Use v8::External:

```rust
impl ConsoleBuffers {
    /// Convert to V8 External for passing as function data
    ///
    /// **Reference**: RUSTY_V8_142_API_REFERENCE.md Section 6
    fn to_v8_external<'s>(
        &self,
        scope: &mut (impl v8::InIsolate + 's),  // Generic over scope types
    ) -> v8::Local<'s, v8::External> {
        // Box the buffers and convert to raw pointer
        let buffers_box = Box::new(self.clone());
        let buffers_ptr = Box::into_raw(buffers_box) as *mut std::ffi::c_void;

        // Create V8 External from pointer
        v8::External::new(scope, buffers_ptr)
    }

    /// Extract ConsoleBuffers from V8 External data
    ///
    /// # Safety
    ///
    /// The data must be a valid pointer to ConsoleBuffers created by to_v8_external
    unsafe fn from_v8_external(external: v8::Local<v8::External>) -> Self {
        let ptr = external.value() as *mut ConsoleBuffers;
        (*ptr).clone()
    }
}
```

#### console.log Callback

```rust
/// Callback for console.log
///
/// **Reference**: RUSTY_V8_142_API_REFERENCE.md Example 2 (console pattern)
fn console_log_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    // Extract buffers from function data
    let data = args.data();
    let external = v8::Local::<v8::External>::try_from(data).unwrap();
    let buffers = unsafe { ConsoleBuffers::from_v8_external(external) };

    // Format and append to stdout
    let message = format_console_args(scope, &args);
    buffers.stdout.borrow_mut().push_str(&message);
    buffers.stdout.borrow_mut().push('\n');
}

/// Format console arguments to string
///
/// Converts all arguments to strings and joins them with spaces,
/// similar to how console.log works in browsers and Node.js.
///
/// **Reference**: RUSTY_V8_142_API_REFERENCE.md Example 4 (reading arguments)
fn format_console_args(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
) -> String {
    let mut parts = Vec::new();

    for i in 0..args.length() {
        let arg = args.get(i);
        if let Some(arg_str) = arg.to_string(scope) {
            let rust_str = arg_str.to_rust_string_lossy(scope);
            parts.push(rust_str);
        }
    }

    parts.join(" ")
}
```

#### Other Console Functions

Implement console.error, console.warn, console.debug using the same pattern:
- console.error → stderr buffer
- console.warn → stdout buffer
- console.debug → stdout buffer

### Registration Function

**Reference**: RUSTY_V8_142_API_REFERENCE.md Example 2 shows Function::builder usage with data.

```rust
/// Register console functions in V8 global scope
///
/// Binds console.log, console.error, console.warn, and console.debug
/// to the provided V8 scope. Output is captured to the provided buffers.
///
/// **Reference**: RUSTY_V8_142_API_REFERENCE.md Section 2.3 (Function::builder)
pub fn register_console(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    buffers: ConsoleBuffers,
) {
    let global = scope.get_current_context().global(scope);

    // Create console object
    let console_key = v8::String::new(scope, "console").unwrap();
    let console_obj = v8::Object::new(scope);

    // Create external data for buffers
    let buffers_data = buffers.to_v8_external(scope);

    // Register console.log
    let log_fn = v8::Function::builder(console_log_callback)
        .data(buffers_data.into())
        .build(scope)
        .unwrap();
    let log_key = v8::String::new(scope, "log").unwrap();
    console_obj.set(scope, log_key.into(), log_fn.into());

    // Register console.error, console.warn, console.debug similarly...

    // Attach console object to global scope
    global.set(scope, console_key.into(), console_obj.into());
}
```

### Tests

```rust
#[test]
fn test_console_log_binding() {
    let _ = initialize_v8_platform();

    let mut params = v8::CreateParams::default();
    let mut isolate = v8::Isolate::new(params);

    let scope = pin!(v8::HandleScope::new(&mut isolate));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let buffers = ConsoleBuffers::new();
    register_console(scope, buffers.clone());

    // Execute console.log
    let code = v8::String::new(scope, "console.log('Hello', 'World')").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    script.run(scope);

    assert_eq!(buffers.get_stdout(), "Hello World\n");
    assert_eq!(buffers.get_stderr(), "");
}

#[test]
fn test_console_error_binding() {
    // Similar to above, but check stderr
}

#[test]
fn test_console_multiple_arguments() {
    // Test: console.log('a', 'b', 'c', 123, true)
    // Expected: "a b c 123 true\n"
}

#[test]
fn test_console_mixed_output() {
    // Test multiple console calls
    // Verify stdout and stderr separation
}
```

### Acceptance Criteria

- [ ] All four console functions implemented
- [ ] Output captured to correct buffers (stdout vs stderr)
- [ ] Multiple arguments joined with spaces
- [ ] All tests pass
- [ ] No memory leaks (External cleanup handled by V8)

---

## Sub-Milestone 2.3: Integration with V8Runtime

**Estimated Effort**: 1 hour
**Priority**: P0 - Blocking

### Goal

Integrate console bindings with V8Runtime so all JavaScript execution has console functions available.

### Implementation

**File**: `src/app/agent_framework/v8_bindings/runtime.rs`

Modify the `execute()` method to register console before executing code:

```rust
pub fn execute(&self, code: &str) -> Result<ExecutionResult> {
    let start_time = Instant::now();

    // ... isolate creation code ...

    // Execute JavaScript in proper scope hierarchy
    let result = {
        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        // ✅ NEW: Create console buffers and register console functions
        let console_buffers = if self.config.capture_console {
            let buffers = ConsoleBuffers::new();
            register_console(scope, buffers.clone());
            Some(buffers)
        } else {
            None
        };

        // ... existing compilation and execution code ...

        // ✅ CHANGE: Return console output along with result
        Ok::<(String, Option<ConsoleBuffers>), anyhow::Error>((result_json, console_buffers))
    }?;

    let (result, console_buffers) = result;
    let (stdout, stderr) = if let Some(buffers) = console_buffers {
        (buffers.get_stdout(), buffers.get_stderr())
    } else {
        (String::new(), String::new())
    };

    Ok(ExecutionResult {
        success: true,
        result: Some(result),
        stdout,  // ✅ NOW POPULATED
        stderr,  // ✅ NOW POPULATED
        execution_time_ms,
    })
}
```

### Tests

```rust
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
    assert!(result.stdout.contains("Hello from JavaScript"));
    assert!(result.stdout.contains("This is a warning"));
    assert!(result.stderr.contains("This is an error"));
}
```

### Acceptance Criteria

- [ ] Console functions available in all JavaScript execution
- [ ] stdout and stderr populated in ExecutionResult
- [ ] capture_console config flag works
- [ ] Tests pass

---

## Sub-Milestone 2.4: Update Module Exports

**Estimated Effort**: 15 minutes
**Priority**: P0 - Blocking

### Goal

Export console types from v8_bindings module.

### Implementation

**File**: `src/app/agent_framework/v8_bindings/mod.rs`

```rust
pub mod console;
pub mod platform;
pub mod runtime;

pub use console::{register_console, ConsoleBuffers};
pub use platform::{initialize_v8_platform, is_v8_initialized};
pub use runtime::{ExecutionResult, RuntimeConfig, V8Runtime};
```

### Acceptance Criteria

- [ ] console module exported
- [ ] ConsoleBuffers and register_console publicly accessible
- [ ] All existing code compiles

---

## Sub-Milestone 2.5: Documentation and Verification

**Estimated Effort**: 1 hour
**Priority**: P1 - Important

### Goal

Document console binding feature and verify complete integration.

### Documentation Updates

**File**: `src/app/agent_framework/v8_bindings/README.md`

Update the following sections:

1. **Architecture** - Add console bindings
2. **API Reference** - Document ConsoleBuffers
3. **Usage Examples** - Show console output capture
4. **RuntimeConfig** - Update capture_console description

Example section to add:

```markdown
### Console Output Capture

JavaScript console functions are bound to Rust callbacks that capture output:

\```rust
let runtime = V8Runtime::new();
let result = runtime.execute(r#"
    console.log("Processing data...");
    const data = [1, 2, 3];
    console.log("Array length:", data.length);
    data.reduce((a, b) => a + b)
\`"#).unwrap();

assert_eq!(result.result.unwrap(), "6");
assert_eq!(result.stdout, "Processing data...\nArray length: 3\n");
\```

**Supported Functions**:
- `console.log()` → stdout
- `console.error()` → stderr
- `console.warn()` → stdout
- `console.debug()` → stdout
```

### Verification Script

Create comprehensive test that exercises all console functionality:

**File**: `tests/v8_console_integration.rs`

```rust
#[test]
fn test_console_complete_workflow() {
    use awsdash::app::agent_framework::v8_bindings::{
        initialize_v8_platform, V8Runtime
    };

    // Initialize platform
    initialize_v8_platform().unwrap();

    // Create runtime
    let runtime = V8Runtime::new();

    // Execute JavaScript with console output
    let code = r#"
        console.log("Starting computation");

        function fibonacci(n) {
            console.debug("Computing fibonacci(" + n + ")");
            if (n <= 1) return n;
            return fibonacci(n - 1) + fibonacci(n - 2);
        }

        try {
            const result = fibonacci(10);
            console.log("Result:", result);
            result;
        } catch (e) {
            console.error("Error:", e.message);
            throw e;
        }
    "#;

    let result = runtime.execute(code).unwrap();

    // Verify execution
    assert!(result.success);
    assert_eq!(result.result.unwrap(), "55");

    // Verify console output
    assert!(result.stdout.contains("Starting computation"));
    assert!(result.stdout.contains("Result: 55"));
    assert!(result.stdout.contains("Computing fibonacci"));
    assert_eq!(result.stderr, "");

    println!("✅ Console integration test passed!");
}
```

### Acceptance Criteria

- [ ] README updated with console documentation
- [ ] Integration test created and passing
- [ ] All existing tests still pass
- [ ] Milestone 2 complete!

---

## Milestone 2 Complete!

### Deliverables

1. ✅ ConsoleBuffers data structure
2. ✅ Four console functions bound (log, error, warn, debug)
3. ✅ Console integration with V8Runtime
4. ✅ Output capture (stdout/stderr separation)
5. ✅ Comprehensive test suite
6. ✅ Documentation updated

### Test Coverage

- ConsoleBuffers creation and clearing
- Individual console function bindings
- Multi-argument formatting
- Mixed output (stdout/stderr separation)
- Integration with V8Runtime.execute()
- Complete workflow test

### Estimated vs Actual

- **Estimated**: 1 day (~6-8 hours)
- **Actual**: (To be measured during implementation)

### Next Steps

**Milestone 3**: Function Binding System - See CODE_EXECUTION_TOOL.md

---

## References

### Internal Documentation

- **Main Planning**: [CODE_EXECUTION_TOOL.md](./CODE_EXECUTION_TOOL.md)
- **Milestone 1**: [CODE_EXECUTION_TOOL_PART_2.md](./CODE_EXECUTION_TOOL_PART_2.md)
- **🎯 V8 API Reference**: [RUSTY_V8_142_API_REFERENCE.md](./RUSTY_V8_142_API_REFERENCE.md)

### External Documentation

- **rusty_v8 Docs**: https://docs.rs/v8/142.0.0/v8/
- **rusty_v8 Tests**: https://github.com/denoland/rusty_v8/blob/v142.1.0/tests/test_api.rs
- **Crate**: https://crates.io/crates/v8

---

## Troubleshooting

### Common Issues

**Issue**: Compilation error about PinScope vs HandleScope
**Solution**: Callback signature must use `&mut v8::PinScope`, not `&mut v8::HandleScope`
**Reference**: RUSTY_V8_142_API_REFERENCE.md Section 2

**Issue**: External data crashes at runtime
**Solution**: Ensure External pointer lifecycle matches function lifecycle
**Reference**: RUSTY_V8_142_API_REFERENCE.md Section 6

**Issue**: Scope type mismatch in register_console
**Solution**: Use generic `impl v8::InIsolate` for to_v8_external
**Reference**: RUSTY_V8_142_API_REFERENCE.md Section 11 (Pitfalls)

**Issue**: Cannot convert arguments to strings
**Solution**: Use `arg.to_string(scope)` then `to_rust_string_lossy(scope)`
**Reference**: RUSTY_V8_142_API_REFERENCE.md Section 4 (Value Conversion)

---

**End of Part 3 - Milestone 2: Console Binding**
