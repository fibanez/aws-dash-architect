#![warn(clippy::all, rust_2018_idioms)]

//! UI Event System for Agent Framework
//!
//! This module provides a global event channel that allows agent tools to trigger
//! UI changes without requiring direct access to AgentManagerWindow.
//!
//! ## Architecture
//!
//! Tools (like start-task) send UI events to a global channel. AgentManagerWindow
//! polls this channel and processes events to update the UI state.

use crate::app::agent_framework::agent_types::AgentId;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};

/// Type alias for the UI event channel
type UIEventChannelType = (Sender<AgentUIEvent>, Arc<Mutex<Receiver<AgentUIEvent>>>);

/// Global UI event channel
///
/// This is initialized once on first access and provides a sender/receiver pair
/// for communicating UI events from agent tools to the UI layer.
///
/// Note: We use Arc<Mutex<Receiver>> because std::sync::mpsc::Receiver is not Clone.
/// The sender is Clone, so we can hand out clones to tools.
static UI_EVENT_CHANNEL: OnceLock<UIEventChannelType> = OnceLock::new();

/// Initialize the global UI event channel
///
/// This is called automatically on first access via get_ui_event_sender() or
/// get_ui_event_receiver(). You can also call it explicitly during application
/// initialization if you want to ensure the channel is ready.
pub fn init_ui_event_channel() {
    UI_EVENT_CHANNEL.get_or_init(|| {
        let (sender, receiver) = channel();
        (sender, Arc::new(Mutex::new(receiver)))
    });
}

/// Get a clone of the UI event sender
///
/// This sender can be used by agent tools to send UI events without requiring
/// access to AgentManagerWindow. The channel is automatically initialized on
/// first access.
pub fn get_ui_event_sender() -> Sender<AgentUIEvent> {
    UI_EVENT_CHANNEL
        .get_or_init(|| {
            let (sender, receiver) = channel();
            (sender, Arc::new(Mutex::new(receiver)))
        })
        .0
        .clone()
}

/// Get a clone of the UI event receiver
///
/// This receiver should be used by AgentManagerWindow to poll for UI events.
/// The channel is automatically initialized on first access.
///
/// Note: The receiver is wrapped in Arc<Mutex<>> because std::sync::mpsc::Receiver
/// is not Clone. The UI should lock the mutex to check for events.
pub fn get_ui_event_receiver() -> Arc<Mutex<Receiver<AgentUIEvent>>> {
    UI_EVENT_CHANNEL
        .get_or_init(|| {
            let (sender, receiver) = channel();
            (sender, Arc::new(Mutex::new(receiver)))
        })
        .1
        .clone()
}

/// Events that trigger UI state changes in AgentManagerWindow
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentUIEvent {
    /// Switch the UI to display a specific agent
    ///
    /// Sent when a new task-agent is spawned and should be shown to the user
    SwitchToAgent(AgentId),

    /// Switch back to the parent agent
    ///
    /// Sent when a task-worker completes and the UI should return to the
    /// task-manager that spawned it
    SwitchToParent(AgentId),

    /// Notify that an agent has completed its work
    ///
    /// This allows the UI to update task indicators, remove completed agents
    /// from the active list, etc.
    AgentCompleted(AgentId),

    // ========== Worker Progress Events ==========

    /// A worker agent has started
    ///
    /// Sent when a TaskWorker is created by a TaskManager.
    /// Includes the short description for inline display.
    WorkerStarted {
        worker_id: AgentId,
        parent_id: AgentId,
        short_description: String,
        message_index: usize,
    },

    /// A worker has started executing a tool
    ///
    /// Sent when a tool (e.g., execute_javascript) starts running.
    WorkerToolStarted {
        worker_id: AgentId,
        parent_id: AgentId,
        tool_name: String,
    },

    /// A worker has finished executing a tool
    ///
    /// Sent when a tool completes (success or failure).
    WorkerToolCompleted {
        worker_id: AgentId,
        parent_id: AgentId,
        tool_name: String,
        success: bool,
    },

    /// A worker agent has completed
    ///
    /// Sent when a TaskWorker finishes its work.
    WorkerCompleted {
        worker_id: AgentId,
        parent_id: AgentId,
        success: bool,
    },

    /// Worker token usage updated
    ///
    /// Sent after each model call with cumulative token counts.
    WorkerTokensUpdated {
        worker_id: AgentId,
        parent_id: AgentId,
        input_tokens: u32,
        output_tokens: u32,
        total_tokens: u32,
    },
}

impl AgentUIEvent {
    /// Create a new SwitchToAgent event
    pub fn switch_to_agent(agent_id: AgentId) -> Self {
        Self::SwitchToAgent(agent_id)
    }

    /// Create a new SwitchToParent event
    pub fn switch_to_parent(parent_id: AgentId) -> Self {
        Self::SwitchToParent(parent_id)
    }

    /// Create a new AgentCompleted event
    pub fn agent_completed(agent_id: AgentId) -> Self {
        Self::AgentCompleted(agent_id)
    }

    /// Create a new WorkerStarted event
    pub fn worker_started(
        worker_id: AgentId,
        parent_id: AgentId,
        short_description: String,
        message_index: usize,
    ) -> Self {
        Self::WorkerStarted {
            worker_id,
            parent_id,
            short_description,
            message_index,
        }
    }

    /// Create a new WorkerToolStarted event
    pub fn worker_tool_started(
        worker_id: AgentId,
        parent_id: AgentId,
        tool_name: String,
    ) -> Self {
        Self::WorkerToolStarted {
            worker_id,
            parent_id,
            tool_name,
        }
    }

