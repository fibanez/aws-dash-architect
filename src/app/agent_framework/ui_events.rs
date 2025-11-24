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

/// Global UI event channel
///
/// This is initialized once on first access and provides a sender/receiver pair
/// for communicating UI events from agent tools to the UI layer.
///
/// Note: We use Arc<Mutex<Receiver>> because std::sync::mpsc::Receiver is not Clone.
/// The sender is Clone, so we can hand out clones to tools.
static UI_EVENT_CHANNEL: OnceLock<(Sender<AgentUIEvent>, Arc<Mutex<Receiver<AgentUIEvent>>>)> =
    OnceLock::new();

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
        assert_eq!(
            rx.try_recv().unwrap(),
            AgentUIEvent::SwitchToParent(agent2)
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            AgentUIEvent::AgentCompleted(agent1)
        );
    }
}
