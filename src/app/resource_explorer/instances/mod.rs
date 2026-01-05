//! Explorer Instances - Multi-pane, multi-tab, multi-window architecture
//!
//! This module provides a flexible Explorer architecture:
//! - Multiple windows: Each with its own set of tabs
//! - Multiple tabs per window: Each tab can have 1-2 panes
//! - Two panes per tab: Second pane toggled via button
//! - Independent state per pane: Own selections, queries, tree view, filters
//! - Shared resources: Global bookmarks and shared Moka cache

pub mod instance;
pub mod manager;
pub mod pane;
pub mod pane_renderer;
pub mod tab;

pub use instance::ExplorerInstance;
pub use manager::{ExplorerManager, ExplorerSharedContext};
pub use pane::ExplorerPane;
pub use pane_renderer::{PaneAction, PaneRenderer};
pub use tab::ExplorerTab;
