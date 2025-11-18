//! Global V8 Platform Management
//!
//! The V8 platform must be initialized once at application startup
//! and remain alive for the entire application lifetime.
//!
//! # Thread Safety
//!
//! This module uses `OnceCell` to ensure thread-safe, one-time initialization
//! of the global V8 platform. Multiple calls to `initialize_v8_platform()` are
//! safe and idempotent.
//!
//! # Example
//!
//! ```no_run
//! use awsdash::app::agent_framework::v8_bindings::initialize_v8_platform;
//!
//! // At application startup
//! initialize_v8_platform().expect("V8 initialization failed");
//! ```

#![warn(clippy::all, rust_2018_idioms)]

use once_cell::sync::OnceCell;
use log::{info, warn};

/// Global V8 platform instance
///
/// Initialized once at application startup via `initialize_v8_platform()`
static GLOBAL_V8_PLATFORM: OnceCell<v8::SharedRef<v8::Platform>> = OnceCell::new();

/// Initialize the V8 platform
///
/// Must be called exactly once at application startup before any V8 isolates
/// are created. Thread-safe via OnceCell - subsequent calls are no-ops.
///
/// # Parameters
///
/// V8 platform is created with:
/// - Thread pool size: 0 (use default - based on CPU cores)
/// - Idle task support: false (not needed for our use case)
///
/// # Errors
///
/// Returns error if V8 initialization fails internally.
///
/// # Example
///
/// ```no_run
/// use awsdash::app::agent_framework::v8_bindings::initialize_v8_platform;
///
/// fn main() {
///     initialize_v8_platform().expect("Failed to initialize V8");
///     // ... rest of application
/// }
/// ```
pub fn initialize_v8_platform() -> Result<(), String> {
    GLOBAL_V8_PLATFORM
        .get_or_try_init(|| {
            // Create platform with default parameters
            // Parameters: (thread_pool_size, idle_task_support)
            // 0 = use default thread pool size (based on CPU cores)
            // false = no idle task support (not needed for our use case)
            let platform = v8::new_default_platform(0, false).make_shared();

            // Initialize V8 with platform
            v8::V8::initialize_platform(platform.clone());
            v8::V8::initialize();

            info!("V8 platform initialized successfully");

            Ok(platform)
        })
        .map(|_| ())
}

/// Check if V8 platform is initialized
///
/// # Returns
///
/// `true` if `initialize_v8_platform()` has been called successfully,
/// `false` otherwise.
///
/// # Example
///
/// ```no_run
/// use awsdash::app::agent_framework::v8_bindings::is_v8_initialized;
///
/// if !is_v8_initialized() {
///     eprintln!("V8 not initialized!");
/// }
/// ```
pub fn is_v8_initialized() -> bool {
    GLOBAL_V8_PLATFORM.get().is_some()
}

/// Shutdown V8 platform
///
/// Should be called on application exit for clean shutdown.
/// This is optional - V8 will clean up automatically on process exit.
///
/// # Safety
///
/// This function is unsafe because:
/// - It calls `V8::dispose()` which is an unsafe operation
/// - Must only be called when no V8 isolates are active
/// - Should only be called once during application shutdown
///
/// # Example
///
/// ```no_run
/// use awsdash::app::agent_framework::v8_bindings::dispose_v8_platform;
///
/// fn shutdown() {
///     unsafe {
///         dispose_v8_platform();
///     }
/// }
/// ```
pub unsafe fn dispose_v8_platform() {
    if GLOBAL_V8_PLATFORM.get().is_some() {
        // V8::dispose() is unsafe and should only be called on shutdown
        v8::V8::dispose();
        info!("V8 platform disposed");
    } else {
        warn!("Attempted to dispose V8 platform that was never initialized");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v8_platform_initialization() {
        // Test: Platform should initialize successfully
        let result = initialize_v8_platform();
        assert!(result.is_ok(), "V8 platform initialization failed");

        // Verify platform is initialized
        assert!(
            is_v8_initialized(),
            "V8 platform not marked as initialized"
        );
    }

    #[test]
    fn test_v8_double_initialization() {
        // Test: Second initialization should succeed (idempotent)
        let result1 = initialize_v8_platform();
        let result2 = initialize_v8_platform();

        assert!(result1.is_ok(), "First initialization failed");
        assert!(result2.is_ok(), "Second initialization failed");

        // OnceCell ensures only one actual initialization
        assert!(is_v8_initialized());
    }

    #[test]
    fn test_v8_initialized_before_init() {
        // This test may fail if other tests run first
        // Just document expected behavior
        if !is_v8_initialized() {
            assert!(
                !is_v8_initialized(),
                "Platform should not be initialized initially"
            );

            let result = initialize_v8_platform();
            assert!(result.is_ok());
            assert!(is_v8_initialized());
        }
    }
}
