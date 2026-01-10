//! Explorer Instance - A single Explorer window with 1-2 panes
//!
//! Each instance represents one Explorer window and can have:
//! - A left pane (always present)
//! - An optional right pane (split view)
//! - Its own window position and size
//! - Open/close state

use super::manager::ExplorerSharedContext;
use super::pane::ExplorerPane;
use super::pane_renderer::PaneAction;
use crate::app::dashui::window_focus::FocusableWindow;
use crate::app::resource_explorer::bookmarks::BookmarkFolder;
use egui::{Context, Ui, Window};
use std::collections::HashSet;
use uuid::Uuid;

/// Redact sensitive string, showing only last 4 characters
///
/// Used for account IDs in logs.
/// Example: "123456789012" -> "********9012"
fn redact_sensitive(value: &str) -> String {
    if value.len() <= 4 {
        "*".repeat(value.len())
    } else {
        format!(
            "{}{}",
            "*".repeat(value.len() - 4),
            &value[value.len() - 4..]
        )
    }
}

/// Drag-drop payload for bookmark manager
///
/// Supports dragging both bookmarks and folders for organization
#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum DragData {
    /// Dragging a bookmark from a folder
    Bookmark {
        id: String,
        source_folder: Option<String>,
    },
    /// Dragging a folder to reorganize hierarchy
    Folder {
        id: String,
        parent_id: Option<String>,
    },
}

/// A single Explorer window with 1-2 panes
pub struct ExplorerInstance {
    /// Unique identifier for this instance
    pub id: Uuid,
    /// Left pane (always present)
    pub left_pane: ExplorerPane,
    /// Right pane (optional, created on demand)
    pub right_pane: Option<ExplorerPane>,
    /// Whether to show the right pane
    pub show_right_pane: bool,
    /// Whether the window is open (visible)
    pub is_open: bool,
    /// Window title (e.g., "Explorer 1", "Explorer 2")
    pub title: String,
    /// Instance number (for default naming)
    instance_number: usize,
    /// Cached static window ID (leaked string for FocusableWindow trait)
    window_id_static: Option<&'static str>,

    // ========================================================================
    // Bookmark Manager Dialog State
    // ========================================================================

    /// Show bookmark edit dialog
    show_bookmark_edit_dialog: bool,
    /// ID of bookmark being edited
    editing_bookmark_id: Option<String>,
    /// Temporary name field for bookmark edit dialog
    bookmark_edit_name: String,
    /// Temporary description field for bookmark edit dialog
    bookmark_edit_description: String,

    // ========================================================================
    // Folder Management State
    // ========================================================================

    /// Show folder create/edit dialog
    show_folder_dialog: bool,
    /// Temporary name field for folder dialog
    folder_dialog_name: String,
    /// Selected parent folder ID for folder dialog
    folder_dialog_parent_id: Option<String>,
    /// ID of folder being edited (None = creating new)
    editing_folder_id: Option<String>,
    /// Set of expanded folder IDs (for tree view state)
    expanded_folders: HashSet<String>,

    // ========================================================================
    // Clipboard State
    // ========================================================================

    /// Bookmark ID currently in clipboard
    bookmark_clipboard: Option<String>,
    /// True if Cut operation (move), False if Copy operation (duplicate)
    bookmark_clipboard_is_cut: bool,
}

impl ExplorerInstance {
    /// Create a new instance with a single pane
    pub fn new(instance_number: usize) -> Self {
        let title = if instance_number == 1 {
            "Explorer".to_string()
        } else {
            format!("Explorer {}", instance_number)
        };

        Self {
            id: Uuid::new_v4(),
            left_pane: ExplorerPane::new(),
            right_pane: None,
            show_right_pane: false,
            is_open: true,
            title,
            instance_number,
            window_id_static: None,

            // Bookmark manager dialog state
            show_bookmark_edit_dialog: false,
            editing_bookmark_id: None,
            bookmark_edit_name: String::new(),
            bookmark_edit_description: String::new(),

            // Folder management state
            show_folder_dialog: false,
            folder_dialog_name: String::new(),
            folder_dialog_parent_id: None,
            editing_folder_id: None,
            expanded_folders: HashSet::new(),

            // Clipboard state
            bookmark_clipboard: None,
            bookmark_clipboard_is_cut: false,
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

    /// Toggle the right pane visibility
    pub fn toggle_right_pane(&mut self) {
        self.show_right_pane = !self.show_right_pane;

        // Create the right pane if it doesn't exist and we're showing it
        if self.show_right_pane && self.right_pane.is_none() {
            self.right_pane = Some(ExplorerPane::new());
        }
    }

    /// Check if any pane has resources
    pub fn has_resources(&self) -> bool {
        self.left_pane.has_resources()
            || self.right_pane.as_ref().is_some_and(|p| p.has_resources())
    }

    /// Check if any pane is loading
    pub fn is_loading(&self) -> bool {
        self.left_pane.is_loading()
            || self.right_pane.as_ref().is_some_and(|p| p.is_loading())
    }

    /// Render the complete instance UI (panes with optional split)
    ///
    /// Returns actions from the panes for processing
    pub fn render(
        &mut self,
        ui: &mut Ui,
        shared_context: &super::manager::ExplorerSharedContext,
    ) -> Vec<PaneAction> {
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
                actions.extend(self.left_pane.render(&mut columns[0], shared_context));

                // Right pane with left border for visual separation
                egui::Frame::new()
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_gray(100)))
                    .show(&mut columns[1], |ui| {
                        if let Some(ref mut right_pane) = self.right_pane {
                            actions.extend(right_pane.render(ui, shared_context));
                        }
                    });
            });
        } else {
            // Single pane view
            actions.extend(self.left_pane.render(ui, shared_context));
        }

        actions
    }

    /// Take pending ResourceExplorerActions from all panes in this instance
    ///
    /// Collects actions from both left and right panes
    pub fn take_pending_actions(
        &mut self,
    ) -> Vec<crate::app::resource_explorer::ResourceExplorerAction> {
        let mut actions = Vec::new();

        // Collect from left pane
        actions.extend(self.left_pane.take_pending_actions());

        // Collect from right pane if it exists
        if let Some(ref mut right_pane) = self.right_pane {
            actions.extend(right_pane.take_pending_actions());
        }

        actions
    }
}

