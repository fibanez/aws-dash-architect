//! Verification window for comparing Dash cache with AWS CLI output.
//!
//! This module provides a UI window that allows users to verify that Dash's
//! cached resource data matches what the AWS CLI returns.
//!
//! CRITICAL: This performs FIELD-BY-FIELD property comparison, not just resource counting.

#![cfg(debug_assertions)]

use super::cli_commands::{check_cli_available, execute_cli_with_details, get_cli_command};
use super::credentials::CredentialCoordinator;
use super::state::{ResourceEntry, ResourceExplorerState};
use super::verification_results::{compare_resources_detailed, ResourceTypeResult, VerificationResults};
use egui::{self, Color32, Context, RichText, ScrollArea, Ui};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

// ============================================================================
// Theme-Aware Color Helpers
// ============================================================================

/// Get theme-aware error color (red in both light/dark themes)
fn error_color(ui: &Ui) -> Color32 {
    ui.visuals().error_fg_color
}

/// Get theme-aware warning color (yellow/orange in both light/dark themes)
fn warn_color(ui: &Ui) -> Color32 {
    ui.visuals().warn_fg_color
}

/// Get theme-aware success color (green that's visible in both themes)
fn success_color(ui: &Ui) -> Color32 {
    // egui doesn't have a built-in success color, so we create one that works in both themes
    // Use a green that's visible on both light and dark backgrounds
    if ui.visuals().dark_mode {
        Color32::from_rgb(100, 200, 100)  // Lighter green for dark mode
    } else {
        Color32::from_rgb(40, 150, 40)    // Darker green for light mode
    }
}

// ============================================================================
// Types
// ============================================================================

/// State of the verification process.
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationState {
    /// Waiting for user to start verification
    Idle,
    /// Verification is in progress
    Running,
    /// Verification completed
    Completed,
    /// Error occurred
    Error(String),
}

/// Verification window component.
pub struct VerificationWindow {
    pub open: bool,
    state: VerificationState,
    results: Option<VerificationResults>,
    cli_version: Option<String>,
    selected_account: Option<String>,
    selected_region: Option<String>,
    status_message: Option<String>,
}

impl Default for VerificationWindow {
    fn default() -> Self {
        Self {
            open: false,
            state: VerificationState::Idle,
            results: None,
            cli_version: None,
            selected_account: None,
            selected_region: None,
            status_message: None,
        }
    }
}

impl VerificationWindow {
    /// Create a new verification window.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if AWS CLI is available and store version.
    pub fn check_cli(&mut self) {
        match check_cli_available() {
            Ok(version) => {
                self.cli_version = Some(version);
                self.status_message = None;
            }
            Err(e) => {
                self.cli_version = None;
                self.status_message = Some(format!("AWS CLI not available: {}", e));
                self.state = VerificationState::Error(e.to_string());
            }
        }
    }

    /// Show the verification window.
    pub fn show(
        &mut self,
        ctx: &Context,
        explorer_state: &Arc<RwLock<ResourceExplorerState>>,
        credential_coordinator: Option<&Arc<CredentialCoordinator>>,
    ) {
        if !self.open {
            return;
        }

        // Check CLI on first open
        if self.cli_version.is_none() && !matches!(self.state, VerificationState::Error(_)) {
            self.check_cli();
        }

        let mut open = self.open;

        egui::Window::new("CLI Verification - Property Comparison")
            .open(&mut open)
            .resizable(true)
            .default_width(800.0)
            .default_height(600.0)
            .show(ctx, |ui| {
                self.render_content(ui, explorer_state, credential_coordinator);
            });

        self.open = open;
    }

    /// Render the window content.
    fn render_content(
        &mut self,
        ui: &mut egui::Ui,
        explorer_state: &Arc<RwLock<ResourceExplorerState>>,
        credential_coordinator: Option<&Arc<CredentialCoordinator>>,
    ) {
        // Header with CLI status
        ui.horizontal(|ui| {
            if let Some(ref version) = self.cli_version {
                ui.label(RichText::new("CLI: ").strong());
                ui.label(version);
            } else {
                ui.label(RichText::new("AWS CLI not detected").color(error_color(ui)));
            }
        });

        ui.separator();

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.label(RichText::new(msg).color(warn_color(ui)));
            ui.separator();
        }

