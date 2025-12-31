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
}

#[async_trait]
impl CallbackHandler for WorkerProgressCallbackHandler {
    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        match event {
            ToolEvent::Started { name, .. } => {
                tracing::debug!(
                    target: "agent::worker_progress",
                    worker_id = %self.worker_id,
                    parent_id = %self.parent_id,
                    tool_name = %name,
                    "Worker tool started"
                );

                // Send UI event for tool start
                let _ = send_ui_event(AgentUIEvent::worker_tool_started(
                    self.worker_id,
                    self.parent_id,
                    name,
                ));
            }
            ToolEvent::Completed { name, .. } => {
                tracing::debug!(
                    target: "agent::worker_progress",
                    worker_id = %self.worker_id,
                    parent_id = %self.parent_id,
                    tool_name = %name,
                    "Worker tool completed successfully"
                );

                // Send UI event for tool completion
                let _ = send_ui_event(AgentUIEvent::worker_tool_completed(
                    self.worker_id,
                    self.parent_id,
                    name,
                    true,
                ));
            }
            ToolEvent::Failed { name, error, .. } => {
                tracing::debug!(
                    target: "agent::worker_progress",
                    worker_id = %self.worker_id,
                    parent_id = %self.parent_id,
                    tool_name = %name,
                    error = %error,
                    "Worker tool failed"
                );

                // Send UI event for tool failure
                let _ = send_ui_event(AgentUIEvent::worker_tool_completed(
                    self.worker_id,
                    self.parent_id,
                    name,
                    false,
                ));
            }
        }
        Ok(())
    }

    /// Handle all callback events, including ModelComplete for token usage
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        // Only handle ModelComplete with token usage - tool events handled by on_tool()
        if let CallbackEvent::ModelComplete {
            tokens: Some(usage),
            ..
        } = event
        {
            tracing::debug!(
                target: "agent::worker_progress",
                worker_id = %self.worker_id,
                parent_id = %self.parent_id,
                input_tokens = usage.input_tokens,
                output_tokens = usage.output_tokens,
                total_tokens = usage.total_tokens,
                "Worker model call completed with token usage"
            );

            // Send UI event for token update
            let _ = send_ui_event(AgentUIEvent::worker_tokens_updated(
                self.worker_id,
                self.parent_id,
                usage.input_tokens,
                usage.output_tokens,
                usage.total_tokens,
            ));
        }
        Ok(())
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
