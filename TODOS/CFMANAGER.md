# CloudFormation Manager Implementation Plan

## Overview
Implementation plan for the CloudFormation Manager module - a comprehensive system for deploying, validating, and managing CloudFormation stacks with advanced parameter collection, AWS resource lookup, and Parameter Store/Secrets Manager integration.

## Architecture Summary
- **Module Location**: `src/app/cloudformation_manager/`
- **Integration Points**: Command Palette, AWS Explorer, Project Management, Credential Coordinator
- **Core Features**: Deploy, Validate, Parameter Collection with Lookup, Progress Monitoring

---

## Milestone 1: Foundation & Template Validation
**Goal**: Implement template validation with AWS CloudFormation service (Low risk, no breaking changes)

### Tasks

#### 1.1 Create CloudFormation Manager Module
- [x] Create `src/app/cloudformation_manager/mod.rs`
- [x] Create `src/app/cloudformation_manager/manager.rs`
- [x] Define basic `CloudFormationManager` struct
- [x] Integrate with existing `CredentialCoordinator`
- [x] Add to main app module imports

**Hints**:
- Follow existing module patterns in `src/app/resource_explorer/`
- Use `Arc<CredentialCoordinator>` for credential management
- Consider async operations from the start

#### 1.2 Implement Validate Template Command
- [x] Add `validate_template()` method to manager
- [x] Integration with `CloudFormationService` in AWS Explorer
- [x] Call AWS CloudFormation `ValidateTemplate` API
- [x] Parse validation errors and warnings
- [x] Handle network/credential errors gracefully

**Implementation Notes**:
```rust
pub struct CloudFormationManager {
    credential_coordinator: Arc<CredentialCoordinator>,
    active_operations: HashMap<String, OperationState>,
}

pub async fn validate_template(
    &self,
    template: &str,
    account_id: &str,
    region: &str,
) -> Result<ValidationResult, anyhow::Error>
```

#### 1.3 Hook into Command Palette
- [x] Modify `src/app/dashui/cloudformation_command_palette.rs`
- [x] Replace placeholder "Validate" action with real implementation
- [x] Add validation result handling
- [x] Show validation progress indicator

**Existing Code Location**: `CloudFormationAction::Validate` in command palette

#### 1.4 Add Validation UI Components
- [x] Create `validation_results_window.rs`
- [x] Display validation errors with line numbers
- [x] Show template warnings and suggestions
- [ ] Link errors to template editor positions (if applicable)
- [x] Add copy-to-clipboard for error messages

**UI Requirements**:
- Modal dialog or dedicated window
- Error list with severity levels
- Expandable error details
- Clear success/failure indication

---

## Milestone 2: Enhanced Parameter Collection System
**Goal**: Build robust parameter input UI with AWS resource lookup and Parameter Store integration

### Tasks

#### 2.1 Parameter Discovery System
- [x] Create `src/app/cloudformation_manager/parameters.rs`
- [x] Parse CloudFormation template parameters section
- [x] Identify parameter types, defaults, constraints
- [x] Detect NoEcho parameters (sensitive data)
- [x] Handle parameter dependencies and conditional parameters

**Parameter Types to Support**:
- String, Number, CommaDelimitedList
- AWS-specific types (AWS::EC2::VPC::Id, etc.)
- NoEcho parameters
- AllowedValues constraints
- AllowedPattern regex validation

#### 2.2 AWS Resource Lookup Integration
- [x] Create `src/app/cloudformation_manager/resource_lookup.rs`
- [x] Map CF parameter types to AWS Explorer resource types
- [x] Create `AwsResourcePickerDialog` component
- [x] Integrate with existing `AWSResourceClient`
- [x] Add fuzzy search for resource selection
- [x] Implement resource caching (5-minute TTL)

**Resource Type Mappings**:
```rust
pub fn get_resource_type_for_parameter(param_type: &str) -> Option<&str> {
    match param_type {
        "AWS::EC2::VPC::Id" => Some("AWS::EC2::VPC"),
        "AWS::EC2::Subnet::Id" => Some("AWS::EC2::Subnet"),
        "AWS::EC2::SecurityGroup::Id" => Some("AWS::EC2::SecurityGroup"),
        // ... more mappings
    }
}
```

#### 2.3 Parameter Store Integration
- [x] Create `src/app/cloudformation_manager/parameter_store.rs`
- [x] Implement SSM Parameter Store client
- [x] Add "Store as Default" functionality
- [x] Transform parameter types to `AWS::SSM::Parameter::Value<Type>`
- [x] Update template with new parameter definitions
- [x] Parameter naming convention: `/{app-name}/{environment}/{param-name}`

**Template Transformation Example**:
```yaml
# Before
DBInstanceType:
  Type: String
  Default: "db.t3.micro"

# After
DBInstanceType:
  Type: AWS::SSM::Parameter::Value<String>
  Default: /myapp/prod/db-instance-type
```

