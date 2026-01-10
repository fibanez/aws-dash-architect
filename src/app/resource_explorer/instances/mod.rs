//! Explorer Instances - Multi-pane, multi-window architecture
//!
//! This module provides a flexible Explorer architecture:
//! - Multiple windows: Each window can have 1-2 panes
//! - Two panes per window: Second pane toggled via button
//! - Independent state per pane: Own selections, queries, tree view, filters
//! - Shared resources: Global bookmarks, shared Moka cache, and query engine

pub mod instance;
pub mod manager;
pub mod pane;
pub mod pane_renderer;

pub use instance::ExplorerInstance;
pub use manager::{ExplorerManager, ExplorerSharedContext};
pub use pane::ExplorerPane;
pub use pane_renderer::{PaneAction, PaneRenderer};
