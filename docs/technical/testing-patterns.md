# Testing Patterns

Testing strategies and frameworks used throughout AWS Dash Architect for comprehensive quality assurance.

## Testing Philosophy

AWS Dash follows a comprehensive testing strategy that ensures reliability, maintainability, and user confidence. The testing approach emphasizes:

* ****Test Pyramid Architecture** - Unit tests form the foundation, integration tests validate workflows, UI tests ensure user experience
* ****Behavior-driven Testing** - Tests validate user-facing behavior rather than implementation details
* ****Continuous Testing** - Automated testing integrated into development workflow
* ****Performance Testing** - Ensuring responsive performance under realistic load conditions

## Testing Architecture Overview

### Test Organization Strategy

*Chunked Testing System*:
The application uses a sophisticated chunked testing strategy optimized for context window management and CI/CD efficiency:

```
Core Tests (Chunk 1):     ~60 tests,  <30 seconds
CloudFormation (Chunk 2): ~50 tests,  1-2 minutes  
UI Components (Chunk 3):  ~40 tests,  1-2 minutes
Projects (Chunk 4):       ~25 tests,  30 seconds
Integration (Chunk 5):    Variable,   10-30 minutes
```

*Smart Verbosity Control*:
* **Level 0 (quiet)**: Minimal output for CI/CD
* **Level 1 (smart)**: Default mode showing failures without flooding
* **Level 2 (detailed)**: Failure details for debugging
* **Level 3 (full)**: Complete cargo test output

## Core Testing Patterns

### Frozen Testing Pattern

*Purpose*: Prevent unintentional breaking changes to data structures and APIs

*Implementation*:
```rust
use insta::{assert_json_snapshot, assert_yaml_snapshot};

#[test]
fn test_cloudformation_resource_serialization() {
    let resource = CloudFormationResource {
        logical_id: "TestBucket".to_string(),
        resource_type: "AWS::S3::Bucket".to_string(),
        properties: json!({
            "BucketName": "test-bucket-12345",
            "PublicReadPolicy": false
        }),
        depends_on: vec!["TestRole".to_string()],
        ..Default::default()
    };
    
    // Snapshot ensures structure doesn't change unintentionally
    assert_json_snapshot!(resource);
}
```

*Benefits*:
* Automatic detection of breaking changes
* Version control integration for reviewing changes
* Documentation of data structure evolution
* Protection against accidental API modifications

### Mock Pattern for External Dependencies

*Purpose*: Enable testing without external dependencies like AWS services

*Implementation*:
```rust
pub trait AwsClient {
    async fn get_credentials(&self) -> Result<Credentials, AwsError>;
    async fn list_accounts(&self) -> Result<Vec<Account>, AwsError>;
}

pub struct MockAwsClient {
    credentials_response: Result<Credentials, AwsError>,
    accounts_response: Result<Vec<Account>, AwsError>,
    call_count: Arc<Mutex<HashMap<String, usize>>>,
}

impl MockAwsClient {
    pub fn new() -> Self {
        Self {
            credentials_response: Ok(Credentials::default()),
            accounts_response: Ok(vec![]),
            call_count: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn with_credentials_error(mut self, error: AwsError) -> Self {
        self.credentials_response = Err(error);
        self
    }
    
    pub fn get_call_count(&self, method: &str) -> usize {
        self.call_count.lock().unwrap().get(method).copied().unwrap_or(0)
    }
}

#[async_trait]
impl AwsClient for MockAwsClient {
    async fn get_credentials(&self) -> Result<Credentials, AwsError> {
        let mut counts = self.call_count.lock().unwrap();
        *counts.entry("get_credentials".to_string()).or_insert(0) += 1;
        self.credentials_response.clone()
    }
}
```

*Usage*:
```rust
#[tokio::test]
async fn test_authentication_flow_with_error() {
    let mock_client = MockAwsClient::new()
        .with_credentials_error(AwsError::NetworkError);
    
    let mut identity_center = AwsIdentityCenter::new();
    identity_center.set_client(Box::new(mock_client));
    
    let result = identity_center.authenticate().await;
    
    assert!(result.is_err());
    assert_eq!(mock_client.get_call_count("get_credentials"), 1);
}
```

### Property-based Testing Pattern

*Purpose*: Test with generated inputs to discover edge cases

