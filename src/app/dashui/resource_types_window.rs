use crate::app::dashui::app::fuzzy_match_score;
use crate::app::dashui::keyboard_navigation::{
    ElementAction, KeyEventResult, NavigableElement, NavigableElementType, NavigableWindow,
    NavigationCommand, NavigationContext, NavigationMode,
};
use crate::app::dashui::window_focus::{FocusableWindow, SimpleShowParams};
use eframe::egui;
use std::collections::HashMap;

#[derive(Default)]
pub struct ResourceTypesWindow {
    pub show: bool,
    pub resource_types: Vec<String>,
    pub resource_types_filter: String,
    pub selected_resource_index: Option<usize>,
}

impl ResourceTypesWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<String> {
        self.show_with_focus(ctx, false)
    }

    pub fn show_with_focus(&mut self, ctx: &egui::Context, bring_to_front: bool) -> Option<String> {
        if !self.show {
            return None;
        }

        let mut resource_to_open = None;

        let mut window = egui::Window::new("CloudFormation Resource Types (us-east-1)")
            .resizable(true)
            .default_width(400.0)
            .default_height(600.0)
            .default_pos(ctx.screen_rect().center());

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            // Add a close button
            if ui.button("Close").clicked() {
                self.show = false;
                self.selected_resource_index = None;
            }

            ui.separator();

            // Add a search/filter box with keyboard focus
            let _filter_response = ui
                .horizontal(|ui| {
                    ui.label("Fuzzy Search:");
                    // Note: Auto-focus removed to allow keyboard navigation to work properly
                    ui.text_edit_singleline(&mut self.resource_types_filter)
                })
                .inner;

            // Handle keyboard navigation
            let ctrl_pressed = ui.input(|i| i.modifiers.ctrl);
            let n_pressed = ui.input(|i| i.key_pressed(egui::Key::N));
            let p_pressed = ui.input(|i| i.key_pressed(egui::Key::P));
            let y_pressed = ui.input(|i| i.key_pressed(egui::Key::Y));
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

            ui.separator();

            // Display the resource types in a scrollable area
            egui::ScrollArea::vertical().show(ui, |ui| {
                let filter = &self.resource_types_filter;

                // Get filtered resources with scores
                let mut filtered_resources: Vec<(String, Option<usize>)> = self
                    .resource_types
                    .iter()
                    .map(|rt| {
                        (
                            rt.clone(),
                            if filter.is_empty() {
                                Some(0)
                            } else {
                                fuzzy_match_score(filter, rt)
                            },
                        )
                    })
                    .filter(|(_, score)| score.is_some())
                    .collect();

                // Sort by score (higher scores first)
                filtered_resources.sort_by(|a, b| b.1.unwrap_or(0).cmp(&a.1.unwrap_or(0)));

                // Count total and filtered resources
                let total = self.resource_types.len();
                let filtered = filtered_resources.len();

                ui.label(format!(
                    "Showing {}/{} resource types (fuzzy search)",
                    filtered, total
                ));

                ui.separator();

                // Handle Ctrl+N to select next item
                if ctrl_pressed && n_pressed && !filtered_resources.is_empty() {
                    if let Some(idx) = self.selected_resource_index {
                        // Select next item, wrapping around if needed
                        self.selected_resource_index = Some((idx + 1) % filtered_resources.len());
                    } else {
                        // Select first item if nothing is selected
                        self.selected_resource_index = Some(0);
                    }
                }

                // Handle Ctrl+P to select previous item
                if ctrl_pressed && p_pressed && !filtered_resources.is_empty() {
                    if let Some(idx) = self.selected_resource_index {
                        // Select previous item, wrapping around if needed
                        self.selected_resource_index = Some(if idx > 0 {
                            idx - 1
                        } else {
                            filtered_resources.len() - 1
                        });
                    } else {
                        // Select last item if nothing is selected
                        self.selected_resource_index = Some(filtered_resources.len() - 1);
                    }
                }

                // Handle Ctrl+Y or Enter to open selected resource
                if (ctrl_pressed && y_pressed || enter_pressed)
                    && self.selected_resource_index.is_some()
                {
                    let idx = self.selected_resource_index.unwrap();
                    if idx < filtered_resources.len() {
                        resource_to_open = Some(filtered_resources[idx].0.clone());
                    }
                }

                // Display the filtered resource types
                for (i, (resource_type, _)) in filtered_resources.iter().enumerate() {
                    let is_selected = self.selected_resource_index == Some(i);

                    // Use selectable label to highlight the selected item
                    let response = ui.selectable_label(is_selected, resource_type);

                    // Handle click to select and open resource details
                    if response.clicked() {
                        self.selected_resource_index = Some(i);
                        resource_to_open = Some(resource_type.clone());
                    }
                }
            });
        });

        // Return the resource type if one was selected
        resource_to_open
    }
}

