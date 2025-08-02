#![warn(clippy::all, rust_2018_idioms)]

/// Enhanced unified logging macros with file, function, and line context
/// This ensures consistency across the codebase and makes debugging much easier
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        log::trace!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
        tracing::trace!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        log::debug!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
        tracing::debug!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        log::info!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
        tracing::info!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        log::warn!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
        tracing::warn!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        log::error!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
        tracing::error!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

/// Enhanced tracing macros with context (for when you only want tracing, not log+tracing)
/// These provide the same context enhancement but only for the tracing system
#[macro_export]
macro_rules! trace_trace {
    ($($arg:tt)*) => {
        tracing::trace!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! trace_debug {
    ($($arg:tt)*) => {
        tracing::debug!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! trace_info {
    ($($arg:tt)*) => {
        tracing::info!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! trace_warn {
    ($($arg:tt)*) => {
        tracing::warn!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

#[macro_export]
macro_rules! trace_error {
    ($($arg:tt)*) => {
        tracing::error!("[{}:{}:{}] {}", file!(), module_path!(), line!(), format!($($arg)*));
    };
}

/*
Enhanced Logging System:

ENHANCED MACROS (with file:module:line context):
- log_trace!, log_debug!, log_info!, log_warn!, log_error! - Write to both log and tracing
- trace_trace!, trace_debug!, trace_info!, trace_warn!, trace_error! - Write only to tracing

All enhanced macros automatically include [file:module:line] context for easy debugging.

ANTI-FLOODING GUIDELINES:
- NEVER use trace/debug logging in render loops, update functions, or frequent callbacks
- Use log_once() for messages that should only appear once per application run
- Prefer higher log levels (warn/error) for operational messages

Example output:
  [src/app/dashui/app.rs:awsdash::app::dashui::app:252] Focus changing from CommandPalette to TemplateSections

USAGE:
- Use log_* macros for important messages that should go to both systems
- Use trace_* macros for debugging information that only needs tracing
- Replace direct tracing::info!() calls with trace_info!() for better context

EXAMPLES:
```rust
// OLD (no context):
tracing::info!("Focus changing from {:?} to {:?}", old, new);
// Output: Focus changing from CommandPalette to TemplateSections

// NEW (with context):
trace_info!("Focus changing from {:?} to {:?}", old, new);
// Output: [src/app/dashui/app.rs:awsdash::app::dashui::app:252] Focus changing from CommandPalette to TemplateSections

// Use log_* for important events that should persist:
log_info!("Project saved successfully");

// Use trace_* for debugging that should only go to tracing:
trace_debug!("Rendering frame with {} resources", count);
```

Log level guidelines for consistent usage across the codebase:

TRACE: Method-level implementation details, individual item processing
- Property/icon lookups and matches
- Individual resource processing within loops
- Detailed timing measurements
- JSON parsing details

DEBUG: Operation progress, state transitions, cache operations
- UI interactions (button clicks, window opens)
- SDK/API client initialization details
- File I/O operation details
- Method entry/exit logs
- Cache hit/miss logs

INFO: User actions, operation completions, important milestones
- High-level operation completion
- User-initiated actions
- Important state changes
- Summary statistics

WARN: Recoverable issues, fallbacks, performance concerns
- Fallback behaviors (e.g., using default icons)
- Deprecated functionality usage
- Performance issues (e.g., operations taking too long)

ERROR: Unrecoverable errors, failed operations, data corruption
- Failed AWS API calls that affect functionality
- File I/O failures that prevent operations
- JSON parsing failures for critical data
*/
