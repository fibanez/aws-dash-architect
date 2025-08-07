//! Compliance Discovery Error Window
//!
//! This window displays errors that occur during compliance program discovery
//! and provides manual retry functionality when GitHub or network issues prevent
//! automatic compliance program loading.

use super::window_focus::FocusableWindow;
use crate::app::compliance_discovery::ComplianceDiscoveryError;
use eframe::egui;
use egui::{Color32, RichText};

#[derive(Default)]
pub struct ComplianceErrorWindow {
    pub visible: bool,
    pub error: Option<ComplianceDiscoveryError>,
    pub retry_requested: bool,
}

impl ComplianceErrorWindow {
    pub fn new() -> Self {
        Self {
            visible: false,
            error: None,
            retry_requested: false,
        }
    }

    pub fn show_error(&mut self, error: ComplianceDiscoveryError) {
        self.visible = true;
        self.error = Some(error);
        self.retry_requested = false;
    }

    pub fn is_retry_requested(&self) -> bool {
        self.retry_requested
    }

    pub fn clear_retry_request(&mut self) {
        self.retry_requested = false;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.error = None;
        self.retry_requested = false;
    }

    pub fn close(&mut self) {
        self.hide();
    }

    /// Show the window and return true if retry was clicked
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        self.show_with_focus(ctx, (), false);
        
        if self.retry_requested {
            self.retry_requested = false;
            true
        } else {
            false
        }
    }

    fn get_error_icon(&self, error: &ComplianceDiscoveryError) -> &'static str {
        match error {
            ComplianceDiscoveryError::GitHubApiFailure { .. } => "🌐",
            ComplianceDiscoveryError::InvalidRepositoryStructure { .. } => "📂",
            ComplianceDiscoveryError::MappingFileDownloadFailure { .. } => "📥",
            ComplianceDiscoveryError::MappingFileParseFailure { .. } => "📄",
            ComplianceDiscoveryError::NetworkFailure { .. } => "🔌",
            ComplianceDiscoveryError::RetryableFailure { .. } => "⚠️",
        }
    }

    fn get_error_title(&self, error: &ComplianceDiscoveryError) -> String {
        match error {
            ComplianceDiscoveryError::GitHubApiFailure { status_code, .. } => {
                format!("GitHub API Error ({})", status_code)
            }
            ComplianceDiscoveryError::InvalidRepositoryStructure { .. } => {
                "Invalid Repository Structure".to_string()
            }
            ComplianceDiscoveryError::MappingFileDownloadFailure { failed_files, .. } => {
                format!("Failed to Download {} Files", failed_files.len())
            }
            ComplianceDiscoveryError::MappingFileParseFailure { .. } => {
                "Mapping File Parse Error".to_string()
            }
            ComplianceDiscoveryError::NetworkFailure { .. } => {
                "Network Connection Error".to_string()
            }
            ComplianceDiscoveryError::RetryableFailure { .. } => {
                "Compliance Discovery Failed".to_string()
            }
        }
    }

    fn get_error_description(&self, error: &ComplianceDiscoveryError) -> String {
        match error {
            ComplianceDiscoveryError::GitHubApiFailure { message, .. } => {
                format!("Unable to connect to GitHub API: {}", message)
            }
            ComplianceDiscoveryError::InvalidRepositoryStructure { message } => {
                format!("Repository structure is invalid: {}", message)
            }
            ComplianceDiscoveryError::MappingFileDownloadFailure { failed_files, errors } => {
                let mut desc = format!("Failed to download mapping files: {}", failed_files.join(", "));
                if !errors.is_empty() {
                    desc.push_str(&format!("\n\nErrors:\n{}", errors.join("\n")));
                }
                desc
            }
            ComplianceDiscoveryError::MappingFileParseFailure { file, error } => {
                format!("Unable to parse mapping file '{}': {}", file, error)
            }
            ComplianceDiscoveryError::NetworkFailure { error } => {
                format!("Network connection failed: {}", error)
            }
            ComplianceDiscoveryError::RetryableFailure { message } => message.clone(),
        }
    }

    fn can_retry(&self, error: &ComplianceDiscoveryError) -> bool {
        match error {
            ComplianceDiscoveryError::GitHubApiFailure { .. } => true,
            ComplianceDiscoveryError::InvalidRepositoryStructure { .. } => false,
            ComplianceDiscoveryError::MappingFileDownloadFailure { .. } => true,
            ComplianceDiscoveryError::MappingFileParseFailure { .. } => false,
            ComplianceDiscoveryError::NetworkFailure { .. } => true,
            ComplianceDiscoveryError::RetryableFailure { .. } => true,
        }
    }

}

