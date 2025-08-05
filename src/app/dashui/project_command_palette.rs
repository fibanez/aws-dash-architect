use crate::app::cfn_resources::AWS_REGIONS;
use crate::app::dashui::fuzzy_file_picker::{FuzzyFilePicker, FuzzyFilePickerStatus};
use crate::app::projects::{AwsAccount, AwsRegion, Environment, Project};
use egui::{self, Align, Color32, Context, Grid, Layout, RichText, Window};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum ProjectCommandAction {
    #[default]
    Closed,
    CommandPalette, // Renamed from ProjectManagement to be more consistent
    NewProject,
    OpenProject, // Added to replace the one from main command palette
    EditProject,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentForm {
    pub name: String,
    pub aws_regions: String,
    pub aws_accounts: String,
    #[serde(skip)]
    pub selected_regions: Vec<String>,
    #[serde(skip)]
    pub selected_accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectForm {
    pub name: String,
    pub description: String,
    pub short_name: String,
    pub local_folder: Option<PathBuf>,
    pub git_url: Option<String>,
    pub environments: Vec<EnvironmentForm>,

    // CloudFormation Guard compliance settings
    #[serde(default)]
    pub guard_rules_enabled: bool,
    #[serde(default)]
    pub compliance_programs: Vec<crate::app::cfn_guard::ComplianceProgram>,
    #[serde(default)]
    pub custom_guard_rules: Vec<String>,
}

impl Default for ProjectForm {
    fn default() -> Self {
        let default_environments = vec![
            EnvironmentForm {
                name: "Dev".to_string(),
                aws_regions: String::new(),
                aws_accounts: String::new(),
                selected_regions: Vec::new(),
                selected_accounts: Vec::new(),
            },
            EnvironmentForm {
                name: "Prod".to_string(),
                aws_regions: String::new(),
                aws_accounts: String::new(),
                selected_regions: Vec::new(),
                selected_accounts: Vec::new(),
            },
        ];

        Self {
            name: String::new(),
            description: String::new(),
            short_name: String::new(),
            local_folder: None,
            git_url: None,
            environments: default_environments,
            guard_rules_enabled: true,
            compliance_programs: Vec::new(),
            custom_guard_rules: Vec::new(),
        }
    }
}

impl ProjectForm {
    pub fn from_project(project: &Project) -> Self {
        let mut environment_forms = Vec::new();

        // Convert Project environments to EnvironmentForm
        for env in &project.environments {
            let regions_vec: Vec<String> = env.aws_regions.iter().map(|r| r.0.clone()).collect();
            let accounts_vec: Vec<String> = env.aws_accounts.iter().map(|a| a.0.clone()).collect();

            let env_form = EnvironmentForm {
                name: env.name.clone(),
                aws_regions: regions_vec.join(", "),
                aws_accounts: accounts_vec.join(", "),
                selected_regions: regions_vec,
                selected_accounts: accounts_vec,
            };
            environment_forms.push(env_form);
        }

        // If no environments exist (for backward compatibility), create default ones
        if environment_forms.is_empty() {
            // Create Dev environment from existing data
            let regions_vec: Vec<String> = project
                .get_all_regions()
                .iter()
                .map(|r| r.0.clone())
                .collect();
            let accounts_vec: Vec<String> = project
                .get_all_accounts()
                .iter()
                .map(|a| a.0.clone())
                .collect();

            let dev_env = EnvironmentForm {
                name: "Dev".to_string(),
                aws_regions: regions_vec.join(", "),
                aws_accounts: accounts_vec.join(", "),
                selected_regions: regions_vec,
                selected_accounts: accounts_vec,
            };

            // Add empty Prod environment
            let prod_env = EnvironmentForm {
                name: "Prod".to_string(),
                aws_regions: String::new(),
                aws_accounts: String::new(),
                selected_regions: Vec::new(),
                selected_accounts: Vec::new(),
            };

            environment_forms.push(dev_env);
            environment_forms.push(prod_env);
        }

        Self {
            name: project.name.clone(),
            description: project.description.clone(),
            short_name: project.short_name.clone(),
            local_folder: project.local_folder.clone(),
            git_url: project.git_url.clone(),
            environments: environment_forms,
            guard_rules_enabled: project.guard_rules_enabled,
            compliance_programs: project.compliance_programs.clone(),
            custom_guard_rules: project.custom_guard_rules.clone(),
        }
    }

    pub fn to_project(&self) -> Project {
        let mut project = Project::new(
            self.name.clone(),
            self.description.clone(),
            self.short_name.clone(),
        );

        project.local_folder = self.local_folder.clone();
        project.git_url = self.git_url.clone();

        // Clear default environments created by Project::new
        project.environments.clear();

        // Convert EnvironmentForm to Project environments
        for env_form in &self.environments {
            let aws_regions = if !env_form.selected_regions.is_empty() {
                env_form
                    .selected_regions
                    .iter()
                    .map(|s| AwsRegion(s.clone()))
                    .collect()
            } else {
                env_form
                    .aws_regions
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| AwsRegion(s.to_string()))
                    .collect()
            };

            let aws_accounts = if !env_form.selected_accounts.is_empty() {
                env_form
                    .selected_accounts
                    .iter()
                    .map(|s| AwsAccount(s.clone()))
                    .collect()
            } else {
                env_form
                    .aws_accounts
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| AwsAccount(s.to_string()))
                    .collect()
            };

            let environment = Environment {
                name: env_form.name.clone(),
                aws_regions,
                aws_accounts,
                deployment_status: None,
            };

            project.environments.push(environment);
        }

        // Set compliance program fields
        project.guard_rules_enabled = self.guard_rules_enabled;
        project.compliance_programs = self.compliance_programs.clone();
        project.custom_guard_rules = self.custom_guard_rules.clone();

        project
    }
}

