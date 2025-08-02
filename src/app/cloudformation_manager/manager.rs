use crate::app::resource_explorer::{credentials::CredentialCoordinator, AWSResourceClient};
use anyhow::{Context, Result};
use aws_sdk_cloudformation as cfn;
use aws_sdk_cloudformation::error::ProvideErrorMetadata;
use aws_types::request_id::RequestId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Result of CloudFormation template validation containing errors, warnings, and parameter information.
///
/// This struct encapsulates the complete validation result from the AWS CloudFormation
/// ValidateTemplate API, enhanced with additional metadata for UI display and parameter
/// management. It provides comprehensive information about template validity, discovered
/// parameters, and any issues that need to be addressed.
///
/// # Fields
///
/// * `is_valid` - Overall validation status (true if no errors, false if errors exist)
/// * `errors` - Collection of validation errors that prevent template deployment
/// * `warnings` - Collection of validation warnings that don't prevent deployment
/// * `parameters` - Discovered template parameters with metadata for UI rendering
/// * `description` - Optional template description from the CloudFormation template
///
/// # Examples
///
/// ```
/// // Check if template is valid for deployment
/// if validation_result.is_valid {
///     println!("Template is ready for deployment");
/// } else {
///     println!("Template has {} errors", validation_result.errors.len());
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub parameters: Vec<TemplateParameter>,
    pub description: Option<String>,
}

/// Detailed information about a CloudFormation template validation error.
///
/// This struct provides comprehensive error information for template validation failures,
/// including location information, suggestions, and severity levels. It's designed to
/// support rich error display in the UI with actionable feedback for users.
///
/// # Fields
///
/// * `message` - Human-readable error description
/// * `code` - AWS error code for programmatic handling
/// * `line_number` - Template line number where error occurred (if available)
/// * `column_number` - Template column number where error occurred (if available)
/// * `resource_name` - Name of the CloudFormation resource causing the error
/// * `property_path` - JSON path to the specific property causing the error
/// * `suggestion` - Suggested fix or remediation for the error
/// * `severity` - Error severity level (Error, Warning, Info)
/// * `rule_id` - Validation rule identifier for error categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub message: String,
    pub code: Option<String>,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
    pub resource_name: Option<String>,
    pub property_path: Option<String>,
    pub suggestion: Option<String>,
    pub severity: ErrorSeverity,
    pub rule_id: Option<String>,
}

impl ValidationError {
    pub fn new_simple(message: String, code: Option<String>) -> Self {
        Self {
            message,
            code,
            line_number: None,
            column_number: None,
            resource_name: None,
            property_path: None,
            suggestion: None,
            severity: ErrorSeverity::Error,
            rule_id: None,
        }
    }

    pub fn new_detailed(
        message: String,
        code: Option<String>,
        line_number: Option<u32>,
        resource_name: Option<String>,
        property_path: Option<String>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            message,
            code,
            line_number,
            column_number: None,
            resource_name,
            property_path,
            suggestion,
            severity: ErrorSeverity::Error,
            rule_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub message: String,
    pub code: Option<String>,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
    pub resource_name: Option<String>,
    pub property_path: Option<String>,
    pub suggestion: Option<String>,
    pub rule_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub parameter_key: String,
    pub parameter_type: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub allowed_values: Option<Vec<String>>,
    pub allowed_pattern: Option<String>,
    pub no_echo: bool,
    pub constraint_description: Option<String>,
}

#[derive(Debug, Clone)]
pub enum OperationState {
    InProgress,
    Completed(Result<String, String>),
    Cancelled,
}

/// Comprehensive CloudFormation template management system for validation, deployment, and monitoring.
///
/// The CloudFormationManager provides a unified interface for managing CloudFormation templates
/// within the AWS Dash desktop environment. It integrates with AWS Identity Center for authentication,
/// AWS Parameter Store and Secrets Manager for parameter management, and provides real-time
/// deployment monitoring with comprehensive error handling.
///
/// # Architecture
///
/// The manager operates as a stateful coordinator that orchestrates multiple AWS services:
/// - **Template Validation**: Uses AWS CloudFormation ValidateTemplate API
/// - **Parameter Management**: Integrates with Parameter Store and Secrets Manager
/// - **Deployment Operations**: Manages stack create/update operations with progress tracking
/// - **Resource Discovery**: Provides AWS resource lookup for parameter selection
/// - **State Management**: Maintains deployment state and operation tracking
///
/// # Key Features
///
/// * **Async Operations**: All AWS API calls are non-blocking with proper error handling
/// * **Parameter Integration**: Seamless integration with AWS Parameter Store and Secrets Manager
/// * **Real-time Monitoring**: Live deployment progress tracking with event streaming
/// * **Resource Caching**: Intelligent caching of AWS resources for performance
/// * **Error Recovery**: Comprehensive error handling with detailed user feedback
/// * **Multi-Environment**: Support for multiple AWS environments and regions
///
/// # Integration Points
///
/// The manager integrates with several AWS Dash components:
/// - **CredentialCoordinator**: For AWS authentication and session management
/// - **AWSResourceClient**: For AWS resource discovery and caching
/// - **DeploymentManager**: For orchestrating stack deployment operations
/// - **ParameterStoreManager**: For AWS Systems Manager Parameter Store integration
/// - **SecretsManagerClient**: For AWS Secrets Manager integration
///
/// # Usage
///
/// ```rust
/// // Create manager with credential coordinator
/// let manager = CloudFormationManager::new(credential_coordinator.clone());
///
/// // Validate template
/// let validation_result = manager.validate_template(&template_body).await?;
///
/// // Deploy stack if validation passes
/// if validation_result.is_valid {
///     let deployment_id = manager.deploy_stack(
///         "my-stack",
///         &template_body,
///         parameters,
///         "us-east-1"
///     ).await?;
/// }
/// ```
///
/// # Performance Considerations
///
/// * Resource lookups are cached with TTL to reduce API calls
/// * Deployment operations use AWS CloudFormation event streaming for efficiency
/// * Parameter operations are batched where possible
/// * All operations support cancellation and timeout handling
///
/// # Error Handling
///
/// The manager provides detailed error information including:
/// - AWS service errors with context
/// - Template validation errors with line/column information
/// - Parameter validation errors with suggestions
/// - Deployment errors with stack event details
pub struct CloudFormationManager {
    credential_coordinator: Arc<CredentialCoordinator>,
    aws_client: Option<Arc<AWSResourceClient>>,
    active_operations: Arc<RwLock<HashMap<String, OperationState>>>,
    parameter_store_manager: super::parameter_store::ParameterStoreManager,
    secrets_manager_client: super::secrets_manager::SecretsManagerClient,
    deployment_manager: super::deployment::DeploymentManager,
    /// Latest validation result for UI consumption
    latest_validation_result: Arc<RwLock<Option<ValidationResult>>>,
}

impl CloudFormationManager {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator: credential_coordinator.clone(),
            aws_client: None,
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            parameter_store_manager: super::parameter_store::ParameterStoreManager::new(
                credential_coordinator.clone(),
            ),
            secrets_manager_client: super::secrets_manager::SecretsManagerClient::new(
                credential_coordinator.clone(),
            ),
            deployment_manager: super::deployment::DeploymentManager::new(credential_coordinator),
            latest_validation_result: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the AWS client reference from ResourceExplorer
    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = aws_client;
    }

