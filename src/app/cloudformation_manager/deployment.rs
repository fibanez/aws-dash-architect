use crate::app::projects::Project;
use crate::app::resource_explorer::credentials::CredentialCoordinator;
use anyhow::Result;
use aws_sdk_cloudformation as cfn;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Deployment states representing the lifecycle of a CloudFormation deployment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeploymentState {
    /// Gathering parameters and validating input
    Collecting,
    /// Pre-deployment validation (template validation, parameter checks)
    Validating,
    /// CloudFormation CreateStack/UpdateStack in progress
    Deploying,
    /// Watching stack events and monitoring progress
    Monitoring,
    /// Deployment completed - bool indicates success (true) or failure (false)
    Complete(bool),
    /// User cancelled the deployment
    Cancelled,
    /// Deployment failed with error
    Failed(String),
}

impl DeploymentState {
    /// Check if the deployment is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DeploymentState::Complete(_) | DeploymentState::Cancelled | DeploymentState::Failed(_)
        )
    }

    /// Check if the deployment is active (in progress)
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            DeploymentState::Collecting
                | DeploymentState::Validating
                | DeploymentState::Deploying
                | DeploymentState::Monitoring
        )
    }

    /// Get a human-readable description of the state
    pub fn description(&self) -> &'static str {
        match self {
            DeploymentState::Collecting => "Collecting parameters",
            DeploymentState::Validating => "Validating template and parameters",
            DeploymentState::Deploying => "Deploying stack",
            DeploymentState::Monitoring => "Monitoring deployment progress",
            DeploymentState::Complete(true) => "Deployment successful",
            DeploymentState::Complete(false) => "Deployment failed",
            DeploymentState::Cancelled => "Deployment cancelled",
            DeploymentState::Failed(_) => "Deployment failed",
        }
    }
}

/// CloudFormation stack event representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackEvent {
    pub event_id: String,
    pub stack_id: Option<String>,
    pub stack_name: String,
    pub logical_resource_id: Option<String>,
    pub physical_resource_id: Option<String>,
    pub resource_type: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub resource_status: String,
    pub resource_status_reason: Option<String>,
    pub resource_properties: Option<String>,
}

impl From<cfn::types::StackEvent> for StackEvent {
    fn from(aws_event: cfn::types::StackEvent) -> Self {
        Self {
            event_id: aws_event.event_id().unwrap_or_default().to_string(),
            stack_id: aws_event.stack_id().map(|s| s.to_string()),
            stack_name: aws_event.stack_name().unwrap_or_default().to_string(),
            logical_resource_id: aws_event.logical_resource_id().map(|s| s.to_string()),
            physical_resource_id: aws_event.physical_resource_id().map(|s| s.to_string()),
            resource_type: aws_event.resource_type().map(|s| s.to_string()),
            timestamp: aws_event
                .timestamp()
                .map(|t| {
                    DateTime::from_timestamp(t.secs(), t.subsec_nanos()).unwrap_or_else(Utc::now)
                })
                .unwrap_or_else(Utc::now),
            resource_status: aws_event
                .resource_status()
                .map(|s| s.as_str().to_string())
                .unwrap_or_default(),
            resource_status_reason: aws_event.resource_status_reason().map(|s| s.to_string()),
            resource_properties: aws_event.resource_properties().map(|s| s.to_string()),
        }
    }
}

/// Type of deployment operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeploymentType {
    /// Creating a new stack
    Create,
    /// Updating an existing stack
    Update,
    /// Deleting a stack
    Delete,
}

