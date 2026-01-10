use crate::app::resource_explorer::PropertyCatalog;
use egui::Ui;
use egui_dnd::dnd;

/// Widget for building hierarchical property grouping configurations with drag-and-drop UI
pub struct PropertyHierarchyBuilderWidget {
    /// Selected hierarchy (order matters - top to bottom = first level to last level)
    selected_hierarchy: Vec<String>,
    /// Property catalog for available properties
    property_catalog: PropertyCatalog,
    /// Search filter for left panel
    search_filter: String,
}

impl PropertyHierarchyBuilderWidget {
    /// Create a new property hierarchy builder widget
    pub fn new(property_catalog: PropertyCatalog, initial_hierarchy: Vec<String>) -> Self {
        Self {
            selected_hierarchy: initial_hierarchy,
            property_catalog,
            search_filter: String::new(),
        }
    }

    /// Render the hierarchy builder UI
    /// Returns: (hierarchy, apply_clicked, cancel_clicked)
    pub fn show(&mut self, ui: &mut Ui) -> (Vec<String>, bool, bool) {
        let mut apply_clicked = false;
        let mut cancel_clicked = false;

        ui.vertical(|ui| {
            ui.heading("Configure Property Hierarchy");
            ui.add_space(8.0);
            ui.label("Drag property paths to build a multi-level grouping hierarchy");
            ui.separator();

            // Two-panel layout using horizontal with explicit sizing
            ui.horizontal(|ui| {
                // Left panel: Available properties (takes 50% of width)
                let available_width = ui.available_width();
                let panel_width = (available_width / 2.0) - 8.0; // Subtract spacing

                ui.allocate_ui_with_layout(
                    egui::vec2(panel_width, 400.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.render_left_panel(ui);
                    },
                );

                ui.add_space(8.0);

                // Right panel: Selected hierarchy (takes remaining 50% of width)
                ui.allocate_ui_with_layout(
                    egui::vec2(panel_width, 400.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.render_right_panel(ui);
                    },
                );
            });

            ui.separator();

            // Preview section
            self.render_preview(ui);

            ui.separator();

            // Action buttons
            let (apply, cancel) = self.render_buttons(ui);
            apply_clicked = apply;
            cancel_clicked = cancel;
        });

        (
            self.selected_hierarchy.clone(),
            apply_clicked,
            cancel_clicked,
        )
    }