*Implementation*:
```rust
use proptest::prelude::*;

// Generator for valid CloudFormation resource names
fn resource_name_strategy() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z0-9]*"
        .prop_filter("Must be valid CF resource name", |s| {
            s.len() <= 255 && !s.is_empty()
        })
}

proptest! {
    #[test]
    fn test_resource_dag_properties(
        resource_names in prop::collection::vec(resource_name_strategy(), 1..50)
    ) {
        let mut dag = ResourceDag::new();
        
        // Add all resources
        for name in &resource_names {
            let resource = create_test_resource(name);
            dag.add_resource_smart(resource);
        }
        
        // Property: DAG should never have cycles
        prop_assert!(!dag.has_cycle());
        
        // Property: All resources should be in deployment order
        let order = dag.get_deployment_order();
        prop_assert_eq!(order.len(), resource_names.len());
        
        // Property: Order should be deterministic
        let order2 = dag.get_deployment_order();
        prop_assert_eq!(order, order2);
    }
}
```

### Parameterized Testing Pattern

*Purpose*: Test same logic with multiple inputs efficiently

*Implementation*:
```rust
use rstest::rstest;

#[rstest]
#[case("AWS::S3::Bucket", true)]
#[case("AWS::EC2::Instance", true)]
#[case("AWS::Lambda::Function", true)]
#[case("Invalid::Resource::Type", false)]
#[case("", false)]
fn test_resource_type_validation(#[case] resource_type: &str, #[case] should_be_valid: bool) {
    let result = validate_resource_type(resource_type);
    assert_eq!(result.is_ok(), should_be_valid);
}

#[rstest]
#[case::latte(ThemeChoice::Latte)]
#[case::frappe(ThemeChoice::Frappe)]
#[case::macchiato(ThemeChoice::Macchiato)]
#[case::mocha(ThemeChoice::Mocha)]
fn test_theme_consistency(#[case] theme: ThemeChoice) {
    let mut app = DashApp::default();
    app.theme = theme;
    
    // Test that all windows render consistently with theme
    let mut kittest = Kittest::default();
    kittest.run(&mut app, |ctx| {
        app.update(ctx, &mut kittest.frame());
    });
    
    // Verify theme was applied correctly
    assert_eq!(app.theme, theme);
    kittest.expect_screenshot(&format!("theme_consistency_{:?}", theme));
}
```

## UI Testing Patterns

### Visual Regression Testing

*Purpose*: Detect unintended visual changes in UI components

*Implementation*:
```rust
use egui_kittest::Kittest;

#[test]
fn test_resource_form_visual_baseline() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    // Setup consistent test scenario
    app.resource_form_window.set_resource_type("AWS::S3::Bucket");
    app.resource_form_window.open = true;
    app.theme = ThemeChoice::Latte;
    
    kittest.run(&mut app, |ctx| {
        app.update(ctx, &mut kittest.frame());
    });
    
    // Compare against baseline screenshot
    kittest.expect_screenshot("resource_form_s3_bucket_baseline");
}
```

*Baseline Management*:
```bash
# Update baselines when changes are intentional
cargo test -- --update-snapshots

# Review baseline changes
git diff tests/screenshots/

# Accept changes
git add tests/screenshots/
```

### Interaction Testing Pattern

*Purpose*: Validate user interaction workflows

*Implementation*:
```rust
#[test]
fn test_command_palette_workflow() {
    let mut kittest = Kittest::default();
    let mut app = DashApp::default();
    
    // Step 1: Open command palette
    kittest.run(&mut app, |ctx| {
        // Simulate Ctrl+P
        ctx.input_mut(|i| {
            i.key_pressed(egui::Key::P);
            i.modifiers.ctrl = true;
        });
        app.update(ctx, &mut kittest.frame());
    });
    
    assert!(app.command_palette.is_open());
    
    // Step 2: Type search query
    kittest.run(&mut app, |ctx| {
        if let Some(search_input) = kittest.find_widget_by_label("Search") {
            kittest.type_text(search_input, "resource");
        }
        app.update(ctx, &mut kittest.frame());
    });
    
    // Step 3: Verify filtered results
    let filtered_commands = app.command_palette.get_filtered_commands();
    assert!(filtered_commands.iter().any(|cmd| cmd.contains("Resource")));
    
    // Step 4: Select command
    kittest.run(&mut app, |ctx| {
        ctx.input_mut(|i| i.key_pressed(egui::Key::Enter));
        app.update(ctx, &mut kittest.frame());
    });
    
    // Verify command was executed
    assert!(app.resource_types_window.is_open());
}
```