/// Deployment operation tracking complete lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentOperation {
    /// Unique identifier for this deployment
    pub id: String,
    /// CloudFormation stack name
    pub stack_name: String,
    /// AWS account ID where deployment is happening
    pub account_id: String,
    /// AWS region for deployment
    pub region: String,
    /// Current deployment state
    pub state: DeploymentState,
    /// Type of deployment (Create/Update/Delete)
    pub deployment_type: DeploymentType,
    /// Stack events collected during deployment
    pub events: Vec<StackEvent>,
    /// When the deployment started
    pub start_time: DateTime<Utc>,
    /// When the deployment finished (if terminal)
    pub end_time: Option<DateTime<Utc>>,
    /// Template being deployed
    pub template: String,
    /// Parameters for the deployment
    pub parameters: HashMap<String, String>,
    /// Stack capabilities required
    pub capabilities: Vec<String>,
    /// Stack policy (if any)
    pub stack_policy: Option<String>,
    /// Tags to apply to the stack
    pub tags: HashMap<String, String>,
    /// Project context
    pub project_name: String,
    /// Environment name
    pub environment: String,
    /// Current progress percentage (0-100)
    pub progress_percent: u8,
    /// Error message if deployment failed
    pub error_message: Option<String>,
    /// Stack outputs after successful deployment
    pub stack_outputs: HashMap<String, String>,
}