#### 2.4 Secrets Manager Integration
- [x] Create `src/app/cloudformation_manager/secrets_manager.rs`
- [x] Implement Secrets Manager client
- [x] Detect sensitive parameters (NoEcho, password patterns)
- [x] Store secrets and generate dynamic references
- [x] Transform template resources to use `{{resolve:secretsmanager:...}}`
- [x] Show template transformation preview

**Note**: Secrets Manager cannot be used in parameter defaults, only in resource properties.

#### 2.5 Enhanced Parameter Input Dialog
- [x] Create `src/app/cloudformation_manager/parameter_dialog.rs`
- [x] Type-specific input widgets (string, number, list, boolean)
- [x] Validation against AllowedValues/AllowedPattern
- [x] "Browse" button for AWS resource types
- [x] "Store as Default" dropdown menu
- [x] Parameter history for quick re-use
- [x] Show parameter descriptions and constraints

**UI Components**:
```rust
pub struct ParameterInputDialog {
    parameters: Vec<Parameter>,
    values: HashMap<String, String>,
    lookup_service: Arc<AWSResourceClient>,
    parameter_store: ParameterStoreManager,
    secrets_manager: SecretsManagerClient,
}
```

#### 2.6 Parameter Persistence
- [x] Save parameter values in project files
- [x] Per-environment parameter overrides
- [x] Parameter history for reuse
- [x] Default value sources (manual, Parameter Store, Secrets Manager)

---

## Milestone 3: Stack Deployment Core ✅
**Goal**: Implement actual stack deployment with progress tracking

### Tasks

#### 3.1 Deployment State Machine ✅
- [x] Create `src/app/cloudformation_manager/deployment.rs`
- [x] Define deployment states and transitions
- [x] Implement state persistence
- [x] Add operation cancellation support

```rust
pub enum DeploymentState {
    Collecting,      // Gathering parameters
    Validating,      // Pre-deployment validation
    Deploying,       // CreateStack/UpdateStack in progress
    Monitoring,      // Watching stack events
    Complete(bool),  // Success/Failure
    Cancelled,       // User cancelled
}

pub struct DeploymentOperation {
    id: String,
    stack_name: String,
    account_id: String,
    region: String,
    state: DeploymentState,
    events: Vec<StackEvent>,
    start_time: SystemTime,
}
```

#### 3.2 Deploy Command Implementation ✅
- [x] Add `deploy_stack()` method to manager
- [x] Environment/Account selection from project
- [x] Stack name input with validation
- [x] Parameter collection using enhanced dialog
- [x] Detect stack existence (CreateStack vs UpdateStack)
- [x] Handle stack capabilities (IAM, CAPABILITY_NAMED_IAM)

**Integration Points**:
- Use project's selected environment for region/account
- Leverage existing credential coordinator
- Integrate with parameter collection system from Milestone 2

#### 3.3 Deployment Monitoring System ✅
- [x] ~~Create `src/app/cloudformation_manager/monitoring.rs`~~ (Implemented in manager.rs)
- [x] Real-time stack event streaming
- [x] Progress tracking for resource creation/updates
- [x] Event log with timestamps and status
- [x] Stack output collection after completion

**Monitoring Features**:
- WebSocket-like event streaming using polling
- Resource-level progress indication
- Real-time status updates
- Rollback detection

#### 3.4 Deployment UI Components ✅
- [x] Create `deployment_progress_window.rs`
- [x] Progress bar showing deployment status
- [x] Live event log display
- [x] Cancel deployment button
- [x] Stack outputs display
- [x] Export deployment logs

#### 3.5 Error Handling & Recovery ✅
- [x] Comprehensive error handling for all failure modes
- [x] Rollback detection and notification
- [x] Failed resource diagnostics
- [x] Retry failed deployments
- [x] Export error logs and stack events

**Error Scenarios to Handle**:
- Network connectivity issues
- Credential expiration
- CloudFormation service errors
- Resource limit exceeded
- Stack rollback scenarios
- Insufficient permissions

---

## Milestone 4: Advanced Deployment Features
**Goal**: Add production-ready deployment capabilities

### Tasks

#### 4.1 Change Set Support
- [ ] Create `src/app/cloudformation_manager/changesets.rs`
- [ ] Create change sets before deployment
- [ ] Preview changes in diff view
- [ ] Accept/reject change sets
- [ ] Change set history tracking

#### 4.2 Stack Policy Management
- [ ] Define stack policies in project configuration
- [ ] Apply policies during deployment
- [ ] Policy violation warnings
- [ ] Policy template library

#### 4.3 Deployment Roles & Capabilities
- [ ] Configure deployment role per environment
- [ ] Service role selection
- [ ] Role capability validation
- [ ] Cross-account deployment support