// ============================================================================
// FocusableWindow Trait Implementation
// ============================================================================

impl FocusableWindow for ExplorerInstance {
    type ShowParams = ExplorerSharedContext;

    fn window_id(&self) -> &'static str {
        // Cache the window ID as a static string (leaked once per window)
        // This is required by the trait which needs &'static str
        // Memory leak: ~40 bytes per window (acceptable for typical usage)
        if let Some(cached_id) = self.window_id_static {
            cached_id
        } else {
            // SAFETY: This leaks memory intentionally to satisfy the trait requirement
            // Each window leaks approximately 40 bytes for the ID string
            // For 100 windows, this is ~4KB total, which is negligible
            Box::leak(format!("explorer_instance_{}", self.id).into_boxed_str())
        }
    }

    fn window_title(&self) -> String {
        self.title.clone()
    }

    fn is_open(&self) -> bool {
        self.is_open
    }

    fn show_with_focus(
        &mut self,
        ctx: &Context,
        shared_context: Self::ShowParams,
        bring_to_front: bool,
    ) {
        // Copy is_open to local variable to avoid borrow conflicts
        let mut is_open = self.is_open;

        let mut window = egui::Window::new(self.window_title())
            .id(egui::Id::new(self.window_id()))
            .default_size([1200.0, 800.0])
            .resizable(true)
            .open(&mut is_open);

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            // Render the instance content (panes)
            let actions = self.render(ui, &shared_context);

            // Process pane actions
            for action in actions {
                self.handle_pane_action(action, ctx, &shared_context);
            }
        });

        // Render dialogs for all panes (outside main window)
        self.left_pane.render_dialogs(ctx, &shared_context);
        if let Some(ref mut right_pane) = self.right_pane {
            right_pane.render_dialogs(ctx, &shared_context);
        }

        // Render shared dialogs (bookmark manager, etc.) that affect the whole instance
        self.render_shared_dialogs(ctx, &shared_context);
        self.render_bookmark_edit_dialog(ctx, &shared_context);
        self.render_folder_dialog(ctx, &shared_context);

        // Execute any pending query triggers (after all locks are released)
        self.left_pane.execute_pending_query(ctx, &shared_context);
        if let Some(ref mut right_pane) = self.right_pane {
            right_pane.execute_pending_query(ctx, &shared_context);
        }

        // Write back the is_open state
        self.is_open = is_open;
    }
}

// ============================================================================
// Shared Dialog Rendering
// ============================================================================

impl ExplorerInstance {
    /// Render shared dialogs that affect the whole instance
    ///
    /// These dialogs are not pane-specific (e.g., bookmark manager)
    fn render_shared_dialogs(&mut self, ctx: &Context, shared_context: &ExplorerSharedContext) {
        // Check if any pane wants to show the bookmark manager
        let show_bookmark_manager = if let Ok(state) = self.left_pane.state.try_read() {
            state.show_bookmark_manager
        } else {
            false
        } || if let Some(ref right_pane) = self.right_pane {
            if let Ok(state) = right_pane.state.try_read() {
                state.show_bookmark_manager
            } else {
                false
            }
        } else {
            false
        };

        if show_bookmark_manager {
            let mut is_open = true;
            let mut bookmark_to_delete: Option<String> = None;
            let mut bookmark_to_edit: Option<String> = None;
            let mut folder_to_delete: Option<String> = None;
            let mut folder_to_rename: Option<String> = None;
            let mut bookmark_to_paste: Option<(String, Option<String>)> = None;
            let mut folder_to_move: Option<(String, Option<String>)> = None;

            egui::Window::new("Manage Bookmarks")
                .default_size([700.0, 500.0])
                .resizable(true)
                .open(&mut is_open)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // Stats
                        ui.horizontal(|ui| {
                            let bookmark_count = shared_context.bookmarks.read().unwrap().get_bookmarks().len();
                            let folder_count = shared_context.bookmarks.read().unwrap().get_all_folders().len();
                            ui.label(format!("Total bookmarks: {}", bookmark_count));
                            ui.add_space(10.0);
                            ui.label(format!("Total folders: {}", folder_count));
                        });

                        // Toolbar
                        ui.horizontal(|ui| {
                            if ui.button("New Folder").clicked() {
                                self.show_folder_dialog = true;
                                self.folder_dialog_name = String::new();
                                self.folder_dialog_parent_id = None;
                                self.editing_folder_id = None;
                            }
                        });

                        ui.separator();

                        egui::ScrollArea::vertical()
                            .max_height(400.0)
                            .show(ui, |ui| {
                                // Top Folder drop zone - clear visual target
                                let top_folder_response = ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Top Folder").strong());
                                });

                                // Check if something is being dragged over Top Folder
                                if let Some(_dragged_data) = top_folder_response.response.dnd_hover_payload::<DragData>() {
                                    // Visual feedback: highlight Top Folder
                                    let painter = ui.painter();
                                    painter.rect_stroke(
                                        top_folder_response.response.rect,
                                        3.0,
                                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)),
                                        egui::epaint::StrokeKind::Outside,
                                    );
                                }

