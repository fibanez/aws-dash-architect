use crate::app::cfn_resources::{load_property_type_definitions, PropertyDefinition};
use crate::app::dashui::keyboard_navigation::{ElementAction, NavigableElement};
use crate::app::dashui::navigable_widgets::{NavigableWidgetManager, WidgetRegistrar};
use crate::{register_button, register_text_input};
use eframe::egui::{self, Color32, Grid, RichText, ScrollArea, Window};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::error;

// Static variable to track focus status for property type windows
static mut PROPERTY_TYPE_FOCUS_SET: bool = false;

/// Represents a form window for editing a CloudFormation property type
pub struct PropertyTypeFormWindow {
    /// Whether to show the window
    pub show: bool,

    /// The property type (e.g., AWS::EC2::Instance.BlockDeviceMapping)
    pub property_type: String,

    /// The property path (for display and identification)
    pub property_path: String,

    /// The properties of the property type
    pub properties: HashMap<String, String>,

    /// The property definitions from CloudFormation specification
    pub property_definitions: Option<HashMap<String, PropertyDefinition>>,

    /// Error message if any
    pub error_message: Option<String>,

    /// Callback when the property type form is saved
    pub on_save: Option<Box<dyn FnMut(String)>>,

    /// Region to use for loading property definitions
    region: String,

    /// Pending sub-forms for nested property types
    pub pending_sub_forms: Vec<(String, String)>,

    /// Window focus tracking id
    window_id: u64,

    /// Widget manager for keyboard navigation
    widget_manager: NavigableWidgetManager,
}

impl PropertyTypeFormWindow {
    /// Create a new property type form window
    pub fn new(region: String) -> Self {
        // Generate a random ID for this window to help with focus tracking
        let window_id = rand::random::<u64>();

        Self {
            show: false,
            property_type: String::new(),
            property_path: String::new(),
            properties: HashMap::new(),
            property_definitions: None,
            error_message: None,
            on_save: None,
            region,
            pending_sub_forms: Vec::new(),
            window_id,
            widget_manager: NavigableWidgetManager::new(),
        }
    }

    /// Open the window for a property type
    pub fn open(
        &mut self,
        property_type: String,
        property_path: String,
        initial_values: Option<HashMap<String, String>>,
        on_save: impl FnMut(String) + 'static,
    ) {
        self.property_type = property_type.clone();
        self.property_path = property_path;
        self.properties = initial_values.unwrap_or_default();
        self.error_message = None;
        self.on_save = Some(Box::new(on_save));
        self.pending_sub_forms.clear();

        // Reset focus flag for the window
        unsafe {
            PROPERTY_TYPE_FOCUS_SET = false;
        }

        // Log the attempt to load property type definitions
        tracing::debug!(
            "Attempting to load property type definitions for: '{}' in region: '{}'",
            property_type,
            self.region
        );

        // Load property type definitions
        match load_property_type_definitions(&self.region, &property_type) {
            Ok(props) => {
                self.property_definitions = Some(props);

                // Initialize required properties with empty strings
                if let Some(defs) = &self.property_definitions {
                    for (prop_name, prop_def) in defs {
                        if prop_def.required && !self.properties.contains_key(prop_name) {
                            self.properties.insert(prop_name.clone(), String::new());
                        }
                    }
                }

                self.show = true;
            }
            Err(e) => {
                error!("Failed to load property type definitions: {}", e);
                self.error_message =
                    Some(format!("Failed to load property type definitions: {}", e));
                self.show = true; // Show anyway to display the error
            }
        }
    }

    /// Show the property type form window
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut form_saved = false;

        if !self.show {
            return form_saved;
        }

        // First, process any pending sub-form creation
        let pending = std::mem::take(&mut self.pending_sub_forms);
        for (type_name, path) in pending {
            // In a real implementation, you would create and manage the nested property forms here
            // For this prototype, we're just storing the pending sub-forms so they can be passed to
            // the parent window for processing
            self.pending_sub_forms.push((type_name, path));
        }

        // We'll use a local flag to track if we should close the window
        let mut close_window = false;

        // Create a unique window ID for this property type window
        let window_id = egui::Id::new(format!("property_type_window_{}", self.window_id));

