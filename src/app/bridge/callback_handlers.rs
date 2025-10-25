//! Callback handlers for Bridge Agent events
//!
//! Handles streaming, tool calls, debug events, and JSON capture for the Bridge Agent system.

#![warn(clippy::all, rust_2018_idioms)]

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use stood::agent::callbacks::{CallbackError, CallbackEvent, CallbackHandler, ToolEvent};
use tracing::{debug, error};

use crate::app::dashui::control_bridge_window::{
    AgentResponse, JsonDebugData, JsonDebugType, Message, MessageRole,
};

// ============================================================================
// JSON CAPTURE HANDLER
// ============================================================================

/// JSON capture handler that captures model request/response JSON for debugging
pub struct JsonCaptureHandler {
    sender: mpsc::Sender<AgentResponse>,
}

impl JsonCaptureHandler {
    pub fn new(sender: mpsc::Sender<AgentResponse>) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl CallbackHandler for JsonCaptureHandler {
    /// Handle streaming events - not used for JSON capture but required for trait
    async fn on_content(&self, _content: &str, _is_complete: bool) -> Result<(), CallbackError> {
        // JSON capture doesn't need content streaming events
        Ok(())
    }

    /// Handle tool events - not used for JSON capture but required for trait
    async fn on_tool(&self, _event: ToolEvent) -> Result<(), CallbackError> {
        // JSON capture doesn't need tool events
        Ok(())
    }

    /// Handle completion events - not used for JSON capture but required for trait
    async fn on_complete(
        &self,
        _result: &stood::agent::result::AgentResult,
    ) -> Result<(), CallbackError> {
        // JSON capture doesn't need completion events
        Ok(())
    }

    /// Handle error events - not used for JSON capture but required for trait
    async fn on_error(&self, _error: &stood::StoodError) -> Result<(), CallbackError> {
        // JSON capture doesn't need error events
        Ok(())
    }

    /// Main event handler for JSON capture - this captures model request/response JSON
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        match event {
            CallbackEvent::ModelStart {
                provider,
                model_id,
                messages,
                tools_available,
                raw_request_json: _,
            } => {
                debug!("📤 Capturing model request JSON");

                // Create JSON representation of the request
                let request_json = serde_json::json!({
                    "type": "model_request",
                    "provider": format!("{:?}", provider),
                    "model_id": model_id,
                    "timestamp": Utc::now().to_rfc3339(),
                    "messages": messages,
                    "tools_available": tools_available,
                });

                let json_data = JsonDebugData {
                    json_type: JsonDebugType::Request,
                    json_content: serde_json::to_string_pretty(&request_json)
                        .unwrap_or_else(|_| "Error serializing request JSON".to_string()),
                    raw_json_content: None, // JsonCaptureHandler doesn't have access to raw JSON
                    timestamp: Utc::now(),
                };

                // Send to UI thread
                if let Err(e) = self.sender.send(AgentResponse::JsonDebug(json_data)) {
                    error!("Failed to send JSON request data to UI: {}", e);
                }
            }
            CallbackEvent::ModelComplete {
                response,
                stop_reason,
                duration,
                tokens,
                raw_response_data: _,
            } => {
                debug!("📥 Capturing model response JSON");

                // Create JSON representation of the response
                let response_json = serde_json::json!({
                    "type": "model_response",
                    "timestamp": Utc::now().to_rfc3339(),
                    "response": response,
                    "stop_reason": format!("{:?}", stop_reason),
                    "duration_ms": duration.as_millis(),
                    "tokens": tokens.map(|t| serde_json::json!({
                        "input_tokens": t.input_tokens,
                        "output_tokens": t.output_tokens,
                        "total_tokens": t.total_tokens,
                    })),
                });

                let json_data = JsonDebugData {
                    json_type: JsonDebugType::Response,
                    json_content: serde_json::to_string_pretty(&response_json)
                        .unwrap_or_else(|_| "Error serializing response JSON".to_string()),
                    raw_json_content: None, // JsonCaptureHandler doesn't have access to raw JSON
                    timestamp: Utc::now(),
                };

                // Send to UI thread
                if let Err(e) = self.sender.send(AgentResponse::JsonDebug(json_data)) {
                    error!("Failed to send JSON response data to UI: {}", e);
                }
            }
            _ => {
                // Ignore other events - we only care about model interactions
            }
        }
        Ok(())
    }
}

// ============================================================================
// BRIDGE TOOL CALLBACK HANDLER
// ============================================================================

/// Bridge Tool Callback Handler - Creates tree structure for tool calls
///
/// This handler creates "Calling tool" nodes when tools start and adds
/// child nodes with tool responses when tools complete.
#[derive(Debug, Clone)]
pub struct BridgeToolCallbackHandler {
    sender: mpsc::Sender<AgentResponse>,
    active_tool_nodes: Arc<Mutex<HashMap<String, String>>>, // tool_use_id -> parent_message_id
}

