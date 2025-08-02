use crate::app::cfn_resource_policies::ResourcePolicyManager;
use crate::app::cfn_resources::{load_property_definitions, PropertyDefinition};
use crate::app::dashui::keyboard_navigation::{
    ElementAction, KeyEventResult, NavigableElement, NavigableWindow, NavigationCommand,
    NavigationContext, NavigationMode,
};
use crate::app::dashui::navigable_widgets::{NavigableWidgetManager, WidgetRegistrar};
use crate::app::dashui::property_type_form_window::PropertyTypeFormWindow;
use crate::app::dashui::reference_picker_window::ReferencePickerWindow;
use crate::app::dashui::value_editor_window::ValueEditorWindow;
use crate::app::dashui::window_focus::{FocusableWindow, FormShowParams};
use crate::app::projects::{CloudFormationResource, Project};
use crate::{register_button, register_text_input};
use eframe::egui::{self, Button, Color32, Grid, RichText, ScrollArea, Window};
use serde_json::json;
use std::collections::HashMap;
use tracing::error;

/// Type alias for pending sub-forms: (property_type_name, property_path, initial_values)
type PendingSubForm = (String, String, Option<HashMap<String, String>>);

/// Simple property display item for flat list rendering
#[derive(Clone)]
pub struct PropertyDisplayItem {
    pub name: String,
    pub definition: PropertyDefinition,
    pub value: serde_json::Value,
    pub category: PropertyCategory,
    pub indent_level: u8,
}

/// Property categorization for display ordering
#[derive(Clone, PartialEq, Eq)]
pub enum PropertyCategory {
    Required,
    WithValue,
    Empty,
}

// Static variable to track focus status
static mut FOCUS_SET: bool = false;

/// Represents the resource form window for editing CloudFormation resource properties
pub struct ResourceFormWindow {
    /// Whether to show the window
    pub show: bool,

    /// The resource type (e.g., AWS::S3::Bucket)
    pub resource_type: String,

    /// The resource ID (logical ID in CloudFormation)
    pub resource_id: String,

    /// The properties of the resource
    pub properties: HashMap<String, serde_json::Value>,

    /// Resource attributes
    pub depends_on: Vec<String>,
    pub condition: String,
    pub metadata: String,
    pub deletion_policy: String,
    pub update_replace_policy: String,
    pub creation_policy: String,
    pub update_policy: String,

    /// The property definitions from CloudFormation specification
    pub property_definitions: Option<HashMap<String, PropertyDefinition>>,

    /// Error message if any
    pub error_message: Option<String>,

    /// Whether this is a new resource or editing an existing one
    pub is_new: bool,

    /// Callback when the resource is saved
    pub on_save: Option<Box<dyn FnMut(CloudFormationResource)>>,

    /// Property type form windows for nested property forms
    pub property_type_forms: Vec<PropertyTypeFormWindow>,

    /// Value editor window for property value editing
    value_editor: ValueEditorWindow,

    /// Reference picker window for CloudFormation references
    reference_picker: ReferencePickerWindow,

    /// AWS region for loading property type definitions
    region: String,

    /// Pending sub-forms to create
    pending_sub_forms: Vec<PendingSubForm>,

    /// Property being edited by value editor
    editing_property: Option<String>,

    /// Property being edited by reference picker
    referencing_property: Option<String>,

    /// Whether to show properties without values (collapsed by default)
    show_empty_properties: bool,

    /// Widget manager for keyboard navigation
    widget_manager: NavigableWidgetManager,
}

impl Default for ResourceFormWindow {
    fn default() -> Self {
        Self {
            show: false,
            resource_type: String::new(),
            resource_id: String::new(),
            properties: HashMap::new(),
            depends_on: Vec::new(),
            condition: String::new(),
            metadata: String::new(),
            deletion_policy: String::new(),
            update_replace_policy: String::new(),
            creation_policy: String::new(),
            update_policy: String::new(),
            property_definitions: None,
            error_message: None,
            is_new: true,
            on_save: None,
            property_type_forms: Vec::new(),
            value_editor: ValueEditorWindow::new(),
            reference_picker: ReferencePickerWindow::new(),
            region: "us-east-1".to_string(),
            pending_sub_forms: Vec::new(),
            editing_property: None,
            referencing_property: None,
            show_empty_properties: false,
            widget_manager: NavigableWidgetManager::new(),
        }
    }
}

impl ResourceFormWindow {
    /// Create a new resource form window
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the window for a new resource
    pub fn open_new(
        &mut self,
        resource_type: String,
        project: &Project,
        on_save: impl FnMut(CloudFormationResource) + 'static,
    ) {
        self.resource_type = resource_type.clone();
        self.resource_id = format!(
            "{}Resource{}",
            resource_type.split("::").last().unwrap_or("Resource"),
            chrono::Utc::now().timestamp()
        );
        self.properties.clear();
        self.depends_on.clear();
        self.condition.clear();
        self.metadata.clear();
        self.deletion_policy.clear();
        self.update_replace_policy.clear();
        self.creation_policy.clear();
        self.update_policy.clear();
        self.error_message = None;
        self.is_new = true;
        self.on_save = Some(Box::new(on_save));
        self.property_type_forms.clear();
        self.pending_sub_forms.clear();

        // Reset focus flag for new window
        unsafe {
            FOCUS_SET = false;
        }

        // Store the region for property type definitions
        self.region = project.get_default_region();

        // Load property definitions
        match load_property_definitions(&self.region, &resource_type) {
            Ok(props) => {
                self.property_definitions = Some(props);

                // Initialize required properties with empty strings
                if let Some(defs) = &self.property_definitions {
                    for (prop_name, prop_def) in defs {
                        if prop_def.required {
                            self.properties.insert(
                                prop_name.clone(),
                                serde_json::Value::String(String::new()),
                            );
                        }
                    }
                }

                self.show = true;
            }
            Err(e) => {
                error!("Failed to load property definitions: {}", e);
                self.error_message = Some(format!("Failed to load property definitions: {}", e));
            }
        }
    }

    /// Open the window for an existing resource
    pub fn open_edit(
        &mut self,
        resource: CloudFormationResource,
        project: &Project,
        on_save: impl FnMut(CloudFormationResource) + 'static,
    ) {
        self.resource_type = resource.resource_type.clone();
        self.resource_id = resource.resource_id.clone();
        self.properties = resource.properties.clone();
        self.depends_on = resource.depends_on.unwrap_or_default();
        self.condition = resource.condition.unwrap_or_default();
        self.metadata = resource.metadata.unwrap_or_default();
        self.deletion_policy = resource.deletion_policy.unwrap_or_default();
        self.update_replace_policy = resource.update_replace_policy.unwrap_or_default();
        self.creation_policy = resource.creation_policy.unwrap_or_default();
        self.update_policy = resource.update_policy.unwrap_or_default();
        self.error_message = None;
        self.is_new = false;
        self.on_save = Some(Box::new(on_save));
        self.property_type_forms.clear();
        self.pending_sub_forms.clear();

        // Reset focus flag for edit window
        unsafe {
            FOCUS_SET = false;
        }

        // Store the region for property type definitions
        self.region = project.get_default_region();

        // Load property definitions
        match load_property_definitions(&self.region, &resource.resource_type) {
            Ok(props) => {
                self.property_definitions = Some(props);
                self.show = true;
            }
            Err(e) => {
                error!("Failed to load property definitions: {}", e);
                self.error_message = Some(format!("Failed to load property definitions: {}", e));
            }
        }
    }

