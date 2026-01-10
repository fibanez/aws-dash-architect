//! Worker agent management
//!
//! This module handles worker completion tracking and progress reporting.

pub mod completion;
pub mod progress;

// Re-export commonly used items
pub use completion::*;
pub use progress::*;
