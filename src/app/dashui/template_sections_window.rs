use crate::app::cf_syntax;
use crate::app::cfn_resource_icons::get_icon_for_resource;
use crate::app::cfn_template::{CloudFormationTemplate, Output, Parameter, Rule};
use crate::app::dashui::app::fuzzy_match;
use crate::app::dashui::keyboard_navigation::{
    ElementAction, KeyEventResult, NavigableElement, NavigableElementType, NavigableWindow,
    NavigationCommand, NavigationContext, NavigationMode,
};
use crate::app::dashui::navigable_widgets::{NavigableWidgetManager, WidgetRegistrar};
use crate::app::dashui::window_focus::{FocusableWindow, ProjectShowParams};
use crate::app::projects::Project;
use crate::log_debug;
use crate::{register_button, register_radio_button, register_text_input};
use egui::{self, Context, Grid, RichText, ScrollArea, TextEdit, Ui, Window};
use egui_code_editor::{CodeEditor, ColorTheme};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, warn};

/// Represents a captured widget during UI rendering for the hint system
#[derive(Debug, Clone)]
pub struct CapturedWidget {
    pub id: String,
    pub rect: egui::Rect,
    pub widget_type: CapturedWidgetType,
    pub action: WidgetAction,
    pub label: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Types of widgets that can be captured for navigation
#[derive(Debug, Clone)]
pub enum CapturedWidgetType {
    Button,
    TextInput,
    ListItem,
    SectionTab,
}

/// Actions that can be performed on widgets
#[derive(Debug, Clone)]
pub enum WidgetAction {
    ClickButton(String),     // Button identifier to click
    FocusTextInput(String),  // Text input identifier to focus
    SelectListItem(String),  // List item identifier to select
    ActivateSection(String), // Section identifier to activate
}

#[derive(Debug)]
pub enum CommandResult {
    TemplateUpdated(Box<CloudFormationTemplate>),
    EditResource(String),
    DeleteResource(String),
    JsonEditResource(String),
}

#[derive(Default)]
pub struct TemplateSectionsWindow {
    pub show: bool,
    pub template: CloudFormationTemplate,
    pub selected_section: TemplateSection,
    pub error_message: Option<String>,
    // For editing
    pub editing_key: Option<String>,
    pub editing_value: String,
    // Resource list fields
    pub filter_text: String,
    pub sort_by: ResourceListSortOrder,
    // For deletion confirmation
    pub delete_confirmation: Option<String>,
    // Cached icon textures - loaded once when template changes
    icon_cache: HashMap<String, Option<egui::TextureHandle>>,
    // Widget position capture for hint system
    captured_widgets: Vec<CapturedWidget>,
    // Real widget registration system
    widget_manager: NavigableWidgetManager,
}

impl std::fmt::Debug for TemplateSectionsWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateSectionsWindow")
            .field("show", &self.show)
            .field("template", &self.template)
            .field("selected_section", &self.selected_section)
            .field("error_message", &self.error_message)
            .field("editing_key", &self.editing_key)
            .field("editing_value", &self.editing_value)
            .field("filter_text", &self.filter_text)
            .field("sort_by", &self.sort_by)
            .field("delete_confirmation", &self.delete_confirmation)
            .field("icon_cache", &format!("{} items", self.icon_cache.len()))
            .finish()
    }
}

/// Sort order for resources in the list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceListSortOrder {
    /// Sort by resource ID (alphabetical)
    Id,
    /// Sort by resource type (alphabetical)
    Type,
}