impl FocusableWindow for ResourceTypesWindow {
    type ShowParams = SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "resource_types"
    }

    fn window_title(&self) -> String {
        "CloudFormation Resource Types".to_string()
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

impl ResourceTypesWindow {
    /// Collect navigable elements from this resource types window
    /// This method creates NavigableElement instances for the resource list and UI controls
    pub fn collect_navigable_elements(&self) -> Vec<NavigableElement> {
        let mut elements = Vec::new();

        if !self.show {
            return elements; // Window is not visible, no elements to collect
        }

        // Create base position estimates for typical window layout
        let mut y_offset = 100.0; // Start below window title
        let x_start = 50.0;
        let button_width = 80.0;
        let input_width = 300.0;
        let list_item_height = 25.0;
        let row_height = 30.0;

        // Close button (top of window)
        elements.push(NavigableElement {
            id: format!("{}_close_button", self.window_id()),
            element_type: NavigableElementType::Button,
            rect: egui::Rect::from_min_size(
                egui::Pos2::new(x_start, y_offset),
                egui::Vec2::new(button_width, 25.0),
            ),
            enabled: true,
            label: Some("Close".to_string()),
            supported_actions: vec![
                ElementAction::Click,
                ElementAction::Activate,
                ElementAction::Smart,
            ],
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("field_type".to_string(), "action_button".to_string());
                meta.insert("action".to_string(), "close".to_string());
                meta.insert("window_id".to_string(), self.window_id().to_string());
                meta.insert("window_title".to_string(), self.window_title());
                meta.insert("widget_content".to_string(), "Close".to_string());
                meta.insert("button_enabled".to_string(), "true".to_string());
                meta
            },
        });
        y_offset += row_height + 10.0;

        // Search/filter input field
        elements.push(NavigableElement {
            id: format!("{}_search_input", self.window_id()),
            element_type: NavigableElementType::TextInput,
            rect: egui::Rect::from_min_size(
                egui::Pos2::new(x_start + 120.0, y_offset),
                egui::Vec2::new(input_width, 25.0),
            ),
            enabled: true,
            label: Some("Fuzzy Search".to_string()),
            supported_actions: vec![
                ElementAction::Focus,
                ElementAction::Select,
                ElementAction::Copy,
                ElementAction::Smart,
            ],
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("field_type".to_string(), "search_filter".to_string());
                meta.insert("window_id".to_string(), self.window_id().to_string());
                meta.insert("window_title".to_string(), self.window_title());
                meta.insert(
                    "widget_content".to_string(),
                    self.resource_types_filter.clone(),
                );
                meta.insert(
                    "total_resources".to_string(),
                    self.resource_types.len().to_string(),
                );
                meta.insert(
                    "filter_active".to_string(),
                    (!self.resource_types_filter.is_empty()).to_string(),
                );
                meta
            },
        });
        y_offset += row_height + 20.0;

        // Get filtered resources list for navigation
        let filtered_resources = self.get_filtered_resources();

        // Add navigable elements for each visible resource type in the list
        for (i, (resource_type, _score)) in filtered_resources.iter().enumerate().take(15) {
            // Limit to first 15 visible items
            let is_selected = self.selected_resource_index == Some(i);

            elements.push(NavigableElement {
                id: format!("{}_resource_{}", self.window_id(), i),
                element_type: NavigableElementType::ListItem,
                rect: egui::Rect::from_min_size(
                    egui::Pos2::new(x_start, y_offset),
                    egui::Vec2::new(input_width + 120.0, list_item_height),
                ),
                enabled: true,
                label: Some(resource_type.clone()),
                supported_actions: vec![
                    ElementAction::Click,
                    ElementAction::Select,
                    ElementAction::Activate,
                    ElementAction::Smart,
                ],
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("field_type".to_string(), "resource_item".to_string());
                    meta.insert("resource_type".to_string(), resource_type.clone());
                    meta.insert("widget_content".to_string(), resource_type.clone());
                    meta.insert("list_index".to_string(), i.to_string());
                    meta.insert("is_selected".to_string(), is_selected.to_string());
                    meta.insert("window_id".to_string(), self.window_id().to_string());
                    meta.insert("window_title".to_string(), self.window_title());
                    meta.insert(
                        "total_filtered".to_string(),
                        filtered_resources.len().to_string(),
                    );
                    if let Some(score) = _score {
                        meta.insert("fuzzy_score".to_string(), score.to_string());
                    }
                    meta
                },
            });

            y_offset += list_item_height + 2.0;
        }

        tracing::info!("ResourceTypesWindow::collect_navigable_elements - Collected {} elements for window '{}' ({})",
                       elements.len(), self.window_title(), self.window_id());

        // Log summary of element types for debugging
        let mut type_counts = std::collections::HashMap::new();
        for element in &elements {
            let field_type = element
                .metadata
                .get("field_type")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            *type_counts.entry(field_type).or_insert(0) += 1;
        }
        tracing::debug!(
            "ResourceTypesWindow::collect_navigable_elements - Element types: {:?}",
            type_counts
        );
        elements
    }

    /// Get filtered resources list (helper method for element collection)
    fn get_filtered_resources(&self) -> Vec<(String, Option<usize>)> {
        let filter = &self.resource_types_filter;

        let mut filtered_resources: Vec<(String, Option<usize>)> = self
            .resource_types
            .iter()
            .map(|rt| {
                (
                    rt.clone(),
                    if filter.is_empty() {
                        Some(0)
                    } else {
                        fuzzy_match_score(filter, rt)
                    },
                )
            })
            .filter(|(_, score)| score.is_some())
            .collect();

        // Sort by score (higher scores first)
        filtered_resources.sort_by(|a, b| b.1.unwrap_or(0).cmp(&a.1.unwrap_or(0)));

        filtered_resources
    }
}

