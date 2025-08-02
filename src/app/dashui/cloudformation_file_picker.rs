use crate::app::dashui::app::fuzzy_match_score;
use eframe::egui;
use egui::{Color32, Context, Key, RichText, Window};
use std::path::PathBuf;

/// Status of the CloudFormation file picker
#[derive(PartialEq)]
pub enum CloudFormationFilePickerStatus {
    /// The picker is open and waiting for input
    Open,
    /// The picker was closed
    Closed,
    /// A file was selected
    Selected(PathBuf),
}

/// A file picker that uses fuzzy search to navigate directories and select CloudFormation templates
pub struct CloudFormationFilePicker {
    /// Current status of the picker
    pub status: CloudFormationFilePickerStatus,

    /// Current directory being browsed
    current_dir: PathBuf,

    /// Current search query
    query: String,

    /// Current path being built (as user selects directories)
    current_path: Vec<String>,

    /// Currently filtered entries in the current directory
    filtered_entries: Vec<(String, bool)>, // (name, is_dir)

    /// Currently selected entry index
    selected_index: Option<usize>,

    /// Error message, if any
    error_message: Option<String>,
}

impl Default for CloudFormationFilePicker {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudFormationFilePicker {
    /// Create a new CloudFormation file picker starting in the user's home directory
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut picker = Self {
            status: CloudFormationFilePickerStatus::Open,
            current_dir: home_dir,
            query: String::new(),
            current_path: Vec::new(),
            filtered_entries: Vec::new(),
            selected_index: None,
            error_message: None,
        };

        // Initial directory listing
        picker.update_entries();

        picker
    }

    /// Check if a file is a CloudFormation template
    fn is_cloudformation_file(name: &str) -> bool {
        name.to_lowercase().ends_with(".json")
            || name.to_lowercase().ends_with(".yaml")
            || name.to_lowercase().ends_with(".yml")
    }

    /// Update the filtered entries based on the current query
    fn update_entries(&mut self) {
        self.filtered_entries.clear();
        self.selected_index = None;

        // Try to read the current directory
        match std::fs::read_dir(&self.current_dir) {
            Ok(entries) => {
                // Collect all entries and sort them (directories first)
                let mut dirs = Vec::new();
                let mut files = Vec::new();

                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = path.is_dir();

                    // Skip hidden files and directories
                    if name.starts_with('.') {
                        continue;
                    }

                    // Apply fuzzy filtering
                    if self.query.is_empty() || Self::matches_query(&self.query, &name) {
                        if is_dir {
                            dirs.push((name, true));
                        } else if Self::is_cloudformation_file(&name) {
                            // Only show CloudFormation template files
                            files.push((name, false));
                        }
                    }
                }

                // Sort directories and files by name
                dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
                files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

                // Combine directories (first) and files
                self.filtered_entries.extend(dirs);
                self.filtered_entries.extend(files);

                // Select the first entry if available
                if !self.filtered_entries.is_empty() {
                    self.selected_index = Some(0);
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Error reading directory: {}", e));
            }
        }
    }

    /// Check if an entry matches the query using fuzzy matching
    fn matches_query(query: &str, name: &str) -> bool {
        fuzzy_match_score(query, name).is_some()
    }

