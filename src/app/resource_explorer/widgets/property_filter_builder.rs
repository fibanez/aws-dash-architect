use crate::app::resource_explorer::{
    BooleanOperator, PropertyCatalog, PropertyFilter, PropertyFilterGroup, PropertyFilterType,
};
use egui::Ui;

/// Widget for building complex property filter queries with visual UI
pub struct PropertyFilterBuilderWidget {
    /// The filter group being edited
    filter_group: PropertyFilterGroup,
    /// Property catalog for autocomplete and type information
    property_catalog: PropertyCatalog,
    /// Frame counter for debouncing logs
    frame_count: u64,
}

impl PropertyFilterBuilderWidget {
    /// Create a new property filter builder widget
    pub fn new(filter_group: PropertyFilterGroup, property_catalog: PropertyCatalog) -> Self {
        Self {
            filter_group,
            property_catalog,
            frame_count: 0,
        }
    }


    /// Render the filter builder UI
    pub fn show(&mut self, ui: &mut Ui) -> PropertyFilterGroup {
        self.frame_count += 1;

        // Only log diagnostics every 5 seconds (300 frames at 60fps)
        if self.frame_count % 300 == 0 {
            tracing::debug!(
                "Filter Frame {}: PropertyFilterBuilder - {} filters, catalog has {} properties",
                self.frame_count,
                self.filter_group.total_filter_count(),
                self.property_catalog.keys().count()
            );
        }

        ui.vertical(|ui| {
            ui.heading("Property Filter Builder");
            ui.add_space(8.0);

            // Render the main filter group
            Self::render_filter_group_static(
                ui,
                &mut self.filter_group,
                &self.property_catalog,
                0,
                &[],
            );
        });

        self.filter_group.clone()
    }

