//! CloudFormation Guard Violations Window
//!
//! This window displays CloudFormation Guard validation violations with detailed information
//! about each violation, including rule names, resource names, messages, and severity levels.

use super::window_focus::FocusableWindow;
use crate::app::cfn_guard::{GuardValidation, GuardViolation, ViolationSeverity};
use eframe::egui;
use egui::{Color32, RichText};
use std::collections::HashMap;

#[derive(Default)]
pub struct GuardViolationsWindow {
    pub visible: bool,
    pub validation_result: Option<GuardValidation>,
    pub template_name: String,
    grouped_violations: HashMap<String, Vec<GuardViolation>>,
    selected_violation: Option<usize>,
    show_severity_filter: HashMap<ViolationSeverity, bool>,
    show_exempted: bool,
    show_non_exempted: bool,
}

impl GuardViolationsWindow {
    pub fn new() -> Self {
        let mut show_severity_filter = HashMap::new();
        show_severity_filter.insert(ViolationSeverity::Critical, true);
        show_severity_filter.insert(ViolationSeverity::High, true);
        show_severity_filter.insert(ViolationSeverity::Medium, true);
        show_severity_filter.insert(ViolationSeverity::Low, true);

        Self {
            visible: false,
            validation_result: None,
            template_name: String::new(),
            grouped_violations: HashMap::new(),
            selected_violation: None,
            show_severity_filter,
            show_exempted: true,
            show_non_exempted: true,
        }
    }

    pub fn show(&mut self, template_name: &str, validation_result: GuardValidation) {
        self.visible = true;
        self.template_name = template_name.to_string();
        self.validation_result = Some(validation_result.clone());

        // Group violations by resource name
        self.grouped_violations.clear();
        for violation in &validation_result.violations {
            self.grouped_violations
                .entry(violation.resource_name.clone())
                .or_default()
                .push(violation.clone());
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.validation_result = None;
        self.selected_violation = None;
        self.grouped_violations.clear();
    }

    fn get_severity_color(&self, severity: &ViolationSeverity) -> Color32 {
        match severity {
            ViolationSeverity::Critical => Color32::from_rgb(200, 50, 50),
            ViolationSeverity::High => Color32::from_rgb(220, 100, 50),
            ViolationSeverity::Medium => Color32::from_rgb(220, 180, 50),
            ViolationSeverity::Low => Color32::from_rgb(100, 150, 220),
        }
    }

    fn get_severity_icon(&self, severity: &ViolationSeverity) -> &'static str {
        match severity {
            ViolationSeverity::Critical => "🔴",
            ViolationSeverity::High => "🟠",
            ViolationSeverity::Medium => "🟡",
            ViolationSeverity::Low => "🔵",
        }
    }

    fn should_show_violation(&self, violation: &GuardViolation) -> bool {
        let severity_match = self.show_severity_filter
            .get(&violation.severity)
            .copied()
            .unwrap_or(true);
            
        let exemption_match = if violation.exempted {
            self.show_exempted
        } else {
            self.show_non_exempted
        };
        
        severity_match && exemption_match
    }
}

impl FocusableWindow for GuardViolationsWindow {
    type ShowParams = ();