    /// Accept the current selection (called when user presses Enter)
    /// For directories, this navigates into the directory
    /// For files, this selects the file
    fn accept_selection(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx < self.filtered_entries.len() {
                let (name, is_dir) = &self.filtered_entries[idx];

                if *is_dir {
                    // Navigate into the directory
                    let new_dir = self.current_dir.join(name);
                    if new_dir.exists() && new_dir.is_dir() {
                        self.current_dir = new_dir;
                        self.current_path.push(name.clone());
                        self.query = String::new();
                        self.update_entries();
                    } else {
                        self.error_message = Some(format!("Cannot access directory: {}", name));
                    }
                } else {
                    // Select the file
                    let file_path = self.current_dir.join(name);
                    self.status = CloudFormationFilePickerStatus::Selected(file_path);
                }
            }
        }
    }

    /// Navigate to the parent directory
    fn navigate_to_parent(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            if !self.current_path.is_empty() {
                self.current_path.pop();
            }
            self.query = String::new();
            self.update_entries();
        }
    }

    /// Show the CloudFormation file picker window
    pub fn show(&mut self, ctx: &Context) {
        if self.status != CloudFormationFilePickerStatus::Open {
            return;
        }

        // Request focus for the search field
        ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("cfn_fuzzy_search_field")));

        // Calculate dimensions
        let screen_rect = ctx.screen_rect();
        let window_width = screen_rect.width() * 0.6;
        let window_height = screen_rect.height() * 0.6;

        // Center the window on screen
        let window_pos = egui::Pos2::new(
            screen_rect.center().x - (window_width / 2.0),
            screen_rect.center().y - (window_height / 2.0),
        );

        Window::new("Select CloudFormation Template")
            .fixed_pos(window_pos)
            .fixed_size([window_width, window_height])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Show current path
                    ui.horizontal(|ui| {
                        ui.label("Current Path: ");
                        ui.label(RichText::new("~").strong());

                        // Show selected path components
                        for component in &self.current_path {
                            ui.label("/");
                            ui.label(RichText::new(component).strong());
                        }
                    });

                    ui.add_space(10.0);

                    // Error message if any
                    if let Some(error) = &self.error_message {
                        ui.colored_label(Color32::RED, error);
                        ui.add_space(10.0);
                    }

                    // Search field
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        let response = ui.add_sized(
                            [ui.available_width() - 100.0, ui.spacing().interact_size.y],
                            egui::TextEdit::singleline(&mut self.query)
                                .id(egui::Id::new("cfn_fuzzy_search_field")),
                        );

                        if response.changed() {
                            self.update_entries();
                        }
                    });

                    ui.add_space(5.0);

                    // Show help text
                    ui.label(
                        RichText::new(
                            "Looking for CloudFormation templates (*.json, *.yaml, *.yml)",
                        )
                        .weak(),
                    );
                    ui.add_space(5.0);

                    // Display entries in a scrollable list
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Show parent directory option if we're not at root
                        if self.current_dir.parent().is_some()
                            && ui
                                .selectable_label(
                                    self.selected_index.is_none()
                                        && self.filtered_entries.is_empty(),
                                    ".. (Parent Directory)",
                                )
                                .clicked()
                        {
                            self.navigate_to_parent();
                        }

                        // Display all filtered entries
                        let mut double_click_action = None;
                        for (idx, (name, is_dir)) in self.filtered_entries.iter().enumerate() {
                            let is_selected = self.selected_index == Some(idx);

                            let label_text = if *is_dir {
                                RichText::new(format!("📁 {}", name))
                                    .color(Color32::from_rgb(100, 170, 255))
                                    .strong()
                            } else {
                                let icon = if name.ends_with(".json") {
                                    "{ }"
                                } else {
                                    "📄"
                                };
                                RichText::new(format!("{} {}", icon, name))
                                    .color(Color32::from_rgb(150, 200, 150))
                            };

                            if ui.selectable_label(is_selected, label_text).clicked() {
                                self.selected_index = Some(idx);
                                // Double-click behavior for files
                                if !is_dir {
                                    double_click_action = Some(idx);
                                }
                            }
                        }

                        // Handle double-click action outside of the loop to avoid borrow conflicts
                        if double_click_action.is_some() {
                            self.accept_selection();
                        }
                    });

                    ui.add_space(10.0);

                    // Show controls help
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Enter: Select file / Navigate into folder").weak());
                        ui.label("|");
                        ui.label(RichText::new("←: Go up a level").weak());
                        ui.label("|");
                        ui.label(RichText::new("Esc: Cancel").weak());
                    });

                    ui.add_space(10.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            if ui.button("Cancel").clicked() {
                                self.status = CloudFormationFilePickerStatus::Closed;
                            }

                            if let Some(idx) = self.selected_index {
                                if idx < self.filtered_entries.len() {
                                    let (_name, is_dir) = &self.filtered_entries[idx];
                                    if !is_dir && ui.button("Select").clicked() {
                                        self.accept_selection();
                                    }
                                }
                            }
                        });
                    });
                });
            });

        // Handle keyboard input
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.status = CloudFormationFilePickerStatus::Closed;
        }

        // Handle Enter to navigate into directory or accept selection
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            self.accept_selection();
        }

        // Handle Left Arrow to go up one level in the directory tree
        if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) && self.query.is_empty() {
            self.navigate_to_parent();
        }

        // Handle arrow keys
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            if let Some(idx) = self.selected_index {
                if idx < self.filtered_entries.len() - 1 {
                    self.selected_index = Some(idx + 1);
                }
            } else if !self.filtered_entries.is_empty() {
                self.selected_index = Some(0);
            }
        }

        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            if let Some(idx) = self.selected_index {
                if idx > 0 {
                    self.selected_index = Some(idx - 1);
                }
            } else if !self.filtered_entries.is_empty() {
                self.selected_index = Some(self.filtered_entries.len() - 1);
            }
        }
    }
}
