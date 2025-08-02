use super::resource_lookup::{AwsResourceInfo, ResourceLookupService};
use egui::{self, Color32, Context, RichText, ScrollArea, TextEdit};
use std::sync::Arc;
use tracing::{debug, error};

/// State for the AWS Resource Picker Dialog
#[derive(Debug, Clone, PartialEq)]
pub enum ResourcePickerState {
    Closed,
    Loading,
    SelectingResource,
    ResourceSelected(String),
}

/// AWS Resource Picker Dialog for CloudFormation parameter selection
pub struct AwsResourcePickerDialog {
    pub state: ResourcePickerState,
    pub parameter_name: String,
    pub parameter_type: String,
    pub account_id: String,
    pub region: String,
    pub search_query: String,
    pub available_resources: Vec<AwsResourceInfo>,
    pub filtered_resources: Vec<AwsResourceInfo>,
    pub selected_resource: Option<AwsResourceInfo>,
    pub error_message: Option<String>,
    resource_lookup_service: Option<Arc<ResourceLookupService>>,
    last_search_query: String,
}

impl Default for AwsResourcePickerDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl AwsResourcePickerDialog {
    pub fn new() -> Self {
        Self {
            state: ResourcePickerState::Closed,
            parameter_name: String::new(),
            parameter_type: String::new(),
            account_id: String::new(),
            region: String::new(),
            search_query: String::new(),
            available_resources: Vec::new(),
            filtered_resources: Vec::new(),
            selected_resource: None,
            error_message: None,
            resource_lookup_service: None,
            last_search_query: String::new(),
        }
    }

    /// Open the resource picker for a specific parameter
    pub fn open_for_parameter(
        &mut self,
        parameter_name: String,
        parameter_type: String,
        account_id: String,
        region: String,
        resource_lookup_service: Arc<ResourceLookupService>,
    ) {
        self.parameter_name = parameter_name;
        self.parameter_type = parameter_type;
        self.account_id = account_id;
        self.region = region;
        self.search_query.clear();
        self.available_resources.clear();
        self.filtered_resources.clear();
        self.selected_resource = None;
        self.error_message = None;
        self.resource_lookup_service = Some(resource_lookup_service);
        self.last_search_query.clear();

        self.state = ResourcePickerState::Loading;

        // Start loading resources asynchronously
        self.load_resources();
    }

    /// Close the resource picker
    pub fn close(&mut self) {
        self.state = ResourcePickerState::Closed;
        self.available_resources.clear();
        self.filtered_resources.clear();
        self.search_query.clear();
        self.error_message = None;
    }

    /// Check if the dialog is open
    pub fn is_open(&self) -> bool {
        !matches!(self.state, ResourcePickerState::Closed)
    }

    /// Get the selected resource ID if a resource was selected
    pub fn get_selected_resource_id(&self) -> Option<String> {
        if matches!(self.state, ResourcePickerState::ResourceSelected(_)) {
            self.selected_resource.as_ref().map(|r| r.id.clone())
        } else {
            None
        }
    }

    /// Show the resource picker dialog
    pub fn show(&mut self, ctx: &Context) -> bool {
        let mut resource_selected = false;

        if !self.is_open() {
            return false;
        }

        let mut open = true;
        egui::Window::new("Select AWS Resource")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .min_width(600.0)
            .min_height(400.0)
            .show(ctx, |ui| {
                // Header with parameter information
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Parameter:").strong());
                        ui.label(&self.parameter_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Type:").strong());
                        ui.label(&self.parameter_type);
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Account:").strong());
                        ui.label(&self.account_id);
                        ui.separator();
                        ui.label(RichText::new("Region:").strong());
                        ui.label(&self.region);
                    });
                });

                ui.separator();

                match &self.state {
                    ResourcePickerState::Loading => {
                        ui.vertical_centered(|ui| {
                            ui.spinner();
                            ui.label("Loading resources...");
                        });
                    }
                    ResourcePickerState::SelectingResource => {
                        self.show_resource_selection(ui);
                    }
                    ResourcePickerState::ResourceSelected(_) => {
                        self.show_selected_resource(ui);
                        resource_selected = true;
                    }
                    ResourcePickerState::Closed => {}
                }

