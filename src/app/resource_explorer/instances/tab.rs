//! Explorer Tab - A tab containing 1-2 panes
//!
//! Each tab can have:
//! - A left pane (always visible)
//! - An optional right pane (toggled via button)
//! - A user-editable name

use super::pane::ExplorerPane;
use super::pane_renderer::PaneAction;
use egui::Ui;
use uuid::Uuid;

/// A tab containing 1-2 panes
pub struct ExplorerTab {
    /// Unique identifier for this tab
    pub id: Uuid,
    /// User-editable tab name
    pub name: String,
    /// Left pane (always present)
    pub left_pane: ExplorerPane,
    /// Right pane (optional, created on demand)
    pub right_pane: Option<ExplorerPane>,
    /// Whether to show the right pane
    pub show_right_pane: bool,
    /// UI state: is the tab name being edited?
    pub is_renaming: bool,
    /// UI state: buffer for rename operation
    pub rename_buffer: String,
}

impl ExplorerTab {
    /// Create a new tab with the given name
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            left_pane: ExplorerPane::new(),
            right_pane: None,
            show_right_pane: false,
            is_renaming: false,
            rename_buffer: String::new(),
        }
    }

    /// Create a new tab with a default numbered name
    pub fn with_number(tab_number: usize) -> Self {
        Self::new(&format!("Tab {}", tab_number))
    }

    /// Get the tab's unique ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Toggle the right pane visibility
    pub fn toggle_right_pane(&mut self) {
        self.show_right_pane = !self.show_right_pane;

        // Create the right pane if it doesn't exist and we're showing it
        if self.show_right_pane && self.right_pane.is_none() {
            self.right_pane = Some(ExplorerPane::new());
        }
    }

    /// Start renaming the tab
    pub fn start_rename(&mut self) {
        self.is_renaming = true;
        self.rename_buffer = self.name.clone();
    }

    /// Finish renaming the tab (apply the new name)
    pub fn finish_rename(&mut self) {
        if !self.rename_buffer.trim().is_empty() {
            self.name = self.rename_buffer.trim().to_string();
        }
        self.is_renaming = false;
        self.rename_buffer.clear();
    }

    /// Cancel renaming (discard changes)
    pub fn cancel_rename(&mut self) {
        self.is_renaming = false;
        self.rename_buffer.clear();
    }

    /// Check if either pane has resources
    pub fn has_resources(&self) -> bool {
        self.left_pane.has_resources()
            || self.right_pane.as_ref().is_some_and(|p| p.has_resources())
    }

    /// Check if either pane is loading
    pub fn is_loading(&self) -> bool {
        self.left_pane.is_loading() || self.right_pane.as_ref().is_some_and(|p| p.is_loading())
    }

    /// Get total resource count across both panes
    pub fn resource_count(&self) -> usize {
        self.left_pane.resource_count() + self.right_pane.as_ref().map_or(0, |p| p.resource_count())
    }

    /// Render the tab content (panes)
    ///
    /// Returns actions from all panes for the window to process.
    pub fn render_content(&mut self, ui: &mut Ui) -> Vec<PaneAction> {
        let mut actions = Vec::new();

        // Render split pane toggle button
        ui.horizontal(|ui| {
            let button_text = if self.show_right_pane {
                "Hide Split"
            } else {
                "Show Split"
            };
            if ui.button(button_text).clicked() {
                self.toggle_right_pane();
            }
        });
        ui.separator();

        if self.show_right_pane {
            // Split view: two panes side by side
            ui.columns(2, |columns| {
                // Left pane
                actions.extend(self.left_pane.render(&mut columns[0]));

                // Right pane (create if needed)
                if let Some(ref mut right_pane) = self.right_pane {
                    actions.extend(right_pane.render(&mut columns[1]));
                }
            });
        } else {
            // Single pane view
            actions.extend(self.left_pane.render(ui));
        }

        actions
    }
}
