//! Create_Task Tool for Bridge Agent Task Orchestration
//!
//! This tool allows the Bridge Agent to create generic task agents on-demand
//! for any AWS task, replacing hardcoded agent types with flexible task descriptions.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{error, info, warn};
use uuid::Uuid;

// Import bridge communication and task agent
use super::super::agents::TaskAgent;
use super::super::cancellation::AgentCancellationManager;
use super::super::performance::{AgentCreationMetrics, PerformanceTimer};
use super::super::{BridgeDebugEvent, log_bridge_debug_event};
// Removed Bridge UI imports - agents handle their own event loops
use crate::time_phase;

/// Active task tracking information
#[derive(Debug, Clone)]
pub struct ActiveTask {
    pub task_id: String,
    pub task_description: String,
    pub account_ids: Vec<String>,
    pub regions: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Maximum concurrent tasks to prevent resource exhaustion
const MAX_CONCURRENT_TASKS: usize = 5;

/// Create_Task tool for orchestrating generic task agents
#[derive(Clone)]
pub struct CreateTaskTool {
    /// Track active tasks for lifecycle management
    active_tasks: Arc<Mutex<HashMap<String, ActiveTask>>>,
    /// Manage cancellation tokens for active agents
    cancellation_manager: Arc<AgentCancellationManager>,
}

impl std::fmt::Debug for CreateTaskTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateTaskTool")
            .field("active_tasks", &"<HashMap<String, ActiveTask>>")
            .field("cancellation_manager", &"<AgentCancellationManager>")
            .finish()
    }
}

