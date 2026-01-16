//! VFS Browser Window
//!
//! A file browser for viewing the Virtual File System (VFS) associated with an agent.
//! Allows traversing directories, selecting files, and viewing/editing file contents.

#![warn(clippy::all, rust_2018_idioms)]

use eframe::egui;
use egui::{Color32, Context, RichText, ScrollArea, TextEdit, Ui};

use crate::app::agent_framework::vfs::{with_vfs, with_vfs_mut, VfsDirEntry};

/// State for the VFS browser window
pub struct VfsBrowserWindow {
    /// Whether the window is open
    open: bool,
    /// Current VFS ID being browsed
    vfs_id: Option<String>,
    /// Display name for the window title
    display_name: String,
    /// Current directory path
    current_path: String,
    /// Directory entries in current path
    entries: Vec<VfsDirEntry>,
    /// Currently selected file path (if any)
    selected_file: Option<String>,
    /// Content of the selected file (if it's a text file)
    file_content: Option<String>,
    /// Whether the file content has been modified
    is_modified: bool,
    /// Error message to display
    error_message: Option<String>,
    /// Navigation history for back button
    path_history: Vec<String>,
}

impl Default for VfsBrowserWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl VfsBrowserWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            vfs_id: None,
            display_name: String::new(),
            current_path: "/".to_string(),
            entries: Vec::new(),
            selected_file: None,
            file_content: None,
            is_modified: false,
            error_message: None,
            path_history: Vec::new(),
        }
    }

    /// Open the VFS browser for a specific VFS instance
    pub fn open_for_vfs(&mut self, vfs_id: String, display_name: String) {
        self.open = true;
        self.vfs_id = Some(vfs_id);
        self.display_name = display_name;
        self.current_path = "/".to_string();
        self.selected_file = None;
        self.file_content = None;
        self.is_modified = false;
        self.error_message = None;
        self.path_history.clear();
        self.refresh_entries();
    }

    /// Close the browser
    pub fn close(&mut self) {
        self.open = false;
        self.vfs_id = None;
        self.file_content = None;
        self.is_modified = false;
    }

    /// Check if the browser is open
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Refresh the directory listing
    fn refresh_entries(&mut self) {
        self.entries.clear();
        self.error_message = None;

        if let Some(ref vfs_id) = self.vfs_id {
            let current_path = self.current_path.clone();
            match with_vfs(vfs_id, |vfs| vfs.list_dir(&current_path)) {
                Some(Ok(entries)) => {
                    self.entries = entries;
                    // Sort: directories first, then by name
                    self.entries.sort_by(|a, b| {
                        match (a.is_directory, b.is_directory) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            _ => a.name.cmp(&b.name),
                        }
                    });
                }
                Some(Err(e)) => {
                    self.error_message = Some(format!("Error listing directory: {}", e));
                }
                None => {
                    self.error_message = Some("VFS not found".to_string());
                }
            }
        }
    }

    /// Navigate to a directory
    fn navigate_to(&mut self, path: String) {
        // Save current path to history before navigating
        self.path_history.push(self.current_path.clone());
        self.current_path = path;
        self.selected_file = None;
        self.file_content = None;
        self.is_modified = false;
        self.refresh_entries();
    }

    /// Navigate back in history
    fn navigate_back(&mut self) {
        if let Some(prev_path) = self.path_history.pop() {
            self.current_path = prev_path;
            self.selected_file = None;
            self.file_content = None;
            self.is_modified = false;
            self.refresh_entries();
        }
    }

    /// Navigate to parent directory
    fn navigate_up(&mut self) {
        if self.current_path != "/" {
            let parent = if let Some(pos) = self.current_path.rfind('/') {
                if pos == 0 {
                    "/".to_string()
                } else {
                    self.current_path[..pos].to_string()
                }
            } else {
                "/".to_string()
            };
            self.navigate_to(parent);
        }
    }

    /// Open a file for viewing/editing
    fn open_file(&mut self, file_path: String) {
        self.selected_file = Some(file_path.clone());
        self.is_modified = false;

        if let Some(ref vfs_id) = self.vfs_id {
            match with_vfs(vfs_id, |vfs| vfs.read_file(&file_path).map(|b| b.to_vec())) {
                Some(Ok(bytes)) => {
                    // Try to interpret as UTF-8 text
                    match String::from_utf8(bytes) {
                        Ok(text) => {
                            self.file_content = Some(text);
                            self.error_message = None;
                        }
                        Err(_) => {
                            self.file_content = None;
                            self.error_message = Some("Binary file - cannot display".to_string());
                        }
                    }
                }
                Some(Err(e)) => {
                    self.file_content = None;
                    self.error_message = Some(format!("Error reading file: {}", e));
                }
                None => {
                    self.file_content = None;
                    self.error_message = Some("VFS not found".to_string());
                }
            }
        }
    }

    /// Save the current file content
    fn save_file(&mut self) {
        if let (Some(ref vfs_id), Some(ref file_path), Some(ref content)) =
            (&self.vfs_id, &self.selected_file, &self.file_content)
        {
            let file_path = file_path.clone();
            let content = content.clone();
            match with_vfs_mut(vfs_id, |vfs| vfs.write_file(&file_path, content.as_bytes())) {
                Some(Ok(())) => {
                    self.is_modified = false;
                    self.error_message = None;
                }
                Some(Err(e)) => {
                    self.error_message = Some(format!("Error saving file: {}", e));
                }
                None => {
                    self.error_message = Some("VFS not found".to_string());
                }
            }
        }
    }

    /// Show the VFS browser window
    pub fn show(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        let mut is_open = self.open;

        egui::Window::new(format!("VFS Browser - {}", self.display_name))
            .open(&mut is_open)
            .default_size([700.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                self.render_content(ui);
            });

        self.open = is_open;
    }

    /// Render the main content
    fn render_content(&mut self, ui: &mut Ui) {
        // Toolbar
        self.render_toolbar(ui);
        ui.separator();

        // Error message if any
        if let Some(ref error) = self.error_message {
            ui.label(RichText::new(error).color(Color32::RED));
            ui.separator();
        }

        // Split view: file browser on left, editor on right
        egui::SidePanel::left("vfs_file_browser")
            .default_width(250.0)
            .min_width(150.0)
            .show_inside(ui, |ui| {
                self.render_file_browser(ui);
            });

        // Central panel for file content
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.render_file_editor(ui);
        });
    }

    /// Render the toolbar with navigation buttons
    fn render_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Back button
            let back_enabled = !self.path_history.is_empty();
            if ui.add_enabled(back_enabled, egui::Button::new("<")).clicked() {
                self.navigate_back();
            }

            // Up button
            let up_enabled = self.current_path != "/";
            if ui.add_enabled(up_enabled, egui::Button::new("Up")).clicked() {
                self.navigate_up();
            }

            // Refresh button
            if ui.button("Refresh").clicked() {
                self.refresh_entries();
            }

            ui.separator();

            // Current path display
            ui.label(RichText::new(&self.current_path).monospace());

            // Show VFS stats on the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(ref vfs_id) = self.vfs_id {
                    if let Some((total, max)) = with_vfs(vfs_id, |vfs| {
                        (vfs.total_size(), vfs.max_size())
                    }) {
                        let usage_pct = (total as f64 / max as f64 * 100.0) as u32;
                        let total_kb = total / 1024;
                        let max_mb = max / (1024 * 1024);
                        ui.label(
                            RichText::new(format!("{}KB / {}MB ({}%)", total_kb, max_mb, usage_pct))
                                .small()
                                .color(Color32::GRAY),
                        );
                    }
                }
            });
        });
    }

    /// Render the file browser panel
    fn render_file_browser(&mut self, ui: &mut Ui) {
        ui.heading("Files");
        ui.separator();

        // Collect actions to avoid borrow issues
        let mut action: Option<FileAction> = None;

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.entries.is_empty() {
                    ui.label(RichText::new("(empty)").weak().italics());
                } else {
                    for entry in &self.entries {
                        let full_path = if self.current_path == "/" {
                            format!("/{}", entry.name)
                        } else {
                            format!("{}/{}", self.current_path, entry.name)
                        };

                        let is_selected = self.selected_file.as_ref() == Some(&full_path);

                        // Choose icon based on type
                        let icon = if entry.is_directory { "[D]" } else { "   " };
                        let label_text = format!("{} {}", icon, entry.name);

                        // Style based on selection
                        let text = if is_selected {
                            RichText::new(&label_text).strong()
                        } else if entry.is_directory {
                            RichText::new(&label_text).color(Color32::from_rgb(100, 150, 255))
                        } else {
                            RichText::new(&label_text)
                        };

                        let response = ui.selectable_label(is_selected, text);

                        if response.clicked() {
                            if entry.is_directory {
                                action = Some(FileAction::NavigateTo(full_path));
                            } else {
                                action = Some(FileAction::OpenFile(full_path));
                            }
                        }

                        // Show file size for files
                        if !entry.is_directory && entry.size > 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(30.0);
                                let size_str = if entry.size < 1024 {
                                    format!("{} B", entry.size)
                                } else {
                                    format!("{} KB", entry.size / 1024)
                                };
                                ui.label(RichText::new(size_str).small().weak());
                            });
                        }
                    }
                }
            });

        // Process action after iteration
        match action {
            Some(FileAction::NavigateTo(path)) => self.navigate_to(path),
            Some(FileAction::OpenFile(path)) => self.open_file(path),
            None => {}
        }
    }

    /// Render the file editor panel
    fn render_file_editor(&mut self, ui: &mut Ui) {
        if let Some(ref file_path) = self.selected_file.clone() {
            // File header
            ui.horizontal(|ui| {
                ui.heading(
                    file_path
                        .rsplit('/')
                        .next()
                        .unwrap_or(file_path),
                );

                if self.is_modified {
                    ui.label(RichText::new("(modified)").color(Color32::YELLOW));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Save button
                    let save_enabled = self.is_modified && self.file_content.is_some();
                    if ui.add_enabled(save_enabled, egui::Button::new("Save")).clicked() {
                        self.save_file();
                    }

                    // Close button
                    if ui.button("Close").clicked() {
                        self.selected_file = None;
                        self.file_content = None;
                        self.is_modified = false;
                    }
                });
            });

            ui.label(RichText::new(file_path).small().weak().monospace());
            ui.separator();

            // File content editor
            if let Some(ref mut content) = self.file_content {
                ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let response = TextEdit::multiline(content)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .show(ui);

                        if response.response.changed() {
                            self.is_modified = true;
                        }
                    });
            } else {
                ui.label(RichText::new("Cannot display file content").weak().italics());
            }
        } else {
            // No file selected
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new("Select a file to view/edit").weak().italics());
            });
        }
    }
}

/// Actions from file browser interaction
enum FileAction {
    NavigateTo(String),
    OpenFile(String),
}
