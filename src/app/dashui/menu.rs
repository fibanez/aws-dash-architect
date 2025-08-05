use crate::app::aws_identity::LoginState;
use crate::app::dashui::app::ThemeChoice;
use crate::log_debug;
use eframe::egui;
use egui::{Color32, RichText};
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq)]
pub enum MenuAction {
    None,
    ThemeChanged,
    ShakeWindows,
    ShowWindowSelector,
    ShowComplianceDetails,
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
    project_info: Option<(String, String, String)>,
    help_window_open: &mut bool, // Changed from _resource_graph_show
    log_window_open: &mut bool,
    resource_count: Option<usize>,
    aws_identity_center: Option<&Arc<Mutex<crate::app::aws_identity::AwsIdentityCenter>>>,
    window_selector: &mut crate::app::dashui::window_selector::WindowSelector,
    compliance_status: Option<ComplianceStatus>,
) -> (MenuAction, Option<String>) {
    let mut theme_changed = false;
    let original_theme = *theme;

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
    });

    if original_theme != *theme {
        theme_changed = true;
    }

    // Add a log button
    if ui.button(RichText::new("📜").size(16.0)).clicked() {
        *log_window_open = !*log_window_open;
        log_debug!("Log button clicked");
    }

    // Add a visual effect button
    if ui.button(RichText::new("✨").size(16.0)).clicked() {
        log_debug!("Visual effect button clicked");
        return (MenuAction::ShakeWindows, None);
    }

    // Add window selector menu
    let selected_window = window_selector.show_menu(ui);

    // Add a help button
    if ui.button(RichText::new("❓").size(16.0)).clicked() {
        *help_window_open = true;
        log_debug!("Help button clicked");
    }

    // AWS login status indicator
    show_aws_login_status(ui, aws_identity_center);

    // Compliance status indicator
    if let Some(compliance_action) = show_compliance_status(ui, compliance_status) {
        return (compliance_action, None);
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

    let menu_action = if theme_changed {
        MenuAction::ThemeChanged
    } else {
        MenuAction::None
    };

    (menu_action, selected_window)
}

/// Displays the AWS login status indicator
fn show_aws_login_status(
    ui: &mut egui::Ui,
    aws_identity_center: Option<&Arc<Mutex<crate::app::aws_identity::AwsIdentityCenter>>>,
) {
    // Get the LoginState as the single source of truth
    let login_state = if let Some(aws_identity) = aws_identity_center {
        if let Ok(identity) = aws_identity.lock() {
            Some(identity.login_state.clone())
        } else {
            None
        }
    } else {
        None
    };

    // Display the indicator based solely on LoginState
    let tooltip_text = match &login_state {
        Some(LoginState::LoggedIn) => "AWS: Logged in and connected",
        Some(LoginState::DeviceAuthorization(_)) => "AWS: Authorization in progress",
        Some(LoginState::Error(_)) => "AWS: Login error",
        _ => "AWS: Not logged in",
    };

    // Simple AWS text indicator
    let click_response = ui.horizontal(|ui| {
        // Login status text and color based solely on LoginState
        let (status_text, text_color) = match &login_state {
            Some(LoginState::LoggedIn) => ("Logged In", Color32::from_rgb(50, 200, 80)),
            Some(LoginState::DeviceAuthorization(_)) => {
                ("Logging In", Color32::from_rgb(220, 180, 50))
            }
            Some(LoginState::Error(_)) => ("Login Error", Color32::from_rgb(200, 50, 50)),
            _ => ("Not Logged In", Color32::from_rgb(180, 180, 180)),
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
                response.on_hover_text("CloudFormation Guard validation passed - click for details");
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
                response.on_hover_text("CloudFormation Guard found policy violations - click to view details");
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
                response.on_hover_text("CloudFormation Guard validation not performed - click to configure");
            }
        }
        None => {
            // No compliance status available - don't show anything
        }
    }
    None
}