        // Handle Escape key for this specific window
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            // Check if this window has focus
            if ctx.memory(|mem| mem.data.get_temp::<bool>(window_id).unwrap_or(false)) {
                close_window = true;
                // Consume the event to prevent it from bubbling up
                ctx.input_mut(|i| i.events.clear());
            }
        }

        // Prepare values that we'll need in the UI closure to avoid borrowing issues
        let property_type = self.property_type.clone();
        let error_message = self.error_message.clone();

        // Prepare json_value in advance
        let json_value = self.to_json_value();

        // Get just the property type name without the full path for display
        let display_name = property_type
            .split('.')
            .next_back()
            .unwrap_or(&property_type);

        // Track if this is the first time showing this window (for focusing first property)
        let focus_first_property = unsafe { !PROPERTY_TYPE_FOCUS_SET };
        let window_id_clone = window_id;

        // Extract show flag to avoid borrowing issues
        let mut show_window = self.show;

        let _window_result = Window::new(format!("Property Type: {}", display_name))
            .open(&mut show_window)
            .min_width(400.0)
            .min_height(300.0)
            .id(window_id)
            .default_pos(ctx.screen_rect().center())
            .show(ctx, |ui| {
                // Start widget registration for this frame with UI context for clipping
                self.widget_manager.start_frame_with_ui_context(
                    ui,
                    format!("property_type_form_{}", self.window_id),
                    self.window_title()
                );

                // Clear stale actions (older than 5 seconds)
                self.widget_manager.clear_stale_actions(5000);

                // Mark this window as having focus
                ctx.memory_mut(|mem| mem.data.insert_temp(window_id_clone, true));
                if let Some(error) = &error_message {
                    ui.colored_label(Color32::from_rgb(220, 50, 50), error);
                    ui.separator();
                }

                // Property type information
                ui.heading(&property_type);
                ui.separator();

                // Property grid layout
                if let Some(prop_defs) = &self.property_definitions {
                    ScrollArea::vertical().show(ui, |ui| {
                        Grid::new("property_type_properties_grid")
                            .num_columns(2)
                            .spacing([10.0, 10.0])
                            .striped(true)
                            .show(ui, |ui| {
                                // Sort properties with required properties first
                                let mut sorted_props: Vec<(&String, &PropertyDefinition)> =
                                    prop_defs.iter().collect();

                                sorted_props.sort_by(|a, b| match (a.1.required, b.1.required) {
                                    (true, false) => std::cmp::Ordering::Less,
                                    (false, true) => std::cmp::Ordering::Greater,
                                    _ => a.0.cmp(b.0),
                                });

                                // Variable to track if we've focused on the first property
                                let mut focused_first_property = unsafe { PROPERTY_TYPE_FOCUS_SET };

                                for (prop_name, prop_def) in sorted_props {
                                    // Property name with required indicator
                                    let label_text = if prop_def.required {
                                        RichText::new(format!("{}*", prop_name))
                                            .strong()
                                            .color(Color32::from_rgb(220, 120, 120))
                                    } else {
                                        RichText::new(prop_name)
                                    };

                                    ui.horizontal(|ui| {
                                        ui.label(label_text);
                                    });

                                    // Get the property value or initialize it
                                    if !self.properties.contains_key(prop_name) {
                                        self.properties.insert(prop_name.clone(), String::new());
                                    }

                                    // Property value editor
                                    let prop_value = self.properties.get_mut(prop_name).unwrap();

                                    // Check if this is a property type (complex type)
                                    if let Some(type_name) = &prop_def.type_name {
                                        if !is_primitive_property_type(type_name) {
                                            // Get current value
                                            let current_value = self.properties.get(prop_name).unwrap_or(&String::new()).clone();

                                            // This is a complex property type - show a button instead of a text field
                                            let sub_property_path = format!("{}.{}", self.property_path, prop_name);

                                            // Create button text based on current state
                                            let button_text = if current_value.is_empty() || current_value == "{}" {
                                                format!("Configure {}", type_name)
                                            } else {
                                                format!("Edit {}", type_name)
                                            };

                                            // Create a brightly colored button that stands out against any background
                                            let accent_color = ui.visuals().selection.bg_fill;
                                            let button = egui::Button::new(egui::RichText::new(button_text).strong())
                                                .fill(accent_color) // Use selection color for high contrast
                                                .stroke(egui::Stroke::new(1.5, ui.visuals().selection.stroke.color)) // Stronger border
                                                .corner_radius(4.0) // Rounded corners
                                                .min_size(egui::vec2(120.0, 24.0)); // Consistent minimum size

                                            if ui.add(button).clicked() {
                                                // Get the fully qualified property type
                                                let property_type = if type_name.contains("::") {
                                                    // If it's already a fully qualified AWS type, use it directly
                                                    type_name.clone()
                                                } else {
                                                    // For nested property types, we need to extract the base resource type
                                                    // The resource type is the AWS::Service::Resource part before the first dot
                                                    if let Some(resource_base) = self.property_type.split('.').next() {
                                                        // Create a proper property type in the format AWS::Service::Resource.PropertyTypeName
                                                        format!("{}.{}", resource_base, type_name)
                                                    } else {
                                                        // Fallback case, though this shouldn't happen with valid property types
                                                        type_name.clone()
                                                    }
                                                };

                                                // Add debug logging
                                                tracing::debug!("Creating nested property type form for: '{}' with path: '{}'",
                                                                property_type, sub_property_path);

                                                self.pending_sub_forms.push((property_type, sub_property_path));
                                            }
                                            ui.end_row();
                                            continue;
                                        }
                                    }

                                    // Different editors based on type
                                    if let Some(primitive_type) = &prop_def.primitive_type {
                                        match primitive_type.as_str() {
                                            "Boolean" => {
                                                let mut bool_value = prop_value == "true";
                                                let response = ui.checkbox(&mut bool_value, "");
                                                if response.changed() {
                                                    *prop_value = bool_value.to_string();
                                                }

                                                // Focus on the first field if not focused yet
                                                if !focused_first_property && focus_first_property {
                                                    response.request_focus();
                                                    focused_first_property = true;
                                                    unsafe { PROPERTY_TYPE_FOCUS_SET = true; }
                                                }
                                            }
                                            "Number" | "Integer" => {
                                                // Store response and check for auto-focus
                                                let response = ui.text_edit_singleline(prop_value);

                                                // Register the text input with navigation system
                                                let input_id = format!("property_input_{}", prop_name);
                                                register_text_input!(self.widget_manager, response, input_id, format!("Property: {}", prop_name));

                                                // Focus on the first field if not focused yet
                                                if !focused_first_property && focus_first_property {
                                                    response.request_focus();
                                                    focused_first_property = true;
                                                    unsafe { PROPERTY_TYPE_FOCUS_SET = true; }
                                                }

                                                // Add hover text to a clone of the response since on_hover_text takes ownership
                                                response.clone().on_hover_text("Enter a number");
                                            }
                                            _ => {
                                                // String or other types
                                                let response = ui.text_edit_singleline(prop_value);

                                                // Focus on the first field if not focused yet
                                                if !focused_first_property && focus_first_property {
                                                    response.request_focus();
                                                    focused_first_property = true;
                                                    unsafe { PROPERTY_TYPE_FOCUS_SET = true; }
                                                }

                                                // Add hover text to a clone of the response
                                                response.clone().on_hover_text("Enter text value");
                                            }
                                        }
                                    } else if let Some(type_name) = &prop_def.type_name {
                                        match type_name.as_str() {
                                            "List" => {
                                                let response = ui.text_edit_multiline(prop_value);

                                                // Focus on the first field if not focused yet
                                                if !focused_first_property && focus_first_property {
                                                    response.request_focus();
                                                    focused_first_property = true;
                                                    unsafe { PROPERTY_TYPE_FOCUS_SET = true; }
                                                }

                                                // Add hover text to a clone of the response
                                                response.clone().on_hover_text("Enter comma-separated values");
                                            }
                                            "Map" => {
                                                let response = ui.text_edit_multiline(prop_value);

                                                // Focus on the first field if not focused yet
                                                if !focused_first_property && focus_first_property {
                                                    response.request_focus();
                                                    focused_first_property = true;
                                                    unsafe { PROPERTY_TYPE_FOCUS_SET = true; }
                                                }

                                                // Add hover text to a clone of the response
                                                response.clone().on_hover_text("Enter JSON object");
                                            }
                                            _ => {
                                                // Other simple types
                                                let response = ui.text_edit_singleline(prop_value);

                                                // Focus on the first field if not focused yet
                                                if !focused_first_property && focus_first_property {
                                                    response.request_focus();
                                                    focused_first_property = true;
                                                    unsafe { PROPERTY_TYPE_FOCUS_SET = true; }
                                                }

                                                // Add hover text to a clone of the response
                                                response.clone().on_hover_text(format!("Type: {}", type_name));
                                            }
                                        }
                                    } else {
                                        // Fallback
                                        let response = ui.text_edit_singleline(prop_value);

                                        // Focus on the first field if not focused yet
                                        if !focused_first_property && focus_first_property {
                                            response.request_focus();
                                            focused_first_property = true;
                                            unsafe { PROPERTY_TYPE_FOCUS_SET = true; }
                                        }

                                        // Add hover text to a clone of the response
                                        response.clone().on_hover_text("Enter value");
                                    }

                                    ui.end_row();
                                }
                            });
                    });
                } else {
                    ui.label("No property type definitions available.");
                }

                ui.separator();

                // Buttons
                ui.horizontal(|ui| {
                    let cancel_button = ui.button("Cancel");

                    // Register the cancel button
                    register_button!(self.widget_manager, cancel_button, "property_form_cancel_button", "Cancel");

                    let should_cancel = cancel_button.clicked() || self.widget_manager.should_element_be_clicked("property_form_cancel_button");

                    if should_cancel {
                        // Consume the pending action if it was triggered by keyboard
                        if self.widget_manager.should_element_be_clicked("property_form_cancel_button") {
                            let _consumed_actions = self.widget_manager.consume_pending_actions("property_form_cancel_button");
                            tracing::info!("🎯 Keyboard navigation activated cancel button in PropertyTypeFormWindow");
                        }
                        close_window = true;
                    }

                    let apply_button = ui.button("Apply");

                    // Register the apply button
                    register_button!(self.widget_manager, apply_button, "property_form_apply_button", "Apply");

                    let should_apply = apply_button.clicked() || self.widget_manager.should_element_be_clicked("property_form_apply_button");

                    if should_apply {
                        // Consume the pending action if it was triggered by keyboard
                        if self.widget_manager.should_element_be_clicked("property_form_apply_button") {
                            let _consumed_actions = self.widget_manager.consume_pending_actions("property_form_apply_button");
                            tracing::info!("🎯 Keyboard navigation activated apply button in PropertyTypeFormWindow");
                        }

                        // We've already done pre-validation when creating json_value
                        // Convert JSON to string
                        let json_string = serde_json::to_string_pretty(&json_value).unwrap_or_default();

                        // Call the save callback
                        if let Some(on_save) = &mut self.on_save {
                            on_save(json_string);
                        }

                        form_saved = true;
                        close_window = true;
                    }
                });

                // Complete frame processing and update widget states
                self.widget_manager.complete_frame(ui.ctx());
            });

        // Update the show flag
        self.show = show_window;

        // Close the window if requested
        if close_window {
            self.show = false;

            // Reset the focus flag when window is closed
            unsafe {
                PROPERTY_TYPE_FOCUS_SET = false;
            }
        }

        form_saved
    }

    /// Convert the properties to a JSON value
    pub fn to_json_value(&self) -> Value {
        let mut json_obj = json!({});

        if let Some(prop_defs) = &self.property_definitions {
            for (prop_name, value) in &self.properties {
                if value.trim().is_empty() {
                    continue; // Skip empty values
                }

                if let Some(prop_def) = prop_defs.get(prop_name) {
                    let json_value = if let Some(primitive_type) = &prop_def.primitive_type {
                        match primitive_type.as_str() {
                            "Boolean" => Value::Bool(value.to_lowercase() == "true"),
                            "Number" => match value.parse::<f64>() {
                                Ok(num) => match serde_json::Number::from_f64(num) {
                                    Some(n) => Value::Number(n),
                                    None => Value::String(value.clone()),
                                },
                                Err(_) => Value::String(value.clone()),
                            },
                            "Integer" => match value.parse::<i64>() {
                                Ok(num) => Value::Number(num.into()),
                                Err(_) => Value::String(value.clone()),
                            },
                            _ => Value::String(value.clone()),
                        }
                    } else if let Some(type_name) = &prop_def.type_name {
                        match type_name.as_str() {
                            "List" => {
                                // Parse comma-separated values
                                let values: Vec<String> = value
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();

                                Value::Array(values.into_iter().map(Value::String).collect())
                            }
                            "Map" => {
                                // Try to parse as JSON
                                match serde_json::from_str::<Value>(value) {
                                    Ok(v) => v,
                                    Err(_) => Value::String(value.clone()),
                                }
                            }
                            _ => {
                                // Check if it's a JSON string (for complex types)
                                if value.trim().starts_with('{') && value.trim().ends_with('}') {
                                    match serde_json::from_str::<Value>(value) {
                                        Ok(v) => v,
                                        Err(_) => Value::String(value.clone()),
                                    }
                                } else {
                                    Value::String(value.clone())
                                }
                            }
                        }
                    } else {
                        Value::String(value.clone())
                    };

                    json_obj[prop_name] = json_value;
                }
            }
        } else {
            // Without property definitions, just add all as strings
            for (prop_name, value) in &self.properties {
                if !value.trim().is_empty() {
                    json_obj[prop_name] = Value::String(value.clone());
                }
            }
        }

        json_obj
    }

    /// Get pending sub-forms that need to be created
    pub fn get_pending_sub_forms(&mut self) -> Vec<(String, String)> {
        let pending = self.pending_sub_forms.clone();
        self.pending_sub_forms.clear();
        pending
    }

    /// Update a property value with JSON from a sub-form
    pub fn update_property_value(&mut self, property_path: &str, json_value: &str) {
        // Extract the property name from the path
        if let Some(prop_name) = property_path.split('.').next_back() {
            self.properties
                .insert(prop_name.to_string(), json_value.to_string());
        }
    }

    /// Queue an action to be executed on a widget in this window
    pub fn queue_widget_action(&mut self, element_id: String, action: ElementAction) {
        self.widget_manager.queue_action(element_id, action);
    }

    /// Collect navigable elements from this property type form window
    /// This method returns real registered widgets from the NavigableWidgetManager
    pub fn collect_navigable_elements(&self) -> Vec<NavigableElement> {
        if !self.show {
            return Vec::new(); // Window is not visible, no elements to collect
        }

        // Get real elements from the widget manager
        let elements = self.widget_manager.collector().get_elements().to_vec();

        // Log widget collection results for debugging
        tracing::info!("🎯 PropertyTypeFormWindow::collect_navigable_elements - Collected {} REAL elements for window '{}'",
                       elements.len(), self.window_title());

        // Log summary of element types for debugging
        if !elements.is_empty() {
            let mut type_counts = std::collections::HashMap::new();
            for element in &elements {
                let widget_type = element
                    .metadata
                    .get("widget_type")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                *type_counts.entry(widget_type).or_insert(0) += 1;
            }
            tracing::info!(
                "🎯 PropertyTypeFormWindow - Real widget types breakdown: {:?}",
                type_counts
            );

            // Log a few example elements for verification
            for (i, element) in elements.iter().take(3).enumerate() {
                tracing::info!("🎯 PropertyTypeFormWindow - Element {}: id='{}' type={:?} rect={:?} enabled={}",
                               i + 1, element.id, element.element_type, element.rect, element.enabled);
            }
        } else {
            tracing::warn!("❌ PropertyTypeFormWindow - No real elements captured - widget registration may not be working");
        }

        elements
    }

    /// Get the window title for this property type form
    pub fn window_title(&self) -> String {
        if self.property_path.is_empty() {
            format!("Property Type: {}", self.property_type)
        } else {
            format!("Edit Property: {}", self.property_path)
        }
    }

    /// Check if this window is open
    pub fn is_open(&self) -> bool {
        self.show
    }
}

/// Determine if a type name represents a primitive type
fn is_primitive_property_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "String" | "Integer" | "Boolean" | "Double" | "Long" | "Timestamp" | "List" | "Map"
    )
}
