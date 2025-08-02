use super::{
    parameter_store::ParameterStoreManager,
    parameters::{ParameterInfo, ParameterInputType},
    resource_lookup::ResourceLookupService,
    resource_picker_dialog::{AwsResourcePickerDialog, ResourcePickerState},
    secrets_manager::SecretsManagerClient,
};
use crate::app::projects::Project;
use egui::{self, Button, Color32, ComboBox, Context, RichText, ScrollArea, TextEdit, Ui};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

/// Parameter value source options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParameterSource {
    Manual,
    ParameterStore,
    SecretsManager,
    History,
}

/// Historical parameter value for reuse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterHistoryEntry {
    pub value: String,
    pub description: String,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub environment: String,
}

/// Parameter validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub error_message: Option<String>,
    pub warning_message: Option<String>,
}

/// State of the parameter input dialog
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterDialogState {
    Closed,
    Editing,
    Validating,
    Saving,
}

/// Enhanced Parameter Input Dialog with comprehensive features
pub struct ParameterInputDialog {
    // Dialog state
    pub state: ParameterDialogState,
    pub is_open: bool,

    // Current project and environment context
    pub project: Option<Project>,
    pub environment: String,
    pub account_id: String,
    pub region: String,

    // Parameters being edited
    pub parameters: Vec<ParameterInfo>,
    pub parameter_values: HashMap<String, String>,
    pub parameter_sources: HashMap<String, ParameterSource>,
    pub validation_results: HashMap<String, ValidationResult>,

    // UI state
    pub current_parameter_index: usize,
    pub search_filter: String,
    pub show_advanced_options: bool,
    pub show_history_panel: bool,

    // AWS resource picker integration
    pub resource_picker: AwsResourcePickerDialog,
    pub current_picker_parameter: Option<String>,

    // Parameter history
    pub parameter_history: HashMap<String, Vec<ParameterHistoryEntry>>,

    // Store as default options
    pub store_as_default_options: HashMap<String, bool>,
    pub selected_store_method: HashMap<String, ParameterSource>,

    // Services
    parameter_store_manager: Option<Arc<ParameterStoreManager>>,
    secrets_manager_client: Option<Arc<SecretsManagerClient>>,
    resource_lookup_service: Option<Arc<ResourceLookupService>>,

    // Error handling
    pub error_message: Option<String>,
    pub success_message: Option<String>,
}

impl Default for ParameterInputDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ParameterInputDialog {
    pub fn new() -> Self {
        Self {
            state: ParameterDialogState::Closed,
            is_open: false,
            project: None,
            environment: String::new(),
            account_id: String::new(),
            region: String::new(),
            parameters: Vec::new(),
            parameter_values: HashMap::new(),
            parameter_sources: HashMap::new(),
            validation_results: HashMap::new(),
            current_parameter_index: 0,
            search_filter: String::new(),
            show_advanced_options: false,
            show_history_panel: false,
            resource_picker: AwsResourcePickerDialog::new(),
            current_picker_parameter: None,
            parameter_history: HashMap::new(),
            store_as_default_options: HashMap::new(),
            selected_store_method: HashMap::new(),
            parameter_store_manager: None,
            secrets_manager_client: None,
            resource_lookup_service: None,
            error_message: None,
            success_message: None,
        }
    }

    /// Open the dialog with parameters for editing
    pub fn open(
        &mut self,
        parameters: Vec<ParameterInfo>,
        project: Option<Project>,
        environment: String,
        account_id: String,
        region: String,
    ) {
        self.parameters = parameters;
        self.project = project;
        self.environment = environment;
        self.account_id = account_id;
        self.region = region;
        self.is_open = true;
        self.state = ParameterDialogState::Editing;
        self.current_parameter_index = 0;

        // Initialize parameter values with defaults
        for param in &self.parameters {
            if let Some(default_value) = &param.default_value {
                self.parameter_values
                    .insert(param.name.clone(), default_value.clone());
            } else {
                self.parameter_values
                    .insert(param.name.clone(), String::new());
            }

            // Initialize parameter sources
            self.parameter_sources
                .insert(param.name.clone(), ParameterSource::Manual);
            self.store_as_default_options
                .insert(param.name.clone(), false);
            self.selected_store_method.insert(
                param.name.clone(),
                if param.is_sensitive {
                    ParameterSource::SecretsManager
                } else {
                    ParameterSource::ParameterStore
                },
            );
        }

        info!(
            "Opened parameter dialog with {} parameters for environment {}",
            self.parameters.len(),
            self.environment
        );
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.is_open = false;
        self.state = ParameterDialogState::Closed;
        self.error_message = None;
        self.success_message = None;
        self.resource_picker.state = ResourcePickerState::Closed;
        debug!("Closed parameter dialog");
    }

