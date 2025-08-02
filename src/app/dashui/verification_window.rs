use super::window_focus::FocusableWindow;
use egui::RichText;
use std::collections::HashMap;

#[derive(Default)]
pub struct VerificationWindow {
    pub visible: bool,
    pub discrepancies: Vec<String>,
    pub template_name: String,
    grouped_discrepancies: HashMap<String, Vec<String>>,
}

impl VerificationWindow {
    pub fn show(&mut self, imported_file: &str, discrepancies: Vec<String>) {
        self.visible = true;
        self.template_name = imported_file.to_string();

        // Group discrepancies once when showing the window
        self.grouped_discrepancies.clear();
        for discrepancy in &discrepancies {
            let section = if discrepancy.contains("AWSTemplateFormatVersion") {
                "Template Format Version"
            } else if discrepancy.contains("Description") {
                "Description"
            } else if discrepancy.contains("Transform") {
                "Transform"
            } else if discrepancy.contains("parameter") {
                "Parameters"
            } else if discrepancy.contains("mapping") {
                "Mappings"
            } else if discrepancy.contains("condition") {
                "Conditions"
            } else if discrepancy.contains("resource") {
                "Resources"
            } else if discrepancy.contains("output") {
                "Outputs"
            } else if discrepancy.contains("metadata") {
                "Metadata"
            } else if discrepancy.contains("rule") {
                "Rules"
            } else {
                "Other"
            };

            self.grouped_discrepancies
                .entry(section.to_string())
                .or_default()
                .push(discrepancy.clone());
        }

        self.discrepancies = discrepancies;
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        egui::Window::new("CloudFormation Import Verification Results")
            .id(egui::Id::new("cfn_verification_window"))
            .resizable(true)
            .default_size([600.0, 400.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(format!("Template: {}", self.template_name));
                    ui.separator();

                    if self.discrepancies.is_empty() {
                        ui.colored_label(
                            egui::Color32::from_rgb(0, 255, 0),
                            RichText::new("✓ All sections verified successfully!").size(16.0),
                        );
                        ui.label("The imported template matches the saved template perfectly.");
                    } else {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 200, 0),
                            RichText::new(format!("⚠ Found {} discrepancies", self.discrepancies.len())).size(16.0),
                        );
                        ui.separator();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (section, issues) in &self.grouped_discrepancies {
                                ui.collapsing(format!("{} ({})", section, issues.len()), |ui| {
                                    for issue in issues {
                                        ui.label(format!("• {}", issue));
                                    }
                                });
                            }
                        });
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.visible = false;
                            self.discrepancies.clear();
                            self.grouped_discrepancies.clear();
                        }

                        if !self.discrepancies.is_empty() && ui.button("Copy to Clipboard").clicked() {
                            let text = format!(
                                "CloudFormation Import Verification Results\nTemplate: {}\n\nDiscrepancies:\n{}",
                                self.template_name,
                                self.discrepancies.join("\n")
                            );
                            ctx.copy_text(text);
                        }
                    });
                });
            });
    }

    /// Show the verification window with focus capability
    pub fn ui_with_focus(&mut self, ctx: &egui::Context, bring_to_front: bool) {
        if !self.visible {
            return;
        }

        let mut window = egui::Window::new("CloudFormation Import Verification Results")
            .id(egui::Id::new("cfn_verification_window"))
            .resizable(true)
            .default_size([600.0, 400.0]);

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(format!("Template: {}", self.template_name));
                ui.separator();

                if self.discrepancies.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(0, 255, 0),
                        RichText::new("✓ All sections verified successfully!").size(16.0),
                    );
                    ui.label("The imported template matches the saved template perfectly.");
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 200, 0),
                        RichText::new(format!("⚠ Found {} discrepancies", self.discrepancies.len())).size(16.0),
                    );
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (section, issues) in &self.grouped_discrepancies {
                            ui.collapsing(format!("{} ({})", section, issues.len()), |ui| {
                                for issue in issues {
                                    ui.label(format!("• {}", issue));
                                }
                            });
                        }
                    });
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.visible = false;
                        self.discrepancies.clear();
                        self.grouped_discrepancies.clear();
                    }

                    if !self.discrepancies.is_empty() && ui.button("Copy to Clipboard").clicked() {
                        let text = format!(
                            "CloudFormation Import Verification Results\nTemplate: {}\n\nDiscrepancies:\n{}",
                            self.template_name,
                            self.discrepancies.join("\n")
                        );
                        ctx.copy_text(text);
                    }
                });
            });
        });
    }
}

impl FocusableWindow for VerificationWindow {
    type ShowParams = super::window_focus::SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "verification_window"
    }

    fn window_title(&self) -> String {
        "CloudFormation Import Verification Results".to_string()
    }

    fn is_open(&self) -> bool {
        self.visible
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        _params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        self.ui_with_focus(ctx, bring_to_front);
    }
}
