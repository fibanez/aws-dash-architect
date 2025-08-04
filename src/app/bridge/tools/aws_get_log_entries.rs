//! AWS Get Log Entries Tool
//!
//! This is a high-level tool that creates a standalone agent with CloudWatch-specific tools
//! to analyze logs based on natural language queries and resource IDs.

use crate::app::dashui::control_bridge_window::{
    AgentResponse as BridgeAgentResponse, LogAnalysisEvent,
};
use crate::app::resource_explorer::aws_client::AWSResourceClient;
use async_trait::async_trait;
use chrono::Utc;
use serde_json;
use std::sync::{mpsc, Arc};
use stood::agent::callbacks::{CallbackError, CallbackEvent, CallbackHandler, ToolEvent};
use stood::agent::Agent;
use stood::telemetry::{TelemetryConfig, LogLevel};
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{debug, info, warn};

use super::super::{
    aws_describe_log_groups_tool, aws_find_account_tool, aws_find_region_tool,
    aws_get_log_events_tool, get_global_aws_client, get_global_aws_credentials,
    get_global_bridge_sender,
};

/// AWS Get Log Entries Tool - Creates standalone agents for log analysis
#[derive(Clone)]
pub struct AwsGetLogEntriesTool {
    aws_client: Option<Arc<AWSResourceClient>>,
}

impl std::fmt::Debug for AwsGetLogEntriesTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwsGetLogEntriesTool")
            .field("aws_client", &self.aws_client.is_some())
            .finish()
    }
}

impl AwsGetLogEntriesTool {
    pub fn new(aws_client: Option<Arc<AWSResourceClient>>) -> Self {
        Self { aws_client }
    }

    /// Create a new tool without AWS client (will be set later)
    pub fn new_uninitialized() -> Self {
        Self { aws_client: None }
    }

    /// Set the AWS client for this tool
    pub fn set_aws_client(&mut self, aws_client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = aws_client;
    }