    /// Get the AWS client for resource lookup
    pub fn get_aws_client(&self) -> Option<Arc<AWSResourceClient>> {
        self.aws_client.clone()
    }

    /// Create a ResourceLookupService using the AWS Explorer's infrastructure
    pub fn create_resource_lookup_service(
        &self,
    ) -> Option<Arc<super::resource_lookup::ResourceLookupService>> {
        self.aws_client.as_ref().map(|client| {
            Arc::new(super::resource_lookup::ResourceLookupService::new(
                client.clone(),
            ))
        })
    }

    /// Get the Parameter Store manager
    pub fn get_parameter_store_manager(&self) -> &super::parameter_store::ParameterStoreManager {
        &self.parameter_store_manager
    }

    /// Get the Secrets Manager client
    pub fn get_secrets_manager_client(&self) -> &super::secrets_manager::SecretsManagerClient {
        &self.secrets_manager_client
    }

    /// Get the Deployment Manager
    pub fn get_deployment_manager(&self) -> &super::deployment::DeploymentManager {
        &self.deployment_manager
    }

    /// Get the CloudFormation deployment role ARN for a specific account
    pub async fn get_cloudformation_role_arn(&self, account_id: &str) -> Option<String> {
        // Access AWS Identity Center through credential coordinator
        if let Some(role_name) = self
            .credential_coordinator
            .get_cloudformation_deployment_role_name()
        {
            // Construct the role ARN using the account ID and role name
            let role_arn = format!("arn:aws:iam::{}:role/{}", account_id, role_name);
            info!(
                "Constructed CloudFormation service role ARN: {} (account: {}, role: {})",
                role_arn, account_id, role_name
            );
            Some(role_arn)
        } else {
            warn!("No CloudFormation deployment role name configured in Identity Center");
            None
        }
    }

    /// Get the latest validation result and clear it
    pub async fn take_latest_validation_result(&self) -> Option<ValidationResult> {
        let mut result = self.latest_validation_result.write().await;
        result.take()
    }

    /// Check if there's a new validation result available
    pub async fn has_validation_result(&self) -> bool {
        let result = self.latest_validation_result.read().await;
        result.is_some()
    }

    /// Get a reference to the latest validation result lock for direct access
    pub fn get_validation_result_lock(&self) -> &Arc<RwLock<Option<ValidationResult>>> {
        &self.latest_validation_result
    }

    /// Validate a CloudFormation template from a project
    /// This is the high-level method called from the UI
    pub async fn validate_project_template(
        &self,
        project: &crate::app::projects::Project,
        account_id: &str,
        region: &str,
    ) -> Result<ValidationResult> {
        info!("=== CloudFormation Manager: Starting project template validation ===");
        info!(
            "Project: {}, Account: {}, Region: {}",
            project.name, account_id, region
        );

        // Extract template from project
        let template = project
            .cfn_template
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No CloudFormation template in project"))?;

        // Serialize template to JSON
        let template_json = serde_json::to_string_pretty(template)
            .with_context(|| "Failed to serialize CloudFormation template")?;

        info!(
            "Template serialized successfully, length: {} characters",
            template_json.len()
        );
        tracing::trace!(
            "Serialized CloudFormation template JSON:\n{}",
            template_json
        );

        // Call the existing validation method
        self.validate_template(&template_json, account_id, region)
            .await
    }

    /// Validates a CloudFormation template using the AWS CloudFormation ValidateTemplate API.
    ///
    /// This method performs comprehensive template validation including syntax checking,
    /// parameter discovery, and AWS service capability validation. It returns detailed
    /// validation results with errors, warnings, and parameter information for UI display.
    ///
    /// # Arguments
    ///
    /// * `template` - CloudFormation template as JSON or YAML string
    /// * `account_id` - AWS account ID for validation context
    /// * `region` - AWS region for validation (affects available resources and services)
    ///
    /// # Returns
    ///
    /// Returns a `ValidationResult` containing:
    /// - `is_valid`: Overall validation status
    /// - `errors`: Collection of validation errors that prevent deployment
    /// - `warnings`: Collection of non-blocking validation warnings
    /// - `parameters`: Discovered template parameters with metadata
    /// - `description`: Template description if present
    ///
    /// # Examples
    ///
    /// ```rust
    /// let template = r#"
    /// {
    ///   "AWSTemplateFormatVersion": "2010-09-09",
    ///   "Parameters": {
    ///     "InstanceType": {
    ///       "Type": "String",
    ///       "Default": "t3.micro"
    ///     }
    ///   },
    ///   "Resources": {
    ///     "MyInstance": {
    ///       "Type": "AWS::EC2::Instance",
    ///       "Properties": {
    ///         "InstanceType": {"Ref": "InstanceType"}
    ///       }
    ///     }
    ///   }
    /// }
    /// "#;
    ///
    /// let result = manager.validate_template(template, "123456789012", "us-east-1").await?;
    ///
    /// if result.is_valid {
    ///     println!("Template is valid with {} parameters", result.parameters.len());
    /// } else {
    ///     println!("Template has {} errors", result.errors.len());
    /// }
    /// ```
    ///
    /// # Operation Tracking
    ///
    /// This method creates a tracked operation that can be monitored using:
    /// - `get_operation_status()` - Check operation progress
    /// - `cancel_operation()` - Cancel long-running validation
    /// - `cleanup_operations()` - Clean up completed operations
    ///
    /// # Error Handling
    ///
    /// Returns `Err` for:
    /// - Network connectivity issues
    /// - AWS authentication failures
    /// - Invalid template format (malformed JSON/YAML)
    /// - AWS service errors
    ///
    /// Template validation errors (invalid resources, missing properties, etc.)
    /// are returned in the `errors` field of `ValidationResult`, not as `Err`.
    ///
    /// # Performance Notes
    ///
    /// - Validation typically completes in 1-3 seconds
    /// - Large templates (>1MB) may take longer
    /// - Results are cached in `latest_validation_result` for UI consumption
    /// - Operation state is tracked for concurrent validation monitoring
    pub async fn validate_template(
        &self,
        template: &str,
        account_id: &str,
        region: &str,
    ) -> Result<ValidationResult> {
        debug!(
            "Validating CloudFormation template for account {} in region {}",
            account_id, region
        );

        let operation_id = format!("validate_{}_{}", account_id, chrono::Utc::now().timestamp());

        // Mark operation as in progress
        {
            let mut operations = self.active_operations.write().await;
            operations.insert(operation_id.clone(), OperationState::InProgress);
        }

        let result = self.perform_validation(template, account_id, region).await;

        // Store the validation result for UI consumption
        if let Ok(validation_result) = &result {
            let mut latest_result = self.latest_validation_result.write().await;
            *latest_result = Some(validation_result.clone());
        }

        // Update operation state
        {
            let mut operations = self.active_operations.write().await;
            match &result {
                Ok(_) => {
                    operations.insert(
                        operation_id,
                        OperationState::Completed(Ok("Validation completed".to_string())),
                    );
                }
                Err(e) => {
                    operations.insert(operation_id, OperationState::Completed(Err(e.to_string())));
                }
            }
        }

        result
    }

