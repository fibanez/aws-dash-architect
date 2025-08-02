# CloudFormation Manager Development Guide

Comprehensive guide for extending and maintaining the CloudFormation Manager within the AWS Dash unified desktop environment.

## Development Overview

The CloudFormation Manager is designed for extensibility and maintainability. This guide covers development patterns, testing strategies, and best practices for contributing to the CloudFormation management capabilities.

## Architecture Patterns

### Module Organization

The CloudFormation Manager follows a modular architecture with clear separation of concerns:

```
src/app/cloudformation_manager/
├── mod.rs                          # Module exports and public API
├── manager.rs                      # Core orchestration logic
├── deployment.rs                   # Deployment state management
├── parameters.rs                   # Parameter discovery and validation
├── parameter_dialog.rs             # Parameter input UI components
├── parameter_persistence.rs        # Parameter storage and history
├── parameter_store.rs              # AWS Parameter Store integration
├── secrets_manager.rs              # AWS Secrets Manager integration
├── resource_lookup.rs              # AWS resource discovery
├── resource_picker_dialog.rs       # Resource selection UI
├── deployment_progress_window.rs   # Deployment monitoring UI
└── validation_results_window.rs    # Template validation UI
```

### Design Patterns

**State Machine Pattern**: Used extensively for deployment and operation tracking:

```rust
#[derive(Debug, Clone)]
pub enum DeploymentState {
    Collecting,     // Gathering parameters
    Validating,     // Template validation
    Deploying,      // Stack operation in progress
    Monitoring,     // Watching for completion
    Complete(Result<DeploymentStats, String>),
    Failed(String),
    Cancelled,
}
```

**Service Integration Pattern**: Consistent pattern for AWS service integration:

```rust
pub struct ServiceClient {
    credential_coordinator: Arc<CredentialCoordinator>,
    // Service-specific configuration
}

impl ServiceClient {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        // Standard initialization pattern
    }
    
    pub async fn operation(&self, params: &OperationParams) -> Result<OperationResult> {
        // Standard error handling and logging
    }
}
```

**UI Component Pattern**: Consistent pattern for UI components:

```rust
pub struct ComponentWindow {
    open: bool,
    state: ComponentState,
    // Component-specific fields
}

impl ComponentWindow {
    pub fn new() -> Self {
        // Standard initialization
    }
    
    pub fn show(&mut self, ctx: &egui::Context, data: &ComponentData) {
        // Standard egui window pattern
    }
}
```

## Adding New Features

### 1. Adding a New AWS Service Integration

**Step 1: Create Service Client Module**

Create a new file in `src/app/cloudformation_manager/` following the service integration pattern:

```rust
// new_service.rs
use crate::app::resource_explorer::credentials::CredentialCoordinator;
use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{debug, error, info};

pub struct NewServiceClient {
    credential_coordinator: Arc<CredentialCoordinator>,
}

impl NewServiceClient {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
        }
    }
    
    pub async fn service_operation(&self, params: &ServiceParams) -> Result<ServiceResult> {
        debug!("Performing service operation with params: {:?}", params);
        
        // Get AWS client
        let aws_client = self.credential_coordinator
            .get_aws_client()
            .await
            .context("Failed to get AWS client")?;
            
        // Perform operation
        let result = aws_client
            .service_operation()
            .params(params)
            .send()
            .await
            .context("Service operation failed")?;
            
        info!("Service operation completed successfully");
        Ok(ServiceResult::from(result))
    }
}
```

**Step 2: Add to Module Exports**

Update `mod.rs` to export the new service:

```rust
pub use new_service::NewServiceClient;
```

**Step 3: Integrate with CloudFormationManager**

Add the service client to the manager:

```rust
// In manager.rs
pub struct CloudFormationManager {
    // existing fields...
    new_service_client: NewServiceClient,
}

impl CloudFormationManager {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            // existing initialization...
            new_service_client: NewServiceClient::new(credential_coordinator.clone()),
        }
    }
}
```

### 2. Adding a New UI Component

**Step 1: Create Component Structure**