#[derive(Default)]
pub struct ProjectCommandPalette {
    pub mode: ProjectCommandAction,
    pub form: ProjectForm,
    pub current_project: Option<Project>,
    pub error_message: Option<String>,
    pub fuzzy_file_picker: Option<FuzzyFilePicker>,
    #[allow(dead_code)]
    aws_identity_center:
        Option<std::sync::Arc<std::sync::Mutex<crate::app::aws_identity::AwsIdentityCenter>>>,
    focus_requested: bool,
}

impl ProjectCommandPalette {
    pub fn new() -> Self {
        Self {
            mode: ProjectCommandAction::Closed,
            form: ProjectForm::default(),
            current_project: None,
            error_message: None,
            fuzzy_file_picker: None,
            aws_identity_center: None,
            focus_requested: false,
        }
    }

    pub fn set_aws_identity_center(
        &mut self,
        aws_identity_center: Option<
            std::sync::Arc<std::sync::Mutex<crate::app::aws_identity::AwsIdentityCenter>>,
        >,
    ) {
        self.aws_identity_center = aws_identity_center;
    }

    pub fn set_mode(&mut self, mode: ProjectCommandAction) {
        self.mode = mode;

        // Reset form when opening a new project
        if mode == ProjectCommandAction::NewProject {
            self.form = ProjectForm::default();
            self.error_message = None;
            self.focus_requested = false; // Reset focus flag
        } else if mode == ProjectCommandAction::EditProject {
            if let Some(project) = &self.current_project {
                self.form = ProjectForm::from_project(project);
                self.error_message = None;
                self.focus_requested = false; // Reset focus flag
            } else {
                // Can't edit if no project is loaded
                self.error_message = Some("No project is currently loaded to edit.".to_string());
                self.mode = ProjectCommandAction::CommandPalette;
            }
        }
    }

