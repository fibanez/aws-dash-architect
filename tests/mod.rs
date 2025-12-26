//! Comprehensive test suite for AWS Dash Architect.
//!
//! This module organizes the test suite into distinct categories for efficient testing
//! and debugging.

// ================================================================================================
// Core System Tests (Chunk 1) - Fundamental data structures and API contracts
// ================================================================================================

/// Basic API contract validation ensuring stable interfaces
mod api_contract_simple_tests;

/// AWS authentication data structure stability and serialization
mod aws_identity_frozen_tests;

// ================================================================================================
// AWS Identity and Authentication Tests - Core authentication workflow testing
// ================================================================================================

/// AWS Identity Center authentication workflow and state management testing
mod aws_identity_tests;

/// DashUI application state management and theme system testing
mod dashui_tests;

// ================================================================================================
// Window Focus Management Tests - Window system coordination and focus handling
// ================================================================================================

/// Window focus system integration testing
mod window_focus_integration_tests;

/// Comprehensive window focus behavior testing
mod window_focus_comprehensive_tests;

/// Window focus ordering and priority testing
mod window_focus_order_tests;

/// Window focus parameter handling and type safety testing
mod window_focus_parameter_tests;

// ================================================================================================
// Test Infrastructure and Utilities
// ================================================================================================

/// Common test utilities and infrastructure support
mod test_helpers {
    use std::fs;

    /// Ensures test directories exist for fixture and snapshot storage.
    pub fn setup_test_dirs() {
        fs::create_dir_all("tests/fixtures").ok();
        fs::create_dir_all("tests/snapshots").ok();
    }
}

#[cfg(test)]
mod setup {
    use super::test_helpers;

    /// Initializes the test environment by creating necessary directories.
    #[test]
    fn setup_test_environment() {
        test_helpers::setup_test_dirs();
    }
}
