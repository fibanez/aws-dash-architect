use crate::app::cfn_resources::{PropertyDefinition, PropertyDefinitionMap};
use eframe::egui;
use tracing::debug;

/// Manager for property type windows
#[derive(Default)]
pub struct PropertyTypeWindowManager {
    pub windows: Vec<PropertyTypeWindow>,
}

impl PropertyTypeWindowManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_window(
        &mut self,
        property_type: String,
        properties: Option<PropertyDefinitionMap>,
    ) {
        // Check if a window for this property type already exists
        if let Some(pos) = self
            .windows
            .iter()
            .position(|w| w.property_type == property_type)
        {
            // Window already exists, just make sure it's shown
            self.windows[pos].show = true;
            return;
        }

        // Create a new property type window
        let new_window = PropertyTypeWindow {
            property_type,
            show: true,
            properties,
        };

        // Add the new window to our collection
        self.windows.push(new_window);
    }

    pub fn show_windows(&mut self, ctx: &egui::Context) -> (Vec<String>, Option<usize>) {
        self.show_windows_with_offsets(ctx, &std::collections::HashMap::new())
    }

    pub fn show_windows_with_offsets(
        &mut self,
        ctx: &egui::Context,
        offsets: &std::collections::HashMap<String, egui::Vec2>,
    ) -> (Vec<String>, Option<usize>) {
        // Remove any windows that are marked as not shown
        self.windows.retain(|window| window.show);

        // Track property types that are clicked and should be opened
        let mut property_types_to_open = Vec::new();

        // Track which window was interacted with (for focus)
        let mut focused_window_idx = None;

        // Process and display each property type window
        for (idx, window) in self.windows.iter_mut().enumerate() {
            // Check if the window is active
            if window.show {
                // Track if this window was interacted with
                let was_interacted = ctx.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer
                            .interact_pos()
                            .is_some_and(|pos| ctx.available_rect().contains(pos))
                });

                if was_interacted {
                    focused_window_idx = Some(idx);
                }

                // Get offset for this window
                let offset = offsets
                    .get(&format!("property_type_{}", idx))
                    .copied()
                    .unwrap_or(egui::Vec2::ZERO);

                // Show the window with offset
                if let Some(property_type) = window.show_with_offset(ctx, offset) {
                    property_types_to_open.push(property_type);
                }
            }
        }

        (property_types_to_open, focused_window_idx)
    }
}

/// Struct to represent a property type window
pub struct PropertyTypeWindow {
    /// The property type name
    pub property_type: String,
    /// Whether this window should be displayed
    pub show: bool,
    /// Properties of the property type
    pub properties: Option<PropertyDefinitionMap>,
}

impl PropertyTypeWindow {
    pub fn show(&mut self, ctx: &egui::Context) -> Option<String> {
        self.show_with_offset(ctx, egui::Vec2::ZERO)
    }

    pub fn show_with_offset(&mut self, ctx: &egui::Context, offset: egui::Vec2) -> Option<String> {
        if !self.show {
            return None;
        }

        let mut property_to_open = None;

        // Get screen size to set appropriate window size
        let screen_size = ctx.screen_rect().size();
        let max_height = screen_size.y * 0.8; // 80% of screen height

        egui::Window::new(format!("Property Type: {}", self.property_type))
            .resizable(true)
            .default_width(600.0)
            .default_height(max_height.min(500.0))
            .max_height(max_height)
            .anchor(egui::Align2::CENTER_CENTER, offset)
            .show(ctx, |ui| {
                // Add a close button
                if ui.button("Close").clicked() {
                    self.show = false;
                }

                ui.separator();

                // Wrap everything in a scrollable area
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Display properties - expanded by default
                    egui::CollapsingHeader::new("Properties")
                        .default_open(true)
                        .show(ui, |ui| {
                            if let Some(props) = &self.properties {
                                if props.is_empty() {
                                    ui.label("No properties defined for this property type");
                                } else {
                                    // Create a table for properties
                                    egui::Grid::new(format!(
                                        "properties_grid_{}",
                                        self.property_type
                                    ))
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
                                                        .property_type
                                                        .rfind('.')
                                                    {
                                                        Some(pos) => &self.property_type[0..pos],
                                                        None => &self.property_type,
                                                    };

                                                    format!("{}.{}", resource_part, property_name)
                                                };

                                                // Use a small button instead of a hyperlink
                                                if ui.small_button(type_str).clicked() {
                                                    debug!(
                                                        "Property type to open: {}",
                                                        full_property_type
                                                    );
                                                    property_to_open = Some(full_property_type);
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
                                ui.label("Failed to load properties for this property type");
                            }
                        });

                    // Property types don't have attributes, so just show a message
                    ui.separator();
                    ui.label("Property types do not have attributes");
                });
            });

        property_to_open
    }
}
