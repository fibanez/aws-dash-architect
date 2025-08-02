use crate::app::cfn_resource_policies::ResourcePolicyManager;
use crate::app::cfn_resources::{
    is_resource_type, AttributeDefinitionMap, PropertyDefinition, PropertyDefinitionMap,
};
use crate::app::dashui::window_focus::{FocusableWindow, SimpleShowParams};
use crate::app::projects::CloudFormationResource;
use eframe::egui;
use open;
use tracing::debug;

#[derive(Default)]
pub struct ResourceDetailsWindow {
    pub show: bool,
    pub selected_resource_type: String,
    pub selected_resource_properties: Option<PropertyDefinitionMap>,
    pub selected_resource_attributes: Option<AttributeDefinitionMap>,
    pub selected_resource_doc_url: Option<String>,
    pub resource_instance: Option<CloudFormationResource>,
}

impl ResourceDetailsWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show details for a resource instance (with actual property values and attributes)
    pub fn show_resource_instance(&mut self, resource: CloudFormationResource) {
        self.resource_instance = Some(resource.clone());
        self.selected_resource_type = resource.resource_type;
        self.show = true;
        // Clear the specification data since we're showing an instance
        self.selected_resource_properties = None;
        self.selected_resource_attributes = None;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<String> {
        self.show_with_offset(ctx, egui::Vec2::ZERO)
    }

    pub fn show_with_focus(&mut self, ctx: &egui::Context, bring_to_front: bool) -> Option<String> {
        self.show_with_offset_and_focus(ctx, egui::Vec2::ZERO, bring_to_front)
    }

    pub fn show_with_offset(&mut self, ctx: &egui::Context, offset: egui::Vec2) -> Option<String> {
        self.show_with_offset_and_focus(ctx, offset, false)
    }

    pub fn show_with_offset_and_focus(
        &mut self,
        ctx: &egui::Context,
        offset: egui::Vec2,
        bring_to_front: bool,
    ) -> Option<String> {
        if !self.show {
            return None;
        }

        let mut property_to_open = None;

        // Get screen size to set appropriate window size
        let screen_size = ctx.screen_rect().size();
        let max_height = screen_size.y * 0.8; // 80% of screen height

        // Determine if this is a resource type or property type for the window title
        let type_prefix = match is_resource_type(&self.selected_resource_type, "us-east-1") {
            Ok(true) => "Resource",
            Ok(false) => "Property Type",
            Err(_) => {
                // If we can't determine from is_resource_type, check if it contains "::" and has a parent resource
                if self.selected_resource_type.contains("::") {
                    "Property Type"
                } else {
                    "Type" // Generic fallback
                }
            }
        };

        let mut window =
            egui::Window::new(format!("{}: {}", type_prefix, self.selected_resource_type))
                .resizable(true)
                .default_width(600.0)
                .default_height(max_height.min(500.0))
                .max_height(max_height)
                .anchor(egui::Align2::CENTER_CENTER, offset);

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            // Add buttons in a horizontal layout
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    self.show = false;
                }

                // Add Docs button that opens documentation URL
                if let Some(doc_url) = &self.selected_resource_doc_url {
                    if !doc_url.is_empty() && ui.button("Docs").clicked() {
                        if let Err(e) = open::that(doc_url) {
                            eprintln!("Failed to open documentation URL: {}", e);
                        }
                    }
                }
            });

            ui.separator();

            // Wrap everything in a scrollable area
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Display properties - expanded by default
                egui::CollapsingHeader::new("Properties")
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some(props) = &self.selected_resource_properties {
                            if props.is_empty() {
                                ui.label("No properties defined for this resource type");
                            } else {
                                // Create a table for properties
                                egui::Grid::new("properties_grid")
                                    .num_columns(4)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        // Header row
                                        ui.strong("Property");
                                        ui.strong("Required");
                                        ui.strong("Type");
                                        ui.strong("Update Type");
                                        ui.end_row();

                                        // Sort properties by required status (required first) then by name
                                        let mut props_vec: Vec<(&String, &PropertyDefinition)> =
                                            props.iter().collect();
                                        props_vec.sort_by(|a, b| {
                                            match (a.1.required, b.1.required) {
                                                (true, false) => std::cmp::Ordering::Less,
                                                (false, true) => std::cmp::Ordering::Greater,
                                                _ => a.0.cmp(b.0),
                                            }
                                        });

                                        for (name, prop) in props_vec {
                                            ui.label(name);
                                            ui.label(if prop.required { "Yes" } else { "No" });

                                            // Format the type and make it clickable if it's a property type
                                            let (type_str, is_property_type, property_type_name) =
                                                match (
                                                    &prop.primitive_type,
                                                    &prop.type_name,
                                                    &prop.item_type,
                                                ) {
                                                    (Some(primitive), _, _) => {
                                                        (primitive.clone(), false, String::new())
                                                    }
                                                    (_, Some(t), Some(item)) if t == "List" => {
                                                        // Check if item is a property type
                                                        let is_pt = ![
                                                            "String",
                                                            "Integer",
                                                            "Boolean",
                                                            "Double",
                                                            "Long",
                                                            "Timestamp",
                                                            "Json",
                                                        ]
                                                        .contains(&item.as_str());
                                                        (
                                                            format!("{}<{}>", t, item),
                                                            is_pt,
                                                            item.clone(),
                                                        )
                                                    }
                                                    (_, Some(t), Some(item)) if t == "Map" => {
                                                        // Check if item is a property type
                                                        let is_pt = ![
                                                            "String",
                                                            "Integer",
                                                            "Boolean",
                                                            "Double",
                                                            "Long",
                                                            "Timestamp",
                                                            "Json",
                                                        ]
                                                        .contains(&item.as_str());
                                                        (
                                                            format!("{}<{}>", t, item),
                                                            is_pt,
                                                            item.clone(),
                                                        )
                                                    }
                                                    (_, Some(t), _) => {
                                                        // Check if type is a property type
                                                        let is_pt = ![
                                                            "String",
                                                            "Integer",
                                                            "Boolean",
                                                            "Double",
                                                            "Long",
                                                            "Timestamp",
                                                            "Json",
                                                            "List",
                                                            "Map",
                                                        ]
                                                        .contains(&t.as_str());
                                                        (t.clone(), is_pt, t.clone())
                                                    }
                                                    _ => (
                                                        "Unknown".to_string(),
                                                        false,
                                                        String::new(),
                                                    ),
                                                };

                                            // If it's a property type, make it clickable
                                            if is_property_type {
                                                // Create the full property type name
                                                let full_property_type = {
                                                    // Extract just the last property name segment
                                                    let property_name = match property_type_name
                                                        .rfind('.')
                                                    {
                                                        Some(pos) => &property_type_name[pos + 1..],
                                                        None => &property_type_name,
                                                    };

                                                    // Extract the resource part (AWS::Service::Resource)
                                                    let resource_part = match self
                                                        .selected_resource_type
                                                        .rfind('.')
                                                    {
                                                        Some(pos) => {
                                                            &self.selected_resource_type[0..pos]
                                                        }
                                                        None => &self.selected_resource_type,
                                                    };

                                                    format!("{}.{}", resource_part, property_name)
                                                };

                                                // Store the property type to open if clicked
                                                let property_to_open_clone =
                                                    full_property_type.clone();

                                                // Use a small button instead of a hyperlink
                                                if ui.small_button(type_str).clicked() {
                                                    let property_type_str =
                                                        property_to_open_clone.clone();
                                                    property_to_open = Some(property_to_open_clone);
                                                    debug!(
                                                        "Property type to open: {}",
                                                        property_type_str
                                                    );
                                                }
                                            } else {
                                                // Regular non-clickable label for primitive types
                                                ui.label(type_str);
                                            }

                                            ui.label(&prop.update_type);
                                            ui.end_row();
                                        }
                                    });
                            }
                        } else {
                            ui.label("Failed to load properties for this resource type");
                        }
                    });

                ui.separator();

                // Determine if this is a resource type or property type
                let is_resource =
                    is_resource_type(&self.selected_resource_type, "us-east-1").unwrap_or(true);

                // Display attributes section only if it's a resource type
                if is_resource {
                    // Display attributes - using collapsing header
                    egui::CollapsingHeader::new("Attributes")
                        .default_open(false)
                        .show(ui, |ui| {
                            if let Some(attrs) = &self.selected_resource_attributes {
                                if attrs.is_empty() {
                                    ui.label("No attributes defined for this resource type");
                                } else {
                                    // Create a table for attributes
                                    egui::Grid::new("attributes_grid")
                                        .num_columns(2)
                                        .striped(true)
                                        .show(ui, |ui| {
                                            // Header row
                                            ui.strong("Attribute");
                                            ui.strong("Type");
                                            ui.end_row();

                                            // Sort attributes by name
                                            let mut attrs_vec: Vec<(
                                                &String,
                                                &crate::app::cfn_resources::AttributeDefinition,
                                            )> = attrs.iter().collect();
                                            attrs_vec.sort_by(|a, b| a.0.cmp(b.0));

                                            for (name, attr) in attrs_vec {
                                                ui.label(name);

                                                // Format the type
                                                let type_str =
                                                    match (&attr.primitive_type, &attr.type_name) {
                                                        (Some(primitive), _) => primitive.clone(),
                                                        (_, Some(t)) => t.clone(),
                                                        _ => "Unknown".to_string(),
                                                    };
                                                ui.label(type_str);
                                                ui.end_row();
                                            }
                                        });
                                }
                            } else {
                                ui.label("Failed to load attributes for this resource type");
                            }
                        });
                } else {
                    // For property types, just show a message that they don't have attributes
                    ui.separator();
                    ui.label("Property types do not have attributes");
                }

                // Display resource instance details if available
                if let Some(resource) = &self.resource_instance {
                    ui.separator();

                    // Resource Instance Properties Section
                    egui::CollapsingHeader::new("Instance Properties")
                        .default_open(true)
                        .show(ui, |ui| {
                            if resource.properties.is_empty() {
                                ui.label("No properties configured for this resource instance");
                            } else {
                                egui::Grid::new("instance_properties_grid")
                                    .num_columns(2)
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.strong("Property");
                                        ui.strong("Value");
                                        ui.end_row();

                                        // Sort properties by name
                                        let mut props_vec: Vec<(&String, &serde_json::Value)> =
                                            resource.properties.iter().collect();
                                        props_vec.sort_by(|a, b| a.0.cmp(b.0));

                                        for (name, value) in props_vec {
                                            ui.label(name);

                                            // Convert JSON value to string for display
                                            let value_str = match value {
                                                serde_json::Value::String(s) => s.clone(),
                                                _ => serde_json::to_string_pretty(value)
                                                    .unwrap_or_else(|_| value.to_string()),
                                            };

                                            // Handle long values by using text_edit for better display
                                            if value_str.len() > 50 {
                                                ui.add(
                                                    egui::TextEdit::multiline(
                                                        &mut value_str.clone(),
                                                    )
                                                    .desired_width(300.0)
                                                    .desired_rows(3)
                                                    .interactive(false),
                                                );
                                            } else {
                                                ui.label(value_str);
                                            }
                                            ui.end_row();
                                        }
                                    });
                            }
                        });

                    // Resource Instance Attributes Section
                    egui::CollapsingHeader::new("Instance Attributes")
                        .default_open(true)
                        .show(ui, |ui| {
                            let mut has_attributes = false;

                            egui::Grid::new("instance_attributes_grid")
                                .num_columns(2)
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.strong("Attribute");
                                    ui.strong("Value");
                                    ui.end_row();

                                    // DependsOn
                                    if let Some(depends_on) = &resource.depends_on {
                                        if !depends_on.is_empty() {
                                            ui.label("DependsOn");
                                            ui.label(depends_on.join(", "));
                                            ui.end_row();
                                            has_attributes = true;
                                        }
                                    }

                                    // Condition
                                    if let Some(condition) = &resource.condition {
                                        if !condition.is_empty() {
                                            ui.label("Condition");
                                            ui.label(condition);
                                            ui.end_row();
                                            has_attributes = true;
                                        }
                                    }

                                    // Metadata
                                    if let Some(metadata) = &resource.metadata {
                                        if !metadata.is_empty() {
                                            ui.label("Metadata");
                                            ui.add(
                                                egui::TextEdit::multiline(&mut metadata.clone())
                                                    .desired_width(300.0)
                                                    .desired_rows(3)
                                                    .interactive(false),
                                            );
                                            ui.end_row();
                                            has_attributes = true;
                                        }
                                    }

                                    // DeletionPolicy
                                    if let Some(deletion_policy) = &resource.deletion_policy {
                                        if !deletion_policy.is_empty() {
                                            ui.label("DeletionPolicy");
                                            ui.label(deletion_policy);
                                            ui.end_row();
                                            has_attributes = true;
                                        }
                                    }

                                    // UpdateReplacePolicy
                                    if let Some(update_replace_policy) =
                                        &resource.update_replace_policy
                                    {
                                        if !update_replace_policy.is_empty() {
                                            ui.label("UpdateReplacePolicy");
                                            ui.label(update_replace_policy);
                                            ui.end_row();
                                            has_attributes = true;
                                        }
                                    }

                                    // CreationPolicy (only if supported)
                                    if ResourcePolicyManager::supports_creation_policy(
                                        &resource.resource_type,
                                    ) {
                                        if let Some(creation_policy) = &resource.creation_policy {
                                            if !creation_policy.is_empty() {
                                                ui.label("CreationPolicy");
                                                ui.add(
                                                    egui::TextEdit::multiline(
                                                        &mut creation_policy.clone(),
                                                    )
                                                    .desired_width(300.0)
                                                    .desired_rows(3)
                                                    .interactive(false),
                                                );
                                                ui.end_row();
                                                has_attributes = true;
                                            }
                                        }
                                    }

                                    // UpdatePolicy (only if supported)
                                    if ResourcePolicyManager::supports_update_policy(
                                        &resource.resource_type,
                                    ) {
                                        if let Some(update_policy) = &resource.update_policy {
                                            if !update_policy.is_empty() {
                                                ui.label("UpdatePolicy");
                                                ui.add(
                                                    egui::TextEdit::multiline(
                                                        &mut update_policy.clone(),
                                                    )
                                                    .desired_width(300.0)
                                                    .desired_rows(3)
                                                    .interactive(false),
                                                );
                                                ui.end_row();
                                                has_attributes = true;
                                            }
                                        }
                                    }
                                });

                            if !has_attributes {
                                ui.label("No attributes configured for this resource instance");
                            }
                        });
                }
            });
        });

        property_to_open
    }
}

impl FocusableWindow for ResourceDetailsWindow {
    type ShowParams = SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "resource_details"
    }

    fn window_title(&self) -> String {
        if self.selected_resource_type.is_empty() {
            "Resource Details".to_string()
        } else {
            format!("Resource Details: {}", self.selected_resource_type)
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
        self.show_with_focus(ctx, bring_to_front);
    }
}