                                // Handle drop on Top Folder
                                if let Some(dropped) = top_folder_response.response.dnd_release_payload::<DragData>() {
                                    match dropped.as_ref() {
                                        DragData::Bookmark { id, source_folder } => {
                                            // Don't drop bookmark if it's already in Top Folder
                                            if source_folder.is_some() {
                                                bookmark_to_paste = Some((id.clone(), None));
                                                self.bookmark_clipboard_is_cut = true;
                                            }
                                        }
                                        DragData::Folder { id, parent_id: source_parent } => {
                                            // Don't drop folder if it's already in Top Folder
                                            if source_parent.is_some() {
                                                folder_to_move = Some((id.clone(), None));
                                            }
                                        }
                                    }
                                }

                                // Context menu for top folder
                                top_folder_response.response.context_menu(|ui| {
                                    if let Some(ref clipboard_id) = self.bookmark_clipboard {
                                        let action = if self.bookmark_clipboard_is_cut {
                                            "Move to Top Folder"
                                        } else {
                                            "Copy to Top Folder"
                                        };

                                        if ui.button(action).clicked() {
                                            bookmark_to_paste = Some((clipboard_id.clone(), None));
                                            ui.close();
                                        }
                                    }
                                });

                                ui.add_space(10.0);

                                // Render hierarchical folder tree starting from root (parent_id = None)
                                self.render_folder_tree_level(
                                    ui,
                                    None,
                                    shared_context,
                                    &mut bookmark_to_delete,
                                    &mut bookmark_to_edit,
                                    &mut folder_to_delete,
                                    &mut folder_to_rename,
                                    &mut bookmark_to_paste,
                                    &mut folder_to_move,
                                );
                            });
                    });
                });

            // Handle bookmark deletion
            if let Some(bookmark_id) = bookmark_to_delete {
                shared_context.bookmarks.write().unwrap().remove_bookmark(&bookmark_id);
                if let Err(e) = shared_context.bookmarks.write().unwrap().save() {
                    tracing::error!("Failed to save after bookmark deletion: {}", e);
                }
            }

            // Handle bookmark editing - populate edit dialog and show it
            if let Some(bookmark_id) = bookmark_to_edit {
                let bookmark = shared_context
                    .bookmarks
                    .read()
                    .unwrap()
                    .get_bookmarks()
                    .iter()
                    .find(|b| b.id == bookmark_id)
                    .cloned();

                if let Some(bookmark) = bookmark {
                    // Populate edit dialog fields
                    self.editing_bookmark_id = Some(bookmark.id.clone());
                    self.bookmark_edit_name = bookmark.name.clone();
                    self.bookmark_edit_description = bookmark.description.clone().unwrap_or_default();
                    self.show_bookmark_edit_dialog = true;
                    tracing::info!("Opening edit dialog for bookmark: {}", bookmark.name);
                }
            }

            // Handle folder deletion
            if let Some(folder_id) = folder_to_delete {
                if let Err(e) = shared_context.bookmarks.write().unwrap().remove_folder(&folder_id) {
                    tracing::error!("Failed to delete folder: {}", e);
                } else {
                    if let Err(e) = shared_context.bookmarks.write().unwrap().save() {
                        tracing::error!("Failed to save after folder deletion: {}", e);
                    }
                    // Remove from expanded state if present
                    self.expanded_folders.remove(&folder_id);
                    tracing::info!("Deleted folder: {}", folder_id);
                }
            }

            // Handle folder rename - populate folder dialog and show it
            if let Some(folder_id) = folder_to_rename {
                let folder = shared_context
                    .bookmarks
                    .read()
                    .unwrap()
                    .get_folder(&folder_id)
                    .cloned();

                if let Some(folder) = folder {
                    self.editing_folder_id = Some(folder.id.clone());
                    self.folder_dialog_name = folder.name.clone();
                    self.folder_dialog_parent_id = folder.parent_id.clone();
                    self.show_folder_dialog = true;
                    tracing::info!("Opening rename dialog for folder: {}", folder.name);
                }
            }

            // Handle bookmark paste (move or copy)
            if let Some((bookmark_id, target_folder_id)) = bookmark_to_paste {
                // Get the bookmark to check if we need to copy
                let bookmark = shared_context
                    .bookmarks
                    .read()
                    .unwrap()
                    .get_bookmark(&bookmark_id)
                    .cloned();

                if let Some(bookmark) = bookmark {
                    if self.bookmark_clipboard_is_cut {
                        // Move operation - just update folder_id
                        shared_context
                            .bookmarks
                            .write()
                            .unwrap()
                            .move_bookmark_to_folder(&bookmark_id, target_folder_id.clone());

                        tracing::info!(
                            "Moved bookmark '{}' to folder: {:?}",
                            bookmark.name,
                            target_folder_id
                        );

                        // Clear clipboard after cut operation
                        self.bookmark_clipboard = None;
                        self.bookmark_clipboard_is_cut = false;
                    } else {
                        // Copy operation - create new bookmark with same data
                        let mut new_bookmark = bookmark.clone();
                        new_bookmark.id = uuid::Uuid::new_v4().to_string();
                        new_bookmark.folder_id = target_folder_id.clone();
                        new_bookmark.created_at = chrono::Utc::now();
                        new_bookmark.modified_at = chrono::Utc::now();

                        shared_context.bookmarks.write().unwrap().add_bookmark(new_bookmark);

                        tracing::info!(
                            "Copied bookmark '{}' to folder: {:?}",
                            bookmark.name,
                            target_folder_id
                        );
                    }

                    if let Err(e) = shared_context.bookmarks.write().unwrap().save() {
                        tracing::error!("Failed to save after bookmark move/copy: {}", e);
                    }
                }
            }

            // Handle folder move (drag-drop)
            if let Some((folder_id, new_parent_id)) = folder_to_move {
                if let Err(e) = shared_context
                    .bookmarks
                    .write()
                    .unwrap()
                    .move_folder_to_parent(&folder_id, new_parent_id.clone())
                {
                    tracing::error!("Failed to move folder: {}", e);
                } else {
                    if let Err(e) = shared_context.bookmarks.write().unwrap().save() {
                        tracing::error!("Failed to save after folder move: {}", e);
                    }

                    tracing::info!(
                        "Moved folder '{}' to parent: {:?}",
                        folder_id,
                        new_parent_id
                    );
                }
            }

            // Close the dialog in both panes if user closed it
            if !is_open {
                if let Ok(mut state) = self.left_pane.state.try_write() {
                    state.show_bookmark_manager = false;
                }
                if let Some(ref right_pane) = self.right_pane {
                    if let Ok(mut state) = right_pane.state.try_write() {
                        state.show_bookmark_manager = false;
                    }
                }
            }
        }
    }

    /// Render bookmark edit dialog
    ///
    /// Simple dialog for editing bookmark name and description.
    /// Based on design doc Phase 2.
    fn render_bookmark_edit_dialog(&mut self, ctx: &Context, shared_context: &ExplorerSharedContext) {
        if !self.show_bookmark_edit_dialog {
            return;
        }

        let mut should_save = false;
        let mut should_cancel = false;

        let response = Window::new("Edit Bookmark")
            .default_size([500.0, 200.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Edit bookmark name and description");
                    ui.add_space(10.0);

                    // Name input
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.bookmark_edit_name);
                    });

                    // Description input
                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_singleline(&mut self.bookmark_edit_description);
                    });

                    ui.add_space(10.0);

                    // Action buttons
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            should_save = true;
                        }

                        if ui.button("Cancel").clicked() {
                            should_cancel = true;
                        }
                    });
                });
            });

        // Handle save operation
        if should_save && !self.bookmark_edit_name.is_empty() {
            if let Some(bookmark_id) = &self.editing_bookmark_id {
                // Get mutable reference to bookmark and update it
                if let Some(bookmark) = shared_context
                    .bookmarks
                    .write()
                    .unwrap()
                    .get_bookmark_mut(bookmark_id)
                {
                    bookmark.name = self.bookmark_edit_name.clone();
                    bookmark.description = if self.bookmark_edit_description.is_empty() {
                        None
                    } else {
                        Some(self.bookmark_edit_description.clone())
                    };
                    bookmark.modified_at = chrono::Utc::now();

                    tracing::info!("Updated bookmark: {}", bookmark.name);
                }

                // Save bookmarks to disk
                if let Err(e) = shared_context.bookmarks.write().unwrap().save() {
                    tracing::error!("Failed to save bookmarks after edit: {}", e);
                }
            }

            // Close dialog and clear state
            self.show_bookmark_edit_dialog = false;
            self.editing_bookmark_id = None;
            self.bookmark_edit_name.clear();
            self.bookmark_edit_description.clear();
        }

        // Handle cancel operation
        if should_cancel {
            self.show_bookmark_edit_dialog = false;
            self.editing_bookmark_id = None;
            self.bookmark_edit_name.clear();
            self.bookmark_edit_description.clear();
        }

        // Handle window close via X button
        if response.is_none() {
            self.show_bookmark_edit_dialog = false;
            self.editing_bookmark_id = None;
            self.bookmark_edit_name.clear();
            self.bookmark_edit_description.clear();
        }
    }

    /// Render folder creation/edit dialog
    ///
    /// Allows creating new folders or editing existing ones with parent folder selection.
    /// Prevents circular references (folder can't be its own parent).
    /// Based on design doc Phase 3.
    fn render_folder_dialog(&mut self, ctx: &Context, shared_context: &ExplorerSharedContext) {
        if !self.show_folder_dialog {
            return;
        }

        let mut should_save = false;
        let mut should_cancel = false;
        let is_editing = self.editing_folder_id.is_some();

        let title = if is_editing {
            "Edit Folder"
        } else {
            "New Folder"
        };

        let response = Window::new(title)
            .default_size([400.0, 250.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Configure folder settings");
                    ui.add_space(10.0);

                    // Name input
                    ui.horizontal(|ui| {
                        ui.label("Folder name:");
                        ui.text_edit_singleline(&mut self.folder_dialog_name);
                    });

                    ui.add_space(10.0);

                    // Parent folder selection
                    ui.label("Parent folder:");
                    let current_parent_name = if let Some(parent_id) = &self.folder_dialog_parent_id
                    {
                        shared_context
                            .bookmarks
                            .read()
                            .unwrap()
                            .get_folder(parent_id)
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| "Top Folder".to_string())
                    } else {
                        "Top Folder".to_string()
                    };

                    egui::ComboBox::from_label("")
                        .selected_text(current_parent_name)
                        .show_ui(ui, |ui| {
                            // Option: Top Folder (no parent)
                            if ui
                                .selectable_label(
                                    self.folder_dialog_parent_id.is_none(),
                                    "Top Folder",
                                )
                                .clicked()
                            {
                                self.folder_dialog_parent_id = None;
                            }

                            // Options: All existing folders (except self if editing)
                            for folder in shared_context
                                .bookmarks
                                .read()
                                .unwrap()
                                .get_all_folders()
                                .iter()
                            {
                                // Don't allow selecting self as parent (circular reference)
                                if self.editing_folder_id.as_ref() != Some(&folder.id) {
                                    let is_selected =
                                        self.folder_dialog_parent_id.as_ref() == Some(&folder.id);
                                    if ui.selectable_label(is_selected, &folder.name).clicked() {
                                        self.folder_dialog_parent_id = Some(folder.id.clone());
                                    }
                                }
                            }
                        });

                    ui.add_space(10.0);

                    // Action buttons
                    ui.horizontal(|ui| {
                        let button_text = if is_editing { "Update" } else { "Create" };
                        if ui
                            .button(button_text)
                            .clicked()
                            && !self.folder_dialog_name.is_empty()
                        {
                            should_save = true;
                        }

                        if ui.button("Cancel").clicked() {
                            should_cancel = true;
                        }
                    });
                });
            });

        // Handle save operation
        if should_save {
            if let Some(editing_id) = &self.editing_folder_id {
                // Update existing folder
                if let Some(folder) = shared_context
                    .bookmarks
                    .write()
                    .unwrap()
                    .get_folder_mut(editing_id)
                {
                    folder.name = self.folder_dialog_name.clone();
                    folder.parent_id = self.folder_dialog_parent_id.clone();
                    folder.modified_at = chrono::Utc::now();

                    tracing::info!("Updated folder: {}", folder.name);
                }

                // Save to disk
                if let Err(e) = shared_context.bookmarks.write().unwrap().save() {
                    tracing::error!("Failed to save folder update: {}", e);
                }
            } else {
                // Create new folder
                let folder = BookmarkFolder::new(
                    self.folder_dialog_name.clone(),
                    self.folder_dialog_parent_id.clone(),
                );

                tracing::info!("Created folder: {}", folder.name);
                shared_context.bookmarks.write().unwrap().add_folder(folder);

                // Save to disk
                if let Err(e) = shared_context.bookmarks.write().unwrap().save() {
                    tracing::error!("Failed to save new folder: {}", e);
                }
            }

            // Close dialog and clear state
            self.show_folder_dialog = false;
            self.editing_folder_id = None;
            self.folder_dialog_name.clear();
            self.folder_dialog_parent_id = None;
        }

        // Handle cancel operation
        if should_cancel {
            self.show_folder_dialog = false;
            self.editing_folder_id = None;
            self.folder_dialog_name.clear();
            self.folder_dialog_parent_id = None;
        }

        // Handle window close via X button
        if response.is_none() {
            self.show_folder_dialog = false;
            self.editing_folder_id = None;
            self.folder_dialog_name.clear();
            self.folder_dialog_parent_id = None;
        }
    }

    /// Helper method to check if moving a folder would create a circular reference
    /// Returns true if the target folder is a descendant of the source folder
    fn would_create_circular_reference(
        &self,
        source_folder_id: &str,
        target_parent_id: Option<&str>,
        shared_context: &ExplorerSharedContext,
    ) -> bool {
        if target_parent_id.is_none() {
            return false; // Moving to top level is always safe
        }

        let target_parent_id = target_parent_id.unwrap();

        // If trying to move folder to itself, that's circular
        if source_folder_id == target_parent_id {
            return true;
        }

        // Check if target is a descendant of source by walking up the parent chain
        let mut current_id = Some(target_parent_id.to_string());
        while let Some(id) = current_id {
            if id == source_folder_id {
                return true; // Found source in parent chain - circular!
            }

            // Move to next parent
            current_id = shared_context
                .bookmarks
                .read()
                .unwrap()
                .get_folder(&id)
                .and_then(|f| f.parent_id.clone());
        }

        false
    }

    /// Recursively renders folder hierarchy and bookmarks at a specific level
    /// This is the core of the hierarchical bookmark manager tree view
    #[allow(clippy::too_many_arguments)]
    fn render_folder_tree_level(
        &mut self,
        ui: &mut Ui,
        parent_id: Option<&String>,
        shared_context: &ExplorerSharedContext,
        bookmark_to_delete: &mut Option<String>,
        bookmark_to_edit: &mut Option<String>,
        folder_to_delete: &mut Option<String>,
        folder_to_rename: &mut Option<String>,
        bookmark_to_paste: &mut Option<(String, Option<String>)>,
        folder_to_move: &mut Option<(String, Option<String>)>,
    ) {
        // Get all folders at this level
        let folders: Vec<_> = shared_context
            .bookmarks
            .read()
            .unwrap()
            .get_subfolders(parent_id)
            .iter()
            .map(|f| (f.id.clone(), f.name.clone()))
            .collect();

        // Render each folder with collapsing header
        for (folder_id, folder_name) in folders {
            let is_expanded = self.expanded_folders.contains(&folder_id);

            // Horizontal layout for entire folder row (creates clear drop zone)
            let row_response = ui.horizontal(|ui| {
                // Drag handle - only this small area is draggable
                let drag_data = DragData::Folder {
                    id: folder_id.clone(),
                    parent_id: parent_id.cloned(),
                };

                let drag_id = ui.make_persistent_id(format!("folder_drag_{}", folder_id));
                let _drag_response = ui.dnd_drag_source(drag_id, drag_data, |ui| {
                    ui.label(":: ");
                }).response;

                // Folder header - this stays interactive (collapse arrow works)
                let header_response = egui::CollapsingHeader::new(&folder_name)
                    .id_salt(&folder_id)
                    .default_open(is_expanded)
                    .show(ui, |ui| {
                        // Recurse into subfolders and bookmarks
                        self.render_folder_tree_level(
                            ui,
                            Some(&folder_id),
                            shared_context,
                            bookmark_to_delete,
                            bookmark_to_edit,
                            folder_to_delete,
                            folder_to_rename,
                            bookmark_to_paste,
                            folder_to_move,
                        );
                    });

                // Track expanded state
                if header_response.header_response.clicked() {
                    if is_expanded {
                        self.expanded_folders.remove(&folder_id);
                    } else {
                        self.expanded_folders.insert(folder_id.clone());
                    }
                }

                // Return header response for context menu
                header_response.header_response
            });

            // Check if something is being dragged over this folder row
            if let Some(dragged_data) = row_response.response.dnd_hover_payload::<DragData>() {
                let can_drop = match dragged_data.as_ref() {
                    DragData::Bookmark { source_folder, .. } => {
                        // Don't allow dropping bookmark on its own folder
                        source_folder.as_ref() != Some(&folder_id)
                    }
                    DragData::Folder { id: dragged_folder_id, .. } => {
                        // Don't allow dropping folder on itself and prevent circular references
                        dragged_folder_id != &folder_id
                            && !self.would_create_circular_reference(
                                dragged_folder_id,
                                Some(&folder_id),
                                shared_context,
                            )
                    }
                };

                if can_drop {
                    // Visual feedback: highlight entire folder row
                    let painter = ui.painter();
                    painter.rect_stroke(
                        row_response.response.rect,
                        3.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)),
                        egui::epaint::StrokeKind::Outside,
                    );
                }
            }

            // Handle drop on folder row
            if let Some(dropped) = row_response.response.dnd_release_payload::<DragData>() {
                match dropped.as_ref() {
                    DragData::Bookmark { id, source_folder } => {
                        // Don't drop bookmark on its own folder
                        if source_folder.as_ref() != Some(&folder_id) {
                            *bookmark_to_paste = Some((id.clone(), Some(folder_id.clone())));
                            self.bookmark_clipboard_is_cut = true; // Drag-drop always moves
                        }
                    }
                    DragData::Folder { id: dragged_folder_id, .. } => {
                        // Don't drop folder on itself and prevent circular references
                        if dragged_folder_id != &folder_id
                            && !self.would_create_circular_reference(
                                dragged_folder_id,
                                Some(&folder_id),
                                shared_context,
                            )
                        {
                            *folder_to_move = Some((dragged_folder_id.clone(), Some(folder_id.clone())));
                        } else {
                            tracing::warn!(
                                "Prevented circular folder move: {} -> {}",
                                dragged_folder_id,
                                folder_id
                            );
                        }
                    }
                }
            }

            // Context menu for folder (using inner response from horizontal)
            row_response.inner.context_menu(|ui| {
                if ui.button("Rename Folder").clicked() {
                    *folder_to_rename = Some(folder_id.clone());
                    ui.close();
                }

                if ui.button("Delete Folder").clicked() {
                    *folder_to_delete = Some(folder_id.clone());
                    ui.close();
                }

                ui.separator();

                // Paste bookmark into this folder
                if let Some(ref clipboard_id) = self.bookmark_clipboard {
                    let action = if self.bookmark_clipboard_is_cut {
                        "Move Here"
                    } else {
                        "Copy Here"
                    };

                    if ui.button(action).clicked() {
                        *bookmark_to_paste = Some((clipboard_id.clone(), Some(folder_id.clone())));
                        ui.close();
                    }
                }
            });
        }

        // Get bookmarks at this level - pre-fetch ALL data to avoid locks in rendering
        let all_bookmarks = shared_context.bookmarks.read().unwrap();
        let bookmarks_data: Vec<_> = all_bookmarks
            .get_bookmarks_in_folder(parent_id)
            .iter()
            .map(|b| {
                (
                    b.id.clone(),
                    b.name.clone(),
                    b.account_ids.len(),
                    b.region_codes.len(),
                    b.resource_type_ids.len(),
                    b.folder_id.clone(),
                )
            })
            .collect();
        drop(all_bookmarks); // Release lock immediately

        // Simple rendering without egui_dnd (keep it simple to avoid conflicts)
        for (bookmark_id, name, account_count, region_count, type_count, folder_id) in bookmarks_data {
            let response = ui.horizontal(|ui| {
                // Drag handle for cross-folder drag-drop
                let drag_data = DragData::Bookmark {
                    id: bookmark_id.clone(),
                    source_folder: folder_id.clone(),
                };
                let drag_id = ui.make_persistent_id(format!("bookmark_drag_{}", bookmark_id));
                ui.dnd_drag_source(drag_id, drag_data, |ui| {
                    ui.label(":: ");
                });

                ui.label(&name);
                ui.label(format!(
                    "({} accounts, {} regions, {} types)",
                    account_count, region_count, type_count
                ));
            });

            // Context menu for bookmark
            response.response.context_menu(|ui| {
                if ui.button("Copy").clicked() {
                    self.bookmark_clipboard = Some(bookmark_id.clone());
                    self.bookmark_clipboard_is_cut = false;
                    ui.close();
                }

                if ui.button("Cut").clicked() {
                    self.bookmark_clipboard = Some(bookmark_id.clone());
                    self.bookmark_clipboard_is_cut = true;
                    ui.close();
                }

                ui.separator();

                if ui.button("Edit").clicked() {
                    *bookmark_to_edit = Some(bookmark_id.clone());
                    ui.close();
                }

                if ui.button("Delete").clicked() {
                    *bookmark_to_delete = Some(bookmark_id.clone());
                    ui.close();
                }
            });
        }
    }
}

