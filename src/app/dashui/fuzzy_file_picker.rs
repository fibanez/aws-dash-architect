use crate::app::dashui::app::fuzzy_match_score;
use eframe::egui;
use egui::{Color32, Context, Key, RichText, Window};
use std::path::PathBuf;

/// Status of the fuzzy file picker
#[derive(PartialEq)]
pub enum FuzzyFilePickerStatus {
    /// The picker is open and waiting for input
    Open,
    /// The picker was closed
    Closed,
    /// A path was selected
    Selected(PathBuf),
}

/// A file picker that uses fuzzy search to navigate directories
pub struct FuzzyFilePicker {
    /// Current status of the picker
    pub status: FuzzyFilePickerStatus,

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

    /// New folder name input
    new_folder_name: String,

    /// Whether the new folder dialog is open
    show_new_folder_dialog: bool,
}

impl Default for FuzzyFilePicker {
    fn default() -> Self {
        Self::new()
    }
}

impl FuzzyFilePicker {
    /// Create a new fuzzy file picker starting in the user's home directory
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let mut picker = Self {
            status: FuzzyFilePickerStatus::Open,
            current_dir: home_dir,
            query: String::new(),
            current_path: Vec::new(),
            filtered_entries: Vec::new(),
            selected_index: None,
            error_message: None,
            new_folder_name: String::new(),
            show_new_folder_dialog: false,
        };

        // Initial directory listing
        picker.update_entries();