```rust
// new_component_window.rs
use egui::{Context, Window};

pub struct NewComponentWindow {
    open: bool,
    state: ComponentState,
}

#[derive(Debug, Clone)]
pub struct ComponentState {
    // Component-specific state
}

impl Default for ComponentState {
    fn default() -> Self {
        Self {
            // Default values
        }
    }
}

impl NewComponentWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            state: ComponentState::default(),
        }
    }
    
    pub fn is_open(&self) -> bool {
        self.open
    }
    
    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }
    
    pub fn show(&mut self, ctx: &Context, data: &ComponentData) {
        if !self.open {
            return;
        }
        
        Window::new("Component Title")
            .open(&mut self.open)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                self.render_content(ui, data);
            });
    }
    
    fn render_content(&mut self, ui: &mut egui::Ui, data: &ComponentData) {
        // Component rendering logic
    }
}
```

**Step 2: Integrate with Main UI**

Add the component to the main DashApp:

```rust
// In src/app/dashui/app.rs
pub struct DashApp {
    // existing fields...
    new_component_window: NewComponentWindow,
}

impl DashApp {
    // In the UI rendering method
    fn render_ui(&mut self, ctx: &Context) {
        // existing UI code...
        
        self.new_component_window.show(ctx, &component_data);
    }
}
```

### 3. Adding Parameter Types

**Step 1: Extend Parameter Type Enum**

```rust
// In parameters.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterInputType {
    // existing types...
    NewParameterType {
        validation_rules: Vec<ValidationRule>,
        ui_hints: Vec<UiHint>,
    },
}
```

**Step 2: Add UI Rendering**

```rust
// In parameter_dialog.rs
fn render_parameter_input(&mut self, ui: &mut egui::Ui, param: &ParameterInfo) {
    match &param.input_type {
        // existing cases...
        ParameterInputType::NewParameterType { validation_rules, ui_hints } => {
            self.render_new_parameter_type(ui, param, validation_rules, ui_hints);
        }
    }
}

fn render_new_parameter_type(
    &mut self,
    ui: &mut egui::Ui,
    param: &ParameterInfo,
    validation_rules: &[ValidationRule],
    ui_hints: &[UiHint],
) {
    // Custom UI rendering for new parameter type
}
```

## Testing Patterns

### Unit Testing

**Testing AWS Service Integration**:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_credential_coordinator;
    
    #[tokio::test]
    async fn test_service_operation_success() {
        let credential_coordinator = create_test_credential_coordinator().await;
        let client = NewServiceClient::new(credential_coordinator);
        
        let params = ServiceParams {
            // test parameters
        };
        
        let result = client.service_operation(&params).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_service_operation_error_handling() {
        // Test error scenarios
    }
}
```

**Testing State Machines**:

```rust
#[test]
fn test_deployment_state_transitions() {
    let mut deployment = DeploymentOperation::new("test-deployment");
    
    assert_eq!(deployment.state, DeploymentState::Collecting);
    
    deployment.transition_to_validating();
    assert_eq!(deployment.state, DeploymentState::Validating);
    
    deployment.transition_to_deploying();
    assert_eq!(deployment.state, DeploymentState::Deploying);
}
```

### Integration Testing

**Testing End-to-End Workflows**:

```rust
#[tokio::test]
#[ignore] // Only run with --ignored flag for integration tests
async fn test_complete_deployment_workflow() {
    let test_setup = create_integration_test_setup().await;
    
    // Template validation
    let validation_result = test_setup.manager
        .validate_template(&test_template, &test_account, &test_region)
        .await
        .expect("Template validation should succeed");
    
    assert!(validation_result.is_valid);
    
    // Parameter collection
    let parameters = collect_test_parameters(&validation_result.parameters);
    
    // Stack deployment
    let deployment_id = test_setup.manager
        .deploy_stack("test-stack", &test_template, parameters, &test_region)
        .await
        .expect("Stack deployment should succeed");
    
    // Monitor deployment
    let final_state = monitor_deployment_completion(&test_setup.manager, &deployment_id).await;
    assert!(matches!(final_state, DeploymentState::Complete(_)));
}
```

### UI Testing

**Testing UI Components**:

```rust
#[cfg(test)]
mod ui_tests {
    use super::*;
    use egui_kittest::Harness;
    
