use crate::app::resource_explorer::tag_discovery::TagDiscovery;
use egui::Ui;
use egui_dnd::dnd;

/// Widget for building hierarchical tag grouping configurations with drag-and-drop UI
pub struct TagHierarchyBuilderWidget {
    /// Selected hierarchy (order matters - top to bottom = first level to last level)
    selected_hierarchy: Vec<String>,
    /// Tag discovery service for available tags
    tag_discovery: TagDiscovery,
    /// Search filter for left panel
    search_filter: String,
}

impl TagHierarchyBuilderWidget {
    /// Create a new tag hierarchy builder widget
    pub fn new(tag_discovery: TagDiscovery, initial_hierarchy: Vec<String>) -> Self {
        Self {
            selected_hierarchy: initial_hierarchy,
            tag_discovery,
            search_filter: String::new(),
        }
    }

    /// Render the hierarchy builder UI
    /// Returns: (hierarchy, apply_clicked, cancel_clicked)
    pub fn show(&mut self, ui: &mut Ui) -> (Vec<String>, bool, bool) {
        let mut apply_clicked = false;
        let mut cancel_clicked = false;

        ui.vertical(|ui| {
            ui.heading("Configure Tag Hierarchy");
            ui.add_space(8.0);
            ui.label("Drag tag keys to build a multi-level grouping hierarchy");
            ui.separator();

            // Two-panel layout
            ui.columns(2, |columns| {
                // Left panel: Available tags
                self.render_left_panel(&mut columns[0]);

                // Right panel: Selected hierarchy
                self.render_right_panel(&mut columns[1]);
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

    /// Render the left panel with available tag keys
    fn render_left_panel(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Available Tags");
            ui.separator();

            // Search box
            ui.horizontal(|ui| {
                ui.label("Search:");
                let search_response = ui.text_edit_singleline(&mut self.search_filter);
                if search_response.changed() {
                    tracing::debug!("Search filter changed to: '{}'", self.search_filter);
                }
            });
            ui.add_space(4.0);

            // Scrollable list of available tags
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    let tag_keys = self.tag_discovery.get_tag_keys_by_popularity();

                    for (tag_key, resource_count) in tag_keys.iter().take(50) {
                        // Apply search filter
                        if !self.search_filter.is_empty()
                            && !tag_key
                                .to_lowercase()
                                .contains(&self.search_filter.to_lowercase())
                        {
                            continue;
                        }

                        // Check if already in hierarchy
                        let is_selected = self.selected_hierarchy.contains(tag_key);

                        // Get tag metadata
                        if let Some(metadata) = self.tag_discovery.get_tag_metadata(tag_key) {
                            let value_count = metadata.value_count();
                            let label = format!(
                                "{} ({} resources, {} values)",
                                tag_key, resource_count, value_count
                            );

                            // Show as drag source
                            let response = ui.dnd_drag_source(
                                egui::Id::new("available_tag").with(tag_key),
                                tag_key.clone(),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        if is_selected {
                                            ui.add_enabled(
                                                false,
                                                egui::Label::new(
                                                    egui::RichText::new(&label).weak(),
                                                ),
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
                                self.selected_hierarchy.push(tag_key.clone());
                                tracing::info!("Added tag to hierarchy: {}", tag_key);
                            }

                            // Show tooltip
                            if response.response.hovered() && !is_selected {
                                response.response.on_hover_text(
                                    "Drag to right panel or double-click to add to hierarchy",
                                );
                            } else if response.response.hovered() && is_selected {
                                response.response.on_hover_text("Already in hierarchy");
                            }
                        }
                    }
                });
        });
    }

    /// Render the right panel with selected hierarchy (drag-and-drop reordering)
    fn render_right_panel(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Hierarchy (drag to reorder)");
            ui.separator();

            if self.selected_hierarchy.len() >= 5 {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 0),
                    "WARNING: Maximum 5 levels reached",
                );
            } else {
                ui.label(format!("Levels: {} / 5", self.selected_hierarchy.len()));
            }
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if self.selected_hierarchy.is_empty() {
                        // Empty state with drop zone
                        let (drop_response, payload) = ui.dnd_drop_zone::<String, ()>(
                            egui::Frame::default()
                                .inner_margin(egui::Margin::same(10))
                                .stroke(egui::Stroke::new(1.0, ui.visuals().weak_text_color()))
                                .fill(ui.visuals().faint_bg_color),
                            |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(50.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Drag tag keys here to build hierarchy",
                                        )
                                        .weak()
                                        .size(14.0),
                                    );
                                    ui.add_space(50.0);
                                });
                            },
                        );

                        // Handle drop for first tag
                        if let Some(dropped_tag) = payload {
                            let tag_string = dropped_tag.as_ref().clone();
                            if self.selected_hierarchy.len() < 5 {
                                self.selected_hierarchy.push(tag_string.clone());
                                tracing::info!("Dropped first tag into hierarchy: {}", tag_string);
                            } else {
                                tracing::debug!("Drop ignored - max hierarchy depth reached (5)");
                            }
                        }

                        // Visual feedback during hover
                        if drop_response.response.hovered() {
                            tracing::debug!(
                                "Empty state drop zone hovered - ready to accept first tag"
                            );
                            let rect = drop_response.response.rect;
                            let stroke =
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 100));
                            ui.painter()
                                .line_segment([rect.left_top(), rect.right_top()], stroke);
                            ui.painter()
                                .line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
                            ui.painter()
                                .line_segment([rect.left_top(), rect.left_bottom()], stroke);
                            ui.painter()
                                .line_segment([rect.right_top(), rect.right_bottom()], stroke);
                        }
                    } else {
                        // Render reorderable list with egui_dnd
                        let mut items_to_remove = Vec::new();
                        let hierarchy_len = self.selected_hierarchy.len();

                        dnd(ui, "hierarchy_list").with_animation_time(0.2).show_vec(
                            &mut self.selected_hierarchy,
                            |ui, tag, handle, state| {
                                handle.ui(ui, |ui| {
                                    ui.label(":::");
                                });

                                ui.horizontal(|ui| {
                                    ui.label(format!("{}.", state.index + 1));
                                    ui.label(tag.as_str());

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.small_button("X").clicked() {
                                                items_to_remove.push(state.index);
                                            }
                                        },
                                    );
                                });

                                // Visual feedback during drag
                                if state.dragged {
                                    let rect = ui.min_rect();
                                    let stroke =
                                        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
                                    ui.painter()
                                        .line_segment([rect.left_top(), rect.right_top()], stroke);
                                    ui.painter().line_segment(
                                        [rect.left_bottom(), rect.right_bottom()],
                                        stroke,
                                    );
                                    ui.painter().line_segment(
                                        [rect.left_top(), rect.left_bottom()],
                                        stroke,
                                    );
                                    ui.painter().line_segment(
                                        [rect.right_top(), rect.right_bottom()],
                                        stroke,
                                    );
                                }

                                // Show arrow indicator between items
                                if state.index < hierarchy_len - 1 {
                                    ui.label(egui::RichText::new("  |").weak().size(12.0));
                                }
                            },
                        );

                        // Remove marked items (in reverse to maintain indices)
                        for index in items_to_remove.iter().rev() {
                            let removed = self.selected_hierarchy.remove(*index);
                            tracing::info!("Removed tag from hierarchy: {}", removed);
                        }

                        // Show drop zone for adding from left panel
                        let (drop_response, payload) = ui.dnd_drop_zone::<String, ()>(
                            egui::Frame::default()
                                .inner_margin(egui::Margin::same(10))
                                .stroke(egui::Stroke::new(1.0, ui.visuals().weak_text_color()))
                                .fill(ui.visuals().faint_bg_color),
                            |ui| {
                                ui.label(egui::RichText::new("Drop here to add").weak().size(12.0));
                            },
                        );

                        // Handle drop
                        if let Some(dropped_tag) = payload {
                            let tag_string = dropped_tag.as_ref().clone();
                            if !self.selected_hierarchy.contains(&tag_string)
                                && self.selected_hierarchy.len() < 5
                            {
                                self.selected_hierarchy.push(tag_string.clone());
                                tracing::info!("Dropped tag into hierarchy: {}", tag_string);
                            } else if self.selected_hierarchy.contains(&tag_string) {
                                tracing::debug!(
                                    "Drop ignored - tag already in hierarchy: {}",
                                    tag_string
                                );
                            } else {
                                tracing::debug!("Drop ignored - max hierarchy depth reached (5)");
                            }
                        }

                        // Visual feedback during hover
                        if drop_response.response.hovered() {
                            tracing::debug!("Drop zone hovered - ready to accept tag");
                            let rect = drop_response.response.rect;
                            let stroke =
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 100));
                            ui.painter()
                                .line_segment([rect.left_top(), rect.right_top()], stroke);
                            ui.painter()
                                .line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
                            ui.painter()
                                .line_segment([rect.left_top(), rect.left_bottom()], stroke);
                            ui.painter()
                                .line_segment([rect.right_top(), rect.right_bottom()], stroke);
                        }
                    }
                });
        });
    }

    /// Render the preview section showing how the hierarchy will look
    fn render_preview(&self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.heading("Preview");
            ui.separator();

            if self.selected_hierarchy.is_empty() {
                ui.label(egui::RichText::new("No hierarchy configured").weak());
            } else {
                // Show hierarchy arrow notation
                let hierarchy_text = self.selected_hierarchy.join(" > ");
                ui.label(egui::RichText::new(&hierarchy_text).strong());

                ui.add_space(8.0);

                // Show sample tree structure
                ui.label("Sample tree structure:");
                ui.add_space(4.0);

                // Generate sample based on first tag's values
                if let Some(first_tag) = self.selected_hierarchy.first() {
                    if let Some(metadata) = self.tag_discovery.get_tag_metadata(first_tag) {
                        let values = metadata.get_sorted_values();

                        for value in values.iter().take(3) {
                            ui.label(format!("  {}: {}", first_tag, value));

                            // Show second level if available
                            if self.selected_hierarchy.len() > 1 {
                                if let Some(second_tag) = self.selected_hierarchy.get(1) {
                                    ui.label(format!("    {}: ...", second_tag));
                                }
                            }
                        }

                        if values.len() > 3 {
                            ui.label(format!("   ... and {} more", values.len() - 3));
                        }
                    }
                }
            }
        });
    }

    /// Render action buttons (Apply and Cancel)
    /// Returns: (apply_clicked, cancel_clicked)
    fn render_buttons(&self, ui: &mut Ui) -> (bool, bool) {
        let mut apply_clicked = false;
        let mut cancel_clicked = false;

        ui.horizontal(|ui| {
            // Apply button (disabled if hierarchy is empty)
            let apply_enabled = !self.selected_hierarchy.is_empty();

            if ui
                .add_enabled(apply_enabled, egui::Button::new("Apply"))
                .clicked()
            {
                apply_clicked = true;
            }

            if !apply_enabled {
                ui.label(
                    egui::RichText::new("Select at least one tag to apply")
                        .weak()
                        .size(12.0),
                );
            }

            ui.add_space(10.0);

            if ui.button("Cancel").clicked() {
                cancel_clicked = true;
            }
        });

        (apply_clicked, cancel_clicked)
    }
}
