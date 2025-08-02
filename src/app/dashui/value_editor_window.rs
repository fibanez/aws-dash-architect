use crate::app::cfn_resources::PropertyDefinition;
use eframe::egui::{self, Button, Color32, Grid, ScrollArea, TextEdit, Window};
use serde_json::Value;

/// Represents the value editor window for editing CloudFormation property values
/// with schema-aware validation and constraint-aware UI components
pub struct ValueEditorWindow {
    /// Whether to show the window
    pub show: bool,

    /// The property name being edited
    pub property_name: String,

    /// The property definition with schema constraints
    pub property_definition: Option<PropertyDefinition>,

    /// The current value being edited (as JSON string)
    pub current_value: String,

    /// The original value for comparison
    original_value: String,

    /// Error message if any
    pub error_message: Option<String>,

    /// Whether the value has been modified
    pub is_modified: bool,

    /// Callback when the value is saved
    pub on_save: Option<Box<dyn FnMut(String)>>,

    /// The last saved value (for external access)
    pub last_saved_value: Option<String>,

    /// The editing mode for the value
    editing_mode: ValueEditingMode,

    /// Validation result for the current value
    validation_result: ValidationResult,
}

/// Different modes for editing property values
#[derive(Debug, Clone, PartialEq)]
enum ValueEditingMode {
    /// Simple text editing for primitive types
    Text,
    /// Boolean toggle for boolean properties
    Boolean,
    /// Dropdown selection for enum properties
    Enum,
    /// Number input with constraints
    Number,
    /// JSON editor for complex objects/arrays
    Json,
}

/// Result of property value validation
#[derive(Debug, Clone)]
struct ValidationResult {
    is_valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl Default for ValueEditorWindow {
    fn default() -> Self {
        Self {
            show: false,
            property_name: String::new(),
            property_definition: None,
            current_value: String::new(),
            original_value: String::new(),
            error_message: None,
            is_modified: false,
            on_save: None,
            last_saved_value: None,
            editing_mode: ValueEditingMode::Text,
            validation_result: ValidationResult {
                is_valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            },
        }
    }
}

impl ValueEditorWindow {
    /// Create a new value editor window
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the window for editing a property value
    pub fn open(
        &mut self,
        property_name: String,
        property_definition: Option<PropertyDefinition>,
        current_value: String,
        on_save: impl FnMut(String) + 'static,
    ) {
        self.property_name = property_name;
        self.property_definition = property_definition;
        self.current_value = current_value.clone();
        self.original_value = current_value;
        self.error_message = None;
        self.is_modified = false;
        self.on_save = Some(Box::new(on_save));

        // Determine the editing mode based on property definition
        self.editing_mode = self.determine_editing_mode();

        // Validate the initial value
        self.validate_current_value();

        self.show = true;
    }

    /// Show the value editor window
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut value_saved = false;

        if !self.show {
            return value_saved;
        }

        let title = format!("Edit Property: {}", self.property_name);

        // Get screen dimensions for window sizing
        let screen_rect = ctx.screen_rect();
        let max_width = screen_rect.width() * 0.6;
        let max_height = screen_rect.height() * 0.7;

        let mut close_window = false;
        let mut show_window = self.show;

        Window::new(title)
            .open(&mut show_window)
            .min_width(400.0)
            .min_height(300.0)
            .max_width(max_width)
            .max_height(max_height)
            .resizable(true)
            .default_pos(screen_rect.center())
            .show(ctx, |ui| {
                // Error message display
                if let Some(error) = &self.error_message {
                    ui.colored_label(Color32::from_rgb(220, 50, 50), error);
                    ui.separator();
                }

                // Property information
                ui.horizontal(|ui| {
                    ui.label("Property:");
                    ui.strong(&self.property_name);
                });

                if let Some(def) = &self.property_definition {
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        if let Some(primitive) = &def.primitive_type {
                            ui.label(primitive);
                        } else if let Some(type_name) = &def.type_name {
                            ui.label(type_name);
                        } else {
                            ui.label("Unknown");
                        }

                        if def.required {
                            ui.colored_label(Color32::RED, "*Required");
                        }
                    });

                    // Show constraints if available
                    if self.has_constraints() {
                        ui.collapsing("Constraints", |ui| {
                            self.show_constraints(ui);
                        });
                    }
                }

