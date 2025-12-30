# UI Component Testing

⚠️ **TEMPORARILY REMOVED** - This comprehensive UI testing framework will be re-implemented in a future release.

~~Comprehensive guide for writing UI tests and testing strategies for AWS Dash Architect's egui-based interface.~~

## Testing Philosophy

UI testing in AWS Dash follows a multi-layered approach combining unit tests for logic, integration tests for workflows, and automated UI tests for visual validation. This ensures both functional correctness and visual consistency across the application.

## Testing Framework Overview

### egui_kittest Integration

AWS Dash uses `egui_kittest` for automated UI testing, providing:

* ****Pixel-perfect regression testing** - Visual snapshots detect unintended changes
* ****Interaction simulation** - Automated clicking, typing, and user input
* ****Accessibility validation** - Screen reader compatibility testing
* ****Cross-platform consistency** - Behavior validation across operating systems

### Test Organization

UI tests are organized into distinct categories following the chunked testing strategy:

```
tests/
├── ui_basic_test.rs              # Basic UI functionality
├── ui_framework_working_test.rs  # Framework validation
├── ui_component_testing_framework.rs # Testing infrastructure
├── ui_test_utilities.rs          # Common testing utilities
├── ui_baseline_*.rs              # Baseline component tests
└── ui_tests/                     # Organized component tests
    ├── basic_ui_tests.rs
    ├── resource_form_tests.rs
    └── mod.rs
```

## Core Testing Patterns

### Basic UI Test Structure

*Standard Test Pattern*:
```rust
use egui_kittest::kittest::Kittest;
use awsdash::DashApp;

#[test]
fn test_window_basic_functionality() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    // Setup test scenario
    app.help_window.open = true;
    
    // Run UI frame
    kittest.run(&mut app, |ctx| {
        // UI rendering happens here
        app.update(ctx, &mut kittest.frame());
    });
    
    // Validate results
    assert!(app.help_window.is_open());
    
    // Optional: Visual regression testing
    kittest.expect_screenshot("help_window_basic");
}
```

### Interaction Testing Pattern

*User Interaction Simulation*:
```rust
#[test]
fn test_button_interaction() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    kittest.run(&mut app, |ctx| {
        app.update(ctx, &mut kittest.frame());
        
        // Simulate user clicking a button
        if let Some(button) = kittest.find_widget_by_label("Open Resource Form") {
            kittest.click_widget(button);
        }
    });
    
    // Verify button click effect
    assert!(app.resource_form_window.is_open());
}
```

### Window Focus Testing

*Focus Behavior Validation*:
```rust
#[test]
fn test_window_focus_coordination() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    // Request focus for specific window
    app.focus_manager.request_focus("resource_form");
    
    kittest.run(&mut app, |ctx| {
        app.update(ctx, &mut kittest.frame());
    });
    
    // Verify focus was applied
    assert!(app.resource_form_window.is_open());
    assert!(!app.focus_manager.has_pending_focus());
}
```

## Testing Strategies by Component Type

### Window Management Testing

*Focus System Validation*:
* Test window bring-to-front behavior
* Validate focus manager state transitions
* Verify parameter passing for different window types
* Test window lifecycle (open, close, minimize)

*Example Test*:
```rust
#[test]
fn test_focusable_window_trait_compliance() {
    let mut window = ResourceFormWindow::default();
    
    // Test trait implementation
    assert_eq!(window.window_id(), "resource_form");
    assert!(!window.is_open());
    
    let params = FormShowParams {
        resource_type: "AWS::S3::Bucket".to_string(),
        window_pos: None,
    };
    
    let mut kittest = Kittest::default();
    kittest.run(&mut window, |ctx| {
        window.show_with_focus(ctx, params, true);
    });
    
    assert!(window.is_open());
}
```

### Form Component Testing

*Schema-driven Form Validation*:
* Test form generation from AWS resource schemas
* Validate property constraints and validation rules
* Test real-time validation feedback
* Verify form submission and data binding

*Example Test*:
```rust
#[test]
fn test_resource_form_validation() {
    let mut form = ResourceFormWindow::default();
    form.set_resource_type("AWS::S3::Bucket");
    
    let mut kittest = Kittest::default();
    kittest.run(&mut form, |ctx| {
        form.show(ctx);
        
        // Test required field validation
        if let Some(name_field) = kittest.find_widget_by_label("Bucket Name") {
            kittest.type_text(name_field, "invalid-bucket-name-");
        }
    });
    
    // Verify validation error is shown
    assert!(form.has_validation_errors());
    assert!(form.get_error_for_field("BucketName").is_some());
}
```

