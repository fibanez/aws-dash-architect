//! Console Bindings for V8 JavaScript Engine
//!
//! Implements console.log, console.error, console.warn, and console.debug
//! as Rust-bound functions that capture output to stdout/stderr buffers.

#![warn(clippy::all, rust_2018_idioms)]

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

    /// Extract ConsoleBuffers from V8 External data
    ///
    /// # Safety
    ///
    /// The data must be a valid pointer to ConsoleBuffers created via v8::External::new
    ///
    /// **Reference**: RUSTY_V8_142_API_REFERENCE.md Section 6 (External Data Pattern)
    unsafe fn from_v8_external(external: v8::Local<'_, v8::External>) -> Self {
        let ptr = external.value() as *mut ConsoleBuffers;
        (*ptr).clone()
    }
}

impl Default for ConsoleBuffers {
    fn default() -> Self {
        Self::new()
    }
}

/// Register console functions in V8 global scope
///
/// Binds console.log, console.error, console.warn, and console.debug
/// to the provided V8 scope. Output is captured to the provided buffers.
///
/// **Reference**: RUSTY_V8_142_API_REFERENCE.md Section 2.3 (Function::builder)
pub fn register_console(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
    buffers: ConsoleBuffers,
) {
    let global = scope.get_current_context().global(scope);

    // Create console object
    let console_key = v8::String::new(scope, "console").unwrap();
    let console_obj = v8::Object::new(scope);

    // Create external data for buffers
    // Reference: RUSTY_V8_142_API_REFERENCE.md Example 2 (External pattern)
    let buffers_box = Box::new(buffers.clone());
    let buffers_ptr = Box::into_raw(buffers_box) as *mut std::ffi::c_void;
    let buffers_data = v8::External::new(scope, buffers_ptr);

    // Register console.log
    {
        let log_fn = v8::Function::builder(console_log_callback)
            .data(buffers_data.into())
            .build(scope)
            .unwrap();
        let log_key = v8::String::new(scope, "log").unwrap();
        console_obj.set(scope, log_key.into(), log_fn.into());
    }

    // Register console.error
    {
        let error_fn = v8::Function::builder(console_error_callback)
            .data(buffers_data.into())
            .build(scope)
            .unwrap();
        let error_key = v8::String::new(scope, "error").unwrap();
        console_obj.set(scope, error_key.into(), error_fn.into());
    }

    // Register console.warn
    {
        let warn_fn = v8::Function::builder(console_warn_callback)
            .data(buffers_data.into())
            .build(scope)
            .unwrap();
        let warn_key = v8::String::new(scope, "warn").unwrap();
        console_obj.set(scope, warn_key.into(), warn_fn.into());
    }

    // Register console.debug
    {
        let debug_fn = v8::Function::builder(console_debug_callback)
            .data(buffers_data.into())
            .build(scope)
            .unwrap();
        let debug_key = v8::String::new(scope, "debug").unwrap();
        console_obj.set(scope, debug_key.into(), debug_fn.into());
    }

    // Attach console object to global scope
    global.set(scope, console_key.into(), console_obj.into());
}

/// Callback for console.log
fn console_log_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    _rv: v8::ReturnValue<'_>,
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

/// Callback for console.error
fn console_error_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    _rv: v8::ReturnValue<'_>,
) {
    // Extract buffers from function data
    let data = args.data();
    let external = v8::Local::<v8::External>::try_from(data).unwrap();
    let buffers = unsafe { ConsoleBuffers::from_v8_external(external) };

    // Format and append to stderr
    let message = format_console_args(scope, &args);
    buffers.stderr.borrow_mut().push_str(&message);
    buffers.stderr.borrow_mut().push('\n');
}

/// Callback for console.warn
fn console_warn_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    _rv: v8::ReturnValue<'_>,
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