    /// Queue an action to be executed on a widget in this window
    pub fn queue_widget_action(&mut self, element_id: String, action: ElementAction) {
        self.widget_manager.queue_action(element_id, action);
    }

    /// Show the resource form window
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        let mut resource_saved = false;

        if !self.show {
            return resource_saved;
        }

        // Handle value editor window
        if self.value_editor.show(ctx) {
            // Check if a value was saved
            if let Some(new_value) = self.value_editor.take_saved_value() {
                if let Some(prop_name) = &self.editing_property {
                    // Parse the new value and update properties
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&new_value) {
                        self.properties.insert(prop_name.clone(), json_value);
                    } else {
                        self.properties
                            .insert(prop_name.clone(), serde_json::Value::String(new_value));
                    }
                    self.editing_property = None;
                }
            }
        }

        // Handle reference picker window
        if self.reference_picker.show(ctx) {
            // Check if a reference was selected
            if let Some(reference) = self.reference_picker.take_selected_reference() {
                if let Some(prop_name) = &self.referencing_property {
                    // Parse the reference as JSON and update properties
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&reference) {
                        self.properties.insert(prop_name.clone(), json_value);
                    }
                    self.referencing_property = None;
                }
            }
        }

        // First, process any pending sub-form creation
        self.process_pending_sub_forms();

        // Handle property type forms first
        let mut sub_forms_updated = false;
        let mut forms_to_remove = Vec::new();

        for (i, form) in self.property_type_forms.iter_mut().enumerate() {
            if form.show(ctx) {
                // Form was saved - need to manually update the property values
                // Get the property name from the path
                if let Some(prop_name) = form.property_path.split('.').next_back() {
                    // Extract just the JSON without calling the callback
                    let json_value = form.to_json_value();

                    // Update the property value with the JSON value directly
                    self.properties.insert(prop_name.to_string(), json_value);
                }

                // If form is not visible anymore, we'll remove it later
                if !form.show {
                    forms_to_remove.push(i);
                }

                sub_forms_updated = true;
            }

            // Check for any pending sub-forms that were created
            let pending = form.get_pending_sub_forms();
            if !pending.is_empty() {
                for (type_name, path) in pending {
                    // Nested property type forms start with no initial values
                    self.pending_sub_forms.push((type_name, path, None));
                }
                sub_forms_updated = true;
            }
        }

        // Remove any closed forms (in reverse order to maintain indices)
        for i in forms_to_remove.into_iter().rev() {
            if i < self.property_type_forms.len() {
                self.property_type_forms.remove(i);
            }
        }

        // If any sub-forms were updated or have pending forms, process them
        if sub_forms_updated {
            self.process_pending_sub_forms();
        }

        let title = if self.is_new {
            format!("New Resource: {}", self.resource_type)
        } else {
            format!("Edit Resource: {}", self.resource_id)
        };

        // We'll use a local flag to track if we should close the window
        let mut close_window = false;

        // Get screen dimensions to calculate appropriate window size
        let screen_rect = ctx.screen_rect();
        let max_width = screen_rect.width() * 0.85; // 85% of screen width
        let max_height = screen_rect.height() * 0.85; // 85% of screen height

        let mut window_open = self.show;
        Window::new(title)
            .open(&mut window_open)
            .min_width(350.0)
            .min_height(400.0)
            .max_width(max_width)
            .max_height(max_height)
            .resizable(true)
            .default_pos(screen_rect.center()) // Center the window on the screen
            .show(ctx, |ui| {
                // Start widget registration for this frame with UI context for clipping
                self.widget_manager.start_frame_with_ui_context(
                    ui,
                    self.window_id().to_string(),
                    self.window_title()
                );

                // Clear stale actions (older than 5 seconds)
                self.widget_manager.clear_stale_actions(5000);

                // Top area with error message and resource ID
                if let Some(error) = &self.error_message {
                    ui.colored_label(Color32::from_rgb(220, 50, 50), error);
                    ui.separator();
                }

                // Resource ID field with auto-focus
                ui.horizontal(|ui| {
                    ui.label("Resource ID:");
                    let id_edit = ui.text_edit_singleline(&mut self.resource_id);

                    // Register the resource ID text input with navigation system
                    register_text_input!(self.widget_manager, id_edit, "resource_id_input", "Resource ID");

                    // Note: Auto-focus removed to allow keyboard navigation to work properly
                    // (Previous auto-focus on resource ID field interfered with hint mode)

                    // Add documentation link button for this resource type
                    let doc_button = ui.small_button("?")
                        .on_hover_text("Open AWS documentation for this resource type");

                    // Register the documentation button
                    register_button!(self.widget_manager, doc_button, "documentation_button", "Open AWS documentation");

                    if doc_button.clicked() || self.widget_manager.should_element_be_clicked("documentation_button") {
                        // Consume the pending action if it was triggered by keyboard
                        if self.widget_manager.should_element_be_clicked("documentation_button") {
                            let _consumed_actions = self.widget_manager.consume_pending_actions("documentation_button");
                            tracing::info!("🎯 Keyboard navigation activated documentation button");
                        }
                        // Create a documentation URL for the resource type
                        let resource_type_parts: Vec<&str> = self.resource_type.split("::").collect();
                        if resource_type_parts.len() >= 3 {
                            let service = resource_type_parts[1].to_lowercase();
                            let resource = resource_type_parts[2].to_lowercase();

                            // AWS documentation URL format
                            let doc_url = format!(
                                "https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-resource-{}-{}.html",
                                service,
                                resource
                            );

                            if let Err(e) = open::that(&doc_url) {
                                error!("Failed to open resource documentation URL: {}", e);
                                self.error_message = Some(format!("Failed to open documentation: {}", e));
                            }
                        }
                    }
                });

                ui.separator();

                // Make the property contents scrollable
                ScrollArea::vertical()
                    .max_height(max_height - 120.0) // Allow space for window title, ID field, and buttons
                    .show(ui, |ui| {
                        // Simple property list rendering - no grids or complex layouts
                        if self.property_definitions.is_some() {
                            self.render_simple_property_list(ui);
                        } else {
                            ui.label("No property definitions available for this resource type.");
                        }

                        // Resource Attributes Section
                        ui.separator();
                        ui.collapsing("Resource Attributes", |ui| {
                            Grid::new("resource_attributes_grid")
                                .num_columns(2)
                                .spacing([10.0, 10.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    // DependsOn
                                    ui.label("DependsOn:");
                                    ui.vertical(|ui| {
                                        // TODO: Get list of other resources in template for dropdown
                                        let depends_on_str = self.depends_on.join(", ");
                                        let mut temp_depends_on = depends_on_str.clone();
                                        if ui.text_edit_singleline(&mut temp_depends_on).changed() {
                                            self.depends_on = temp_depends_on
                                                .split(',')
                                                .map(|s| s.trim().to_string())
                                                .filter(|s| !s.is_empty())
                                                .collect();
                                        }
                                        ui.small("Comma-separated list of resource logical IDs");
                                    });
                                    ui.end_row();

                                    // Condition
                                    ui.label("Condition:");
                                    ui.vertical(|ui| {
                                        ui.text_edit_singleline(&mut self.condition);
                                        ui.small("Reference to a condition in the template");
                                    });
                                    ui.end_row();

                                    // Metadata
                                    ui.label("Metadata:");
                                    ui.vertical(|ui| {
                                        ui.text_edit_multiline(&mut self.metadata);
                                        ui.small("JSON metadata for the resource");
                                    });
                                    ui.end_row();

                                    // DeletionPolicy
                                    ui.label("DeletionPolicy:");
                                    ui.vertical(|ui| {
                                        let available_policies = ResourcePolicyManager::get_available_deletion_policies(&self.resource_type);
                                        egui::ComboBox::from_id_salt("deletion_policy")
                                            .selected_text(if self.deletion_policy.is_empty() { "Select..." } else { &self.deletion_policy })
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut self.deletion_policy, String::new(), "None");
                                                for policy in available_policies {
                                                    ui.selectable_value(&mut self.deletion_policy, policy.to_string(), policy);
                                                }
                                            });
                                        ui.small("Policy for resource deletion");
                                    });
                                    ui.end_row();

                                    // UpdateReplacePolicy
                                    ui.label("UpdateReplacePolicy:");
                                    ui.vertical(|ui| {
                                        let available_policies = ResourcePolicyManager::get_update_replace_policy_options();
                                        egui::ComboBox::from_id_salt("update_replace_policy")
                                            .selected_text(if self.update_replace_policy.is_empty() { "Select..." } else { &self.update_replace_policy })
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut self.update_replace_policy, String::new(), "None");
                                                for policy in available_policies {
                                                    ui.selectable_value(&mut self.update_replace_policy, policy.to_string(), policy);
                                                }
                                            });
                                        ui.small("Policy for resource replacement during updates");
                                    });
                                    ui.end_row();

                                    // CreationPolicy (only show for supported resource types)
                                    if ResourcePolicyManager::supports_creation_policy(&self.resource_type) {
                                        ui.label("CreationPolicy:");
                                        ui.vertical(|ui| {
                                            ui.text_edit_multiline(&mut self.creation_policy);
                                            ui.small("JSON configuration for resource creation signals");

                                            // Add helper buttons for common configurations
                                            ui.horizontal(|ui| {
                                                let creation_policy_types = ResourcePolicyManager::get_creation_policy_types(&self.resource_type);
                                                for policy_type in creation_policy_types {
                                                    let desc = ResourcePolicyManager::get_creation_policy_description(&policy_type);
                                                    if ui.small_button(format!("{:?}", policy_type)).on_hover_text(desc).clicked() {
                                                        let template = ResourcePolicyManager::get_creation_policy_template(&policy_type);
                                                        self.creation_policy = serde_json::to_string_pretty(&template).unwrap_or_default();
                                                    }
                                                }
                                            });
                                        });
                                        ui.end_row();
                                    }

                                    // UpdatePolicy (only show for supported resource types)
                                    if ResourcePolicyManager::supports_update_policy(&self.resource_type) {
                                        ui.label("UpdatePolicy:");
                                        ui.vertical(|ui| {
                                            ui.text_edit_multiline(&mut self.update_policy);
                                            ui.small("JSON configuration for resource updates");

                                            // Add helper buttons for common configurations
                                            ui.horizontal(|ui| {
                                                let update_policy_types = ResourcePolicyManager::get_update_policy_types(&self.resource_type);
                                                for policy_type in update_policy_types {
                                                    let desc = ResourcePolicyManager::get_update_policy_description(&policy_type);
                                                    if ui.small_button(format!("{:?}", policy_type)).on_hover_text(desc).clicked() {
                                                        let template = ResourcePolicyManager::get_update_policy_template(&policy_type);
                                                        self.update_policy = serde_json::to_string_pretty(&template).unwrap_or_default();
                                                    }
                                                }
                                            });
                                        });
                                        ui.end_row();
                                    }
                                });
                        });
                    });

                ui.separator();

                // Buttons - outside the scroll area
                ui.horizontal(|ui| {
                    let cancel_button = ui.button("Cancel");

                    // Register the cancel button
                    register_button!(self.widget_manager, cancel_button, "cancel_button", "Cancel");

                    let should_cancel = cancel_button.clicked() || self.widget_manager.should_element_be_clicked("cancel_button");

                    if should_cancel {
                        // Consume the pending action if it was triggered by keyboard
                        if self.widget_manager.should_element_be_clicked("cancel_button") {
                            let _consumed_actions = self.widget_manager.consume_pending_actions("cancel_button");
                            tracing::info!("🎯 Keyboard navigation activated cancel button");
                        }
                        close_window = true;
                    }

                    let save_button =
                        ui.add_enabled(!self.resource_id.is_empty(), Button::new("Save"));

                    // Register the save button
                    register_button!(self.widget_manager, save_button, "save_button", "Save");

                    let should_save = save_button.clicked() || self.widget_manager.should_element_be_clicked("save_button");

                    if should_save {
                        // Consume the pending action if it was triggered by keyboard
                        if self.widget_manager.should_element_be_clicked("save_button") {
                            let _consumed_actions = self.widget_manager.consume_pending_actions("save_button");
                            tracing::info!("🎯 Keyboard navigation activated save button");
                        }
                        // Validate required properties
                        let mut missing_required = Vec::new();

                        if let Some(prop_defs) = self.property_definitions.clone() {
                            for (prop_name, prop_def) in prop_defs {
                                if prop_def.required {
                                    if let Some(value) = self.properties.get(&prop_name) {
                                        match value {
                                            serde_json::Value::String(s) if s.trim().is_empty() => {
                                                missing_required.push(prop_name.clone());
                                            }
                                            serde_json::Value::Null => {
                                                missing_required.push(prop_name.clone());
                                            }
                                            serde_json::Value::Array(arr) if arr.is_empty() => {
                                                missing_required.push(prop_name.clone());
                                            }
                                            serde_json::Value::Object(obj) if obj.is_empty() => {
                                                missing_required.push(prop_name.clone());
                                            }
                                            _ => {} // Value is present and non-empty
                                        }
                                    } else {
                                        missing_required.push(prop_name.clone());
                                    }
                                }
                            }
                        }

                        if !missing_required.is_empty() {
                            self.error_message = Some(format!(
                                "Missing required properties: {}",
                                missing_required.join(", ")
                            ));
                        } else {
                            // Create the resource
                            let mut resource = CloudFormationResource::new(
                                self.resource_id.clone(),
                                self.resource_type.clone(),
                            );
                            resource.properties = self.properties.clone();
                            resource.depends_on = if self.depends_on.is_empty() { None } else { Some(self.depends_on.clone()) };
                            resource.condition = if self.condition.is_empty() { None } else { Some(self.condition.clone()) };
                            resource.metadata = if self.metadata.is_empty() { None } else { Some(self.metadata.clone()) };
                            resource.deletion_policy = if self.deletion_policy.is_empty() { None } else { Some(self.deletion_policy.clone()) };
                            resource.update_replace_policy = if self.update_replace_policy.is_empty() { None } else { Some(self.update_replace_policy.clone()) };
                            resource.creation_policy = if self.creation_policy.is_empty() { None } else { Some(self.creation_policy.clone()) };
                            resource.update_policy = if self.update_policy.is_empty() { None } else { Some(self.update_policy.clone()) };

                            // Call the save callback
                            if let Some(on_save) = &mut self.on_save {
                                on_save(resource);
                            }

                            resource_saved = true;
                            close_window = true;
                        }
                    }
                });

            // Complete frame processing and update widget states
            self.widget_manager.complete_frame(ui.ctx());
            });

        // Update the window state
        self.show = window_open;

        // Close the window if requested
        if close_window {
            self.show = false;
            // Also close any property type forms
            for form in &mut self.property_type_forms {
                form.show = false;
            }

            // Reset the focus flag when window is closed
            unsafe {
                FOCUS_SET = false;
            }
        }

        resource_saved
    }

    /// Process any pending sub-forms that need to be created
    fn process_pending_sub_forms(&mut self) {
        let pending = std::mem::take(&mut self.pending_sub_forms);

        for (property_type, property_path, initial_values) in pending {
            // Create a new property type form
            let mut form = PropertyTypeFormWindow::new(self.region.clone());

            // Use a no-op callback - we'll handle the update in the show method
            let form_callback = {
                move |_json_value: String| {
                    // The actual update happens in the show method
                }
            };

            // Open the form with the provided initial values
            form.open(
                property_type,
                property_path.clone(),
                initial_values,
                form_callback,
            );

            // Store the form
            self.property_type_forms.push(form);
        }
    }

    // We now handle property updates directly in the show method

    /// Generate a CloudFormation template for the current resource
    pub fn generate_template(&self) -> anyhow::Result<String> {
        // Start building the CloudFormation template
        let resource_id = self.resource_id.clone();
        let resource_type = self.resource_type.clone();

        let mut template = json!({
            "AWSTemplateFormatVersion": "2010-09-09",
            "Description": format!("CloudFormation template for {}", resource_id),
            "Resources": {
                resource_id: {
                    "Type": resource_type,
                    "Properties": {}
                }
            }
        });

        // Convert string properties to appropriate JSON types
        let mut json_properties = json!({});

        if let Some(prop_defs) = &self.property_definitions {
            for (prop_name, value) in &self.properties {
                // Skip empty values based on JSON type
                match value {
                    serde_json::Value::String(s) if s.trim().is_empty() => continue,
                    serde_json::Value::Null => continue,
                    serde_json::Value::Array(arr) if arr.is_empty() => continue,
                    serde_json::Value::Object(obj) if obj.is_empty() => continue,
                    _ => {}
                }

                if let Some(_prop_def) = prop_defs.get(prop_name) {
                    // Values are already in JSON format, just use them directly
                    json_properties[prop_name] = value.clone();
                }
            }
        } else {
            // Without property definitions, just use values directly
            for (prop_name, value) in &self.properties {
                // Skip empty values based on JSON type
                match value {
                    serde_json::Value::String(s) if s.trim().is_empty() => continue,
                    serde_json::Value::Null => continue,
                    serde_json::Value::Array(arr) if arr.is_empty() => continue,
                    serde_json::Value::Object(obj) if obj.is_empty() => continue,
                    _ => {
                        json_properties[prop_name] = value.clone();
                    }
                }
            }
        }

        // Set the properties
        template["Resources"][&self.resource_id]["Properties"] = json_properties;

        // Convert to pretty JSON
        Ok(serde_json::to_string_pretty(&template)?)
    }

    // Note: format_property_value_preview method removed - now using
    // PropertyValueClassification::get_display_preview for better intrinsic function support

    /// Convert a property value to string for editing
    fn property_value_to_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            _ => serde_json::to_string_pretty(value).unwrap_or_default(),
        }
    }

    /// Get available resources for reference picker
    fn get_available_resources(&self) -> Vec<CloudFormationResource> {
        // This would typically come from the project context
        // For now, return an empty list - this would be provided by the caller
        Vec::new()
    }

    /// Check if a property has a meaningful value (not empty, null, or empty string)
    fn property_has_value(&self, value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::String(s) => !s.is_empty(),
            serde_json::Value::Null => false,
            serde_json::Value::Object(obj) => !obj.is_empty(),
            serde_json::Value::Array(arr) => !arr.is_empty(),
            serde_json::Value::Number(_) => true,
            serde_json::Value::Bool(_) => true,
        }
    }

    /// Get flat list of properties for simple rendering
    fn get_flat_property_list(&self) -> Vec<PropertyDisplayItem> {
        let mut items = Vec::new();

        if let Some(prop_defs) = &self.property_definitions {
            for (name, def) in prop_defs {
                let value = self
                    .properties
                    .get(name)
                    .cloned()
                    .unwrap_or(serde_json::Value::String(String::new()));

                let category = if def.required {
                    PropertyCategory::Required
                } else if self.property_has_value(&value) {
                    PropertyCategory::WithValue
                } else {
                    PropertyCategory::Empty
                };

                items.push(PropertyDisplayItem {
                    name: name.clone(),
                    definition: def.clone(),
                    value,
                    category,
                    indent_level: 0,
                });
            }
        }

        // Simple sort - no complex algorithms
        items.sort_by(|a, b| {
            use PropertyCategory::*;
            match (&a.category, &b.category) {
                (Required, Required) => a.name.cmp(&b.name),
                (Required, _) => std::cmp::Ordering::Less,
                (WithValue, Required) => std::cmp::Ordering::Greater,
                (WithValue, WithValue) => a.name.cmp(&b.name),
                (WithValue, Empty) => std::cmp::Ordering::Less,
                (Empty, Empty) => a.name.cmp(&b.name),
                (Empty, _) => std::cmp::Ordering::Greater,
            }
        });

        items
    }

    /// Render simple property list with manual column alignment
    fn render_simple_property_list(&mut self, ui: &mut egui::Ui) {
        let items = self.get_flat_property_list();

        let mut current_category: Option<PropertyCategory> = None;
        let mut empty_section_visible = self.show_empty_properties;
        let empty_count = items
            .iter()
            .filter(|i| matches!(i.category, PropertyCategory::Empty))
            .count();

        // First pass: measure all property names to find the maximum width
        let mut max_name_width: f32 = 0.0;

        for item in &items {
            // Skip measuring empty properties if section will be collapsed
            if matches!(item.category, PropertyCategory::Empty) && !empty_section_visible {
                continue;
            }

            let name_text = if item.definition.required {
                format!("{}*", item.name)
            } else {
                item.name.clone()
            };

            // We don't need to allocate anything, just measure text

            // Get the actual text size using the current style
            let text_style = egui::TextStyle::Body; // Same style for both required and non-required

            let font_id = ui
                .style()
                .text_styles
                .get(&text_style)
                .cloned()
                .unwrap_or_else(egui::FontId::default);

            let text_galley =
                ui.fonts(|fonts| fonts.layout_no_wrap(name_text, font_id, Color32::WHITE));

            max_name_width = max_name_width.max(text_galley.size().x);
        }

        // Add some padding to the max width
        max_name_width += 20.0;

        // Second pass: render with alignment
        for item in &items {
            // Handle category transitions without titles or separators for Required and WithValue
            if current_category != Some(item.category.clone()) {
                current_category = Some(item.category.clone());

                match item.category {
                    PropertyCategory::Required => {
                        // No title or separator - just start rendering properties
                    }
                    PropertyCategory::WithValue => {
                        ui.add_space(10.0);
                        // No title or separator - just start rendering properties
                    }
                    PropertyCategory::Empty => {
                        ui.add_space(10.0);
                        let arrow = if empty_section_visible { "▼" } else { "▶" };
                        if ui
                            .button(format!("{} Other Properties ({})", arrow, empty_count))
                            .clicked()
                        {
                            empty_section_visible = !empty_section_visible;
                            self.show_empty_properties = empty_section_visible;
                        }
                        if !empty_section_visible {
                            break; // Skip rendering empty properties
                        }
                        ui.separator();
                    }
                }
            }

            // Skip empty properties if section is collapsed
            if matches!(item.category, PropertyCategory::Empty) && !empty_section_visible {
                continue;
            }

            // Render property row with calculated alignment
            self.render_aligned_property_row(ui, item, max_name_width);
        }
    }

    /// Render a single property row with manual column alignment
    fn render_aligned_property_row(
        &mut self,
        ui: &mut egui::Ui,
        item: &PropertyDisplayItem,
        max_name_width: f32,
    ) {
        ui.horizontal(|ui| {
            // Simple indentation
            ui.add_space(item.indent_level as f32 * 20.0);

            // Property name column with fixed width
            ui.allocate_ui_with_layout(
                egui::Vec2::new(max_name_width, ui.available_height()),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    // Property name with required indicator and bold formatting for non-required
                    let name_text = if item.definition.required {
                        RichText::new(format!("{}*", item.name))
                            .color(Color32::from_rgb(220, 120, 120))
                    } else {
                        RichText::new(&item.name).strong() // Make property names bold
                    };
                    ui.label(name_text);
                },
            );

            // Equals sign column - fixed width for alignment
            ui.allocate_ui_with_layout(
                egui::Vec2::new(20.0, ui.available_height()),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label("=");
                },
            );

            // Value preview column - takes remaining space
            let preview = self.get_simple_value_preview(&item.value);
            ui.label(preview);

            // Buttons column - right aligned
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clear_button = ui.add_enabled(
                    self.property_has_value(&item.value),
                    egui::Button::new("Clear"),
                );

                // Register the clear button
                let clear_id = format!("clear_property_{}", item.name);
                register_button!(
                    self.widget_manager,
                    clear_button,
                    clear_id,
                    "Clear property value"
                );

                let should_clear = clear_button.clicked()
                    || self.widget_manager.should_element_be_clicked(&clear_id);

                if should_clear {
                    // Consume the pending action if it was triggered by keyboard
                    if self.widget_manager.should_element_be_clicked(&clear_id) {
                        let _consumed_actions =
                            self.widget_manager.consume_pending_actions(&clear_id);
                        tracing::info!(
                            "🎯 Keyboard navigation activated clear button for property: {}",
                            item.name
                        );
                    }

                    if item.definition.required {
                        self.properties
                            .insert(item.name.clone(), serde_json::Value::String(String::new()));
                    } else {
                        self.properties.remove(&item.name);
                    }
                }

                let ref_button = ui.button("Ref");

                // Register the ref button
                let ref_id = format!("ref_property_{}", item.name);
                register_button!(self.widget_manager, ref_button, ref_id, "Add reference");

                let should_ref =
                    ref_button.clicked() || self.widget_manager.should_element_be_clicked(&ref_id);

                if should_ref {
                    // Consume the pending action if it was triggered by keyboard
                    if self.widget_manager.should_element_be_clicked(&ref_id) {
                        let _consumed_actions =
                            self.widget_manager.consume_pending_actions(&ref_id);
                        tracing::info!(
                            "🎯 Keyboard navigation activated ref button for property: {}",
                            item.name
                        );
                    }

                    self.referencing_property = Some(item.name.clone());
                    self.reference_picker.open(
                        item.name.clone(),
                        Some(item.definition.clone()),
                        self.get_available_resources(),
                        |_| {}, // no-op callback, handled in show method
                    );
                }

                let edit_button = ui.button("Edit");

                // Register the edit button
                let edit_id = format!("edit_property_{}", item.name);
                register_button!(self.widget_manager, edit_button, edit_id, "Edit property");

                let should_edit = edit_button.clicked()
                    || self.widget_manager.should_element_be_clicked(&edit_id);

                if should_edit {
                    // Consume the pending action if it was triggered by keyboard
                    if self.widget_manager.should_element_be_clicked(&edit_id) {
                        let _consumed_actions =
                            self.widget_manager.consume_pending_actions(&edit_id);
                        tracing::info!(
                            "🎯 Keyboard navigation activated edit button for property: {}",
                            item.name
                        );
                    }
                    // Smart Edit button: automatically choose the right editor based on property type
                    if self.is_property_type_property(&item.definition) {
                        // This is a Property Type - open PropertyTypeFormWindow
                        if let Some(property_type_name) =
                            self.get_property_type_name(&item.definition)
                        {
                            let property_path = format!("{}.{}", self.resource_id, item.name);

                            // Convert current value to initial values for property type form
                            let initial_values = self.get_property_type_initial_values(&item.value);

                            // Store property info for the pending sub-form
                            self.pending_sub_forms.push((
                                property_type_name,
                                property_path,
                                initial_values,
                            ));
                        }
                    } else {
                        // This is a primitive type - open ValueEditorWindow (existing behavior)
                        self.editing_property = Some(item.name.clone());
                        self.value_editor.open(
                            format!("Edit {}", item.name),
                            Some(item.definition.clone()),
                            self.property_value_to_string(&item.value),
                            |_| {}, // no-op callback, handled in show method
                        );
                    }
                }
            });
        });
    }

    /// Get simple preview of property value
    fn get_simple_value_preview(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) if s.is_empty() => "(empty)".to_string(),
            serde_json::Value::String(s) => s.chars().take(30).collect(),
            serde_json::Value::Null => "(null)".to_string(),
            _ => serde_json::to_string(value)
                .unwrap_or("(invalid)".to_string())
                .chars()
                .take(30)
                .collect(),
        }
    }

    /// Check if a property definition refers to a CloudFormation Property Type
    /// (as opposed to a primitive type like String, Number, etc.)
    fn is_property_type_property(&self, prop_def: &PropertyDefinition) -> bool {
        // Check if type_name refers to a CloudFormation Property Type
        if let Some(type_name) = &prop_def.type_name {
            // Skip primitive types like List, Map, String, etc.
            if matches!(
                type_name.as_str(),
                "List"
                    | "Map"
                    | "String"
                    | "Number"
                    | "Boolean"
                    | "Integer"
                    | "Double"
                    | "Long"
                    | "Timestamp"
            ) {
                return false;
            }

            // If it contains AWS:: or looks like a property type, it's a property type
            // Property types follow patterns like:
            // - AWS::EC2::Instance.BlockDeviceMapping
            // - AWS::S3::Bucket.NotificationConfiguration
            // - AWS::Lambda::Function.Code
            type_name.contains("::") ||
            (type_name.contains(".") && !type_name.starts_with('.')) ||
            // Sometimes nested property types are referenced by just the property name
            // and we need to construct the full name from the parent resource type
            (!type_name.contains("::") && !type_name.contains('.') && type_name.len() > 1)
        } else {
            false
        }
    }

    /// Get the full Property Type name for a property definition
    /// This constructs the proper AWS::Service::Resource.PropertyType format
    fn get_property_type_name(&self, prop_def: &PropertyDefinition) -> Option<String> {
        if let Some(type_name) = &prop_def.type_name {
            // If it's already a fully qualified type, return it
            if type_name.contains("::") {
                return Some(type_name.clone());
            }

            // If it's a relative property type reference, construct the full name
            // by combining with the current resource type
            if self.is_property_type_property(prop_def) {
                // Extract the base resource type (AWS::Service::Resource part)
                let resource_parts: Vec<&str> = self.resource_type.split("::").collect();
                if resource_parts.len() >= 3 {
                    let base_type = format!(
                        "{}::{}::{}",
                        resource_parts[0], resource_parts[1], resource_parts[2]
                    );
                    return Some(format!("{}.{}", base_type, type_name));
                }
            }

            None
        } else {
            None
        }
    }

    /// Convert a property value to initial values for PropertyTypeFormWindow
    /// PropertyTypeFormWindow expects HashMap<String, String> for initial values
    fn get_property_type_initial_values(
        &self,
        value: &serde_json::Value,
    ) -> Option<HashMap<String, String>> {
        match value {
            serde_json::Value::Object(obj) => {
                // Already a JSON object, convert to HashMap<String, String>
                let mut initial_values = HashMap::new();
                for (k, v) in obj {
                    let string_value = match v {
                        serde_json::Value::String(s) => s.clone(),
                        _ => serde_json::to_string(v).unwrap_or_default(),
                    };
                    initial_values.insert(k.clone(), string_value);
                }
                Some(initial_values)
            }
            serde_json::Value::String(s)
                if !s.trim().is_empty() && s.trim().starts_with('{') && s.trim().ends_with('}') =>
            {
                // Try to parse string as JSON
                if let Ok(serde_json::Value::Object(obj)) =
                    serde_json::from_str::<serde_json::Value>(s)
                {
                    let mut initial_values = HashMap::new();
                    for (k, v) in obj {
                        let string_value = match v {
                            serde_json::Value::String(s) => s,
                            _ => serde_json::to_string(&v).unwrap_or_default(),
                        };
                        initial_values.insert(k, string_value);
                    }
                    Some(initial_values)
                } else {
                    None
                }
            }
            _ => {
                // For other types (empty, null, etc.), return None so PropertyTypeFormWindow starts fresh
                None
            }
        }
    }

    /// Show the resource form window with focus capability
    pub fn show_with_focus(&mut self, ctx: &egui::Context, bring_to_front: bool) -> bool {
        if !self.show {
            return false;
        }

        // Store the bring_to_front parameter to use in the main show logic
        self.show_with_focus_logic(ctx, bring_to_front)
    }

    /// Internal method that implements the show logic with focus support
    pub fn show_with_focus_logic(&mut self, ctx: &egui::Context, bring_to_front: bool) -> bool {
        let mut resource_saved = false;

        if !self.show {
            return resource_saved;
        }

        // Handle value editor window
        if self.value_editor.show(ctx) {
            // Check if a value was saved
            if let Some(new_value) = self.value_editor.take_saved_value() {
                if let Some(prop_name) = &self.editing_property {
                    // Parse the new value and update properties
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&new_value) {
                        self.properties.insert(prop_name.clone(), json_value);
                    } else {
                        self.properties
                            .insert(prop_name.clone(), serde_json::Value::String(new_value));
                    }
                    self.editing_property = None;
                }
            }
        }

        // Handle reference picker window
        if self.reference_picker.show(ctx) {
            // Check if a reference was selected
            if let Some(reference) = self.reference_picker.take_selected_reference() {
                if let Some(prop_name) = &self.referencing_property {
                    // Parse the reference as JSON and update properties
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&reference) {
                        self.properties.insert(prop_name.clone(), json_value);
                    }
                    self.referencing_property = None;
                }
            }
        }

        // First, process any pending sub-form creation
        self.process_pending_sub_forms();

        // Handle property type forms first
        let mut sub_forms_updated = false;
        let mut forms_to_remove = Vec::new();

        for (i, form) in self.property_type_forms.iter_mut().enumerate() {
            if form.show(ctx) {
                // Form was saved - need to manually update the property values
                // Get the property name from the path
                if let Some(prop_name) = form.property_path.split('.').next_back() {
                    // Extract just the JSON without calling the callback
                    let json_value = form.to_json_value();

                    // Update the property value with the JSON value directly
                    self.properties.insert(prop_name.to_string(), json_value);
                }

                // If form is not visible anymore, we'll remove it later
                if !form.show {
                    forms_to_remove.push(i);
                }

                sub_forms_updated = true;
            }

            // Check for any pending sub-forms that were created
            let pending = form.get_pending_sub_forms();
            if !pending.is_empty() {
                for (type_name, path) in pending {
                    // Nested property type forms start with no initial values
                    self.pending_sub_forms.push((type_name, path, None));
                }
                sub_forms_updated = true;
            }
        }

        // Remove any closed forms (in reverse order to maintain indices)
        for i in forms_to_remove.into_iter().rev() {
            if i < self.property_type_forms.len() {
                self.property_type_forms.remove(i);
            }
        }

        // If any sub-forms were updated or have pending forms, process them
        if sub_forms_updated {
            self.process_pending_sub_forms();
        }

        let title = if self.is_new {
            format!("New Resource: {}", self.resource_type)
        } else {
            format!("Edit Resource: {}", self.resource_id)
        };

        // We'll use a local flag to track if we should close the window
        let mut close_window = false;

        // Get screen dimensions to calculate appropriate window size
        let screen_rect = ctx.screen_rect();
        let max_width = screen_rect.width() * 0.85; // 85% of screen width
        let max_height = screen_rect.height() * 0.85; // 85% of screen height

        let mut window_open = self.show;
        let mut window = Window::new(title)
            .open(&mut window_open)
            .min_width(350.0)
            .min_height(400.0)
            .max_width(max_width)
            .max_height(max_height)
            .resizable(true)
            .default_pos(screen_rect.center()); // Center the window on the screen

        // Apply focus order if needed
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            // Start widget registration for this frame with UI context for clipping
            self.widget_manager.start_frame_with_ui_context(
                ui,
                self.window_id().to_string(),
                self.window_title()
            );

            // Clear stale actions (older than 5 seconds)
            self.widget_manager.clear_stale_actions(5000);

            // Top area with error message and resource ID
            if let Some(error) = &self.error_message {
                ui.colored_label(Color32::from_rgb(220, 50, 50), error);
                ui.separator();
            }

            // Resource ID field with auto-focus
            ui.horizontal(|ui| {
                ui.label("Resource ID:");
                let id_edit = ui.text_edit_singleline(&mut self.resource_id);

                // Register the resource ID text input with navigation system
                register_text_input!(self.widget_manager, id_edit, "resource_id_input", "Resource ID");

                // Note: Auto-focus removed to allow keyboard navigation to work properly
                // (Previous auto-focus on resource ID field interfered with hint mode)

                // Add documentation link button for this resource type
                let doc_button = ui.small_button("?")
                    .on_hover_text("Open AWS documentation for this resource type");

                // Register the documentation button
                register_button!(self.widget_manager, doc_button, "documentation_button", "Open AWS documentation");

                if doc_button.clicked() || self.widget_manager.should_element_be_clicked("documentation_button") {
                    // Consume the pending action if it was triggered by keyboard
                    if self.widget_manager.should_element_be_clicked("documentation_button") {
                        let _consumed_actions = self.widget_manager.consume_pending_actions("documentation_button");
                        tracing::info!("🎯 Keyboard navigation activated documentation button");
                    }
                    // Create a documentation URL for the resource type
                    let resource_type_parts: Vec<&str> = self.resource_type.split("::").collect();
                    if resource_type_parts.len() >= 3 {
                        let service = resource_type_parts[1].to_lowercase();
                        let resource = resource_type_parts[2].to_lowercase();

                        // AWS documentation URL format
                        let doc_url = format!(
                            "https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/aws-resource-{}-{}.html",
                            service,
                            resource
                        );

                        if let Err(e) = open::that(&doc_url) {
                            error!("Failed to open resource documentation URL: {}", e);
                            self.error_message = Some(format!("Failed to open documentation: {}", e));
                        }
                    }
                }
            });

            ui.separator();

            // Make the property contents scrollable
            ScrollArea::vertical()
                .max_height(max_height - 120.0) // Allow space for window title, ID field, and buttons
                .show(ui, |ui| {
                    // Simple property list rendering - no grids or complex layouts
                    if self.property_definitions.is_some() {
                        self.render_simple_property_list(ui);
                    } else {
                        ui.label("No property definitions available for this resource type.");
                    }

                    // Resource Attributes Section
                    ui.separator();
                    ui.collapsing("Resource Attributes", |ui| {
                        Grid::new("resource_attributes_grid")
                            .num_columns(2)
                            .spacing([10.0, 10.0])
                            .striped(true)
                            .show(ui, |ui| {
                                // DependsOn
                                ui.label("DependsOn:");
                                ui.vertical(|ui| {
                                    // TODO: Get list of other resources in template for dropdown
                                    let depends_on_str = self.depends_on.join(", ");
                                    let mut temp_depends_on = depends_on_str.clone();
                                    if ui.text_edit_singleline(&mut temp_depends_on).changed() {
                                        self.depends_on = temp_depends_on
                                            .split(',')
                                            .map(|s| s.trim().to_string())
                                            .filter(|s| !s.is_empty())
                                            .collect();
                                    }
                                    ui.small("Comma-separated list of resource logical IDs");
                                });
                                ui.end_row();

                                // Condition
                                ui.label("Condition:");
                                ui.vertical(|ui| {
                                    ui.text_edit_singleline(&mut self.condition);
                                    ui.small("Reference to a condition in the template");
                                });
                                ui.end_row();

                                // Metadata
                                ui.label("Metadata:");
                                ui.vertical(|ui| {
                                    ui.text_edit_multiline(&mut self.metadata);
                                    ui.small("JSON metadata for the resource");
                                });
                                ui.end_row();

                                // DeletionPolicy
                                ui.label("DeletionPolicy:");
                                ui.vertical(|ui| {
                                    let available_policies = ResourcePolicyManager::get_available_deletion_policies(&self.resource_type);
                                    egui::ComboBox::from_id_salt("deletion_policy")
                                        .selected_text(if self.deletion_policy.is_empty() { "Select..." } else { &self.deletion_policy })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut self.deletion_policy, String::new(), "None");
                                            for policy in available_policies {
                                                ui.selectable_value(&mut self.deletion_policy, policy.to_string(), policy);
                                            }
                                        });
                                    ui.small("Policy for resource deletion");
                                });
                                ui.end_row();

                                // UpdateReplacePolicy
                                ui.label("UpdateReplacePolicy:");
                                ui.vertical(|ui| {
                                    let available_policies = ResourcePolicyManager::get_update_replace_policy_options();
                                    egui::ComboBox::from_id_salt("update_replace_policy")
                                        .selected_text(if self.update_replace_policy.is_empty() { "Select..." } else { &self.update_replace_policy })
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut self.update_replace_policy, String::new(), "None");
                                            for policy in available_policies {
                                                ui.selectable_value(&mut self.update_replace_policy, policy.to_string(), policy);
                                            }
                                        });
                                    ui.small("Policy for resource replacement during updates");
                                });
                                ui.end_row();

                                // CreationPolicy (only show for supported resource types)
                                if ResourcePolicyManager::supports_creation_policy(&self.resource_type) {
                                    ui.label("CreationPolicy:");
                                    ui.vertical(|ui| {
                                        ui.text_edit_multiline(&mut self.creation_policy);
                                        ui.small("JSON configuration for resource creation signals");

                                        // Add helper buttons for common configurations
                                        ui.horizontal(|ui| {
                                            let creation_policy_types = ResourcePolicyManager::get_creation_policy_types(&self.resource_type);
                                            for policy_type in creation_policy_types {
                                                let desc = ResourcePolicyManager::get_creation_policy_description(&policy_type);
                                                if ui.small_button(format!("{:?}", policy_type)).on_hover_text(desc).clicked() {
                                                    let template = ResourcePolicyManager::get_creation_policy_template(&policy_type);
                                                    self.creation_policy = serde_json::to_string_pretty(&template).unwrap_or_default();
                                                }
                                            }
                                        });
                                    });
                                    ui.end_row();
                                }

                                // UpdatePolicy (only show for supported resource types)
                                if ResourcePolicyManager::supports_update_policy(&self.resource_type) {
                                    ui.label("UpdatePolicy:");
                                    ui.vertical(|ui| {
                                        ui.text_edit_multiline(&mut self.update_policy);
                                        ui.small("JSON configuration for resource updates");

                                        // Add helper buttons for common configurations
                                        ui.horizontal(|ui| {
                                            let update_policy_types = ResourcePolicyManager::get_update_policy_types(&self.resource_type);
                                            for policy_type in update_policy_types {
                                                let desc = ResourcePolicyManager::get_update_policy_description(&policy_type);
                                                if ui.small_button(format!("{:?}", policy_type)).on_hover_text(desc).clicked() {
                                                    let template = ResourcePolicyManager::get_update_policy_template(&policy_type);
                                                    self.update_policy = serde_json::to_string_pretty(&template).unwrap_or_default();
                                                }
                                            }
                                        });
                                    });
                                    ui.end_row();
                                }
                            });
                    });
                });

            ui.separator();

            // Buttons - outside the scroll area
            ui.horizontal(|ui| {
                let cancel_button = ui.button("Cancel");

                // Register the cancel button
                register_button!(self.widget_manager, cancel_button, "cancel_button", "Cancel");

                let should_cancel = cancel_button.clicked() || self.widget_manager.should_element_be_clicked("cancel_button");

                if should_cancel {
                    // Consume the pending action if it was triggered by keyboard
                    if self.widget_manager.should_element_be_clicked("cancel_button") {
                        let _consumed_actions = self.widget_manager.consume_pending_actions("cancel_button");
                        tracing::info!("🎯 Keyboard navigation activated cancel button");
                    }
                    close_window = true;
                }

                let save_button =
                    ui.add_enabled(!self.resource_id.is_empty(), Button::new("Save"));

                // Register the save button
                register_button!(self.widget_manager, save_button, "save_button", "Save");

                let should_save = save_button.clicked() || self.widget_manager.should_element_be_clicked("save_button");

                if should_save {
                    // Consume the pending action if it was triggered by keyboard
                    if self.widget_manager.should_element_be_clicked("save_button") {
                        let _consumed_actions = self.widget_manager.consume_pending_actions("save_button");
                        tracing::info!("🎯 Keyboard navigation activated save button");
                    }
                    // Validate required properties
                    let mut missing_required = Vec::new();

                    if let Some(prop_defs) = self.property_definitions.clone() {
                        for (prop_name, prop_def) in prop_defs {
                            if prop_def.required {
                                if let Some(value) = self.properties.get(&prop_name) {
                                    match value {
                                        serde_json::Value::String(s) if s.trim().is_empty() => {
                                            missing_required.push(prop_name.clone());
                                        }
                                        serde_json::Value::Null => {
                                            missing_required.push(prop_name.clone());
                                        }
                                        serde_json::Value::Array(arr) if arr.is_empty() => {
                                            missing_required.push(prop_name.clone());
                                        }
                                        serde_json::Value::Object(obj) if obj.is_empty() => {
                                            missing_required.push(prop_name.clone());
                                        }
                                        _ => {} // Value is present and non-empty
                                    }
                                } else {
                                    missing_required.push(prop_name.clone());
                                }
                            }
                        }
                    }

                    if !missing_required.is_empty() {
                        self.error_message = Some(format!(
                            "Missing required properties: {}",
                            missing_required.join(", ")
                        ));
                    } else {
                        // Create the resource
                        let mut resource = CloudFormationResource::new(
                            self.resource_id.clone(),
                            self.resource_type.clone(),
                        );
                        resource.properties = self.properties.clone();
                        resource.depends_on = if self.depends_on.is_empty() { None } else { Some(self.depends_on.clone()) };
                        resource.condition = if self.condition.is_empty() { None } else { Some(self.condition.clone()) };
                        resource.metadata = if self.metadata.is_empty() { None } else { Some(self.metadata.clone()) };
                        resource.deletion_policy = if self.deletion_policy.is_empty() { None } else { Some(self.deletion_policy.clone()) };
                        resource.update_replace_policy = if self.update_replace_policy.is_empty() { None } else { Some(self.update_replace_policy.clone()) };
                        resource.creation_policy = if self.creation_policy.is_empty() { None } else { Some(self.creation_policy.clone()) };
                        resource.update_policy = if self.update_policy.is_empty() { None } else { Some(self.update_policy.clone()) };

                        // Call the save callback
                        if let Some(on_save) = &mut self.on_save {
                            on_save(resource);
                        }

                        resource_saved = true;
                        close_window = true;
                    }
                }
            });

            // Complete frame processing and update widget states
            self.widget_manager.complete_frame(ui.ctx());
        });

        // Update the window state
        self.show = window_open;

        // Close the window if requested
        if close_window {
            self.show = false;
            // Also close any property type forms
            for form in &mut self.property_type_forms {
                form.show = false;
            }

            // Reset the focus flag when window is closed
            unsafe {
                FOCUS_SET = false;
            }
        }

        resource_saved
    }
}

