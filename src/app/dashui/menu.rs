use crate::app::aws_identity::LoginState;
use crate::app::dashui::app::{NavigationStatusBarSettings, ThemeChoice};
use eframe::egui;
use egui::{Color32, RichText};
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq)]
pub enum MenuAction {
    None,
    ThemeChanged,
    NavigationStatusBarChanged,
    AgentLoggingChanged,
    ShowComplianceDetails,
    ValidateCompliance,
    LoginAWS,
    AWSExplorer,
    AgentManager,
    PagesManager,
    Quit,
}

/// Compliance status for Guard validation
#[derive(Debug, Clone, PartialEq)]
pub enum ComplianceStatus {
    /// All rules passed - template is compliant
    Compliant,
    /// Some rules failed - violations found
    Violations(usize),
    /// Validation is currently running
    Validating,
    /// Validation not performed or Guard disabled
    NotValidated,
    /// Error occurred during validation
    ValidationError(String),
}

#[allow(clippy::too_many_arguments)]
pub fn build_menu(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    theme: &mut ThemeChoice,
    navigation_status_bar_settings: &mut NavigationStatusBarSettings,
    agent_logging_enabled: &mut bool,
    project_info: Option<(String, String, String)>,
    log_window_open: &mut bool,
    resource_count: Option<usize>,
    aws_identity_center: Option<&Arc<Mutex<crate::app::aws_identity::AwsIdentityCenter>>>,
    compliance_status: Option<ComplianceStatus>,
    compliance_programs: Option<&Vec<String>>,
) -> MenuAction {
    let mut theme_changed = false;
    let mut navigation_status_bar_changed = false;
    let mut agent_logging_changed = false;
    let mut menu_action = MenuAction::None;
    let original_theme = *theme;
    let original_status_bar_setting = *navigation_status_bar_settings;
    let original_agent_logging = *agent_logging_enabled;

    // Dash menu with command palette items
    ui.menu_button("Dash", |ui| {
        if ui.button("Login to AWS").clicked() {
            menu_action = MenuAction::LoginAWS;
        }
        if ui.button("Explorer").clicked() {
            menu_action = MenuAction::AWSExplorer;
        }
        if ui.button("Agents").clicked() {
            menu_action = MenuAction::AgentManager;
        }
        if ui.button("Pages").clicked() {
            menu_action = MenuAction::PagesManager;
        }
        ui.separator();
        if ui.button("Quit").clicked() {
            menu_action = MenuAction::Quit;
        }
    });

    ui.menu_button(RichText::new("🎨").size(18.0), |ui| {
        if ui.button("Latte").clicked() {
            catppuccin_egui::set_theme(ctx, catppuccin_egui::LATTE);
            *theme = ThemeChoice::Latte;
        }
        if ui.button("Frappe").clicked() {
            catppuccin_egui::set_theme(ctx, catppuccin_egui::FRAPPE);
            *theme = ThemeChoice::Frappe;
        }
        if ui.button("Macchiato").clicked() {
            catppuccin_egui::set_theme(ctx, catppuccin_egui::MACCHIATO);
            *theme = ThemeChoice::Macchiato;
        }
        if ui.button("Mocha").clicked() {
            catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA);
            *theme = ThemeChoice::Mocha;
        }

        ui.separator();

        // Navigation Status Bar toggle
        let checkbox_response = ui.checkbox(
            &mut navigation_status_bar_settings.show_status_bar,
            "Show Navigation Status Bar",
        );
        if checkbox_response.hovered() {
            checkbox_response.on_hover_text(
                "Toggle the Vimium-like navigation status bar showing mode, keys, and hints",
            );
        }

        ui.separator();

        // Agent Logging toggle
        let logging_response = ui.checkbox(agent_logging_enabled, "Agent Logging");
        if logging_response.hovered() {
            logging_response.on_hover_text(
                "Enable CloudWatch Agent Logging for monitoring and evaluation. \
                 Requires CloudWatch permissions in your AWS role.",
            );
        }
    });

    if original_theme != *theme {
        theme_changed = true;
    }

    if original_status_bar_setting.show_status_bar != navigation_status_bar_settings.show_status_bar
    {
        navigation_status_bar_changed = true;
    }

    if original_agent_logging != *agent_logging_enabled {
        agent_logging_changed = true;
    }

    // AWS login status indicator
    show_aws_login_status(ui, aws_identity_center);

    // Compliance programs display and validation button
    if let Some(validation_action) =
        show_compliance_programs_and_validation(ui, compliance_programs, compliance_status)
    {
        return validation_action;
    }

    // Add a log button - positioned on far right
    if ui.button(RichText::new("📜").size(16.0)).clicked() {
        *log_window_open = !*log_window_open;
        log_debug!("Log button clicked");
    }

    ui.add_space(16.0);

    // Display project info if available
    if let Some((name, regions, accounts)) = project_info {
        ui.horizontal(|ui| {
            ui.label("Project:");
            ui.label(
                RichText::new(name)
                    .color(Color32::from_rgb(180, 140, 220))
                    .strong(),
            );

            if !regions.is_empty() {
                ui.separator();
                ui.label("Regions:");
                ui.label(RichText::new(regions).color(Color32::from_rgb(100, 170, 255)));
            }

            if !accounts.is_empty() {
                ui.separator();
                ui.label("Accounts:");
                ui.label(RichText::new(accounts).color(Color32::from_rgb(255, 190, 70)));
            }

            // Display resource count if available
            if let Some(count) = resource_count {
                ui.separator();
                ui.label(
                    RichText::new(format!("{} CloudFormation Resources", count))
                        .color(Color32::from_rgb(140, 200, 170))
                        .strong(),
                );
            }
        });
    }

    if menu_action != MenuAction::None {
        menu_action
    } else if theme_changed {
        MenuAction::ThemeChanged
    } else if navigation_status_bar_changed {
        MenuAction::NavigationStatusBarChanged
    } else if agent_logging_changed {
        MenuAction::AgentLoggingChanged
    } else {
        MenuAction::None
    }
}

