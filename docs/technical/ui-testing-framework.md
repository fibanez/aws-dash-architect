# UI Testing Framework

⚠️ **TEMPORARILY REMOVED** - This comprehensive UI testing framework will be re-implemented in a future release.

~~Automated testing with egui_kittest for reliable user interface validation.~~

## Overview

You can test AWS Dash user interface components automatically using the egui_kittest framework. This enables you to verify UI behavior, catch visual regressions, and validate user interactions without manual testing. The framework provides three specialized testing utilities that work together to ensure your CloudFormation property editing interface functions correctly across all scenarios.

## Dependencies and Setup

### Cargo Configuration

```toml
[dev-dependencies]
egui_kittest = { version = "0.31", features = ["snapshot", "wgpu"] }
```

### Core Imports

```rust
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
```

## Testing Architecture

### UITestHarness (`tests/ui_tests/mod.rs`)

Create consistent UI tests across your application components. The wrapper provides standardized patterns that reduce boilerplate code and ensure reliable test execution:

```rust
pub struct UITestHarness;

impl UITestHarness {
    /// Create a new test harness for egui application
    pub fn new<F>(app: F) -> Harness<'static>
    where F: FnMut(&egui::Context) + 'static

    /// Assert that a UI element exists by label  
    pub fn assert_element_exists(harness: &Harness<'_>, label: &str) -> TestResult

    /// Simulate a button click and validate the result
    pub fn click_button_and_validate<F>(...) -> TestResult
}
```

### UIComponentTestFramework (`tests/ui_component_testing_framework.rs`)

Test complex window components with validation and interaction simulation. This framework handles the intricacies of window lifecycle and state management:

```rust
impl UIComponentTestFramework {
    /// Create a test harness for window-based UI components
    pub fn create_window_test_harness<F>(mut window_content: F) -> Harness<'static>

    /// Validate that required UI elements exist in a component
    pub fn validate_required_elements(...) -> ComponentTestResult

    /// Test button interactions and state changes
    pub fn test_button_interaction(...) -> ComponentTestResult

    /// Test snapshot consistency for visual regression
    pub fn test_component_snapshot(...) -> ComponentTestResult
}
```

### UITestFramework (`tests/ui_test_utilities.rs`)

Generate realistic CloudFormation test data for thorough UI validation. These utilities provide pre-configured AWS resources and validation scenarios:

```rust
pub struct UITestFramework;

impl UITestFramework {
    /// Create a test CloudFormation resource for UI testing
    pub fn create_test_resource() -> CloudFormationResource

    /// Create test property definitions for AWS::S3::Bucket
    pub fn create_test_property_definitions() -> HashMap<String, PropertyDefinition>

    /// Create test validation data with various error scenarios
    pub fn create_test_validation_data() -> Vec<TestValidationScenario>
}
```

## Testing Patterns

### Basic UI Test Structure

```rust
let app = |ctx: &egui::Context| {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("Hello, World!");
        let _ = ui.button("Click Me");
    });
};

let harness = Harness::new(app);
```

### Element Finding and Interaction

```rust
// Safe element checking
let exists = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let _node = harness.get_by_label("Label Text");
})).is_ok();

// Button interaction
let button = harness.get_by_label("Button Label");
button.click();
harness.run(); // Process the interaction
```

### Snapshot Testing

```rust
// Visual regression testing
harness.snapshot(&snapshot_name);
```

## Test Organization

Tests are organized into chunked categories for efficient execution:

### Chunk 1: Core System Tests
- Basic API contracts and data structures
- ~60 tests, <30s execution time

### Chunk 2: CloudFormation Logic Tests  
- Template processing and validation
- ~50 tests, 1-2min execution time

### Chunk 3: UI Component Tests
- User interface components and interactions
- ~40 tests, 1-2min execution time
- **Primary use of egui_kittest framework**

### Chunk 4: Project Management Tests
- File operations and resource organization
- ~25 tests, 30s execution time

### Chunk 5: Integration Tests
- End-to-end workflows
- 10-30min execution time

## Running UI Tests

### Fast Test Suite (Recommended)
```bash
./scripts/test-chunks.sh fast
```

### UI-Specific Tests
```bash
./scripts/test-chunks.sh ui
```

### Individual UI Test Files
```bash
cargo test ui_basic_test
cargo test ui_component_testing_framework
cargo test reference_picker_window_tests
```

## Test Data and Utilities

### CloudFormation Test Resources

*AWS::S3::Bucket Test Resource*:
```rust
let test_resource = UITestFramework::create_test_resource();
// Contains: BucketName, AccessControl, PublicAccessBlockConfiguration
```

*Property Definitions*:
```rust
let properties = UITestFramework::create_test_property_definitions();
// Includes: Required fields, types, validation rules
```

*Validation Scenarios*:
```rust
let validation_data = UITestFramework::create_test_validation_data();
// Test cases: Valid inputs, missing required fields, invalid formats
```

## Component Testing Capabilities

### Window Focus Management
- Window registration and focus handling
- Multi-window interaction patterns
- Focus order validation

### Form Component Testing
- Property type forms (AWS::S3::Bucket, AWS::Lambda::Function)
- Input validation and error handling
- Required field highlighting
- Dynamic property updates

### Button Interaction Testing
- Click simulation and state changes
- Button highlighting and styling
- Confirmation dialogs and user flows

### Visual Regression Testing
- Snapshot comparison for UI consistency
- Multi-theme testing support
- Component layout validation

## Advanced Testing Features

### Multi-Theme Consistency
Test components across different visual themes to ensure consistent behavior.

### CloudFormation Integration Testing
- Template parsing and visualization
- Resource dependency validation  
- Property editing workflows

### Window System Testing
- Window creation and lifecycle management
- Inter-window communication patterns
- Focus management in complex workflows

## Best Practices

### Test Structure
1. **Setup**: Create test harness with realistic data
2. **Action**: Simulate user interactions
3. **Verification**: Assert expected state changes
4. **Cleanup**: Ensure proper resource disposal

### Naming Conventions
- Test files: `*_tests.rs` or `ui_*_test.rs`
- Test functions: `test_<component>_<behavior>`
- Snapshot names: Descriptive component and state names

### Error Handling
Use `std::panic::catch_unwind` for safe element queries that may fail in test scenarios.

### Performance Considerations
- Limit snapshot testing to critical visual components
- Use chunked test execution for large test suites
- Group related tests for efficient setup/teardown

## Related Documentation

- [Testing Patterns](testing-patterns.md) - Overall testing strategies
- [UI Component Testing](ui-component-testing.md) - Detailed component testing guide
- [Trait Patterns](trait-patterns.md) - Common implementation patterns
- [Parameter Patterns](parameter-patterns.md) - Type-safe parameter handling

## File Locations

*Core Framework*:
- `tests/ui_tests/mod.rs` - UITestHarness wrapper
- `tests/ui_component_testing_framework.rs` - Component testing framework
- `tests/ui_test_utilities.rs` - CloudFormation test utilities

*Example Tests*:
- `tests/ui_basic_test.rs` - Basic functionality validation
- `tests/ui_framework_working_test.rs` - Framework validation
- `tests/reference_picker_window_tests.rs` - Component-specific tests

*Test Execution*:
- `scripts/test-chunks.sh` - Chunked test execution
- Verbosity levels: quiet, smart, detailed, full