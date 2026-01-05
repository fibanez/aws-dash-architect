//! Explorer Pane - A single explorer pane with independent state
//!
//! Each pane has its own ResourceExplorerState and can independently:
//! - Query resources
//! - Apply filters
//! - Navigate the tree view
//! - Apply bookmarks

use super::pane_renderer::{PaneAction, PaneRenderer};
use crate::app::resource_explorer::state::ResourceExplorerState;
use egui::Ui;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A single explorer pane with independent state
pub struct ExplorerPane {
    /// Unique identifier for this pane
    pub id: Uuid,
    /// Independent state for this pane (query scope, filters, resources, tree)
    /// Wrapped in Arc<RwLock> for async access from query tasks
    pub state: Arc<RwLock<ResourceExplorerState>>,
    /// Renderer for this pane (tree view, active tags, search, etc.)
    pub renderer: PaneRenderer,
    /// UI state: scroll offset in the tree view
    pub scroll_offset: f32,
    /// UI state: currently selected resource ARN
    pub selected_resource: Option<String>,
}

impl Default for ExplorerPane {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerPane {
    /// Create a new empty pane
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            state: Arc::new(RwLock::new(ResourceExplorerState::new())),
            renderer: PaneRenderer::new(),
            scroll_offset: 0.0,
            selected_resource: None,
        }
    }

    /// Get the pane's unique ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the state for read access
    pub fn get_state(&self) -> Arc<RwLock<ResourceExplorerState>> {
        self.state.clone()
    }

    /// Get the renderer mutably
    pub fn renderer_mut(&mut self) -> &mut PaneRenderer {
        &mut self.renderer
    }

    /// Check if this pane has any resources loaded
    pub fn has_resources(&self) -> bool {
        self.state
            .try_read()
            .map(|s| !s.resources.is_empty())
            .unwrap_or(false)
    }

    /// Check if this pane is currently loading
    pub fn is_loading(&self) -> bool {
        self.state
            .try_read()
            .map(|s| s.is_loading())
            .unwrap_or(false)
    }

    /// Get the number of resources in this pane
    pub fn resource_count(&self) -> usize {
        self.state
            .try_read()
            .map(|s| s.resources.len())
            .unwrap_or(0)
    }

    /// Clear the pane state (like terminate, but preserves pane identity)
    pub fn clear(&mut self) {
        if let Ok(mut state) = self.state.try_write() {
            *state = ResourceExplorerState::new();
        }
        self.renderer.reset();
        self.scroll_offset = 0.0;
        self.selected_resource = None;
    }

    /// Render the pane content
    ///
    /// This method renders the complete pane UI including:
    /// - Active selection tags
    /// - Search bar
    /// - Tree view of resources
    ///
    /// Returns a list of actions triggered during rendering (e.g., tag removals)
    /// that should be processed by the window/manager.
    pub fn render(&mut self, ui: &mut Ui) -> Vec<PaneAction> {
        // Try to acquire write lock on state (non-blocking to avoid UI freeze)
        if let Ok(mut state) = self.state.try_write() {
            self.renderer.render(ui, &mut state)
        } else {
            // State is locked (probably by async query), show loading indicator
            ui.centered_and_justified(|ui| {
                ui.spinner();
                ui.label("Loading...");
            });
            Vec::new()
        }
    }
}
