use crate::app::cf_syntax;
use crate::app::cfn_template::Resource;
use crate::app::dashui::app::ThemeChoice;
use crate::app::dashui::window_focus::{FocusableWindow, ThemeShowParams};
use eframe::egui::{self, Color32, RichText, ScrollArea, Window};
use egui_code_editor::{CodeEditor, ColorTheme};
use tracing::info;

/// Window for editing CloudFormation resource JSON
pub struct ResourceJsonEditorWindow {
    /// Whether the window is open
    pub show: bool,

    /// The resource ID being edited
    pub resource_id: String,

    /// The current resource being edited
    pub resource: Option<Resource>,

    /// JSON content for editing
    pub json_content: String,

    /// Error message if JSON is invalid
    pub error_message: Option<String>,

    /// Callback when resource is saved
    pub on_save: Option<Box<dyn FnMut(String, Resource) + 'static>>,

    /// Flag indicating that a save was requested
    pub save_requested: bool,

    /// The updated resource after save
    pub saved_resource: Option<Resource>,
}

#[allow(clippy::derivable_impls)]
impl Default for ResourceJsonEditorWindow {
    fn default() -> Self {
        Self {
            show: false,
            resource_id: String::new(),
            resource: None,
            json_content: String::new(),
            error_message: None,
            on_save: None,
            save_requested: false,
            saved_resource: None,
        }
    }
}

impl ResourceJsonEditorWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the editor for a specific resource
    pub fn open_for_resource(
        &mut self,
        resource_id: String,
        resource: Resource,
        on_save: impl FnMut(String, Resource) + 'static,
    ) {
        self.resource_id = resource_id;
        self.resource = Some(resource.clone());
        self.on_save = Some(Box::new(on_save));
        self.error_message = None;

        // Convert resource to pretty JSON
        match serde_json::to_string_pretty(&resource) {
            Ok(json) => self.json_content = json,
            Err(e) => {
                self.error_message = Some(format!("Failed to serialize resource: {}", e));
                self.json_content = String::new();
            }
        }

        self.show = true;
    }

    /// Show the editor window
    pub fn show(&mut self, ctx: &egui::Context, global_theme: ThemeChoice) {
        if !self.show {
            return;
        }

        let mut save_clicked = false;
        let mut close_window = false;

        // Calculate size based on screen, similar to chat window
        let screen_rect = ctx.screen_rect();
        let editor_height = screen_rect.height() * 0.8; // 80% of screen height
        let editor_width = screen_rect.width() * 0.6; // 60% of screen width for better readability

        Window::new(format!("JSON Editor: {}", self.resource_id))
            .open(&mut self.show)
            .min_width(600.0)
            .min_height(400.0)
            .default_width(editor_width.min(800.0))
            .default_height(editor_height.min(800.0))
            .max_height(screen_rect.height() * 0.9) // Prevent exceeding screen
            .resizable(true)
            .show(ctx, |ui| {
                // Set constraints to prevent unexpected growth
                ui.set_max_height(editor_height);
                // Error display
                if let Some(error) = &self.error_message {
                    ui.colored_label(Color32::from_rgb(220, 50, 50), error);
                    ui.separator();
                }

                // Calculate available height for editor, reserving space for buttons and error message
                let available_height = ui.available_height() - 80.0; // Reserve ~80px for buttons and separator

                // Constrain the editor area height
                ui.set_min_height(available_height);
                ui.set_max_height(available_height);

                // JSON editor
                ScrollArea::vertical()
                    .auto_shrink([false, false]) // Prevent auto-shrinking
                    .max_height(available_height)
                    .id_salt("json_editor_scroll") // Fixed ID to prevent resize issues
                    .show(ui, |ui| {
                        // Calculate rows based on available height (roughly 20px per row)
                        let rows = (available_height / 20.0).max(10.0) as usize;

                        // Use CodeEditor widget for JSON editing
                        // Set theme based on global theme
                        let code_theme = if global_theme == ThemeChoice::Latte {
                            ColorTheme::GITHUB_LIGHT
                        } else {
                            ColorTheme::GITHUB_DARK
                        };

                        // For debugging: try different syntax options
                        let syntax_to_use =
                            if std::env::var("AWS_DASH_DEBUG_SYNTAX").unwrap_or_default() == "rust"
                            {
                                cf_syntax::rust_syntax_for_comparison()
                            } else if std::env::var("AWS_DASH_DEBUG_SYNTAX").unwrap_or_default()
                                == "simple"
                            {
                                cf_syntax::simple_test_syntax()
                            } else {
                                cf_syntax::cloudformation_json_syntax()
                            };

                        CodeEditor::default()
                            .id_source("resource_json_editor")
                            .with_rows(rows)
                            .with_fontsize(14.0)
                            .with_theme(code_theme)
                            .with_syntax(syntax_to_use) // CloudFormation JSON syntax (or debug alternative)
                            .show(ui, &mut self.json_content);
                    });

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    // Save button
                    if ui.button(RichText::new("Save").size(16.0)).clicked() {
                        save_clicked = true;
                    }

                    // Cancel button
                    if ui.button(RichText::new("Cancel").size(16.0)).clicked() {
                        close_window = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Validate button
                        if ui.button("Validate JSON").clicked() {
                            match serde_json::from_str::<Resource>(&self.json_content) {
                                Ok(_) => {
                                    self.error_message = Some("JSON is valid!".to_string());
                                }
                                Err(e) => {
                                    self.error_message = Some(format!("Invalid JSON: {}", e));
                                }
                            }
                        }
                    });
                });
            });

        // Handle save action
        if save_clicked {
            match serde_json::from_str::<Resource>(&self.json_content) {
                Ok(resource) => {
                    info!("Saving resource JSON for: {}", self.resource_id);

                    // Call the save callback
                    if let Some(on_save) = &mut self.on_save {
                        on_save(self.resource_id.clone(), resource.clone());
                    }

                    // Store the saved resource for the app to handle
                    self.saved_resource = Some(resource);
                    self.save_requested = true;

                    self.show = false;
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Invalid JSON: {}", e));
                }
            }
        }

        // Handle cancel action
        if close_window {
            self.show = false;
        }
    }

    /// Show the editor window with focus capability
    pub fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        theme: ThemeChoice,
        _bring_to_front: bool,
    ) {
        // For now, just delegate to the existing show method
        // Note: bring_to_front parameter is not used yet but could be implemented
        // by modifying the Window creation in the show method
        self.show(ctx, theme);
    }
}

impl FocusableWindow for ResourceJsonEditorWindow {
    type ShowParams = ThemeShowParams;

    fn window_id(&self) -> &'static str {
        "resource_json_editor"
    }

    fn window_title(&self) -> String {
        "Resource JSON Editor".to_string()
    }

    fn is_open(&self) -> bool {
        self.show
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        // Convert theme string to ThemeChoice
        let theme = match params.theme.as_str() {
            "Latte" => ThemeChoice::Latte,
            "Frappe" => ThemeChoice::Frappe,
            "Macchiato" => ThemeChoice::Macchiato,
            "Mocha" => ThemeChoice::Mocha,
            _ => ThemeChoice::default(),
        };
        self.show_with_focus(ctx, theme, bring_to_front);
    }
}
