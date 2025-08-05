//! Comprehensive test suite for AWS Dash Architect.
//!
//! This module organizes the test suite into distinct categories for efficient testing
//! and debugging. The test architecture supports the chunked testing strategy outlined
//! in [`CLAUDE.md`](../CLAUDE.md) for optimal context window management.
//!
//! # Test Organization
//!
//! ## Core System Tests (Chunk 1)
//! **Purpose**: Validate fundamental data structures and API contracts
//! **Performance**: ~60 tests, <30 seconds
//! **Coverage**: Core business logic without UI dependencies
//!
//! - [`test_api_contract_simple`] - Basic API contract validation
//! - [`test_aws_identity_frozen`] - AWS authentication data structure stability
//! - [`test_cfn_dag_frozen`] - Dependency graph data integrity
//! - [`test_projects_frozen`] - Project management data consistency
//!
//! ## CloudFormation Logic Tests (Chunk 2)
//! **Purpose**: Validate CloudFormation template processing and validation
//! **Performance**: ~50 tests, 1-2 minutes
//! **Coverage**: Template parsing, dependency resolution, schema validation
//!
//! - Template parsing and serialization tests
//! - Dependency validation and circular reference detection
//! - Schema constraint parsing and validation
//! - Intrinsic function classification and processing
//!
//! ## UI Component Tests (Chunk 3)
//! **Purpose**: Validate user interface components and interactions
//! **Performance**: ~40 tests, 1-2 minutes
//! **Coverage**: Window management, user interactions, visual components
//!
//! - Basic UI functionality and component rendering
//! - Window focus management and coordination
//! - Button highlighting and interaction patterns
//! - Property type form generation and validation
//! - Resource form workflows and data binding
//!
//! ## Project Management Tests (Chunk 4)
//! **Purpose**: Validate project organization and file operations
//! **Performance**: ~25 tests, 30 seconds
//! **Coverage**: File I/O, resource management, environment coordination
//!
//! - Project file structure and persistence
//! - Resource import/export workflows
//! - Environment management and organization
//! - File format compatibility and migration
//!
//! ## Integration Tests (Chunk 5)
//! **Purpose**: End-to-end workflow validation with real AWS templates
//! **Performance**: Variable, 10-30 minutes depending on scope
//! **Coverage**: Complete workflows using real-world CloudFormation templates
//!
//! - Real-world AWS template processing
//! - Cross-component integration validation
//! - Performance benchmarking and optimization
//! - End-to-end user workflow simulation
//!
//! # Test Infrastructure
//!
//! ## Snapshot Testing
//! Uses [`insta`] for snapshot-based regression testing of data structures.
//! Snapshots are stored in [`tests/snapshots/`] and automatically managed.
//!
//! ## UI Testing Framework
//! Uses [`egui_kittest`] for automated UI testing with snapshot comparison.
//! Provides pixel-perfect regression testing for visual components.
//!
//! ## Test Fixtures
//! Standardized test data in [`tests/fixtures/`] includes:
//! - Sample CloudFormation templates
//! - AWS resource specifications
//! - Project configuration examples
//! - DAG test structures
//!
//! ## Test Utilities
//! Common testing infrastructure provided by [`test_helpers`] module:
//! - Test environment setup and teardown
//! - Fixture loading and management
//! - Mock AWS service interactions
//! - Performance measurement utilities
//!
//! # Running Tests
//!
//! ## Chunked Test Execution
//! ```bash
//! # Fast test suite (recommended for development)
//! ./scripts/test-chunks.sh fast
//!
//! # Individual test chunks
//! ./scripts/test-chunks.sh core      # Core functionality tests
//! ./scripts/test-chunks.sh cfn       # CloudFormation processing tests
//! ./scripts/test-chunks.sh ui        # UI component tests
//! ./scripts/test-chunks.sh projects  # Project management tests
//! ./scripts/test-chunks.sh integration # Full integration tests
//! ```
//!
//! ## Verbosity Control
//! ```bash
//! # Smart output (default) - shows failures without flooding
//! ./scripts/test-chunks.sh core
//!
//! # Detailed output - shows failure details for debugging
//! TEST_MODE=detailed ./scripts/test-chunks.sh core
//!
//! # Quiet output - minimal output for CI/CD
//! TEST_MODE=quiet ./scripts/test-chunks.sh core
//! ```
//!
//! # Test Categories
//!
//! ## Frozen Tests
//! **Purpose**: Prevent unintentional breaking changes to data structures
//! **Method**: Snapshot testing with [`insta`] crate
//! **Coverage**: All public APIs and data serialization formats
//!
//! ## Unit Tests
//! **Purpose**: Validate individual component functionality
//! **Method**: Traditional assertion-based testing
//! **Coverage**: Business logic, algorithms, data transformations
//!
//! ## Integration Tests
//! **Purpose**: Validate component interactions and workflows
//! **Method**: Multi-component test scenarios
//! **Coverage**: Cross-module communication, file I/O, AWS integration
//!
//! ## UI Tests
//! **Purpose**: Validate user interface behavior and visual consistency
//! **Method**: [`egui_kittest`] automated UI testing
//! **Coverage**: Window management, user interactions, visual regression
//!
//! See the [testing strategy documentation](../docs/technical/testing-patterns.wiki) for
//! detailed information about test patterns and the [UI testing guide](../docs/technical/ui-component-testing.wiki)
//! for UI-specific testing approaches.