impl BridgeToolCallbackHandler {
    pub fn new(sender: mpsc::Sender<AgentResponse>) -> Self {
        Self {
            sender,
            active_tool_nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Map tool names to user-friendly actions
    fn get_user_friendly_action(tool_name: &str) -> &'static str {
        match tool_name {
            "aws_list_resources" => "List",
            "aws_describe_resource" => "Describe",
            "aws_find_account" => "Find Account",
            "aws_find_region" => "Find Region",
            "create_task" => "Task",
            "search_logs" => "Search Logs",
            "analyze_logs" => "Analyze",
            _ => "Tool", // Generic fallback
        }
    }
}

#[async_trait]
impl CallbackHandler for BridgeToolCallbackHandler {
    /// Handle streaming content - not needed for tool callbacks
    async fn on_content(&self, _content: &str, _is_complete: bool) -> Result<(), CallbackError> {
        Ok(())
    }

    /// Handle tool execution events to create tree structure
    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        match event {
            ToolEvent::Started { name, input } => {
                // Create "Calling tool" parent node
                let tool_node_id = format!("tool_{}_{}", name, Utc::now().timestamp_millis());

                // Get friendly name for display
                let friendly_name = Self::get_user_friendly_action(&name);

                // For create_task, show the task description prominently
                let (content, summary) = if name == "create_task" {
                    let task_description = input
                        .get("task_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Task execution");

                    // Show the task description as the main content
                    let content = format!("🎯 {}", task_description);
                    let summary = format!(
                        "{}: {}",
                        friendly_name,
                        task_description.chars().take(50).collect::<String>()
                            + if task_description.len() > 50 {
                                "..."
                            } else {
                                ""
                            }
                    );

                    (content, summary)
                } else {
                    // For other tools, show friendly name
                    let content = format!("🔧 {}", friendly_name);
                    let summary = friendly_name.to_string();
                    (content, summary)
                };

                // Create nested message with JSON input parameters
                let json_input_message = Message {
                    id: format!("{}_input", tool_node_id),
                    role: MessageRole::JsonRequest,
                    content: serde_json::to_string_pretty(&input)
                        .unwrap_or_else(|_| format!("{:?}", input)),
                    timestamp: Utc::now(),
                    summary: Some("Input Parameters".to_string()),
                    debug_info: None,
                    nested_messages: Vec::new(),
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                let parent_message = Message {
                    id: tool_node_id.clone(),
                    role: MessageRole::System,
                    content,
                    timestamp: Utc::now(),
                    summary: Some(summary),
                    debug_info: None,
                    nested_messages: vec![json_input_message],
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                // Store the parent node ID for when the tool completes
                // Note: We use tool name + timestamp as a unique ID since tool_use_id may not be available
                let tool_key = format!(
                    "{}_{}",
                    name,
                    parent_message.timestamp.timestamp_millis()
                );
                self.active_tool_nodes
                    .lock()
                    .unwrap()
                    .insert(tool_key, tool_node_id.clone());

                // Send parent node to UI via ToolCallStart message
                let response = AgentResponse::ToolCallStart { parent_message };

                if let Err(e) = self.sender.send(response) {
                    error!("Failed to send tool start message to GUI: {}", e);
                }
            }

            ToolEvent::Completed {
                name,
                output,
                duration,
            } => {
                // Get friendly name
                let friendly_name = Self::get_user_friendly_action(&name);

                // Create nested message with JSON output
                let json_output_message = Message {
                    id: format!("tool_output_{}_{}", name, Utc::now().timestamp_millis()),
                    role: MessageRole::JsonResponse,
                    content: match &output {
                        Some(value) => serde_json::to_string_pretty(value)
                            .unwrap_or_else(|_| format!("{:?}", value)),
                        None => "null".to_string(),
                    },
                    timestamp: Utc::now(),
                    summary: Some("Output Result".to_string()),
                    debug_info: None,
                    nested_messages: Vec::new(),
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                let child_message = Message {
                    id: format!("tool_response_{}_{}", name, Utc::now().timestamp_millis()),
                    role: MessageRole::Assistant,
                    content: format!(
                        "✅ {} completed ({:.2}s)",
                        friendly_name,
                        duration.as_secs_f64()
                    ),
                    timestamp: Utc::now(),
                    summary: Some(format!("{} Result", friendly_name)),
                    debug_info: None,
                    nested_messages: vec![json_output_message],
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                // Find the most recent tool node with this name (simple matching)
                let parent_node_id = {
                    let active_nodes = self.active_tool_nodes.lock().unwrap();
                    active_nodes
                        .iter()
                        .filter(|(key, _)| key.starts_with(&format!("{}_", name)))
                        .max_by_key(|(key, _)| {
                            key.split('_')
                                .next_back()
                                .unwrap_or("0")
                                .parse::<i64>()
                                .unwrap_or(0)
                        })
                        .map(|(_, id)| id.clone())
                };

                if let Some(parent_id) = parent_node_id {
                    // Send child node to UI
                    let response = AgentResponse::ToolCallComplete {
                        parent_message_id: parent_id.clone(),
                        child_message,
                    };

                    if let Err(e) = self.sender.send(response) {
                        error!("Failed to send tool complete message to GUI: {}", e);
                    }

                    // Clean up the mapping
                    self.active_tool_nodes
                        .lock()
                        .unwrap()
                        .retain(|_, v| *v != parent_id);
                }
            }

            ToolEvent::Failed {
                name,
                error,
                duration,
            } => {
                // Get friendly name
                let friendly_name = Self::get_user_friendly_action(&name);

                // Create child error node
                let child_message = Message {
                    id: format!("tool_error_{}_{}", name, Utc::now().timestamp_millis()),
                    role: MessageRole::Debug,
                    content: format!(
                        "❌ {} failed ({:.2}s):\n{}",
                        friendly_name,
                        duration.as_secs_f64(),
                        error
                    ),
                    timestamp: Utc::now(),
                    summary: Some(format!("{} Error", friendly_name)),
                    debug_info: None,
                    nested_messages: Vec::new(),
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                // Find the most recent tool node with this name
                let parent_node_id = {
                    let active_nodes = self.active_tool_nodes.lock().unwrap();
                    active_nodes
                        .iter()
                        .filter(|(key, _)| key.starts_with(&format!("{}_", name)))
                        .max_by_key(|(key, _)| {
                            key.split('_')
                                .next_back()
                                .unwrap_or("0")
                                .parse::<i64>()
                                .unwrap_or(0)
                        })
                        .map(|(_, id)| id.clone())
                };

                if let Some(parent_id) = parent_node_id {
                    let response = AgentResponse::ToolCallComplete {
                        parent_message_id: parent_id.clone(),
                        child_message,
                    };

                    if let Err(e) = self.sender.send(response) {
                        error!("Failed to send tool error message to GUI: {}", e);
                    }

                    // Clean up the mapping
                    self.active_tool_nodes
                        .lock()
                        .unwrap()
                        .retain(|_, v| *v != parent_id);
                }
            }
        }
        Ok(())
    }

    /// Handle completion events - not needed for tool callbacks
    async fn on_complete(
        &self,
        _result: &stood::agent::result::AgentResult,
    ) -> Result<(), CallbackError> {
        Ok(())
    }

    /// Handle error events - not needed for tool callbacks
    async fn on_error(&self, _error: &stood::StoodError) -> Result<(), CallbackError> {
        Ok(())
    }

    /// Handle all callback events
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        match event {
            CallbackEvent::ToolStart {
                tool_name, input, ..
            } => {
                self.on_tool(ToolEvent::Started {
                    name: tool_name,
                    input,
                })
                .await
            }
            CallbackEvent::ToolComplete {
                tool_name,
                output,
                error,
                duration,
                ..
            } => {
                if let Some(err) = error {
                    self.on_tool(ToolEvent::Failed {
                        name: tool_name,
                        error: err,
                        duration,
                    })
                    .await
                } else {
                    self.on_tool(ToolEvent::Completed {
                        name: tool_name,
                        output,
                        duration,
                    })
                    .await
                }
            }
            _ => Ok(()), // Ignore other events
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_capture_handler_creation() {
        let (sender, _receiver) = mpsc::channel();
        let _handler = JsonCaptureHandler::new(sender);
        // Handler should be created successfully
    }

    #[test]
    fn test_bridge_tool_callback_handler_creation() {
        let (sender, _receiver) = mpsc::channel();
        let _handler = BridgeToolCallbackHandler::new(sender);
        // Handler should be created successfully
    }

    #[test]
    fn test_user_friendly_action_mapping() {
        assert_eq!(
            BridgeToolCallbackHandler::get_user_friendly_action("create_task"),
            "Task"
        );
        assert_eq!(
            BridgeToolCallbackHandler::get_user_friendly_action("aws_find_account"),
            "Find Account"
        );
        assert_eq!(
            BridgeToolCallbackHandler::get_user_friendly_action("aws_list_resources"),
            "List"
        );
        assert_eq!(
            BridgeToolCallbackHandler::get_user_friendly_action("unknown_tool"),
            "Tool"
        );
    }
}