    /// Set the services for the dialog
    pub fn set_services(
        &mut self,
        parameter_store_manager: Option<Arc<ParameterStoreManager>>,
        secrets_manager_client: Option<Arc<SecretsManagerClient>>,
        resource_lookup_service: Option<Arc<ResourceLookupService>>,
    ) {
        self.parameter_store_manager = parameter_store_manager;
        self.secrets_manager_client = secrets_manager_client;
        self.resource_lookup_service = resource_lookup_service;
    }

    /// Render the parameter input dialog
    pub fn show(&mut self, ctx: &Context) -> bool {
        if !self.is_open {
            return false;
        }

        let mut dialog_open = true;
        let mut parameters_confirmed = false;

        // Calculate responsive dialog size
        let screen_rect = ctx.screen_rect();
        let dialog_width = (screen_rect.width() * 0.7).clamp(600.0, 1000.0);
        let dialog_height = (screen_rect.height() * 0.8).clamp(500.0, 800.0);

        egui::Window::new("CloudFormation Parameters")
            .id(egui::Id::new("cfn_parameter_dialog"))
            .default_width(dialog_width)
            .default_height(dialog_height)
            .min_width(600.0)
            .min_height(400.0)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut dialog_open)
            .show(ctx, |ui| {
                parameters_confirmed = self.show_content(ui);
            });

        if !dialog_open {
            self.close();
        }

        // Handle resource picker results
        if let ResourcePickerState::ResourceSelected(resource_id) =
            &self.resource_picker.state.clone()
        {
            if let Some(param_name) = &self.current_picker_parameter {
                self.parameter_values
                    .insert(param_name.clone(), resource_id.clone());
                info!(
                    "Selected resource {} for parameter {}",
                    resource_id, param_name
                );
            }
            self.resource_picker.state = ResourcePickerState::Closed;
            self.current_picker_parameter = None;
        }

        // Show resource picker if active
        if self.resource_picker.state != ResourcePickerState::Closed {
            self.resource_picker.show(ctx);
        }

