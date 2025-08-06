//! Universal Sub-Agent Callback Handler
//!
//! This handler captures events from all specialized agents (log analyzer, resource auditor,
//! security scanner) and forwards them to the Bridge UI with user-friendly language that
//! focuses on tasks and actions rather than technical "agent" terminology.

use crate::app::dashui::control_bridge_window::{
    AgentResponse as BridgeAgentResponse, SubAgentEvent,
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json;
use std::sync::mpsc;
use stood::agent::callbacks::{CallbackError, CallbackEvent, CallbackHandler, ToolEvent};
use tracing::{debug, warn};

/// Universal callback handler for all specialized agent types
///
/// This handler captures events from any specialized agent and converts them to
/// user-friendly SubAgentEvent messages with task-focused language.
#[derive(Debug)]
pub struct SubAgentCallbackHandler {
    agent_id: String,
    agent_type: String,
    sender: Option<mpsc::Sender<BridgeAgentResponse>>,
}

impl SubAgentCallbackHandler {
    /// Create a new handler without Bridge communication (standalone mode)
    pub fn new(agent_id: String, agent_type: String) -> Self {
        Self {
            agent_id,
            agent_type,
            sender: None,
        }
    }

    /// Create a new handler with Bridge communication
    pub fn with_sender(
        agent_id: String,
        agent_type: String,
        sender: mpsc::Sender<BridgeAgentResponse>,
    ) -> Self {
        Self {
            agent_id,
            agent_type,
            sender: Some(sender),
        }
    }

    /// Send event to Bridge UI
    fn send_event(&self, event: SubAgentEvent) {
        if let Some(ref sender) = self.sender {
            let response = BridgeAgentResponse::SubAgentEvent {
                agent_id: self.agent_id.clone(),
                agent_type: self.agent_type.clone(),
                event,
            };
            if let Err(e) = sender.send(response) {
                warn!("Failed to send sub-agent event to Bridge UI: {}", e);
            }
        } else {
            debug!("📊 Sub-agent event (no Bridge sender): {:?}", event);
        }
    }

    /// Convert tool name to user-friendly action description
    fn get_user_friendly_action(&self, tool_name: &str) -> String {
        match tool_name {
            // CloudWatch and logging tools
            "aws_describe_log_groups" => "Discovering log groups".to_string(),
            "aws_get_log_events" => "Retrieving log entries".to_string(),

            // Resource management tools
            "aws_list_resources" => "Listing AWS resources".to_string(),
            "aws_describe_resource" => "Analyzing resource details".to_string(),
            "aws_get_resource_tags" => "Checking resource tags".to_string(),

            // Security assessment tools
            "aws_security_groups" => "Analyzing security groups".to_string(),
            "aws_iam_policies" => "Reviewing IAM policies".to_string(),
            "aws_vpc_config" => "Examining VPC configuration".to_string(),

            // Task management tools
            "todo_write" => "Planning analysis steps".to_string(),
            "todo_read" => "Checking progress".to_string(),

            // Context tools
            "aws_find_account" => "Identifying AWS account".to_string(),
            "aws_find_region" => "Setting AWS region".to_string(),

            // Generic fallback
            _ => format!("Processing {}", tool_name.replace("_", " ")),
        }
    }

    /// Get task description for processing started events
    #[allow(dead_code)]
    fn get_task_description(&self, _task_description: &str) -> String {
        let task_verb = match self.agent_type.as_str() {
            "aws-log-analyzer" => "🔍 Analyzing CloudWatch logs",
            "aws-resource-auditor" => "📊 Auditing AWS resources",
            "aws-security-scanner" => "🔒 Scanning security posture",
            _ => "⚙️ Processing request",
        };

        format!("{}: {}", task_verb, _task_description)
    }
}

#[async_trait]
impl CallbackHandler for SubAgentCallbackHandler {
    /// Handle streaming content events (not used for sub-agent event bubbling)
    async fn on_content(&self, _content: &str, _is_complete: bool) -> Result<(), CallbackError> {
        // Sub-agent event bubbling doesn't need content streaming
        Ok(())
    }

    /// Handle tool execution events with user-friendly language
    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        let sub_agent_event = match event {
            ToolEvent::Started { name, input } => {
                debug!("🔧 Sub-agent tool started: {}", name);

                // Create input summary for technical details (child node)
                let input_summary = if let serde_json::Value::Object(obj) = input {
                    if obj.is_empty() {
                        None
                    } else {
                        Some(format!("Parameters: {} fields", obj.len()))
                    }
                } else {
                    None
                };

                SubAgentEvent::ToolStarted {
                    timestamp: Utc::now(),
                    tool_name: self.get_user_friendly_action(&name),
                    input_summary,
                }
            }
            ToolEvent::Completed { name, output, .. } => {
                debug!("✅ Sub-agent tool completed: {}", name);

                // Create output summary for technical details (child node)
                let output_summary = if let Some(output_data) = output {
                    match output_data {
                        serde_json::Value::String(s) => {
                            if s.len() > 100 {
                                Some(format!("Response: {} characters", s.len()))
                            } else {
                                Some(format!("Response: {}", s))
                            }
                        }
                        serde_json::Value::Object(obj) => {
                            Some(format!("Response: {} fields", obj.len()))
                        }
                        serde_json::Value::Array(arr) => {
                            Some(format!("Response: {} items", arr.len()))
                        }
                        _ => Some("Response received".to_string()),
                    }
                } else {
                    Some("Completed successfully".to_string())
                };

                SubAgentEvent::ToolCompleted {
                    timestamp: Utc::now(),
                    tool_name: self.get_user_friendly_action(&name),
                    success: true,
                    output_summary,
                }
            }
            ToolEvent::Failed { name, error, .. } => {
                debug!("❌ Sub-agent tool failed: {} - {}", name, error);
                SubAgentEvent::ToolCompleted {
                    timestamp: Utc::now(),
                    tool_name: self.get_user_friendly_action(&name),
                    success: false,
                    output_summary: Some(format!("Error: {}", error)),
                }
            }
        };

        self.send_event(sub_agent_event);
        Ok(())
    }

    /// Handle execution completion events
    async fn on_complete(
        &self,
        _result: &stood::agent::result::AgentResult,
    ) -> Result<(), CallbackError> {
        let sub_agent_event = SubAgentEvent::TaskComplete {
            timestamp: Utc::now(),
        };

        self.send_event(sub_agent_event);
        Ok(())
    }

    /// Handle error events
    async fn on_error(&self, error: &stood::StoodError) -> Result<(), CallbackError> {
        let sub_agent_event = SubAgentEvent::Error {
            timestamp: Utc::now(),
            error: error.to_string(),
        };

        self.send_event(sub_agent_event);
        Ok(())
    }

    /// Handle all callback events including ModelStart and other specialized events
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        let sub_agent_event = match event {
            CallbackEvent::ModelStart { messages, .. } => {
                debug!(
                    "🚀 Sub-agent model started with {} messages",
                    messages.len()
                );

                // Include raw JSON for technical details if available
                let raw_json = if messages.len() < 5 {
                    // Only include raw JSON for small message counts to avoid UI clutter
                    serde_json::to_string_pretty(&messages).ok().map(|json| {
                        if json.len() > 1000 {
                            format!("{}...", &json[..1000]) // Truncate long JSON
                        } else {
                            json
                        }
                    })
                } else {
                    None
                };

                SubAgentEvent::ModelRequest {
                    timestamp: Utc::now(),
                    messages_count: messages.len(),
                    raw_json,
                }
            }
            CallbackEvent::ModelComplete { response, .. } => {
                debug!("✅ Sub-agent model completed");

                let response_length = response.len();

                SubAgentEvent::ModelResponse {
                    timestamp: Utc::now(),
                    response_length,
                    tokens_used: None, // Tokens are not available in this event
                }
            }
            // For other events, delegate to the specific handlers
            _ => {
                // Handle other events through the default trait implementations
                match event {
                    CallbackEvent::ToolStart {
                        tool_name, input, ..
                    } => {
                        return self
                            .on_tool(ToolEvent::Started {
                                name: tool_name,
                                input,
                            })
                            .await;
                    }
                    CallbackEvent::ToolComplete {
                        tool_name,
                        output,
                        error,
                        ..
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
                                    output,
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

        self.send_event(sub_agent_event);
        Ok(())
    }
}