    fn window_id(&self) -> &'static str {
        "guard_violations"
    }

    fn window_title(&self) -> String {
        "Guard Violations".to_string()
    }

    fn is_open(&self) -> bool {
        self.visible
    }

    fn show_with_focus(&mut self, ctx: &egui::Context, _params: (), bring_to_front: bool) {
        if !self.visible {
            return;
        }

        let mut window = egui::Window::new("CloudFormation Guard Violations")
            .resizable(true)
            .default_width(800.0)
            .default_height(600.0)
            .collapsible(true);

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            if let Some(validation) = &self.validation_result {
                // Header with summary
                ui.horizontal(|ui| {
                    ui.heading("Validation Results");
                    ui.separator();
                    ui.label(format!("Template: {}", self.template_name));
                });

                ui.separator();

                // Summary information
                ui.horizontal(|ui| {
                    let exempted_count = validation.violations.iter().filter(|v| v.exempted).count();
                    let active_violations = validation.violations.len() - exempted_count;
                    
                    let status_color = if validation.compliant {
                        Color32::from_rgb(50, 200, 80)
                    } else {
                        Color32::from_rgb(200, 50, 50)
                    };

                    let status_text = if validation.compliant {
                        "✅ Compliant"
                    } else {
                        "❌ Non-Compliant"
                    };

                    ui.label(
                        RichText::new(status_text)
                            .color(status_color)
                            .strong()
                            .size(14.0),
                    );

                    ui.separator();
                    ui.label(format!("Total Rules: {}", validation.total_rules));
                    ui.separator();
                    ui.label(format!("Active Violations: {}", active_violations));
                    ui.separator();
                    if exempted_count > 0 {
                        ui.label(
                            RichText::new(format!("Exempted: {}", exempted_count))
                                .color(Color32::from_rgb(100, 150, 200))
                        );
                        ui.separator();
                    }
                    ui.label(format!("Total: {}", validation.violations.len()));
                });

                ui.separator();

                // Severity filter
                ui.horizontal(|ui| {
                    ui.label("Show Severity:");
                    ui.checkbox(
                        &mut self.show_severity_filter.entry(ViolationSeverity::Critical).or_insert(true),
                        RichText::new("🔴 Critical").color(Color32::from_rgb(200, 50, 50)),
                    );
                    ui.checkbox(
                        &mut self.show_severity_filter.entry(ViolationSeverity::High).or_insert(true),
                        RichText::new("🟠 High").color(Color32::from_rgb(220, 100, 50)),
                    );
                    ui.checkbox(
                        &mut self.show_severity_filter.entry(ViolationSeverity::Medium).or_insert(true),
                        RichText::new("🟡 Medium").color(Color32::from_rgb(220, 180, 50)),
                    );
                    ui.checkbox(
                        &mut self.show_severity_filter.entry(ViolationSeverity::Low).or_insert(true),
                        RichText::new("🔵 Low").color(Color32::from_rgb(100, 150, 220)),
                    );
                });
                
                ui.horizontal(|ui| {
                    ui.label("Show Status:");
                    ui.checkbox(
                        &mut self.show_non_exempted,
                        RichText::new("🚨 Active Violations").color(Color32::from_rgb(200, 100, 100)),
                    );
                    ui.checkbox(
                        &mut self.show_exempted,
                        RichText::new("⚠️ Exempted Violations").color(Color32::from_rgb(100, 150, 200)),
                    );
                });

                ui.separator();

                // Violations list in a scroll area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if validation.violations.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    RichText::new("🎉 No violations found! Your CloudFormation template is compliant.")
                                        .color(Color32::from_rgb(50, 200, 80))
                                        .size(16.0)
                                        .strong(),
                                );
                            });
                        } else {
                            // Show violations grouped by resource
                            for (resource_name, violations) in &self.grouped_violations {
                                let filtered_violations: Vec<_> = violations
                                    .iter()
                                    .filter(|v| self.should_show_violation(v))
                                    .collect();

                                if filtered_violations.is_empty() {
                                    continue;
                                }

                                ui.collapsing(
                                    RichText::new(format!("Resource: {} ({} violations)", resource_name, filtered_violations.len()))
                                        .strong()
                                        .size(14.0),
                                    |ui| {
                                        for violation in filtered_violations {
                                            ui.group(|ui| {
                                                // Violation header
                                                ui.horizontal(|ui| {
                                                    ui.label(self.get_severity_icon(&violation.severity));
                                                    
                                                    // Show exemption status
                                                    if violation.exempted {
                                                        ui.label("⚠️");
                                                        ui.label(
                                                            RichText::new(&violation.rule_name)
                                                                .color(Color32::from_rgb(150, 150, 150))
                                                                .strikethrough()
                                                                .strong(),
                                                        );
                                                        ui.label(
                                                            RichText::new("EXEMPTED")
                                                                .color(Color32::from_rgb(100, 150, 200))
                                                                .size(10.0)
                                                                .strong(),
                                                        );
                                                    } else {
                                                        ui.label(
                                                            RichText::new(&violation.rule_name)
                                                                .color(self.get_severity_color(&violation.severity))
                                                                .strong(),
                                                        );
                                                    }
                                                    
                                                    ui.separator();
                                                    ui.label(format!("{:?}", violation.severity));
                                                });

                                                // Violation message
                                                ui.label(
                                                    RichText::new(&violation.message)
                                                        .size(12.0)
                                                        .color(if violation.exempted { 
                                                            Color32::from_rgb(120, 120, 120) 
                                                        } else { 
                                                            Color32::LIGHT_GRAY 
                                                        }),
                                                );
                                                
                                                // Show exemption reason if available
                                                if violation.exempted {
                                                    if let Some(reason) = &violation.exemption_reason {
                                                        ui.horizontal(|ui| {
                                                            ui.label("📋");
                                                            ui.label(
                                                                RichText::new(format!("Exemption reason: {}", reason))
                                                                    .size(11.0)
                                                                    .color(Color32::from_rgb(100, 150, 200))
                                                                    .italics(),
                                                            );
                                                        });
                                                    }
                                                }
                                                
                                                // Action buttons
                                                ui.horizontal(|ui| {
                                                    if violation.exempted {
                                                        if ui.small_button("Remove Exemption").clicked() {
                                                            // TODO: Remove exemption from template
                                                            log::info!("Remove exemption requested for rule: {} on resource: {}", 
                                                                     violation.rule_name, violation.resource_name);
                                                        }
                                                    } else {
                                                        if ui.small_button("Add Exemption").clicked() {
                                                            // TODO: Add exemption to template
                                                            log::info!("Add exemption requested for rule: {} on resource: {}", 
                                                                     violation.rule_name, violation.resource_name);
                                                        }
                                                    }
                                                });
                                            });
                                            ui.add_space(4.0);
                                        }
                                    },
                                );
                            }
                        }
                    });

                ui.separator();

                // Footer with actions
                let has_violations = !validation.violations.is_empty();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.hide();
                    }

                    ui.separator();

                    if ui.button("Re-validate").clicked() {
                        // TODO: Trigger re-validation
                        // This could send a signal to the main app to re-run validation
                    }

                    if has_violations {
                        ui.separator();
                        if ui.button("Export Report").clicked() {
                            // TODO: Export violations to a file
                            // This could generate a CSV or JSON report
                        }
                    }
                });
            }
        });
    }
}