    pub fn show_command_palette(&mut self, ctx: &Context) -> Option<ProjectCommandAction> {
        let mut result = None;

        // Calculate dimensions similar to command palette
        let screen_rect = ctx.screen_rect();
        let window_height = screen_rect.height() * 0.25; // 1/4 of screen height
        let window_width = screen_rect.width() * 0.9; // 90% of screen width

        // Position at bottom of screen, centered horizontally (like command palette)
        let window_pos = egui::Pos2::new(
            screen_rect.center().x - (window_width / 2.0),
            screen_rect.max.y - window_height - 30.0, // 30px margin from bottom (slightly higher than main palette)
        );

        // Calculate column properties
        let column_width = (window_width * 0.35).min(400.0); // 35% of width, max 400px
        let column_spacing = window_width * 0.1; // 10% of width for spacing
        let left_margin = (window_width - (2.0 * column_width + column_spacing)) / 2.0;

        // Create window with fixed position and size
        let window_size = egui::Vec2::new(window_width, window_height);

        let _response = egui::Area::new(egui::Id::new("project_management"))
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

                        // Define command entries similar to command palette
                        let commands = [
                            (
                                egui::Key::N,
                                'N',
                                "New Project",
                                Color32::from_rgb(40, 140, 60), // Green
                                "Create a new project",
                                ProjectCommandAction::NewProject,
                            ),
                            (
                                egui::Key::O,
                                'O',
                                "Open Project",
                                Color32::from_rgb(100, 170, 255), // Blue
                                "Open an existing project",
                                ProjectCommandAction::OpenProject,
                            ),
                            (
                                egui::Key::E,
                                'E',
                                "Edit Project",
                                Color32::from_rgb(180, 140, 220), // Purple
                                "Edit the current project",
                                ProjectCommandAction::EditProject,
                            ),
                        ];

                        // Function to draw a command button
                        let mut draw_command =
                            |ui: &mut egui::Ui,
                             cmd: &(egui::Key, char, &str, Color32, &str, ProjectCommandAction),
                             _idx: usize| {
                                let mut clicked = false;
                                let key_pressed = ctx.input(|input| input.key_pressed(cmd.0));
                                let is_enabled = if cmd.5 == ProjectCommandAction::EditProject {
                                    self.current_project.is_some()
                                } else {
                                    true
                                };

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

                                        // Draw colored circle (grayed out if disabled)
                                        let fill_color = if is_enabled {
                                            cmd.3.linear_multiply(0.8) // Slightly darker
                                        } else {
                                            Color32::from_gray(100) // Gray for disabled
                                        };

                                        ui.painter().circle(
                                            rect.center(),
                                            rect.width() / 2.0,
                                            fill_color,
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
                                    ui.add_enabled_ui(is_enabled, |ui| {
                                        ui.vertical(|ui| {
                                            // Label with color
                                            ui.label(
                                                RichText::new(cmd.2)
                                                    .size(16.0)
                                                    .color(cmd.3)
                                                    .strong(),
                                            );
                                            // Description in smaller text
                                            ui.label(RichText::new(cmd.4).size(13.0).weak());
                                        });
                                    });

                                    if is_enabled && (response.clicked() || key_pressed) {
                                        clicked = true;
                                    }
                                });

                                if clicked || (key_pressed && is_enabled) {
                                    result = Some(cmd.5);
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
                    self.mode = ProjectCommandAction::Closed;
                }
            }
        }

        // Close on Escape key
        if self.mode == ProjectCommandAction::CommandPalette
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.mode = ProjectCommandAction::Closed;
        }

        result
    }

    pub fn show_project_form(&mut self, ctx: &Context) -> bool {
        let mut project_saved = false;
        let is_new = self.mode == ProjectCommandAction::NewProject;
        let title = if is_new {
            "New Project"
        } else {
            "Edit Project"
        };

        // Request focus for project name field when showing new project form (only once)
        if is_new && !self.focus_requested {
            ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("project_name_field")));
            self.focus_requested = true;
        }

        // Show fuzzy file picker if it's active
        if let Some(file_picker) = &mut self.fuzzy_file_picker {
            file_picker.show(ctx);

            // Handle file picker status
            match &file_picker.status {
                FuzzyFilePickerStatus::Selected(path) => {
                    // User selected a path
                    self.form.local_folder = Some(path.clone());
                    self.fuzzy_file_picker = None; // Close the picker
                }
                FuzzyFilePickerStatus::Closed => {
                    // User closed the picker
                    self.fuzzy_file_picker = None;
                }
                FuzzyFilePickerStatus::Open => {
                    // Picker is still open, continue showing it
                }
            }
        }

        let mut is_open = self.mode == ProjectCommandAction::NewProject
            || self.mode == ProjectCommandAction::EditProject
            || self.mode == ProjectCommandAction::OpenProject;

        Window::new(title)
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .min_width(500.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // Only add the heading if this is editing an existing project
                    // For new project, the window title is sufficient
                    if !is_new {
                        ui.heading(title);
                        ui.add_space(10.0);
                    } else {
                        // Just add some space at the top
                        ui.add_space(5.0);
                    }

                    if let Some(error) = &self.error_message {
                        ui.colored_label(Color32::from_rgb(220, 50, 50), error);
                        ui.add_space(10.0);
                    }

                    Grid::new("project_form_grid")
                        .num_columns(2)
                        .spacing([10.0, 10.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Project name
                            ui.label("Project Name:");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.form.name)
                                    .id(egui::Id::new("project_name_field")),
                            );
                            ui.end_row();

                            // Short name
                            ui.label("Short Name:");
                            ui.text_edit_singleline(&mut self.form.short_name);
                            ui.end_row();

                            // Description
                            ui.label("Description:");
                            ui.text_edit_singleline(&mut self.form.description);
                            ui.end_row();

                            // Project folder selection
                            ui.label("Project Folder:");
                            ui.horizontal(|ui| {
                                let folder_text = match &self.form.local_folder {
                                    Some(path) => path.to_string_lossy().to_string(),
                                    None => "No folder selected".to_string(),
                                };

                                let width = (ui.available_width() - 100.0).max(10.0);
                                let height = ui.spacing().interact_size.y.max(10.0);
                                ui.add_sized(
                                    [width, height],
                                    egui::TextEdit::singleline(&mut folder_text.as_str())
                                        .interactive(false),
                                );

                                if ui.button("Browse...").clicked() {
                                    // Initialize and show our custom fuzzy file picker
                                    self.fuzzy_file_picker = Some(FuzzyFilePicker::new());
                                }
                            });
                            ui.end_row();

                            // Git URL
                            ui.label("Git URL (optional):");
                            let mut git_url = self.form.git_url.clone().unwrap_or_default();
                            ui.text_edit_singleline(&mut git_url);
                            self.form.git_url = if git_url.is_empty() {
                                None
                            } else {
                                Some(git_url)
                            };
                            ui.end_row();

                            // CloudFormation Guard settings section
                            ui.label(RichText::new("CloudFormation Guard").strong().size(16.0));
                            ui.label(""); // Empty cell for spacing
                            ui.end_row();

                            // Guard validation enabled checkbox
                            ui.label("Enable Guard Validation:");
                            ui.checkbox(
                                &mut self.form.guard_rules_enabled,
                                "Validate templates with CloudFormation Guard",
                            );
                            ui.end_row();

                            // Compliance programs selection
                            ui.label("Compliance Programs:");
                            ui.vertical(|ui| {
                                let all_programs = vec![
                                    (
                                        crate::app::cfn_guard::ComplianceProgram::NIST80053R5,
                                        "NIST 800-53 Rev 5",
                                    ),
                                    (
                                        crate::app::cfn_guard::ComplianceProgram::NIST80053R4,
                                        "NIST 800-53 Rev 4",
                                    ),
                                    (crate::app::cfn_guard::ComplianceProgram::PCIDSS, "PCI DSS"),
                                    (crate::app::cfn_guard::ComplianceProgram::HIPAA, "HIPAA"),
                                    (crate::app::cfn_guard::ComplianceProgram::SOC, "SOC 2"),
                                    (crate::app::cfn_guard::ComplianceProgram::FedRAMP, "FedRAMP"),
                                    (
                                        crate::app::cfn_guard::ComplianceProgram::NIST800171,
                                        "NIST 800-171",
                                    ),
                                ];

                                for (program, description) in all_programs {
                                    let mut selected =
                                        self.form.compliance_programs.contains(&program);
                                    if ui.checkbox(&mut selected, description).changed() {
                                        if selected {
                                            if !self.form.compliance_programs.contains(&program) {
                                                self.form.compliance_programs.push(program);
                                            }
                                        } else {
                                            self.form.compliance_programs.retain(|p| p != &program);
                                        }
                                    }
                                }

                                if self.form.compliance_programs.is_empty() {
                                    ui.label(
                                        RichText::new("No compliance programs selected")
                                            .weak()
                                            .italics(),
                                    );
                                } else {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} programs selected",
                                            self.form.compliance_programs.len()
                                        ))
                                        .weak(),
                                    );
                                }
                            });
                            ui.end_row();

                            // Custom guard rules
                            ui.label("Custom Rules (paths):");
                            ui.vertical(|ui| {
                                let mut to_remove = Vec::new();
                                for (index, rule_path) in
                                    self.form.custom_guard_rules.iter_mut().enumerate()
                                {
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(rule_path);
                                        if ui.button("Remove").clicked() {
                                            to_remove.push(index);
                                        }
                                    });
                                }

                                // Remove rules marked for removal (in reverse order to maintain indices)
                                for &index in to_remove.iter().rev() {
                                    self.form.custom_guard_rules.remove(index);
                                }

                                if ui.button("+ Add Custom Rule").clicked() {
                                    self.form.custom_guard_rules.push(String::new());
                                }

                                if self.form.custom_guard_rules.is_empty() {
                                    ui.label(
                                        RichText::new("No custom rules configured")
                                            .weak()
                                            .italics(),
                                    );
                                }
                            });
                            ui.end_row();

                            // Environment section header
                            ui.label(RichText::new("Environments").strong().size(16.0));
                            ui.end_row();

                            // For each environment, display a collapsible section
                            for env_idx in 0..self.form.environments.len() {
                                let env = &mut self.form.environments[env_idx];

                                // Environment name header
                                ui.label(format!("Environment {}:", env_idx + 1));
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(&mut env.name);
                                });
                                ui.end_row();

                                // AWS Regions for this environment
                                ui.label("   AWS Regions:");
                                ui.vertical(|ui| {
                                    // Show selected regions
                                    if !env.selected_regions.is_empty() {
                                        ui.horizontal_wrapped(|ui| {
                                            let mut to_remove = None;
                                            for (idx, region) in
                                                env.selected_regions.iter().enumerate()
                                            {
                                                ui.group(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(region);
                                                        if ui.small_button("×").clicked() {
                                                            to_remove = Some(idx);
                                                        }
                                                    });
                                                });
                                            }
                                            if let Some(idx) = to_remove {
                                                env.selected_regions.remove(idx);
                                            }
                                        });
                                        ui.add_space(4.0);
                                    }

                                    // Region selection dropdown
                                    egui::ComboBox::from_id_salt(format!(
                                        "region_select_{}",
                                        env_idx
                                    ))
                                    .selected_text("Select region...")
                                    .width(200.0)
                                    .show_ui(ui, |ui| {
                                        for region in AWS_REGIONS {
                                            if ui.selectable_label(false, *region).clicked()
                                                && !env
                                                    .selected_regions
                                                    .contains(&region.to_string())
                                            {
                                                env.selected_regions.push(region.to_string());
                                            }
                                        }
                                    });
                                });
                                ui.end_row();

                                // AWS Accounts for this environment
                                ui.label("   AWS Accounts:");
                                ui.vertical(|ui| {
                                    // Show selected accounts
                                    if !env.selected_accounts.is_empty() {
                                        ui.horizontal_wrapped(|ui| {
                                            let mut to_remove = None;
                                            for (idx, account_id) in
                                                env.selected_accounts.iter().enumerate()
                                            {
                                                ui.group(|ui| {
                                                    ui.horizontal(|ui| {
                                                        // Try to find account name from AWS Identity Center
                                                        let display_text =
                                                            if let Some(aws_identity_center) =
                                                                &self.aws_identity_center
                                                            {
                                                                if let Ok(identity_center) =
                                                                    aws_identity_center.lock()
                                                                {
                                                                    identity_center
                                                                        .accounts
                                                                        .iter()
                                                                        .find(|acc| {
                                                                            &acc.account_id
                                                                                == account_id
                                                                        })
                                                                        .map(|acc| {
                                                                            format!(
                                                                                "{} ({})",
                                                                                acc.account_name,
                                                                                acc.account_id
                                                                            )
                                                                        })
                                                                        .unwrap_or_else(|| {
                                                                            account_id.clone()
                                                                        })
                                                                } else {
                                                                    account_id.clone()
                                                                }
                                                            } else {
                                                                account_id.clone()
                                                            };

                                                        ui.label(&display_text);
                                                        if ui.small_button("×").clicked() {
                                                            to_remove = Some(idx);
                                                        }
                                                    });
                                                });
                                            }
                                            if let Some(idx) = to_remove {
                                                env.selected_accounts.remove(idx);
                                            }
                                        });
                                        ui.add_space(4.0);
                                    }

                                    // Account selection dropdown
                                    if let Some(aws_identity_center) = &self.aws_identity_center {
                                        if let Ok(identity_center) = aws_identity_center.lock() {
                                            egui::ComboBox::from_id_salt(format!(
                                                "account_select_{}",
                                                env_idx
                                            ))
                                            .selected_text("Select account...")
                                            .width(300.0)
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    for account in &identity_center.accounts {
                                                        let display_text = format!(
                                                            "{} - {}",
                                                            account.account_name,
                                                            account.account_id
                                                        );
                                                        if ui
                                                            .selectable_label(false, &display_text)
                                                            .clicked()
                                                            && !env
                                                                .selected_accounts
                                                                .contains(&account.account_id)
                                                        {
                                                            env.selected_accounts
                                                                .push(account.account_id.clone());
                                                        }
                                                    }
                                                },
                                            );
                                        }
                                    } else {
                                        ui.label(
                                            RichText::new("AWS Identity Center not configured")
                                                .weak(),
                                        );
                                        ui.label(
                                            RichText::new("Login first to select accounts")
                                                .size(11.0)
                                                .weak(),
                                        );
                                    }
                                });
                                ui.end_row();

                                // Add a spacer between environments
                                if env_idx < self.form.environments.len() - 1 {
                                    ui.label("");
                                    ui.label("");
                                    ui.end_row();
                                }
                            }

                            // Add new environment button
                            ui.label("");
                            if ui.button("+ Add Environment").clicked() {
                                self.form.environments.push(EnvironmentForm {
                                    name: format!(
                                        "Environment {}",
                                        self.form.environments.len() + 1
                                    ),
                                    aws_regions: String::new(),
                                    aws_accounts: String::new(),
                                    selected_regions: Vec::new(),
                                    selected_accounts: Vec::new(),
                                });
                            }
                            ui.end_row();
                        });

                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::RIGHT), |ui| {
                            if ui.button("Cancel").clicked() {
                                self.mode = ProjectCommandAction::CommandPalette;
                            }

                            if ui.button("Save").clicked() {
                                self.error_message = None;

                                // Sync selected values with text fields before saving
                                for env in &mut self.form.environments {
                                    if !env.selected_regions.is_empty() {
                                        env.aws_regions = env.selected_regions.join(", ");
                                    }
                                    if !env.selected_accounts.is_empty() {
                                        env.aws_accounts = env.selected_accounts.join(", ");
                                    }
                                }

                                // Validate form
                                if self.form.name.trim().is_empty() {
                                    self.error_message =
                                        Some("Project name is required".to_string());
                                } else if self.form.short_name.trim().is_empty() {
                                    self.error_message = Some("Short name is required".to_string());
                                } else if self.form.local_folder.is_none() {
                                    self.error_message =
                                        Some("Project folder is required".to_string());
                                } else {
                                    // Create or update project
                                    let project = self.form.to_project();

                                    // Save project to Project.json
                                    if let Some(folder) = &project.local_folder {
                                        let file_path = folder.join("Project.json");
                                        match serde_json::to_string_pretty(&project) {
                                            Ok(json_content) => {
                                                match fs::write(&file_path, json_content) {
                                                    Ok(_) => {
                                                        info!(
                                                            "Project saved to {}",
                                                            file_path.display()
                                                        );
                                                        self.current_project = Some(project);
                                                        self.mode = ProjectCommandAction::Closed;
                                                        project_saved = true;
                                                    }
                                                    Err(e) => {
                                                        error!("Error writing Project.json: {}", e);
                                                        self.error_message = Some(format!(
                                                            "Error saving project: {}",
                                                            e
                                                        ));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("Error serializing project: {}", e);
                                                self.error_message = Some(format!(
                                                    "Error serializing project: {}",
                                                    e
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    });
                });
            });

        // If the window was closed (by X button), go back to command palette
        if !is_open {
            self.mode = ProjectCommandAction::CommandPalette;
        }

        // If escape was pressed, go back to project management
        if (self.mode == ProjectCommandAction::NewProject
            || self.mode == ProjectCommandAction::EditProject
            || self.mode == ProjectCommandAction::OpenProject)
            && ctx.input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.mode = ProjectCommandAction::CommandPalette;
        }

        project_saved
    }

    pub fn load_project(&mut self, path: &Path) -> Result<(), String> {
        let file_path = path.join("Project.json");
        tracing::info!(
            "📂 PROJECT_LOAD: Starting project load from {}",
            file_path.display()
        );

        if !file_path.exists() {
            return Err(format!("Project file not found at {}", file_path.display()));
        }

        match fs::read_to_string(&file_path) {
            Ok(content) => match serde_json::from_str::<Project>(&content) {
                Ok(mut project) => {
                    // Set the local folder to ensure resources can be loaded
                    if project.local_folder.is_none() {
                        tracing::info!("Setting project local folder to {}", path.display());
                        project.local_folder = Some(path.to_path_buf());
                    }

                    // DAG is now built dynamically from resources - no initialization needed

                    // Check resources directory size vs DAG size
                    if let Some(local_folder) = &project.local_folder {
                        let resources_dir = local_folder.join("Resources");
                        if resources_dir.exists() {
                            let mut resource_count = 0;
                            if let Ok(entries) = fs::read_dir(&resources_dir) {
                                for entry in entries.flatten() {
                                    let path = entry.path();
                                    if path.is_file()
                                        && path.extension().is_some_and(|ext| ext == "json")
                                    {
                                        resource_count += 1;
                                    }
                                }
                            }

                            let project_resources_count = project.get_resources().len();

                            tracing::info!(
                                "Project has {} resource files and {} project resources",
                                resource_count,
                                project_resources_count
                            );

                            if resource_count > 0 && project_resources_count == 0 {
                                tracing::info!(
                                    "Resources exist but not accessible through project - may need loading"
                                );
                                // Resources will be loaded dynamically when accessed
                            }
                        }
                    }

                    // Check if we need to migrate to single file format
                    if let Some(local_folder) = &project.local_folder {
                        let resources_dir = local_folder.join("Resources");
                        let single_file = resources_dir.join("resources.json");
                        let template_file = resources_dir.join("cloudformation_template.json");

                        // BUGFIX: Don't migrate if CloudFormation template already exists
                        // CloudFormation template format is newer than single file format
                        // Migrate only if we have individual files but no single file AND no template file
                        if resources_dir.exists()
                            && !single_file.exists()
                            && !template_file.exists()
                        {
                            tracing::info!("Migrating resources to single file format");
                            match project.migrate_to_single_file() {
                                Ok(()) => {
                                    tracing::info!("Successfully migrated to single file format");
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to migrate to single file format: {}",
                                        e
                                    );
                                }
                            }
                        }
                    }

                    // Load resources from the Resources directory
                    tracing::info!("📂 PROJECT_LOAD: Loading resources from project directory");
                    match project.load_resources_from_directory() {
                        Ok(count) => {
                            tracing::info!(
                                "📂 PROJECT_LOAD: Successfully loaded {} resources from directory",
                                count
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "📂 PROJECT_LOAD: Failed to load resources from directory: {}",
                                e
                            );
                            // Continue anyway, not a critical error
                        }
                    }

                    // Load the CloudFormation template if it exists
                    if let Some(local_folder) = &project.local_folder {
                        let template_path = local_folder
                            .join("Resources")
                            .join("cloudformation_template.json");
                        if template_path.exists() {
                            tracing::info!(
                                "Loading CloudFormation template from {:?}",
                                template_path
                            );
                            match crate::app::cfn_template::CloudFormationTemplate::from_file(
                                &template_path,
                            ) {
                                Ok(template) => {
                                    project.cfn_template = Some(template);
                                    tracing::info!("Successfully loaded CloudFormation template");
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to load CloudFormation template: {}", e);
                                    // Continue anyway, not a critical error
                                }
                            }
                        }
                    }

                    let final_resource_count = project.get_resources().len();
                    // Build a temporary DAG to get position count for logging
                    let final_dag_positions = project
                        .build_dag_from_resources()
                        .get_node_positions()
                        .len();

                    tracing::info!(
                        "📂 PROJECT_LOAD: Project loaded - {} resources, {} DAG positions",
                        final_resource_count,
                        final_dag_positions
                    );

                    self.current_project = Some(project);

                    tracing::info!(
                        "✅ PROJECT_LOAD: Project loading complete - current_project set"
                    );
                    Ok(())
                }
                Err(e) => Err(format!("Error parsing Project.json: {}", e)),
            },
            Err(e) => Err(format!("Error reading Project.json: {}", e)),
        }
    }

    pub fn show(&mut self, ctx: &Context) -> bool {
        let mut project_saved = false;
        let previous_project_state = self.current_project.is_some();

        match self.mode {
            ProjectCommandAction::Closed => {}
            ProjectCommandAction::CommandPalette => {
                if let Some(new_mode) = self.show_command_palette(ctx) {
                    self.set_mode(new_mode);
                }
            }
            ProjectCommandAction::NewProject | ProjectCommandAction::EditProject => {
                project_saved = self.show_project_form(ctx);
            }
            ProjectCommandAction::OpenProject => {
                // Initialize file picker if not already done
                if self.fuzzy_file_picker.is_none() {
                    self.open_project_dialog();
                }

                // Show fuzzy file picker if it's active
                let mut picker_action = None;
                let mut picker_path = None;

                // First, interact with the picker and get its status
                if let Some(file_picker) = &mut self.fuzzy_file_picker {
                    file_picker.show(ctx);

                    // Check status without borrowing self again
                    match &file_picker.status {
                        FuzzyFilePickerStatus::Selected(path) => {
                            picker_action = Some(true); // true = selected a path
                            picker_path = Some(path.clone());
                        }
                        FuzzyFilePickerStatus::Closed => {
                            picker_action = Some(false); // false = closed without selecting
                        }
                        FuzzyFilePickerStatus::Open => {
                            // Picker is still open, do nothing
                        }
                    }
                }

                // Now process the picker result if needed
                if let Some(is_selected) = picker_action {
                    // Close the picker first
                    self.fuzzy_file_picker = None;

                    if is_selected {
                        // User selected a path, try to load the project
                        if let Some(path) = picker_path {
                            match self.load_project(&path) {
                                Ok(_) => {
                                    info!("Project loaded from {}", path.display());
                                    self.mode = ProjectCommandAction::Closed;
                                    // Project was successfully loaded
                                    project_saved = true;
                                }
                                Err(e) => {
                                    error!("Error loading project: {}", e);
                                    self.error_message =
                                        Some(format!("Error loading project: {}", e));
                                    self.mode = ProjectCommandAction::CommandPalette;
                                }
                            }
                        }
                    } else {
                        // User cancelled, go back to command palette
                        self.mode = ProjectCommandAction::CommandPalette;
                    }
                }
            }
        }

        // This was used for signaling if a project was loaded or saved
        // We're now detecting this in the app.rs file directly
        let _project_state_changed =
            (self.current_project.is_some() != previous_project_state) || project_saved;

        project_saved
    }

    pub fn get_current_project_summary(&self) -> Option<(String, String, String)> {
        self.current_project.as_ref().map(|project| {
            // Get all regions across all environments for display
            let all_regions = project.get_all_regions();
            let regions = all_regions
                .iter()
                .map(|r| r.0.clone())
                .collect::<Vec<_>>()
                .join(", ");

            // Get all accounts across all environments for display, preferring names over IDs
            let all_accounts = project.get_all_accounts();
            let accounts = if let Some(aws_identity_center) = &self.aws_identity_center {
                if let Ok(identity_center) = aws_identity_center.lock() {
                    // Try to find account names from AWS Identity Center
                    all_accounts
                        .iter()
                        .map(|project_account| {
                            // Look for matching account in identity center
                            identity_center
                                .accounts
                                .iter()
                                .find(|identity_account| {
                                    identity_account.account_id == project_account.0
                                })
                                .map(|identity_account| identity_account.account_name.clone())
                                .unwrap_or_else(|| project_account.0.clone()) // Fallback to ID if name not found
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    // Fallback to account IDs if identity center is locked
                    all_accounts
                        .iter()
                        .map(|a| a.0.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            } else {
                // Fallback to account IDs if no identity center available
                all_accounts
                    .iter()
                    .map(|a| a.0.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            (project.name.clone(), regions, accounts)
        })
    }

    pub fn open_project_dialog(&mut self) {
        self.error_message = None;

        // Initialize our custom fuzzy file picker if not already active
        if self.fuzzy_file_picker.is_none() {
            self.fuzzy_file_picker = Some(FuzzyFilePicker::new());
        }

        // The fuzzy_file_picker will be shown in the show() method
        // in the ProjectCommandAction::OpenProject case
    }
}