// ================================================================================================
// Core System Tests (Chunk 1) - Fundamental data structures and API contracts
// ================================================================================================

/// Basic API contract validation ensuring stable interfaces
mod api_contract_simple_tests;

/// AWS authentication data structure stability and serialization
mod aws_identity_frozen_tests;

/// Dependency graph data integrity and algorithm correctness
mod cfn_dag_frozen_tests;

/// Project management data consistency and file format stability
mod projects_frozen_tests;

// ================================================================================================
// CloudFormation Logic Tests (Chunk 2) - Template processing and validation
// ================================================================================================

/// CloudFormation template parsing, serialization, and validation
mod cfn_template_tests;

/// CloudFormation template verification and cross-reference validation
mod cfn_template_verification_tests;

/// Dependency validation and circular reference detection
mod cfn_dependency_validation_tests;

/// Intrinsic function classification and processing
mod intrinsic_function_classification_tests;

/// Schema constraint parsing and validation rule generation
mod schema_constraint_tests;

/// Real-world AWS template processing and compatibility testing
mod aws_real_world_templates;

/// Template fixture import discrepancy detection and resolution
mod fixture_import_discrepancies_tests;

/// CloudFormation graph verification and dependency analysis
mod cloudformation_graph_verification_tests;

// ================================================================================================
// UI Component Tests (Chunk 3) - REMOVED (August 2025)
// ================================================================================================
// UI tests were removed due to compilation issues and import problems.
// See tests/UI_TESTING_SETUP.md for complete archive and restoration notes.
// Approximately 20 UI test files with ~100-150 tests were removed covering:
// - Basic UI components and interactions
// - CloudFormation forms and editors
// - Visual consistency and styling
// - Window management and workflows

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

/// Window trait implementation testing and polymorphic behavior validation
mod window_trait_implementation_tests;

// ================================================================================================
// Project Management Tests (Chunk 4) - File operations and resource organization
// ================================================================================================

/// Project management functionality and file operations testing
mod projects_tests;

// ================================================================================================
// Test Infrastructure and Utilities
// ================================================================================================

/// Common test utilities and infrastructure support
mod test_helpers {
    use std::fs;

    /// Ensures test directories exist for fixture and snapshot storage.
    ///
    /// This function creates the necessary directory structure for test execution
    /// including fixture storage and snapshot management. Called automatically
    /// during test setup to ensure consistent test environment.
    ///
    /// # Created Directories
    /// - `tests/fixtures/` - Test data files and sample templates
    /// - `tests/snapshots/` - Insta snapshot files for regression testing
    ///
    /// # Error Handling
    /// Ignores errors during directory creation as directories may already exist.
    /// This approach ensures tests can run in various environments without
    /// failing due to existing directory structures.
    pub fn setup_test_dirs() {
        fs::create_dir_all("tests/fixtures").ok();
        fs::create_dir_all("tests/snapshots").ok();
    }
}

#[cfg(test)]
mod setup {
    use super::test_helpers;

    /// Initializes the test environment by creating necessary directories.
    ///
    /// This test ensures that the test infrastructure is properly set up
    /// before other tests run. It creates the fixture and snapshot directories
    /// required by various test modules.
    ///
    /// This test runs as part of the core test chunk and provides foundation
    /// infrastructure for all other tests in the suite.
    #[test]
    fn setup_test_environment() {
        test_helpers::setup_test_dirs();
    }
}