        parameters_confirmed
    }

    /// Show the main content of the dialog
    fn show_content(&mut self, ui: &mut Ui) -> bool {
        let mut parameters_confirmed = false;

        // Header with deployment context
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📋 Deployment Context").strong());
                ui.separator();
                ui.label(format!("Environment: {}", self.environment));
                ui.separator();
                ui.label(format!("Region: {}", self.region));
                if let Some(project) = &self.project {
                    ui.separator();
                    ui.label(format!("Project: {}", project.name));
                }
            });
        });

        ui.add_space(8.0);

        // Error/Success messages
        if let Some(error) = &self.error_message {
            ui.colored_label(Color32::from_rgb(220, 50, 50), format!("❌ {}", error));
            ui.add_space(8.0);
        }
        if let Some(success) = &self.success_message {
            ui.colored_label(Color32::from_rgb(40, 180, 40), format!("✅ {}", success));
            ui.add_space(8.0);
        }

        // Search and quick actions
        ui.horizontal(|ui| {
            ui.label("🔍 Filter:");
            ui.add_sized(
                [200.0, 20.0],
                TextEdit::singleline(&mut self.search_filter).hint_text("Search parameters..."),
            );

            ui.add_space(20.0);
            ui.checkbox(&mut self.show_advanced_options, "Show Constraints");

            ui.add_space(20.0);

            if ui.button("Validate All").clicked() {
                self.validate_all_parameters();
            }

            if ui.button("Load Defaults").clicked() {
                self.load_parameter_history();
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Filter and clone parameters for display to avoid borrowing issues
        let filtered_params: Vec<ParameterInfo> = self
            .parameters
            .iter()
            .filter(|param| {
                if self.search_filter.is_empty() {
                    true
                } else {
                    param
                        .name
                        .to_lowercase()
                        .contains(&self.search_filter.to_lowercase())
                        || param.description.as_ref().is_some_and(|desc| {
                            desc.to_lowercase()
                                .contains(&self.search_filter.to_lowercase())
                        })
                }
            })
            .cloned()
            .collect();

        let has_search_filter = !self.search_filter.is_empty();

        // Main parameters form - scrollable area
        let available_height = ui.available_height() - 60.0; // Leave space for buttons

        ScrollArea::vertical()
            .max_height(available_height)
            .show(ui, |ui| {
                if filtered_params.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label(RichText::new("No parameters found").weak());
                        if has_search_filter {
                            ui.label("Try adjusting your search filter");
                        }
                    });
                } else {
                    // Process each parameter directly in the UI
                    for param in &filtered_params {
                        self.render_parameter_row(ui, param);
                        ui.add_space(5.0);
                    }
                }
            });

        ui.add_space(8.0);
        ui.separator();

        // Bottom buttons
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                self.close();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let all_valid = self.validate_all_parameters();
                let button_text = if all_valid {
                    "Deploy Stack"
                } else {
                    "Fix Validation Errors"
                };
                let button_color = if all_valid {
                    Color32::from_rgb(40, 140, 60)
                } else {
                    Color32::from_rgb(180, 140, 40)
                };

                if ui
                    .add(
                        Button::new(RichText::new(button_text).color(Color32::WHITE))
                            .fill(button_color)
                            .min_size(egui::vec2(120.0, 32.0)),
                    )
                    .clicked()
                    && all_valid
                {
                    parameters_confirmed = true;
                    self.close();
                }
            });
        });

        parameters_confirmed
    }

    /// Render a parameter row with all its components
    fn render_parameter_row(&mut self, ui: &mut Ui, param: &ParameterInfo) {
        let current_value = self
            .parameter_values
            .get(&param.name)
            .cloned()
            .unwrap_or_default();
        let is_valid = self
            .validation_results
            .get(&param.name)
            .map_or(true, |result| result.is_valid);

        // Parameter row in a grouped frame
        ui.group(|ui| {
            ui.vertical(|ui| {
                // Parameter header with name and indicators
                ui.horizontal(|ui| {
                    // Parameter name with type
                    ui.label(
                        RichText::new(&param.name)
                            .strong()
                            .size(14.0)
                            .color(if is_valid {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(220, 50, 50)
                            }),
                    );

                    // Type badge
                    ui.label(
                        RichText::new(format!("({})", param.parameter_type))
                            .weak()
                            .size(11.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Status indicators on the right
                        if param.is_sensitive {
                            ui.label(
                                RichText::new("🔒 Sensitive")
                                    .color(Color32::from_rgb(255, 200, 100)),
                            );
                        }
                        if param.is_aws_specific {
                            ui.label(
                                RichText::new("☁ AWS Resource")
                                    .color(Color32::from_rgb(100, 150, 255)),
                            );
                        }
                        if param.default_value.is_none() {
                            ui.label(
                                RichText::new("* Required").color(Color32::from_rgb(255, 100, 100)),
                            );
                        }
                    });
                });

                // Description if available
                if let Some(description) = &param.description {
                    ui.label(RichText::new(description).weak().size(12.0));
                }

                ui.add_space(5.0);

                // Input field based on parameter type
                self.render_parameter_input(ui, param, current_value);

                // Validation feedback
                if let Some(validation) = self.validation_results.get(&param.name) {
                    if !validation.is_valid {
                        if let Some(error) = &validation.error_message {
                            ui.colored_label(
                                Color32::from_rgb(220, 50, 50),
                                format!("❌ {}", error),
                            );
                        }
                    }
                    if let Some(warning) = &validation.warning_message {
                        ui.colored_label(
                            Color32::from_rgb(255, 200, 100),
                            format!("⚠️ {}", warning),
                        );
                    }
                }

                // Show constraints if available
                if self.show_advanced_options {
                    Self::show_parameter_constraints_static(ui, param);
                }
            });
        });
    }

    /// Render the input field for a parameter
    fn render_parameter_input(
        &mut self,
        ui: &mut Ui,
        param: &ParameterInfo,
        current_value: String,
    ) {
        ui.horizontal(|ui| {
            ui.label("Value:");
            ui.add_space(10.0);

            let mut new_value = current_value.clone();
            let input_width = ui.available_width() - 150.0; // Leave space for browse button

            match self.determine_input_type(param) {
                ParameterInputType::Select => {
                    if let Some(allowed_values) = &param.allowed_values {
                        ComboBox::from_id_salt(format!("param_{}", param.name))
                            .selected_text(&new_value)
                            .width(input_width)
                            .show_ui(ui, |ui| {
                                for value in allowed_values {
                                    ui.selectable_value(&mut new_value, value.clone(), value);
                                }
                            });
                    }
                }
                ParameterInputType::AwsResourcePicker => {
                    ui.add_sized(
                        [input_width - 80.0, 24.0],
                        if param.no_echo {
                            TextEdit::singleline(&mut new_value).password(true)
                        } else {
                            TextEdit::singleline(&mut new_value)
                                .hint_text("Select AWS resource or enter manually")
                        },
                    );
                    if ui.button("Browse...").clicked() {
                        self.open_resource_picker(param);
                    }
                }
                ParameterInputType::TextArea => {
                    ui.vertical(|ui| {
                        ui.add_sized(
                            [input_width, 80.0],
                            TextEdit::multiline(&mut new_value).hint_text("Enter multi-line text"),
                        );
                    });
                }
                ParameterInputType::Number => {
                    ui.add_sized(
                        [input_width, 24.0],
                        TextEdit::singleline(&mut new_value).hint_text("Enter a number"),
                    );
                }
                _ => {
                    // Default text input
                    ui.add_sized(
                        [input_width, 24.0],
                        if param.no_echo {
                            TextEdit::singleline(&mut new_value).password(true)
                        } else {
                            TextEdit::singleline(&mut new_value)
                                .hint_text(format!("Enter {}", param.parameter_type.to_lowercase()))
                        },
                    );
                }
            }

            // Update value if changed
            if new_value != current_value {
                self.parameter_values
                    .insert(param.name.clone(), new_value.clone());
                self.validate_parameter(param, &new_value);
            }
        });
    }

    /// Show parameter constraints in a compact format (static version)
    fn show_parameter_constraints_static(ui: &mut Ui, param: &ParameterInfo) {
        let mut constraints = Vec::new();

        if let Some(allowed_values) = &param.allowed_values {
            constraints.push(format!("Allowed: {}", allowed_values.join(", ")));
        }
        if let Some(pattern) = &param.allowed_pattern {
            constraints.push(format!("Pattern: {}", pattern));
        }
        if let Some(min) = param.min_length {
            constraints.push(format!("Min length: {}", min));
        }
        if let Some(max) = param.max_length {
            constraints.push(format!("Max length: {}", max));
        }
        if let Some(min) = param.min_value {
            constraints.push(format!("Min value: {}", min));
        }
        if let Some(max) = param.max_value {
            constraints.push(format!("Max value: {}", max));
        }

        if !constraints.is_empty() {
            ui.collapsing("Constraints", |ui| {
                for constraint in constraints {
                    ui.label(RichText::new(constraint).weak().size(11.0));
                }
            });
        }
    }

    /// Legacy method - replaced by render_parameter_row
    #[allow(dead_code)]
    fn show_parameter_details(&mut self, ui: &mut Ui, param: &ParameterInfo) {
        ui.heading(&param.name);

        // Parameter description
        if let Some(description) = &param.description {
            ui.label(RichText::new(description).italics().color(Color32::GRAY));
        }

        ui.separator();

        // Parameter type and constraints
        ui.horizontal(|ui| {
            ui.label(format!("Type: {}", param.parameter_type));
            if param.no_echo {
                ui.colored_label(Color32::YELLOW, "🔒 NoEcho");
            }
            if param.is_aws_specific {
                ui.colored_label(Color32::BLUE, "🔗 AWS Resource");
            }
        });

        // Constraints
        if param.allowed_values.is_some()
            || param.allowed_pattern.is_some()
            || param.min_length.is_some()
            || param.max_length.is_some()
        {
            ui.collapsing("Constraints", |ui| {
                if let Some(allowed_values) = &param.allowed_values {
                    ui.label(format!("Allowed values: {}", allowed_values.join(", ")));
                }
                if let Some(pattern) = &param.allowed_pattern {
                    ui.label(format!("Pattern: {}", pattern));
                }
                if let Some(min) = param.min_length {
                    ui.label(format!("Min length: {}", min));
                }
                if let Some(max) = param.max_length {
                    ui.label(format!("Max length: {}", max));
                }
            });
        }

        ui.separator();

        // Current value input
        let current_value = self
            .parameter_values
            .get(&param.name)
            .cloned()
            .unwrap_or_default();
        let mut new_value = current_value.clone();

        match self.determine_input_type(param) {
            ParameterInputType::Text => {
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    let text_edit = if param.no_echo {
                        TextEdit::singleline(&mut new_value).password(true)
                    } else {
                        TextEdit::singleline(&mut new_value)
                    };
                    ui.add_sized([300.0, 20.0], text_edit);
                });
            }
            ParameterInputType::TextArea => {
                ui.label("Value:");
                ui.add_sized([400.0, 100.0], TextEdit::multiline(&mut new_value));
            }
            ParameterInputType::Number => {
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    ui.add_sized(
                        [200.0, 20.0],
                        TextEdit::singleline(&mut new_value).hint_text("Enter number"),
                    );
                });
            }
            ParameterInputType::Select => {
                if let Some(allowed_values) = &param.allowed_values {
                    ui.horizontal(|ui| {
                        ui.label("Value:");
                        ComboBox::from_label("")
                            .selected_text(&new_value)
                            .show_ui(ui, |ui| {
                                for value in allowed_values {
                                    ui.selectable_value(&mut new_value, value.clone(), value);
                                }
                            });
                    });
                }
            }
            ParameterInputType::AwsResourcePicker => {
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    ui.add_sized([200.0, 20.0], TextEdit::singleline(&mut new_value));
                    if ui.button("Browse...").clicked() {
                        self.open_resource_picker(param);
                    }
                });
            }
            _ => {
                // Default to text input
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    ui.add_sized([300.0, 20.0], TextEdit::singleline(&mut new_value));
                });
            }
        }

        // Update value if changed
        if new_value != current_value {
            self.parameter_values
                .insert(param.name.clone(), new_value.clone());
            self.validate_parameter(param, &new_value);
        }

        // Validation result
        if let Some(validation) = self.validation_results.get(&param.name) {
            if !validation.is_valid {
                if let Some(error) = &validation.error_message {
                    ui.colored_label(Color32::RED, format!("❌ {}", error));
                }
            } else {
                ui.colored_label(Color32::GREEN, "✅ Valid");
            }

            if let Some(warning) = &validation.warning_message {
                ui.colored_label(Color32::YELLOW, format!("⚠️ {}", warning));
            }
        }

        ui.separator();

        // Store as Default options
        if self.show_advanced_options {
            ui.collapsing("Store as Default", |ui| {
                let mut store_default = self
                    .store_as_default_options
                    .get(&param.name)
                    .cloned()
                    .unwrap_or(false);
                if ui
                    .checkbox(&mut store_default, "Store this value as default")
                    .changed()
                {
                    self.store_as_default_options
                        .insert(param.name.clone(), store_default);
                }

                if store_default {
                    let mut selected_method = self
                        .selected_store_method
                        .get(&param.name)
                        .cloned()
                        .unwrap_or(ParameterSource::ParameterStore);

                    ui.horizontal(|ui| {
                        ui.label("Store in:");
                        ui.radio_value(
                            &mut selected_method,
                            ParameterSource::ParameterStore,
                            "Parameter Store",
                        );
                        if param.is_sensitive {
                            ui.radio_value(
                                &mut selected_method,
                                ParameterSource::SecretsManager,
                                "Secrets Manager",
                            );
                        }
                    });

                    self.selected_store_method
                        .insert(param.name.clone(), selected_method);
                }
            });
        }

        // Parameter history
        if self.show_history_panel {
            ui.collapsing("Parameter History", |ui| {
                if let Some(history) = self.parameter_history.get(&param.name) {
                    for entry in history.iter().take(5) {
                        ui.horizontal(|ui| {
                            if ui.small_button("Use").clicked() {
                                self.parameter_values
                                    .insert(param.name.clone(), entry.value.clone());
                            }
                            ui.label(&entry.value);
                            ui.small(&entry.environment);
                            ui.small(entry.last_used.format("%Y-%m-%d").to_string());
                        });
                    }
                } else {
                    ui.label("No history available");
                }
            });
        }
    }

    /// Determine the appropriate input type for a parameter
    fn determine_input_type(&self, param: &ParameterInfo) -> ParameterInputType {
        // Check for allowed values (dropdown)
        if param.allowed_values.is_some() {
            return ParameterInputType::Select;
        }

        // Check for AWS resource types
        if param.is_aws_specific || param.aws_resource_type.is_some() {
            return ParameterInputType::AwsResourcePicker;
        }

        // Check for number types
        if param.parameter_type.contains("Number") {
            return ParameterInputType::Number;
        }

        // Check for long text (based on max length or description hints)
        if let Some(max_length) = param.max_length {
            if max_length > 100 {
                return ParameterInputType::TextArea;
            }
        }

        if let Some(description) = &param.description {
            if description.to_lowercase().contains("multiline")
                || description.to_lowercase().contains("text block")
            {
                return ParameterInputType::TextArea;
            }
        }

        // Default to single-line text
        ParameterInputType::Text
    }

    /// Open the AWS resource picker for a parameter
    fn open_resource_picker(&mut self, param: &ParameterInfo) {
        if let Some(aws_resource_type) = &param.aws_resource_type {
            if let Some(resource_lookup_service) = &self.resource_lookup_service {
                self.current_picker_parameter = Some(param.name.clone());
                self.resource_picker.open_for_parameter(
                    param.name.clone(),
                    aws_resource_type.clone(),
                    self.account_id.clone(),
                    self.region.clone(),
                    resource_lookup_service.clone(),
                );
            }
        }
    }

    /// Validate a single parameter
    fn validate_parameter(&mut self, param: &ParameterInfo, value: &str) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: true,
            error_message: None,
            warning_message: None,
        };

        // Check required parameters
        if value.is_empty() && param.default_value.is_none() {
            result.is_valid = false;
            result.error_message = Some("This parameter is required".to_string());
            self.validation_results
                .insert(param.name.clone(), result.clone());
            return result;
        }

        // Skip validation if empty and has default
        if value.is_empty() {
            self.validation_results
                .insert(param.name.clone(), result.clone());
            return result;
        }

        // Validate allowed values
        if let Some(allowed_values) = &param.allowed_values {
            if !allowed_values.contains(&value.to_string()) {
                result.is_valid = false;
                result.error_message = Some(format!(
                    "Value must be one of: {}",
                    allowed_values.join(", ")
                ));
            }
        }

        // Validate pattern
        if let Some(pattern) = &param.allowed_pattern {
            if let Ok(regex) = Regex::new(pattern) {
                if !regex.is_match(value) {
                    result.is_valid = false;
                    result.error_message = Some(format!("Value must match pattern: {}", pattern));
                }
            }
        }

        // Validate length constraints
        if let Some(min_length) = param.min_length {
            if value.len() < min_length as usize {
                result.is_valid = false;
                result.error_message = Some(format!(
                    "Value must be at least {} characters long",
                    min_length
                ));
            }
        }

        if let Some(max_length) = param.max_length {
            if value.len() > max_length as usize {
                result.is_valid = false;
                result.error_message = Some(format!(
                    "Value must be no more than {} characters long",
                    max_length
                ));
            }
        }

        // Validate number constraints
        if param.parameter_type.contains("Number") {
            if let Ok(num_value) = value.parse::<f64>() {
                if let Some(min_value) = param.min_value {
                    if num_value < min_value {
                        result.is_valid = false;
                        result.error_message =
                            Some(format!("Value must be at least {}", min_value));
                    }
                }

                if let Some(max_value) = param.max_value {
                    if num_value > max_value {
                        result.is_valid = false;
                        result.error_message =
                            Some(format!("Value must be no more than {}", max_value));
                    }
                }
            } else {
                result.is_valid = false;
                result.error_message = Some("Value must be a valid number".to_string());
            }
        }

        self.validation_results
            .insert(param.name.clone(), result.clone());
        result
    }

    /// Validate all parameters
    fn validate_all_parameters(&mut self) -> bool {
        let mut all_valid = true;

        for param in &self.parameters.clone() {
            let value = self
                .parameter_values
                .get(&param.name)
                .cloned()
                .unwrap_or_default();
            let result = self.validate_parameter(param, &value);
            if !result.is_valid {
                all_valid = false;
            }
        }

        if all_valid {
            self.success_message = Some("All parameters are valid".to_string());
            self.error_message = None;
        } else {
            self.error_message = Some("Some parameters have validation errors".to_string());
            self.success_message = None;
        }

        all_valid
    }

    /// Load parameter history (placeholder implementation)
    fn load_parameter_history(&mut self) {
        // This would load from project files or parameter store
        debug!(
            "Loading parameter history for environment {}",
            self.environment
        );
        // Implementation would go here
    }

    /// Get the final parameter values
    pub fn get_parameter_values(&self) -> HashMap<String, String> {
        self.parameter_values.clone()
    }

    /// Get parameters that should be stored as defaults
    pub fn get_store_as_default_parameters(&self) -> HashMap<String, ParameterSource> {
        self.store_as_default_options
            .iter()
            .filter_map(|(name, &should_store)| {
                if should_store {
                    self.selected_store_method
                        .get(name)
                        .map(|source| (name.clone(), source.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_dialog_creation() {
        let dialog = ParameterInputDialog::new();
        assert_eq!(dialog.state, ParameterDialogState::Closed);
        assert!(!dialog.is_open);
    }

    #[test]
    fn test_determine_input_type() {
        let dialog = ParameterInputDialog::new();

        // Test select type
        let select_param = ParameterInfo {
            name: "TestParam".to_string(),
            parameter_type: "String".to_string(),
            description: None,
            default_value: None,
            allowed_values: Some(vec!["option1".to_string(), "option2".to_string()]),
            allowed_pattern: None,
            constraint_description: None,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            no_echo: false,
            is_aws_specific: false,
            aws_resource_type: None,
            is_sensitive: false,
            validation_hints: Vec::new(),
        };

        assert_eq!(
            dialog.determine_input_type(&select_param),
            ParameterInputType::Select
        );

        // Test AWS resource picker
        let aws_param = ParameterInfo {
            name: "TestParam".to_string(),
            parameter_type: "AWS::EC2::VPC::Id".to_string(),
            description: None,
            default_value: None,
            allowed_values: None,
            allowed_pattern: None,
            constraint_description: None,
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            no_echo: false,
            is_aws_specific: true,
            aws_resource_type: Some("AWS::EC2::VPC".to_string()),
            is_sensitive: false,
            validation_hints: Vec::new(),
        };

        assert_eq!(
            dialog.determine_input_type(&aws_param),
            ParameterInputType::AwsResourcePicker
        );
    }

    #[test]
    fn test_parameter_validation() {
        let mut dialog = ParameterInputDialog::new();

        let param = ParameterInfo {
            name: "TestParam".to_string(),
            parameter_type: "String".to_string(),
            description: None,
            default_value: None,
            allowed_values: Some(vec!["valid1".to_string(), "valid2".to_string()]),
            allowed_pattern: None,
            constraint_description: None,
            min_length: Some(3),
            max_length: Some(10),
            min_value: None,
            max_value: None,
            no_echo: false,
            is_aws_specific: false,
            aws_resource_type: None,
            is_sensitive: false,
            validation_hints: Vec::new(),
        };

        // Test valid value
        let result = dialog.validate_parameter(&param, "valid1");
        assert!(result.is_valid);

        // Test invalid value (not in allowed values)
        let result = dialog.validate_parameter(&param, "invalid");
        assert!(!result.is_valid);

        // Test invalid value (too short)
        let result = dialog.validate_parameter(&param, "ab");
        assert!(!result.is_valid);
    }
}