impl DeploymentOperation {
    /// Create a new deployment operation
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stack_name: String,
        account_id: String,
        region: String,
        deployment_type: DeploymentType,
        template: String,
        parameters: HashMap<String, String>,
        project_name: String,
        environment: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            stack_name,
            account_id,
            region,
            state: DeploymentState::Collecting,
            deployment_type,
            events: Vec::new(),
            start_time: Utc::now(),
            end_time: None,
            template,
            parameters,
            capabilities: Vec::new(),
            stack_policy: None,
            tags: HashMap::new(),
            project_name,
            environment,
            progress_percent: 0,
            error_message: None,
            stack_outputs: HashMap::new(),
        }
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: DeploymentState) {
        debug!(
            "Deployment {} transitioning from {:?} to {:?}",
            self.id, self.state, new_state
        );
        self.state = new_state;

        // Update progress based on new state
        self.update_progress();

        if self.state.is_terminal() && self.end_time.is_none() {
            self.end_time = Some(Utc::now());
        }
    }

    /// Add a stack event
    pub fn add_event(&mut self, event: StackEvent) {
        self.events.push(event);
        // Update progress based on events
        self.update_progress();
    }

    /// Update progress percentage based on events
    fn update_progress(&mut self) {
        match self.state {
            DeploymentState::Collecting => self.progress_percent = 10,
            DeploymentState::Validating => self.progress_percent = 20,
            DeploymentState::Deploying => {
                // Calculate progress based on resource creation/update events
                let total_resources = self.estimate_total_resources();
                let completed_resources = self.count_completed_resources();
                if total_resources > 0 {
                    self.progress_percent =
                        20 + ((completed_resources * 60) / total_resources) as u8;
                } else {
                    self.progress_percent = 30;
                }
            }
            DeploymentState::Monitoring => self.progress_percent = 90,
            DeploymentState::Complete(true) => self.progress_percent = 100,
            DeploymentState::Complete(false) | DeploymentState::Failed(_) => {
                // Keep current progress on failure
            }
            DeploymentState::Cancelled => {
                // Keep current progress on cancellation
            }
        }
    }

    /// Estimate total resources to be created/updated
    fn estimate_total_resources(&self) -> usize {
        // Parse template to count resources
        if let Ok(template_value) = serde_json::from_str::<serde_json::Value>(&self.template) {
            if let Some(resources) = template_value.get("Resources") {
                if let Some(resources_obj) = resources.as_object() {
                    return resources_obj.len();
                }
            }
        }

        // Fallback: estimate based on events seen so far
        self.events
            .iter()
            .filter_map(|e| e.logical_resource_id.as_ref())
            .collect::<std::collections::HashSet<_>>()
            .len()
            .max(1) // At least 1 to avoid division by zero
    }

    /// Count completed resources based on events
    fn count_completed_resources(&self) -> usize {
        self.events
            .iter()
            .filter(|event| {
                event.resource_status.ends_with("_COMPLETE")
                    || event.resource_status.ends_with("_FAILED")
            })
            .filter_map(|e| e.logical_resource_id.as_ref())
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    /// Get the duration of the deployment so far
    pub fn duration(&self) -> Duration {
        let end = self.end_time.unwrap_or_else(Utc::now);
        (end - self.start_time).to_std().unwrap_or(Duration::ZERO)
    }

    /// Check if deployment can be cancelled
    pub fn can_cancel(&self) -> bool {
        matches!(
            self.state,
            DeploymentState::Collecting
                | DeploymentState::Validating
                | DeploymentState::Deploying
                | DeploymentState::Monitoring
        )
    }

    /// Mark deployment as cancelled
    pub fn cancel(&mut self) {
        if self.can_cancel() {
            self.transition_to(DeploymentState::Cancelled);
            info!("Deployment {} cancelled by user", self.id);
        }
    }

    /// Mark deployment as failed
    pub fn fail(&mut self, error_message: String) {
        self.error_message = Some(error_message.clone());
        self.transition_to(DeploymentState::Failed(error_message));
    }

    /// Get the latest event for a specific resource
    pub fn get_latest_resource_event(&self, logical_resource_id: &str) -> Option<&StackEvent> {
        self.events
            .iter()
            .rev() // Most recent first
            .find(|event| {
                event
                    .logical_resource_id
                    .as_ref()
                    .is_some_and(|id| id == logical_resource_id)
            })
    }

    /// Get all unique resources mentioned in events
    pub fn get_resource_ids(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|event| event.logical_resource_id.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Get detailed diagnostics for failed resources
    pub fn get_failed_resource_diagnostics(&self) -> Vec<ResourceDiagnostic> {
        let mut diagnostics = Vec::new();
        let resource_ids = self.get_resource_ids();

        for resource_id in resource_ids {
            if let Some(latest_event) = self.get_latest_resource_event(&resource_id) {
                if latest_event.resource_status.contains("FAILED") {
                    let diagnostic = ResourceDiagnostic {
                        logical_resource_id: resource_id.clone(),
                        resource_type: latest_event.resource_type.clone().unwrap_or_default(),
                        status: latest_event.resource_status.clone(),
                        failure_reason: latest_event.resource_status_reason.clone(),
                        physical_resource_id: latest_event.physical_resource_id.clone(),
                        timestamp: latest_event.timestamp,
                        related_events: self.get_resource_event_history(&resource_id),
                        suggested_actions: self.generate_suggested_actions(latest_event),
                    };
                    diagnostics.push(diagnostic);
                }
            }
        }

        diagnostics
    }

    /// Get all events for a specific resource
    pub fn get_resource_event_history(&self, logical_resource_id: &str) -> Vec<StackEvent> {
        self.events
            .iter()
            .filter(|event| {
                event
                    .logical_resource_id
                    .as_ref()
                    .is_some_and(|id| id == logical_resource_id)
            })
            .cloned()
            .collect()
    }

    /// Generate suggested actions based on failure reason
    fn generate_suggested_actions(&self, event: &StackEvent) -> Vec<String> {
        let mut suggestions = Vec::new();

        if let Some(reason) = &event.resource_status_reason {
            let reason_lower = reason.to_lowercase();

            // Common failure patterns and suggestions
            if reason_lower.contains("already exists") {
                suggestions.push("Resource name conflict: Consider using a unique name or deleting the existing resource".to_string());
            }

            if reason_lower.contains("insufficient") && reason_lower.contains("permission") {
                suggestions.push(
                    "Insufficient permissions: Check IAM policies and resource access permissions"
                        .to_string(),
                );
                suggestions.push(
                    "Ensure the deployment role has necessary permissions for this resource type"
                        .to_string(),
                );
            }

            if reason_lower.contains("limit") && reason_lower.contains("exceed") {
                suggestions.push(
                    "Service limit exceeded: Request a limit increase or clean up unused resources"
                        .to_string(),
                );
            }

            if reason_lower.contains("invalid") && reason_lower.contains("parameter") {
                suggestions.push(
                    "Invalid parameter value: Check parameter constraints and allowed values"
                        .to_string(),
                );
            }

            if reason_lower.contains("dependency") || reason_lower.contains("depends on") {
                suggestions.push(
                    "Dependency issue: Verify dependent resources exist and are in correct state"
                        .to_string(),
                );
            }

            if reason_lower.contains("timeout") {
                suggestions.push(
                    "Operation timeout: Resource creation took too long, check resource health"
                        .to_string(),
                );
                suggestions.push("Consider simplifying the resource configuration or checking external dependencies".to_string());
            }

            if reason_lower.contains("rollback") {
                suggestions.push(
                    "Rollback occurred: Check events of dependent resources for root cause"
                        .to_string(),
                );
            }

            // Resource-specific suggestions
            if let Some(resource_type) = &event.resource_type {
                match resource_type.as_str() {
                    "AWS::EC2::Instance" => {
                        suggestions.push("EC2 Instance: Check AMI availability, instance type limits, and VPC configuration".to_string());
                    }
                    "AWS::RDS::DBInstance" => {
                        suggestions.push("RDS Instance: Verify DB engine version, instance class availability, and subnet group".to_string());
                    }
                    "AWS::S3::Bucket" => {
                        suggestions.push("S3 Bucket: Ensure bucket name is globally unique and follows naming conventions".to_string());
                    }
                    "AWS::IAM::Role" => {
                        suggestions.push(
                            "IAM Role: Check trust policy syntax and ensure proper permissions"
                                .to_string(),
                        );
                    }
                    "AWS::Lambda::Function" => {
                        suggestions.push("Lambda Function: Verify code package, runtime version, and execution role".to_string());
                    }
                    _ => {}
                }
            }
        }

        if suggestions.is_empty() {
            suggestions
                .push("Check CloudFormation documentation for this resource type".to_string());
            suggestions.push("Review AWS service quotas and regional availability".to_string());
        }

        suggestions
    }

    /// Check if deployment has any failed resources
    pub fn has_failed_resources(&self) -> bool {
        self.events
            .iter()
            .any(|event| event.resource_status.contains("FAILED"))
    }

    /// Get rollback events
    pub fn get_rollback_events(&self) -> Vec<&StackEvent> {
        self.events
            .iter()
            .filter(|event| event.resource_status.contains("ROLLBACK"))
            .collect()
    }

    /// Check if deployment is in rollback state
    pub fn is_rolling_back(&self) -> bool {
        !self.get_rollback_events().is_empty()
    }

    /// Get deployment health summary
    pub fn get_health_summary(&self) -> DeploymentHealthSummary {
        let resource_ids = self.get_resource_ids();
        let mut healthy = 0;
        let mut failed = 0;
        let mut in_progress = 0;
        let mut unknown = 0;

        for resource_id in &resource_ids {
            if let Some(event) = self.get_latest_resource_event(resource_id) {
                if event.resource_status.contains("COMPLETE")
                    && !event.resource_status.contains("ROLLBACK")
                {
                    healthy += 1;
                } else if event.resource_status.contains("FAILED") {
                    failed += 1;
                } else if event.resource_status.contains("IN_PROGRESS") {
                    in_progress += 1;
                } else {
                    unknown += 1;
                }
            } else {
                unknown += 1;
            }
        }

        DeploymentHealthSummary {
            total_resources: resource_ids.len(),
            healthy_resources: healthy,
            failed_resources: failed,
            in_progress_resources: in_progress,
            unknown_resources: unknown,
            has_rollback: self.is_rolling_back(),
            failed_diagnostics: self.get_failed_resource_diagnostics(),
        }
    }
}

/// Deployment manager for handling CloudFormation deployments
#[derive(Clone)]
pub struct DeploymentManager {
    #[allow(dead_code)]
    credential_coordinator: Arc<CredentialCoordinator>,
    active_deployments: Arc<RwLock<HashMap<String, DeploymentOperation>>>,
    event_sender: Option<mpsc::UnboundedSender<DeploymentEvent>>,
}

/// Events emitted during deployment lifecycle
#[derive(Debug, Clone)]
pub enum DeploymentEvent {
    /// Deployment state changed
    StateChanged {
        deployment_id: String,
        old_state: DeploymentState,
        new_state: DeploymentState,
    },
    /// New stack event received
    StackEvent {
        deployment_id: String,
        event: StackEvent,
    },
    /// Progress updated
    ProgressUpdated {
        deployment_id: String,
        progress_percent: u8,
    },
    /// Deployment completed
    Completed {
        deployment_id: String,
        success: bool,
        outputs: HashMap<String, String>,
    },
    /// Deployment failed
    Failed {
        deployment_id: String,
        error_message: String,
    },
}

impl DeploymentManager {
    /// Create a new deployment manager
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        Self {
            credential_coordinator,
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
            event_sender: None,
        }
    }

    /// Set event sender for deployment notifications
    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<DeploymentEvent>) {
        self.event_sender = Some(sender);
    }

    /// Create a new deployment operation
    #[allow(clippy::too_many_arguments)]
    pub async fn create_deployment(
        &self,
        stack_name: String,
        account_id: String,
        region: String,
        deployment_type: DeploymentType,
        template: String,
        parameters: HashMap<String, String>,
        project: &Project,
        environment: String,
    ) -> Result<String> {
        let deployment = DeploymentOperation::new(
            stack_name,
            account_id.clone(),
            region.clone(),
            deployment_type,
            template,
            parameters,
            project.name.clone(),
            environment,
        );

        let deployment_id = deployment.id.clone();
        let stack_name = deployment.stack_name.clone();

        {
            let mut deployments = self.active_deployments.write().await;
            deployments.insert(deployment_id.clone(), deployment);
        }

        info!(
            "Created deployment {} for stack {} in {}/{}",
            deployment_id, stack_name, account_id, region
        );

        Ok(deployment_id)
    }

    /// Get a deployment by ID
    pub async fn get_deployment(&self, deployment_id: &str) -> Option<DeploymentOperation> {
        let deployments = self.active_deployments.read().await;
        deployments.get(deployment_id).cloned()
    }

    /// Get all active deployments
    pub async fn get_active_deployments(&self) -> Vec<DeploymentOperation> {
        let deployments = self.active_deployments.read().await;
        deployments
            .values()
            .filter(|d| d.state.is_active())
            .cloned()
            .collect()
    }

    /// Get all deployments (active and terminal)
    pub async fn get_all_deployments(&self) -> Vec<DeploymentOperation> {
        let deployments = self.active_deployments.read().await;
        deployments.values().cloned().collect()
    }

    /// Update deployment state
    pub async fn update_deployment_state(
        &self,
        deployment_id: &str,
        new_state: DeploymentState,
    ) -> Result<()> {
        let mut deployments = self.active_deployments.write().await;

        if let Some(deployment) = deployments.get_mut(deployment_id) {
            let old_state = deployment.state.clone();
            deployment.transition_to(new_state.clone());

            // Send event notification
            if let Some(sender) = &self.event_sender {
                let _ = sender.send(DeploymentEvent::StateChanged {
                    deployment_id: deployment_id.to_string(),
                    old_state,
                    new_state,
                });
            }

            debug!("Updated deployment {} state", deployment_id);
        } else {
            warn!(
                "Attempted to update non-existent deployment {}",
                deployment_id
            );
        }

        Ok(())
    }

    /// Add event to deployment
    pub async fn add_deployment_event(&self, deployment_id: &str, event: StackEvent) -> Result<()> {
        let mut deployments = self.active_deployments.write().await;

        if let Some(deployment) = deployments.get_mut(deployment_id) {
            deployment.add_event(event.clone());

            // Send event notification
            if let Some(sender) = &self.event_sender {
                let _ = sender.send(DeploymentEvent::StackEvent {
                    deployment_id: deployment_id.to_string(),
                    event,
                });

                let _ = sender.send(DeploymentEvent::ProgressUpdated {
                    deployment_id: deployment_id.to_string(),
                    progress_percent: deployment.progress_percent,
                });
            }

            debug!("Added event to deployment {}", deployment_id);
        }

        Ok(())
    }

    /// Cancel a deployment
    pub async fn cancel_deployment(&self, deployment_id: &str) -> Result<()> {
        let mut deployments = self.active_deployments.write().await;

        if let Some(deployment) = deployments.get_mut(deployment_id) {
            if deployment.can_cancel() {
                deployment.cancel();

                // Send notification
                if let Some(sender) = &self.event_sender {
                    let _ = sender.send(DeploymentEvent::StateChanged {
                        deployment_id: deployment_id.to_string(),
                        old_state: deployment.state.clone(),
                        new_state: DeploymentState::Cancelled,
                    });
                }

                info!("Cancelled deployment {}", deployment_id);
            } else {
                warn!(
                    "Cannot cancel deployment {} in state {:?}",
                    deployment_id, deployment.state
                );
            }
        }

        Ok(())
    }

    /// Mark deployment as completed
    pub async fn complete_deployment(
        &self,
        deployment_id: &str,
        success: bool,
        outputs: HashMap<String, String>,
    ) -> Result<()> {
        let mut deployments = self.active_deployments.write().await;

        if let Some(deployment) = deployments.get_mut(deployment_id) {
            deployment.stack_outputs = outputs.clone();
            deployment.transition_to(DeploymentState::Complete(success));

            // Send notification
            if let Some(sender) = &self.event_sender {
                let _ = sender.send(DeploymentEvent::Completed {
                    deployment_id: deployment_id.to_string(),
                    success,
                    outputs,
                });
            }

            info!(
                "Completed deployment {} with success={}",
                deployment_id, success
            );
        }

        Ok(())
    }

    /// Mark deployment as failed
    pub async fn fail_deployment(&self, deployment_id: &str, error_message: String) -> Result<()> {
        let mut deployments = self.active_deployments.write().await;

        if let Some(deployment) = deployments.get_mut(deployment_id) {
            deployment.fail(error_message.clone());

            // Send notification
            if let Some(sender) = &self.event_sender {
                let _ = sender.send(DeploymentEvent::Failed {
                    deployment_id: deployment_id.to_string(),
                    error_message,
                });
            }

            error!("Failed deployment {}", deployment_id);
        }

        Ok(())
    }

    /// Clean up old terminal deployments
    pub async fn cleanup_deployments(&self, max_age_hours: u64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours as i64);
        let mut deployments = self.active_deployments.write().await;

        let initial_count = deployments.len();
        deployments.retain(|_id, deployment| {
            if deployment.state.is_terminal() {
                if let Some(end_time) = deployment.end_time {
                    end_time > cutoff
                } else {
                    deployment.start_time > cutoff
                }
            } else {
                true // Keep active deployments
            }
        });

        let removed_count = initial_count - deployments.len();
        if removed_count > 0 {
            debug!("Cleaned up {} old deployments", removed_count);
        }
    }

    /// Get deployment statistics
    pub async fn get_deployment_stats(&self) -> DeploymentStats {
        let deployments = self.active_deployments.read().await;
        let total = deployments.len();
        let active = deployments.values().filter(|d| d.state.is_active()).count();
        let successful = deployments
            .values()
            .filter(|d| matches!(d.state, DeploymentState::Complete(true)))
            .count();
        let failed = deployments
            .values()
            .filter(|d| {
                matches!(
                    d.state,
                    DeploymentState::Complete(false) | DeploymentState::Failed(_)
                )
            })
            .count();
        let cancelled = deployments
            .values()
            .filter(|d| matches!(d.state, DeploymentState::Cancelled))
            .count();

        DeploymentStats {
            total,
            active,
            successful,
            failed,
            cancelled,
        }
    }

    /// Retry a failed deployment with the same parameters
    pub async fn retry_deployment(&self, deployment_id: &str) -> Result<String> {
        let deployments = self.active_deployments.read().await;
        let deployment = deployments
            .get(deployment_id)
            .ok_or_else(|| anyhow::anyhow!("Deployment {} not found", deployment_id))?;

        // Only allow retry for failed or cancelled deployments
        if !matches!(
            deployment.state,
            DeploymentState::Failed(_)
                | DeploymentState::Cancelled
                | DeploymentState::Complete(false)
        ) {
            return Err(anyhow::anyhow!(
                "Cannot retry deployment in state: {:?}",
                deployment.state
            ));
        }

        // Create a new deployment with the same parameters
        let new_deployment = DeploymentOperation::new(
            deployment.stack_name.clone(),
            deployment.account_id.clone(),
            deployment.region.clone(),
            deployment.deployment_type.clone(),
            deployment.template.clone(),
            deployment.parameters.clone(),
            deployment.project_name.clone(),
            deployment.environment.clone(),
        );

        let new_deployment_id = new_deployment.id.clone();

        // Store the new deployment
        drop(deployments); // Release read lock
        {
            let mut deployments = self.active_deployments.write().await;
            deployments.insert(new_deployment_id.clone(), new_deployment);
        }

        info!(
            "Created retry deployment {} for original deployment {}",
            new_deployment_id, deployment_id
        );
        Ok(new_deployment_id)
    }

    /// Get health summary for a deployment
    pub async fn get_deployment_health(
        &self,
        deployment_id: &str,
    ) -> Option<DeploymentHealthSummary> {
        let deployments = self.active_deployments.read().await;
        deployments
            .get(deployment_id)
            .map(|d| d.get_health_summary())
    }

    /// Get failed resource diagnostics for a deployment
    pub async fn get_deployment_diagnostics(&self, deployment_id: &str) -> Vec<ResourceDiagnostic> {
        let deployments = self.active_deployments.read().await;
        deployments
            .get(deployment_id)
            .map(|d| d.get_failed_resource_diagnostics())
            .unwrap_or_default()
    }
}