impl FocusableWindow for ResourceFormWindow {
    type ShowParams = FormShowParams;

    fn window_id(&self) -> &'static str {
        "resource_form"
    }

    fn window_title(&self) -> String {
        if self.resource_id.is_empty() {
            format!("New {} Resource", self.resource_type)
        } else {
            format!("Edit Resource: {}", self.resource_id)
        }
    }

    fn is_open(&self) -> bool {
        self.show
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        _params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        // Note: We ignore the return value here as the trait doesn't require it
        // The actual resource saved logic is handled in the app.rs handler
        let _ = self.show_with_focus_logic(ctx, bring_to_front);
    }
}

impl ResourceFormWindow {
    /// Collect navigable elements from this resource form window
    /// This method returns real registered widgets from the NavigableWidgetManager
    pub fn collect_navigable_elements(&self) -> Vec<NavigableElement> {
        if !self.show {
            return Vec::new(); // Window is not visible, no elements to collect
        }

        // Get real elements from the widget manager
        let mut elements = self.widget_manager.collector().get_elements().to_vec();

        // Also collect elements from any open PropertyTypeFormWindow instances
        for form in &self.property_type_forms {
            if form.is_open() {
                let form_elements = form.collect_navigable_elements();
                tracing::info!(
                    "🎯 ResourceFormWindow - PropertyTypeFormWindow '{}' contributed {} elements",
                    form.window_title(),
                    form_elements.len()
                );
                elements.extend(form_elements);
            }
        }

        // Log widget collection results for R4.1 validation
        tracing::info!("🎯 R4.1 ResourceFormWindow::collect_navigable_elements - Collected {} REAL elements for window '{}' ({}) including PropertyTypeFormWindow elements",
                       elements.len(), self.window_title(), self.window_id());

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
                "🎯 R4.1 ResourceFormWindow - Real widget types breakdown: {:?}",
                type_counts
            );