    #[test]
    fn test_parameter_dialog_rendering() {
        let mut harness = Harness::new(|ctx| {
            let mut dialog = ParameterInputDialog::new();
            let test_params = create_test_parameters();
            dialog.show(ctx, &test_params);
        });
        
        harness.run();
        // Add specific UI assertions
    }
}
```

## Error Handling Patterns

### Consistent Error Types

```rust
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CloudFormationError {
    #[error("Template validation failed: {message}")]
    ValidationError { message: String },
    
    #[error("Deployment failed: {stack_name} - {reason}")]
    DeploymentError { stack_name: String, reason: String },
    
    #[error("Parameter error: {parameter_name} - {message}")]
    ParameterError { parameter_name: String, message: String },
    
    #[error("AWS service error: {service} - {message}")]
    AwsServiceError { service: String, message: String },
}
```

### Error Context Pattern

```rust
pub async fn deploy_stack(&self, params: DeploymentParams) -> Result<String> {
    let validation_result = self
        .validate_template(&params.template, &params.account_id, &params.region)
        .await
        .with_context(|| format!("Failed to validate template for stack {}", params.stack_name))?;
    
    if !validation_result.is_valid {
        return Err(CloudFormationError::ValidationError {
            message: format!("Template validation failed with {} errors", validation_result.errors.len())
        }.into());
    }
    
    // Continue with deployment...
}
```

## Performance Optimization

### Caching Patterns

**Resource Caching**:

```rust
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CachedResource<T> {
    data: T,
    cached_at: Instant,
    ttl: Duration,
}

impl<T> CachedResource<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            cached_at: Instant::now(),
            ttl,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        self.cached_at.elapsed() > self.ttl
    }
    
    pub fn get(&self) -> Option<&T> {
        if self.is_expired() {
            None
        } else {
            Some(&self.data)
        }
    }
}
```

**Async Operation Batching**:

```rust
use tokio::sync::mpsc;
use std::collections::HashMap;

pub struct BatchProcessor<T, R> {
    tx: mpsc::UnboundedSender<(T, oneshot::Sender<R>)>,
}

impl<T, R> BatchProcessor<T, R> {
    pub fn new<F, Fut>(batch_processor: F) -> Self 
    where
        F: Fn(Vec<T>) -> Fut + Send + 'static,
        Fut: Future<Output = Vec<R>> + Send,
        T: Send + 'static,
        R: Send + 'static,
    {
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        tokio::spawn(async move {
            let mut batch = Vec::new();
            let mut senders = Vec::new();
            
            while let Some((item, sender)) = rx.recv().await {
                batch.push(item);
                senders.push(sender);
                
                if batch.len() >= BATCH_SIZE {
                    let results = batch_processor(batch).await;
                    for (result, sender) in results.into_iter().zip(senders.into_iter()) {
                        let _ = sender.send(result);
                    }
                    batch = Vec::new();
                    senders = Vec::new();
                }
            }
        });
        
        Self { tx }
    }
    
    pub async fn process(&self, item: T) -> Result<R> {
        let (tx, rx) = oneshot::channel();
        self.tx.send((item, tx))?;
        rx.await.map_err(|_| anyhow::anyhow!("Batch processor dropped"))
    }
}
```

## Logging and Observability

### Structured Logging

```rust
use tracing::{debug, error, info, warn, instrument, Span};

impl CloudFormationManager {
    #[instrument(skip(self, template), fields(template_size = template.len()))]
    pub async fn validate_template(&self, template: &str, account_id: &str, region: &str) -> Result<ValidationResult> {
        let span = Span::current();
        span.record("account_id", account_id);
        span.record("region", region);
        
        debug!("Starting template validation");
        
        let result = self.perform_validation(template, account_id, region).await;
        
        match &result {
            Ok(validation_result) => {
                info!(
                    errors = validation_result.errors.len(),
                    warnings = validation_result.warnings.len(),
                    parameters = validation_result.parameters.len(),
                    "Template validation completed"
                );
            }
            Err(e) => {
                error!(error = %e, "Template validation failed");
            }
        }
        
        result
    }
}
```

### Metrics Collection

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct CloudFormationMetrics {
    validations_performed: AtomicU64,
    deployments_started: AtomicU64,
    deployments_completed: AtomicU64,
    deployments_failed: AtomicU64,
}

impl CloudFormationMetrics {
    pub fn increment_validations(&self) {
        self.validations_performed.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_validation_count(&self) -> u64 {
        self.validations_performed.load(Ordering::Relaxed)
    }
    
    // Similar methods for other metrics...
}
```

## Configuration Management

### Environment-Specific Configuration

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFormationConfig {
    pub default_region: String,
    pub validation_timeout_seconds: u64,
    pub deployment_timeout_minutes: u64,
    pub cache_ttl_minutes: u64,
    pub parameter_store_prefix: String,
    pub environment_configs: HashMap<String, EnvironmentConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub account_id: String,
    pub region: String,
    pub deployment_role_arn: Option<String>,
    pub parameter_store_prefix: String,
}

