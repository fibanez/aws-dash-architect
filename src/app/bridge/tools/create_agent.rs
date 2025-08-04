//! Create_Agent Tool for Bridge Agent Orchestration
//!
//! This tool allows the Bridge Agent to create specialized agents on-demand
//! for complex AWS tasks, following the agent-on-demand architecture pattern.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use stood::tools::{Tool, ToolError, ToolResult};
// Note: CancellationToken will be added when integrating with real stood Agent system
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// Import bridge communication and specialized agents
use super::super::agents::{AwsLogAnalyzerAgent, AwsResourceAuditorAgent, AwsSecurityScannerAgent};
use super::super::get_global_bridge_sender;
use super::super::performance::{PerformanceTimer, AgentCreationMetrics};
use crate::app::dashui::control_bridge_window::AgentResponse as BridgeAgentResponse;
use crate::time_phase;

/// AWS context required for all AWS operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsContext {
    pub account_id: String,
    pub region: String,
    pub resource_identifier: String,
    pub access_key: String,
    pub secret_key: String,
    pub session_token: Option<String>,
}

impl AwsContext {
    /// Sanitized version for logging (NEVER includes credentials)
    pub fn sanitized_for_logging(&self) -> AwsContextSanitized {
        AwsContextSanitized {
            account_id: self.account_id.clone(),
            region: self.region.clone(),
            resource_identifier: self.resource_identifier.clone(),
        }
    }
}

/// Sanitized AWS context for logging (NEVER contains credentials)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsContextSanitized {
    pub account_id: String,
    pub region: String,
    pub resource_identifier: String,
}

/// Available specialized agent types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    #[serde(rename = "aws-log-analyzer")]
    AwsLogAnalyzer,
    #[serde(rename = "aws-resource-auditor")]
    AwsResourceAuditor,
    #[serde(rename = "aws-security-scanner")]
    AwsSecurityScanner,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::AwsLogAnalyzer => write!(f, "aws-log-analyzer"),
            AgentType::AwsResourceAuditor => write!(f, "aws-resource-auditor"),
            AgentType::AwsSecurityScanner => write!(f, "aws-security-scanner"),
        }
    }
}

/// Active agent tracking information
#[derive(Debug, Clone)]
pub struct ActiveAgent {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub created_at: DateTime<Utc>,
    pub task_description: String,
    // Note: cancel_token will be added when integrating with real stood Agent system
}

/// Maximum concurrent agents to prevent resource exhaustion
const MAX_CONCURRENT_AGENTS: usize = 3;

/// Create_Agent tool for orchestrating specialized agents
#[derive(Clone)]
pub struct CreateAgentTool {
    /// Track active agents for lifecycle management
    active_agents: Arc<Mutex<HashMap<String, ActiveAgent>>>,
}

impl std::fmt::Debug for CreateAgentTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateAgentTool")
            .field("active_agents", &"<HashMap<String, ActiveAgent>>")
            .finish()
    }
}

impl CreateAgentTool {
    pub fn new() -> Self {
        Self {
            active_agents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// CRITICAL: Validate AWS context before any operations
    fn validate_aws_context(&self, context: &AwsContext) -> Result<(), ToolError> {
        if context.account_id.is_empty() || context.account_id == "current" {
            return Err(ToolError::InvalidParameters {
                message: "account_id it should be a 12 digit number and it is REQUIRED for AWS operations. Use aws_find_account tool first.".to_string(),
            });
        }

        if context.region.is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "region is REQUIRED for AWS operations. Use aws_find_region tool first."
                    .to_string(),
            });
        }