impl FocusableWindow for ComplianceErrorWindow {
    type ShowParams = ();

    fn window_id(&self) -> &'static str {
        "compliance_error"
    }

    fn window_title(&self) -> String {
        "Compliance Discovery Error".to_string()
    }

    fn is_open(&self) -> bool {
        self.visible
    }

    fn show_with_focus(&mut self, ctx: &egui::Context, _params: (), bring_to_front: bool) {
        if !self.visible {
            return;
        }

        let available_rect = ctx.available_rect();
        let mut window = egui::Window::new("Compliance Discovery Error")
            .movable(true)
            .resizable(true)
            .default_size([500.0, 400.0])
            .min_width(400.0)
            .min_height(300.0)
            .max_width(available_rect.width() * 0.8)
            .max_height(available_rect.height() * 0.8)
            .collapsible(false);

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        let mut is_open = self.visible;
        window.open(&mut is_open).show(ctx, |ui| {
            if let Some(error) = &self.error {
                // Error header with icon and title
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.get_error_icon(error)).size(24.0));
                    ui.label(
                        RichText::new(self.get_error_title(error))
                            .color(Color32::from_rgb(220, 100, 50))
                            .strong()
                            .size(16.0),
                    );
                });

                ui.separator();

                // Error description
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Error Details:")
                            .strong()
                            .color(Color32::LIGHT_GRAY),
                    );
                    
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(self.get_error_description(error))
                                    .color(Color32::WHITE)
                                    .size(12.0),
                            );
                        });
                });

                ui.separator();

                // What this means section
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("What This Means:")
                            .strong()
                            .color(Color32::LIGHT_GRAY),
                    );
                    ui.label(
                        RichText::new("CloudFormation Guard validation is disabled because compliance programs could not be loaded from GitHub. No compliance checks will be performed until this is resolved.")
                            .color(Color32::from_rgb(255, 220, 100))
                            .size(12.0),
                    );
                });

                ui.separator();

                // Actions section
                let can_retry = self.can_retry(error);
                let mut retry_clicked = false;
                let mut close_clicked = false;

                ui.horizontal(|ui| {
                    // Retry button (if error is retryable)
                    if can_retry {
                        let retry_button = egui::Button::new(
                            RichText::new("🔄 Retry Discovery")
                                .color(Color32::WHITE)
                        )
                        .fill(Color32::from_rgb(70, 130, 180));

                        if ui.add(retry_button).clicked() {
                            retry_clicked = true;
                        }
                    } else {
                        ui.label(
                            RichText::new("⚠️ This error cannot be automatically retried")
                                .color(Color32::from_rgb(200, 200, 100))
                                .italics(),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close_clicked = true;
                        }
                    });
                });

                // Handle actions outside the closure
                if retry_clicked {
                    self.retry_requested = true;
                }
                if close_clicked {
                    self.hide();
                }


                // Troubleshooting section
                ui.separator();
                ui.collapsing("Troubleshooting", |ui| {
                    ui.label("Common solutions:");
                    ui.label("• Check your internet connection");
                    ui.label("• Verify GitHub.com is accessible");
                    ui.label("• Check if you're behind a corporate firewall");
                    ui.label("• Try again later if GitHub is experiencing issues");
                    ui.label("• Contact your system administrator if problems persist");
                });
            }
        });

        if !is_open {
            self.hide();
        }
    }
}