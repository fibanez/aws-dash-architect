#![warn(clippy::all, rust_2018_idioms)]

//! Worker Progress Callback Handler
//!
//! This module provides a callback handler that forwards tool events from
//! worker agents to the UI event system for inline progress display.
//!
//! ## How it works
//!
//! 1. WorkerProgressCallbackHandler is created with worker_id and parent_id
//! 2. It is registered with the stood agent via `.with_callback_handler()`
//! 3. When tools execute, stood calls `on_tool()` with ToolEvent
//! 4. The handler forwards these as AgentUIEvent to the global channel
//! 5. AgentManagerWindow processes these events for inline display

use crate::app::agent_framework::{send_ui_event, AgentId, AgentUIEvent};
use crate::perf_checkpoint;
use async_trait::async_trait;
use stood::agent::callbacks::{CallbackError, CallbackEvent, CallbackHandler, ToolEvent};

/// Callback handler that forwards worker tool events to the UI
///
/// This handler is attached to TaskWorker agents to capture their tool
/// execution progress and forward it to the UI for inline display.
pub struct WorkerProgressCallbackHandler {
    /// ID of the worker agent
    worker_id: AgentId,
    /// ID of the parent (manager) agent
    parent_id: AgentId,
}

impl WorkerProgressCallbackHandler {
    /// Create a new worker progress callback handler
    ///
    /// # Arguments
    /// * `worker_id` - The ID of the worker agent this handler monitors
    /// * `parent_id` - The ID of the parent (manager) agent
    pub fn new(worker_id: AgentId, parent_id: AgentId) -> Self {
        Self {
            worker_id,
            parent_id,
        }
    }