impl Default for ResourceListSortOrder {
    fn default() -> Self {
        Self::Id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TemplateSection {
    #[default]
    Resources,
    Description,
    Parameters,
    Outputs,
    Mappings,
    Metadata,
    Conditions,
    Rules,
    Transform,
}

enum Action {
    None,
    Edit(String, String), // key, serialized value
    Delete(String),
}

impl TemplateSectionsWindow {
    pub fn new() -> Self {
        Self {
            show: false,
            template: CloudFormationTemplate::default(),
            selected_section: TemplateSection::Resources,
            error_message: None,
            editing_key: None,
            editing_value: String::new(),
            filter_text: String::new(),
            sort_by: ResourceListSortOrder::Id,
            delete_confirmation: None,
            icon_cache: HashMap::new(),
            captured_widgets: Vec::new(),
            widget_manager: NavigableWidgetManager::new(),
        }
    }

    pub fn set_template(&mut self, template: CloudFormationTemplate) {
        self.template = template;
        self.error_message = None;
        self.editing_key = None;
        self.editing_value = String::new();
        self.delete_confirmation = None;

        // Clear icon cache when template changes
        self.icon_cache.clear();
        log_debug!("Cleared icon cache for new template");

        // Clear captured widgets when template changes
        self.captured_widgets.clear();
    }

    /// Get appropriate code theme based on UI theme
    fn get_code_theme(&self, ctx: &Context) -> ColorTheme {
        // Detect theme based on background color
        let bg_color = ctx.style().visuals.extreme_bg_color;
        let brightness = (bg_color.r() as f32 + bg_color.g() as f32 + bg_color.b() as f32) / 3.0;

        if brightness > 128.0 {
            ColorTheme::GITHUB_LIGHT
        } else {
            ColorTheme::GITHUB_DARK
        }
    }

    /// Start a new frame - clear previous widget captures
    pub fn start_widget_capture(&mut self) {
        self.captured_widgets.clear();
    }

    /// Capture a widget during UI rendering for the hint system
    pub fn capture_widget(
        &mut self,
        response: &egui::Response,
        widget_id: String,
        widget_type: CapturedWidgetType,
        action: WidgetAction,
        label: Option<String>,
    ) {
        let mut metadata = HashMap::new();
        metadata.insert("window_id".to_string(), self.window_id().to_string());
        metadata.insert("window_title".to_string(), self.window_title());
        if let Some(ref label_text) = label {
            metadata.insert("widget_content".to_string(), label_text.clone());
        }

        self.captured_widgets.push(CapturedWidget {
            id: widget_id,
            rect: response.rect,
            widget_type,
            action,
            label,
            metadata,
        });
    }

    /// Get all captured widgets from the current frame
    pub fn get_captured_widgets(&self) -> &[CapturedWidget] {
        &self.captured_widgets
    }

    /// Queue an action to be executed on a widget
    pub fn queue_widget_action(&mut self, element_id: String, action: ElementAction) {
        self.widget_manager.queue_action(element_id, action);
    }

    /// Load icon for a resource type if not already cached
    fn get_or_load_icon(
        &mut self,
        ctx: &Context,
        resource_type: &str,
    ) -> Option<&egui::TextureHandle> {
        // Check if we already have this icon in cache
        if self.icon_cache.contains_key(resource_type) {
            return self
                .icon_cache
                .get(resource_type)
                .and_then(|opt| opt.as_ref());
        }

        // Log that we're loading this icon (only happens once per resource type)
        debug!("Loading icon for resource type: {}", resource_type);

        // Get icon path
        let icon_path = get_icon_for_resource(resource_type);

        // Load the icon
        let texture = Self::load_icon_texture(ctx, resource_type, icon_path);

        // Cache the result (even if it's None)
        self.icon_cache.insert(resource_type.to_string(), texture);

        // Return reference to the cached value
        self.icon_cache
            .get(resource_type)
            .and_then(|opt| opt.as_ref())
    }

    /// Load an icon texture from disk
    fn load_icon_texture(
        ctx: &Context,
        resource_type: &str,
        icon_path: &str,
    ) -> Option<egui::TextureHandle> {
        // Log current working directory (only once)
        static LOGGED_CWD: std::sync::Once = std::sync::Once::new();
        LOGGED_CWD.call_once(|| {
            if let Ok(cwd) = std::env::current_dir() {
                log_debug!("Current working directory: {:?}", cwd);
            }
        });

        // Check if file exists first
        let path = std::path::Path::new(icon_path);
        if !path.exists() {
            warn!("Icon file does not exist: {:?}", path);
            // Try with absolute path from executable location
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let abs_icon_path = exe_dir.join(icon_path);
                    debug!("Trying absolute path: {:?}", abs_icon_path);
                    if abs_icon_path.exists() {
                        // Try to load from absolute path
                        return match std::fs::read(&abs_icon_path) {
                            Ok(image_bytes) => {
                                log_debug!(
                                    "Successfully read {} bytes from absolute icon path for {}",
                                    image_bytes.len(),
                                    resource_type
                                );
                                match egui_extras::image::load_image_bytes(&image_bytes) {
                                    Ok(image) => {
                                        log_debug!("Successfully decoded image for resource type: {}, size: {:?}", resource_type, image.size);
                                        let texture = ctx.load_texture(
                                            format!("resource_icon_{}", resource_type),
                                            image,
                                            Default::default(),
                                        );
                                        Some(texture)
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to decode image for {}: {:?}",
                                            resource_type, e
                                        );
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to read absolute icon path for {}: {:?}",
                                    resource_type, e
                                );
                                None
                            }
                        };
                    }
                }
            }
            return None;
        }

        // Try to load the icon from relative path
        match std::fs::read(icon_path) {
            Ok(image_bytes) => {
                log_debug!(
                    "Successfully read {} bytes from icon file: {}",
                    image_bytes.len(),
                    icon_path
                );
                match egui_extras::image::load_image_bytes(&image_bytes) {
                    Ok(image) => {
                        log_debug!(
                            "Successfully decoded image for resource type: {}, size: {:?}",
                            resource_type,
                            image.size
                        );
                        let texture = ctx.load_texture(
                            format!("resource_icon_{}", resource_type),
                            image,
                            Default::default(),
                        );
                        Some(texture)
                    }
                    Err(e) => {
                        error!(
                            "Failed to decode image for resource type: {}, error: {:?}",
                            resource_type, e
                        );
                        None
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read icon file: {}, error: {:?}", icon_path, e);
                None
            }
        }
    }

    pub fn show(
        &mut self,
        ctx: &Context,
        project: Option<&Project>,
        window_pos: Option<egui::Pos2>,
    ) -> (Option<CommandResult>, Option<egui::Rect>) {
        self.show_with_focus(ctx, project, window_pos, false)
    }

    pub fn show_with_focus(
        &mut self,
        ctx: &Context,
        project: Option<&Project>,
        window_pos: Option<egui::Pos2>,
        bring_to_front: bool,
    ) -> (Option<CommandResult>, Option<egui::Rect>) {
        if !self.show {
            return (None, None);
        }

        let mut template_updated = false;
        let mut open = self.show;
        let mut resource_command = None;
        let mut window_rect = None;

        // Calculate size based on screen, similar to chat and json editor windows
        let screen_rect = ctx.screen_rect();
        let window_height = screen_rect.height() * 0.8; // 80% of screen height
        let window_width = screen_rect.width() * 0.7; // 70% of screen width

        let mut window = Window::new("CloudFormation Template")
            .id(egui::Id::new("template_sections_window"))
            .open(&mut open)
            .resizable(true)
            .collapsible(true)
            .min_width(800.0)
            .min_height(600.0)
            .default_width(window_width.min(1200.0))
            .default_height(window_height.min(800.0))
            .max_height(screen_rect.height() * 0.9); // Prevent exceeding screen

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        // Apply position if provided
        if let Some(pos) = window_pos {
            window = window.current_pos(pos);
        }

        if let Some(response) = window.show(ctx, |ui| {
            // Set constraints to prevent unexpected growth
            ui.set_max_height(window_height);
            resource_command = self.render_content(ui, project);

            // Add save button for non-resources sections
            if self.selected_section != TemplateSection::Resources {
                ui.separator();
                if ui.button("Save Changes").clicked() {
                    template_updated = true;
                }
            }
        }) {
            window_rect = Some(response.response.rect);
        }

        // Update self.show with the result of the open flag
        self.show = open;

        let result = if let Some(command) = resource_command {
            Some(command)
        } else if template_updated {
            Some(CommandResult::TemplateUpdated(Box::new(
                self.template.clone(),
            )))
        } else {
            None
        };

        (result, window_rect)
    }

    fn render_content(&mut self, ui: &mut Ui, project: Option<&Project>) -> Option<CommandResult> {
        // Start widget registration for this frame with UI context for clipping
        self.widget_manager.start_frame_with_ui_context(
            ui,
            self.window_id().to_string(),
            self.window_title(),
        );

        // Clear stale actions (older than 5 seconds)
        self.widget_manager.clear_stale_actions(5000);

        // Section tabs with real widget registration
        ui.horizontal(|ui| {
            let sections = [
                (TemplateSection::Resources, "Resources"),
                (TemplateSection::Description, "Description"),
                (TemplateSection::Parameters, "Parameters"),
                (TemplateSection::Outputs, "Outputs"),
                (TemplateSection::Mappings, "Mappings"),
                (TemplateSection::Metadata, "Metadata"),
                (TemplateSection::Conditions, "Conditions"),
                (TemplateSection::Rules, "Rules"),
                (TemplateSection::Transform, "Transform"),
            ];

            for (section, label) in sections {
                let is_selected = self.selected_section == section;
                let button = ui.button(RichText::new(label).color(if is_selected {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::GRAY
                }));

                // Register the section tab button with navigation system
                let tab_id = format!("section_tab_{:?}", section);
                register_button!(self.widget_manager, button, tab_id, label);

                // Check for keyboard navigation actions and execute them
                let should_click =
                    button.clicked() || self.widget_manager.should_element_be_clicked(&tab_id);

                if should_click {
                    // Consume the pending action if it was triggered by keyboard
                    if self.widget_manager.should_element_be_clicked(&tab_id) {
                        let _consumed_actions =
                            self.widget_manager.consume_pending_actions(&tab_id);
                        tracing::info!(
                            "🎯 Keyboard navigation activated section tab: {:?}",
                            section
                        );
                    }

                    self.selected_section = section;
                    self.editing_key = None;
                    self.editing_value = String::new();
                }
            }
        });

        ui.separator();

        // Error display - positioned after tabs but before content
        if let Some(error) = &self.error_message {
            ui.colored_label(egui::Color32::RED, error);
            ui.separator();
        }

        // Section content
        let mut resource_command = None;

        match self.selected_section {
            TemplateSection::Resources => {
                resource_command = self.render_resources(ui, project);
            }
            _ => {
                // For all other sections, use the scroll area
                // Calculate available height for scroll area, reserving space for tabs and error message
                let available_height = ui.available_height() - 60.0; // Reserve ~60px for buttons/separator, same as resources tab

                // Constrain the content area height
                ui.set_min_height(available_height);
                ui.set_max_height(available_height);

                ScrollArea::vertical()
                    .auto_shrink([false, false]) // Prevent auto-shrinking
                    .max_height(available_height)
                    .id_salt("template_section_content")
                    .show(ui, |ui| match self.selected_section {
                        TemplateSection::Description => self.render_description(ui),
                        TemplateSection::Parameters => self.render_parameters(ui),
                        TemplateSection::Outputs => self.render_outputs(ui),
                        TemplateSection::Mappings => self.render_mappings(ui),
                        TemplateSection::Metadata => self.render_metadata(ui),
                        TemplateSection::Conditions => self.render_conditions(ui),
                        TemplateSection::Rules => self.render_rules(ui),
                        TemplateSection::Transform => self.render_transform(ui),
                        TemplateSection::Resources => {} // Already handled above
                    });
            }
        }

        resource_command
    }

    fn render_description(&mut self, ui: &mut Ui) {
        ui.add_space(28.0); // Add consistent top spacing

        let description = self.template.description.clone().unwrap_or_default();
        let mut edited_description = description.clone();

        ui.add(
            TextEdit::multiline(&mut edited_description)
                .desired_width(f32::INFINITY)
                .desired_rows(10),
        );

        if edited_description != description {
            self.template.description = if edited_description.is_empty() {
                None
            } else {
                Some(edited_description)
            };
        }
    }

    fn render_parameters(&mut self, ui: &mut Ui) {
        ui.add_space(28.0); // Add consistent top spacing

        ui.horizontal(|ui| {
            if ui.button("+ Add Parameter").clicked() {
                self.editing_key = Some("NewParameter".to_string());
                self.editing_value = String::new();
            }
        });

        // Track actions to perform after UI rendering
        let mut action = Action::None;

        Grid::new("parameters_grid")
            .num_columns(5)
            .striped(true)
            .spacing([5.0, 5.0])
            .show(ui, |ui| {
                // Headers
                ui.label(RichText::new("Name").strong());
                ui.label(RichText::new("Type").strong());
                ui.label(RichText::new("Default").strong());
                ui.label(RichText::new("Description").strong());
                ui.label(RichText::new("Actions").strong());
                ui.end_row();

                let param_keys: Vec<_> = self.template.parameters.keys().cloned().collect();
                for key in param_keys {
                    if let Some(param) = self.template.parameters.get(&key) {
                        ui.label(&key);
                        ui.label(&param.parameter_type);
                        ui.label(
                            param
                                .default
                                .as_ref()
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "None".to_string()),
                        );
                        ui.label(param.description.as_ref().unwrap_or(&"".to_string()));

                        ui.horizontal(|ui| {
                            if ui.button("Edit").clicked() {
                                action = Action::Edit(
                                    key.clone(),
                                    serde_json::to_string_pretty(param).unwrap_or_default(),
                                );
                            }
                            if ui.button("Delete").clicked() {
                                action = Action::Delete(key.clone());
                            }
                        });
                        ui.end_row();
                    }
                }
            });

        // Apply the action after the UI rendering
        match action {
            Action::Edit(key, value) => {
                self.editing_key = Some(key);
                self.editing_value = value;
            }
            Action::Delete(key) => {
                self.template.parameters.remove(&key);
            }
            Action::None => {}
        }

        // Edit/Add parameter dialog
        if let Some(key) = self.editing_key.clone() {
            ui.separator();
            ui.label(format!("Editing: {}", key));

            CodeEditor::default()
                .id_source("parameter_editor")
                .with_rows(10)
                .with_fontsize(14.0)
                .with_theme(self.get_code_theme(ui.ctx()))
                .with_syntax(cf_syntax::cloudformation_json_syntax())
                .show(ui, &mut self.editing_value);

            let mut save = false;
            let mut cancel = false;

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save = true;
                }

                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });

