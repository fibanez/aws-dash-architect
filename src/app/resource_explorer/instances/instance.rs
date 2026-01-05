//! Explorer Instance - A single Explorer window with multiple tabs
//!
//! Each instance represents one Explorer window and can have:
//! - Multiple tabs
//! - Its own window position and size
//! - Open/close state

use super::pane_renderer::PaneAction;
use super::tab::ExplorerTab;
use egui::{Color32, Ui};
use uuid::Uuid;

/// A single Explorer window with multiple tabs
pub struct ExplorerInstance {
    /// Unique identifier for this instance
    pub id: Uuid,
    /// List of tabs in this instance
    pub tabs: Vec<ExplorerTab>,
    /// Index of the currently active tab
    pub active_tab_index: usize,
    /// Whether the window is open (visible)
    pub is_open: bool,
    /// Window title (e.g., "Explorer 1", "Explorer 2")
    pub title: String,
    /// Instance number (for default naming)
    instance_number: usize,
}

impl ExplorerInstance {
    /// Create a new instance with a default tab
    pub fn new(instance_number: usize) -> Self {
        let title = if instance_number == 1 {
            "Explorer".to_string()
        } else {
            format!("Explorer {}", instance_number)
        };

        Self {
            id: Uuid::new_v4(),
            tabs: vec![ExplorerTab::with_number(1)],
            active_tab_index: 0,
            is_open: true,
            title,
            instance_number,
        }
    }

    /// Get the instance's unique ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the instance number
    pub fn instance_number(&self) -> usize {
        self.instance_number
    }

    /// Add a new tab and return a mutable reference to it
    pub fn add_tab(&mut self) -> &mut ExplorerTab {
        let tab_number = self.tabs.len() + 1;
        self.tabs.push(ExplorerTab::with_number(tab_number));
        self.active_tab_index = self.tabs.len() - 1;
        self.tabs.last_mut().unwrap()
    }

    /// Close a tab by index
    /// Returns true if the tab was closed, false if it was the last tab
    pub fn close_tab(&mut self, index: usize) -> bool {
        // Don't close the last tab
        if self.tabs.len() <= 1 {
            return false;
        }

        if index < self.tabs.len() {
            self.tabs.remove(index);

            // Adjust active tab index if needed
            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len().saturating_sub(1);
            } else if self.active_tab_index > index {
                self.active_tab_index = self.active_tab_index.saturating_sub(1);
            }
            true
        } else {
            false
        }
    }

    /// Get the currently active tab
    pub fn active_tab(&self) -> Option<&ExplorerTab> {
        self.tabs.get(self.active_tab_index)
    }

    /// Get the currently active tab mutably
    pub fn active_tab_mut(&mut self) -> Option<&mut ExplorerTab> {
        self.tabs.get_mut(self.active_tab_index)
    }

    /// Set the active tab by index
    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab_index = index;
        }
    }

    /// Get the number of tabs
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Check if any tab has resources
    pub fn has_resources(&self) -> bool {
        self.tabs.iter().any(|t| t.has_resources())
    }

    /// Check if any tab is loading
    pub fn is_loading(&self) -> bool {
        self.tabs.iter().any(|t| t.is_loading())
    }

    /// Render the tab bar
    ///
    /// Returns the index of a tab to close if one was requested, and
    /// optionally an index to switch to
    pub fn render_tab_bar(&mut self, ui: &mut Ui) -> Option<usize> {
        let mut tab_to_close: Option<usize> = None;
        let mut tab_to_activate: Option<usize> = None;
        let mut tab_to_start_rename: Option<usize> = None;
        let mut add_new_tab = false;

        // Cache values before the loop to avoid borrow conflicts
        let tab_count = self.tabs.len();
        let active_index = self.active_tab_index;

        ui.horizontal(|ui| {
            // Render each tab
            for (index, tab) in self.tabs.iter_mut().enumerate() {
                let is_active = index == active_index;

                // Tab button styling
                let bg_color = if is_active {
                    Color32::from_rgb(60, 60, 70)
                } else {
                    Color32::from_rgb(40, 40, 50)
                };

                egui::Frame::new()
                    .fill(bg_color)
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if tab.is_renaming {
                                // Rename mode: show text input
                                let response = ui.text_edit_singleline(&mut tab.rename_buffer);
                                if response.lost_focus() {
                                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        tab.finish_rename();
                                    } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                        tab.cancel_rename();
                                    } else {
                                        // Clicked outside - finish rename
                                        tab.finish_rename();
                                    }
                                }
                                response.request_focus();
                            } else {
                                // Normal mode: show tab name as clickable label
                                let label = egui::Label::new(
                                    egui::RichText::new(&tab.name)
                                        .color(if is_active {
                                            Color32::WHITE
                                        } else {
                                            Color32::LIGHT_GRAY
                                        })
                                        .small(),
                                )
                                .sense(egui::Sense::click());

                                let response = ui.add(label);

                                // Single click: switch to tab
                                if response.clicked() {
                                    tab_to_activate = Some(index);
                                }

                                // Double click: start rename
                                if response.double_clicked() {
                                    tab_to_start_rename = Some(index);
                                }
                            }

                            // Close button (only if more than one tab)
                            if tab_count > 1
                                && ui
                                    .small_button("x")
                                    .on_hover_text("Close tab")
                                    .clicked()
                            {
                                tab_to_close = Some(index);
                            }
                        });
                    });

                ui.add_space(2.0);
            }

            // Add tab button
            if ui.button("+").on_hover_text("New tab").clicked() {
                add_new_tab = true;
            }
        });

        // Apply deferred actions
        if let Some(index) = tab_to_activate {
            self.active_tab_index = index;
        }
        if let Some(index) = tab_to_start_rename {
            if let Some(tab) = self.tabs.get_mut(index) {
                tab.start_rename();
            }
        }
        if add_new_tab {
            self.add_tab();
        }

        tab_to_close
    }

    /// Render the active tab's content
    ///
    /// Returns actions from the tab's panes for processing
    pub fn render_active_tab_content(&mut self, ui: &mut Ui) -> Vec<PaneAction> {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.render_content(ui)
        } else {
            Vec::new()
        }
    }

    /// Render the complete instance UI (tab bar + content)
    ///
    /// Returns actions from the active tab's panes for processing
    pub fn render(&mut self, ui: &mut Ui) -> Vec<PaneAction> {
        // Render tab bar
        if let Some(tab_to_close) = self.render_tab_bar(ui) {
            self.close_tab(tab_to_close);
        }
        ui.separator();

        // Render active tab content
        self.render_active_tab_content(ui)
    }
}
