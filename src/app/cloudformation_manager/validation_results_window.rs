use super::manager::ValidationResult;
use egui::{self, Color32, Context, RichText, ScrollArea};

#[derive(Default)]
pub struct ValidationResultsWindow {
    pub open: bool,
    pub result: Option<ValidationResult>,
    pub show_details: bool,
}

impl ValidationResultsWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            result: None,
            show_details: false,
        }
    }

    pub fn show_result(&mut self, result: ValidationResult) {
        self.result = Some(result);
        self.open = true;
        self.show_details = false;
    }

    pub fn show(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        let mut open = self.open;
        egui::Window::new("CloudFormation Template Validation Results")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .min_width(500.0)
            .min_height(300.0)
            .show(ctx, |ui| {
                if let Some(result) = self.result.clone() {
                    self.show_validation_result(ui, &result);
                } else {
                    ui.label("No validation result available");
                }
            });

        self.open = open;
    }

    fn show_validation_result(&mut self, ui: &mut egui::Ui, result: &ValidationResult) {
        // Header with overall status
        ui.horizontal(|ui| {
            if result.is_valid {
                ui.colored_label(Color32::from_rgb(40, 180, 40), "✓ Template is valid");
            } else {
                ui.colored_label(
                    Color32::from_rgb(220, 50, 50),
                    "✗ Template validation failed",
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Copy to Clipboard").clicked() {
                    let summary = self.create_clipboard_summary(result);
                    ui.ctx().copy_text(summary);
                }
            });
        });

        // Template description if available
        if let Some(description) = &result.description {
            ui.group(|ui| {
                ui.label(RichText::new("Template Description").strong());
                ui.label(description);
            });
            ui.add_space(5.0);
        }

        // Error section
        if !result.errors.is_empty() {
            ui.group(|ui| {
                ui.label(
                    RichText::new(format!("Errors ({})", result.errors.len()))
                        .color(Color32::from_rgb(220, 50, 50))
                        .strong(),
                );

                ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    for (i, error) in result.errors.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(220, 50, 50), "•");
                            ui.vertical(|ui| {
                                // Main error message
                                ui.label(&error.message);

                                // Show detailed error information if available
                                ui.horizontal(|ui| {
                                    if let Some(code) = &error.code {
                                        ui.label(
                                            RichText::new(format!("Code: {}", code))
                                                .size(11.0)
                                                .weak(),
                                        );
                                    }
                                    if let Some(line) = error.line_number {
                                        ui.label(
                                            RichText::new(format!("Line: {}", line))
                                                .size(11.0)
                                                .weak()
                                                .color(Color32::from_rgb(150, 150, 150)),
                                        );
                                    }
                                    if let Some(resource) = &error.resource_name {
                                        ui.label(
                                            RichText::new(format!("Resource: {}", resource))
                                                .size(11.0)
                                                .weak()
                                                .color(Color32::from_rgb(100, 150, 200)),
                                        );
                                    }
                                });

                                if let Some(property_path) = &error.property_path {
                                    ui.label(
                                        RichText::new(format!("Property: {}", property_path))
                                            .size(10.0)
                                            .weak()
                                            .italics(),
                                    );
                                }

                                if let Some(suggestion) = &error.suggestion {
                                    ui.label(
                                        RichText::new(format!("💡 {}", suggestion))
                                            .size(10.0)
                                            .color(Color32::from_rgb(100, 200, 100)),
                                    );
                                }
                            });
                        });

                        if i < result.errors.len() - 1 {
                            ui.separator();
                        }
                    }
                });
            });
            ui.add_space(5.0);
        }

        // Warning section
        if !result.warnings.is_empty() {
            ui.group(|ui| {
                ui.label(
                    RichText::new(format!("Warnings ({})", result.warnings.len()))
                        .color(Color32::from_rgb(255, 150, 0))
                        .strong(),
                );

                ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                    for (i, warning) in result.warnings.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(255, 150, 0), "•");
                            ui.vertical(|ui| {
                                // Main warning message
                                ui.label(&warning.message);

                                // Show detailed warning information if available
                                ui.horizontal(|ui| {
                                    if let Some(code) = &warning.code {
                                        ui.label(
                                            RichText::new(format!("Code: {}", code))
                                                .size(11.0)
                                                .weak(),
                                        );
                                    }
                                    if let Some(line) = warning.line_number {
                                        ui.label(
                                            RichText::new(format!("Line: {}", line))
                                                .size(11.0)
                                                .weak()
                                                .color(Color32::from_rgb(150, 150, 150)),
                                        );
                                    }
                                    if let Some(resource) = &warning.resource_name {
                                        ui.label(
                                            RichText::new(format!("Resource: {}", resource))
                                                .size(11.0)
                                                .weak()
                                                .color(Color32::from_rgb(100, 150, 200)),
                                        );
                                    }
                                });

                                if let Some(property_path) = &warning.property_path {
                                    ui.label(
                                        RichText::new(format!("Property: {}", property_path))
                                            .size(10.0)
                                            .weak()
                                            .italics(),
                                    );
                                }

                                if let Some(suggestion) = &warning.suggestion {
                                    ui.label(
                                        RichText::new(format!("💡 {}", suggestion))
                                            .size(10.0)
                                            .color(Color32::from_rgb(100, 200, 100)),
                                    );
                                }
                            });
                        });

                        if i < result.warnings.len() - 1 {
                            ui.separator();
                        }
                    }
                });
            });
            ui.add_space(5.0);
        }

        // Parameters section
        if !result.parameters.is_empty() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Template Parameters ({})", result.parameters.len()))
                            .strong(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .small_button(if self.show_details {
                                "Hide Details"
                            } else {
                                "Show Details"
                            })
                            .clicked()
                        {
                            self.show_details = !self.show_details;
                        }
                    });
                });

                if self.show_details {
                    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (i, param) in result.parameters.iter().enumerate() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(&param.parameter_key).strong());
                                    ui.label(format!("({})", param.parameter_type));
                                    if param.no_echo {
                                        ui.colored_label(Color32::from_rgb(255, 150, 0), "NoEcho");
                                    }
                                });

                                if let Some(description) = &param.description {
                                    ui.label(RichText::new(description).size(12.0).weak());
                                }

                                if let Some(default) = &param.default_value {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Default:").size(11.0));
                                        ui.label(RichText::new(default).size(11.0).monospace());
                                    });
                                }

                                if let Some(allowed_values) = &param.allowed_values {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Allowed:").size(11.0));
                                        ui.label(
                                            RichText::new(allowed_values.join(", "))
                                                .size(11.0)
                                                .monospace(),
                                        );
                                    });
                                }

                                if let Some(pattern) = &param.allowed_pattern {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Pattern:").size(11.0));
                                        ui.label(RichText::new(pattern).size(11.0).monospace());
                                    });
                                }
                            });

                            if i < result.parameters.len() - 1 {
                                ui.add_space(2.0);
                            }
                        }
                    });
                } else {
                    // Show just parameter names
                    ui.horizontal_wrapped(|ui| {
                        for param in &result.parameters {
                            ui.label(RichText::new(&param.parameter_key).monospace());
                        }
                    });
                }
            });
        }

        // Action buttons
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Close").clicked() {
                self.open = false;
            }

            if result.is_valid
                && !result.parameters.is_empty()
                && ui.button("Deploy Template").clicked()
            {
                // TODO: Transition to deploy mode with these parameters
                tracing::info!("Deploy button clicked - TODO: implement deployment flow");
            }
        });
    }

    fn create_clipboard_summary(&self, result: &ValidationResult) -> String {
        let mut summary = String::new();

        summary.push_str("CloudFormation Template Validation Results\n");
        summary.push_str("==========================================\n\n");

        if result.is_valid {
            summary.push_str("✓ Template is VALID\n\n");
        } else {
            summary.push_str("✗ Template validation FAILED\n\n");
        }

        if let Some(description) = &result.description {
            summary.push_str(&format!("Description: {}\n\n", description));
        }

        if !result.errors.is_empty() {
            summary.push_str(&format!("Errors ({}):\n", result.errors.len()));
            for error in &result.errors {
                summary.push_str(&format!("• {}\n", error.message));
                if let Some(code) = &error.code {
                    summary.push_str(&format!("  Code: {}\n", code));
                }
            }
            summary.push('\n');
        }

        if !result.warnings.is_empty() {
            summary.push_str(&format!("Warnings ({}):\n", result.warnings.len()));
            for warning in &result.warnings {
                summary.push_str(&format!("• {}\n", warning.message));
                if let Some(code) = &warning.code {
                    summary.push_str(&format!("  Code: {}\n", code));
                }
            }
            summary.push('\n');
        }

        if !result.parameters.is_empty() {
            summary.push_str(&format!("Parameters ({}):\n", result.parameters.len()));
            for param in &result.parameters {
                summary.push_str(&format!(
                    "• {} ({})",
                    param.parameter_key, param.parameter_type
                ));
                if param.no_echo {
                    summary.push_str(" [NoEcho]");
                }
                summary.push('\n');

                if let Some(description) = &param.description {
                    summary.push_str(&format!("  Description: {}\n", description));
                }
                if let Some(default) = &param.default_value {
                    summary.push_str(&format!("  Default: {}\n", default));
                }
            }
        }

        summary
    }
}