            if save {
                match serde_json::from_str::<Parameter>(&self.editing_value) {
                    Ok(param) => {
                        self.template.parameters.insert(key, param);
                        self.editing_key = None;
                        self.editing_value = String::new();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Invalid parameter JSON: {}", e));
                    }
                }
            }

            if cancel {
                self.editing_key = None;
                self.editing_value = String::new();
            }
        }
    }

    fn render_outputs(&mut self, ui: &mut Ui) {
        ui.add_space(28.0); // Add consistent top spacing

        ui.horizontal(|ui| {
            if ui.button("+ Add Output").clicked() {
                self.editing_key = Some("NewOutput".to_string());
                self.editing_value = String::new();
            }
        });

        // Track actions to perform after UI rendering
        let mut action = Action::None;

        Grid::new("outputs_grid")
            .num_columns(4)
            .striped(true)
            .spacing([5.0, 5.0])
            .show(ui, |ui| {
                // Headers
                ui.label(RichText::new("Name").strong());
                ui.label(RichText::new("Value").strong());
                ui.label(RichText::new("Description").strong());
                ui.label(RichText::new("Actions").strong());
                ui.end_row();

                let output_keys: Vec<_> = self.template.outputs.keys().cloned().collect();
                for key in output_keys {
                    if let Some(output) = self.template.outputs.get(&key) {
                        ui.label(&key);
                        ui.label(output.value.to_string());
                        ui.label(output.description.as_ref().unwrap_or(&"".to_string()));

                        ui.horizontal(|ui| {
                            if ui.button("Edit").clicked() {
                                action = Action::Edit(
                                    key.clone(),
                                    serde_json::to_string_pretty(output).unwrap_or_default(),
                                );
                            }
                            if ui.button("Delete").clicked() {
                                action = Action::Delete(key.clone());
                            }
                        });
                        ui.end_row();
                    }
                }
            });

        // Apply the action after the UI rendering
        match action {
            Action::Edit(key, value) => {
                self.editing_key = Some(key);
                self.editing_value = value;
            }
            Action::Delete(key) => {
                self.template.outputs.remove(&key);
            }
            Action::None => {}
        }

        // Edit/Add output dialog
        if let Some(key) = self.editing_key.clone() {
            ui.separator();
            ui.label(format!("Editing: {}", key));

            CodeEditor::default()
                .id_source("output_editor")
                .with_rows(10)
                .with_fontsize(14.0)
                .with_theme(self.get_code_theme(ui.ctx()))
                .with_syntax(cf_syntax::cloudformation_json_syntax())
                .show(ui, &mut self.editing_value);

            let mut save = false;
            let mut cancel = false;

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save = true;
                }

                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });

            if save {
                match serde_json::from_str::<Output>(&self.editing_value) {
                    Ok(output) => {
                        self.template.outputs.insert(key, output);
                        self.editing_key = None;
                        self.editing_value = String::new();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Invalid output JSON: {}", e));
                    }
                }
            }

            if cancel {
                self.editing_key = None;
                self.editing_value = String::new();
            }
        }
    }

    fn render_mappings(&mut self, ui: &mut Ui) {
        ui.add_space(28.0); // Add consistent top spacing

        ui.horizontal(|ui| {
            if ui.button("+ Add Mapping").clicked() {
                self.editing_key = Some("NewMapping".to_string());
                self.editing_value = "{}".to_string();
            }
        });

        // Track actions to perform after UI rendering
        let mut action = Action::None;

        let mapping_keys: Vec<_> = self.template.mappings.keys().cloned().collect();
        for key in mapping_keys {
            let mapping_json = self
                .template
                .mappings
                .get(&key)
                .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
                .unwrap_or_default();

            ui.collapsing(&key, |ui| {
                ui.label(&mapping_json);

                ui.horizontal(|ui| {
                    if ui.button("Edit").clicked() {
                        action = Action::Edit(key.clone(), mapping_json);
                    }
                    if ui.button("Delete").clicked() {
                        action = Action::Delete(key.clone());
                    }
                });
            });
        }

        // Apply the action
        match action {
            Action::Edit(key, value) => {
                self.editing_key = Some(key);
                self.editing_value = value;
            }
            Action::Delete(key) => {
                self.template.mappings.remove(&key);
            }
            Action::None => {}
        }

        // Edit/Add mapping dialog
        if let Some(key) = self.editing_key.clone() {
            ui.separator();
            ui.label(format!("Editing: {}", key));

            CodeEditor::default()
                .id_source("mapping_editor")
                .with_rows(10)
                .with_fontsize(14.0)
                .with_theme(self.get_code_theme(ui.ctx()))
                .with_syntax(cf_syntax::cloudformation_json_syntax())
                .show(ui, &mut self.editing_value);

            let mut save = false;
            let mut cancel = false;

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save = true;
                }

                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });

            if save {
                match serde_json::from_str::<Value>(&self.editing_value) {
                    Ok(value) => {
                        self.template.mappings.insert(key, value);
                        self.editing_key = None;
                        self.editing_value = String::new();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Invalid mapping JSON: {}", e));
                    }
                }
            }

            if cancel {
                self.editing_key = None;
                self.editing_value = String::new();
            }
        }
    }

    fn render_metadata(&mut self, ui: &mut Ui) {
        ui.add_space(28.0); // Add consistent top spacing

        ui.horizontal(|ui| {
            if ui.button("+ Add Metadata").clicked() {
                self.editing_key = Some("NewMetadata".to_string());
                self.editing_value = "{}".to_string();
            }
        });

        // Track actions to perform after UI rendering
        let mut action = Action::None;

        let metadata_keys: Vec<_> = self.template.metadata.keys().cloned().collect();
        for key in metadata_keys {
            let metadata_json = self
                .template
                .metadata
                .get(&key)
                .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
                .unwrap_or_default();

            ui.collapsing(&key, |ui| {
                ui.label(&metadata_json);

                ui.horizontal(|ui| {
                    if ui.button("Edit").clicked() {
                        action = Action::Edit(key.clone(), metadata_json);
                    }
                    if ui.button("Delete").clicked() {
                        action = Action::Delete(key.clone());
                    }
                });
            });
        }

        // Apply the action
        match action {
            Action::Edit(key, value) => {
                self.editing_key = Some(key);
                self.editing_value = value;
            }
            Action::Delete(key) => {
                self.template.metadata.remove(&key);
            }
            Action::None => {}
        }

        // Edit/Add metadata dialog
        if let Some(key) = self.editing_key.clone() {
            ui.separator();
            ui.label(format!("Editing: {}", key));

            CodeEditor::default()
                .id_source("metadata_editor")
                .with_rows(10)
                .with_fontsize(14.0)
                .with_theme(self.get_code_theme(ui.ctx()))
                .with_syntax(cf_syntax::cloudformation_json_syntax())
                .show(ui, &mut self.editing_value);

            let mut save = false;
            let mut cancel = false;

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save = true;
                }

                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });

            if save {
                match serde_json::from_str::<Value>(&self.editing_value) {
                    Ok(value) => {
                        self.template.metadata.insert(key, value);
                        self.editing_key = None;
                        self.editing_value = String::new();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Invalid metadata JSON: {}", e));
                    }
                }
            }

            if cancel {
                self.editing_key = None;
                self.editing_value = String::new();
            }
        }
    }

    fn render_conditions(&mut self, ui: &mut Ui) {
        ui.add_space(28.0); // Add consistent top spacing

        ui.horizontal(|ui| {
            if ui.button("+ Add Condition").clicked() {
                self.editing_key = Some("NewCondition".to_string());
                self.editing_value = "{}".to_string();
            }
        });

        // Track actions to perform after UI rendering
        let mut action = Action::None;

        let condition_keys: Vec<_> = self.template.conditions.keys().cloned().collect();
        for key in condition_keys {
            let condition_json = self
                .template
                .conditions
                .get(&key)
                .map(|c| serde_json::to_string_pretty(c).unwrap_or_default())
                .unwrap_or_default();

            ui.collapsing(&key, |ui| {
                ui.label(&condition_json);

                ui.horizontal(|ui| {
                    if ui.button("Edit").clicked() {
                        action = Action::Edit(key.clone(), condition_json);
                    }
                    if ui.button("Delete").clicked() {
                        action = Action::Delete(key.clone());
                    }
                });
            });
        }

        // Apply the action
        match action {
            Action::Edit(key, value) => {
                self.editing_key = Some(key);
                self.editing_value = value;
            }
            Action::Delete(key) => {
                self.template.conditions.remove(&key);
            }
            Action::None => {}
        }

        // Edit/Add condition dialog
        if let Some(key) = self.editing_key.clone() {
            ui.separator();
            ui.label(format!("Editing: {}", key));

            CodeEditor::default()
                .id_source("condition_editor")
                .with_rows(10)
                .with_fontsize(14.0)
                .with_theme(self.get_code_theme(ui.ctx()))
                .with_syntax(cf_syntax::cloudformation_json_syntax())
                .show(ui, &mut self.editing_value);

            let mut save = false;
            let mut cancel = false;

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save = true;
                }

                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });

            if save {
                match serde_json::from_str::<Value>(&self.editing_value) {
                    Ok(value) => {
                        self.template.conditions.insert(key, value);
                        self.editing_key = None;
                        self.editing_value = String::new();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Invalid condition JSON: {}", e));
                    }
                }
            }

            if cancel {
                self.editing_key = None;
                self.editing_value = String::new();
            }
        }
    }

    fn render_rules(&mut self, ui: &mut Ui) {
        ui.add_space(28.0); // Add consistent top spacing

        ui.horizontal(|ui| {
            if ui.button("+ Add Rule").clicked() {
                self.editing_key = Some("NewRule".to_string());
                self.editing_value = String::new();
            }
        });

        // Track actions to perform after UI rendering
        let mut action = Action::None;

        let rule_keys: Vec<_> = self.template.rules.keys().cloned().collect();
        for key in rule_keys {
            let rule_json = self
                .template
                .rules
                .get(&key)
                .map(|r| serde_json::to_string_pretty(r).unwrap_or_default())
                .unwrap_or_default();

            ui.collapsing(&key, |ui| {
                ui.label(&rule_json);

                ui.horizontal(|ui| {
                    if ui.button("Edit").clicked() {
                        action = Action::Edit(key.clone(), rule_json);
                    }
                    if ui.button("Delete").clicked() {
                        action = Action::Delete(key.clone());
                    }
                });
            });
        }

        // Apply the action
        match action {
            Action::Edit(key, value) => {
                self.editing_key = Some(key);
                self.editing_value = value;
            }
            Action::Delete(key) => {
                self.template.rules.remove(&key);
            }
            Action::None => {}
        }

        // Edit/Add rule dialog
        if let Some(key) = self.editing_key.clone() {
            ui.separator();
            ui.label(format!("Editing: {}", key));

            CodeEditor::default()
                .id_source("rule_editor")
                .with_rows(10)
                .with_fontsize(14.0)
                .with_theme(self.get_code_theme(ui.ctx()))
                .with_syntax(cf_syntax::cloudformation_json_syntax())
                .show(ui, &mut self.editing_value);

            let mut save = false;
            let mut cancel = false;

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    save = true;
                }

                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });

            if save {
                match serde_json::from_str::<Rule>(&self.editing_value) {
                    Ok(rule) => {
                        self.template.rules.insert(key, rule);
                        self.editing_key = None;
                        self.editing_value = String::new();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Invalid rule JSON: {}", e));
                    }
                }
            }

            if cancel {
                self.editing_key = None;
                self.editing_value = String::new();
            }
        }
    }

    fn render_transform(&mut self, ui: &mut Ui) {
        ui.add_space(28.0); // Add consistent top spacing

        // Track which transform indices to remove after the loop
        let mut indices_to_remove = Vec::new();

        if let Some(transforms) = &mut self.template.transform {
            for (i, transform) in transforms.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(transform);
                    if ui.button("Remove").clicked() {
                        indices_to_remove.push(i);
                    }
                });
            }

            // Remove transforms in reverse order to maintain correct indices
            for &i in indices_to_remove.iter().rev() {
                transforms.remove(i);
            }
        }

        ui.horizontal(|ui| {
            if ui.button("+ Add Transform").clicked() {
                let transforms = self.template.transform.get_or_insert_with(Vec::new);
                transforms.push("AWS::Serverless-2016-10-31".to_string());
            }
        });
    }

    fn render_resources(
        &mut self,
        ui: &mut Ui,
        project: Option<&Project>,
    ) -> Option<CommandResult> {
        let mut resource_command = None;

        // Show deletion confirmation dialog if active
        if let Some(resource_id) = &self.delete_confirmation.clone() {
            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ui.ctx(), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(format!("Delete Resource: {}", resource_id));
                        ui.add_space(10.0);
                        ui.label("Are you sure you want to delete this resource?");
                        ui.label("This action cannot be undone.");
                        ui.add_space(20.0);

                        ui.horizontal(|ui| {
                            if ui.button("Cancel").clicked() {
                                self.delete_confirmation = None;
                            }

                            ui.add_space(10.0);

                            if ui
                                .button(
                                    RichText::new("Delete")
                                        .color(egui::Color32::from_rgb(255, 0, 0)),
                                )
                                .clicked()
                            {
                                resource_command =
                                    Some(CommandResult::DeleteResource(resource_id.clone()));
                                self.delete_confirmation = None;
                            }
                        });
                    });
                });
        }

        // Show information based on project/resources state
        match project {
            None => {
                // No project case
                ui.add_space(40.0);
                ui.heading(
                    RichText::new("No Project Open")
                        .size(24.0)
                        .color(egui::Color32::from_rgb(220, 120, 120)),
                );
                ui.add_space(20.0);
                ui.label(RichText::new("Open a project to view resources").size(16.0));
                ui.add_space(10.0);

                if ui.button("Open Project").clicked() {
                    log_debug!("Open Project button clicked from template window");
                }
            }

            Some(project) => {
                // Get resources from the CloudFormation template directly
                let template_resources = project
                    .cfn_template
                    .as_ref()
                    .map(|template| &template.resources)
                    .filter(|resources| !resources.is_empty());

                if template_resources.is_none() || template_resources.unwrap().is_empty() {
                    // Project exists but no resources
                    ui.add_space(40.0);
                    ui.heading(
                        RichText::new("No Resources Loaded")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(220, 120, 120)),
                    );
                    ui.add_space(20.0);
                    ui.label(RichText::new("Use the Add Resource command to create new resources or import a CloudFormation template").size(9.0));
                    ui.add_space(10.0);

                    if ui.button("Add Resource").clicked() {
                        log_debug!("Add Resource button clicked from template window");
                    }
                } else {
                    // Get resources from the CloudFormation template
                    let cfn_resources = project
                        .cfn_template
                        .as_ref()
                        .map(|template| &template.resources)
                        .unwrap();

                    // Show filter UI - outside scroll area
                    ui.horizontal(|ui| {
                        ui.label("Filter:");
                        let filter_response = ui.text_edit_singleline(&mut self.filter_text);

                        // Register the filter text input
                        register_text_input!(
                            self.widget_manager,
                            filter_response,
                            "resource_filter",
                            "Filter resources"
                        );

                        ui.separator();

                        // Sort dropdown with widget registration
                        ui.label("Sort by:");
                        let sort_id_response = ui.radio_value(
                            &mut self.sort_by,
                            ResourceListSortOrder::Id,
                            "Resource ID",
                        );
                        register_radio_button!(
                            self.widget_manager,
                            sort_id_response,
                            "sort_by_id",
                            "Sort by Resource ID"
                        );

                        let sort_type_response = ui.radio_value(
                            &mut self.sort_by,
                            ResourceListSortOrder::Type,
                            "Resource Type",
                        );
                        register_radio_button!(
                            self.widget_manager,
                            sort_type_response,
                            "sort_by_type",
                            "Sort by Resource Type"
                        );
                    });

                    ui.separator();

                    // Check if we have any resources after filtering
                    let filtered_resources: Vec<(&String, &crate::app::cfn_template::Resource)> =
                        cfn_resources
                            .iter()
                            .filter(|(id, resource)| {
                                self.filter_text.is_empty()
                                    || fuzzy_match(&self.filter_text, id)
                                    || fuzzy_match(&self.filter_text, &resource.resource_type)
                            })
                            .collect();

                    // Calculate available height for the scroll area
                    let available_height = ui.available_height() - 60.0; // Leave room for status bar

                    if filtered_resources.is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.heading("No matching resources");
                            ui.label("Try a different filter");
                        });
                    } else {
                        // Sort resources based on the selected sort order
                        let mut sorted_resources = filtered_resources;
                        match self.sort_by {
                            ResourceListSortOrder::Id => {
                                sorted_resources.sort_by(|a, b| a.0.cmp(b.0));
                            }
                            ResourceListSortOrder::Type => {
                                sorted_resources
                                    .sort_by(|a, b| a.1.resource_type.cmp(&b.1.resource_type));
                            }
                        }

                        // Add some spacing before the resources table
                        ui.add_space(10.0);

                        // Pre-load icons for all visible resources (happens only once per resource type)
                        let ctx = ui.ctx().clone();
                        for (_, resource) in sorted_resources.iter() {
                            self.get_or_load_icon(&ctx, &resource.resource_type);
                        }

                        // Resources table with alternating background colors
                        ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .max_height(available_height)
                            .id_salt("resources_scroll_area")
                            .show(ui, |ui| {
                                for (i, (id, resource)) in sorted_resources.iter().enumerate() {
                                    // Create a frame with alternating background colors
                                    let is_odd = i % 2 == 1;
                                    let bg_color = if is_odd {
                                        ui.style().visuals.faint_bg_color // Darker shade for odd rows
                                    } else {
                                        ui.style().visuals.extreme_bg_color // Normal color for even rows
                                    };

                                    // Create a frame for the entire row
                                    let frame = egui::Frame::default()
                                        .fill(bg_color)
                                        .inner_margin(5.0) // Use a simple margin value
                                        .outer_margin(1.0); // Simple outer margin

                                    frame.show(ui, |ui| {
                                        // Use horizontal layout for the columns
                                        ui.horizontal(|ui| {
                                            // Column widths - first define the total space
                                            let avail_width = ui.available_width();

                                            // Resource ID column with icon (left aligned, clickable)
                                            ui.with_layout(
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    ui.set_width(avail_width * 0.4); // 40% width

                                                    // Display the icon (already loaded in cache)
                                                    if let Some(Some(texture)) =
                                                        self.icon_cache.get(&resource.resource_type)
                                                    {
                                                        let icon_size = 16.0;
                                                        ui.add(
                                                            egui::Image::from_texture(texture)
                                                                .max_size(egui::vec2(
                                                                    icon_size, icon_size,
                                                                )),
                                                        );
                                                        ui.add_space(4.0); // Small spacing between icon and text
                                                    }

                                                    let id_text = RichText::new((*id).clone())
                                                        .size(14.0)
                                                        .strong();
                                                    if ui
                                                        .add(
                                                            egui::Label::new(id_text)
                                                                .sense(egui::Sense::click()),
                                                        )
                                                        .clicked()
                                                    {
                                                        // Handle resource selection
                                                        log_debug!("Resource {} clicked", id);
                                                    }
                                                },
                                            );

                                            // Resource Type column
                                            ui.with_layout(
                                                egui::Layout::left_to_right(egui::Align::Center),
                                                |ui| {
                                                    ui.set_width(avail_width * 0.4); // 40% width
                                                    let type_text =
                                                        RichText::new(&resource.resource_type)
                                                            .weak();
                                                    ui.label(type_text);
                                                },
                                            );

                                            // Actions column
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.set_width(avail_width * 0.2); // 20% width

                                                    // Add some spacing between buttons
                                                    ui.spacing_mut().item_spacing.x = 5.0;

                                                    // Create a styled JSON button with document icon
                                                    let json_button = egui::Button::new(
                                                        RichText::new("</>").size(10.0),
                                                    )
                                                    .fill(egui::Color32::TRANSPARENT)
                                                    .stroke(egui::Stroke::new(
                                                        1.0,
                                                        ui.visuals().text_color(),
                                                    ))
                                                    .min_size(egui::vec2(30.0, 24.0))
                                                    .corner_radius(4.0);

                                                    let json_response = ui
                                                        .add(json_button)
                                                        .on_hover_text("View/Edit JSON");

                                                    // Register JSON button
                                                    let json_id = format!("json_resource_{}", id);
                                                    register_button!(self.widget_manager, json_response, json_id, "View/Edit JSON");

                                                    // Check for keyboard navigation actions and execute them
                                                    let should_click = json_response.clicked() || self.widget_manager.should_element_be_clicked(&json_id);

                                                    if should_click {
                                                        // Consume the pending action if it was triggered by keyboard
                                                        if self.widget_manager.should_element_be_clicked(&json_id) {
                                                            let _consumed_actions = self.widget_manager.consume_pending_actions(&json_id);
                                                            tracing::info!("🎯 Keyboard navigation activated JSON button for resource: {}", id);
                                                        }

                                                        // Handle JSON editing
                                                        log_debug!(
                                                            "JSON view for resource {} clicked",
                                                            id
                                                        );
                                                        resource_command =
                                                            Some(CommandResult::JsonEditResource(
                                                                (*id).clone(),
                                                            ));
                                                    }

                                                    // Create a styled Edit button
                                                    let accent_color =
                                                        ui.visuals().selection.bg_fill;
                                                    let button = egui::Button::new(
                                                        RichText::new("📄").strong(),
                                                    )
                                                    .fill(accent_color)
                                                    .stroke(egui::Stroke::new(
                                                        1.0,
                                                        ui.visuals().selection.stroke.color,
                                                    ))
                                                    .min_size(egui::vec2(24.0, 24.0))
                                                    .corner_radius(4.0);

                                                    let edit_response = ui.add(button);

                                                    // Register edit button
                                                    let edit_id = format!("edit_resource_{}", id);
                                                    register_button!(self.widget_manager, edit_response, edit_id, "Edit resource");

                                                    // Check for keyboard navigation actions and execute them
                                                    let should_click = edit_response.clicked() || self.widget_manager.should_element_be_clicked(&edit_id);

                                                    if should_click {
                                                        // Consume the pending action if it was triggered by keyboard
                                                        if self.widget_manager.should_element_be_clicked(&edit_id) {
                                                            let _consumed_actions = self.widget_manager.consume_pending_actions(&edit_id);
                                                            tracing::info!("🎯 Keyboard navigation activated edit button for resource: {}", id);
                                                        }

                                                        // Handle resource editing
                                                        log_debug!("Edit resource {} clicked", id);
                                                        resource_command =
                                                            Some(CommandResult::EditResource(
                                                                (*id).clone(),
                                                            ));
                                                    }
                                                },
                                            );
                                        });
                                    });
                                }
                            });

                        // Add a separator before the status bar
                        ui.separator();

                        // Show resource count in a status bar style
                        egui::Frame::default()
                            .fill(ui.style().visuals.extreme_bg_color)
                            .inner_margin(10.0) // Simple inner margin
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // Left side: resource count
                                    ui.add(egui::Label::new(
                                        RichText::new(format!(
                                            "Showing {} of {} resources",
                                            sorted_resources.len(),
                                            cfn_resources.len()
                                        ))
                                        .strong(),
                                    ));

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            // Add a refresh button on the right
                                            if ui.button("Refresh").clicked() {
                                                // This would trigger a refresh if implemented
                                                log_debug!(
                                                    "Refresh button clicked in resource list"
                                                );
                                            }
                                        },
                                    );
                                });
                            });
                    }
                }
            }
        }

        // Complete frame processing and update widget states
        self.widget_manager.complete_frame(ui.ctx());

        resource_command
    }
}

