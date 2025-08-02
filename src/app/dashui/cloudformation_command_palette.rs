use crate::app::cfn_template::CloudFormationTemplate;
use crate::app::dashui::cloudformation_file_picker::{
    CloudFormationFilePicker, CloudFormationFilePickerStatus,
};
use egui::{self, Color32, Context, RichText};
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum CloudFormationCommandAction {
    #[default]
    Closed,
    CommandPalette,
    AddResource,
    Deploy,
    Import,
    Validate,
    Validating, // New state for showing validation progress
    EditSections,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CloudFormationPaletteResult {
    None,
    AddResource,
    EditSections,
}

#[derive(Default)]
pub struct CloudFormationCommandPalette {
    pub mode: CloudFormationCommandAction,
    pub error_message: Option<String>,
    pub file_picker: Option<CloudFormationFilePicker>,
    pub imported_template: Option<CloudFormationTemplate>,
    pub import_path: Option<PathBuf>,
    pub last_imported_template: Option<CloudFormationTemplate>,
    pub last_import_path: Option<String>,
    pub validation_requested: bool,
    pub selected_environment: String,
    pub selected_account_id: String,
    pub selected_region: String,
    pub validation_in_progress: bool,
    pub validation_start_time: Option<std::time::Instant>,
    pub deploy_stack_name: String,
    pub deploy_requested: bool,
}

impl CloudFormationCommandPalette {
    pub fn new() -> Self {
        Self {
            mode: CloudFormationCommandAction::Closed,
            error_message: None,
            file_picker: None,
            imported_template: None,
            import_path: None,
            last_imported_template: None,
            last_import_path: None,
            validation_requested: false,
            selected_environment: String::new(),
            selected_account_id: String::new(),
            selected_region: String::from("us-east-1"), // Default region
            validation_in_progress: false,
            validation_start_time: None,
            deploy_stack_name: String::new(),
            deploy_requested: false,
        }
    }

    pub fn set_mode(&mut self, mode: CloudFormationCommandAction) {
        self.mode = mode;
        self.error_message = None;
    }

    pub fn complete_validation(&mut self, success: bool, message: String) {
        self.validation_in_progress = false;
        self.validation_start_time = None;
        if success {
            self.mode = CloudFormationCommandAction::Closed;
            // Success will be shown via notifications and validation results window
        } else {
            self.error_message = Some(message);
            self.mode = CloudFormationCommandAction::Validate; // Go back to validation dialog to show error
        }
    }

    pub fn show_command_palette_with_login_status(
        &mut self,
        ctx: &Context,
        is_logged_in: bool,
    ) -> Option<CloudFormationCommandAction> {
        let mut result = None;

        // Calculate dimensions similar to command palette
        let screen_rect = ctx.screen_rect();
        let window_height = screen_rect.height() * 0.25; // 1/4 of screen height
        let window_width = window_height * 2.33 * 0.8; // 20% less wide than original

        // Position at bottom of screen, centered horizontally (like command palette)
        let window_pos = egui::Pos2::new(
            screen_rect.center().x - (window_width / 2.0),
            screen_rect.max.y - window_height - 20.0, // 20px margin from bottom
        );

        // Calculate column properties
        let column_width = (window_width * 0.35).min(280.0); // 35% of width, max 280px
        let column_spacing = window_width * 0.1; // 10% of width for spacing
        let left_margin = (window_width - (2.0 * column_width + column_spacing)) / 2.0;

        // Create window with fixed position and size
        let window_size = egui::Vec2::new(window_width, window_height);

        egui::Area::new(egui::Id::new("cloudformation_command_palette"))
            .fixed_pos(window_pos)
            .movable(false)
            .show(ctx, |ui| {
                let frame = egui::Frame::NONE
                    .fill(ui.style().visuals.extreme_bg_color)
                    .stroke(egui::Stroke::new(
                        1.5,
                        ui.style().visuals.widgets.active.bg_fill,
                    ))
                    .inner_margin(egui::Margin {
                        left: 25,
                        right: 25,
                        top: 20,
                        bottom: 20,
                    })
                    .corner_radius(8.0);

                frame.show(ui, |ui| {
                    ui.set_min_size(window_size);

                    // Add a small top space
                    ui.add_space(10.0);

                    if let Some(error) = &self.error_message {
                        ui.vertical_centered(|ui| {
                            ui.colored_label(Color32::from_rgb(220, 50, 50), error);
                        });
                        ui.add_space(10.0);
                    }

                    // Two column layout with calculated positions
                    ui.horizontal(|ui| {
                        // Add left margin
                        ui.add_space(left_margin);

                        // Define command entries with login status awareness
                        let commands = if is_logged_in {
                            [
                                (
                                    egui::Key::A,
                                    'A',
                                    "Add Resource",
                                    Color32::from_rgb(40, 140, 60), // Green
                                    "Add a new resource",
                                    CloudFormationCommandAction::AddResource,
                                ),
                                (
                                    egui::Key::D,
                                    'D',
                                    "Deploy",
                                    Color32::from_rgb(40, 140, 60), // Green
                                    "Deploy a CloudFormation Stack",
                                    CloudFormationCommandAction::Deploy,
                                ),
                                (
                                    egui::Key::I,
                                    'I',
                                    "Import",
                                    Color32::from_rgb(100, 170, 255), // Blue
                                    "Import an existing CloudFormation Stack",
                                    CloudFormationCommandAction::Import,
                                ),
                                (
                                    egui::Key::V,
                                    'V',
                                    "Validate",
                                    Color32::from_rgb(255, 190, 70), // Orange/Yellow
                                    "Validate a CloudFormation Template",
                                    CloudFormationCommandAction::Validate,
                                ),
                                (
                                    egui::Key::E,
                                    'E',
                                    "Edit Sections",
                                    Color32::from_rgb(140, 200, 170), // Teal
                                    "Edit Template Parameters, Outputs, Mappings, etc.",
                                    CloudFormationCommandAction::EditSections,
                                ),
                            ]
                        } else {
                            [
                                (
                                    egui::Key::A,
                                    'A',
                                    "Add Resource",
                                    Color32::from_rgb(40, 140, 60), // Green - works without login
                                    "Add a new resource",
                                    CloudFormationCommandAction::AddResource,
                                ),
                                (
                                    egui::Key::D,
                                    'D',
                                    "Deploy",
                                    Color32::from_rgb(120, 120, 120), // Grayed out
                                    "⚠️ Please login to AWS Identity Center first",
                                    CloudFormationCommandAction::Deploy,
                                ),
                                (
                                    egui::Key::I,
                                    'I',
                                    "Import",
                                    Color32::from_rgb(100, 170, 255), // Blue - works without login
                                    "Import an existing CloudFormation Stack",
                                    CloudFormationCommandAction::Import,
                                ),
                                (
                                    egui::Key::V,
                                    'V',
                                    "Validate",
                                    Color32::from_rgb(120, 120, 120), // Grayed out
                                    "⚠️ Please login to AWS Identity Center first",
                                    CloudFormationCommandAction::Validate,
                                ),
                                (
                                    egui::Key::E,
                                    'E',
                                    "Edit Sections",
                                    Color32::from_rgb(140, 200, 170), // Teal - works without login
                                    "Edit Template Parameters, Outputs, Mappings, etc.",
                                    CloudFormationCommandAction::EditSections,
                                ),
                            ]
                        };

                        // Function to draw a command button
                        let mut draw_command =
                            |ui: &mut egui::Ui,
                             cmd: &(
                                egui::Key,
                                char,
                                &str,
                                Color32,
                                &str,
                                CloudFormationCommandAction,
                            ),
                             _idx: usize| {
                                let mut clicked = false;
                                let key_pressed = ctx.input(|input| input.key_pressed(cmd.0));

                                // Draw styled command button
                                ui.horizontal(|ui| {
                                    // Key in circle with color
                                    let circle_size = egui::Vec2::new(32.0, 32.0);
                                    let (rect, response) =
                                        ui.allocate_exact_size(circle_size, egui::Sense::click());

                                    if ui.is_rect_visible(rect) {
                                        let visuals = ui.style().interact(&response);
                                        let circle_stroke =
                                            egui::Stroke::new(1.5, visuals.fg_stroke.color);

                                        // Draw colored circle
                                        ui.painter().circle(
                                            rect.center(),
                                            rect.width() / 2.0,
                                            cmd.3.linear_multiply(0.8), // Slightly darker
                                            circle_stroke,
                                        );

                                        // Draw key character centered in circle
                                        let text = cmd.1.to_string();

                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            text,
                                            egui::FontId::proportional(16.0),
                                            egui::Color32::WHITE,
                                        );
                                    }

                                    ui.add_space(8.0);

                                    // Command information with arrow
                                    ui.vertical(|ui| {
                                        // Label with color
                                        ui.label(
                                            RichText::new(cmd.2).size(16.0).color(cmd.3).strong(),
                                        );
                                        // Description in smaller text
                                        ui.label(RichText::new(cmd.4).size(13.0).weak());
                                    });

                                    if response.clicked() || key_pressed {
                                        clicked = true;
                                    }
                                });

                                if clicked || key_pressed {
                                    // Check if action requires login and user is not logged in
                                    match cmd.5 {
                                        CloudFormationCommandAction::Deploy | CloudFormationCommandAction::Validate => {
                                            if is_logged_in {
                                                result = Some(cmd.5);
                                            } else {
                                                // Show error message instead of proceeding
                                                self.error_message = Some("Please login to AWS Identity Center first before deploying or validating CloudFormation templates.".to_string());
                                            }
                                        }
                                        _ => {
                                            // AddResource, Import and Edit Sections work without login
                                            result = Some(cmd.5);
                                        }
                                    }
                                }

                                ui.add_space(20.0);
                            };

                        // Split commands for two columns
                        #[allow(clippy::manual_div_ceil)]
                        let first_column_count = (commands.len() + 1) / 2;

                        // First column
                        ui.vertical(|ui| {
                            ui.set_width(column_width);

                            for (i, cmd) in commands.iter().enumerate().take(first_column_count) {
                                draw_command(ui, cmd, i);
                            }
                        });

                        // Add calculated spacing between columns
                        ui.add_space(column_spacing);

                        // Second column
                        ui.vertical(|ui| {
                            ui.set_width(column_width);

                            for (i, cmd) in commands.iter().enumerate().skip(first_column_count) {
                                draw_command(ui, cmd, i);
                            }
                        });
                    });
                });
            });

        // Close palette if clicking outside
        if ctx.input(|i| i.pointer.any_click()) {
            let mouse_pos = ctx.input(|i| i.pointer.interact_pos());
            if let Some(pos) = mouse_pos {
                let rect = egui::Rect::from_min_size(window_pos, window_size);
                if !rect.contains(pos) {
                    self.mode = CloudFormationCommandAction::Closed;
                }
            }
        }

        // Close on Escape key
        if self.mode == CloudFormationCommandAction::CommandPalette
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.mode = CloudFormationCommandAction::Closed;
        }

        result
    }

    pub fn show(
        &mut self,
        ctx: &Context,
        project_palette: Option<&super::project_command_palette::ProjectCommandPalette>,
        is_logged_in: bool,
    ) -> CloudFormationPaletteResult {
        let _action_performed = false;

        match self.mode {
            CloudFormationCommandAction::Closed => {}
            CloudFormationCommandAction::CommandPalette => {
                if let Some(new_mode) =
                    self.show_command_palette_with_login_status(ctx, is_logged_in)
                {
                    self.set_mode(new_mode);
                }
            }
            CloudFormationCommandAction::AddResource => {
                // Return AddResource to signal that we want to add a resource
                self.mode = CloudFormationCommandAction::Closed;
                return CloudFormationPaletteResult::AddResource;
            }
            CloudFormationCommandAction::Deploy => {
                // Show Deploy Stack dialog with stack name input
                egui::Window::new("Deploy CloudFormation Stack")
                    .collapsible(false)
                    .resizable(false)
                    .min_width(500.0)
                    .show(ctx, |ui| {
                        // Check if we have a project with template loaded
                        let has_project_template = if let Some(project) = project_palette.as_ref()
                            .and_then(|p| p.current_project.as_ref()) {
                            project.cfn_template.is_some()
                        } else {
                            false
                        };

                        if has_project_template {
                            ui.label("Deploy CloudFormation Stack");
                            ui.add_space(5.0);

                            if let Some(project) = project_palette.as_ref()
                                .and_then(|p| p.current_project.as_ref()) {
                                ui.label(format!("Project: {}", project.name));
                                if let Some(folder) = &project.local_folder {
                                    ui.label(format!("Template: {}/Resources/cloudformation_template.json", folder.display()));
                                }
                            }

                            ui.separator();
                            ui.add_space(10.0);

                            // Get project configuration for deployment
                            let project = project_palette.as_ref()
                                .and_then(|p| p.current_project.as_ref()).unwrap();

                            let project_environments = &project.environments;
                            let has_environments = !project_environments.is_empty();

                            // Set default environment if not already set
                            if self.selected_environment.is_empty() && has_environments {
                                self.selected_environment = project_environments[0].name.clone();
                            }

                            // Get selected environment configuration
                            let selected_env = if !self.selected_environment.is_empty() {
                                project_environments.iter().find(|env| env.name == self.selected_environment)
                            } else {
                                None
                            };

                            // Update account/region based on selected environment
                            if let Some(env) = selected_env {
                                if self.selected_account_id.is_empty() && !env.aws_accounts.is_empty() {
                                    self.selected_account_id = env.aws_accounts[0].0.clone();
                                }
                                if self.selected_region.is_empty() && !env.aws_regions.is_empty() {
                                    self.selected_region = env.aws_regions[0].0.clone();
                                }
                            }

                            // Show project configuration status
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Deployment Configuration").strong());

                                ui.horizontal(|ui| {
                                    if has_environments {
                                        ui.colored_label(egui::Color32::from_rgb(40, 180, 40), "✓");
                                        ui.label(format!("Environments: {}", project_environments.len()));
                                    } else {
                                        ui.colored_label(egui::Color32::from_rgb(220, 50, 50), "✗");
                                        ui.label("No environments configured");
                                    }
                                });

                                if let Some(env) = selected_env {
                                    ui.horizontal(|ui| {
                                        if !env.aws_accounts.is_empty() {
                                            ui.colored_label(egui::Color32::from_rgb(40, 180, 40), "✓");
                                            ui.label(format!("Accounts in {}: {}", env.name, env.aws_accounts.len()));
                                        } else {
                                            ui.colored_label(egui::Color32::from_rgb(220, 50, 50), "✗");
                                            ui.label(format!("No accounts in {} environment", env.name));
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        if !env.aws_regions.is_empty() {
                                            ui.colored_label(egui::Color32::from_rgb(40, 180, 40), "✓");
                                            ui.label(format!("Regions in {}: {}", env.name, env.aws_regions.len()));
                                        } else {
                                            ui.colored_label(egui::Color32::from_rgb(220, 50, 50), "✗");
                                            ui.label(format!("No regions in {} environment", env.name));
                                        }
                                    });
                                }
                            });

                            ui.add_space(10.0);

                            if has_environments && selected_env.is_some() &&
                               !selected_env.unwrap().aws_accounts.is_empty() &&
                               !selected_env.unwrap().aws_regions.is_empty() {

                                let env = selected_env.unwrap();

                                // Environment selection
                                ui.horizontal(|ui| {
                                    ui.label("Environment:");
                                    egui::ComboBox::from_id_salt("cfn_deploy_environment")
                                        .selected_text(&self.selected_environment)
                                        .show_ui(ui, |ui| {
                                            for environment in project_environments {
                                                if ui.selectable_value(&mut self.selected_environment, environment.name.clone(), &environment.name).clicked() {
                                                    // Reset account/region when environment changes
                                                    self.selected_account_id.clear();
                                                    self.selected_region.clear();
                                                }
                                            }
                                        });
                                });

                                ui.add_space(5.0);
                                // Account ID selection (from selected environment)
                                ui.horizontal(|ui| {
                                    ui.label("Account ID:");
                                    egui::ComboBox::from_id_salt("cfn_deploy_account_id")
                                        .selected_text(&self.selected_account_id)
                                        .show_ui(ui, |ui| {
                                            for account in &env.aws_accounts {
                                                ui.selectable_value(&mut self.selected_account_id, account.0.clone(), &account.0);
                                            }
                                        });
                                });

                                ui.add_space(5.0);

                                // Region selection (from selected environment)
                                ui.horizontal(|ui| {
                                    ui.label("Region:");
                                    egui::ComboBox::from_id_salt("cfn_deploy_region")
                                        .selected_text(&self.selected_region)
                                        .show_ui(ui, |ui| {
                                            for region in &env.aws_regions {
                                                ui.selectable_value(&mut self.selected_region, region.0.clone(), &region.0);
                                            }
                                        });
                                });

                                ui.add_space(10.0);

                                // Stack name input
                                ui.horizontal(|ui| {
                                    ui.label("Stack Name:");
                                    ui.text_edit_singleline(&mut self.deploy_stack_name);
                                });

                                // Generate default stack name if empty
                                if self.deploy_stack_name.is_empty() {
                                    self.deploy_stack_name = format!("{}-stack", project.name.to_lowercase().replace(" ", "-"));
                                }

                                ui.add_space(10.0);
                            } else {
                                ui.colored_label(
                                    egui::Color32::from_rgb(220, 150, 0),
                                    "⚠ Environment configuration incomplete"
                                );
                                if !has_environments {
                                    ui.label("No environments configured in project.");
                                } else if selected_env.is_none() {
                                    ui.label("Please select a valid environment.");
                                } else {
                                    let env = selected_env.unwrap();
                                    if env.aws_accounts.is_empty() {
                                        ui.label(format!("Environment '{}' has no AWS accounts configured.", env.name));
                                    }
                                    if env.aws_regions.is_empty() {
                                        ui.label(format!("Environment '{}' has no AWS regions configured.", env.name));
                                    }
                                }
                                ui.label("Please configure environments with account IDs and regions in your project.");
                                ui.add_space(10.0);
                            }

                            ui.horizontal(|ui| {
                                let can_deploy = selected_env.is_some() &&
                                    !selected_env.unwrap().aws_accounts.is_empty() &&
                                    !selected_env.unwrap().aws_regions.is_empty() &&
                                    !self.selected_account_id.trim().is_empty() &&
                                    !self.selected_region.trim().is_empty() &&
                                    !self.deploy_stack_name.trim().is_empty();

                                if ui.add_enabled(can_deploy, egui::Button::new("Deploy Stack")).clicked() {
                                    self.deploy_requested = true;
                                    self.mode = CloudFormationCommandAction::Closed; // Close the deploy dialog
                                }

                                if ui.button("Cancel").clicked() {
                                    self.mode = CloudFormationCommandAction::Closed; // Close the deploy dialog
                                }
                            });
                        } else {
                            // Check if we have a project but no template, or no project at all
                            let has_project = project_palette.as_ref()
                                .and_then(|p| p.current_project.as_ref())
                                .is_some();

                            if has_project {
                                ui.colored_label(egui::Color32::from_rgb(220, 150, 0), "No CloudFormation template in project");
                                ui.add_space(5.0);
                                ui.label("The current project doesn't have a CloudFormation template loaded.");
                                ui.label("Please open the project and import/create a template first.");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(220, 50, 50), "No project open");
                                ui.add_space(5.0);
                                ui.label("Please open a project first to deploy CloudFormation templates.");
                                ui.label("Projects provide the account ID and region configuration needed for deployment.");
                            }

                            ui.add_space(10.0);

                            if ui.button("Close").clicked() {
                                self.mode = CloudFormationCommandAction::Closed; // Close the deploy dialog
                            }
                        }
                    });

                // Check for escape key to close deploy dialog
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.mode = CloudFormationCommandAction::Closed;
                }
            }
            CloudFormationCommandAction::Import => {
                // Initialize file picker if not already done
                if self.file_picker.is_none() {
                    self.file_picker = Some(CloudFormationFilePicker::new());
                }

                // Show the file picker
                let mut picker_action = None;
                let mut picker_path = None;

                if let Some(file_picker) = &mut self.file_picker {
                    file_picker.show(ctx);

                    // Check status without borrowing self again
                    match &file_picker.status {
                        CloudFormationFilePickerStatus::Selected(path) => {
                            picker_action = Some(true); // true = selected a path
                            picker_path = Some(path.clone());
                        }
                        CloudFormationFilePickerStatus::Closed => {
                            picker_action = Some(false); // false = closed without selecting
                        }
                        CloudFormationFilePickerStatus::Open => {
                            // Picker is still open, do nothing
                        }
                    }
                }

                // Process the picker result if needed
                if let Some(is_selected) = picker_action {
                    // Close the picker first
                    self.file_picker = None;

                    if is_selected {
                        // User selected a file, try to import the template
                        if let Some(path) = picker_path {
                            info!("Importing CloudFormation template from: {}", path.display());
                            match self.import_template(&path) {
                                Ok(template) => {
                                    // Store for verification
                                    self.last_imported_template = Some(template.clone());
                                    self.last_import_path = Some(path.display().to_string());

                                    self.imported_template = Some(template);
                                    self.import_path = Some(path);
                                    info!("Successfully imported CloudFormation template");
                                    self.mode = CloudFormationCommandAction::Closed;
                                    // No specific action result needed for import
                                    return CloudFormationPaletteResult::None;
                                }
                                Err(e) => {
                                    error!("Error importing template: {}", e);
                                    self.error_message =
                                        Some(format!("Error importing template: {}", e));
                                    self.mode = CloudFormationCommandAction::CommandPalette;
                                }
                            }
                        }
                    } else {
                        // User cancelled, go back to command palette
                        self.mode = CloudFormationCommandAction::CommandPalette;
                    }
                }
            }
            CloudFormationCommandAction::Validate => {
                // Show validation dialog with account/region selection
                egui::Window::new("Validate CloudFormation Template")
                    .collapsible(false)
                    .resizable(false)
                    .min_width(500.0)
                    .show(ctx, |ui| {
                        // Check if we have a project with template loaded
                        let has_project_template = if let Some(project) = project_palette.as_ref()
                            .and_then(|p| p.current_project.as_ref()) {
                            project.cfn_template.is_some()
                        } else {
                            false
                        };

                        if has_project_template {
                            ui.label("Project template loaded and ready for validation");
                            ui.add_space(5.0);

                            if let Some(project) = project_palette.as_ref()
                                .and_then(|p| p.current_project.as_ref()) {
                                ui.label(format!("Project: {}", project.name));
                                if let Some(folder) = &project.local_folder {
                                    ui.label(format!("Location: {}/Resources/cloudformation_template.json", folder.display()));
                                }
                            }

                            ui.separator();
                            ui.add_space(10.0);

                            // Get project configuration for validation
                            let project = project_palette.as_ref()
                                .and_then(|p| p.current_project.as_ref()).unwrap();

                            let project_accounts = project.get_all_accounts();
                            let project_regions = project.get_all_regions();
                            let has_accounts = !project_accounts.is_empty();
                            let has_regions = !project_regions.is_empty();

                            // Set defaults if not already set
                            if self.selected_account_id.is_empty() && has_accounts {
                                self.selected_account_id = project_accounts[0].0.clone();
                            }
                            if (self.selected_region.is_empty() || self.selected_region == "us-east-1") && has_regions {
                                self.selected_region = project_regions[0].0.clone();
                            }

                            // Show project configuration status
                            ui.group(|ui| {
                                ui.label(egui::RichText::new("Project Configuration").strong());

                                ui.horizontal(|ui| {
                                    if has_accounts {
                                        ui.colored_label(egui::Color32::from_rgb(40, 180, 40), "✓");
                                        ui.label(format!("Account IDs: {}", project_accounts.len()));
                                    } else {
                                        ui.colored_label(egui::Color32::from_rgb(220, 50, 50), "✗");
                                        ui.label("No account IDs configured");
                                    }
                                });

                                ui.horizontal(|ui| {
                                    if has_regions {
                                        ui.colored_label(egui::Color32::from_rgb(40, 180, 40), "✓");
                                        ui.label(format!("Regions: {}", project_regions.len()));
                                    } else {
                                        ui.colored_label(egui::Color32::from_rgb(220, 50, 50), "✗");
                                        ui.label("No regions configured");
                                    }
                                });
                            });

                            ui.add_space(10.0);

                            if has_accounts && has_regions {
                                // Account ID selection
                                ui.horizontal(|ui| {
                                    ui.label("Account ID:");
                                    egui::ComboBox::from_id_salt("cfn_validation_account_id")
                                        .selected_text(&self.selected_account_id)
                                        .show_ui(ui, |ui| {
                                            for account in &project_accounts {
                                                ui.selectable_value(&mut self.selected_account_id, account.0.clone(), &account.0);
                                            }
                                        });
                                });

                                ui.add_space(5.0);

                                // Region selection
                                ui.horizontal(|ui| {
                                    ui.label("Region:");
                                    egui::ComboBox::from_id_salt("cfn_validation_region")
                                        .selected_text(&self.selected_region)
                                        .show_ui(ui, |ui| {
                                            for region in &project_regions {
                                                ui.selectable_value(&mut self.selected_region, region.0.clone(), &region.0);
                                            }
                                        });
                                });

                                ui.add_space(10.0);
                            } else {
                                ui.colored_label(
                                    egui::Color32::from_rgb(220, 150, 0),
                                    "⚠ Project configuration incomplete"
                                );
                                ui.label("Please configure account IDs and regions in your project before validation.");
                                ui.add_space(10.0);
                            }

                            ui.horizontal(|ui| {
                                let can_validate = has_accounts && has_regions &&
                                    !self.selected_account_id.trim().is_empty() &&
                                    !self.selected_region.trim().is_empty();

                                if ui.add_enabled(can_validate, egui::Button::new("Validate")).clicked() {
                                    self.validation_requested = true;
                                    self.validation_in_progress = true;
                                    self.validation_start_time = Some(std::time::Instant::now());
                                    self.mode = CloudFormationCommandAction::Validating; // Switch to validating state
                                }

                                if ui.button("Cancel").clicked() {
                                    self.mode = CloudFormationCommandAction::Closed; // Close the validation window
                                }
                            });
                        } else {
                            // Check if we have a project but no template, or no project at all
                            let has_project = project_palette.as_ref()
                                .and_then(|p| p.current_project.as_ref())
                                .is_some();

                            if has_project {
                                ui.colored_label(egui::Color32::from_rgb(220, 150, 0), "No CloudFormation template in project");
                                ui.add_space(5.0);
                                ui.label("The current project doesn't have a CloudFormation template loaded.");
                                ui.label("Please open the project and import/create a template first.");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(220, 50, 50), "No project open");
                                ui.add_space(5.0);
                                ui.label("Please open a project first to validate CloudFormation templates.");
                                ui.label("Projects provide the account ID and region configuration needed for validation.");
                            }

                            ui.add_space(10.0);

                            if ui.button("Close").clicked() {
                                self.mode = CloudFormationCommandAction::Closed; // Close the validation window
                            }
                        }
                    });

                // Check for escape key to close validation window
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.mode = CloudFormationCommandAction::Closed;
                }
            }
            CloudFormationCommandAction::Validating => {
                // Show validation progress window
                egui::Window::new("Validating CloudFormation Template")
                    .collapsible(false)
                    .resizable(false)
                    .min_width(500.0)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Validating CloudFormation template...");
                        });

                        if let Some(start_time) = self.validation_start_time {
                            let elapsed = start_time.elapsed();
                            ui.label(format!("Elapsed time: {:.1}s", elapsed.as_secs_f32()));
                        }

                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.label("Account ID:");
                            ui.strong(&self.selected_account_id);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Region:");
                            ui.strong(&self.selected_region);
                        });

                        ui.add_space(10.0);

                        if ui.button("Cancel Validation").clicked() {
                            self.validation_in_progress = false;
                            self.validation_start_time = None;
                            self.mode = CloudFormationCommandAction::Closed;
                        }
                    });

                // Check for escape key to cancel validation
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.validation_in_progress = false;
                    self.validation_start_time = None;
                    self.mode = CloudFormationCommandAction::Closed;
                }
            }
            CloudFormationCommandAction::EditSections => {
                // Return EditSections to signal that we want to open the template sections window
                self.mode = CloudFormationCommandAction::CommandPalette;
                return CloudFormationPaletteResult::EditSections;
            }
        }

        CloudFormationPaletteResult::None
    }

    /// Import a CloudFormation template from a file
    fn import_template(&self, path: &PathBuf) -> Result<CloudFormationTemplate, String> {
        // Read the file content
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        // Determine the file type and parse accordingly
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("json") => {
                // Parse as JSON
                let template: CloudFormationTemplate = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse JSON: {}", e))?;
                Ok(template)
            }
            Some("yaml") | Some("yml") => {
                // Parse as YAML
                let template: CloudFormationTemplate = serde_yaml::from_str(&content)
                    .map_err(|e| format!("Failed to parse YAML: {}", e))?;
                Ok(template)
            }
            _ => {
                // Try to detect format from content
                if content.trim_start().starts_with("{") {
                    // Likely JSON
                    let template: CloudFormationTemplate = serde_json::from_str(&content)
                        .map_err(|e| format!("Failed to parse as JSON: {}", e))?;
                    Ok(template)
                } else {
                    // Try YAML
                    let template: CloudFormationTemplate = serde_yaml::from_str(&content)
                        .map_err(|e| format!("Failed to parse as YAML: {}", e))?;
                    Ok(template)
                }
            }
        }
    }
}