/// Callback for console.debug
fn console_debug_callback(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments<'_>,
    _rv: v8::ReturnValue<'_>,
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
/// **Reference**: RUSTY_V8_142_API_REFERENCE.md Example 4 (Reading Arguments)
fn format_console_args(
    scope: &mut v8::PinScope<'_, '_>,
    args: &v8::FunctionCallbackArguments<'_>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
    use std::pin::pin;

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

    #[test]
    fn test_console_log_binding() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
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
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let buffers = ConsoleBuffers::new();
        register_console(scope, buffers.clone());

        // Execute console.error
        let code = v8::String::new(scope, "console.error('Error:', 'Failed')").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        script.run(scope);

        assert_eq!(buffers.get_stdout(), "");
        assert_eq!(buffers.get_stderr(), "Error: Failed\n");
    }

    #[test]
    fn test_console_warn_binding() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let buffers = ConsoleBuffers::new();
        register_console(scope, buffers.clone());

        // Execute console.warn
        let code = v8::String::new(scope, "console.warn('Warning!')").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        script.run(scope);

        assert_eq!(buffers.get_stdout(), "Warning!\n");
        assert_eq!(buffers.get_stderr(), "");
    }

    #[test]
    fn test_console_debug_binding() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let buffers = ConsoleBuffers::new();
        register_console(scope, buffers.clone());

        // Execute console.debug
        let code = v8::String::new(scope, "console.debug('Debug info')").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        script.run(scope);

        assert_eq!(buffers.get_stdout(), "Debug info\n");
        assert_eq!(buffers.get_stderr(), "");
    }

    #[test]
    fn test_console_multiple_arguments() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let buffers = ConsoleBuffers::new();
        register_console(scope, buffers.clone());

        // Execute console.log with multiple arguments
        let code = v8::String::new(scope, "console.log('a', 'b', 'c', 123, true)").unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        script.run(scope);

        assert_eq!(buffers.get_stdout(), "a b c 123 true\n");
    }

    #[test]
    fn test_console_mixed_output() {
        let _ = initialize_v8_platform();

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let buffers = ConsoleBuffers::new();
        register_console(scope, buffers.clone());

        // Execute mixed console calls
        let code = v8::String::new(
            scope,
            r#"
                console.log('Line 1');
                console.error('Error 1');
                console.log('Line 2');
                console.error('Error 2');
                console.warn('Warning');
            "#,
        )
        .unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        script.run(scope);

        assert_eq!(buffers.get_stdout(), "Line 1\nLine 2\nWarning\n");
        assert_eq!(buffers.get_stderr(), "Error 1\nError 2\n");
    }

    #[test]
    fn test_v8_javascript_demo() {
        let _ = initialize_v8_platform();

        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║  🚀 V8 JavaScript Engine - Live Execution Demo  🚀   ║");
        println!("╚════════════════════════════════════════════════════════╝\n");

        let params = v8::CreateParams::default();
        let mut isolate = v8::Isolate::new(params);

        let scope = pin!(v8::HandleScope::new(&mut isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let buffers = ConsoleBuffers::new();
        register_console(scope, buffers.clone());

        // JavaScript program to execute
        let js_code = r#"
console.log("🎯 JavaScript execution started!");
console.log("⚙️  Running inside V8 engine (Chrome 142)");

// Define a fibonacci function
function fibonacci(n) {
    console.log(`   📊 Computing fibonacci(${n})...`);
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log("\n📝 Testing JavaScript features:");
console.log("   ✓ Variables and constants");
console.log("   ✓ Functions and recursion");
console.log("   ✓ Template literals");
console.log("   ✓ Arrow functions");

const numbers = [8, 10, 12];
console.log(`\n🔢 Computing Fibonacci for: ${numbers.join(', ')}`);

const results = numbers.map(n => ({
    input: n,
    result: fibonacci(n)
}));

console.log("\n📊 Results:");
results.forEach(r => {
    console.log(`   fibonacci(${r.input}) = ${r.result}`);
});

console.log("\n✅ JavaScript execution complete!");

// Return the results as JSON string
JSON.stringify(results, null, 2);
"#;

        println!("📄 JavaScript Code:");
        println!("─────────────────────────────────────────────────────────");
        println!("{}", js_code.trim());
        println!("─────────────────────────────────────────────────────────\n");

        println!("⚡ Executing JavaScript...\n");

        // Compile and execute
        let code = v8::String::new(scope, js_code).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();

        // Get console output
        let stdout = buffers.get_stdout();
        let stderr = buffers.get_stderr();

        println!("📤 JavaScript Console Output:");
        println!("═════════════════════════════════════════════════════════");
        for line in stdout.lines() {
            println!("{}", line);
        }
        println!("═════════════════════════════════════════════════════════\n");

        if !stderr.is_empty() {
            println!("⚠️  Errors: {}", stderr);
        }

        // Get return value
        let result_str = result.to_string(scope).unwrap();
        let result_json = result_str.to_rust_string_lossy(scope);

        println!("🎁 JavaScript Return Value:");
        println!("─────────────────────────────────────────────────────────");
        println!("{}", result_json);
        println!("─────────────────────────────────────────────────────────\n");

        println!("✨ Demo Complete! V8 JavaScript engine is operational! ✨\n");

        // Assertions to verify it actually worked
        assert!(stdout.contains("JavaScript execution started"));
        assert!(stdout.contains("fibonacci(8) = 21"));
        assert!(stdout.contains("fibonacci(10) = 55"));
        assert!(stdout.contains("fibonacci(12) = 144"));
        assert!(result_json.contains("21"));
        assert!(result_json.contains("55"));
        assert!(result_json.contains("144"));
    }
}