/// Displays the AWS login status indicator
fn show_aws_login_status(
    ui: &mut egui::Ui,
    aws_identity_center: Option<&Arc<Mutex<crate::app::aws_identity::AwsIdentityCenter>>>,
) {
    // Get the LoginState as the single source of truth
    // Use try_lock() to avoid blocking UI when login thread holds the mutex
    let (login_state, lock_busy) = if let Some(aws_identity) = aws_identity_center {
        match aws_identity.try_lock() {
            Ok(identity) => (Some(identity.login_state.clone()), false),
            Err(_) => {
                // Lock is held by login thread - indicate busy state
                (None, true)
            }
        }
    } else {
        (None, false)
    };

    // Display the indicator based on LoginState (or busy state if lock contended)
    let tooltip_text = if lock_busy {
        "AWS: Authorizing... (please wait)"
    } else {
        match &login_state {
            Some(LoginState::LoggedIn) => "AWS: Logged in and connected",
            Some(LoginState::DeviceAuthorization(_)) => "AWS: Authorization in progress",
            Some(LoginState::Error(_)) => "AWS: Login error",
            _ => "AWS: Not logged in",
        }
    };

    // Simple AWS text indicator
    let click_response = ui.horizontal(|ui| {
        // Login status text and color (handle lock_busy as "authorizing")
        let (status_text, text_color) = if lock_busy {
            ("Authorizing...", Color32::from_rgb(220, 180, 50))
        } else {
            match &login_state {
                Some(LoginState::LoggedIn) => ("Logged In", Color32::from_rgb(50, 200, 80)),
                Some(LoginState::DeviceAuthorization(_)) => {
                    ("Logging In", Color32::from_rgb(220, 180, 50))
                }
                Some(LoginState::Error(_)) => ("Login Error", Color32::from_rgb(200, 50, 50)),
                _ => ("Not Logged In", Color32::from_rgb(180, 180, 180)),
            }
        };

        let status_label = RichText::new(status_text)
            .strong()
            .size(12.0)
            .color(text_color);

        ui.label(status_label);
    });

    // Add tooltip on hover
    let response = click_response.response.on_hover_text(tooltip_text);

    // Check if clicked
    if response.clicked() {
        log_debug!("AWS login indicator clicked");
        // Could trigger login window here if needed
    }
}