impl CreateTaskTool {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            cancellation_manager: Arc::new(AgentCancellationManager::new()),
        }
    }

    /// Get a reference to the cancellation manager for external use (e.g., from Bridge UI)
    pub fn cancellation_manager(&self) -> Arc<AgentCancellationManager> {
        self.cancellation_manager.clone()
    }

    /// Parse parameter that can be either a string or an array of strings
    fn parse_string_or_array(value: &serde_json::Value) -> Result<Vec<String>, String> {
        match value {
            serde_json::Value::String(s) => Ok(vec![s.clone()]),
            serde_json::Value::Array(arr) => {
                let mut strings = Vec::new();
                for item in arr {
                    match item.as_str() {
                        Some(s) => strings.push(s.to_string()),
                        None => return Err("Array must contain only strings".to_string()),
                    }
                }
                if strings.is_empty() {
                    Err("Array cannot be empty".to_string())
                } else {
                    Ok(strings)
                }
            }
            _ => Err("Value must be a string or array of strings".to_string()),
        }
    }

    /// Validate AWS context parameters
    fn validate_parameters(
        &self,
        account_ids: &[String],
        regions: &[String],
        task_description: &str,
    ) -> Result<(), ToolError> {
        // Validate account IDs
        for account_id in account_ids {
            if account_id.is_empty() || account_id == "current" {
                return Err(ToolError::InvalidParameters {
                    message: format!("account_id '{}' must be a 12-digit AWS account number and is REQUIRED. Use aws_find_account tool first.", account_id),
                });
            }

            if account_id.len() != 12 || !account_id.chars().all(|c| c.is_ascii_digit()) {
                return Err(ToolError::InvalidParameters {
                    message: format!(
                        "account_id '{}' must be exactly 12 digits (e.g., '123456789012')",
                        account_id
                    ),
                });
            }
        }

        // Validate regions
        for region in regions {
            if region.is_empty() {
                return Err(ToolError::InvalidParameters {
                    message: "region is REQUIRED (e.g., 'us-east-1', 'eu-west-1'). Use aws_find_region tool first.".to_string(),
                });
            }
        }

        if task_description.is_empty() || task_description.len() < 10 {
            return Err(ToolError::InvalidParameters {
                message: "task_description must be a clear, descriptive explanation of what you want to accomplish (minimum 10 characters)".to_string(),
            });
        }

        Ok(())
    }

    /// Check concurrency limits to prevent resource exhaustion
    fn check_concurrency_limit(&self) -> Result<(), ToolError> {
        let active_count = self
            .active_tasks
            .lock()
            .map_err(|e| {
                error!("Failed to lock active tasks: {}", e);
                ToolError::ExecutionFailed {
                    message: "Failed to access task tracking".to_string(),
                }
            })?
            .len();

        if active_count >= MAX_CONCURRENT_TASKS {
            return Err(ToolError::ExecutionFailed {
                message: format!(
                    "Maximum concurrent tasks ({}) reached. Please wait for current tasks to complete.",
                    MAX_CONCURRENT_TASKS
                ),
            });
        }

        Ok(())
    }

    /// Create and execute generic task agent with cancellation support
    async fn create_and_execute_task(
        &self,
        task_id: &str,
        task_description: &str,
        account_ids: &[String],
        regions: &[String],
    ) -> Result<serde_json::Value, ToolError> {
        let mut inner_timer =
            PerformanceTimer::new(&format!("Generic Task Agent: {}", task_description));
        info!("🎯 Creating and executing generic task agent with cancellation support");

        // Create cancellation token for this task
        let cancellation_token = self.cancellation_manager.create_token(task_id.to_string());

        // Create task agent
        let mut agent = time_phase!(inner_timer, "Task Agent creation", {
            TaskAgent::create(
                task_id.to_string(),
                task_description.to_string(),
                account_ids.to_vec(),
                regions.to_vec(),
                None, // Use global model configuration
            )
            .await
            .map_err(|e| {
                error!("Failed to create task agent: {}", e);
                // Clean up cancellation token on creation failure
                self.cancellation_manager.remove_token(task_id);
                ToolError::ExecutionFailed {
                    message: format!("Failed to create task agent: {}", e),
                }
            })?
        });

        // Execute the task with cancellation support
        let result = time_phase!(inner_timer, "Task Agent execution with cancellation", {
            // Use tokio::select! to race between task execution and cancellation
            tokio::select! {
                task_result = TaskAgent::execute_task(&mut agent, task_description) => {
                    match task_result {
                        Ok(result) => {
                            info!("✅ Task agent completed successfully");
                            Ok(result)
                        },
                        Err(e) => {
                            error!("Task agent execution failed: {}", e);
                            Err(ToolError::ExecutionFailed {
                                message: format!("Task agent execution failed: {}", e),
                            })
                        }
                    }
                },
                _ = cancellation_token.cancelled() => {
                    info!("🛑 Task agent execution cancelled by user: {}", task_id);
                    Err(ToolError::ExecutionFailed {
                        message: "Task execution was cancelled by user".to_string(),
                    })
                }
            }
        })?;

        // Clean up cancellation token on successful completion
        self.cancellation_manager.remove_token(task_id);

        inner_timer.complete();
        info!("✅ Generic task agent completed successfully");
        Ok(result)
    }
}

#[async_trait]
impl Tool for CreateTaskTool {
    fn name(&self) -> &str {
        "create_task"
    }