impl NavigableWindow for ResourceTypesWindow {
    fn get_navigation_context(&self) -> NavigationContext {
        let mut settings = HashMap::new();
        settings.insert("window_id".to_string(), self.window_id().to_string());
        settings.insert("window_title".to_string(), self.window_title());
        settings.insert("help_text".to_string(), "Resource Types Navigation:\n- j/k: Navigate list\n- /: Focus search\n- Enter: Select resource\n- Ctrl+N/P: Navigate with existing shortcuts".to_string());

        NavigationContext {
            supports_hints: true,
            supports_visual_mode: true,
            handles_scrolling: true,
            settings,
        }
    }

    fn get_custom_key_bindings(&self) -> HashMap<String, NavigationCommand> {
        let mut bindings = HashMap::new();

        // Add resource types window specific key bindings
        bindings.insert("/".to_string(), NavigationCommand::FocusSearchField); // Focus search field
        bindings.insert("ctrl+n".to_string(), NavigationCommand::NextElement); // Next resource (already implemented)
        bindings.insert("ctrl+p".to_string(), NavigationCommand::PreviousElement); // Previous resource (already implemented)
        bindings.insert("ctrl+y".to_string(), NavigationCommand::ActivateElement); // Open selected resource (already implemented)
        bindings.insert("enter".to_string(), NavigationCommand::ActivateElement); // Open selected resource
        bindings.insert("escape".to_string(), NavigationCommand::CloseWindow); // Close window

        bindings
    }

    fn handle_navigation_command(&mut self, command: NavigationCommand) -> KeyEventResult {
        match command {
            NavigationCommand::NextElement => {
                // Navigate to next resource (existing Ctrl+N logic)
                let filtered_resources = self.get_filtered_resources();
                if !filtered_resources.is_empty() {
                    if let Some(idx) = self.selected_resource_index {
                        self.selected_resource_index = Some((idx + 1) % filtered_resources.len());
                    } else {
                        self.selected_resource_index = Some(0);
                    }
                }
                KeyEventResult::Handled
            }
            NavigationCommand::PreviousElement => {
                // Navigate to previous resource (existing Ctrl+P logic)
                let filtered_resources = self.get_filtered_resources();
                if !filtered_resources.is_empty() {
                    if let Some(idx) = self.selected_resource_index {
                        self.selected_resource_index = Some(if idx > 0 {
                            idx - 1
                        } else {
                            filtered_resources.len() - 1
                        });
                    } else {
                        self.selected_resource_index = Some(filtered_resources.len() - 1);
                    }
                }
                KeyEventResult::Handled
            }
            NavigationCommand::ActivateElement => {
                // Open selected resource (existing Enter/Ctrl+Y logic)
                // Note: The actual resource opening would be handled by the calling code
                tracing::info!("Resource types window: Activate selected resource");
                KeyEventResult::Handled
            }
            NavigationCommand::FocusSearchField => {
                // Focus the search field - this would be handled by egui focus logic
                tracing::info!("Resource types window: Focus search field");
                KeyEventResult::Handled
            }
            NavigationCommand::CloseWindow => {
                // Close the resource types window
                self.show = false;
                self.selected_resource_index = None;
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
        tracing::debug!(
            "Resource types window navigation mode changed to: {:?}",
            new_mode
        );
    }
}