### Command Palette Testing

*Search and Selection Validation*:
* Test fuzzy search functionality
* Validate command filtering and categorization
* Test keyboard navigation
* Verify command execution

*Example Test*:
```rust
#[test]
fn test_command_palette_search() {
    let mut palette = CommandPalette::default();
    palette.open = true;
    
    let mut kittest = Kittest::default();
    kittest.run(&mut palette, |ctx| {
        palette.show(ctx);
        
        // Simulate typing search query
        if let Some(search_field) = kittest.find_widget_by_label("Search") {
            kittest.type_text(search_field, "resource");
        }
    });
    
    // Verify filtered results
    let results = palette.get_filtered_commands();
    assert!(results.iter().any(|cmd| cmd.contains("Resource")));
}
```

### Visualization Component Testing

*Graph Interaction Validation*:
* Test node selection and manipulation
* Validate dependency visualization accuracy
* Test layout algorithm consistency
* Verify interactive graph operations

*Example Test*:
```rust
#[test]
fn test_dependency_graph_interaction() {
    let mut graph = CloudFormationSceneGraph::default();
    
    // Setup test data
    let mut dag = ResourceDag::new();
    dag.add_resource_smart("Bucket", create_test_bucket());
    dag.add_resource_smart("Role", create_test_role());
    graph.load_from_dag(&dag);
    
    let mut kittest = Kittest::default();
    kittest.run(&mut graph, |ctx| {
        graph.show(ctx);
        
        // Test node selection
        if let Some(bucket_node) = kittest.find_widget_by_label("Bucket") {
            kittest.click_widget(bucket_node);
        }
    });
    
    // Verify selection state
    assert_eq!(graph.get_selected_node(), Some("Bucket"));
}
```

## Visual Regression Testing

### Snapshot Testing Strategy

*Baseline Establishment*:
* Create reference screenshots for all major UI components
* Store baselines in version control for tracking changes
* Use consistent test data for reproducible results
* Test across different themes and window sizes

*Snapshot Test Pattern*:
```rust
#[test]
fn test_resource_form_visual_baseline() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    // Setup consistent test scenario
    app.resource_form_window.set_resource_type("AWS::S3::Bucket");
    app.resource_form_window.open = true;
    app.theme = ThemeChoice::Latte; // Consistent theme
    
    kittest.run(&mut app, |ctx| {
        app.update(ctx, &mut kittest.frame());
    });
    
    // Generate and compare screenshot
    kittest.expect_screenshot("resource_form_s3_bucket_latte");
}
```

### Theme Consistency Testing

*Multi-theme Validation*:
```rust
#[test]
fn test_all_themes_consistency() {
    for theme in [ThemeChoice::Latte, ThemeChoice::Frappe, 
                  ThemeChoice::Macchiato, ThemeChoice::Mocha] {
        let mut kittest = Kittest::default();
        let mut app = DashApp::default();
        app.theme = theme;
        app.help_window.open = true;
        
        kittest.run(&mut app, |ctx| {
            app.update(ctx, &mut kittest.frame());
        });
        
        kittest.expect_screenshot(&format!("help_window_{:?}", theme));
    }
}
```

## Performance Testing

### UI Responsiveness Validation

*Performance Benchmarking*:
```rust
use std::time::Instant;

#[test]
fn test_large_template_performance() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    // Load large template with many resources
    let large_template = create_template_with_100_resources();
    app.load_template(large_template);
    
    let start = Instant::now();
    
    kittest.run(&mut app, |ctx| {
        app.update(ctx, &mut kittest.frame());
    });
    
    let duration = start.elapsed();
    
    // Verify acceptable performance
    assert!(duration.as_millis() < 100, "UI update took too long: {:?}", duration);
}
```

### Memory Usage Testing

*Resource Cleanup Validation*:
```rust
#[test]
fn test_window_cleanup() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    // Open and close windows multiple times
    for _ in 0..10 {
        app.resource_form_window.open = true;
        kittest.run(&mut app, |ctx| {
            app.update(ctx, &mut kittest.frame());
        });
        
        app.resource_form_window.open = false;
        kittest.run(&mut app, |ctx| {
            app.update(ctx, &mut kittest.frame());
        });
    }
    
    // Verify no memory leaks (implementation-specific)
    // This would require memory profiling integration
}
```

## Accessibility Testing

### Screen Reader Compatibility