    /// Extract intent from tool input
    ///
    /// Tries to find a human-readable description in the tool input.
    /// Falls back to tool name if no intent is found.
    fn extract_intent(tool_name: &str, input: &serde_json::Value) -> String {
        // Try to find intent in common field names
        if let Some(intent) = input.get("intent").and_then(|v| v.as_str()) {
            return intent.to_string();
        }

        if let Some(description) = input.get("description").and_then(|v| v.as_str()) {
            return description.to_string();
        }

        // For write_file and edit_file, use the path
        if tool_name == "write_file" || tool_name == "edit_file" {
            if let Some(path) = input.get("path").and_then(|v| v.as_str()) {
                return format!("{} {}",
                    if tool_name == "write_file" { "Writing" } else { "Editing" },
                    path
                );
            }
        }

        // For read_file, list_files, delete_file
        if tool_name == "read_file" || tool_name == "delete_file" {
            if let Some(path) = input.get("path").and_then(|v| v.as_str()) {
                return format!("{} {}",
                    match tool_name {
                        "read_file" => "Reading",
                        "delete_file" => "Deleting",
                        _ => "Processing",
                    },
                    path
                );
            }
        }

        // For execute_javascript, try to get a summary from the code
        if tool_name == "execute_javascript" {
            if let Some(code) = input.get("code").and_then(|v| v.as_str()) {
                // Try to find a comment at the top of the code
                if let Some(first_line) = code.lines().next() {
                    if first_line.trim_start().starts_with("//") {
                        return first_line.trim_start().trim_start_matches("//").trim().to_string();
                    }
                }
                return "Executing JavaScript".to_string();
            }
        }

        // Fallback: use tool name with better formatting
        tool_name.replace('_', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[async_trait]
impl CallbackHandler for WorkerProgressCallbackHandler {
    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        perf_checkpoint!(
            "worker.on_tool_CALLED",
            &format!("worker={} event={:?}", self.worker_id, event)
        );

        match event {
            ToolEvent::Started { name, input } => {
                // Extract intent from tool input if available
                let intent = Self::extract_intent(&name, &input);

                perf_checkpoint!(
                    "worker.tool_started",
                    &format!("worker={} tool={} intent={}", self.worker_id, name, intent)
                );

                // Send UI event for tool start with intent
                let send_result = send_ui_event(AgentUIEvent::worker_tool_started(
                    self.worker_id,
                    self.parent_id,
                    name.clone(),
                    intent.clone(),
                ));

                if let Err(e) = send_result {
                    perf_checkpoint!(
                        "worker.tool_started.send_failed",
                        &format!("worker={} error={}", self.worker_id, e)
                    );
                } else {
                    perf_checkpoint!(
                        "worker.tool_started.sent",
                        &format!("worker={} tool={}", self.worker_id, name)
                    );
                }
            }
            ToolEvent::Completed { name, .. } => {
                perf_checkpoint!(
                    "worker.tool_completed",
                    &format!("worker={} tool={} success=true", self.worker_id, name)
                );

                // Send UI event for tool completion
                // Tokens will be added via separate ModelComplete event
                let send_result = send_ui_event(AgentUIEvent::worker_tool_completed(
                    self.worker_id,
                    self.parent_id,
                    name.clone(),
                    true,
                    None, // tokens added separately via ModelComplete
                ));

                if let Err(e) = send_result {
                    perf_checkpoint!(
                        "worker.tool_completed.send_failed",
                        &format!("worker={} error={}", self.worker_id, e)
                    );
                } else {
                    perf_checkpoint!(
                        "worker.tool_completed.sent",
                        &format!("worker={} tool={}", self.worker_id, name)
                    );
                }
            }
            ToolEvent::Failed { name, error, .. } => {
                perf_checkpoint!(
                    "worker.tool_failed",
                    &format!("worker={} tool={} error={}", self.worker_id, name, error)
                );

                // Send UI event for tool failure
                let send_result = send_ui_event(AgentUIEvent::worker_tool_completed(
                    self.worker_id,
                    self.parent_id,
                    name.clone(),
                    false,
                    None, // No tokens for failed tools
                ));

                if let Err(e) = send_result {
                    perf_checkpoint!(
                        "worker.tool_failed.send_failed",
                        &format!("worker={} error={}", self.worker_id, e)
                    );
                } else {
                    perf_checkpoint!(
                        "worker.tool_failed.sent",
                        &format!("worker={} tool={}", self.worker_id, name)
                    );
                }
            }
        }
        Ok(())
    }

    /// Handle all callback events
    ///
    /// This override extends the default routing to also capture ModelComplete events
    /// for token tracking. All other events (tool start/complete) are handled by the
    /// default implementation which routes to on_tool().
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        // Handle ModelComplete for token tracking
        if let CallbackEvent::ModelComplete {
            tokens: Some(usage),
            ..
        } = &event
        {
            perf_checkpoint!(
                "worker.model_complete",
                &format!("worker={} in={} out={} total={}",
                    self.worker_id, usage.input_tokens, usage.output_tokens, usage.total_tokens)
            );

            // Send UI event for token update
            let send_result = send_ui_event(AgentUIEvent::worker_tokens_updated(
                self.worker_id,
                self.parent_id,
                usage.input_tokens,
                usage.output_tokens,
                usage.total_tokens,
            ));

            if let Err(e) = send_result {
                perf_checkpoint!(
                    "worker.model_complete.send_failed",
                    &format!("worker={} error={}", self.worker_id, e)
                );
            } else {
                perf_checkpoint!(
                    "worker.model_complete.sent",
                    &format!("worker={} tokens={}", self.worker_id, usage.total_tokens)
                );
            }
        }

        // Call default handler to route tool events to on_tool()
        // This is critical - without this, tool events are lost!
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
    fn test_worker_progress_handler_creation() {
        let worker_id = AgentId::new();
        let parent_id = AgentId::new();
        let handler = WorkerProgressCallbackHandler::new(worker_id, parent_id);

        assert_eq!(handler.worker_id, worker_id);
        assert_eq!(handler.parent_id, parent_id);
    }

    #[tokio::test]
    async fn test_on_tool_started() {
        use crate::app::agent_framework::{get_ui_event_receiver, init_ui_event_channel};

        init_ui_event_channel();

        let worker_id = AgentId::new();
        let parent_id = AgentId::new();
        let handler = WorkerProgressCallbackHandler::new(worker_id, parent_id);

        // Trigger tool started event
        let event = ToolEvent::Started {
            name: "execute_javascript".to_string(),
            input: serde_json::json!({"code": "test"}),
        };

        handler.on_tool(event).await.unwrap();

        // Verify UI event was sent
        let receiver = get_ui_event_receiver();
        let received = receiver.lock().unwrap().try_recv();
        assert!(received.is_ok());

        if let Ok(AgentUIEvent::WorkerToolStarted {
            worker_id: w,
            parent_id: p,
            tool_name,
        }) = received
        {
            assert_eq!(w, worker_id);
            assert_eq!(p, parent_id);
            assert_eq!(tool_name, "execute_javascript");
        } else {
            panic!("Expected WorkerToolStarted event");
        }
    }

    #[tokio::test]
    async fn test_on_tool_completed() {
        use crate::app::agent_framework::{get_ui_event_receiver, init_ui_event_channel};

        init_ui_event_channel();

        let worker_id = AgentId::new();
        let parent_id = AgentId::new();
        let handler = WorkerProgressCallbackHandler::new(worker_id, parent_id);

        // Trigger tool completed event
        let event = ToolEvent::Completed {
            name: "execute_javascript".to_string(),
            output: Some(serde_json::json!({"result": "success"})),
            duration: std::time::Duration::from_millis(100),
        };

        handler.on_tool(event).await.unwrap();

        // Verify UI event was sent
        let receiver = get_ui_event_receiver();
        let received = receiver.lock().unwrap().try_recv();
        assert!(received.is_ok());

        if let Ok(AgentUIEvent::WorkerToolCompleted {
            worker_id: w,
            parent_id: p,
            tool_name,
            success,
        }) = received
        {
            assert_eq!(w, worker_id);
            assert_eq!(p, parent_id);
            assert_eq!(tool_name, "execute_javascript");
            assert!(success);
        } else {
            panic!("Expected WorkerToolCompleted event");
        }
    }

    #[tokio::test]
    async fn test_on_tool_failed() {
        use crate::app::agent_framework::{get_ui_event_receiver, init_ui_event_channel};

        init_ui_event_channel();

        let worker_id = AgentId::new();
        let parent_id = AgentId::new();
        let handler = WorkerProgressCallbackHandler::new(worker_id, parent_id);

        // Trigger tool failed event
        let event = ToolEvent::Failed {
            name: "execute_javascript".to_string(),
            error: "Test error".to_string(),
            duration: std::time::Duration::from_millis(50),
        };

        handler.on_tool(event).await.unwrap();

        // Verify UI event was sent
        let receiver = get_ui_event_receiver();
        let received = receiver.lock().unwrap().try_recv();
        assert!(received.is_ok());

        if let Ok(AgentUIEvent::WorkerToolCompleted {
            worker_id: w,
            parent_id: p,
            tool_name,
            success,
        }) = received
        {
            assert_eq!(w, worker_id);
            assert_eq!(p, parent_id);
            assert_eq!(tool_name, "execute_javascript");
            assert!(!success);
        } else {
            panic!("Expected WorkerToolCompleted event with success=false");
        }
    }
}