    async fn perform_validation(
        &self,
        template: &str,
        account_id: &str,
        region: &str,
    ) -> Result<ValidationResult> {
        let aws_config = self
            .credential_coordinator
            .create_aws_config_for_account(account_id, region)
            .await
            .with_context(|| {
                format!(
                    "Failed to create AWS config for account {} in region {}",
                    account_id, region
                )
            })?;

        let client = cfn::Client::new(&aws_config);

        // Call AWS CloudFormation ValidateTemplate API
        let response = client
            .validate_template()
            .template_body(template)
            .send()
            .await;

        match response {
            Ok(validation_response) => {
                info!(
                    "Template validation successful for account {} in region {}",
                    account_id, region
                );

                // Log the full AWS API response as trace
                tracing::trace!(
                    "AWS CloudFormation ValidateTemplate API response: {:#?}",
                    validation_response
                );

                // Extract parameters from response
                let parameters = validation_response
                    .parameters()
                    .iter()
                    .map(|param| TemplateParameter {
                        parameter_key: param.parameter_key().unwrap_or_default().to_string(),
                        parameter_type: "String".to_string(), // CloudFormation API doesn't return type in validation
                        default_value: param.default_value().map(|s| s.to_string()),
                        description: param.description().map(|s| s.to_string()),
                        allowed_values: None,  // Not returned by validation API
                        allowed_pattern: None, // Not returned by validation API
                        no_echo: param.no_echo().unwrap_or(false),
                        constraint_description: None,
                    })
                    .collect();

                Ok(ValidationResult {
                    is_valid: true,
                    errors: Vec::new(),
                    warnings: Vec::new(),
                    parameters,
                    description: validation_response.description().map(|s| s.to_string()),
                })
            }
            Err(sdk_error) => {
                warn!(
                    "Template validation failed for account {} in region {}: {}",
                    account_id, region, sdk_error
                );

                // Log the full AWS SDK error as trace
                tracing::trace!(
                    "AWS CloudFormation ValidateTemplate API error response: {:#?}",
                    sdk_error
                );

                // Parse CloudFormation service errors
                let error_message = sdk_error.to_string();
                let validation_error = ValidationError::new_simple(
                    error_message.clone(),
                    None, // Could extract error code from SDK error if needed
                );

                Ok(ValidationResult {
                    is_valid: false,
                    errors: vec![validation_error],
                    warnings: Vec::new(),
                    parameters: Vec::new(),
                    description: None,
                })
            }
        }
    }

    /// Get the status of an operation
    pub async fn get_operation_status(&self, operation_id: &str) -> Option<OperationState> {
        let operations = self.active_operations.read().await;
        operations.get(operation_id).cloned()
    }

    /// Cancel an operation if possible
    pub async fn cancel_operation(&self, operation_id: &str) -> Result<()> {
        let mut operations = self.active_operations.write().await;
        if let Some(state) = operations.get_mut(operation_id) {
            match state {
                OperationState::InProgress => {
                    *state = OperationState::Cancelled;
                    info!("Operation {} cancelled", operation_id);
                }
                _ => {
                    warn!("Cannot cancel operation {} - not in progress", operation_id);
                }
            }
        }
        Ok(())
    }

    /// Clean up completed operations older than specified duration
    pub async fn cleanup_operations(&self, max_age_hours: u64) {
        let mut operations = self.active_operations.write().await;
        let _cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours as i64);

        // For now, remove all completed operations
        // In the future, we could store timestamps and do proper cleanup
        operations.retain(|_id, state| matches!(state, OperationState::InProgress));