### State Machine Testing

*Purpose*: Validate complex state transitions

*Implementation*:
```rust
#[test]
fn test_aws_authentication_state_machine() {
    let mut identity_center = AwsIdentityCenter::new();
    
    // Initial state
    assert_eq!(identity_center.get_state(), LoginState::Disconnected);
    
    // State transition: initialize
    identity_center.initialize();
    assert_eq!(identity_center.get_state(), LoginState::Initializing);
    
    // State transition: start device flow
    let auth_data = identity_center.start_device_authorization().unwrap();
    assert_eq!(identity_center.get_state(), LoginState::AwaitingAuthorization);
    assert!(auth_data.device_code.len() > 0);
    
    // State transition: complete authentication
    identity_center.complete_device_authorization(&auth_data).unwrap();
    assert_eq!(identity_center.get_state(), LoginState::Authenticated);
    
    // State transition: logout
    identity_center.logout();
    assert_eq!(identity_center.get_state(), LoginState::Disconnected);
}
```

## Integration Testing Patterns

### End-to-end Workflow Testing

*Purpose*: Validate complete user workflows

*Implementation*:
```rust
#[test]
fn test_complete_template_creation_workflow() {
    let mut app = DashApp::default();
    let mut kittest = Kittest::default();
    
    // Step 1: Create new project
    app.create_new_project("Test Project");
    assert!(app.current_project.is_some());
    
    // Step 2: Add S3 bucket resource
    kittest.run(&mut app, |ctx| {
        app.open_resource_form("AWS::S3::Bucket");
        app.update(ctx, &mut kittest.frame());
    });
    
    // Step 3: Configure bucket properties
    kittest.run(&mut app, |ctx| {
        if let Some(name_field) = kittest.find_widget_by_label("Bucket Name") {
            kittest.type_text(name_field, "my-test-bucket");
        }
        app.update(ctx, &mut kittest.frame());
    });
    
    // Step 4: Save resource
    kittest.run(&mut app, |ctx| {
        if let Some(save_button) = kittest.find_widget_by_label("Save Resource") {
            kittest.click_widget(save_button);
        }
        app.update(ctx, &mut kittest.frame());
    });
    
    // Step 5: Verify resource was added to project
    let project = app.current_project.as_ref().unwrap();
    let project_guard = project.read().unwrap();
    assert_eq!(project_guard.get_resources().len(), 1);
    
    // Step 6: Export template
    let template = app.export_cloudformation_template().unwrap();
    assert!(template.resources.contains_key("S3Bucket"));
    
    // Step 7: Validate exported template
    let validation_result = template.validate();
    assert!(validation_result.is_ok());
}
```

### Cross-component Integration

*Purpose*: Test component interactions and data flow

*Implementation*:
```rust
#[test]
fn test_dag_template_synchronization() {
    let mut project = Project::default();
    
    // Add resources to project
    let bucket = create_test_s3_bucket();
    let role = create_test_iam_role_with_bucket_dependency();
    
    project.add_resource(bucket);
    project.add_resource(role);
    
    // Build DAG dynamically from project resources
    let dag = project.build_dag_from_resources();
    assert_eq!(dag.get_resource_count(), 2);
    
    // Verify dependency relationship
    let deployment_order = dag.get_deployment_order();
    let bucket_index = deployment_order.iter().position(|r| r == "TestBucket").unwrap();
    let role_index = deployment_order.iter().position(|r| r == "TestRole").unwrap();
    assert!(bucket_index < role_index, "Bucket should be deployed before role");
    
    // Export to CloudFormation template
    let template = project.to_cloudformation_template();
    
    // Verify template has correct dependencies
    let role_resource = template.resources.get("TestRole").unwrap();
    assert!(role_resource.depends_on.contains(&"TestBucket".to_string()));
    
    // Round-trip test: reimport template
    let mut new_project = Project::default();
    new_project.import_from_template(&template).unwrap();
    
    assert_eq!(new_project.get_resources().len(), 2);
    
    // Verify DAG consistency after round-trip
    let new_dag = new_project.build_dag_from_resources();
    assert_eq!(new_dag.get_deployment_order(), deployment_order);
}
```

## Performance Testing Patterns

### Load Testing

*Purpose*: Validate performance under realistic load