*Accessibility Validation*:
```rust
#[test]
fn test_accessibility_compliance() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    app.resource_form_window.open = true;
    
    kittest.run(&mut app, |ctx| {
        app.update(ctx, &mut kittest.frame());
    });
    
    // Verify accessibility features
    let accessibility_tree = kittest.get_accessibility_tree();
    
    // Check for proper labeling
    assert!(accessibility_tree.has_label_for("Bucket Name"));
    
    // Verify keyboard navigation
    assert!(accessibility_tree.supports_keyboard_navigation());
    
    // Check color contrast (if supported by framework)
    assert!(accessibility_tree.meets_contrast_requirements());
}
```

## Test Utilities and Helpers

### Common Test Setup

*Shared Test Infrastructure*:
```rust
pub struct TestContext {
    pub kittest: Kittest,
    pub app: DashApp,
}

impl TestContext {
    pub fn new() -> Self {
        Self {
            kittest: Kittest::default(),
            app: DashApp::default(),
        }
    }
    
    pub fn with_theme(mut self, theme: ThemeChoice) -> Self {
        self.app.theme = theme;
        self
    }
    
    pub fn with_open_window(mut self, window_id: &str) -> Self {
        match window_id {
            "help" => self.app.help_window.open = true,
            "resource_form" => self.app.resource_form_window.open = true,
            _ => panic!("Unknown window: {}", window_id),
        }
        self
    }
    
    pub fn run_frame(&mut self) {
        self.kittest.run(&mut self.app, |ctx| {
            self.app.update(ctx, &mut self.kittest.frame());
        });
    }
}
```

### Mock Data Generators

*Consistent Test Data*:
```rust
pub fn create_test_s3_bucket() -> CloudFormationResource {
    CloudFormationResource {
        logical_id: "TestBucket".to_string(),
        resource_type: "AWS::S3::Bucket".to_string(),
        properties: json!({
            "BucketName": "test-bucket-12345",
            "PublicReadPolicy": false
        }),
        ..Default::default()
    }
}

pub fn create_test_template_with_dependencies() -> CloudFormationTemplate {
    let mut template = CloudFormationTemplate::default();
    
    // Add resources with known dependencies
    template.resources.insert("Bucket".to_string(), create_test_s3_bucket());
    template.resources.insert("Role".to_string(), create_test_iam_role());
    
    template
}
```

## Debugging Test Failures

### Visual Debugging

*Screenshot Comparison*:
When visual tests fail, compare screenshots to identify differences:

```bash
# View failed screenshot comparison
diff tests/screenshots/expected/help_window.png tests/screenshots/actual/help_window.png
```

*Debug Output*:
```rust
#[test]
fn test_with_debug_output() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    // Enable debug output
    kittest.set_debug_mode(true);
    
    kittest.run(&mut app, |ctx| {
        println!("Window state: {:?}", app.get_window_states());
        app.update(ctx, &mut kittest.frame());
        println!("After update: {:?}", app.get_window_states());
    });
}
```

### Test Isolation

*Avoiding Test Interdependence*:
* Reset application state between tests
* Use fresh instances for each test
* Avoid shared mutable state
* Clean up temporary files and resources

## Continuous Integration

### Automated UI Testing

*CI Pipeline Integration*:
```bash
# Run UI tests in headless mode
EGUI_HEADLESS=1 cargo test ui_

# Generate and compare screenshots
cargo test --test ui_baseline_tests

# Performance benchmarking
cargo test --release --test ui_performance_tests
```

*Test Categorization*:
* ****Fast UI Tests** - Basic functionality (<1 second each)
* ****Visual Tests** - Screenshot comparison (1-5 seconds each)  
* ****Performance Tests** - Benchmarking and profiling (5-30 seconds each)
* ****Integration Tests** - End-to-end workflows (10-60 seconds each)

## Best Practices

### Test Design Guidelines

*Maintainable Tests*:
* Write tests that are easy to understand and modify
* Use descriptive test names that explain the scenario
* Keep tests focused on single behaviors
* Avoid testing implementation details

*Reliable Tests*:
* Use deterministic test data
* Avoid timing-dependent assertions
* Handle async operations properly
* Test error conditions and edge cases

*Efficient Tests*:
* Minimize test setup and teardown time
* Use appropriate test granularity
* Parallel test execution where possible
* Smart test categorization for CI/CD

### Common Pitfalls

*Avoiding Flaky Tests*:
* Don't rely on specific timing
* Use proper synchronization for async operations
* Handle window focus and platform differences
* Test with consistent data and environment

*Performance Considerations*:
* Don't over-test UI implementation details
* Focus on user-visible behavior
* Use mocks for expensive operations
* Consider test execution time in design

## Related Documentation

* [Testing Strategies](testing-patterns.md)