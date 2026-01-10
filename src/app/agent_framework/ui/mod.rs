//! UI components and events
//!
//! This module provides UI integration for agents, including status display
//! and event handling.

pub mod agent_events;
pub mod events;
pub mod status_display;

// Re-export commonly used items
pub use agent_events::*;
pub use events::*;
pub use status_display::*;