        if context.resource_identifier.is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "resource_identifier is REQUIRED (resource ID, name, or ARN).".to_string(),
            });
        }

        if context.access_key.is_empty() || context.secret_key.is_empty() {
            return Err(ToolError::InvalidParameters {
                message: "AWS credentials are required but missing.".to_string(),
            });
        }

        Ok(())
    }

    /// Check concurrency limits to prevent resource exhaustion
    fn check_concurrency_limit(&self) -> Result<(), ToolError> {
        let active_count = self
            .active_agents
            .lock()
            .map_err(|e| {
                error!("Failed to lock active agents: {}", e);
                ToolError::ExecutionFailed {
                    message: "Failed to access agent tracking".to_string(),
                }
            })?
            .len();

        if active_count >= MAX_CONCURRENT_AGENTS {
            return Err(ToolError::ExecutionFailed {
                message: format!(
                    "Maximum concurrent agents ({}) reached. Please wait for current tasks to complete or use the Stop button to cancel active agents.",
                    MAX_CONCURRENT_AGENTS
                ),
            });
        }

        Ok(())
    }

    // Note: System prompts are now created within each specialized agent implementation
    // These methods have been removed to avoid duplication

    /// Create and execute real specialized agent
    async fn create_real_agent(
        &self,
        agent_type: &AgentType,
        task_description: &str,
        aws_context: &AwsContext,
        agent_id: &str,
    ) -> Result<serde_json::Value, ToolError> {
        let mut inner_timer = PerformanceTimer::new(&format!("Specialized Agent: {}", agent_type));
        info!("🚀 Creating real specialized agent: {}", agent_type);

        let result = match agent_type {
            AgentType::AwsLogAnalyzer => {
                // Create log analyzer agent with timing
                let mut agent = time_phase!(inner_timer, "Log Analyzer Agent creation", {
                    AwsLogAnalyzerAgent::create(
                        task_description.to_string(),
                        aws_context.clone(),
                        agent_id.to_string(),
                    )
                    .await
                    .map_err(|e| {
                        error!("Failed to create log analyzer agent: {}", e);
                        ToolError::ExecutionFailed {
                            message: format!("Failed to create log analyzer agent: {}", e),
                        }
                    })?
                });

                // Execute the analysis task with timing
                time_phase!(inner_timer, "Log Analyzer Agent execution", {
                    AwsLogAnalyzerAgent::execute_analysis(&mut agent, task_description)
                        .await
                        .map_err(|e| {
                            error!("Log analyzer agent execution failed: {}", e);
                            ToolError::ExecutionFailed {
                                message: format!("Log analyzer agent execution failed: {}", e),
                            }
                        })?
                })
            }
            AgentType::AwsResourceAuditor => {
                // Create resource auditor agent with timing
                let mut agent = time_phase!(inner_timer, "Resource Auditor Agent creation", {
                    AwsResourceAuditorAgent::create(
                        task_description.to_string(),
                        aws_context.clone(),
                        agent_id.to_string(),
                    )
                    .await
                    .map_err(|e| {
                        error!("Failed to create resource auditor agent: {}", e);
                        ToolError::ExecutionFailed {
                            message: format!("Failed to create resource auditor agent: {}", e),
                        }
                    })?
                });

                // Execute the audit task with timing
                time_phase!(inner_timer, "Resource Auditor Agent execution", {
                    AwsResourceAuditorAgent::execute_audit(&mut agent, task_description)
                        .await
                        .map_err(|e| {
                            error!("Resource auditor agent execution failed: {}", e);
                            ToolError::ExecutionFailed {
                                message: format!("Resource auditor agent execution failed: {}", e),
                            }
                        })?
                })
            }
            AgentType::AwsSecurityScanner => {
                // Create security scanner agent with timing
                let mut agent = time_phase!(inner_timer, "Security Scanner Agent creation", {
                    AwsSecurityScannerAgent::create(
                        task_description.to_string(),
                        aws_context.clone(),
                        agent_id.to_string(),
                    )
                    .await
                    .map_err(|e| {
                        error!("Failed to create security scanner agent: {}", e);
                        ToolError::ExecutionFailed {
                            message: format!("Failed to create security scanner agent: {}", e),
                        }
                    })?
                });

                // Execute the security assessment task with timing
                time_phase!(inner_timer, "Security Scanner Agent execution", {
                    AwsSecurityScannerAgent::execute_security_scan(&mut agent, task_description)
                        .await
                        .map_err(|e| {
                            error!("Security scanner agent execution failed: {}", e);
                            ToolError::ExecutionFailed {
                                message: format!("Security scanner agent execution failed: {}", e),
                            }
                        })?
                })
            }
        };

        inner_timer.complete();
        info!("✅ Specialized agent {} completed successfully", agent_type);
        Ok(result)
    }
}

#[async_trait]
impl Tool for CreateAgentTool {
    fn name(&self) -> &str {
        "create_agent"
    }