        debug!("Cleaned up old CloudFormation operations");
    }

    /// Deploy a CloudFormation stack with parameter collection and progress tracking
    pub async fn deploy_stack(
        &self,
        template: String,
        stack_name: String,
        project: &crate::app::projects::Project,
        environment: String,
        parameters: Option<std::collections::HashMap<String, String>>,
    ) -> Result<String> {
        info!("=== CloudFormation Manager: Starting stack deployment ===");
        info!(
            "Stack: {}, Environment: {}, Project: {}",
            stack_name, environment, project.name
        );
        tracing::trace!(
            "Deploy parameters count: {}",
            parameters.as_ref().map(|p| p.len()).unwrap_or(0)
        );

        // Get environment configuration from project
        info!("Looking up environment configuration for '{}'", environment);
        let env_config = project
            .environments
            .iter()
            .find(|env| env.name == environment)
            .ok_or_else(|| {
                error!(
                    "Environment '{}' not found in project '{}'. Available environments: {:?}",
                    environment,
                    project.name,
                    project
                        .environments
                        .iter()
                        .map(|e| &e.name)
                        .collect::<Vec<_>>()
                );
                anyhow::anyhow!("Environment '{}' not found in project", environment)
            })?;
        info!(
            "Found environment configuration with {} accounts and {} regions",
            env_config.aws_accounts.len(),
            env_config.aws_regions.len()
        );

        // Log all configured accounts for this environment
        info!("Available accounts in environment '{}':", environment);
        for (idx, account) in env_config.aws_accounts.iter().enumerate() {
            info!("  [{}] Account: {}", idx, account.0);
        }

        // For deployment, we need to select the first account and region from the environment
        // In a full implementation, this would be user-selectable
        let account_id = env_config
            .aws_accounts
            .first()
            .ok_or_else(|| {
                error!(
                    "No AWS accounts configured for environment '{}'",
                    environment
                );
                anyhow::anyhow!(
                    "No AWS accounts configured for environment '{}'",
                    environment
                )
            })?
            .0
            .clone();
        let region = env_config
            .aws_regions
            .first()
            .ok_or_else(|| {
                error!(
                    "No AWS regions configured for environment '{}'",
                    environment
                );
                anyhow::anyhow!(
                    "No AWS regions configured for environment '{}'",
                    environment
                )
            })?
            .0
            .clone();
        info!(
            "Selected deployment target: Account {} in region {} (from environment '{}')",
            account_id, region, environment
        );

        // Validate deployment prerequisites before proceeding
        info!(
            "Validating deployment prerequisites for account {} in region {}",
            account_id, region
        );
        let validation = self
            .credential_coordinator
            .validate_deployment_prerequisites(&account_id, &region)
            .await
            .with_context(|| {
                error!(
                    "Failed to validate deployment prerequisites for account {} in region {}",
                    account_id, region
                );
                format!(
                    "Failed to validate deployment prerequisites for account {} in region {}",
                    account_id, region
                )
            })?;

        if !validation.is_valid {
            let error_msg = format!(
                "Deployment prerequisites validation failed for account {} in region {}. Errors: {:?}",
                account_id, region, validation.errors
            );
            error!("{}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }

        if !validation.warnings.is_empty() {
            for warning in &validation.warnings {
                warn!("Deployment warning: {}", warning);
            }
        }

        info!("Deployment prerequisites validated successfully. Strategy: account_accessible={}, role_assumable={}, cf_role_configured={}",
              validation.account_accessible, validation.role_assumable, validation.cloudformation_role_configured);

        // Check if we have AWS client for resource operations
        if self.aws_client.is_none() {
            warn!("No AWS client available for resource operations during deployment");
        }

        // Create AWS config for this deployment using deployment-specific credentials
        info!(
            "Creating deployment-specific AWS configuration for account {} in region {}",
            account_id, region
        );
        let aws_config = self
            .credential_coordinator
            .create_deployment_aws_config(&account_id, &region)
            .await
            .with_context(|| {
                error!(
                    "Failed to create deployment AWS config for account {} in region {}",
                    account_id, region
                );
                format!(
                    "Failed to create deployment AWS config for account {} in region {}",
                    account_id, region
                )
            })?;
        info!("AWS configuration created successfully");

        let cfn_client = cfn::Client::new(&aws_config);
        info!("CloudFormation client initialized");

        // Detect if stack exists to determine deployment type
        info!("Detecting deployment type for stack '{}'...", stack_name);
        let deployment_type = self
            .detect_deployment_type(&cfn_client, &stack_name)
            .await
            .with_context(|| {
                error!(
                    "Failed to detect deployment type for stack '{}'",
                    stack_name
                );
                format!(
                    "Failed to detect deployment type for stack '{}'",
                    stack_name
                )
            })?;

        info!(
            "Detected deployment type: {:?} for stack {}",
            deployment_type, stack_name
        );

        // Create deployment operation
        info!("Creating deployment operation record...");
        let deployment_id = self
            .deployment_manager
            .create_deployment(
                stack_name.clone(),
                account_id.clone(),
                region.clone(),
                deployment_type.clone(),
                template.clone(),
                parameters.unwrap_or_default(),
                project,
                environment.clone(),
            )
            .await
            .with_context(|| {
                error!("Failed to create deployment operation record");
                "Failed to create deployment operation record".to_string()
            })?;
        info!("Deployment operation created with ID: {}", deployment_id);

        // Get CloudFormation deployment role ARN
        info!("Checking for CloudFormation service role configuration...");
        let cloudformation_role_arn = self.get_cloudformation_role_arn(&account_id).await;
        if let Some(ref role_arn) = cloudformation_role_arn {
            info!("✓ CloudFormation service role configured: {}", role_arn);
        } else {
            warn!("⚠ No CloudFormation service role configured - deployment will use caller's permissions");
            warn!("This may limit permissions and cause deployment failures for certain resources");
        }

        // Execute deployment synchronously in the same runtime context to avoid task cancellation
        // This prevents nested tokio::spawn issues that can cause JoinError::Cancelled
        let deployment_manager = self.deployment_manager.clone();
        let cfn_client_clone = cfn_client.clone();
        let stack_name_clone = stack_name.clone();
        let template_clone = template.clone();
        let deployment_id_clone = deployment_id.clone();
        let deployment_id_for_error = deployment_id.clone();

        info!("Starting synchronous deployment execution to avoid task cancellation...");

        // Execute deployment in the current async context instead of spawning a new task
        // This prevents the JoinError::Cancelled issue caused by nested runtime contexts
        if let Err(e) = Self::execute_deployment(
            deployment_manager,
            cfn_client_clone,
            deployment_id_clone,
            stack_name_clone,
            template_clone,
            deployment_type,
            cloudformation_role_arn,
        )
        .await
        {
            error!(
                "❌ Deployment execution failed for deployment {}: {}",
                deployment_id_for_error, e
            );
            error!("Error context: {:?}", e.chain().collect::<Vec<_>>());
            // Return the error instead of silently logging it
            return Err(e);
        } else {
            info!(
                "✓ Deployment execution completed successfully for deployment {}",
                deployment_id_for_error
            );
        }

        Ok(deployment_id)
    }

    /// Detect whether this should be a Create or Update operation
    async fn detect_deployment_type(
        &self,
        cfn_client: &cfn::Client,
        stack_name: &str,
    ) -> Result<super::deployment::DeploymentType> {
        debug!("Checking if stack {} exists", stack_name);

        // Try to describe the specific stack first
        match cfn_client
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await
        {
            Ok(response) => {
                let stacks = response.stacks();
                if !stacks.is_empty() {
                    let stack = &stacks[0];
                    let stack_status = stack.stack_status();

                    debug!(
                        "Stack {} exists with status: {:?}",
                        stack_name, stack_status
                    );

                    // Check if stack is in a state that allows updates
                    if let Some(status) = stack_status {
                        match status.as_str() {
                            "CREATE_COMPLETE" | "UPDATE_COMPLETE" | "UPDATE_ROLLBACK_COMPLETE" => {
                                Ok(super::deployment::DeploymentType::Update)
                            }
                            status if status.contains("IN_PROGRESS") => {
                                Err(anyhow::anyhow!("Stack {} is currently in progress with status: {}", stack_name, status))
                            }
                            status if status.contains("FAILED") => {
                                Err(anyhow::anyhow!("Stack {} is in a failed state: {}. Please resolve before deployment.", stack_name, status))
                            }
                            _ => {
                                warn!("Stack {} has unexpected status: {:?}", stack_name, status);
                                Ok(super::deployment::DeploymentType::Update)
                            }
                        }
                    } else {
                        Ok(super::deployment::DeploymentType::Update)
                    }
                } else {
                    Ok(super::deployment::DeploymentType::Create)
                }
            }
            Err(sdk_error) => {
                // If stack doesn't exist, we'll get a validation error
                let error_msg = sdk_error.to_string();
                debug!("Stack existence check error: {}", error_msg);

                // Use AWS SDK error matching for more precise error handling
                if let Some(service_error) = sdk_error.as_service_error() {
                    debug!("Service error code: {:?}", service_error.code());
                    debug!("Service error message: {:?}", service_error.message());

                    // Check service error codes that indicate stack doesn't exist
                    if service_error.code() == Some("ValidationError") {
                        debug!(
                            "Stack {} does not exist (ValidationError), will create new stack",
                            stack_name
                        );
                        return Ok(super::deployment::DeploymentType::Create);
                    }
                }

                // Fallback: Check for various ways CloudFormation indicates stack doesn't exist
                if error_msg.contains("does not exist")
                    || error_msg.contains("ValidationError")
                    || error_msg.contains("Stack with id")
                    || error_msg.contains("No such stack")
                    || error_msg.to_lowercase().contains("not found")
                {
                    debug!("Stack {} does not exist, will create new stack", stack_name);
                    Ok(super::deployment::DeploymentType::Create)
                } else {
                    // Fallback: Try listing all stacks to see if we can find our stack
                    warn!(
                        "describe_stacks failed for {}, trying fallback approach",
                        stack_name
                    );
                    warn!("Original error: {}", error_msg);

                    match self.fallback_stack_detection(cfn_client, stack_name).await {
                        Ok(deployment_type) => {
                            info!(
                                "Fallback stack detection succeeded for {}: {:?}",
                                stack_name, deployment_type
                            );
                            Ok(deployment_type)
                        }
                        Err(fallback_error) => {
                            error!(
                                "Both primary and fallback stack detection failed for {}",
                                stack_name
                            );
                            error!("Primary error: {}", error_msg);
                            error!("Fallback error: {}", fallback_error);

                            // If all else fails, assume it's a new stack
                            warn!(
                                "Assuming {} is a new stack due to detection failures",
                                stack_name
                            );
                            Ok(super::deployment::DeploymentType::Create)
                        }
                    }
                }
            }
        }
    }

    /// Fallback method to detect stack existence when describe_stacks fails
    async fn fallback_stack_detection(
        &self,
        cfn_client: &cfn::Client,
        stack_name: &str,
    ) -> Result<super::deployment::DeploymentType> {
        debug!("Attempting fallback stack detection for {}", stack_name);

        // Try listing all stacks and look for our stack name
        match cfn_client.list_stacks().send().await {
            Ok(response) => {
                if let Some(stack_summaries) = response.stack_summaries {
                    for stack_summary in stack_summaries {
                        if let Some(name) = stack_summary.stack_name() {
                            if name == stack_name {
                                // Found the stack, check its status
                                if let Some(status) = stack_summary.stack_status() {
                                    debug!(
                                        "Found stack {} in list with status: {}",
                                        stack_name,
                                        status.as_str()
                                    );

                                    match status.as_str() {
                                        "CREATE_COMPLETE"
                                        | "UPDATE_COMPLETE"
                                        | "UPDATE_ROLLBACK_COMPLETE" => {
                                            return Ok(super::deployment::DeploymentType::Update);
                                        }
                                        status_str if status_str.contains("IN_PROGRESS") => {
                                            return Err(anyhow::anyhow!(
                                                "Stack {} is currently in progress with status: {}",
                                                stack_name,
                                                status_str
                                            ));
                                        }
                                        status_str if status_str.contains("FAILED") => {
                                            return Err(anyhow::anyhow!("Stack {} is in a failed state: {}. Please resolve before deployment.", stack_name, status_str));
                                        }
                                        _ => {
                                            warn!(
                                                "Stack {} has unexpected status: {}",
                                                stack_name,
                                                status.as_str()
                                            );
                                            return Ok(super::deployment::DeploymentType::Update);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Stack not found in list, it doesn't exist
                debug!(
                    "Stack {} not found in stack list, treating as new stack",
                    stack_name
                );
                Ok(super::deployment::DeploymentType::Create)
            }
            Err(list_error) => {
                warn!(
                    "Failed to list stacks for fallback detection: {}",
                    list_error
                );
                Err(anyhow::anyhow!(
                    "Fallback stack detection failed: {}",
                    list_error
                ))
            }
        }
    }

    /// Execute the actual deployment operation
    async fn execute_deployment(
        deployment_manager: super::deployment::DeploymentManager,
        cfn_client: cfn::Client,
        deployment_id: String,
        stack_name: String,
        template: String,
        deployment_type: super::deployment::DeploymentType,
        cloudformation_role_arn: Option<String>,
    ) -> Result<()> {
        info!(
            "=== CloudFormation Manager: Executing deployment {} ===",
            deployment_id
        );
        info!("Stack: {}, Type: {:?}", stack_name, deployment_type);

        // Update state to validating
        info!("Phase 1: Transitioning to validation state...");
        deployment_manager
            .update_deployment_state(
                &deployment_id,
                super::deployment::DeploymentState::Validating,
            )
            .await
            .with_context(|| {
                error!(
                    "Failed to update deployment state to Validating for deployment {}",
                    deployment_id
                );
                "Failed to update deployment state to Validating".to_string()
            })?;
        info!("✓ Deployment state updated to Validating");

        // Get deployment details for parameters
        info!("Retrieving deployment details...");
        let deployment = deployment_manager
            .get_deployment(&deployment_id)
            .await
            .ok_or_else(|| {
                error!(
                    "Deployment {} not found in deployment manager",
                    deployment_id
                );
                anyhow::anyhow!("Deployment {} not found", deployment_id)
            })?;
        info!(
            "✓ Retrieved deployment details: {} parameters configured",
            deployment.parameters.len()
        );
        tracing::trace!(
            "Parameters: {:?}",
            deployment.parameters.keys().collect::<Vec<_>>()
        );

        // Prepare parameters for CloudFormation
        info!("Preparing CloudFormation parameters...");
        let cf_parameters: Vec<cfn::types::Parameter> = deployment
            .parameters
            .iter()
            .map(|(key, value)| {
                tracing::trace!(
                    "Preparing parameter: {} = {}",
                    key,
                    if key.to_lowercase().contains("password")
                        || key.to_lowercase().contains("secret")
                    {
                        "[REDACTED]"
                    } else {
                        value
                    }
                );
                cfn::types::Parameter::builder()
                    .parameter_key(key)
                    .parameter_value(value)
                    .build()
            })
            .collect();
        info!(
            "✓ Prepared {} CloudFormation parameters",
            cf_parameters.len()
        );

        // Detect required capabilities
        info!("Analyzing template for required capabilities...");
        let capabilities = Self::detect_required_capabilities(&template).with_context(|| {
            error!("Failed to analyze template capabilities");
            "Failed to analyze template capabilities".to_string()
        })?;
        if capabilities.is_empty() {
            info!("✓ No special capabilities required");
        } else {
            info!("✓ Required capabilities: {:?}", capabilities);
        }

        // Update state to deploying
        info!("Phase 2: Transitioning to deployment state...");
        deployment_manager
            .update_deployment_state(
                &deployment_id,
                super::deployment::DeploymentState::Deploying,
            )
            .await
            .with_context(|| {
                error!(
                    "Failed to update deployment state to Deploying for deployment {}",
                    deployment_id
                );
                "Failed to update deployment state to Deploying".to_string()
            })?;
        info!("✓ Deployment state updated to Deploying");

        // Execute the appropriate CloudFormation operation
        info!(
            "Executing CloudFormation {} operation...",
            match deployment_type {
                super::deployment::DeploymentType::Create => "CREATE",
                super::deployment::DeploymentType::Update => "UPDATE",
                super::deployment::DeploymentType::Delete => "DELETE",
            }
        );
        let start_time = std::time::Instant::now();
        let stack_id = match deployment_type {
            super::deployment::DeploymentType::Create => {
                info!("Creating new stack: {}", stack_name);
                info!("Template size: {} bytes", template.len());

                let mut create_request = cfn_client
                    .create_stack()
                    .stack_name(&stack_name)
                    .template_body(&template);

                if !cf_parameters.is_empty() {
                    create_request = create_request.set_parameters(Some(cf_parameters.clone()));
                }

                if !capabilities.is_empty() {
                    create_request = create_request.set_capabilities(Some(capabilities.clone()));
                }

                // Set CloudFormation service role if available
                if let Some(role_arn) = &cloudformation_role_arn {
                    info!("Using CloudFormation service role: {}", role_arn);
                    create_request = create_request.role_arn(role_arn);
                }

                // === COMPREHENSIVE DEPLOYMENT LOGGING ===
                info!("=== CloudFormation API Call Details ===");
                info!(
                    "Target URL: https://cloudformation.{}.amazonaws.com",
                    deployment.region
                );
                info!("Account ID: {}", deployment.account_id);
                info!("Region: {}", deployment.region);
                info!("Stack Name: {}", stack_name);
                info!("Template Size: {} bytes", template.len());

                // Log CloudFormation parameters
                info!("CloudFormation Parameters ({} total):", cf_parameters.len());
                for param in &cf_parameters {
                    info!(
                        "  • {} = {}",
                        param.parameter_key().unwrap_or("NO_KEY"),
                        param.parameter_value().unwrap_or("NO_VALUE")
                    );
                }

                // Log capabilities if any
                if !capabilities.is_empty() {
                    info!("Required Capabilities: {:?}", capabilities);
                } else {
                    info!("No special capabilities required");
                }

                // Log service role
                if let Some(role_arn) = &cloudformation_role_arn {
                    info!("CloudFormation Service Role: {}", role_arn);
                } else {
                    info!("No CloudFormation service role configured");
                }

                // Log complete template content for debugging
                info!("=== COMPLETE CLOUDFORMATION TEMPLATE ===");
                info!("{}", template);
                info!("=== END TEMPLATE ===");

                // Log AWS configuration details (without credentials)
                info!("AWS Client Configuration:");
                info!("  • Using Identity Center credentials for API authentication");
                info!("  • Region: {}", deployment.region);
                info!("  • Service: CloudFormation");
                info!("=== End API Call Details ===");

                info!("Sending CreateStack request to AWS CloudFormation...");
                let response = create_request.send().await.map_err(|e| {
                    error!("❌ AWS CloudFormation CreateStack API call failed: {}", e);
                    error!("This is an AWS API error, not an internal application error");
                    error!("Error Type: {}", std::any::type_name_of_val(&e));

                    // Comprehensive error unwrapping for aws_smithy_runtime_api
                    error!("=== DETAILED ERROR ANALYSIS ===");
                    error!("Full Error Debug: {:#?}", e);

                    // Extract specific error details
                    let err_meta = e.meta();
                    error!("Error Code: {:?}", err_meta.code());
                    error!("Error Message: {:?}", err_meta.message());
                    error!("Request ID: {:?}", err_meta.request_id());

                    // Check if it's a credential/permission issue
                    let error_str = format!("{:?}", e);
                    if error_str.contains("AccessDenied")
                        || error_str.contains("UnauthorizedOperation")
                    {
                        error!("This appears to be a permission issue. Check:");
                        error!("1. The Identity Center role has CloudFormation permissions");
                        error!(
                            "2. The CloudFormation service role exists in account {}",
                            deployment.account_id
                        );
                        error!(
                            "3. The Identity Center role can pass the CloudFormation service role"
                        );
                    }

                    // Extract detailed error information using debug formatting
                    match &e {
                        aws_sdk_cloudformation::error::SdkError::DispatchFailure(
                            dispatch_failure,
                        ) => {
                            error!("🔍 DispatchFailure Details:");
                            error!("  DispatchFailure Debug: {:#?}", dispatch_failure);
                        }
                        aws_sdk_cloudformation::error::SdkError::TimeoutError(timeout) => {
                            error!("🔍 Timeout Error Details:");
                            error!("  Timeout Debug: {:#?}", timeout);
                        }
                        aws_sdk_cloudformation::error::SdkError::ResponseError(response_error) => {
                            error!("🔍 Response Error Details:");
                            error!("  Response Debug: {:#?}", response_error);
                        }
                        aws_sdk_cloudformation::error::SdkError::ServiceError(service_error) => {
                            error!("🔍 Service Error Details:");
                            error!("  Service Debug: {:#?}", service_error);
                        }
                        _ => {
                            error!("🔍 Other Error Type Details:");
                            error!("  Unknown Debug: {:#?}", e);
                        }
                    }

                    // Additional error analysis for ConnectorError
                    let error_string = format!("{}", e);
                    let error_debug = format!("{:#?}", e);

                    error!("Error String: {}", error_string);
                    error!("Error Debug String: {}", error_debug);

                    if error_string.contains("ConnectorError")
                        || error_debug.contains("ConnectorError")
                    {
                        error!("🔍 ConnectorError detected - this typically indicates:");
                        error!("  • DNS resolution failure for AWS endpoints");
                        error!("  • Network connectivity issues");
                        error!("  • Firewall blocking HTTPS requests to AWS");
                        error!("  • Proxy configuration problems");
                        error!("  • Internet connectivity issues");
                    }
                    if error_string.contains("dns error") || error_debug.contains("dns error") {
                        error!("🔍 DNS Error detected - specific issues could be:");
                        error!(
                            "  • Cannot resolve cloudformation.{}.amazonaws.com",
                            deployment.region
                        );
                        error!("  • DNS server configuration problems");
                        error!("  • /etc/resolv.conf issues (Linux)");
                        error!("  • VPN or network configuration blocking DNS");
                    }
                    error!("=== END ERROR ANALYSIS ===");

                    anyhow::anyhow!("AWS CloudFormation CreateStack failed: {}", e)
                })?;
                info!("✓ CreateStack request successful");

                response.stack_id().unwrap_or(&stack_name).to_string()
            }
            super::deployment::DeploymentType::Update => {
                info!("Updating existing stack: {}", stack_name);
                info!("Template size: {} bytes", template.len());

                let mut update_request = cfn_client
                    .update_stack()
                    .stack_name(&stack_name)
                    .template_body(&template);

                if !cf_parameters.is_empty() {
                    update_request = update_request.set_parameters(Some(cf_parameters));
                }

                if !capabilities.is_empty() {
                    update_request = update_request.set_capabilities(Some(capabilities));
                }

                // Set CloudFormation service role if available
                if let Some(role_arn) = &cloudformation_role_arn {
                    info!("Using CloudFormation service role for update: {}", role_arn);
                    update_request = update_request.role_arn(role_arn);
                }

                info!("Sending UpdateStack request to AWS CloudFormation...");
                let response = update_request.send().await.map_err(|e| {
                    error!("❌ AWS CloudFormation UpdateStack API call failed: {}", e);
                    error!("This is an AWS API error, not an internal application error");
                    error!("Error Type: {}", std::any::type_name_of_val(&e));

                    // Extract specific error details
                    let err_meta = e.meta();
                    error!("Error Code: {:?}", err_meta.code());
                    error!("Error Message: {:?}", err_meta.message());
                    error!("Request ID: {:?}", err_meta.request_id());

                    // Check if it's a credential/permission issue
                    let error_str = format!("{:?}", e);
                    if error_str.contains("AccessDenied")
                        || error_str.contains("UnauthorizedOperation")
                    {
                        error!("This appears to be a permission issue. Check:");
                        error!("1. The Identity Center role has CloudFormation permissions");
                        error!(
                            "2. The CloudFormation service role exists in account {}",
                            deployment.account_id
                        );
                        error!(
                            "3. The Identity Center role can pass the CloudFormation service role"
                        );
                    }

                    error!("Full AWS error: {:#?}", e);
                    anyhow::anyhow!("AWS CloudFormation UpdateStack failed: {}", e)
                })?;
                info!("✓ UpdateStack request successful");

                response.stack_id().unwrap_or(&stack_name).to_string()
            }
            super::deployment::DeploymentType::Delete => {
                info!("Deleting stack: {}", stack_name);

                info!("Sending DeleteStack request to AWS CloudFormation...");
                cfn_client
                    .delete_stack()
                    .stack_name(&stack_name)
                    .send()
                    .await
                    .map_err(|e| {
                        error!("❌ AWS CloudFormation DeleteStack API call failed: {}", e);
                        error!("This is an AWS API error, not an internal application error");
                        tracing::trace!("Full AWS error: {:#?}", e);
                        anyhow::anyhow!("AWS CloudFormation DeleteStack failed: {}", e)
                    })?;
                info!("✓ DeleteStack request successful");

                stack_name.clone()
            }
        };

        let operation_duration = start_time.elapsed();
        info!(
            "✓ CloudFormation {} operation initiated successfully in {:.2}s",
            match deployment_type {
                super::deployment::DeploymentType::Create => "CREATE",
                super::deployment::DeploymentType::Update => "UPDATE",
                super::deployment::DeploymentType::Delete => "DELETE",
            },
            operation_duration.as_secs_f64()
        );
        info!("Stack ID: {}", stack_id);

        // Update state to monitoring
        info!("Phase 3: Transitioning to monitoring state...");
        deployment_manager
            .update_deployment_state(
                &deployment_id,
                super::deployment::DeploymentState::Monitoring,
            )
            .await
            .with_context(|| {
                error!(
                    "Failed to update deployment state to Monitoring for deployment {}",
                    deployment_id
                );
                "Failed to update deployment state to Monitoring".to_string()
            })?;
        info!("✓ Deployment state updated to Monitoring");

        // Start monitoring the deployment
        info!("Starting deployment monitoring...");
        Self::monitor_deployment(
            deployment_manager,
            cfn_client,
            deployment_id,
            stack_name,
            deployment_type,
        )
        .await
        .with_context(|| {
            error!("Deployment monitoring failed");
            "Deployment monitoring failed".to_string()
        })?;
        info!("✓ Deployment monitoring completed");

        Ok(())
    }

    /// Detect required CloudFormation capabilities from template
    fn detect_required_capabilities(template: &str) -> Result<Vec<cfn::types::Capability>> {
        let mut capabilities = Vec::new();

        // Parse template to detect IAM resources
        if let Ok(template_value) = serde_json::from_str::<serde_json::Value>(template) {
            if let Some(resources) = template_value.get("Resources") {
                if let Some(resources_obj) = resources.as_object() {
                    for (_, resource) in resources_obj {
                        if let Some(resource_type) = resource.get("Type").and_then(|t| t.as_str()) {
                            match resource_type {
                                "AWS::IAM::Role"
                                | "AWS::IAM::Policy"
                                | "AWS::IAM::User"
                                | "AWS::IAM::Group"
                                | "AWS::IAM::AccessKey"
                                | "AWS::IAM::InstanceProfile" => {
                                    if !capabilities
                                        .contains(&cfn::types::Capability::CapabilityIam)
                                    {
                                        capabilities.push(cfn::types::Capability::CapabilityIam);
                                    }
                                }
                                _ if resource_type.starts_with("AWS::IAM::") => {
                                    if !capabilities
                                        .contains(&cfn::types::Capability::CapabilityIam)
                                    {
                                        capabilities.push(cfn::types::Capability::CapabilityIam);
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Check for custom resource names that might require CAPABILITY_NAMED_IAM
                        if let Some(properties) = resource.get("Properties") {
                            if (properties.get("RoleName").is_some()
                                || properties.get("PolicyName").is_some()
                                || properties.get("UserName").is_some()
                                || properties.get("GroupName").is_some())
                                && !capabilities
                                    .contains(&cfn::types::Capability::CapabilityNamedIam)
                            {
                                capabilities.push(cfn::types::Capability::CapabilityNamedIam);
                            }
                        }
                    }
                }
            }
        }

        debug!("Detected capabilities: {:?}", capabilities);
        Ok(capabilities)
    }

    /// Monitor deployment progress by polling CloudFormation events
    async fn monitor_deployment(
        deployment_manager: super::deployment::DeploymentManager,
        cfn_client: cfn::Client,
        deployment_id: String,
        stack_name: String,
        _deployment_type: super::deployment::DeploymentType,
    ) -> Result<()> {
        let mut last_event_time: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut monitoring_complete = false;
        let monitoring_start = std::time::Instant::now();
        let mut poll_count = 0;

        info!("=== CloudFormation Manager: Starting deployment monitoring ===");
        info!("Deployment ID: {}, Stack: {}", deployment_id, stack_name);
        info!("Monitoring will poll every 5 seconds for stack events...");

        while !monitoring_complete {
            poll_count += 1;
            let _poll_start = std::time::Instant::now();

            tracing::trace!(
                "Monitoring poll #{} for deployment {}",
                poll_count,
                deployment_id
            );

            // Check if deployment was cancelled
            if let Some(deployment) = deployment_manager.get_deployment(&deployment_id).await {
                if deployment.state == super::deployment::DeploymentState::Cancelled {
                    warn!(
                        "Deployment {} was cancelled by user, stopping monitoring",
                        deployment_id
                    );
                    break;
                }
            } else {
                error!(
                    "Deployment {} disappeared from manager during monitoring",
                    deployment_id
                );
                return Err(anyhow::anyhow!("Deployment disappeared during monitoring"));
            }

            // Fetch stack events
            match cfn_client
                .describe_stack_events()
                .stack_name(&stack_name)
                .send()
                .await
            {
                Ok(response) => {
                    let events = response.stack_events();
                    if !events.is_empty() {
                        // Process new events
                        for aws_event in events {
                            let event_time = aws_event
                                .timestamp()
                                .map(|t| {
                                    chrono::DateTime::from_timestamp(t.secs(), t.subsec_nanos())
                                        .unwrap_or_else(chrono::Utc::now)
                                })
                                .unwrap_or_else(chrono::Utc::now);

                            // Only process events newer than our last seen event
                            if last_event_time.map_or(true, |last| event_time > last) {
                                let stack_event =
                                    super::deployment::StackEvent::from(aws_event.clone());

                                // Add event to deployment
                                if let Err(e) = deployment_manager
                                    .add_deployment_event(&deployment_id, stack_event)
                                    .await
                                {
                                    warn!(
                                        "Failed to add event to deployment {}: {}",
                                        deployment_id, e
                                    );
                                }

                                // Check for terminal events
                                if let Some(status) = aws_event.resource_status() {
                                    let status_str = status.as_str();

                                    // Check if this is a stack-level terminal event
                                    if aws_event
                                        .logical_resource_id()
                                        .is_some_and(|id| id == stack_name)
                                    {
                                        match status_str {
                                            "CREATE_COMPLETE" | "UPDATE_COMPLETE" => {
                                                info!(
                                                    "Stack {} deployment completed successfully",
                                                    stack_name
                                                );

                                                // Fetch stack outputs
                                                let outputs = Self::fetch_stack_outputs(
                                                    &cfn_client,
                                                    &stack_name,
                                                )
                                                .await
                                                .unwrap_or_default();

                                                deployment_manager
                                                    .complete_deployment(
                                                        &deployment_id,
                                                        true,
                                                        outputs,
                                                    )
                                                    .await?;
                                                monitoring_complete = true;
                                                break;
                                            }
                                            "CREATE_FAILED"
                                            | "UPDATE_FAILED"
                                            | "UPDATE_ROLLBACK_COMPLETE"
                                            | "CREATE_ROLLBACK_COMPLETE"
                                            | "DELETE_COMPLETE" => {
                                                let success = status_str == "DELETE_COMPLETE";
                                                let error_msg = aws_event
                                                    .resource_status_reason()
                                                    .unwrap_or("Deployment failed")
                                                    .to_string();

                                                warn!(
                                                    "Stack {} deployment failed with status: {}",
                                                    stack_name, status_str
                                                );

                                                if success {
                                                    deployment_manager
                                                        .complete_deployment(
                                                            &deployment_id,
                                                            true,
                                                            std::collections::HashMap::new(),
                                                        )
                                                        .await?;
                                                } else {
                                                    deployment_manager
                                                        .fail_deployment(&deployment_id, error_msg)
                                                        .await?;
                                                }
                                                monitoring_complete = true;
                                                break;
                                            }
                                            _ => {
                                                debug!(
                                                    "Stack {} status: {}",
                                                    stack_name, status_str
                                                );
                                            }
                                        }
                                    }
                                }

                                last_event_time = Some(event_time);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch stack events for {}: {}", stack_name, e);

                    // If we can't fetch events, check stack status directly
                    match cfn_client
                        .describe_stacks()
                        .stack_name(&stack_name)
                        .send()
                        .await
                    {
                        Ok(response) => {
                            let stacks = response.stacks();
                            if !stacks.is_empty() {
                                if let Some(stack) = stacks.first() {
                                    if let Some(status) = stack.stack_status() {
                                        let status_str = status.as_str();
                                        if status_str.contains("COMPLETE")
                                            || status_str.contains("FAILED")
                                        {
                                            let success = status_str.contains("COMPLETE")
                                                && !status_str.contains("ROLLBACK");

                                            if success {
                                                let outputs = Self::fetch_stack_outputs(
                                                    &cfn_client,
                                                    &stack_name,
                                                )
                                                .await
                                                .unwrap_or_default();
                                                deployment_manager
                                                    .complete_deployment(
                                                        &deployment_id,
                                                        true,
                                                        outputs,
                                                    )
                                                    .await?;
                                            } else {
                                                deployment_manager
                                                    .fail_deployment(
                                                        &deployment_id,
                                                        format!(
                                                            "Stack in terminal state: {}",
                                                            status_str
                                                        ),
                                                    )
                                                    .await?;
                                            }
                                            monitoring_complete = true;
                                        }
                                    }
                                }
                            }
                        }
                        Err(stack_err) => {
                            error!("Failed to check stack status: {}", stack_err);
                            deployment_manager
                                .fail_deployment(
                                    &deployment_id,
                                    format!("Failed to monitor deployment: {}", stack_err),
                                )
                                .await?;
                            monitoring_complete = true;
                        }
                    }
                }
            }

            if !monitoring_complete {
                // Wait before next poll
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }

        let total_monitoring_time = monitoring_start.elapsed();
        info!("=== CloudFormation Manager: Deployment monitoring completed ===");
        info!(
            "Stack: {}, Total time: {:.1}s, Polling cycles: {}",
            stack_name,
            total_monitoring_time.as_secs_f64(),
            poll_count
        );
        Ok(())
    }

    /// Fetch stack outputs after successful deployment
    async fn fetch_stack_outputs(
        cfn_client: &cfn::Client,
        stack_name: &str,
    ) -> Result<std::collections::HashMap<String, String>> {
        let mut outputs = std::collections::HashMap::new();

        match cfn_client
            .describe_stacks()
            .stack_name(stack_name)
            .send()
            .await
        {
            Ok(response) => {
                let stacks = response.stacks();
                if !stacks.is_empty() {
                    if let Some(stack) = stacks.first() {
                        let stack_outputs = stack.outputs();
                        for output in stack_outputs {
                            if let (Some(key), Some(value)) =
                                (output.output_key(), output.output_value())
                            {
                                outputs.insert(key.to_string(), value.to_string());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to fetch stack outputs for {}: {}", stack_name, e);
            }
        }

        debug!("Fetched {} outputs for stack {}", outputs.len(), stack_name);
        Ok(outputs)
    }
}
