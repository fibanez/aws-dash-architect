//! Universal Sub-Agent Callback Handler (DEPRECATED)
//!
//! This handler is deprecated. Agents now handle their own event loops without streaming.
//! Kept as a stub for backward compatibility with specialized tools.

use async_trait::async_trait;
use std::sync::mpsc;
use stood::agent::callbacks::{CallbackError, CallbackEvent, CallbackHandler};
use tracing::debug;

/// Stub callback handler for backward compatibility
///
/// This handler does nothing - agents handle their own event loops now
#[derive(Debug)]
pub struct SubAgentCallbackHandler {
    agent_id: String,
    _agent_type: String,
    _sender: Option<mpsc::Sender<()>>,
}

impl SubAgentCallbackHandler {
    /// Create a new handler without Bridge communication (standalone mode)
    pub fn new(agent_id: String, agent_type: String) -> Self {
        debug!("Creating stub SubAgentCallbackHandler for {}", agent_id);
        Self {
            agent_id,
            _agent_type: agent_type,
            _sender: None,
        }
    }

    /// Create a new handler with Bridge communication (ignored)
    pub fn with_sender(
        agent_id: String,
        agent_type: String,
        _sender: mpsc::Sender<crate::app::dashui::control_bridge_window::AgentResponse>,
    ) -> Self {
        debug!("Creating stub SubAgentCallbackHandler with sender for {}", agent_id);
        Self {
            agent_id,
            _agent_type: agent_type,
            _sender: None,
        }
    }
}

#[async_trait]
impl CallbackHandler for SubAgentCallbackHandler {
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        // No-op - agents handle their own event loops now
        debug!(
            "SubAgentCallbackHandler ({}) ignoring event: {:?}",
            self.agent_id,
            event
        );
        Ok(())
    }
}