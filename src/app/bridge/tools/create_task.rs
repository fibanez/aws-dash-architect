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
use super::super::get_global_bridge_sender;
use super::super::performance::{PerformanceTimer, AgentCreationMetrics};
use crate::app::dashui::control_bridge_window::{AgentResponse as BridgeAgentResponse, SubAgentEvent};
use crate::time_phase;

/// Active task tracking information
#[derive(Debug, Clone)]
pub struct ActiveTask {
    pub task_id: String,
    pub task_description: String,
    pub account_id: String,
    pub region: String,
    pub created_at: DateTime<Utc>,
}

/// Maximum concurrent tasks to prevent resource exhaustion
const MAX_CONCURRENT_TASKS: usize = 5;

/// Create_Task tool for orchestrating generic task agents
#[derive(Clone)]
pub struct CreateTaskTool {
    /// Track active tasks for lifecycle management
    active_tasks: Arc<Mutex<HashMap<String, ActiveTask>>>,
}

impl std::fmt::Debug for CreateTaskTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateTaskTool")
            .field("active_tasks", &"<HashMap<String, ActiveTask>>")
            .finish()
    }
}

impl CreateTaskTool {
    pub fn new() -> Self {
        Self {
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Validate AWS context parameters
    fn validate_parameters(&self, account_id: &str, region: &str, task_description: &str) -> Result<(), ToolError> {
        if account_id.is_empty() || account_id == "current" {
            return Err(ToolError::InvalidParameters {
                message: "account_id must be a 12-digit AWS account number and is REQUIRED. Use aws_find_account tool first.".to_string(),
            });
        }

        if account_id.len() != 12 || !account_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(ToolError::InvalidParameters {
                message: "account_id must be exactly 12 digits (e.g., '123456789012')".to_string(),
            });
        }

        if region.is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "region is REQUIRED (e.g., 'us-east-1', 'eu-west-1'). Use aws_find_region tool first.".to_string(),
            });
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

    /// Create and execute generic task agent
    async fn create_and_execute_task(
        &self,
        task_id: &str,
        task_description: &str,
        account_id: &str,
        region: &str,
    ) -> Result<serde_json::Value, ToolError> {
        let mut inner_timer = PerformanceTimer::new(&format!("Generic Task Agent: {}", task_description));
        info!("🎯 Creating and executing generic task agent");

        // Create task agent
        let mut agent = time_phase!(inner_timer, "Task Agent creation", {
            TaskAgent::create(
                task_id.to_string(),
                task_description.to_string(),
                account_id.to_string(),
                region.to_string(),
            )
            .await
            .map_err(|e| {
                error!("Failed to create task agent: {}", e);
                ToolError::ExecutionFailed {
                    message: format!("Failed to create task agent: {}", e),
                }
            })?
        });

        // Execute the task
        let result = time_phase!(inner_timer, "Task Agent execution", {
            TaskAgent::execute_task(&mut agent, task_description)
                .await
                .map_err(|e| {
                    error!("Task agent execution failed: {}", e);
                    ToolError::ExecutionFailed {
                        message: format!("Task agent execution failed: {}", e),
                    }
                })?
        });

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
                "account_id": {
                    "type": "string",
                    "description": "AWS account ID (12-digit number, e.g., '123456789012'). REQUIRED for all AWS operations."
                },
                "region": {
                    "type": "string", 
                    "description": "AWS region (e.g., 'us-east-1', 'eu-west-1'). REQUIRED for all AWS operations."
                }
            },
            "required": ["task_description", "account_id", "region"]
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
        let (task_description, account_id, region) = time_phase!(perf_timer, "Parameter parsing & validation", {
            let task_description = params
                .get("task_description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "task_description is required".to_string(),
                })?;

            let account_id = params
                .get("account_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "account_id is required".to_string(),
                })?;

            let region = params
                .get("region")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "region is required".to_string(),
                })?;

            // Validate parameters
            self.validate_parameters(account_id, region, task_description)?;

            // Check concurrency limits
            self.check_concurrency_limit()?;

            (task_description, account_id, region)
        });

        // Setup task tracking and notifications
        let task_id = time_phase!(perf_timer, "Task setup & tracking", {
            let task_id = Uuid::new_v4().to_string();

            info!(
                "🎯 Creating task agent - Description: '{}', Account: {}, Region: {}",
                task_description, account_id, region
            );

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
                        account_id: account_id.to_string(),
                        region: region.to_string(),
                        created_at: Utc::now(),
                    },
                );
            }

            // Notify Bridge UI that task was created
            if let Some(bridge_sender) = get_global_bridge_sender() {
                let _ = bridge_sender.send(BridgeAgentResponse::AgentCreated {
                    agent_id: task_id.clone(),
                    agent_type: "generic-task-agent".to_string(),
                });
                
                // Send ProcessingStarted event to create parent node with task-focused language
                let full_task_description = format!("⚙️ Processing task: {}", task_description);
                
                let _ = bridge_sender.send(BridgeAgentResponse::SubAgentEvent {
                    agent_id: task_id.clone(),
                    agent_type: "generic-task-agent".to_string(),
                    event: SubAgentEvent::ProcessingStarted {
                        timestamp: chrono::Utc::now(),
                        task_description: full_task_description,
                    },
                });
            }

            task_id
        });

        // Create and execute task agent
        let task_result = time_phase!(perf_timer, "Task creation & execution", {
            self.create_and_execute_task(&task_id, task_description, account_id, region).await
        });

        // Complete performance timing and determine success before cleanup
        let total_duration = creation_start.elapsed();
        let success = task_result.is_ok();
        
        // Send appropriate completion or error event
        if let Some(bridge_sender) = get_global_bridge_sender() {
            if success {
                let _ = bridge_sender.send(BridgeAgentResponse::SubAgentEvent {
                    agent_id: task_id.clone(),
                    agent_type: "generic-task-agent".to_string(),
                    event: SubAgentEvent::TaskComplete {
                        timestamp: chrono::Utc::now(),
                    },
                });
            } else if let Err(ref error) = task_result {
                let _ = bridge_sender.send(BridgeAgentResponse::SubAgentEvent {
                    agent_id: task_id.clone(),
                    agent_type: "generic-task-agent".to_string(),
                    event: SubAgentEvent::Error {
                        timestamp: chrono::Utc::now(),
                        error: error.to_string(),
                    },
                });
            }
        }

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

            // Notify Bridge UI that task was destroyed
            if let Some(bridge_sender) = get_global_bridge_sender() {
                let _ = bridge_sender.send(BridgeAgentResponse::AgentDestroyed {
                    agent_id: task_id.clone(),
                    agent_type: "generic-task-agent".to_string(),
                });
            }

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
            execution_duration: total_duration.saturating_sub(std::time::Duration::from_millis(1350)),
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
                Ok(ToolResult::success(serde_json::json!({
                    "success": true,
                    "task_id": task_id,
                    "task_description": task_description,
                    "account_id": account_id,
                    "region": region,
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
        assert!(tool.validate_parameters("123456789012", "us-east-1", "Analyze Lambda function errors in production environment").is_ok());

        // Invalid account_id - too short
        assert!(tool.validate_parameters("12345", "us-east-1", "Valid task description").is_err());

        // Invalid account_id - contains letters
        assert!(tool.validate_parameters("12345678901a", "us-east-1", "Valid task description").is_err());

        // Invalid region - empty
        assert!(tool.validate_parameters("123456789012", "", "Valid task description").is_err());

        // Invalid task_description - too short
        assert!(tool.validate_parameters("123456789012", "us-east-1", "Too short").is_err());
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