    fn description(&self) -> &str {
        "Create specialized AWS agents on-demand for complex tasks. \
         CRITICAL: All AWS operations require account_id, region, and resource_identifier. \
         Use this tool to delegate complex AWS tasks to specialized agents with focused toolsets."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent_type": {
                    "type": "string",
                    "enum": ["aws-log-analyzer", "aws-resource-auditor", "aws-security-scanner"],
                    "description": "Type of specialized agent to create"
                },
                "task_description": {
                    "type": "string",
                    "description": "Detailed description of the task for the agent to perform"
                },
                "aws_context": {
                    "type": "object",
                    "properties": {
                        "account_id": {
                            "type": "string",
                            "description": "AWS account ID (REQUIRED)"
                        },
                        "region": {
                            "type": "string",
                            "description": "AWS region (REQUIRED)"
                        },
                        "resource_identifier": {
                            "type": "string",
                            "description": "Resource ID, name, or ARN (REQUIRED)"
                        },
                        "access_key": {
                            "type": "string",
                            "description": "AWS access key (REQUIRED)"
                        },
                        "secret_key": {
                            "type": "string",
                            "description": "AWS secret key (REQUIRED)"
                        },
                        "session_token": {
                            "type": "string",
                            "description": "AWS session token (optional)"
                        }
                    },
                    "required": ["account_id", "region", "resource_identifier", "access_key", "secret_key"],
                    "description": "AWS context required for all operations"
                }
            },
            "required": ["agent_type", "task_description", "aws_context"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Start comprehensive performance timing
        let mut perf_timer = PerformanceTimer::new("Agent Creation");
        let creation_start = Instant::now();
        
        info!("🚀 Executing Create_Agent tool");

        let params = time_phase!(perf_timer, "Parameter extraction", {
            parameters.ok_or_else(|| {
                warn!("❌ No parameters provided to Create_Agent tool");
                ToolError::InvalidParameters {
                    message: "Parameters are required for agent creation".to_string(),
                }
            })?
        });

        // Extract and validate parameters with timing
        let (agent_type, task_description, aws_context) = time_phase!(perf_timer, "Parameter parsing & validation", {
            let agent_type_str = params
                .get("agent_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "agent_type is required".to_string(),
                })?;

            let agent_type = serde_json::from_str::<AgentType>(&format!("\"{}\"", agent_type_str))
                .map_err(|_| {
                    ToolError::InvalidParameters {
                        message: format!("Invalid agent_type: {}. Must be one of: aws-log-analyzer, aws-resource-auditor, aws-security-scanner", agent_type_str),
                    }
                })?;

            let task_description = params
                .get("task_description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters {
                    message: "task_description is required".to_string(),
                })?;

            let aws_context: AwsContext = serde_json::from_value(
                params
                    .get("aws_context")
                    .cloned()
                    .ok_or_else(|| {
                        ToolError::InvalidParameters {
                            message: "aws_context is required with account_id, region, resource_identifier, and credentials".to_string(),
                        }
                    })?
            ).map_err(|e| {
                ToolError::InvalidParameters {
                    message: format!("Invalid aws_context: {}", e),
                }
            })?;

            // CRITICAL: Validate AWS context
            self.validate_aws_context(&aws_context)?;

            // Check concurrency limits
            self.check_concurrency_limit()?;

            (agent_type, task_description, aws_context)
        });

        // Setup agent tracking and notifications
        let (agent_id, sanitized_context) = time_phase!(perf_timer, "Agent setup & tracking", {
            let agent_id = Uuid::new_v4().to_string();

            // Log sanitized context (NEVER log credentials)
            let sanitized_context = aws_context.sanitized_for_logging();
            info!(
                "🎯 Creating {} agent for task: {}",
                agent_type, task_description
            );
            debug!("🔍 AWS context: {:?}", sanitized_context);

            // Store active agent info
            {
                let mut active_agents = self.active_agents.lock().map_err(|e| {
                    error!("Failed to lock active agents: {}", e);
                    ToolError::ExecutionFailed {
                        message: "Failed to track active agent".to_string(),
                    }
                })?;

                active_agents.insert(
                    agent_id.clone(),
                    ActiveAgent {
                        agent_id: agent_id.clone(),
                        agent_type: agent_type.clone(),
                        created_at: Utc::now(),
                        task_description: task_description.to_string(),
                    },
                );
            }

            // Notify Bridge UI that agent was created
            if let Some(bridge_sender) = get_global_bridge_sender() {
                let _ = bridge_sender.send(BridgeAgentResponse::AgentCreated {
                    agent_id: agent_id.clone(),
                    agent_type: agent_type.to_string(),
                });
            }

            (agent_id, sanitized_context)
        });

        // Note: System prompts are now created within each specialized agent
        // No longer needed here since agents create their own prompts

        // Create and execute real specialized agent with timing
        let agent_result = time_phase!(perf_timer, "Agent creation & execution", {
            self.create_real_agent(&agent_type, &task_description, &aws_context, &agent_id).await
        });

        // Cleanup and notifications
        time_phase!(perf_timer, "Cleanup & notifications", {
            // Cleanup active agent tracking
            {
                let mut active_agents = self.active_agents.lock().map_err(|e| {
                    error!("Failed to lock active agents for cleanup: {}", e);
                    ToolError::ExecutionFailed {
                        message: "Failed to cleanup active agent".to_string(),
                    }
                })?;
                active_agents.remove(&agent_id);
            }

            // Notify Bridge UI that agent was destroyed
            if let Some(bridge_sender) = get_global_bridge_sender() {
                let _ = bridge_sender.send(BridgeAgentResponse::AgentDestroyed {
                    agent_id: agent_id.clone(),
                    agent_type: agent_type.to_string(),
                });
            }

            Ok::<(), ToolError>(())
        })?;

        // Complete performance timing and generate final metrics
        let total_duration = creation_start.elapsed();
        let success = agent_result.is_ok();
        
        // Create comprehensive metrics (placeholder durations for individual phases)
        let metrics = AgentCreationMetrics {
            agent_type: agent_type.to_string(),
            agent_id: agent_id.clone(),
            total_duration,
            validation_duration: std::time::Duration::from_millis(50), // Will be replaced with actual timing
            credential_duration: std::time::Duration::from_millis(100), // Will be replaced with actual timing  
            builder_setup_duration: std::time::Duration::from_millis(200), // Will be replaced with actual timing
            agent_build_duration: std::time::Duration::from_millis(1000), // Will be replaced with actual timing
            execution_duration: total_duration.saturating_sub(std::time::Duration::from_millis(1350)),
            success,
        };

        // Log structured performance metrics
        metrics.log_structured();
        metrics.analyze_performance();
        
        // Complete the performance timer
        perf_timer.complete();

        // Return results
        match agent_result {
            Ok(result) => {
                info!("✅ {} completed successfully", agent_id);
                Ok(ToolResult::success(serde_json::json!({
                    "success": true,
                    "agent_id": agent_id,
                    "agent_type": agent_type,
                    "task_description": task_description,
                    "aws_context": sanitized_context,
                    "result": result,
                    "created_at": Utc::now().to_rfc3339(),
                    "performance": {
                        "total_duration_ms": total_duration.as_millis(),
                        "success": success
                    }
                })))
            }
            Err(e) => {
                error!("❌ Agent {} failed: {}", agent_id, e);
                Err(e)
            }
        }
    }
}