impl Default for CloudFormationConfig {
    fn default() -> Self {
        Self {
            default_region: "us-east-1".to_string(),
            validation_timeout_seconds: 30,
            deployment_timeout_minutes: 60,
            cache_ttl_minutes: 5,
            parameter_store_prefix: "/awsdash".to_string(),
            environment_configs: HashMap::new(),
        }
    }
}
```

## Security Best Practices

### Sensitive Data Handling

```rust
use zeroize::Zeroize;

#[derive(Debug, Zeroize)]
#[zeroize(drop)]
pub struct SensitiveParameter {
    name: String,
    value: String,
    source: ParameterSource,
}

impl SensitiveParameter {
    pub fn new(name: String, value: String, source: ParameterSource) -> Self {
        Self { name, value, source }
    }
    
    pub fn get_value(&self) -> &str {
        &self.value
    }
    
    // Automatically zeroized on drop
}
```

### Credential Management

```rust
pub async fn get_deployment_credentials(&self, account_id: &str) -> Result<aws_types::Credentials> {
    // Use existing credential coordinator
    let credentials = self.credential_coordinator
        .get_credentials_for_account(account_id)
        .await
        .context("Failed to get deployment credentials")?;
    
    // Validate credentials are not expired
    if credentials.expiry() < Some(SystemTime::now() + Duration::from_secs(300)) {
        return Err(anyhow::anyhow!("Credentials expire too soon for deployment"));
    }
    
    Ok(credentials)
}
```

## Debugging and Troubleshooting

### Debug Utilities

```rust
#[cfg(debug_assertions)]
pub mod debug {
    use super::*;
    
    pub fn dump_deployment_state(deployment: &DeploymentOperation) {
        println!("=== Deployment State Debug ===");
        println!("ID: {}", deployment.id);
        println!("State: {:?}", deployment.state);
        println!("Created: {:?}", deployment.created_at);
        println!("Stack Name: {}", deployment.stack_name);
        
        if let Some(stats) = &deployment.stats {
            println!("Stats: {:?}", stats);
        }
        
        println!("Events: {} total", deployment.events.len());
        for (i, event) in deployment.events.iter().enumerate().rev().take(5) {
            println!("  Event {}: {:?}", i, event);
        }
        println!("==============================");
    }
}
```

### Integration with Application Logging

The CloudFormation Manager integrates with the application's logging system located at `$HOME/.local/share/awsdash/logs/awsdash.log`. Use appropriate log levels:

- **TRACE**: Detailed operation flows, AWS API requests/responses
- **DEBUG**: State transitions, parameter processing, cache operations
- **INFO**: Successful operations, deployment milestones
- **WARN**: Recoverable errors, deprecated feature usage
- **ERROR**: Operation failures, AWS service errors

## Contributing Guidelines

### Code Review Checklist

**Functionality**:
- [ ] New feature follows established patterns
- [ ] Error handling is comprehensive and consistent
- [ ] AWS service integration follows security best practices
- [ ] UI components follow accessibility guidelines

**Testing**:
- [ ] Unit tests cover all new functionality
- [ ] Integration tests verify AWS service interaction
- [ ] UI tests validate component behavior
- [ ] Error scenarios are tested

**Documentation**:
- [ ] Public APIs have comprehensive rustdoc comments
- [ ] Complex algorithms are documented
- [ ] Configuration options are documented
- [ ] Integration points are clearly described

**Performance**:
- [ ] No unnecessary AWS API calls
- [ ] Appropriate caching is implemented
- [ ] Memory usage is reasonable for large templates
- [ ] UI remains responsive during operations

### Release Process

1. **Feature Development**: Implement feature with tests and documentation
2. **Code Review**: Peer review focusing on architecture and security
3. **Integration Testing**: Test with real AWS services in development account
4. **Performance Testing**: Validate performance with large templates and deployments
5. **Documentation Update**: Update technical documentation and user guides
6. **Release**: Tag release and update changelog

## Related Documentation

- [CloudFormation System](cloudformation-system.md) - Core CloudFormation integration
- [Parameter Patterns](parameter-patterns.md) - Parameter management patterns
- [UI Component Testing](ui-component-testing.md) - UI testing strategies
- [AWS Integration Patterns](aws-integration-patterns.md) - AWS service integration guidelines
- [System Architecture](system-architecture.md) - Overall system architecture