    /// Render the left panel with available properties
    fn render_left_panel(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Available Properties");
            ui.separator();

            // Search box
            ui.horizontal(|ui| {
                ui.label("Search:");
                let search_response = ui.text_edit_singleline(&mut self.search_filter);
                if search_response.changed() {
                    tracing::debug!(
                        "Property search filter changed to: '{}'",
                        self.search_filter
                    );
                }
            });
            ui.add_space(4.0);

            // Scrollable list of available properties
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .auto_shrink([false, false]) // Ensure scroll area always shows
                .show(ui, |ui| {
                    let property_keys: Vec<_> = self
                        .property_catalog
                        .get_keys_sorted()
                        .into_iter()
                        .filter(|key| {
                            // Filter out unwanted root-level internal properties only
                            // Allow nested properties like "detailed_properties.CidrBlock"
                            key.path != "account_color"
                                && key.path != "account_id"
                                && key.path != "detailed_properties"
                                && key.path != "detailed_timestamp"
                                && key.path != "display_name"
                                && key.path != "properties"
                                && key.path != "properties"
                        })
                        .take(100)
                        .collect();

                    for prop_key in property_keys {
                        // Apply search filter
                        if !self.search_filter.is_empty()
                            && !prop_key
                                .path
                                .to_lowercase()
                                .contains(&self.search_filter.to_lowercase())
                        {
                            continue;
                        }

                        // Check if already in hierarchy
                        let is_selected = self.selected_hierarchy.contains(&prop_key.path);

                        // Show full property path
                        let display_path = &prop_key.path;

                        // Format label
                        let type_name = prop_key.value_type.display_name();
                        let label = format!(
                            "{} ({}, {} resources)",
                            display_path, type_name, prop_key.frequency
                        );

                        // Show as drag source
                        let response = ui.dnd_drag_source(
                            egui::Id::new("available_property").with(&prop_key.path),
                            prop_key.path.clone(),
                            |ui| {
                                ui.horizontal(|ui| {
                                    if is_selected {
                                        ui.add_enabled(
                                            false,
                                            egui::Label::new(egui::RichText::new(&label).weak()),
                                        );
                                    } else {
                                        ui.label(&label);
                                    }
                                });
                            },
                        );

                        // Double-click to add
                        if response.response.double_clicked()
                            && !is_selected
                            && self.selected_hierarchy.len() < 5
                        {
                            self.selected_hierarchy.push(prop_key.path.clone());
                            tracing::info!("Added property to hierarchy: {}", prop_key.path);
                        }
                    }
                });
        });
    }

    /// Render the right panel with selected hierarchy
    fn render_right_panel(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Selected Hierarchy");
            ui.separator();

            if self.selected_hierarchy.is_empty() {
                ui.label("No properties selected");
                ui.label("Drag properties from the left or double-click to add");
            } else {
                ui.label(format!(
                    "{} level(s) selected (max 5)",
                    self.selected_hierarchy.len()
                ));
            }

            ui.add_space(4.0);

            // Scrollable drag-drop list
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    let mut items_to_remove = Vec::new();

                    dnd(ui, "property_hierarchy_dnd")
                        .with_animation_time(0.2)
                        .show_vec(
                            &mut self.selected_hierarchy,
                            |ui, property_path, handle, state| {
                                handle.ui(ui, |ui| {
                                    ui.label(":::");
                                });

                                ui.horizontal(|ui| {
                                    ui.label(format!("{}.", state.index + 1));
                                    ui.label(property_path.as_str());

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.small_button("X").clicked() {
                                                items_to_remove.push(state.index);
                                            }
                                        },
                                    );
                                });
                            },
                        );

                    // Remove marked items (in reverse to maintain indices)
                    for index in items_to_remove.iter().rev() {
                        let removed = self.selected_hierarchy.remove(*index);
                        tracing::info!("Removed property from hierarchy: {}", removed);
                    }

                    // Handle drop from left panel
                    let (_, payload) = ui.dnd_drop_zone::<String, ()>(
                        egui::Frame::default()
                            .inner_margin(egui::Margin::same(10))
                            .stroke(egui::Stroke::new(1.0, ui.visuals().weak_text_color()))
                            .fill(ui.visuals().faint_bg_color),
                        |ui| {
                            ui.label(egui::RichText::new("Drop here to add").weak().size(14.0));
                        },
                    );

                    if let Some(property_path) = payload {
                        if !self.selected_hierarchy.contains(&property_path)
                            && self.selected_hierarchy.len() < 5
                        {
                            self.selected_hierarchy.push(property_path.as_ref().clone());
                            tracing::info!("Dropped property into hierarchy: {}", property_path);
                        }
                    }
                });

            ui.add_space(4.0);

            // Clear all button
            if !self.selected_hierarchy.is_empty() && ui.button("Clear All").clicked() {
                self.selected_hierarchy.clear();
                tracing::info!("Cleared all properties from hierarchy");
            }
        });
    }

    /// Render preview of the hierarchy
    fn render_preview(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Preview");

            if self.selected_hierarchy.is_empty() {
                ui.label("No hierarchy configured");
            } else {
                ui.label("Grouping structure (top to bottom):");
                ui.add_space(4.0);

                for (idx, property_path) in self.selected_hierarchy.iter().enumerate() {
                    let indent = idx as f32 * 20.0;
                    // Show full property path
                    let display_name = property_path;
                    ui.horizontal(|ui| {
                        ui.add_space(indent);
                        ui.label(format!("Level {}: {}", idx + 1, display_name));
                    });
                }

                ui.add_space(8.0);
                ui.label("Example:");
                ui.label("  instance.state.name > placement.availability_zone");
                ui.label("    - running > us-east-1a");
                ui.label("    - running > us-east-1b");
                ui.label("    - stopped > us-east-1a");
            }
        });
    }

    /// Render action buttons
    /// Returns: (apply_clicked, cancel_clicked)
    fn render_buttons(&self, ui: &mut Ui) -> (bool, bool) {
        let mut apply_clicked = false;
        let mut cancel_clicked = false;

        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                cancel_clicked = true;
            }

            if self.selected_hierarchy.is_empty() {
                ui.add_enabled(false, egui::Button::new("Apply"));
            } else if ui.button("Apply").clicked() {
                apply_clicked = true;
            }
        });

        (apply_clicked, cancel_clicked)
    }

    /// Get the current hierarchy
    pub fn get_hierarchy(&self) -> Vec<String> {
        self.selected_hierarchy.clone()
    }
}