// ============================================================================
// Pane Action Handling
// ============================================================================

impl ExplorerInstance {
    /// Handle actions from pane rendering (bookmarks, dialogs, etc.)
    fn handle_pane_action(
        &mut self,
        action: PaneAction,
        ctx: &Context,
        shared_context: &ExplorerSharedContext,
    ) {
        match action {
            PaneAction::ApplyBookmark { bookmark_id, source_pane_id } => {
                // Find the bookmark
                let bookmark_clone = shared_context
                    .bookmarks
                    .read()
                    .unwrap()
                    .get_bookmarks()
                    .iter()
                    .find(|b| b.id == bookmark_id)
                    .cloned();

                if let Some(bookmark) = bookmark_clone {
                    // Determine which pane to apply the bookmark to based on source_pane_id
                    let target_pane = if self.left_pane.id() == source_pane_id {
                        &mut self.left_pane
                    } else if let Some(ref mut right) = self.right_pane {
                        if right.id() == source_pane_id {
                            right
                        } else {
                            // Fallback to left pane if source not found
                            tracing::warn!("Source pane {} not found, using left pane", source_pane_id);
                            &mut self.left_pane
                        }
                    } else {
                        // No right pane exists, use left pane
                        &mut self.left_pane
                    };

                    tracing::info!(
                        "Applying bookmark '{}' to pane {} (source: {})",
                        bookmark.name,
                        target_pane.id(),
                        source_pane_id
                    );

                    Self::apply_bookmark_to_pane(target_pane, &bookmark, ctx, shared_context);

                    // Update access tracking
                    if let Some(bookmark_mut) = shared_context
                        .bookmarks
                        .write()
                        .unwrap()
                        .get_bookmark_mut(&bookmark_id)
                    {
                        bookmark_mut.access_count += 1;
                        bookmark_mut.last_accessed = Some(chrono::Utc::now());
                        bookmark_mut.modified_at = chrono::Utc::now();
                    }

                    // Save updated bookmark
                    if let Err(e) = shared_context.bookmarks.write().unwrap().save() {
                        tracing::error!("Failed to save bookmark access tracking: {}", e);
                    }
                } else {
                    tracing::warn!("Bookmark not found: {}", bookmark_id);
                }
            }
            PaneAction::RemoveAccount { account_id, source_pane_id } => {
                // Validate pane ID and get target pane (strict validation - reject if not found)
                let target_pane = if self.left_pane.id() == source_pane_id {
                    Some(&mut self.left_pane)
                } else if let Some(ref mut right) = self.right_pane {
                    if right.id() == source_pane_id {
                        Some(right)
                    } else {
                        None // Pane ID doesn't match - reject action
                    }
                } else {
                    None // Right pane doesn't exist and source isn't left pane
                };

                if let Some(target_pane) = target_pane {
                    tracing::info!(
                        "Removing account {} from pane {} (source: {})",
                        redact_sensitive(&account_id),
                        target_pane.id(),
                        source_pane_id
                    );

                    if let Ok(mut state) = target_pane.state.try_write() {
                        let was_phase2_running = state.phase2_enrichment_in_progress;
                        state.remove_account(&account_id);
                        Self::handle_active_selection_reduction(&mut state);

                        if was_phase2_running {
                            // TODO: Maybe restart Phase 2 enrichment
                        }
                    }
                } else {
                    tracing::warn!(
                        "Rejecting RemoveAccount action - source pane {} not found (account: {})",
                        source_pane_id,
                        redact_sensitive(&account_id)
                    );
                }
            }
            PaneAction::RemoveRegion { region_code, source_pane_id } => {
                // Validate pane ID and get target pane (strict validation - reject if not found)
                let target_pane = if self.left_pane.id() == source_pane_id {
                    Some(&mut self.left_pane)
                } else if let Some(ref mut right) = self.right_pane {
                    if right.id() == source_pane_id {
                        Some(right)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(target_pane) = target_pane {
                    tracing::info!(
                        "Removing region {} from pane {} (source: {})",
                        region_code,
                        target_pane.id(),
                        source_pane_id
                    );

                    if let Ok(mut state) = target_pane.state.try_write() {
                        let was_phase2_running = state.phase2_enrichment_in_progress;
                        state.remove_region(&region_code);
                        Self::handle_active_selection_reduction(&mut state);

                        if was_phase2_running {
                            // TODO: Maybe restart Phase 2 enrichment
                        }
                    }
                } else {
                    tracing::warn!(
                        "Rejecting RemoveRegion action - source pane {} not found (region: {})",
                        source_pane_id,
                        region_code
                    );
                }
            }
            PaneAction::RemoveResourceType { resource_type, source_pane_id } => {
                // Validate pane ID and get target pane (strict validation - reject if not found)
                let target_pane = if self.left_pane.id() == source_pane_id {
                    Some(&mut self.left_pane)
                } else if let Some(ref mut right) = self.right_pane {
                    if right.id() == source_pane_id {
                        Some(right)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(target_pane) = target_pane {
                    tracing::info!(
                        "Removing resource type {} from pane {} (source: {})",
                        resource_type,
                        target_pane.id(),
                        source_pane_id
                    );

                    if let Ok(mut state) = target_pane.state.try_write() {
                        let was_phase2_running = state.phase2_enrichment_in_progress;
                        state.remove_resource_type(&resource_type);
                        Self::handle_active_selection_reduction(&mut state);

                        if was_phase2_running {
                            // TODO: Maybe restart Phase 2 enrichment
                        }
                    }
                } else {
                    tracing::warn!(
                        "Rejecting RemoveResourceType action - source pane {} not found (type: {})",
                        source_pane_id,
                        resource_type
                    );
                }
            }
            PaneAction::ShowTagFilterBuilder => {
                // TODO: Track which pane the action came from
                // For now, apply to left pane
                if let Ok(mut state) = self.left_pane.state.try_write() {
                    state.show_filter_builder = true;
                }
            }
            PaneAction::ShowPropertyFilterBuilder => {
                // TODO: Track which pane the action came from
                // For now, apply to left pane
                if let Ok(mut state) = self.left_pane.state.try_write() {
                    state.show_property_filter_builder = true;
                }
            }
            PaneAction::ShowTagHierarchyBuilder => {
                // TODO: Track which pane the action came from
                // For now, apply to left pane
                if let Ok(mut state) = self.left_pane.state.try_write() {
                    state.show_tag_hierarchy_builder = true;
                }
            }
            PaneAction::ShowPropertyHierarchyBuilder => {
                // TODO: Track which pane the action came from
                // For now, apply to left pane
                if let Ok(mut state) = self.left_pane.state.try_write() {
                    state.show_property_hierarchy_builder = true;
                }
            }
            PaneAction::ShowFailedQueriesDialog => {
                // TODO: Implement failed queries dialog for multi-pane architecture
                // For now, log a warning
                tracing::warn!("Failed queries dialog not yet implemented for multi-pane architecture");
            }
        }
    }

    /// Handle active selection reduction (when tags are removed)
    ///
    /// Based on window.rs handle_active_selection_reduction (lines 4112-4123)
    fn handle_active_selection_reduction(state: &mut crate::app::resource_explorer::state::ResourceExplorerState) {
        // Cancel Phase 2 enrichment
        state.cancel_phase2_enrichment();

        // Filter resources by current scope (remove resources that don't match anymore)
        state.resources.retain(|resource| {
            Self::resource_matches_scope(resource, &state.query_scope)
        });

        tracing::info!("Filtered resources after tag removal: {} remaining", state.resources.len());
    }

    /// Check if a resource matches the current query scope
    ///
    /// Based on window.rs resource_matches_scope (lines 4125-4155)
    fn resource_matches_scope(
        resource: &crate::app::resource_explorer::state::ResourceEntry,
        scope: &crate::app::resource_explorer::state::QueryScope,
    ) -> bool {
        let account_matches = scope
            .accounts
            .iter()
            .any(|a| a.account_id == resource.account_id);

        // True global resources (IAM, Route53, etc.) match any region in the scope.
        // S3 buckets are hybrid-global: queried globally but have actual regions.
        let is_true_global_resource = resource.region == "Global"
            && resource.resource_type != "AWS::S3::Bucket";

        let region_matches = is_true_global_resource
            || scope
                .regions
                .iter()
                .any(|r| r.region_code == resource.region);

        let resource_type_matches = scope
            .resource_types
            .iter()
            .any(|rt| rt.resource_type == resource.resource_type);

        account_matches && region_matches && resource_type_matches
    }

    /// Apply a bookmark to a specific pane
    fn apply_bookmark_to_pane(
        pane: &mut ExplorerPane,
        bookmark: &crate::app::resource_explorer::bookmarks::Bookmark,
        _ctx: &Context,
        shared_context: &ExplorerSharedContext,
    ) {
        use crate::app::resource_explorer::dialogs::get_default_resource_types;
        use crate::app::resource_explorer::state::{AccountSelection, RegionSelection};

        let should_trigger_query = if let Ok(mut state) = pane.state.try_write() {
            // Reset Phase 2 state from any previous bookmark
            state.cancel_phase2_enrichment();

            // Clear existing query scope
            state.query_scope.accounts.clear();
            state.query_scope.regions.clear();
            state.query_scope.resource_types.clear();

            // Rebuild AccountSelection objects from stored account IDs
            let available_accounts = if let Some(ref identity_center) = shared_context.aws_identity_center {
                if let Ok(ic) = identity_center.lock() {
                    ic.accounts.clone()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            for account_id in &bookmark.account_ids {
                if let Some(aws_account) = available_accounts
                    .iter()
                    .find(|a| &a.account_id == account_id)
                {
                    let account_sel = AccountSelection::new(
                        account_id.clone(),
                        aws_account.account_name.clone(),
                    );
                    state.add_account(account_sel);
                    tracing::debug!(
                        "  OK: Restored account: {} ({})",
                        aws_account.account_name,
                        account_id
                    );
                } else {
                    tracing::warn!(
                        "  WARN: Account {} not found in available accounts, skipping",
                        account_id
                    );
                }
            }

            // Rebuild RegionSelection objects from stored region codes
            for region_code in &bookmark.region_codes {
                let display_name = Self::format_region_display_name(region_code);
                let region_sel = RegionSelection::new(region_code.clone(), display_name.clone());
                state.add_region(region_sel);
                tracing::debug!("  OK: Restored region: {} ({})", display_name, region_code);
            }

            // Rebuild ResourceTypeSelection objects from stored resource type IDs
            let available_types = get_default_resource_types();
            for resource_type_id in &bookmark.resource_type_ids {
                if let Some(res_type) = available_types
                    .iter()
                    .find(|rt| &rt.resource_type == resource_type_id)
                {
                    state.add_resource_type(res_type.clone());
                    tracing::debug!(
                        "  OK: Restored resource type: {} ({})",
                        res_type.display_name,
                        resource_type_id
                    );
                } else {
                    tracing::warn!(
                        "  WARN: Resource type {} not found in available types, skipping",
                        resource_type_id
                    );
                }
            }

            // Apply other state components
            state.primary_grouping = bookmark.grouping.clone();
            state.tag_filter_group = bookmark.tag_filters.clone();
            state.search_filter = bookmark.search_filter.clone();

            tracing::info!(
                "  → Restored: {} accounts, {} regions, {} resource types, grouping: {:?}",
                state.query_scope.accounts.len(),
                state.query_scope.regions.len(),
                state.query_scope.resource_types.len(),
                state.primary_grouping
            );

            // Check if we should trigger query with restored scope
            !state.query_scope.accounts.is_empty()
                && !state.query_scope.regions.is_empty()
                && !state.query_scope.resource_types.is_empty()
        } else {
            tracing::warn!("Could not acquire write lock on pane state to apply bookmark");
            false
        };

        // Trigger query after releasing the lock
        if should_trigger_query {
            tracing::info!("  → Triggering query for restored bookmark");
            pane.mark_pending_query();
        }
    }

    /// Format region code into human-readable display name
    fn format_region_display_name(region_code: &str) -> String {
        // Special case for global
        if region_code == "Global" || region_code == "global" {
            return "Global".to_string();
        }

        // Parse AWS region code format: us-east-1 → US East (N. Virginia)
        let parts: Vec<&str> = region_code.split('-').collect();
        if parts.len() >= 2 {
            let geo = match parts[0] {
                "us" => "US",
                "eu" => "EU",
                "ap" => "Asia Pacific",
                "sa" => "South America",
                "ca" => "Canada",
                "me" => "Middle East",
                "af" => "Africa",
                _ => parts[0],
            };

            let direction = match parts[1] {
                "east" => "East",
                "west" => "West",
                "north" => "North",
                "south" => "South",
                "central" => "Central",
                "northeast" => "Northeast",
                "southeast" => "Southeast",
                _ => parts[1],
            };

            format!("{} {} ({})", geo, direction, region_code)
        } else {
            region_code.to_string()
        }
    }
}
