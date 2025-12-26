use crate::app::resource_explorer::state::{
    BooleanOperator, TagFilter, TagFilterGroup, TagFilterType,
};
use crate::app::resource_explorer::tag_discovery::TagDiscovery;
use egui::Ui;

/// Widget for building complex tag filter queries with visual UI
pub struct TagFilterBuilderWidget {
    /// The filter group being edited
    filter_group: TagFilterGroup,
    /// Tag discovery service for autocomplete
    tag_discovery: TagDiscovery,
    /// Frame counter for debouncing logs
    frame_count: u64,
}

impl TagFilterBuilderWidget {
    /// Create a new tag filter builder widget
    pub fn new(filter_group: TagFilterGroup, tag_discovery: TagDiscovery) -> Self {
        Self {
            filter_group,
            tag_discovery,
            frame_count: 0,
        }
    }

    /// Render the filter builder UI
    pub fn show(&mut self, ui: &mut Ui) -> TagFilterGroup {
        self.frame_count += 1;

        // Only log diagnostics every 5 seconds (300 frames at 60fps)
        if self.frame_count % 300 == 0 {
            tracing::debug!(
                "🔍 Frame {}: TagFilterBuilder - {} filters, tag_discovery has {} keys",
                self.frame_count,
                self.filter_group.filter_count(),
                self.tag_discovery.tag_key_count()
            );
        }

        ui.vertical(|ui| {
            ui.heading("Tag Filter Builder");
            ui.add_space(8.0);

            // Render the main filter group
            Self::render_filter_group_static(
                ui,
                &mut self.filter_group,
                &self.tag_discovery,
                0,
                &[],
            );
        });

        self.filter_group.clone()
    }

    /// Get the current filter as a formatted expression string for logging
    pub fn get_filter_expression(&self) -> String {
        Self::format_filter_expression(&self.filter_group, 0)
    }

    /// Format the filter group as a parenthesized expression for logging
    #[allow(clippy::only_used_in_recursion)]
    pub fn format_filter_expression(group: &TagFilterGroup, depth: usize) -> String {
        let mut parts = Vec::new();
        let empty_string = String::new();

        // Add individual filters
        for filter in &group.filters {
            let filter_expr = if filter.tag_key.is_empty() {
                "(empty filter)".to_string()
            } else {
                match filter.filter_type {
                    TagFilterType::Equals => {
                        let value = filter.values.first().unwrap_or(&empty_string);
                        format!("{} = {}", filter.tag_key, value)
                    }
                    TagFilterType::NotEquals => {
                        let value = filter.values.first().unwrap_or(&empty_string);
                        format!("{} != {}", filter.tag_key, value)
                    }
                    TagFilterType::Contains => {
                        let value = filter.values.first().unwrap_or(&empty_string);
                        format!("{} contains {}", filter.tag_key, value)
                    }
                    TagFilterType::NotContains => {
                        let value = filter.values.first().unwrap_or(&empty_string);
                        format!("{} not-contains {}", filter.tag_key, value)
                    }
                    TagFilterType::StartsWith => {
                        let value = filter.values.first().unwrap_or(&empty_string);
                        format!("{} starts-with {}", filter.tag_key, value)
                    }
                    TagFilterType::EndsWith => {
                        let value = filter.values.first().unwrap_or(&empty_string);
                        format!("{} ends-with {}", filter.tag_key, value)
                    }
                    TagFilterType::In => {
                        format!("{} in [{}]", filter.tag_key, filter.values.join(", "))
                    }
                    TagFilterType::NotIn => {
                        format!("{} not-in [{}]", filter.tag_key, filter.values.join(", "))
                    }
                    TagFilterType::Exists => {
                        format!("{} exists", filter.tag_key)
                    }
                    TagFilterType::NotExists => {
                        format!("{} not-exists", filter.tag_key)
                    }
                    TagFilterType::Regex => {
                        let value = filter.values.first().unwrap_or(&empty_string);
                        format!("{} matches /{}/", filter.tag_key, value)
                    }
                }
            };
            parts.push(filter_expr);
        }

        // Add sub-groups recursively
        for sub_group in &group.sub_groups {
            let sub_expr = Self::format_filter_expression(sub_group, depth + 1);
            parts.push(format!("({})", sub_expr));
        }

        // Join with operator
        if parts.is_empty() {
            "(empty)".to_string()
        } else if parts.len() == 1 {
            parts[0].clone()
        } else {
            let operator = match group.operator {
                BooleanOperator::And => " AND ",
                BooleanOperator::Or => " OR ",
            };
            parts.join(operator)
        }
    }