    /// Create a standalone agent with CloudWatch tools
    async fn create_log_analysis_agent(
        &self,
        aws_client: Arc<AWSResourceClient>,
    ) -> Result<Agent, ToolError> {
        let system_prompt = r#"You are a CloudWatch log analysis specialist. Your role is to help users find and analyze log entries from AWS CloudWatch.

You have access to the following tools:
- aws_describe_log_groups: Discover and list CloudWatch log groups with filtering capabilities
- aws_get_log_events: Retrieve specific log events from log groups and streams
- aws_find_account: Identify AWS account information
- aws_find_region: Identify AWS region information

When analyzing logs:
1. If given a resource ID, first try to identify the corresponding log group names
2. Use aws_describe_log_groups to discover relevant log groups
3. Use aws_get_log_events to retrieve the actual log events
4. Apply appropriate time filters and search patterns
5. Summarize findings in a clear, structured way

Be proactive in suggesting related log groups and time ranges if the initial search doesn't yield results."#;

        // Configure telemetry for the log analysis agent with descriptive naming
        let mut telemetry_config = TelemetryConfig::default()
            .with_service_name("aws-dash-log-entries-agent")
            .with_service_version("1.0.0")
            .with_otlp_endpoint("http://localhost:4320") // HTTP OTLP endpoint (matches auto-detection)
            .with_batch_processing()
            .with_log_level(LogLevel::DEBUG); // Enable DEBUG level logging to remote server

        // Enable debug tracing and add comprehensive service attributes
        telemetry_config.enable_debug_tracing = true;
        telemetry_config
            .service_attributes
            .insert("application".to_string(), "aws-dash-architect".to_string());
        telemetry_config.service_attributes.insert(
            "agent.type".to_string(),
            "aws-log-analysis-specialist".to_string(),
        );
        telemetry_config.service_attributes.insert(
            "agent.role".to_string(),
            "cloudwatch-log-analyzer".to_string(),
        );
        telemetry_config.service_attributes.insert(
            "agent.description".to_string(),
            "AWS CloudWatch Log Analysis Agent".to_string(),
        );
        telemetry_config
            .service_attributes
            .insert("component".to_string(), "log-analysis-system".to_string());
        telemetry_config.service_attributes.insert(
            "agent.capabilities".to_string(),
            "log-discovery,event-retrieval,pattern-analysis".to_string(),
        );
        telemetry_config
            .service_attributes
            .insert("environment".to_string(), "aws-dash-desktop".to_string());

        // Add unique session identifier for this agent instance
        let session_id = format!("aws-dash-log-entries-{}", Utc::now().timestamp_millis());
        telemetry_config
            .service_attributes
            .insert("session.id".to_string(), session_id.clone());
        telemetry_config.service_attributes.insert(
            "deployment.environment".to_string(),
            "desktop-application".to_string(),
        );

        // Create agent with CloudWatch-specific tools, telemetry, and callback handler
        let mut agent_builder = Agent::builder()
            .system_prompt(system_prompt)
            .with_telemetry(telemetry_config)
            .tools(vec![
                aws_describe_log_groups_tool(Some(aws_client.clone())),
                aws_get_log_events_tool(Some(aws_client.clone())),
                aws_find_account_tool(),
                aws_find_region_tool(),
            ]);

        // Add callback handler for event bubbling to Bridge
        if let Some(bridge_sender) = get_global_bridge_sender() {
            info!("📡 Log analysis agent using Bridge event bubbling");
            agent_builder = agent_builder.with_callback_handler(
                LogEntriesCallbackHandler::with_sender(session_id.clone(), bridge_sender),
            );
        } else {
            info!("📊 Log analysis agent without Bridge event bubbling (standalone mode)");
            agent_builder = agent_builder
                .with_callback_handler(LogEntriesCallbackHandler::new(session_id.clone()));
        }

        // Add AWS credentials if available globally (same as main agent)
        if let Some((access_key, secret_key, session_token, region)) = get_global_aws_credentials()
        {
            info!("🔐 Using global AWS credentials for standalone log analysis agent");
            agent_builder =
                agent_builder.with_credentials(access_key, secret_key, session_token, region);
        } else {
            warn!("⚠️ No global AWS credentials available for standalone agent - using default credential chain");
        }

        let agent = agent_builder
            .build()
            .await
            .map_err(|e| {
                warn!("❌ Failed to create log analysis agent: {}", e);
                ToolError::ExecutionFailed {
                    message: format!("Failed to create log analysis agent: {}. Could not initialize standalone agent for log analysis", e),
                }
            })?;

        Ok(agent)
    }
}

#[async_trait]
impl Tool for AwsGetLogEntriesTool {
    fn name(&self) -> &str {
        "aws_get_log_entries"
    }