impl Default for CreateAgentTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_aws_context() -> AwsContext {
        AwsContext {
            account_id: "123456789012".to_string(),
            region: "us-east-1".to_string(),
            resource_identifier: "test-resource".to_string(),
            access_key: "AKIA...".to_string(),
            secret_key: "secret...".to_string(),
            session_token: None,
        }
    }

    #[test]
    fn test_aws_context_validation() {
        let tool = CreateAgentTool::new();

        // Valid context
        let valid_context = create_test_aws_context();
        assert!(tool.validate_aws_context(&valid_context).is_ok());

        // Invalid - missing account_id
        let mut invalid_context = valid_context.clone();
        invalid_context.account_id = "".to_string();
        assert!(tool.validate_aws_context(&invalid_context).is_err());

        // Invalid - missing region
        let mut invalid_context = valid_context.clone();
        invalid_context.region = "".to_string();
        assert!(tool.validate_aws_context(&invalid_context).is_err());

        // Invalid - missing resource_identifier
        let mut invalid_context = valid_context.clone();
        invalid_context.resource_identifier = "".to_string();
        assert!(tool.validate_aws_context(&invalid_context).is_err());
    }

    #[test]
    fn test_sanitized_context() {
        let context = create_test_aws_context();
        let sanitized = context.sanitized_for_logging();

        assert_eq!(sanitized.account_id, context.account_id);
        assert_eq!(sanitized.region, context.region);
        assert_eq!(sanitized.resource_identifier, context.resource_identifier);
        // Credentials should NOT be in sanitized version
    }

    #[tokio::test]
    async fn test_agent_creation_mock() {
        let tool = CreateAgentTool::new();

        let params = serde_json::json!({
            "agent_type": "aws-log-analyzer",
            "task_description": "Analyze Lambda function errors",
            "aws_context": {
                "account_id": "123456789012",
                "region": "us-east-1",
                "resource_identifier": "my-lambda-function",
                "access_key": "AKIA...",
                "secret_key": "secret..."
            }
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(result.success);
        let response = result.data.unwrap();
        assert_eq!(response["agent_type"], "aws-log-analyzer");
        assert!(response["agent_id"].is_string());
    }
}