impl FocusableWindow for TemplateSectionsWindow {
    type ShowParams = ProjectShowParams;

    fn window_id(&self) -> &'static str {
        "template_sections"
    }

    fn window_title(&self) -> String {
        "CloudFormation Template".to_string()
    }

    fn is_open(&self) -> bool {
        self.show
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        self.show_with_focus(
            ctx,
            params.project.as_ref(),
            params.window_pos,
            bring_to_front,
        );
    }
}

impl TemplateSectionsWindow {
    /// Collect navigable elements from this template sections window
    /// This method returns real registered widgets from the NavigableWidgetManager
    pub fn collect_navigable_elements(&self) -> Vec<NavigableElement> {
        if !self.show {
            return Vec::new(); // Window is not visible, no elements to collect
        }

        // Get real elements from the widget manager
        let elements = self.widget_manager.collector().get_elements().to_vec();

        // R3.2 testing logs - only show if debug logging is enabled
        if self.widget_manager.is_debug_logging_enabled() {
            tracing::info!("🎯 R3.2 HINT TESTING - TemplateSectionsWindow::collect_navigable_elements - Collected {} REAL elements for window '{}' ({})",
                           elements.len(), self.window_title(), self.window_id());

            // Log summary of element types for debugging
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
                "🎯 R3.2 HINT TESTING - Real widget types breakdown: {:?}",
                type_counts
            );

            // Log a few example elements for verification
            for (i, element) in elements.iter().take(5).enumerate() {
                tracing::info!(
                    "🎯 R3.2 HINT TESTING - Element {}: id='{}' type={:?} rect={:?} enabled={}",
                    i + 1,
                    element.id,
                    element.element_type,
                    element.rect,
                    element.enabled
                );
            }

            // Test-specific logging for R3.2 validation
            if elements.len() >= 80 {
                tracing::info!("✅ R3.2 SUCCESS: Template window now captures {}+ real elements (exceeding target!)", elements.len());
            } else if !elements.is_empty() {
                tracing::info!("⚠️ R3.2 PARTIAL: Template window captures {} elements (below 80+ target, but real widgets detected)", elements.len());
            } else {
                tracing::warn!("❌ R3.2 ISSUE: No real elements captured - widget registration may not be working during rendering");
            }
        }

