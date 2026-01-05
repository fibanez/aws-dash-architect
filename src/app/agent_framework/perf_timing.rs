//! Debug-only Performance Timing Module
//!
//! This module provides zero-overhead performance timing for debug builds only.
//! In release builds, all macros expand to nothing.
//!
//! # Usage
//!
//! ```rust
//! use crate::app::agent_framework::perf_timing::*;
//!
//! // Time a single expression
//! let result = perf_timed!("fetch_credentials", get_credentials());
//!
//! // Time a block of code
//! perf_timed_block!("agent_initialization", {
//!     let agent = create_agent();
//!     agent.configure();
//! });
//!
//! // Manual start/end for complex flows
//! perf_start!("create_stood_agent");
//! // ... complex operations ...
//! perf_end!("create_stood_agent");
//! ```
//!
//! # Output
//!
//! Timing data is written to: `~/.local/share/awsdash/logs/agent_perf_timing.log`
//!
//! Format: `[timestamp] [thread] operation_name: duration_ms (context)`

#[cfg(debug_assertions)]
mod debug_impl {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::Instant;

    // Global log file handle
    lazy_static::lazy_static! {
        static ref PERF_LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
        static ref LOG_INITIALIZED: Mutex<bool> = Mutex::new(false);
    }

    // Thread-local storage for active timers
    thread_local! {
        static ACTIVE_TIMERS: RefCell<HashMap<String, Instant>> = RefCell::new(HashMap::new());
        static TIMER_STACK: RefCell<Vec<(String, Instant)>> = RefCell::new(Vec::new());
    }

    /// Initialize the performance timing log file
    pub fn init_perf_log() {
        let mut initialized = LOG_INITIALIZED.lock().unwrap();
        if *initialized {
            return;
        }

        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("awsdash")
            .join("logs");

        // Create directory if needed
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            eprintln!("[PERF] Failed to create log directory: {}", e);
            return;
        }

        let log_path = log_dir.join("agent_perf_timing.log");

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            Ok(file) => {
                let mut log_file = PERF_LOG_FILE.lock().unwrap();
                *log_file = Some(file);
                *initialized = true;

                // Write session header
                if let Some(ref mut f) = *log_file {
                    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let separator = "=".repeat(20);
                    let _ = writeln!(f, "\n{} Performance Timing Session Started at {} {}",
                        separator, timestamp, separator);
                    let _ = writeln!(f, "Build: DEBUG (perf_timing enabled)");
                    let _ = writeln!(f, "{}", "=".repeat(80));
                }
            }
            Err(e) => {
                eprintln!("[PERF] Failed to open log file: {}", e);
            }
        }
    }

    /// Log a performance timing entry
    pub fn log_timing(operation: &str, duration_us: u64, context: Option<&str>) {
        let mut log_file = PERF_LOG_FILE.lock().unwrap();
        if let Some(ref mut file) = *log_file {
            let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
            let thread_id = std::thread::current().id();
            let thread_name = std::thread::current()
                .name()
                .unwrap_or("unnamed")
                .to_string();

            // Format duration appropriately
            let duration_str = if duration_us >= 1_000_000 {
                format!("{:.2}s", duration_us as f64 / 1_000_000.0)
            } else if duration_us >= 1_000 {
                format!("{:.2}ms", duration_us as f64 / 1_000.0)
            } else {
                format!("{}us", duration_us)
            };

            let context_str = context.map(|c| format!(" ({})", c)).unwrap_or_default();

            let _ = writeln!(
                file,
                "[{}] [{:?}/{}] {}: {}{}",
                timestamp, thread_id, thread_name, operation, duration_str, context_str
            );
            let _ = file.flush();
        }
    }

    /// Start timing an operation (named timer)
    pub fn start_timer(name: &str) {
        init_perf_log();
        ACTIVE_TIMERS.with(|timers| {
            timers.borrow_mut().insert(name.to_string(), Instant::now());
        });
    }

    /// End timing an operation and log the result
    pub fn end_timer(name: &str, context: Option<&str>) {
        ACTIVE_TIMERS.with(|timers| {
            if let Some(start) = timers.borrow_mut().remove(name) {
                let duration = start.elapsed();
                log_timing(name, duration.as_micros() as u64, context);
            }
        });
    }

    /// Push a timer onto the stack (for nested timing)
    pub fn push_timer(name: &str) {
        init_perf_log();
        TIMER_STACK.with(|stack| {
            stack.borrow_mut().push((name.to_string(), Instant::now()));
        });
    }

    /// Pop a timer from the stack and log
    pub fn pop_timer(context: Option<&str>) {
        TIMER_STACK.with(|stack| {
            if let Some((name, start)) = stack.borrow_mut().pop() {
                let duration = start.elapsed();
                let indent = "  ".repeat(stack.borrow().len());
                let indented_name = format!("{}{}", indent, name);
                log_timing(&indented_name, duration.as_micros() as u64, context);
            }
        });
    }

    /// Log a checkpoint (instant timing point)
    pub fn log_checkpoint(name: &str, context: Option<&str>) {
        init_perf_log();
        let mut log_file = PERF_LOG_FILE.lock().unwrap();
        if let Some(ref mut file) = *log_file {
            let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
            let thread_id = std::thread::current().id();
            let context_str = context.map(|c| format!(" - {}", c)).unwrap_or_default();

            let _ = writeln!(file, "[{}] [{:?}] CHECKPOINT: {}{}", timestamp, thread_id, name, context_str);
            let _ = file.flush();
        }
    }

    /// RAII guard for automatic timing
    pub struct TimingGuard {
        name: String,
        start: Instant,
        context: Option<String>,
    }

    impl TimingGuard {
        pub fn new(name: &str) -> Self {
            init_perf_log();
            Self {
                name: name.to_string(),
                start: Instant::now(),
                context: None,
            }
        }

        pub fn with_context(name: &str, context: &str) -> Self {
            init_perf_log();
            Self {
                name: name.to_string(),
                start: Instant::now(),
                context: Some(context.to_string()),
            }
        }

        pub fn set_context(&mut self, context: &str) {
            self.context = Some(context.to_string());
        }
    }

    impl Drop for TimingGuard {
        fn drop(&mut self) {
            let duration = self.start.elapsed();
            log_timing(&self.name, duration.as_micros() as u64, self.context.as_deref());
        }
    }
}