                ui.separator();

                // Validation status
                self.show_validation_status(ui);

                ui.separator();

                // Value editor content
                ScrollArea::vertical()
                    .max_height(max_height - 200.0)
                    .show(ui, |ui| {
                        self.show_value_editor(ui);
                    });

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close_window = true;
                    }

                    // Save button - enabled only if valid and modified
                    let save_enabled = self.validation_result.is_valid && self.is_modified;
                    let save_button = ui.add_enabled(save_enabled, Button::new("Save"));

                    if save_button.clicked() {
                        // Store the saved value for external access
                        self.last_saved_value = Some(self.current_value.clone());

                        if let Some(on_save) = &mut self.on_save {
                            on_save(self.current_value.clone());
                        }
                        value_saved = true;
                        close_window = true;
                    }

                    // Reset button - enabled only if modified
                    let reset_button = ui.add_enabled(self.is_modified, Button::new("Reset"));
                    if reset_button.clicked() {
                        self.current_value = self.original_value.clone();
                        self.is_modified = false;
                        self.validate_current_value();
                    }

                    // Validation info
                    if !self.validation_result.is_valid {
                        ui.colored_label(Color32::RED, "Fix errors to save");
                    } else if !self.is_modified {
                        ui.colored_label(Color32::GRAY, "No changes");
                    }
                });
            });

        // Update show state
        self.show = show_window;

        // Close window if requested
        if close_window {
            self.show = false;
        }

        value_saved
    }

    /// Get and clear the last saved value
    pub fn take_saved_value(&mut self) -> Option<String> {
        self.last_saved_value.take()
    }

    /// Determine the appropriate editing mode based on property definition
    fn determine_editing_mode(&self) -> ValueEditingMode {
        if let Some(def) = &self.property_definition {
            // Check for enum values first
            if def.enum_values.is_some() {
                return ValueEditingMode::Enum;
            }

            // Check primitive type
            if let Some(primitive) = &def.primitive_type {
                match primitive.as_str() {
                    "Boolean" => ValueEditingMode::Boolean,
                    "Number" | "Integer" | "Double" | "Long" => ValueEditingMode::Number,
                    "String" => ValueEditingMode::Text,
                    _ => ValueEditingMode::Text,
                }
            } else if let Some(type_name) = &def.type_name {
                match type_name.as_str() {
                    "List" | "Map" => ValueEditingMode::Json,
                    _ => {
                        // Complex property type
                        ValueEditingMode::Json
                    }
                }
            } else {
                ValueEditingMode::Text
            }
        } else {
            // No definition available, try to infer from current value
            if self.current_value.trim().starts_with('{')
                || self.current_value.trim().starts_with('[')
            {
                ValueEditingMode::Json
            } else {
                ValueEditingMode::Text
            }
        }
    }

    /// Show the appropriate value editor based on editing mode
    fn show_value_editor(&mut self, ui: &mut egui::Ui) {
        let old_value = self.current_value.clone();

        match self.editing_mode {
            ValueEditingMode::Boolean => self.show_boolean_editor(ui),
            ValueEditingMode::Enum => self.show_enum_editor(ui),
            ValueEditingMode::Number => self.show_number_editor(ui),
            ValueEditingMode::Json => self.show_json_editor(ui),
            ValueEditingMode::Text => self.show_text_editor(ui),
        }

        // Check if value changed
        if self.current_value != old_value {
            self.is_modified = self.current_value != self.original_value;
            self.validate_current_value();
        }
    }

    /// Show boolean editor (toggle)
    fn show_boolean_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("Boolean Value:");

        let mut bool_value = self.current_value.to_lowercase() == "true";

        if ui.checkbox(&mut bool_value, "").changed() {
            self.current_value = bool_value.to_string();
        }

        ui.horizontal(|ui| {
            ui.radio_value(&mut bool_value, true, "True");
            ui.radio_value(&mut bool_value, false, "False");
        });

        self.current_value = bool_value.to_string();
    }

    /// Show enum editor (dropdown)
    fn show_enum_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("Select Value:");

        if let Some(def) = &self.property_definition {
            if let Some(enum_values) = &def.enum_values {
                egui::ComboBox::from_id_salt("enum_value_selector")
                    .selected_text(if self.current_value.is_empty() {
                        "Select..."
                    } else {
                        &self.current_value
                    })
                    .show_ui(ui, |ui| {
                        // Add empty option if not required
                        if !def.required {
                            ui.selectable_value(&mut self.current_value, String::new(), "(None)");
                        }

                        for value in enum_values {
                            ui.selectable_value(&mut self.current_value, value.clone(), value);
                        }
                    });
            }
        }
    }

    /// Show number editor with constraints
    fn show_number_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("Number Value:");

        let response = ui.text_edit_singleline(&mut self.current_value);
        response.on_hover_text("Enter a numeric value");

        // Show constraints if available
        if let Some(def) = &self.property_definition {
            let mut constraint_text = Vec::new();

            if let Some(min) = def.min_value {
                constraint_text.push(format!("Min: {}", min));
            }
            if let Some(max) = def.max_value {
                constraint_text.push(format!("Max: {}", max));
            }

            if !constraint_text.is_empty() {
                ui.small(format!("Constraints: {}", constraint_text.join(", ")));
            }
        }
    }

    /// Show text editor with pattern validation
    fn show_text_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("Text Value:");

        let response = ui.text_edit_multiline(&mut self.current_value);

        // Show constraints if available
        if let Some(def) = &self.property_definition {
            let mut constraint_text = Vec::new();

            if let Some(min_len) = def.min_length {
                constraint_text.push(format!("Min length: {}", min_len));
            }
            if let Some(max_len) = def.max_length {
                constraint_text.push(format!("Max length: {}", max_len));
            }
            if let Some(pattern) = &def.pattern {
                constraint_text.push(format!("Pattern: {}", pattern));
            }

            if !constraint_text.is_empty() {
                ui.small(format!("Constraints: {}", constraint_text.join(", ")));
            }
        }

        response.on_hover_text("Enter text value");
    }

    /// Show JSON editor for complex values
    fn show_json_editor(&mut self, ui: &mut egui::Ui) {
        ui.label("JSON Value:");

        let response = ui.add(
            TextEdit::multiline(&mut self.current_value)
                .font(egui::TextStyle::Monospace)
                .min_size([400.0, 200.0].into()),
        );

        response.on_hover_text("Enter valid JSON");

        // Format JSON button
        ui.horizontal(|ui| {
            if ui.button("Format JSON").clicked() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&self.current_value) {
                    if let Ok(formatted) = serde_json::to_string_pretty(&parsed) {
                        self.current_value = formatted;
                    }
                }
            }

            if ui.button("Minify JSON").clicked() {
                if let Ok(parsed) = serde_json::from_str::<Value>(&self.current_value) {
                    if let Ok(minified) = serde_json::to_string(&parsed) {
                        self.current_value = minified;
                    }
                }
            }
        });
    }

    /// Validate the current value against schema constraints
    fn validate_current_value(&mut self) {
        let mut errors = Vec::new();
        let warnings = Vec::new();

        if let Some(def) = &self.property_definition {
            // Check if required but empty
            if def.required && self.current_value.trim().is_empty() {
                errors.push("This property is required".to_string());
            }

            // Skip other validations if empty and not required
            if self.current_value.trim().is_empty() && !def.required {
                self.validation_result = ValidationResult {
                    is_valid: true,
                    errors,
                    warnings,
                };
                return;
            }

            // Validate based on type
            if let Some(primitive) = &def.primitive_type {
                match primitive.as_str() {
                    "Boolean" => {
                        if !matches!(self.current_value.to_lowercase().as_str(), "true" | "false") {
                            errors.push("Value must be 'true' or 'false'".to_string());
                        }
                    }
                    "Number" | "Double" => {
                        if self.current_value.parse::<f64>().is_err() {
                            errors.push("Value must be a valid number".to_string());
                        } else if let Ok(num) = self.current_value.parse::<f64>() {
                            if let Some(min) = def.min_value {
                                if num < min {
                                    errors.push(format!("Value must be at least {}", min));
                                }
                            }
                            if let Some(max) = def.max_value {
                                if num > max {
                                    errors.push(format!("Value must be at most {}", max));
                                }
                            }
                        }
                    }
                    "Integer" | "Long" => {
                        if self.current_value.parse::<i64>().is_err() {
                            errors.push("Value must be a valid integer".to_string());
                        } else if let Ok(num) = self.current_value.parse::<i64>() {
                            if let Some(min) = def.min_value {
                                if (num as f64) < min {
                                    errors.push(format!("Value must be at least {}", min));
                                }
                            }
                            if let Some(max) = def.max_value {
                                if (num as f64) > max {
                                    errors.push(format!("Value must be at most {}", max));
                                }
                            }
                        }
                    }
                    "String" => {
                        // String length validation
                        if let Some(min_len) = def.min_length {
                            if self.current_value.len() < min_len {
                                errors
                                    .push(format!("Value must be at least {} characters", min_len));
                            }
                        }
                        if let Some(max_len) = def.max_length {
                            if self.current_value.len() > max_len {
                                errors
                                    .push(format!("Value must be at most {} characters", max_len));
                            }
                        }

                        // Pattern validation
                        if let Some(pattern) = &def.pattern {
                            if let Ok(regex) = regex::Regex::new(pattern) {
                                if !regex.is_match(&self.current_value) {
                                    errors.push(format!("Value must match pattern: {}", pattern));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Enum validation
            if let Some(enum_values) = &def.enum_values {
                if !enum_values.contains(&self.current_value) && !self.current_value.is_empty() {
                    errors.push(format!("Value must be one of: {}", enum_values.join(", ")));
                }
            }

            // JSON validation for complex types
            if matches!(self.editing_mode, ValueEditingMode::Json)
                && !self.current_value.trim().is_empty()
            {
                if let Err(e) = serde_json::from_str::<Value>(&self.current_value) {
                    errors.push(format!("Invalid JSON: {}", e));
                }
            }
        }

        self.validation_result = ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        };
    }

    /// Show validation status in the UI
    fn show_validation_status(&self, ui: &mut egui::Ui) {
        if !self.validation_result.errors.is_empty() {
            ui.colored_label(Color32::RED, "❌ Validation Errors:");
            for error in &self.validation_result.errors {
                ui.colored_label(Color32::RED, format!("  • {}", error));
            }
        } else if self.is_modified {
            ui.colored_label(Color32::GREEN, "✅ Valid");
        }

        if !self.validation_result.warnings.is_empty() {
            ui.colored_label(Color32::YELLOW, "⚠ Warnings:");
            for warning in &self.validation_result.warnings {
                ui.colored_label(Color32::YELLOW, format!("  • {}", warning));
            }
        }
    }

    /// Check if the property has constraints to display
    fn has_constraints(&self) -> bool {
        if let Some(def) = &self.property_definition {
            def.enum_values.is_some()
                || def.pattern.is_some()
                || def.min_length.is_some()
                || def.max_length.is_some()
                || def.min_value.is_some()
                || def.max_value.is_some()
                || def.unique_items.is_some()
        } else {
            false
        }
    }

    /// Show constraints information
    fn show_constraints(&self, ui: &mut egui::Ui) {
        if let Some(def) = &self.property_definition {
            Grid::new("constraints_grid")
                .num_columns(2)
                .spacing([10.0, 5.0])
                .show(ui, |ui| {
                    if let Some(enum_values) = &def.enum_values {
                        ui.label("Allowed values:");
                        ui.label(enum_values.join(", "));
                        ui.end_row();
                    }

                    if let Some(pattern) = &def.pattern {
                        ui.label("Pattern:");
                        ui.label(pattern);
                        ui.end_row();
                    }

                    if let Some(min_len) = def.min_length {
                        ui.label("Min length:");
                        ui.label(min_len.to_string());
                        ui.end_row();
                    }

                    if let Some(max_len) = def.max_length {
                        ui.label("Max length:");
                        ui.label(max_len.to_string());
                        ui.end_row();
                    }

                    if let Some(min_val) = def.min_value {
                        ui.label("Min value:");
                        ui.label(min_val.to_string());
                        ui.end_row();
                    }

                    if let Some(max_val) = def.max_value {
                        ui.label("Max value:");
                        ui.label(max_val.to_string());
                        ui.end_row();
                    }

                    if let Some(unique) = def.unique_items {
                        ui.label("Unique items:");
                        ui.label(if unique { "Yes" } else { "No" });
                        ui.end_row();
                    }
                });
        }
    }
}
