//! Utility functions and registries
//!
//! This module provides utility functions for workspace management
//! and tool registration.

pub mod registry;
pub mod workspace;

// Re-export commonly used items
pub use registry::*;
pub use workspace::*;