        elements
    }
}

impl NavigableWindow for TemplateSectionsWindow {
    fn get_navigation_context(&self) -> NavigationContext {
        let mut settings = HashMap::new();
        settings.insert("window_id".to_string(), self.window_id().to_string());
        settings.insert("window_title".to_string(), self.window_title());
        settings.insert("help_text".to_string(), "Template Sections Navigation:\n- j/k: Navigate sections\n- Tab: Navigate elements\n- Enter: Activate buttons\n- Ctrl+E: Edit selected item".to_string());

        NavigationContext {
            supports_hints: true,
            supports_visual_mode: true,
            handles_scrolling: true,
            settings,
        }
    }

    fn get_custom_key_bindings(&self) -> HashMap<String, NavigationCommand> {
        let mut bindings = HashMap::new();

        // Add template sections specific key bindings
        bindings.insert("ctrl+e".to_string(), NavigationCommand::ActivateElement); // Edit shortcut
        bindings.insert("ctrl+j".to_string(), NavigationCommand::ActivateElement); // JSON shortcut
        bindings.insert("delete".to_string(), NavigationCommand::ActivateElement); // Delete shortcut
        bindings.insert("escape".to_string(), NavigationCommand::CloseWindow); // Close window

        bindings
    }

    fn handle_navigation_command(&mut self, command: NavigationCommand) -> KeyEventResult {
        match command {
            NavigationCommand::ActivateElement => {
                // For now, just log the action - in a full implementation this would activate the selected element
                tracing::info!("Template sections window: Activate element command");
                KeyEventResult::Handled
            }
            NavigationCommand::CloseWindow => {
                // Close the template sections window
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
        tracing::debug!(
            "Template sections window navigation mode changed to: {:?}",
            new_mode
        );
    }
}