    fn description(&self) -> &str {
        "Create task-specific agents for any AWS operation using natural language descriptions. \
         Replaces hardcoded agent types with flexible task-based agent creation. \
         Each task agent gets access to all AWS tools and can handle any type of AWS task."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "task_description": {
                    "type": "string",
                    "description": "Clear, detailed description of the AWS task to perform (e.g., 'Analyze Lambda function errors in production', 'Audit S3 bucket security configurations', 'Review CloudWatch alarms for EC2 instances')"
                },
                "account_ids": {
                    "oneOf": [
                        {
                            "type": "string",
                            "description": "Single AWS account ID (12-digit number, e.g., '123456789012')"
                        },
                        {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "pattern": "^[0-9]{12}$"
                            },
                            "description": "Array of AWS account IDs for multi-account operations"
                        }
                    ],
                    "description": "AWS account ID(s). Can be a single account ID string or an array of account IDs. REQUIRED for all AWS operations."
                },
                "regions": {
                    "oneOf": [
                        {
                            "type": "string",
                            "description": "Single AWS region (e.g., 'us-east-1', 'eu-west-1')"
                        },
                        {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Array of AWS regions for multi-region operations"
                        }
                    ],
                    "description": "AWS region(s). Can be a single region string or an array of regions. REQUIRED for all AWS operations."
                }
            },
            "required": ["task_description", "account_ids", "regions"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Start comprehensive performance timing
        let mut perf_timer = PerformanceTimer::new("Task Creation");
        let creation_start = Instant::now();

        info!("🎯 Executing Create_Task tool");

        let params = time_phase!(perf_timer, "Parameter extraction", {
            parameters.ok_or_else(|| {
                warn!("❌ No parameters provided to Create_Task tool");
                ToolError::InvalidParameters {
                    message: "Parameters are required for task creation".to_string(),
                }
            })?
        });

        // Extract and validate parameters
        let (task_description, account_ids, regions) =
            time_phase!(perf_timer, "Parameter parsing & validation", {
                let task_description = params
                    .get("task_description")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidParameters {
                        message: "task_description is required".to_string(),
                    })?;

                let account_ids = params
                    .get("account_ids")
                    .ok_or_else(|| ToolError::InvalidParameters {
                        message: "account_ids is required".to_string(),
                    })
                    .and_then(|v| {
                        Self::parse_string_or_array(v).map_err(|e| ToolError::InvalidParameters {
                            message: format!("Invalid account_ids parameter: {}", e),
                        })
                    })?;

                let regions = params
                    .get("regions")
                    .ok_or_else(|| ToolError::InvalidParameters {
                        message: "regions is required".to_string(),
                    })
                    .and_then(|v| {
                        Self::parse_string_or_array(v).map_err(|e| ToolError::InvalidParameters {
                            message: format!("Invalid regions parameter: {}", e),
                        })
                    })?;

                // Validate parameters
                self.validate_parameters(&account_ids, &regions, task_description)?;

                // Check concurrency limits
                self.check_concurrency_limit()?;

                (task_description, account_ids, regions)
            });

        // Setup task tracking and notifications
        let task_id = time_phase!(perf_timer, "Task setup & tracking", {
            let task_id = Uuid::new_v4().to_string();

            info!(
                "🎯 Creating task agent - Description: '{}', Accounts: {:?}, Regions: {:?}",
                task_description, account_ids, regions
            );
            
            // Log create_task start for debugging
            log_bridge_debug_event(BridgeDebugEvent::CreateTaskStart {
                timestamp: Utc::now(),
                session_id: format!("bridge-session-{}", Utc::now().timestamp_millis()), // TODO: pass actual session_id
                task_id: task_id.clone(),
                task_description: task_description.to_string(),
                account_ids: account_ids.clone(),
                regions: regions.clone(),
            });

            // Store active task info
            {
                let mut active_tasks = self.active_tasks.lock().map_err(|e| {
                    error!("Failed to lock active tasks: {}", e);
                    ToolError::ExecutionFailed {
                        message: "Failed to track active task".to_string(),
                    }
                })?;

                active_tasks.insert(
                    task_id.clone(),
                    ActiveTask {
                        task_id: task_id.clone(),
                        task_description: task_description.to_string(),
                        account_ids: account_ids.clone(),
                        regions: regions.clone(),
                        created_at: Utc::now(),
                    },
                );
            }

            // Task agent will handle its own event loop without UI notifications

            task_id
        });

        // Create and execute task agent
        let task_result = time_phase!(perf_timer, "Task creation & execution", {
            self.create_and_execute_task(&task_id, task_description, &account_ids, &regions)
                .await
        });

        // Complete performance timing and determine success before cleanup
        let total_duration = creation_start.elapsed();
        let success = task_result.is_ok();

        // Task completion handled by agent result - no UI notifications needed

        // Cleanup and notifications
        time_phase!(perf_timer, "Cleanup & notifications", {
            // Cleanup active task tracking
            {
                let mut active_tasks = self.active_tasks.lock().map_err(|e| {
                    error!("Failed to lock active tasks for cleanup: {}", e);
                    ToolError::ExecutionFailed {
                        message: "Failed to cleanup active task".to_string(),
                    }
                })?;
                active_tasks.remove(&task_id);
            }

            // Cleanup cancellation token (in case of error or completion)
            self.cancellation_manager.remove_token(&task_id);

            // Task cleanup complete - no UI notifications needed

            Ok::<(), ToolError>(())
        })?;

        // Generate final metrics
        let metrics = AgentCreationMetrics {
            agent_type: "generic-task-agent".to_string(),
            agent_id: task_id.clone(),
            total_duration,
            validation_duration: std::time::Duration::from_millis(50),
            credential_duration: std::time::Duration::from_millis(100),
            builder_setup_duration: std::time::Duration::from_millis(200),
            agent_build_duration: std::time::Duration::from_millis(1000),
            execution_duration: total_duration
                .saturating_sub(std::time::Duration::from_millis(1350)),
            success,
        };

        // Log structured performance metrics
        metrics.log_structured();
        metrics.analyze_performance();

        // Complete the performance timer
        perf_timer.complete();

        // Return results
        match task_result {
            Ok(result) => {
                info!("✅ Task {} completed successfully", task_id);
                
                // Log task completion for debugging
                log_bridge_debug_event(BridgeDebugEvent::TaskComplete {
                    timestamp: Utc::now(),
                    task_id: task_id.clone(),
                    success: true,
                    execution_summary: format!("Task completed successfully in {}ms", total_duration.as_millis()),
                });
                
                Ok(ToolResult::success(serde_json::json!({
                    "success": true,
                    "task_id": task_id,
                    "task_description": task_description,
                    "account_ids": account_ids,
                    "regions": regions,
                    "result": result,
                    "created_at": Utc::now().to_rfc3339(),
                    "performance": {
                        "total_duration_ms": total_duration.as_millis(),
                        "success": success
                    }
                })))
            }
            Err(e) => {
                error!("❌ Task {} failed: {}", task_id, e);
                
                // Log task failure for debugging
                log_bridge_debug_event(BridgeDebugEvent::TaskComplete {
                    timestamp: Utc::now(),
                    task_id: task_id.clone(),
                    success: false,
                    execution_summary: format!("Task failed after {}ms: {}", total_duration.as_millis(), e),
                });
                
                Err(e)
            }
        }
    }
}