/// Displays the compliance status indicator for CloudFormation Guard validation
#[allow(dead_code)]
fn show_compliance_status(
    ui: &mut egui::Ui,
    compliance_status: Option<ComplianceStatus>,
) -> Option<MenuAction> {
    match compliance_status {
        Some(ComplianceStatus::Compliant) => {
            let response = ui.button(
                RichText::new("✅ Compliant")
                    .color(Color32::from_rgb(50, 200, 80))
                    .strong()
                    .size(12.0),
            );
            if response.clicked() {
                log_debug!("Compliance status (compliant) clicked");
                return Some(MenuAction::ShowComplianceDetails);
            }
            if response.hovered() {
                response
                    .on_hover_text("CloudFormation Guard validation passed - click for details");
            }
        }
        Some(ComplianceStatus::Violations(count)) => {
            let response = ui.button(
                RichText::new(format!("❌ {} Violations", count))
                    .color(Color32::from_rgb(200, 50, 50))
                    .strong()
                    .size(12.0),
            );
            if response.clicked() {
                log_debug!("Compliance status ({} violations) clicked", count);
                return Some(MenuAction::ShowComplianceDetails);
            }
            if response.hovered() {
                response.on_hover_text(
                    "CloudFormation Guard found policy violations - click to view details",
                );
            }
        }
        Some(ComplianceStatus::Validating) => {
            let response = ui.button(
                RichText::new("⚠️ Validating...")
                    .color(Color32::from_rgb(220, 180, 50))
                    .strong()
                    .size(12.0),
            );
            if response.hovered() {
                response.on_hover_text("CloudFormation Guard validation in progress");
            }
        }
        Some(ComplianceStatus::ValidationError(ref error)) => {
            let response = ui.button(
                RichText::new("🚫 Validation Error")
                    .color(Color32::from_rgb(200, 100, 50))
                    .strong()
                    .size(12.0),
            );
            if response.clicked() {
                log_debug!("Compliance status (error) clicked");
                return Some(MenuAction::ShowComplianceDetails);
            }
            if response.hovered() {
                response.on_hover_text(format!("CloudFormation Guard validation error: {}", error));
            }
        }
        Some(ComplianceStatus::NotValidated) => {
            let response = ui.button(
                RichText::new("⚪ Not Validated")
                    .color(Color32::from_rgb(150, 150, 150))
                    .strong()
                    .size(12.0),
            );
            if response.clicked() {
                log_debug!("Compliance status (not validated) clicked");
                return Some(MenuAction::ShowComplianceDetails);
            }
            if response.hovered() {
                response.on_hover_text(
                    "CloudFormation Guard validation not performed - click to configure",
                );
            }
        }
        None => {
            // No compliance status available - don't show anything
        }
    }
    None
}

/// Displays compliance programs and validation button
fn show_compliance_programs_and_validation(
    ui: &mut egui::Ui,
    compliance_programs: Option<&Vec<String>>,
    compliance_status: Option<ComplianceStatus>,
) -> Option<MenuAction> {
    match compliance_programs {
        Some(programs) if !programs.is_empty() => {
            let mut action = None;
            ui.horizontal(|ui| {
                // Display compliance programs
                ui.label("Compliance:");
                let programs_text = programs
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                ui.label(
                    RichText::new(programs_text)
                        .color(Color32::from_rgb(160, 120, 200))
                        .size(11.0),
                );

                ui.separator();

                // Validation button based on current status
                action = match compliance_status {
                    Some(ComplianceStatus::Compliant) => {
                        let response = ui.button(
                            RichText::new("✅ Compliant")
                                .color(Color32::from_rgb(50, 200, 80))
                                .strong()
                                .size(12.0),
                        );
                        if response.clicked() {
                            log_debug!("Compliance details button clicked");
                            Some(MenuAction::ShowComplianceDetails)
                        } else {
                            response.on_hover_text("CloudFormation Guard validation passed - click for details");
                            None
                        }
                    }
                    Some(ComplianceStatus::Violations(count)) => {
                        let response = ui.button(
                            RichText::new(format!("❌ {} Violations", count))
                                .color(Color32::from_rgb(200, 50, 50))
                                .strong()
                                .size(12.0),
                        );
                        if response.clicked() {
                            log_debug!("Compliance violations button clicked");
                            Some(MenuAction::ShowComplianceDetails)
                        } else {
                            response.on_hover_text("CloudFormation Guard found policy violations - click to view details");
                            None
                        }
                    }
                    Some(ComplianceStatus::Validating) => {
                        let response = ui.button(
                            RichText::new("⚠️ Validating...")
                                .color(Color32::from_rgb(220, 180, 50))
                                .strong()
                                .size(12.0),
                        );
                        response.on_hover_text("CloudFormation Guard validation in progress");
                        None
                    }
                    Some(ComplianceStatus::ValidationError(ref error)) => {
                        let response = ui.button(
                            RichText::new("🚫 Validation Error")
                                .color(Color32::from_rgb(200, 100, 50))
                                .strong()
                                .size(12.0),
                        );
                        if response.clicked() {
                            log_debug!("Compliance error button clicked");
                            Some(MenuAction::ShowComplianceDetails)
                        } else {
                            response.on_hover_text(format!("CloudFormation Guard validation error: {}", error));
                            None
                        }
                    }
                    Some(ComplianceStatus::NotValidated) | None => {
                        let response = ui.button(
                            RichText::new("🔍 Validate")
                                .color(Color32::from_rgb(100, 170, 255))
                                .strong()
                                .size(12.0),
                        );
                        if response.clicked() {
                            log_debug!("Compliance validation button clicked");
                            Some(MenuAction::ValidateCompliance)
                        } else {
                            response.on_hover_text("Click to validate CloudFormation template against compliance programs");
                            None
                        }
                    }
                }
            });
            action
        }
        _ => {
            // No compliance programs configured - don't show anything
            None
        }
    }
}