*Implementation*:
```rust
use std::time::{Duration, Instant};

#[test]
fn test_large_template_performance() {
    let template = create_template_with_1000_resources();
    
    // Test template parsing performance
    let start = Instant::now();
    let parsed_template = CloudFormationTemplate::from_json(&template.to_json()).unwrap();
    let parse_time = start.elapsed();
    
    assert!(parse_time < Duration::from_millis(500), 
           "Large template parsing took too long: {:?}", parse_time);
    
    // Test DAG construction performance
    let start = Instant::now();
    let dag = ResourceDag::from_template(&parsed_template);
    let dag_time = start.elapsed();
    
    assert!(dag_time < Duration::from_millis(1000),
           "DAG construction took too long: {:?}", dag_time);
    
    // Test topological sort performance
    let start = Instant::now();
    let order = dag.get_deployment_order();
    let sort_time = start.elapsed();
    
    assert!(sort_time < Duration::from_millis(100),
           "Topological sort took too long: {:?}", sort_time);
    assert_eq!(order.len(), 1000);
}
```

### Memory Usage Testing

*Purpose*: Detect memory leaks and excessive usage

*Implementation*:
```rust
#[test]
fn test_window_lifecycle_memory_usage() {
    let initial_memory = get_memory_usage();
    
    // Create and destroy windows multiple times
    for _ in 0..100 {
        let mut app = DashApp::default();
        
        // Open all windows
        app.help_window.open = true;
        app.resource_form_window.open = true;
        app.log_window.open = true;
        
        // Simulate some usage
        let mut kittest = Kittest::default();
        kittest.run(&mut app, |ctx| {
            app.update(ctx, &mut kittest.frame());
        });
        
        // Close all windows
        app.help_window.open = false;
        app.resource_form_window.open = false;
        app.log_window.open = false;
        
        // Force cleanup
        drop(app);
    }
    
    // Allow time for cleanup
    std::thread::sleep(Duration::from_millis(100));
    
    let final_memory = get_memory_usage();
    let memory_growth = final_memory - initial_memory;
    
    // Memory growth should be minimal (< 10MB for 100 cycles)
    assert!(memory_growth < 10 * 1024 * 1024, 
           "Memory leak detected: grew by {} bytes", memory_growth);
}
```

## Error Handling Testing Patterns

### Fault Injection Testing

*Purpose*: Test error handling and recovery mechanisms

*Implementation*:
```rust
#[test]
fn test_network_failure_handling() {
    let mut downloader = CfnResourcesDownloader::new();
    
    // Inject network failure
    downloader.set_network_simulation(NetworkSimulation::Failure);
    
    let result = downloader.download_resources("us-east-1");
    
    // Should gracefully handle failure
    assert!(result.is_err());
    
    // Should fall back to cached data
    let cached_resources = downloader.get_cached_resources("us-east-1");
    assert!(cached_resources.is_some());
    
    // Should log appropriate error
    let logs = capture_logs();
    assert!(logs.iter().any(|log| log.contains("Network failure")));
}

#[test]
fn test_corrupted_data_recovery() {
    let mut project = Project::default();
    
    // Add valid resource
    project.add_resource(create_test_s3_bucket());
    
    // Simulate data corruption
    project.corrupt_resource_data("TestBucket");
    
    // Should detect corruption
    let validation_result = project.validate();
    assert!(validation_result.has_errors());
    
    // Should offer recovery options
    let recovery_options = project.get_recovery_options();
    assert!(!recovery_options.is_empty());
    
    // Should be able to recover
    project.apply_recovery_option(&recovery_options[0]).unwrap();
    let post_recovery_validation = project.validate();
    assert!(!post_recovery_validation.has_errors());
}
```

## Test Utilities and Infrastructure

### Common Test Fixtures

*Purpose*: Reusable test data and setup

*Implementation*:
```rust
pub struct TestFixtures;

impl TestFixtures {
    pub fn create_s3_bucket(name: &str) -> CloudFormationResource {
        CloudFormationResource {
            logical_id: name.to_string(),
            resource_type: "AWS::S3::Bucket".to_string(),
            properties: json!({
                "BucketName": format!("{}-{}", name.to_lowercase(), random_suffix()),
                "PublicReadPolicy": false,
                "VersioningConfiguration": {
                    "Status": "Enabled"
                }
            }),
            ..Default::default()
        }
    }
    
    pub fn create_template_with_dependencies() -> CloudFormationTemplate {
        let mut template = CloudFormationTemplate::default();
        
        let bucket = Self::create_s3_bucket("TestBucket");
        let role = Self::create_iam_role_with_dependency("TestRole", "TestBucket");
        let policy = Self::create_iam_policy_with_dependency("TestPolicy", "TestRole");
        
        template.resources.insert("TestBucket".to_string(), bucket);
        template.resources.insert("TestRole".to_string(), role);
        template.resources.insert("TestPolicy".to_string(), policy);
        
        template
    }
    
    pub fn create_large_template(resource_count: usize) -> CloudFormationTemplate {
        let mut template = CloudFormationTemplate::default();
        
        for i in 0..resource_count {
            let resource = Self::create_s3_bucket(&format!("Bucket{}", i));
            template.resources.insert(format!("Bucket{}", i), resource);
        }
        
        template
    }
}
```