// Re-export debug implementation
#[cfg(debug_assertions)]
pub use debug_impl::*;

// Release stubs - all operations are no-ops
#[cfg(not(debug_assertions))]
pub fn init_perf_log() {}

#[cfg(not(debug_assertions))]
pub fn log_timing(_operation: &str, _duration_us: u64, _context: Option<&str>) {}

#[cfg(not(debug_assertions))]
pub fn start_timer(_name: &str) {}

#[cfg(not(debug_assertions))]
pub fn end_timer(_name: &str, _context: Option<&str>) {}

#[cfg(not(debug_assertions))]
pub fn push_timer(_name: &str) {}

#[cfg(not(debug_assertions))]
pub fn pop_timer(_context: Option<&str>) {}

#[cfg(not(debug_assertions))]
pub fn log_checkpoint(_name: &str, _context: Option<&str>) {}

#[cfg(not(debug_assertions))]
pub struct TimingGuard;

#[cfg(not(debug_assertions))]
impl TimingGuard {
    pub fn new(_name: &str) -> Self {
        Self
    }
    pub fn with_context(_name: &str, _context: &str) -> Self {
        Self
    }
    pub fn set_context(&mut self, _context: &str) {}
}

/// Start timing a named operation
///
/// # Example
/// ```rust
/// perf_start!("my_operation");
/// // ... code ...
/// perf_end!("my_operation");
/// ```
#[macro_export]
macro_rules! perf_start {
    ($name:expr) => {
        $crate::app::agent_framework::perf_timing::start_timer($name)
    };
}

/// End timing a named operation
///
/// # Example
/// ```rust
/// perf_start!("my_operation");
/// // ... code ...
/// perf_end!("my_operation");
/// ```
#[macro_export]
macro_rules! perf_end {
    ($name:expr) => {
        $crate::app::agent_framework::perf_timing::end_timer($name, None)
    };
    ($name:expr, $context:expr) => {
        $crate::app::agent_framework::perf_timing::end_timer($name, Some($context))
    };
}

/// Time an expression and return its value
///
/// # Example
/// ```rust
/// let result = perf_timed!("fetch_data", expensive_fetch());
/// ```
#[macro_export]
macro_rules! perf_timed {
    ($name:expr, $expr:expr) => {{
        let _guard = $crate::app::agent_framework::perf_timing::TimingGuard::new($name);
        $expr
    }};
    ($name:expr, $context:expr, $expr:expr) => {{
        let _guard = $crate::app::agent_framework::perf_timing::TimingGuard::with_context($name, $context);
        $expr
    }};
}

/// Time a block of code
///
/// # Example
/// ```rust
/// perf_timed_block!("initialization", {
///     init_config();
///     init_resources();
/// });
/// ```
#[macro_export]
macro_rules! perf_timed_block {
    ($name:expr, $block:block) => {{
        let _guard = $crate::app::agent_framework::perf_timing::TimingGuard::new($name);
        $block
    }};
    ($name:expr, $context:expr, $block:block) => {{
        let _guard = $crate::app::agent_framework::perf_timing::TimingGuard::with_context($name, $context);
        $block
    }};
}

/// Log a checkpoint (for tracking progress without timing)
///
/// # Example
/// ```rust
/// perf_checkpoint!("agent_created");
/// perf_checkpoint!("credentials_loaded", "AWS us-east-1");
/// ```
#[macro_export]
macro_rules! perf_checkpoint {
    ($name:expr) => {
        $crate::app::agent_framework::perf_timing::log_checkpoint($name, None)
    };
    ($name:expr, $context:expr) => {
        $crate::app::agent_framework::perf_timing::log_checkpoint($name, Some($context))
    };
}

/// Create a timing guard that will log when dropped
///
/// # Example
/// ```rust
/// fn my_function() {
///     let _timing = perf_guard!("my_function");
///     // ... function body ...
///     // timing logged automatically on drop
/// }
/// ```
#[macro_export]
macro_rules! perf_guard {
    ($name:expr) => {
        $crate::app::agent_framework::perf_timing::TimingGuard::new($name)
    };
    ($name:expr, $context:expr) => {
        $crate::app::agent_framework::perf_timing::TimingGuard::with_context($name, $context)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_guard() {
        let _guard = TimingGuard::new("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        // Guard logs on drop
    }

    #[test]
    fn test_timing_macros() {
        perf_start!("manual_timer");
        std::thread::sleep(std::time::Duration::from_millis(5));
        perf_end!("manual_timer");

        let result = perf_timed!("timed_expr", {
            std::thread::sleep(std::time::Duration::from_millis(5));
            42
        });
        assert_eq!(result, 42);

        perf_checkpoint!("test_checkpoint");
    }
}