        // Account/Region info from cache
        if let Some((account, region, resource_count, type_count)) = self.get_cache_info(explorer_state) {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Account: ").strong());
                ui.label(&account);
                ui.add_space(20.0);
                ui.label(RichText::new("Region: ").strong());
                ui.label(&region);
                ui.add_space(20.0);
                ui.label(format!("{} resources in {} types", resource_count, type_count));
            });

            self.selected_account = Some(account);
            self.selected_region = Some(region);
        } else {
            ui.label("No cached resources to verify. Run a query first.");
            return;
        }

        ui.separator();

        // Action buttons
        ui.horizontal(|ui| {
            let can_run = self.cli_version.is_some()
                && self.selected_account.is_some()
                && credential_coordinator.is_some()
                && !matches!(self.state, VerificationState::Running);

            if ui
                .add_enabled(can_run, egui::Button::new("Run Verification"))
                .clicked()
            {
                self.run_verification(explorer_state, credential_coordinator.unwrap());
            }

            if self.results.is_some() {
                if ui.button("Export Results").clicked() {
                    self.export_results();
                }

                if ui.button("Clear Results").clicked() {
                    self.results = None;
                    self.state = VerificationState::Idle;
                }
            }
        });

        ui.separator();

        // Results display
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                self.render_results(ui);
            });
    }

    /// Get info about what's in the cache.
    fn get_cache_info(
        &self,
        explorer_state: &Arc<RwLock<ResourceExplorerState>>,
    ) -> Option<(String, String, usize, usize)> {
        if let Ok(state) = explorer_state.try_read() {
            let resources = &state.resources;
            if resources.is_empty() {
                return None;
            }

            // Count unique resource types
            let type_count = resources.iter()
                .map(|r| &r.resource_type)
                .collect::<std::collections::HashSet<_>>()
                .len();

            // Get account/region from first resource
            if let Some(first) = resources.first() {
                let account = first.account_id.clone();
                let region = if first.region == "Global" {
                    "us-east-1".to_string()
                } else {
                    first.region.clone()
                };
                return Some((account, region, resources.len(), type_count));
            }
        }
        None
    }

    /// Run verification against CLI with field-by-field comparison.
    fn run_verification(
        &mut self,
        explorer_state: &Arc<RwLock<ResourceExplorerState>>,
        credential_coordinator: &Arc<CredentialCoordinator>,
    ) {
        self.state = VerificationState::Running;
        self.status_message = Some("Running verification with property comparison...".to_string());

        let account = match &self.selected_account {
            Some(a) => a.clone(),
            None => {
                self.state = VerificationState::Error("No account selected".to_string());
                return;
            }
        };

        let region = match &self.selected_region {
            Some(r) => r.clone(),
            None => {
                self.state = VerificationState::Error("No region selected".to_string());
                return;
            }
        };

        info!("[Verification] Starting verification for account {} region {}", account, region);

        // Get cached resources grouped by type
        let cached_by_type = match explorer_state.try_read() {
            Ok(state) => self.group_resources_by_type(&state.resources),
            Err(_) => {
                self.state = VerificationState::Error("Could not read cache".to_string());
                return;
            }
        };

        info!("[Verification] Found {} resource types to verify", cached_by_type.len());

        // Get credentials
        let creds = {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(credential_coordinator.get_credentials_for_account(&account)) {
                Ok(c) => c,
                Err(e) => {
                    self.state =
                        VerificationState::Error(format!("Failed to get credentials: {}", e));
                    return;
                }
            }
        };

        let mut results = VerificationResults::new(account.clone(), region.clone());

        // Verify each resource type that has a CLI mapping
        for (resource_type, dash_resources) in &cached_by_type {
            info!("[Verification] Processing {} with {} resources", resource_type, dash_resources.len());

            if let Some(cmd) = get_cli_command(resource_type) {
                let query_region = if cmd.is_global { "us-east-1" } else { &region };

                match execute_cli_with_details(&cmd, resource_type, &creds, query_region) {
                    Ok(cli_result) => {
                        if let Some(ref err) = cli_result.error {
                            error!("[Verification] CLI error for {}: {}", resource_type, err);
                            results.add_result(ResourceTypeResult {
                                resource_type: resource_type.clone(),
                                dash_count: dash_resources.len(),
                                cli_count: 0,
                                matched: false,
                                missing_in_dash: Vec::new(),
                                missing_in_cli: Vec::new(),
                                resource_comparisons: Vec::new(),
                                cli_execution: Some(cli_result.execution),
                                error: Some(err.clone()),
                                total_fields_compared: 0,
                                total_fields_matched: 0,
                                total_fields_mismatched: 0,
                            });
                        } else {
                            // Use detailed comparison with field-by-field checking
                            let result = compare_resources_detailed(
                                resource_type,
                                dash_resources,
                                &cli_result.resources_by_id,
                                &cli_result.resource_ids,
                                cli_result.execution,
                            );

                            info!(
                                "[Verification] {} - {} resources, {} fields compared, {} matched, {} mismatched",
                                resource_type,
                                result.dash_count,
                                result.total_fields_compared,
                                result.total_fields_matched,
                                result.total_fields_mismatched
                            );

                            results.add_result(result);
                        }
                    }
                    Err(e) => {
                        error!("[Verification] Failed to execute CLI for {}: {}", resource_type, e);
                        results.add_result(ResourceTypeResult {
                            resource_type: resource_type.clone(),
                            dash_count: dash_resources.len(),
                            cli_count: 0,
                            matched: false,
                            missing_in_dash: Vec::new(),
                            missing_in_cli: Vec::new(),
                            resource_comparisons: Vec::new(),
                            cli_execution: None,
                            error: Some(e.to_string()),
                            total_fields_compared: 0,
                            total_fields_matched: 0,
                            total_fields_mismatched: 0,
                        });
                    }
                }
            } else {
                debug!("[Verification] No CLI mapping for resource type: {}", resource_type);
            }
        }

        let total_fields = results.total_fields_compared();
        let matched_fields = results.total_fields_matched();
        let status_msg = format!(
            "Verification completed: {} fields compared, {} matched ({:.1}%)",
            total_fields,
            matched_fields,
            if total_fields > 0 { (matched_fields as f64 / total_fields as f64) * 100.0 } else { 100.0 }
        );

        info!("[Verification] {}", status_msg);

        self.results = Some(results);
        self.state = VerificationState::Completed;
        self.status_message = Some(status_msg);
    }

    /// Group resources by their type.
    fn group_resources_by_type(
        &self,
        resources: &[ResourceEntry],
    ) -> HashMap<String, Vec<ResourceEntry>> {
        let mut grouped: HashMap<String, Vec<ResourceEntry>> = HashMap::new();
        for resource in resources {
            grouped
                .entry(resource.resource_type.clone())
                .or_default()
                .push(resource.clone());
        }
        grouped
    }

    /// Render verification results.
    fn render_results(&self, ui: &mut egui::Ui) {
        match &self.state {
            VerificationState::Idle => {
                ui.label("Click 'Run Verification' to compare cached resources with AWS CLI.");
                ui.add_space(10.0);
                ui.label(RichText::new("This will:").strong());
                ui.label("1. Execute AWS CLI commands for each resource type");
                ui.label("2. Compare field-by-field values between Dash cache and CLI output");
                ui.label("3. Report exact matches and mismatches for each property");
            }
            VerificationState::Running => {
                ui.spinner();
                ui.label("Verifying resources with field-by-field comparison...");
            }
            VerificationState::Error(e) => {
                ui.label(RichText::new(format!("Error: {}", e)).color(error_color(ui)));
            }
            VerificationState::Completed => {
                if let Some(ref results) = self.results {
                    // Overall summary
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Overall: ").strong());
                        let total = results.total_fields_compared();
                        let matched = results.total_fields_matched();
                        let pct = if total > 0 { (matched as f64 / total as f64) * 100.0 } else { 100.0 };

                        if pct >= 100.0 {
                            ui.label(RichText::new(format!("{} fields, 100% match", total)).color(success_color(ui)));
                        } else if pct >= 90.0 {
                            ui.label(RichText::new(format!("{} fields, {:.1}% match", total, pct)).color(warn_color(ui)));
                        } else {
                            ui.label(RichText::new(format!("{} fields, {:.1}% match", total, pct)).color(error_color(ui)));
                        }
                    });

                    ui.add_space(10.0);

                    // Per-type results
                    for result in &results.results {
                        self.render_single_result(ui, result);
                    }
                }
            }
        }
    }

    /// Render a single resource type result with field details.
    fn render_single_result(&self, ui: &mut egui::Ui, result: &ResourceTypeResult) {
        // Header line with theme-aware status color
        ui.horizontal(|ui| {
            let (status_text, status_color) = if result.error.is_some() {
                ("ERROR", error_color(ui))
            } else if result.matched {
                ("OK", success_color(ui))
            } else {
                ("FAIL", error_color(ui))
            };
            ui.label(RichText::new(format!("[{}]", status_text)).color(status_color));
            ui.label(RichText::new(&result.resource_type).strong());
        });

        // Stats line
        ui.horizontal(|ui| {
            ui.label(format!(
                "Resources: {} Dash / {} CLI | Fields: {} compared, {} matched, {} mismatched",
                result.dash_count,
                result.cli_count,
                result.total_fields_compared,
                result.total_fields_matched,
                result.total_fields_mismatched
            ));
        });

        // CLI execution info
        if let Some(ref exec) = result.cli_execution {
            ui.horizontal(|ui| {
                ui.label(RichText::new("CLI: ").small());
                ui.label(RichText::new(format!("{}ms, {} bytes", exec.duration_ms, exec.response_size_bytes)).small());
            });
        }

        // Show details for failures or errors
        if !result.matched || result.error.is_some() {
            ui.indent("failure_details", |ui| {
                if let Some(ref err) = result.error {
                    ui.label(RichText::new(format!("Error: {}", err)).color(error_color(ui)));
                }

                if !result.missing_in_dash.is_empty() {
                    ui.label(RichText::new("Missing in Dash:").color(warn_color(ui)));
                    for id in &result.missing_in_dash {
                        ui.label(format!("  - {}", id));
                    }
                }

                if !result.missing_in_cli.is_empty() {
                    ui.label(RichText::new("Missing in CLI:").color(warn_color(ui)));
                    for id in &result.missing_in_cli {
                        ui.label(format!("  - {}", id));
                    }
                }

                // Show field mismatches (limit to first few)
                let mut mismatch_count = 0;
                for resource in &result.resource_comparisons {
                    if !resource.found_in_dash || !resource.found_in_cli {
                        continue;
                    }

                    let mismatches: Vec<_> = resource.field_comparisons.iter()
                        .filter(|f| !f.matched && !f.skipped)
                        .collect();

                    if !mismatches.is_empty() && mismatch_count < 5 {
                        ui.label(RichText::new(format!("Resource: {}", resource.resource_id)).strong());
                        for field in &mismatches {
                            ui.label(format!("  {} - Dash: {} | CLI: {}",
                                field.field_name,
                                field.dash_value.as_deref().unwrap_or("null"),
                                field.cli_value.as_deref().unwrap_or("null")
                            ));
                        }
                        mismatch_count += 1;
                    }
                }

                if mismatch_count >= 5 {
                    ui.label(RichText::new("... (export results for full details)").italics());
                }
            });
        }

        ui.add_space(8.0);
    }

    /// Export results to files.
    fn export_results(&mut self) {
        if let Some(ref results) = self.results {
            match results.write_to_files() {
                Ok((summary, _details)) => {
                    self.status_message = Some(format!(
                        "Results exported to {:?} (summary, details, raw JSON)",
                        summary.parent().unwrap_or(&summary)
                    ));
                    info!("Exported verification results to files");
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to export: {}", e));
                    error!("Failed to export verification results: {}", e);
                }
            }
        }
    }
}