    /// Render a filter group with all its filters and sub-groups
    fn render_filter_group_static(
        ui: &mut Ui,
        group: &mut PropertyFilterGroup,
        catalog: &PropertyCatalog,
        depth: usize,
        group_path: &[usize],
    ) {
        // Visual grouping with indentation
        let indent = depth as f32 * 20.0;
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.add_space(indent);

            // Group operator toggle (AND/OR) - using button for clarity
            if depth > 0 || !group.filters.is_empty() || !group.sub_groups.is_empty() {
                let operator_text = match group.operator {
                    BooleanOperator::And => "AND",
                    BooleanOperator::Or => "OR",
                };

                if ui.button(operator_text).clicked() {
                    // Toggle operator
                    group.operator = match group.operator {
                        BooleanOperator::And => BooleanOperator::Or,
                        BooleanOperator::Or => BooleanOperator::And,
                    };
                }
            }
        });

        // Render each filter in the group
        let mut filters_to_remove = Vec::new();
        for (index, filter) in group.filters.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add_space(indent);

                // Render the filter row with depth, group_path, and index for unique IDs
                if Self::render_filter_row_static(ui, filter, catalog, depth, group_path, index) {
                    filters_to_remove.push(index);
                }
            });
        }

        // Remove deleted filters (in reverse order to maintain indices)
        for index in filters_to_remove.iter().rev() {
            group.filters.remove(*index);
        }

        // Add Filter and Add Sub-Group buttons for this group
        ui.horizontal(|ui| {
            ui.add_space(indent);

            if ui.button("+ Add Filter").clicked() {
                tracing::info!("Adding property filter to group at depth {}", depth);
                group.add_filter(PropertyFilter::new(
                    String::new(),
                    PropertyFilterType::Equals,
                ));
            }

            if ui.button("+ Add Sub-Group").clicked() {
                tracing::info!("Adding property sub-group at depth {}", depth);
                let new_group = PropertyFilterGroup::new();
                group.add_sub_group(new_group);
            }
        });

        ui.add_space(4.0);

        // Render sub-groups recursively
        let mut groups_to_remove = Vec::new();
        for (index, sub_group) in group.sub_groups.iter_mut().enumerate() {
            // Create new group path for this sub-group
            let mut new_path = group_path.to_vec();
            new_path.push(index);

            ui.group(|ui| {
                ui.add_space(4.0);
                Self::render_filter_group_static(ui, sub_group, catalog, depth + 1, &new_path);

                ui.horizontal(|ui| {
                    ui.add_space(indent + 20.0);
                    if ui.button("X Remove Group").clicked() {
                        groups_to_remove.push(index);
                    }
                });
            });
        }

        // Remove deleted groups (in reverse order to maintain indices)
        for index in groups_to_remove.iter().rev() {
            group.sub_groups.remove(*index);
        }
    }

    /// Render a single property filter row
    /// Returns true if the filter should be deleted
    fn render_filter_row_static(
        ui: &mut Ui,
        filter: &mut PropertyFilter,
        catalog: &PropertyCatalog,
        _depth: usize,
        group_path: &[usize],
        filter_index: usize,
    ) -> bool {
        let mut should_delete = false;

        // Create unique ID suffix from group path and filter index
        let id_suffix = if group_path.is_empty() {
            format!("{}", filter_index)
        } else {
            format!(
                "{}_{}",
                group_path
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join("_"),
                filter_index
            )
        };

        // Property path dropdown with stable index-based ID
        let property_keys: Vec<_> = catalog
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
                    && key.path != "raw_properties"
                    && key.path != "properties"
            })
            .take(100)
            .collect();

        // Show full property path
        let display_path = &filter.property_path;

        let combo_response = egui::ComboBox::from_id_salt(format!("property_{}", id_suffix))
            .selected_text(if filter.property_path.is_empty() {
                "Select property..."
            } else {
                display_path
            })
            .width(300.0)
            .show_ui(ui, |ui| {
                for prop_key in property_keys.iter() {
                    // Show full property path
                    let display_name = &prop_key.path;

                    let type_name = prop_key.value_type.display_name();
                    let label = format!("{} ({})", display_name, type_name);
                    if ui
                        .selectable_label(filter.property_path == prop_key.path, label)
                        .clicked()
                    {
                        tracing::info!("Property SELECTED: {}", prop_key.path);
                        filter.property_path = prop_key.path.clone();
                        filter.expected_type = Some(prop_key.value_type);
                    }
                }
            });

        // Event-based logging only (not per-frame)
        if combo_response.response.clicked() {
            tracing::info!("Property ComboBox CLICKED!");
        }

        ui.add_space(4.0);

        // Filter type dropdown with stable index-based ID
        egui::ComboBox::from_id_salt(format!("filter_type_{}", id_suffix))
            .selected_text(filter.filter_type.display_name())
            .width(120.0)
            .show_ui(ui, |ui| {
                for filter_type in PropertyFilterType::all() {
                    if ui
                        .selectable_label(
                            filter.filter_type == filter_type,
                            filter_type.display_name(),
                        )
                        .clicked()
                    {
                        tracing::info!("Filter type SELECTED: {:?}", filter_type);
                        filter.filter_type = filter_type;
                    }
                }
            });

        ui.add_space(4.0);

        // Value input (if required)
        if filter.filter_type.requires_value() {
            if filter.filter_type.supports_multiple_values() {
                // Multiple values (In/NotIn) - comma-separated text input
                if filter.values.is_empty() {
                    filter.values.push(String::new());
                }

                let mut values_str = filter.values.join(",");
                let text_edit = ui
                    .add(egui::TextEdit::singleline(&mut values_str).hint_text("value1,value2,..."));

                if text_edit.changed() {
                    filter.values = values_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            } else {
                // Single value - use autocomplete for Equals filter type
                if filter.values.is_empty() {
                    filter.values.push(String::new());
                }

                // Find common values for this property (for autocomplete)
                let common_values: Vec<String> = if !filter.property_path.is_empty() {
                    catalog
                        .keys()
                        .find(|k| k.path == filter.property_path)
                        .map(|k| k.common_values.clone())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

                // For Equals filter with available values, show dropdown
                if filter.filter_type == PropertyFilterType::Equals && !common_values.is_empty() {
                    egui::ComboBox::from_id_salt(format!("value_{}", id_suffix))
                        .selected_text(if filter.values[0].is_empty() {
                            "Select value..."
                        } else {
                            &filter.values[0]
                        })
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for value in &common_values {
                                if ui
                                    .selectable_label(filter.values[0] == *value, value)
                                    .clicked()
                                {
                                    filter.values[0] = value.clone();
                                }
                            }

                            // Allow custom value entry
                            ui.separator();
                            ui.label("Or enter custom value:");
                            ui.text_edit_singleline(&mut filter.values[0]);
                        });
                } else {
                    // Text input for other filter types
                    ui.add(
                        egui::TextEdit::singleline(&mut filter.values[0])
                            .hint_text("Enter value..."),
                    );
                }
            }
        } else {
            // No value needed
            ui.label("(no value needed)");
        }

        ui.add_space(4.0);

        // Delete button
        if ui.button("X").clicked() {
            should_delete = true;
        }

        should_delete
    }
}