#### 4.4 Multi-Stack Dependencies
- [ ] Deploy dependent stacks in correct order
- [ ] Cross-stack reference validation
- [ ] Nested stack support
- [ ] Dependency visualization

---

## Technical Implementation Guidelines

### Code Organization
```
src/app/cloudformation_manager/
├── mod.rs                  # Module exports
├── manager.rs              # Main CloudFormationManager
├── parameters.rs           # Parameter discovery & validation
├── resource_lookup.rs      # AWS resource integration
├── parameter_store.rs      # SSM Parameter Store client
├── secrets_manager.rs      # Secrets Manager client
├── deployment.rs           # Deployment state machine
├── monitoring.rs           # Stack event monitoring
├── changesets.rs           # Change set management
└── ui/
    ├── parameter_dialog.rs     # Parameter input UI
    ├── validation_results.rs   # Validation display
    ├── deployment_progress.rs  # Deployment monitoring UI
    └── resource_picker.rs      # AWS resource picker
```

### Integration Patterns
- **Credential Management**: Use `Arc<CredentialCoordinator>`
- **AWS API Calls**: Follow existing patterns in `resource_explorer/aws_services/`
- **UI Components**: Follow egui patterns in `dashui/`
- **Error Handling**: Use `anyhow` with context
- **Async Operations**: Use Tokio runtime with proper error handling

### Testing Strategy
- [ ] Unit tests for parameter parsing/validation
- [ ] Integration tests with mock AWS responses
- [ ] UI tests for dialog interactions
- [ ] End-to-end deployment flow tests
- [ ] Error scenario testing

### Performance Considerations
- [ ] Cache AWS Explorer results (5-minute TTL)
- [ ] Async operations for all AWS API calls
- [ ] Pagination for large resource lists
- [ ] Background refresh for resource data
- [ ] Efficient event streaming for monitoring

### Security Requirements
- [ ] Never log sensitive parameter values
- [ ] Secure credential handling
- [ ] Validate all user inputs
- [ ] Safe template transformations
- [ ] Audit trail for parameter storage

---

## Implementation Order & Timeline

### Week 1: Foundation
- [x] Milestone 1 complete (Validation functionality)
- [x] Basic CloudFormation Manager structure

### Week 2: Parameters
- [x] Parameter discovery and basic input dialog
- [x] AWS resource lookup integration

### Week 3: Parameter Store
- [x] Parameter Store and Secrets Manager integration
- [x] Enhanced parameter dialog with all features

### Week 4: Deployment Core
- [ ] Basic deployment functionality
- [ ] State machine implementation

### Week 5: Monitoring & Polish
- [ ] Real-time monitoring
- [ ] Error handling improvements
- [ ] UI polish and testing

### Future: Advanced Features
- [ ] Change sets
- [ ] Multi-stack support
- [ ] Advanced policy management

---

## Risk Mitigation

### High Risk Items
- [ ] AWS API rate limiting during resource lookup
- [ ] Credential expiration during long deployments
- [ ] Complex parameter dependencies
- [ ] Template transformation edge cases

### Mitigation Strategies
- [ ] Implement retry logic with exponential backoff
- [ ] Proactive credential refresh
- [ ] Comprehensive validation before deployment
- [ ] Template backup before modifications

---

## Success Criteria

### Milestone 1 Success
- [x] Template validation works with real AWS API
- [x] Errors are clearly displayed to users
- [x] No breaking changes to existing functionality

### Milestone 2 Success
- [ ] All parameter types supported with appropriate UI
- [ ] AWS resource lookup works smoothly
- [ ] Parameter Store integration transforms templates correctly

### Milestone 3 Success
- [ ] Successful stack deployment from start to finish
- [ ] Real-time progress monitoring
- [ ] Comprehensive error handling

### Final Success
- [ ] Production-ready CloudFormation deployment workflow
- [ ] Significantly improved user experience over AWS Console
- [ ] Seamless integration with existing AWS Dash features

---

## Notes & Reminders

### Command Palette Integration
- Existing actions: Deploy (placeholder), Import (working), Validate (placeholder), Edit (working)
- Need to replace placeholders with real implementations

### Existing Codebase Leverage
- Use `CloudFormationService` for AWS API calls
- Use `CredentialCoordinator` for authentication
- Use existing fuzzy search patterns
- Follow UI patterns from `dashui/` modules

### Documentation Updates
- [ ] Update main README with CloudFormation Manager features
- [ ] Add user guide for deployment workflow
- [ ] Document new keyboard shortcuts
- [ ] Add troubleshooting guide

### Performance Targets
- Parameter dialog opens in <500ms
- AWS resource lookup completes in <2s
- Deployment monitoring updates every 5s
- Template validation completes in <10s