                // Error display
                if let Some(error) = &self.error_message {
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(220, 50, 50), format!("Error: {}", error));
                }

                // Action buttons
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.close();
                    }

                    if matches!(self.state, ResourcePickerState::ResourceSelected(_))
                        && ui.button("Use Selected Resource").clicked()
                    {
                        resource_selected = true;
                    }
                });
            });

        if !open {
            self.close();
        }

        resource_selected
    }

    /// Show the resource selection interface
    fn show_resource_selection(&mut self, ui: &mut egui::Ui) {
        // Search box
        ui.horizontal(|ui| {
            ui.label("Search:");
            let response = ui.add(
                TextEdit::singleline(&mut self.search_query)
                    .hint_text("Type to filter resources...")
                    .desired_width(300.0),
            );

            if response.changed() || self.search_query != self.last_search_query {
                self.filter_resources();
                self.last_search_query = self.search_query.clone();
            }

            ui.label(format!("({} resources)", self.filtered_resources.len()));
        });

        ui.add_space(5.0);

        // Resource list
        ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            if self.filtered_resources.is_empty() {
                ui.label("No resources found.");
            } else {
                let resources = self.filtered_resources.clone();
                for resource in &resources {
                    self.show_resource_item(ui, resource);
                }
            }
        });
    }

    /// Show a single resource item
    fn show_resource_item(&mut self, ui: &mut egui::Ui, resource: &AwsResourceInfo) {
        let response = ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // Resource ID
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("ID:").strong().size(12.0));
                        ui.label(RichText::new(&resource.id).monospace().size(12.0));
                    });

                    // Resource name if available
                    if let Some(name) = &resource.name {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Name:").strong().size(12.0));
                            ui.label(RichText::new(name).size(12.0));
                        });
                    }

                    // Status if available
                    if let Some(status) = &resource.status {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Status:").strong().size(12.0));
                            let status_color = match status.as_str() {
                                "available" | "running" | "active" => {
                                    Color32::from_rgb(40, 180, 40)
                                }
                                "pending" | "creating" => Color32::from_rgb(255, 150, 0),
                                "terminated" | "failed" | "error" => Color32::from_rgb(220, 50, 50),
                                _ => Color32::GRAY,
                            };
                            ui.colored_label(status_color, status);
                        });
                    }

                    // Tags if available
                    if !resource.tags.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Tags:").strong().size(11.0));
                            for (key, value) in &resource.tags {
                                ui.label(
                                    RichText::new(format!("{}:{}", key, value))
                                        .size(10.0)
                                        .color(Color32::GRAY),
                                );
                            }
                        });
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Select").clicked() {
                        self.selected_resource = Some(resource.clone());
                        self.state = ResourcePickerState::ResourceSelected(resource.id.clone());
                    }
                });
            });
        });

        // Highlight on hover
        if response.response.hovered() {
            ui.painter().rect_stroke(
                response.response.rect,
                2.0,
                egui::Stroke::new(1.0, Color32::from_rgb(100, 150, 255)),
                egui::StrokeKind::Outside,
            );
        }
    }

    /// Show the selected resource confirmation
    fn show_selected_resource(&mut self, ui: &mut egui::Ui) {
        if let Some(resource) = &self.selected_resource {
            ui.group(|ui| {
                ui.label(RichText::new("Selected Resource").strong().size(16.0));
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(RichText::new("ID:").strong());
                    ui.label(RichText::new(&resource.id).monospace());
                });

                if let Some(name) = &resource.name {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Name:").strong());
                        ui.label(name);
                    });
                }

                if let Some(arn) = &resource.arn {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("ARN:").strong());
                        ui.label(RichText::new(arn).monospace().size(11.0));
                    });
                }

                if let Some(description) = &resource.description {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Description:").strong());
                        ui.label(description);
                    });
                }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Choose Different Resource").clicked() {
                    self.selected_resource = None;
                    self.state = ResourcePickerState::SelectingResource;
                }
            });
        }
    }

    /// Load resources asynchronously
    fn load_resources(&mut self) {
        if let Some(service) = &self.resource_lookup_service {
            let service = service.clone();
            let parameter_type = self.parameter_type.clone();
            let account_id = self.account_id.clone();
            let region = self.region.clone();

            // Note: This is a simplified approach. In a real implementation,
            // you would want to use a proper async channel to communicate back to the UI
            // For now, we'll simulate the loading by immediately transitioning to selection
            tokio::spawn(async move {
                match service
                    .get_resources_for_parameter_type(&parameter_type, &account_id, &region)
                    .await
                {
                    Ok(resources) => {
                        debug!(
                            "Loaded {} resources for parameter type {}",
                            resources.len(),
                            parameter_type
                        );
                        // In a real implementation, send resources back to UI thread
                    }
                    Err(e) => {
                        error!("Failed to load resources: {}", e);
                        // In a real implementation, send error back to UI thread
                    }
                }
            });

            // For now, immediately transition to selection state
            // TODO: Implement proper async communication
            self.state = ResourcePickerState::SelectingResource;
        }
    }

    /// Filter resources based on search query
    fn filter_resources(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_resources = self.available_resources.clone();
        } else {
            let query_lower = self.search_query.to_lowercase();
            self.filtered_resources = self
                .available_resources
                .iter()
                .filter(|resource| {
                    resource.id.to_lowercase().contains(&query_lower)
                        || resource
                            .name
                            .as_ref()
                            .map(|name| name.to_lowercase().contains(&query_lower))
                            .unwrap_or(false)
                        || resource
                            .tags
                            .values()
                            .any(|tag_value| tag_value.to_lowercase().contains(&query_lower))
                })
                .cloned()
                .collect();
        }
    }

    /// Set available resources (for testing or manual setting)
    pub fn set_available_resources(&mut self, resources: Vec<AwsResourceInfo>) {
        self.available_resources = resources;
        self.filter_resources();
        if self.state == ResourcePickerState::Loading {
            self.state = ResourcePickerState::SelectingResource;
        }
    }

    /// Set an error message
    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        if self.state == ResourcePickerState::Loading {
            self.state = ResourcePickerState::SelectingResource;
        }
    }
}