    fn description(&self) -> &str {
        "High-level tool for retrieving CloudWatch logs using natural language queries. \
         Creates a specialized agent that can discover log groups, retrieve log events, \
         and analyze them based on resource IDs and descriptive queries. "
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query_description": {
                    "type": "string",
                    "description": "Natural language description of what to find in the logs (e.g., 'Find errors in the last hour', 'Show Lambda cold starts', 'Get recent API Gateway 5xx responses')"
                },
                "resource_id": {
                    "type": "string",
                    "description": "AWS resource identifier to find logs for (e.g., Lambda function name, ECS service name, API Gateway ID, EC2 instance ID)"
                },
                "time_range": {
                    "type": "string",
                    "description": "Optional time range specification (e.g., 'last 1 hour', 'last 24 hours', '2024-01-01 to 2024-01-02')"
                },
                "max_events": {
                    "type": "integer",
                    "description": "Maximum number of log events to analyze (default: 100, max: 1000)",
                    "minimum": 1,
                    "maximum": 1000
                },
                "account_id": {
                    "type": "string",
                    "description": "AWS account ID (required)"
                },
                "region": {
                    "type": "string",
                    "description": "AWS region (required)"
                }
            },
            "required": ["query_description", "resource_id", "account_id", "region"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        info!("🚀 Executing AWS Get Log Entries tool - creating standalone agent");

        // Get AWS client - prefer passed client over global
        let aws_client = self
            .aws_client
            .clone()
            .or_else(get_global_aws_client)
            .ok_or_else(|| {
                warn!("❌ AWS client not available for log entries operation");
                ToolError::ExecutionFailed {
                    message: "AWS client not configured. Please ensure AWS credentials are set up"
                        .to_string(),
                }
            })?;

        // Parse parameters
        let params = parameters.unwrap_or_default();

        let query_description = params
            .get("query_description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                warn!("❌ query_description parameter is required");
                ToolError::InvalidParameters {
                    message: "query_description parameter is required. Provide a natural language description of what to find in the logs".to_string(),
                }
            })?;

        let resource_id = params
            .get("resource_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                warn!("❌ resource_id parameter is required");
                ToolError::InvalidParameters {
                    message: "resource_id parameter is required. Provide the AWS resource identifier to find logs for".to_string(),
                }
            })?;

        let time_range = params
            .get("time_range")
            .and_then(|v| v.as_str())
            .unwrap_or("last 1 hour");

        let max_events = params
            .get("max_events")
            .and_then(|v| v.as_i64())
            .unwrap_or(100)
            .min(1000) as u32;

        let account_id = params
            .get("account_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                warn!("❌ account_id parameter is required");
                ToolError::InvalidParameters {
                    message: "account_id parameter is required. Please provide the AWS account ID (e.g., '123456789012')".to_string(),
                }
            })?;

        let region = params
            .get("region")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                warn!("❌ region parameter is required");
                ToolError::InvalidParameters {
                    message: "region parameter is required. Please provide the AWS region (e.g., 'us-east-1', 'eu-west-1')".to_string(),
                }
            })?;

        info!(
            "🔍 Creating log analysis agent for query: '{}', resource: '{}'",
            query_description, resource_id
        );

        // Create standalone agent for log analysis with automatic Bridge integration
        let mut agent = self.create_log_analysis_agent(aws_client).await?;

        // Construct the query for the agent
        let mut agent_query = format!(
            "I need to get the CloudWatch logs for the following:\n\n\
             Query: {}\n\
             Resource ID: {}\n\
             Time Range: {}\n\
             Max Events: {}\n\n",
            query_description, resource_id, time_range, max_events
        );

        agent_query.push_str(&format!("Account ID: {}\n", account_id));
        agent_query.push_str(&format!("Region: {}\n", region));

        agent_query.push_str(&format!(
            "\nPlease:\n\
             1. Identify the most relevant log groups for resource '{}'\n\
             2. Search for log events matching the query within the specified time range\n\
             3. Apply appropriate filters and patterns\n\
             4. Provide a summary of findings with key insights\n",
            resource_id
        ));

        info!("📋 Sending query to log analysis agent");

        // Execute the query with the standalone agent
        match agent.execute(&agent_query).await {
            Ok(response) => {
                info!("✅ Log analysis completed successfully");

                // The agent will be automatically dropped here, cleaning up memory
                let result = ToolResult::success(serde_json::json!({
                    "success": true,
                    "query_description": query_description,
                    "resource_id": resource_id,
                    "time_range": time_range,
                    "max_events": max_events,
                    "analysis_result": response.response,
                    "agent_metadata": {
                        "created": true,
                        "destroyed": true,
                        "memory_managed": true
                    }
                }));

                info!("🧹 Standalone agent destroyed and memory cleaned up");
                Ok(result)
            }
            Err(e) => {
                warn!("❌ Log analysis agent failed: {}", e);
                // Agent will still be dropped here for cleanup
                Err(ToolError::ExecutionFailed {
                    message: format!(
                        "Log analysis failed for resource '{}' with query '{}': {}",
                        resource_id, query_description, e
                    ),
                })
            }
        }
        // Agent is automatically dropped here, ensuring memory cleanup
    }
}

/// Helper function to convert LogAnalysisEvent to Bridge AgentResponse
fn create_bridge_response(agent_id: String, event: LogAnalysisEvent) -> BridgeAgentResponse {
    BridgeAgentResponse::LogAnalysisEvent { agent_id, event }
}

/// Callback handler for log analysis events
///
/// This handler captures raw callback events from the log analysis agent and forwards
/// them to the Bridge UI as LogAnalysisEvent messages for display in the message tree.
#[derive(Debug)]
pub struct LogEntriesCallbackHandler {
    agent_id: String,
    sender: Option<mpsc::Sender<BridgeAgentResponse>>,
}