            // Log a few example elements for verification
            for (i, element) in elements.iter().take(3).enumerate() {
                tracing::info!("🎯 R4.1 ResourceFormWindow - Element {}: id='{}' type={:?} rect={:?} enabled={}",
                               i + 1, element.id, element.element_type, element.rect, element.enabled);
            }
        } else {
            tracing::warn!("❌ R4.1 ResourceFormWindow - No real elements captured - widget registration may not be working");
        }

        elements
    }
}

impl NavigableWindow for ResourceFormWindow {
    fn get_navigation_context(&self) -> NavigationContext {
        let mut settings = HashMap::new();
        settings.insert("window_id".to_string(), self.window_id().to_string());
        settings.insert("window_title".to_string(), self.window_title());
        settings.insert("help_text".to_string(), "Resource Form Navigation:\n- Tab/Shift+Tab: Navigate form fields\n- Enter: Activate buttons\n- Ctrl+S: Save resource".to_string());

        NavigationContext {
            supports_hints: true,
            supports_visual_mode: true,
            handles_scrolling: true,
            settings,
        }
    }

    fn get_custom_key_bindings(&self) -> HashMap<String, NavigationCommand> {
        let mut bindings = HashMap::new();

        // Add resource form specific key bindings
        bindings.insert("ctrl+s".to_string(), NavigationCommand::ActivateElement); // Save shortcut
        bindings.insert("ctrl+n".to_string(), NavigationCommand::ActivateElement); // New resource shortcut
        bindings.insert("escape".to_string(), NavigationCommand::CloseWindow); // Close form

        bindings
    }

    fn handle_navigation_command(&mut self, command: NavigationCommand) -> KeyEventResult {
        match command {
            NavigationCommand::ActivateElement => {
                // For now, just log the action - in a full implementation this would save the resource
                tracing::info!("Resource form: Save command activated");
                KeyEventResult::Handled
            }
            NavigationCommand::CloseWindow => {
                // Close the resource form window
                self.show = false;
                KeyEventResult::Handled
            }
            _ => {
                // Let the global navigation handle other commands
                KeyEventResult::PassThrough
            }
        }
    }

    fn on_navigation_mode_changed(&mut self, _old_mode: NavigationMode, new_mode: NavigationMode) {
        // Log mode changes for debugging
        tracing::debug!("Resource form navigation mode changed to: {:?}", new_mode);
    }
}