    /// Create a new WorkerToolCompleted event
    pub fn worker_tool_completed(
        worker_id: AgentId,
        parent_id: AgentId,
        tool_name: String,
        success: bool,
    ) -> Self {
        Self::WorkerToolCompleted {
            worker_id,
            parent_id,
            tool_name,
            success,
        }
    }

    /// Create a new WorkerCompleted event
    pub fn worker_completed(
        worker_id: AgentId,
        parent_id: AgentId,
        success: bool,
    ) -> Self {
        Self::WorkerCompleted {
            worker_id,
            parent_id,
            success,
        }
    }

    /// Create a new WorkerTokensUpdated event
    pub fn worker_tokens_updated(
        worker_id: AgentId,
        parent_id: AgentId,
        input_tokens: u32,
        output_tokens: u32,
        total_tokens: u32,
    ) -> Self {
        Self::WorkerTokensUpdated {
            worker_id,
            parent_id,
            input_tokens,
            output_tokens,
            total_tokens,
        }
    }
}

/// Send a UI event to the global channel
///
/// This is a convenience function that tools can use to send events without
/// needing to get the sender themselves.
///
/// # Errors
///
/// Returns an error if the channel receiver has been dropped (should never
/// happen in normal operation since AgentManagerWindow holds the receiver).
pub fn send_ui_event(event: AgentUIEvent) -> Result<(), String> {
    get_ui_event_sender()
        .send(event)
        .map_err(|e| format!("Failed to send UI event: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_event_creation() {
        let agent_id = AgentId::new();

        let switch = AgentUIEvent::switch_to_agent(agent_id);
        assert_eq!(switch, AgentUIEvent::SwitchToAgent(agent_id));

        let parent = AgentUIEvent::switch_to_parent(agent_id);
        assert_eq!(parent, AgentUIEvent::SwitchToParent(agent_id));

        let completed = AgentUIEvent::agent_completed(agent_id);
        assert_eq!(completed, AgentUIEvent::AgentCompleted(agent_id));
    }

    #[test]
    fn test_channel_initialization() {
        // Getting sender/receiver should work
        let sender = get_ui_event_sender();
        let receiver = get_ui_event_receiver();

        // Should be able to send and receive
        let agent_id = AgentId::new();
        let event = AgentUIEvent::switch_to_agent(agent_id);

        sender.send(event.clone()).unwrap();
        let received = receiver.lock().unwrap().try_recv().unwrap();

        assert_eq!(received, event);
    }

    #[test]
    fn test_send_ui_event_helper() {
        let receiver = get_ui_event_receiver();
        let agent_id = AgentId::new();

        // Send event using helper function
        send_ui_event(AgentUIEvent::switch_to_agent(agent_id)).unwrap();

        // Should be received
        let received = receiver.lock().unwrap().try_recv().unwrap();
        assert_eq!(received, AgentUIEvent::SwitchToAgent(agent_id));
    }

    #[test]
    fn test_multiple_events_in_order() {
        let receiver = get_ui_event_receiver();
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();

        // Send multiple events
        send_ui_event(AgentUIEvent::switch_to_agent(agent1)).unwrap();
        send_ui_event(AgentUIEvent::switch_to_parent(agent2)).unwrap();
        send_ui_event(AgentUIEvent::agent_completed(agent1)).unwrap();

        // Should be received in order
        let rx = receiver.lock().unwrap();
        assert_eq!(rx.try_recv().unwrap(), AgentUIEvent::SwitchToAgent(agent1));
        assert_eq!(rx.try_recv().unwrap(), AgentUIEvent::SwitchToParent(agent2));
        assert_eq!(rx.try_recv().unwrap(), AgentUIEvent::AgentCompleted(agent1));
    }

    #[test]
    fn test_worker_progress_events() {
        let worker_id = AgentId::new();
        let parent_id = AgentId::new();

        // Test WorkerStarted
        let started = AgentUIEvent::worker_started(
            worker_id,
            parent_id,
            "Listing instances".to_string(),
            5,
        );
        assert!(matches!(
            started,
            AgentUIEvent::WorkerStarted {
                worker_id: w,
                parent_id: p,
                short_description: _,
                message_index: 5,
            } if w == worker_id && p == parent_id
        ));

        // Test WorkerToolStarted
        let tool_started = AgentUIEvent::worker_tool_started(
            worker_id,
            parent_id,
            "execute_javascript".to_string(),
        );
        assert!(matches!(
            tool_started,
            AgentUIEvent::WorkerToolStarted {
                worker_id: w,
                parent_id: p,
                tool_name: _,
            } if w == worker_id && p == parent_id
        ));

        // Test WorkerToolCompleted
        let tool_completed = AgentUIEvent::worker_tool_completed(
            worker_id,
            parent_id,
            "execute_javascript".to_string(),
            true,
        );
        assert!(matches!(
            tool_completed,
            AgentUIEvent::WorkerToolCompleted {
                worker_id: w,
                parent_id: p,
                tool_name: _,
                success: true,
            } if w == worker_id && p == parent_id
        ));

        // Test WorkerCompleted
        let worker_completed = AgentUIEvent::worker_completed(
            worker_id,
            parent_id,
            true,
        );
        assert!(matches!(
            worker_completed,
            AgentUIEvent::WorkerCompleted {
                worker_id: w,
                parent_id: p,
                success: true,
            } if w == worker_id && p == parent_id
        ));
    }
}