    /// Render a filter group with all its filters and sub-groups
    fn render_filter_group_static(
        ui: &mut Ui,
        group: &mut TagFilterGroup,
        tag_discovery: &TagDiscovery,
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
                if Self::render_filter_row_static(
                    ui,
                    filter,
                    tag_discovery,
                    depth,
                    group_path,
                    index,
                ) {
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
                tracing::info!("Adding filter to group at depth {}", depth);
                group.add_filter(TagFilter {
                    tag_key: String::new(),
                    filter_type: TagFilterType::Equals,
                    values: vec![String::new()],
                    pattern: None,
                });
            }

            if ui.button("+ Add Sub-Group").clicked() {
                tracing::info!("Adding sub-group at depth {}", depth);
                let new_group = TagFilterGroup::new();
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
                Self::render_filter_group_static(
                    ui,
                    sub_group,
                    tag_discovery,
                    depth + 1,
                    &new_path,
                );

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

    /// Render a single filter row
    /// Returns true if the filter should be deleted
    fn render_filter_row_static(
        ui: &mut Ui,
        filter: &mut TagFilter,
        tag_discovery: &TagDiscovery,
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

        // Tag key dropdown with stable index-based ID (no visible label)
        // Get tag keys BEFORE creating ComboBox
        let tag_keys = tag_discovery.get_tag_keys_by_popularity();

        let combo_response = egui::ComboBox::from_id_salt(format!("tag_key_{}", id_suffix))
            .selected_text(if filter.tag_key.is_empty() {
                "Select tag key..."
            } else {
                &filter.tag_key
            })
            .width(150.0)
            .show_ui(ui, |ui| {
                for (tag_key, count) in tag_keys.iter().take(20) {
                    let label = format!("{} ({})", tag_key, count);
                    if ui
                        .selectable_label(filter.tag_key == *tag_key, label)
                        .clicked()
                    {
                        tracing::info!("🔍 ✅ Tag key SELECTED: {}", tag_key);
                        filter.tag_key = tag_key.clone();
                    }
                }
            });

        // Event-based logging only (not per-frame)
        if combo_response.response.clicked() {
            tracing::info!("🔍 ✅ Tag Key ComboBox CLICKED!");
        }

        ui.add_space(4.0);

        // Filter type dropdown with stable index-based ID (no visible label)
        egui::ComboBox::from_id_salt(format!("filter_type_{}", id_suffix))
            .selected_text(Self::filter_type_display(&filter.filter_type))
            .width(120.0)
            .show_ui(ui, |ui| {
                let filter_types = [
                    TagFilterType::Equals,
                    TagFilterType::NotEquals,
                    TagFilterType::Contains,
                    TagFilterType::NotContains,
                    TagFilterType::StartsWith,
                    TagFilterType::EndsWith,
                    TagFilterType::In,
                    TagFilterType::NotIn,
                    TagFilterType::Exists,
                    TagFilterType::NotExists,
                    TagFilterType::Regex,
                ];

                for ft in &filter_types {
                    if ui
                        .selectable_label(
                            std::mem::discriminant(&filter.filter_type)
                                == std::mem::discriminant(ft),
                            Self::filter_type_display(ft),
                        )
                        .clicked()
                    {
                        tracing::info!("🔍 ✅ Filter type SELECTED: {:?}", ft);
                        filter.filter_type = ft.clone();
                    }
                }
            });

        ui.add_space(4.0);

        // Value input (only for filter types that need values)
        match filter.filter_type {
            TagFilterType::Exists | TagFilterType::NotExists => {
                // No value needed
                ui.label("(no value needed)");
            }
            TagFilterType::In | TagFilterType::NotIn => {
                // Multi-value input with autocomplete
                if filter.values.is_empty() {
                    filter.values.push(String::new());
                }

                // Get discovered values for the selected tag key
                let discovered_values = if !filter.tag_key.is_empty() {
                    tag_discovery.get_tag_values(&filter.tag_key)
                } else {
                    Vec::new()
                };

                // Multi-value dropdown with stable index-based ID (no visible label)
                egui::ComboBox::from_id_salt(format!("multi_value_{}", id_suffix))
                    .selected_text(if filter.values.is_empty() || filter.values[0].is_empty() {
                        format!("Select values... ({} selected)", filter.values.len())
                    } else {
                        format!("{} values selected", filter.values.len())
                    })
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        // Show discovered values as checkboxes for multi-select
                        for value in discovered_values.iter().take(20) {
                            let mut is_selected = filter.values.contains(value);
                            if ui.checkbox(&mut is_selected, value).changed() {
                                if is_selected && !filter.values.contains(value) {
                                    tracing::info!("🔍 ✅ Multi-value ADDED: {}", value);
                                    filter.values.push(value.clone());
                                } else if !is_selected {
                                    tracing::info!("🔍 ✅ Multi-value REMOVED: {}", value);
                                    filter.values.retain(|v| v != value);
                                }
                            }
                        }

                        ui.separator();

                        // Add manual input option
                        ui.label("Or enter custom value:");
                        let mut custom_value = String::new();
                        if ui.text_edit_singleline(&mut custom_value).lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !custom_value.is_empty()
                            && !filter.values.contains(&custom_value)
                        {
                            filter.values.push(custom_value);
                        }
                    });
            }
            _ => {
                // Single value input with autocomplete
                if filter.values.is_empty() {
                    filter.values.push(String::new());
                }

                // Get discovered values for the selected tag key
                let discovered_values = if !filter.tag_key.is_empty() {
                    tag_discovery.get_tag_values(&filter.tag_key)
                } else {
                    Vec::new()
                };

                if discovered_values.is_empty() {
                    // No autocomplete available - use plain text input
                    ui.text_edit_singleline(&mut filter.values[0]);
                } else {
                    // Single-value dropdown with stable index-based ID (no visible label)
                    egui::ComboBox::from_id_salt(format!("value_{}", id_suffix))
                        .selected_text(if filter.values[0].is_empty() {
                            "Select or enter value..."
                        } else {
                            &filter.values[0]
                        })
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            // Show discovered values
                            for value in discovered_values.iter().take(20) {
                                if ui
                                    .selectable_label(filter.values[0] == *value, value)
                                    .clicked()
                                {
                                    tracing::info!("🔍 ✅ Single value SELECTED: {}", value);
                                    filter.values[0] = value.clone();
                                }
                            }

                            ui.separator();

                            // Add manual input option
                            ui.label("Or enter custom value:");
                            ui.text_edit_singleline(&mut filter.values[0]);
                        });
                }
            }
        }

        ui.add_space(4.0);

        // Delete button
        if ui.button("X").clicked() {
            should_delete = true;
        }

        should_delete
    }

    /// Get display text for filter type
    fn filter_type_display(filter_type: &TagFilterType) -> &'static str {
        match filter_type {
            TagFilterType::Equals => "Equals",
            TagFilterType::NotEquals => "Not Equals",
            TagFilterType::Contains => "Contains",
            TagFilterType::NotContains => "Not Contains",
            TagFilterType::StartsWith => "Starts With",
            TagFilterType::EndsWith => "Ends With",
            TagFilterType::In => "In",
            TagFilterType::NotIn => "Not In",
            TagFilterType::Exists => "Exists",
            TagFilterType::NotExists => "Not Exists",
            TagFilterType::Regex => "Regex",
        }
    }
}