/// Deployment statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStats {
    pub total: usize,
    pub active: usize,
    pub successful: usize,
    pub failed: usize,
    pub cancelled: usize,
}

/// Diagnostic information for failed resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDiagnostic {
    pub logical_resource_id: String,
    pub resource_type: String,
    pub status: String,
    pub failure_reason: Option<String>,
    pub physical_resource_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub related_events: Vec<StackEvent>,
    pub suggested_actions: Vec<String>,
}

/// Health summary for a deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentHealthSummary {
    pub total_resources: usize,
    pub healthy_resources: usize,
    pub failed_resources: usize,
    pub in_progress_resources: usize,
    pub unknown_resources: usize,
    pub has_rollback: bool,
    pub failed_diagnostics: Vec<ResourceDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_state_transitions() {
        let mut deployment = DeploymentOperation::new(
            "test-stack".to_string(),
            "123456789012".to_string(),
            "us-east-1".to_string(),
            DeploymentType::Create,
            "{}".to_string(),
            HashMap::new(),
            "Test Project".to_string(),
            "dev".to_string(),
        );

        assert_eq!(deployment.state, DeploymentState::Collecting);
        assert!(deployment.state.is_active());
        assert!(!deployment.state.is_terminal());

        deployment.transition_to(DeploymentState::Validating);
        assert_eq!(deployment.state, DeploymentState::Validating);

        deployment.transition_to(DeploymentState::Complete(true));
        assert_eq!(deployment.state, DeploymentState::Complete(true));
        assert!(!deployment.state.is_active());
        assert!(deployment.state.is_terminal());
        assert!(deployment.end_time.is_some());
    }

    #[test]
    fn test_deployment_cancellation() {
        let mut deployment = DeploymentOperation::new(
            "test-stack".to_string(),
            "123456789012".to_string(),
            "us-east-1".to_string(),
            DeploymentType::Create,
            "{}".to_string(),
            HashMap::new(),
            "Test Project".to_string(),
            "dev".to_string(),
        );

        assert!(deployment.can_cancel());
        deployment.cancel();
        assert_eq!(deployment.state, DeploymentState::Cancelled);
        assert!(!deployment.can_cancel());
    }

    #[test]
    fn test_progress_calculation() {
        let mut deployment = DeploymentOperation::new(
            "test-stack".to_string(),
            "123456789012".to_string(),
            "us-east-1".to_string(),
            DeploymentType::Create,
            r#"{"Resources": {"Resource1": {}, "Resource2": {}}}"#.to_string(),
            HashMap::new(),
            "Test Project".to_string(),
            "dev".to_string(),
        );

        deployment.transition_to(DeploymentState::Deploying);
        assert_eq!(deployment.progress_percent, 20);

        // Add completion event
        let event = StackEvent {
            event_id: "event1".to_string(),
            stack_id: Some("stack-id".to_string()),
            stack_name: "test-stack".to_string(),
            logical_resource_id: Some("Resource1".to_string()),
            physical_resource_id: Some("physical-id".to_string()),
            resource_type: Some("AWS::S3::Bucket".to_string()),
            timestamp: Utc::now(),
            resource_status: "CREATE_COMPLETE".to_string(),
            resource_status_reason: None,
            resource_properties: None,
        };

        deployment.add_event(event);
        assert!(deployment.progress_percent > 20);
    }
}