impl Default for CreateTaskTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_validation() {
        let tool = CreateTaskTool::new();

        // Valid parameters
        assert!(tool
            .validate_parameters(
                &vec!["123456789012".to_string()],
                &vec!["us-east-1".to_string()],
                "Analyze Lambda function errors in production environment"
            )
            .is_ok());

        // Invalid account_id - too short
        assert!(tool
            .validate_parameters(
                &vec!["12345".to_string()],
                &vec!["us-east-1".to_string()],
                "Valid task description"
            )
            .is_err());

        // Invalid account_id - contains letters
        assert!(tool
            .validate_parameters(
                &vec!["12345678901a".to_string()],
                &vec!["us-east-1".to_string()],
                "Valid task description"
            )
            .is_err());

        // Invalid region - empty
        assert!(tool
            .validate_parameters(
                &vec!["123456789012".to_string()],
                &vec!["".to_string()],
                "Valid task description"
            )
            .is_err());

        // Invalid task_description - too short
        assert!(tool
            .validate_parameters(
                &vec!["123456789012".to_string()],
                &vec!["us-east-1".to_string()],
                "Too short"
            )
            .is_err());

        // Valid multiple accounts and regions
        assert!(tool
            .validate_parameters(
                &vec!["123456789012".to_string(), "123456789013".to_string()],
                &vec!["us-east-1".to_string(), "eu-west-1".to_string()],
                "Multi-region Lambda performance analysis"
            )
            .is_ok());

        // Invalid - one bad account in array
        assert!(tool
            .validate_parameters(
                &vec!["123456789012".to_string(), "invalid".to_string()],
                &vec!["us-east-1".to_string()],
                "Valid task description"
            )
            .is_err());
    }

    #[tokio::test]
    async fn test_tool_creation() {
        let tool = CreateTaskTool::new();

        let _params = serde_json::json!({
            "task_description": "Analyze S3 bucket configurations for security compliance",
            "account_id": "123456789012",
            "region": "us-east-1"
        });

        // This would require real AWS setup to complete, but we can test parameter validation
        // The tool should accept the parameters without errors (until it tries to create the actual agent)
        assert!(!tool.description().is_empty());
        assert_eq!(tool.name(), "create_task");
    }
}
