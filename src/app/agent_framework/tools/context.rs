#![warn(clippy::all, rust_2018_idioms)]

//! Tool Context - Thread-local storage for agent execution context
//!
//! This module provides a mechanism for tools to access the agent ID and agent type
//! of the agent that is currently executing them. This is needed because the stood
//! library's Tool trait doesn't currently provide agent_id or agent_type in the AgentContext.
//!
//! ## Usage
//!
//! Before executing an agent's orchestrator, call `set_current_agent_id` and `set_current_agent_type`:
//! ```rust,ignore
//! set_current_agent_id(agent_id);
//! set_current_agent_type(agent_type);
//! // Execute tools...
//! clear_current_agent_id();
//! clear_current_agent_type();
//! ```
//!
//! Tools can then retrieve the current agent ID and type:
//! ```rust,ignore
//! let agent_id = get_current_agent_id().ok_or(...)?;
//! let agent_type = get_current_agent_type().ok_or(...)?;
//! ```

use crate::app::agent_framework::{AgentId, AgentType};
use std::cell::RefCell;

thread_local! {
    /// Thread-local storage for the currently executing agent's ID
    static CURRENT_AGENT_ID: RefCell<Option<AgentId>> = const { RefCell::new(None) };

    /// Thread-local storage for the currently executing agent's type
    static CURRENT_AGENT_TYPE: RefCell<Option<AgentType>> = const { RefCell::new(None) };
}

/// Set the current agent ID for this thread
///
/// This should be called before executing an agent's orchestrator
/// so that tools can access the agent ID during execution.
pub fn set_current_agent_id(agent_id: AgentId) {
    CURRENT_AGENT_ID.with(|id| {
        *id.borrow_mut() = Some(agent_id);
    });
}

/// Get the current agent ID for this thread
///
/// Returns None if no agent is currently executing on this thread.
pub fn get_current_agent_id() -> Option<AgentId> {
    CURRENT_AGENT_ID.with(|id| *id.borrow())
}

/// Clear the current agent ID for this thread
///
/// This should be called after agent execution completes.
pub fn clear_current_agent_id() {
    CURRENT_AGENT_ID.with(|id| {
        *id.borrow_mut() = None;
    });
}

/// Set the current agent type for this thread
///
/// This should be called before executing an agent's orchestrator
/// so that tools can access the agent type during execution.
pub fn set_current_agent_type(agent_type: AgentType) {
    CURRENT_AGENT_TYPE.with(|t| {
        *t.borrow_mut() = Some(agent_type);
    });
}

/// Get the current agent type for this thread
///
/// Returns None if no agent is currently executing on this thread.
pub fn get_current_agent_type() -> Option<AgentType> {
    CURRENT_AGENT_TYPE.with(|t| t.borrow().clone())
}

/// Clear the current agent type for this thread
///
/// This should be called after agent execution completes.
pub fn clear_current_agent_type() {
    CURRENT_AGENT_TYPE.with(|t| {
        *t.borrow_mut() = None;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_storage() {
        // Initially None
        assert!(get_current_agent_id().is_none());

        // Set and retrieve
        let agent_id = AgentId::new();
        set_current_agent_id(agent_id);
        assert_eq!(get_current_agent_id(), Some(agent_id));

        // Clear
        clear_current_agent_id();
        assert!(get_current_agent_id().is_none());
    }

    #[test]
    fn test_agent_id_overwrite() {
        let agent_id1 = AgentId::new();
        let agent_id2 = AgentId::new();

        set_current_agent_id(agent_id1);
        assert_eq!(get_current_agent_id(), Some(agent_id1));

        // Overwrite with second ID
        set_current_agent_id(agent_id2);
        assert_eq!(get_current_agent_id(), Some(agent_id2));

        clear_current_agent_id();
    }
}