### Test Environment Setup

*Purpose*: Consistent test environment configuration

*Implementation*:
```rust
pub struct TestEnvironment {
    temp_dir: TempDir,
    config: TestConfig,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let config = TestConfig {
            cache_dir: temp_dir.path().join("cache"),
            project_dir: temp_dir.path().join("projects"),
            log_level: "debug".to_string(),
        };
        
        // Setup test directories
        std::fs::create_dir_all(&config.cache_dir).unwrap();
        std::fs::create_dir_all(&config.project_dir).unwrap();
        
        Self { temp_dir, config }
    }
    
    pub fn with_cached_resources(mut self) -> Self {
        // Pre-populate cache with test data
        let cache_file = self.config.cache_dir.join("us-east-1-resources.json");
        let test_resources = TestFixtures::create_resource_definitions();
        std::fs::write(cache_file, serde_json::to_string(&test_resources).unwrap()).unwrap();
        self
    }
    
    pub fn get_project_path(&self, name: &str) -> PathBuf {
        self.config.project_dir.join(format!("{}.awsdash", name))
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Cleanup is handled by TempDir automatically
    }
}
```

## Continuous Integration Testing

### CI Pipeline Integration

*GitHub Actions Configuration*:
```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        test-chunk: [core, cfn, ui, projects]
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        
    - name: Run Test Chunk
      run: ./scripts/test-chunks.sh ${{ matrix.test-chunk }}
      env:
        TEST_MODE: smart
        
    - name: Upload Test Results
      uses: actions/upload-artifact@v3
      if: failure()
      with:
        name: test-results-${{ matrix.test-chunk }}
        path: target/test-results/
```

### Test Result Analysis

*Automated Test Reporting*:
```rust
pub struct TestResultAnalyzer {
    results: Vec<TestResult>,
    baseline: TestBaseline,
}

impl TestResultAnalyzer {
    pub fn analyze_regression(&self) -> RegressionReport {
        let mut report = RegressionReport::new();
        
        for result in &self.results {
            if let Some(baseline_time) = self.baseline.get_duration(&result.test_name) {
                let regression = calculate_regression(baseline_time, result.duration);
                
                if regression > 0.1 { // 10% regression threshold
                    report.add_regression(TestRegression {
                        test_name: result.test_name.clone(),
                        baseline_duration: baseline_time,
                        current_duration: result.duration,
                        regression_percent: regression * 100.0,
                    });
                }
            }
        }
        
        report
    }
}
```

## Best Practices

### Test Design Principles

*Test Clarity*:
* Write tests that clearly express intent
* Use descriptive test names that explain the scenario
* Structure tests with clear Given-When-Then sections
* Avoid testing implementation details

*Test Reliability*:
* Make tests deterministic and repeatable
* Avoid timing dependencies in tests
* Use proper cleanup and isolation
* Handle async operations correctly

*Test Maintainability*:
* Keep tests simple and focused
* Use shared fixtures for common setup
* Refactor tests when code changes
* Document complex test scenarios

### Common Testing Anti-patterns

*Avoiding Flaky Tests*:
* Don't rely on specific timing or delays
* Use proper synchronization for async operations
* Avoid shared mutable state between tests
* Handle platform differences gracefully

*Test Performance*:
* Don't over-test implementation details
* Use appropriate test granularity
* Consider test execution time in CI/CD
* Parallelize independent tests with memory constraints (use `-j 4` with cargo test)
* ⚠️ Always use `-j 4` flag with cargo test commands to prevent memory exhaustion

## Related Documentation

* [UI Component Testing](ui-component-testing.md)
* [Trait Design Patterns](trait-patterns.md)
* [Performance Optimization](performance-optimization.md)
* [System Architecture](system-architecture.md)