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

    #[allow(dead_code)]
    fn should_show_violation(&self, violation: &GuardViolation) -> bool {
        let severity_match = self
            .show_severity_filter
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

        let available_rect = ctx.available_rect();
        let mut window = egui::Window::new("CloudFormation Guard Violations")
            .movable(true)
            .resizable(true)
            .default_size([900.0, 700.0])
            .min_width(600.0)
            .min_height(400.0)
            .max_height(available_rect.height() * 0.95)
            .collapsible(true);

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        let mut is_open = self.visible;
        let _result = window.open(&mut is_open).show(ctx, |ui| {
            if let Some(validation) = self.validation_result.clone() {
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
                        self.show_severity_filter.entry(ViolationSeverity::Critical).or_insert(true),
                        RichText::new("🔴 Critical").color(Color32::from_rgb(200, 50, 50)),
                    );
                    ui.checkbox(
                        self.show_severity_filter.entry(ViolationSeverity::High).or_insert(true),
                        RichText::new("🟠 High").color(Color32::from_rgb(220, 100, 50)),
                    );
                    ui.checkbox(
                        self.show_severity_filter.entry(ViolationSeverity::Medium).or_insert(true),
                        RichText::new("🟡 Medium").color(Color32::from_rgb(220, 180, 50)),
                    );
                    ui.checkbox(
                        self.show_severity_filter.entry(ViolationSeverity::Low).or_insert(true),
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

                // Rules organized in a tree structure - calculate available space like Bridge window
                let current_window_height = ui.available_height();
                let footer_area_height = 80.0; // Reserve space for footer buttons
                let max_scroll_height = (current_window_height - footer_area_height).min(600.0);
                
                let scroll_area = egui::ScrollArea::vertical()
                    .id_salt("guard_violations_scroll")
                    .auto_shrink([false, false]) // Don't auto-shrink - let user control window size
                    .max_height(max_scroll_height); // Prevent expansion beyond available space

                let _scroll_response = scroll_area.show(ui, |ui| {
                    // Set a fixed width to prevent content from expanding the window
                    let available_width = ui.available_width();
                    ui.set_max_width(available_width);
                    self.show_rules_tree(ui, &validation);
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
        self.visible = is_open;
    }
}

impl GuardViolationsWindow {
    /// Display rules organized in a tree structure by status
    fn show_rules_tree(&mut self, ui: &mut egui::Ui, validation: &GuardValidation) {
        let rule_results = &validation.rule_results;
        
        // Not Applicable Rules Section
        if !rule_results.not_applicable_rules.is_empty() {
            ui.collapsing(
                RichText::new(format!("🔘 Not Applicable Rules ({})", rule_results.not_applicable_rules.len()))
                    .color(Color32::from_rgb(120, 120, 120))
                    .strong()
                    .size(15.0),
                |ui| {
                    ui.label(
                        RichText::new("These rules are part of the compliance program but don't apply to resources in this template.")
                            .color(Color32::from_rgb(140, 140, 140))
                            .size(11.0)
                            .italics()
                    );
                    ui.add_space(5.0);
                    
                    for rule in &rule_results.not_applicable_rules {
                        if self.rule_matches_severity_filter(&rule.severity) {
                            self.show_rule_item(ui, rule, "🔘", Color32::from_rgb(120, 120, 120), false);
                        }
                    }
                },
            );
            ui.add_space(8.0);
        }

        // Compliant Rules Section
        if !rule_results.compliant_rules.is_empty() {
            ui.collapsing(
                RichText::new(format!("✅ Compliant Rules ({})", rule_results.compliant_rules.len()))
                    .color(Color32::from_rgb(50, 200, 80))
                    .strong()
                    .size(15.0),
                |ui| {
                    ui.label(
                        RichText::new("These rules were evaluated and passed successfully.")
                            .color(Color32::from_rgb(70, 180, 90))
                            .size(11.0)
                            .italics()
                    );
                    ui.add_space(5.0);
                    
                    for rule in &rule_results.compliant_rules {
                        if self.rule_matches_severity_filter(&rule.severity) {
                            self.show_rule_item(ui, rule, "✅", Color32::from_rgb(50, 200, 80), true);
                        }
                    }
                },
            );
            ui.add_space(8.0);
        }

        // Violations Section - Always show if there are violation rules
        if !rule_results.violation_rules.is_empty() {
            ui.collapsing(
                RichText::new(format!("❌ Violations ({})", rule_results.violation_rules.len()))
                    .color(Color32::from_rgb(200, 50, 50))
                    .strong()
                    .size(15.0),
                |ui| {
                    ui.label(
                        RichText::new("These rules failed validation and require attention.")
                            .color(Color32::from_rgb(180, 70, 70))
                            .size(11.0)
                            .italics()
                    );
                    ui.add_space(5.0);
                    
                    if !self.show_non_exempted {
                        ui.label(
                            RichText::new("ℹ️ Active violations are currently hidden by filter settings.")
                                .color(Color32::from_rgb(150, 150, 150))
                                .size(11.0)
                                .italics()
                        );
                    } else {
                        for rule in &rule_results.violation_rules {
                            if self.rule_matches_severity_filter(&rule.severity) {
                                self.show_rule_item(ui, rule, "❌", Color32::from_rgb(200, 50, 50), true);
                                
                                // Show related violations
                                let related_violations: Vec<_> = validation.violations.iter()
                                    .filter(|v| v.rule_name == rule.name && !v.exempted)
                                    .collect();
                                
                                if !related_violations.is_empty() {
                                    ui.indent("violations", |ui| {
                                        for violation in related_violations {
                                            ui.horizontal(|ui| {
                                                ui.label("  🔸");
                                                ui.label(format!("Resource: {}", violation.resource_name));
                                                ui.separator();
                                                ui.label(
                                                    RichText::new(&violation.message)
                                                        .size(11.0)
                                                        .color(Color32::LIGHT_GRAY)
                                                );
                                            });
                                        }
                                    });
                                    ui.add_space(3.0);
                                }
                            }
                        }
                    }
                },
            );
            ui.add_space(8.0);
        }

        // Exempted Rules Section - Always show if there are exempted rules
        if !rule_results.exempted_rules.is_empty() {
            ui.collapsing(
                RichText::new(format!("⚠️ Exempted Rules ({})", rule_results.exempted_rules.len()))
                    .color(Color32::from_rgb(100, 150, 200))
                    .strong()
                    .size(15.0),
                |ui| {
                    ui.label(
                        RichText::new("These rules had violations but are exempted via CloudFormation Metadata.")
                            .color(Color32::from_rgb(120, 140, 180))
                            .size(11.0)
                            .italics()
                    );
                    ui.add_space(5.0);
                    
                    if !self.show_exempted {
                        ui.label(
                            RichText::new("ℹ️ Exempted violations are currently hidden by filter settings.")
                                .color(Color32::from_rgb(150, 150, 150))
                                .size(11.0)
                                .italics()
                        );
                    } else {
                        for rule in &rule_results.exempted_rules {
                            if self.rule_matches_severity_filter(&rule.severity) {
                                self.show_rule_item(ui, rule, "⚠️", Color32::from_rgb(100, 150, 200), true);
                                
                                // Show exempted violations
                                let exempted_violations: Vec<_> = validation.violations.iter()
                                    .filter(|v| v.rule_name == rule.name && v.exempted)
                                    .collect();
                                
                                if !exempted_violations.is_empty() {
                                    ui.indent("exemptions", |ui| {
                                        for violation in exempted_violations {
                                            ui.horizontal(|ui| {
                                                ui.label("  ⚠️");
                                                ui.label(format!("Resource: {}", violation.resource_name));
                                                if let Some(reason) = &violation.exemption_reason {
                                                    ui.separator();
                                                    ui.label(
                                                        RichText::new(format!("Reason: {}", reason))
                                                            .size(11.0)
                                                            .color(Color32::from_rgb(120, 140, 180))
                                                            .italics()
                                                    );
                                                }
                                            });
                                        }
                                    });
                                    ui.add_space(3.0);
                                }
                            }
                        }
                    }
                },
            );
        }
        
        // Show message if no rules match current filters
        if self.is_filtered_empty(rule_results) {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("No rules match the current severity and status filters.")
                        .color(Color32::from_rgb(150, 150, 150))
                        .size(14.0)
                        .italics(),
                );
            });
        }
    }

    /// Display a single rule item
    fn show_rule_item(&self, ui: &mut egui::Ui, rule: &crate::app::cfn_guard::GuardRule, icon: &str, color: Color32, show_details: bool) {
        ui.horizontal(|ui| {
            ui.label(icon);
            ui.label(
                RichText::new(&rule.name)
                    .color(color)
                    .strong()
                    .size(12.0)
            );
            
            if show_details {
                ui.separator();
                ui.label(self.get_severity_icon(&rule.severity));
                ui.label(
                    RichText::new(format!("{:?}", rule.severity))
                        .color(self.get_severity_color(&rule.severity))
                        .size(10.0)
                );
                
                if rule.applied_resources > 0 {
                    ui.separator();
                    ui.label(
                        RichText::new(format!("{} resources", rule.applied_resources))
                            .size(10.0)
                            .color(Color32::GRAY)
                    );
                }
            }
        });
        
        // Description
        ui.label(
            RichText::new(&rule.description)
                .size(11.0)
                .color(Color32::LIGHT_GRAY)
                .italics()
        );
        
        // Resource types
        if !rule.resource_types.is_empty() {
            ui.horizontal(|ui| {
                ui.label("Applies to:");
                for (i, resource_type) in rule.resource_types.iter().enumerate() {
                    if i > 0 { ui.label(","); }
                    ui.label(
                        RichText::new(resource_type)
                            .size(10.0)
                            .color(Color32::from_rgb(100, 170, 255))
                            .monospace()
                    );
                }
            });
        }
        
        ui.add_space(6.0);
    }

    /// Check if rule matches current severity filter
    fn rule_matches_severity_filter(&self, severity: &ViolationSeverity) -> bool {
        *self.show_severity_filter.get(severity).unwrap_or(&true)
    }

    /// Check if current filters result in empty display
    fn is_filtered_empty(&self, rule_results: &crate::app::cfn_guard::GuardRuleResults) -> bool {
        let has_not_applicable = !rule_results.not_applicable_rules.is_empty() && 
            rule_results.not_applicable_rules.iter().any(|r| self.rule_matches_severity_filter(&r.severity));
        let has_compliant = !rule_results.compliant_rules.is_empty() && 
            rule_results.compliant_rules.iter().any(|r| self.rule_matches_severity_filter(&r.severity));
        let has_violations = !rule_results.violation_rules.is_empty() && self.show_non_exempted &&
            rule_results.violation_rules.iter().any(|r| self.rule_matches_severity_filter(&r.severity));
        let has_exempted = !rule_results.exempted_rules.is_empty() && self.show_exempted &&
            rule_results.exempted_rules.iter().any(|r| self.rule_matches_severity_filter(&r.severity));
            
        !(has_not_applicable || has_compliant || has_violations || has_exempted)
    }
}