        picker
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
                        } else {
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

    /// Select the current entry (this function is kept for compatibility but is no longer used)
    #[allow(dead_code)]
    fn select_current_entry(&mut self) {
        self.accept_selection();
    }

    /// Accept the current selection (called when user presses Enter)
    /// For directories, this navigates into the directory
    /// For files or if no selection, this selects the current directory
    fn accept_selection(&mut self) {
        // If a directory is selected, navigate into it
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

                        // Check for Project.json file
                        let project_file = self.current_dir.join("Project.json");
                        if project_file.exists() && project_file.is_file() {
                            // Found a Project.json file, select this directory
                            self.status = FuzzyFilePickerStatus::Selected(self.current_dir.clone());
                        }
                        return;
                    } else {
                        self.error_message = Some(format!("Cannot access directory: {}", name));
                    }
                }
            }
        }

        // If not a directory or no selection, accept the current directory
        self.status = FuzzyFilePickerStatus::Selected(self.current_dir.clone());
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

    /// Show the fuzzy file picker window
    pub fn show(&mut self, ctx: &Context) {
        if self.status != FuzzyFilePickerStatus::Open {
            return;
        }

        // Request focus for the search field only if the new folder dialog is not open
        if !self.show_new_folder_dialog {
            ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("fuzzy_search_field")));
        }

        // Calculate dimensions
        let screen_rect = ctx.screen_rect();
        let window_width = screen_rect.width() * 0.6;
        let window_height = screen_rect.height() * 0.6;

        // Center the window on screen
        let window_pos = egui::Pos2::new(
            screen_rect.center().x - (window_width / 2.0),
            screen_rect.center().y - (window_height / 2.0),
        );

        Window::new("Project Folder Selection")
            .fixed_pos(window_pos)
            .fixed_size([window_width, window_height])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Show current path
                    ui.horizontal(|ui| {
                        ui.label("Current Path: ");

                        // Show home directory label
                        // We don't use the actual home directory name as it can cause borrowing issues

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

                    // New Folder button above search
                    if ui.button("Create new folder").clicked() {
                        self.show_new_folder_dialog = true;
                        self.new_folder_name.clear();
                        // Request focus for the new folder name field on the next frame
                        ctx.memory_mut(|mem| {
                            mem.request_focus(egui::Id::new("new_folder_name_field"))
                        });
                    }

                    ui.add_space(10.0);

                    // Search field
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        let response = ui.add_sized(
                            [ui.available_width() - 100.0, ui.spacing().interact_size.y],
                            egui::TextEdit::singleline(&mut self.query)
                                .id(egui::Id::new("fuzzy_search_field")),
                        );

                        if response.changed() {
                            self.update_entries();
                        }
                    });

                    ui.add_space(5.0);

                    // Track if we need to navigate to a folder after the loop
                    let mut folder_to_navigate: Option<String> = None;
                    
                    // Display entries in a scrollable list
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Show parent directory option
                        if ui
                            .selectable_label(
                                self.selected_index == Some(0) && self.filtered_entries.is_empty(),
                                ".. (Parent Directory)",
                            )
                            .clicked()
                        {
                            self.navigate_to_parent();
                        }

                        // Display all filtered entries
                        for (idx, (name, is_dir)) in self.filtered_entries.iter().enumerate() {
                            let is_selected = self.selected_index == Some(idx);

                            let label_text = if *is_dir {
                                RichText::new(format!("📁 {}", name))
                                    .color(Color32::from_rgb(100, 170, 255))
                                    .strong()
                            } else {
                                RichText::new(format!("📄 {}", name))
                            };

                            if ui.selectable_label(is_selected, label_text).clicked() {
                                self.selected_index = Some(idx);
                                // If clicking on a directory, mark it for navigation
                                if *is_dir {
                                    folder_to_navigate = Some(name.clone());
                                }
                            }
                        }
                    });

                    // Handle folder navigation after the loop to avoid borrowing issues
                    if let Some(folder_name) = folder_to_navigate {
                        let new_dir = self.current_dir.join(&folder_name);
                        if new_dir.exists() && new_dir.is_dir() {
                            self.current_dir = new_dir;
                            self.current_path.push(folder_name);
                            self.query = String::new();
                            self.update_entries();

                            // Check for Project.json file
                            let project_file = self.current_dir.join("Project.json");
                            if project_file.exists() && project_file.is_file() {
                                // Found a Project.json file, select this directory
                                self.status = FuzzyFilePickerStatus::Selected(self.current_dir.clone());
                            }
                        } else {
                            self.error_message = Some(format!("Cannot access directory: {}", folder_name));
                        }
                    }

                    ui.add_space(10.0);

                    // Show controls help
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Enter: Navigate into folder or select").weak());
                        ui.label("|");
                        ui.label(RichText::new("←: Go up a level").weak());
                        ui.label("|");
                        ui.label(RichText::new("Ctrl+N: Create new folder").weak());
                        ui.label("|");
                        ui.label(RichText::new("Esc: Cancel").weak());
                    });

                    ui.add_space(10.0);

                    // Removed inline dialog - will be in separate window

                    // Buttons
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.status = FuzzyFilePickerStatus::Closed;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                            if ui.button("Accept").clicked() {
                                self.accept_selection();
                            }
                        });
                    });
                });
            });

        // Show New Folder Name window if dialog is open
        if self.show_new_folder_dialog {
            // Request focus for the text field
            ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("new_folder_name_field")));

            Window::new("New Folder Name")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Folder name:");
                            let response = ui.add_sized(
                                [200.0, ui.spacing().interact_size.y],
                                egui::TextEdit::singleline(&mut self.new_folder_name)
                                    .hint_text("Enter folder name")
                                    .id(egui::Id::new("new_folder_name_field")),
                            );

                            // Check for Enter key
                            if (response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                                || (response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)))
                            {
                                self.create_new_folder();
                            }
                        });

                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.show_new_folder_dialog = false;
                                self.new_folder_name.clear();
                            }

                            ui.add_space(10.0);

                            if ui.button("Create").clicked() {
                                self.create_new_folder();
                            }
                        });
                    });
                });
        }

        // Handle keyboard input
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            if self.show_new_folder_dialog {
                self.show_new_folder_dialog = false;
                self.new_folder_name.clear();
            } else {
                self.status = FuzzyFilePickerStatus::Closed;
            }
        }

        // Handle Ctrl+N to create new folder
        if ctx.input(|i| i.key_pressed(Key::N) && i.modifiers.ctrl) && !self.show_new_folder_dialog
        {
            self.show_new_folder_dialog = true;
            self.new_folder_name.clear();
            // Request focus for the new folder name field on the next frame
            ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("new_folder_name_field")));
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

    /// Create a new folder in the current directory
    fn create_new_folder(&mut self) {
        if self.new_folder_name.is_empty() {
            self.error_message = Some("Folder name cannot be empty".to_string());
            return;
        }

        // Remove any invalid characters from the folder name
        let sanitized_name = self
            .new_folder_name
            .replace("/", "")
            .replace("\\", "")
            .replace(":", "")
            .replace("*", "")
            .replace("?", "")
            .replace("\"", "")
            .replace("<", "")
            .replace(">", "")
            .replace("|", "");

        if sanitized_name.is_empty() {
            self.error_message = Some("Folder name contains only invalid characters".to_string());
            return;
        }

        let new_folder_path = self.current_dir.join(&sanitized_name);

        // Check if the folder already exists
        if new_folder_path.exists() {
            self.error_message = Some(format!("Folder '{}' already exists", sanitized_name));
            return;
        }

        // Create the folder
        match std::fs::create_dir(&new_folder_path) {
            Ok(_) => {
                self.show_new_folder_dialog = false;
                self.new_folder_name.clear();
                self.update_entries();

                // Navigate into the new folder
                let new_dir = self.current_dir.join(&sanitized_name);
                if new_dir.exists() && new_dir.is_dir() {
                    self.current_dir = new_dir;
                    self.current_path.push(sanitized_name);
                    self.query = String::new();
                    self.update_entries();
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to create folder: {}", e));
            }
        }
    }
}