impl LogEntriesCallbackHandler {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            sender: None,
        }
    }

    pub fn with_sender(agent_id: String, sender: mpsc::Sender<BridgeAgentResponse>) -> Self {
        Self {
            agent_id,
            sender: Some(sender),
        }
    }

    fn send_event(&self, event: LogAnalysisEvent) {
        if let Some(ref sender) = self.sender {
            let response = create_bridge_response(self.agent_id.clone(), event);
            if let Err(e) = sender.send(response) {
                warn!("Failed to send log analysis event to Bridge: {}", e);
            }
        } else {
            debug!("📊 Log analysis event (no sender): {:?}", event);
        }
    }
}

#[async_trait]
impl CallbackHandler for LogEntriesCallbackHandler {
    /// Handle streaming content events (not used for log analysis event bubbling)
    async fn on_content(&self, _content: &str, _is_complete: bool) -> Result<(), CallbackError> {
        // Log analysis event bubbling doesn't need content streaming
        Ok(())
    }

    /// Handle tool execution events
    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        let log_event = match event {
            ToolEvent::Started { name, .. } => {
                debug!("🔧 Log analysis tool started: {}", name);
                LogAnalysisEvent::ToolStart {
                    timestamp: Utc::now(),
                    tool_name: name,
                }
            }
            ToolEvent::Completed { name, .. } => {
                debug!("✅ Log analysis tool completed: {}", name);
                LogAnalysisEvent::ToolComplete {
                    timestamp: Utc::now(),
                    tool_name: name,
                    success: true,
                }
            }
            ToolEvent::Failed { name, error, .. } => {
                debug!("❌ Log analysis tool failed: {} - {}", name, error);
                LogAnalysisEvent::ToolComplete {
                    timestamp: Utc::now(),
                    tool_name: name,
                    success: false,
                }
            }
        };

        self.send_event(log_event);
        Ok(())
    }

    /// Handle execution completion events
    async fn on_complete(
        &self,
        _result: &stood::agent::result::AgentResult,
    ) -> Result<(), CallbackError> {
        let log_event = LogAnalysisEvent::EventLoopComplete {
            timestamp: Utc::now(),
        };

        self.send_event(log_event);
        Ok(())
    }

    /// Handle error events
    async fn on_error(&self, error: &stood::StoodError) -> Result<(), CallbackError> {
        let log_event = LogAnalysisEvent::Error {
            timestamp: Utc::now(),
            error: error.to_string(),
        };

        self.send_event(log_event);
        Ok(())
    }

    /// Handle all callback events including ModelStart
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        let log_event = match event {
            CallbackEvent::ModelStart { messages, .. } => {
                debug!(
                    "🚀 Log analysis model started with {} messages",
                    messages.len()
                );
                LogAnalysisEvent::ModelStart {
                    timestamp: Utc::now(),
                    messages_count: messages.len(),
                }
            }
            // For other events, delegate to the specific handlers
            _ => {
                // Handle other events through the default trait implementations
                match event {
                    CallbackEvent::ToolStart { tool_name, .. } => {
                        return self
                            .on_tool(ToolEvent::Started {
                                name: tool_name,
                                input: serde_json::Value::Null,
                            })
                            .await;
                    }
                    CallbackEvent::ToolComplete {
                        tool_name, error, ..
                    } => {
                        if let Some(err) = error {
                            return self
                                .on_tool(ToolEvent::Failed {
                                    name: tool_name,
                                    error: err,
                                    duration: std::time::Duration::ZERO,
                                })
                                .await;
                        } else {
                            return self
                                .on_tool(ToolEvent::Completed {
                                    name: tool_name,
                                    output: None,
                                    duration: std::time::Duration::ZERO,
                                })
                                .await;
                        }
                    }
                    CallbackEvent::EventLoopComplete { result, .. } => {
                        // Convert to AgentResult and call on_complete
                        let agent_result = stood::agent::result::AgentResult::from(
                            result,
                            std::time::Duration::ZERO,
                        );
                        return self.on_complete(&agent_result).await;
                    }
                    CallbackEvent::Error { error, .. } => {
                        return self.on_error(&error).await;
                    }
                    _ => return Ok(()), // Ignore other events
                }
            }
        };

        self.send_event(log_event);
        Ok(())
    }
}

