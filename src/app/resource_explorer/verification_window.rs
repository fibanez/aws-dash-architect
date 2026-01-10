//! Verification window for comparing Dash cache with AWS CLI output.
//!
//! This module provides a UI window that allows users to verify that Dash's
//! cached resource data matches what the AWS CLI returns.
//!
//! CRITICAL: This performs FIELD-BY-FIELD property comparison, not just resource counting.
//!
//! Uses background thread pattern to avoid blocking UI during CLI execution.

#![cfg(debug_assertions)]

use super::cli_commands::{
    check_cli_available, execute_cli_with_details_progress, get_cli_command, DetailProgressCallback,
};
use super::credentials::CredentialCoordinator;
use super::global_services::is_global_service;
use super::state::{ResourceEntry, ResourceExplorerState};
use super::verification_results::{
    compare_resources_detailed, ResourceTypeResult, VerificationResults,
};
use egui::{self, Color32, Context, RichText, ScrollArea, Ui};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tokio::sync::RwLock;
use tracing::{error, info};

// ============================================================================
// Background Progress Types
// ============================================================================

/// Progress state shared between background thread and UI
#[derive(Debug, Clone)]
pub struct VerificationProgress {
    /// Current state
    pub state: VerificationProgressState,
    /// Current resource type being verified
    pub current_type: Option<String>,
    /// Current CLI command being executed
    pub current_command: Option<String>,
    /// Number of types completed
    pub completed: usize,
    /// Total types to verify
    pub total: usize,
    /// Completed results (set when done)
    pub results: Option<VerificationResults>,
    /// Types that were skipped (no CLI mapping)
    pub skipped_types: Vec<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Detailed status message (changes frequently for visual feedback)
    pub status_detail: Option<String>,
    /// Current phase within a resource type
    pub phase: VerificationPhase,
    /// Number of resources found in CLI response
    pub cli_resource_count: usize,
    /// Number of resources in Dash cache for current type
    pub dash_resource_count: usize,
}

/// Current phase of verification for a resource type
#[derive(Debug, Clone, PartialEq, Default)]
pub enum VerificationPhase {
    #[default]
    Starting,
    ExecutingCliList,
    ExecutingCliDetails,
    ComparingResources,
    Done,
}

/// UI-friendly progress display data
#[derive(Debug, Clone, Default)]
struct ProgressDisplay {
    current_type: Option<String>,
    current_command: Option<String>,
    completed: usize,
    total: usize,
    status_detail: Option<String>,
    phase: VerificationPhase,
    cli_resource_count: usize,
    dash_resource_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerificationProgressState {
    /// Not started
    Idle,
    /// Running in background
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
}

impl Default for VerificationProgress {
    fn default() -> Self {
        Self {
            state: VerificationProgressState::Idle,
            current_type: None,
            current_command: None,
            completed: 0,
            total: 0,
            results: None,
            skipped_types: Vec::new(),
            error: None,
            status_detail: None,
            phase: VerificationPhase::default(),
            cli_resource_count: 0,
            dash_resource_count: 0,
        }
    }
}

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
        Color32::from_rgb(100, 200, 100) // Lighter green for dark mode
    } else {
        Color32::from_rgb(40, 150, 40) // Darker green for light mode
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

    // Background thread progress (shared with background thread)
    background_progress: Arc<Mutex<VerificationProgress>>,

    // Background thread handle (to detect if thread is still running)
    #[allow(dead_code)]
    background_thread: Option<JoinHandle<()>>,

    // Types that were skipped (no CLI mapping) - shown in UI
    skipped_types: Vec<String>,
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
            background_progress: Arc::new(Mutex::new(VerificationProgress::default())),
            background_thread: None,
            skipped_types: Vec::new(),
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

        // Poll background thread progress during Running state
        if matches!(self.state, VerificationState::Running) {
            self.poll_background_progress(ctx);
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

    /// Poll the background thread for progress updates.
    fn poll_background_progress(&mut self, ctx: &Context) {
        // Read progress from shared state (non-blocking)
        let progress = {
            match self.background_progress.try_lock() {
                Ok(guard) => guard.clone(),
                Err(_) => {
                    // Couldn't get lock, request repaint to try again next frame
                    ctx.request_repaint();
                    return;
                }
            }
        };

        // Update UI state based on background progress
        match progress.state {
            VerificationProgressState::Running => {
                // Update status message with progress
                self.status_message = Some(format!(
                    "Verifying {} ({}/{})",
                    progress.current_type.as_deref().unwrap_or("..."),
                    progress.completed,
                    progress.total
                ));
                // Request repaint to keep polling
                ctx.request_repaint();
            }
            VerificationProgressState::Completed => {
                // Move results from shared state to local state
                if let Some(results) = progress.results {
                    let total_fields = results.total_fields_compared();
                    let matched_fields = results.total_fields_matched();
                    self.status_message = Some(format!(
                        "Verification completed: {} fields compared, {} matched ({:.1}%)",
                        total_fields,
                        matched_fields,
                        if total_fields > 0 {
                            (matched_fields as f64 / total_fields as f64) * 100.0
                        } else {
                            100.0
                        }
                    ));
                    self.results = Some(results);
                    self.skipped_types = progress.skipped_types;
                }
                self.state = VerificationState::Completed;
                // Reset background progress
                if let Ok(mut guard) = self.background_progress.lock() {
                    *guard = VerificationProgress::default();
                }
            }
            VerificationProgressState::Failed => {
                self.state = VerificationState::Error(
                    progress
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string()),
                );
                self.skipped_types = progress.skipped_types;
                // Reset background progress
                if let Ok(mut guard) = self.background_progress.lock() {
                    *guard = VerificationProgress::default();
                }
            }
            VerificationProgressState::Idle => {
                // Not started yet, keep polling
                ctx.request_repaint();
            }
        }
    }

    /// Get progress info for UI display (read from shared state)
    fn get_progress_info(&self) -> ProgressDisplay {
        match self.background_progress.try_lock() {
            Ok(guard) => ProgressDisplay {
                current_type: guard.current_type.clone(),
                current_command: guard.current_command.clone(),
                completed: guard.completed,
                total: guard.total,
                status_detail: guard.status_detail.clone(),
                phase: guard.phase.clone(),
                cli_resource_count: guard.cli_resource_count,
                dash_resource_count: guard.dash_resource_count,
            },
            Err(_) => ProgressDisplay::default(),
        }
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
        if let Some((account, region, resource_count, type_count)) =
            self.get_cache_info(explorer_state)
        {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Account: ").strong());
                ui.label(&account);
                ui.add_space(20.0);
                ui.label(RichText::new("Region: ").strong());
                ui.label(&region);
                ui.add_space(20.0);
                ui.label(format!(
                    "{} resources in {} types",
                    resource_count, type_count
                ));
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
            let type_count = resources
                .iter()
                .map(|r| &r.resource_type)
                .collect::<std::collections::HashSet<_>>()
                .len();

            // Get account from first resource
            let account = resources.first()?.account_id.clone();

            // Get region from query scope (user's selected regions)
            // Use the first selected region, falling back to us-east-1 if none
            let region = state
                .query_scope
                .regions
                .first()
                .map(|r| r.region_code.clone())
                .unwrap_or_else(|| "us-east-1".to_string());

            return Some((account, region, resources.len(), type_count));
        }
        None
    }

    /// Run verification against CLI with field-by-field comparison.
    /// Spawns a background thread to avoid blocking the UI.
    fn run_verification(
        &mut self,
        explorer_state: &Arc<RwLock<ResourceExplorerState>>,
        credential_coordinator: &Arc<CredentialCoordinator>,
    ) {
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

        info!(
            "[Verification] Starting verification for account {} region {}",
            account, region
        );

        // Get cached resources AND selected resource types from query_scope
        let (cached_by_type, selected_types, all_selected_types, skipped, selected_regions) =
            match explorer_state.try_read() {
                Ok(state) => {
                    // Debug: Log Phase 2 status and detailed_properties count
                    let s3_resources_count = state
                        .resources
                        .iter()
                        .filter(|r| r.resource_type == "AWS::S3::Bucket")
                        .count();
                    let s3_with_details = state
                        .resources
                        .iter()
                        .filter(|r| {
                            r.resource_type == "AWS::S3::Bucket" && r.detailed_properties.is_some()
                        })
                        .count();
                    info!(
                    "[Verification] Reading state: phase2_completed={}, phase2_in_progress={}, S3 buckets={}, S3 with detailed_properties={}",
                    state.phase2_enrichment_completed,
                    state.phase2_enrichment_in_progress,
                    s3_resources_count,
                    s3_with_details
                );

                    // Use filtered resources from state.resources (only visible resources)
                    let cached = self.group_resources_by_type(&state.resources);

                    // Get selected regions for filtering CLI results
                    let regions: Vec<String> = state
                        .query_scope
                        .regions
                        .iter()
                        .map(|r| r.region_code.clone())
                        .collect();

                    // Get ALL selected types from query scope
                    let all_types: Vec<String> = state
                        .query_scope
                        .resource_types
                        .iter()
                        .map(|rt| rt.resource_type.clone())
                        .collect();

                    // Get selected types that have CLI mappings
                    let supported: Vec<String> = state
                        .query_scope
                        .resource_types
                        .iter()
                        .filter(|rt| get_cli_command(&rt.resource_type).is_some())
                        .map(|rt| rt.resource_type.clone())
                        .collect();

                    // Track skipped types (no CLI mapping)
                    let skipped: Vec<String> = state
                        .query_scope
                        .resource_types
                        .iter()
                        .filter(|rt| get_cli_command(&rt.resource_type).is_none())
                        .map(|rt| rt.resource_type.clone())
                        .collect();

                    (cached, supported, all_types, skipped, regions)
                }
                Err(_) => {
                    self.state = VerificationState::Error("Could not read cache".to_string());
                    return;
                }
            };

        // Store skipped types for display
        self.skipped_types = skipped.clone();

        if selected_types.is_empty() {
            let msg = if !all_selected_types.is_empty() {
                format!(
                    "No CLI verification support for selected types: {}",
                    all_selected_types.join(", ")
                )
            } else {
                "No resource types selected.".to_string()
            };
            self.state = VerificationState::Error(msg);
            return;
        }

        info!(
            "[Verification] {} types with CLI support, {} skipped (no CLI mapping)",
            selected_types.len(),
            skipped.len()
        );

        // Get credentials (this is synchronous but fast)
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

        // Initialize shared progress state
        {
            let mut progress = self.background_progress.lock().unwrap();
            progress.state = VerificationProgressState::Running;
            progress.total = selected_types.len();
            progress.completed = 0;
            progress.current_type = None;
            progress.current_command = None;
            progress.results = None;
            progress.skipped_types = skipped;
            progress.error = None;
        }

        // Set UI state to Running
        self.state = VerificationState::Running;
        self.status_message = Some("Starting verification...".to_string());

        // Clone what we need for the background thread
        let progress_arc = self.background_progress.clone();

        // Spawn background thread for CLI verification
        let handle = std::thread::spawn(move || {
            run_verification_background(
                progress_arc,
                account,
                region,
                selected_types,
                cached_by_type,
                selected_regions,
                creds,
            );
        });

        self.background_thread = Some(handle);
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
                // Get progress from shared state
                let progress = self.get_progress_info();

                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!(
                        "Verifying ({}/{} types)...",
                        progress.completed, progress.total
                    ));
                });

                ui.add_space(5.0);

                // Show current resource type
                if let Some(ref resource_type) = progress.current_type {
                    // Extract short name from AWS::Service::Type
                    let short_name = resource_type.split("::").last().unwrap_or(resource_type);
                    ui.label(RichText::new(format!("Type: {}", short_name)).strong());
                }

                // Show phase-specific status
                let phase_text = match progress.phase {
                    VerificationPhase::Starting => "Preparing...",
                    VerificationPhase::ExecutingCliList => "Executing CLI list command...",
                    VerificationPhase::ExecutingCliDetails => "Fetching resource details...",
                    VerificationPhase::ComparingResources => "Comparing resources...",
                    VerificationPhase::Done => "Complete",
                };
                ui.label(format!("  Phase: {}", phase_text));

                // Show CLI command
                if let Some(ref command) = progress.current_command {
                    ui.label(
                        RichText::new(format!("  Command: {}", command))
                            .monospace()
                            .small(),
                    );
                }

                // Show resource counts if available
                if progress.phase == VerificationPhase::ComparingResources
                    || progress.phase == VerificationPhase::Done
                {
                    ui.label(format!(
                        "  Resources: {} in Dash, {} from CLI",
                        progress.dash_resource_count, progress.cli_resource_count
                    ));
                }

                // Show detailed status message (changes frequently)
                if let Some(ref detail) = progress.status_detail {
                    ui.label(RichText::new(format!("  > {}", detail)).weak().italics());
                }

                // Show skipped types during verification
                if !self.skipped_types.is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new("Skipped (no CLI support):").color(warn_color(ui)));
                    for rt in &self.skipped_types {
                        ui.label(format!("  - {}", rt));
                    }
                }
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
                        let pct = if total > 0 {
                            (matched as f64 / total as f64) * 100.0
                        } else {
                            100.0
                        };

                        if pct >= 100.0 {
                            ui.label(
                                RichText::new(format!("{} fields, 100% match", total))
                                    .color(success_color(ui)),
                            );
                        } else if pct >= 90.0 {
                            ui.label(
                                RichText::new(format!("{} fields, {:.1}% match", total, pct))
                                    .color(warn_color(ui)),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!("{} fields, {:.1}% match", total, pct))
                                    .color(error_color(ui)),
                            );
                        }
                    });

                    ui.add_space(10.0);

                    // Per-type results
                    for result in &results.results {
                        self.render_single_result(ui, result);
                    }

                    // Show skipped types (no CLI support)
                    if !self.skipped_types.is_empty() {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.label(RichText::new("Skipped (no CLI support):").color(warn_color(ui)));
                        for rt in &self.skipped_types {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("[SKIP]").color(warn_color(ui)));
                                ui.label(rt);
                            });
                        }
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

        // Discovery gap warning (Feature 2: Dash found 0 but CLI found resources)
        if result.dash_count == 0 && result.cli_count > 0 {
            ui.label(
                RichText::new(format!(
                    "DISCOVERY GAP: CLI found {} resources, Dash found 0 - check resource discovery",
                    result.cli_count
                ))
                .color(error_color(ui))
            );
        }

        // CLI execution info
        if let Some(ref exec) = result.cli_execution {
            ui.horizontal(|ui| {
                ui.label(RichText::new("CLI: ").small());
                ui.label(
                    RichText::new(format!(
                        "{}ms, {} bytes",
                        exec.duration_ms, exec.response_size_bytes
                    ))
                    .small(),
                );
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

                    let mismatches: Vec<_> = resource
                        .field_comparisons
                        .iter()
                        .filter(|f| !f.matched && !f.skipped)
                        .collect();

                    if !mismatches.is_empty() && mismatch_count < 5 {
                        ui.label(
                            RichText::new(format!("Resource: {}", resource.resource_id)).strong(),
                        );
                        for field in &mismatches {
                            ui.label(format!(
                                "  {} - Dash: {} | CLI: {}",
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

// ============================================================================
// Background Verification Function
// ============================================================================

/// Run CLI verification in background thread.
/// Updates shared progress state as it processes each resource type.
fn run_verification_background(
    progress: Arc<Mutex<VerificationProgress>>,
    account: String,
    region: String,
    selected_types: Vec<String>,
    cached_by_type: HashMap<String, Vec<ResourceEntry>>,
    selected_regions: Vec<String>,
    creds: super::credentials::AccountCredentials,
) {
    info!(
        "[Verification Background] Starting verification of {} types",
        selected_types.len()
    );

    let mut results = VerificationResults::new(account.clone(), region.clone());
    let total = selected_types.len();

    for (index, resource_type) in selected_types.iter().enumerate() {
        // Get cached resources for this type (may be empty)
        let dash_resources = cached_by_type
            .get(resource_type)
            .cloned()
            .unwrap_or_default();

        // Update progress: Starting this type
        {
            if let Ok(mut p) = progress.lock() {
                p.current_type = Some(resource_type.clone());
                p.phase = VerificationPhase::Starting;
                p.dash_resource_count = dash_resources.len();
                p.cli_resource_count = 0;
                p.status_detail = Some(format!(
                    "Starting {} ({} cached)",
                    resource_type.split("::").last().unwrap_or(resource_type),
                    dash_resources.len()
                ));
                if let Some(cmd) = get_cli_command(resource_type) {
                    p.current_command = Some(format!("aws {} {}", cmd.service, cmd.operation));
                }
            }
        }

        info!(
            "[Verification Background] Processing {} ({}/{}) - {} cached resources",
            resource_type,
            index + 1,
            total,
            dash_resources.len()
        );

        // Execute CLI command
        if let Some(cmd) = get_cli_command(resource_type) {
            // Use the selected region from query scope
            // For global services (S3, IAM), the API returns the same results regardless of region
            // but we use the user's selected region for consistency
            let query_region = &region;

            // Update progress: Executing CLI list
            {
                if let Ok(mut p) = progress.lock() {
                    p.phase = VerificationPhase::ExecutingCliList;
                    p.status_detail = Some(format!(
                        "Running: aws {} {} --region {}",
                        cmd.service, cmd.operation, query_region
                    ));
                }
            }

            // Create progress callback for detail fetching
            let progress_for_callback = progress.clone();
            let resource_type_for_callback = resource_type.clone();
            let detail_callback: Option<DetailProgressCallback> =
                Some(Box::new(move |current, total, resource_id| {
                    if let Ok(mut p) = progress_for_callback.lock() {
                        p.phase = VerificationPhase::ExecutingCliDetails;
                        p.status_detail = Some(format!(
                            "Fetching details {}/{}: {}",
                            current, total, resource_id
                        ));
                        // Update CLI resource count as we discover them
                        if current == 1 {
                            p.cli_resource_count = total;
                        }
                    }
                    // Log progress for visibility
                    if current % 10 == 0 || current == total {
                        info!(
                            "[CLI Details] {}: {}/{} resources",
                            resource_type_for_callback, current, total
                        );
                    }
                }));

            match execute_cli_with_details_progress(
                &cmd,
                resource_type,
                &creds,
                query_region,
                detail_callback,
            ) {
                Ok(cli_result) => {
                    // Update progress: Got CLI response
                    {
                        if let Ok(mut p) = progress.lock() {
                            p.cli_resource_count = cli_result.resource_ids.len();
                            p.status_detail = Some(format!(
                                "CLI returned {} resources",
                                cli_result.resource_ids.len()
                            ));
                        }
                    }

                    if let Some(ref err) = cli_result.error {
                        error!(
                            "[Verification Background] CLI error for {}: {}",
                            resource_type, err
                        );
                        // Update progress: Error
                        {
                            if let Ok(mut p) = progress.lock() {
                                p.status_detail = Some(format!("CLI error: {}", err));
                            }
                        }
                        results.add_result(ResourceTypeResult {
                            resource_type: resource_type.clone(),
                            dash_count: dash_resources.len(),
                            cli_count: 0,
                            matched: false,
                            missing_in_dash: Vec::new(),
                            missing_in_cli: Vec::new(),
                            resource_comparisons: Vec::new(),
                            cli_execution: Some(cli_result.execution),
                            error: Some(err.to_string()),
                            total_fields_compared: 0,
                            total_fields_matched: 0,
                            total_fields_mismatched: 0,
                        });
                    } else {
                        // For global services like S3, filter CLI results to only include
                        // resources in the selected regions. CLI returns all resources globally,
                        // but we only want to compare resources in selected regions.
                        let (filtered_resources_by_id, filtered_resource_ids) =
                            if cmd.is_global && is_global_service(resource_type) {
                                filter_cli_results_by_region(
                                    resource_type,
                                    &cli_result.resources_by_id,
                                    &cli_result.resource_ids,
                                    &selected_regions,
                                )
                            } else {
                                (
                                    cli_result.resources_by_id.clone(),
                                    cli_result.resource_ids.clone(),
                                )
                            };

                        // Update progress: Comparing resources
                        {
                            if let Ok(mut p) = progress.lock() {
                                p.phase = VerificationPhase::ComparingResources;
                                p.cli_resource_count = filtered_resource_ids.len();
                                p.status_detail = Some(format!(
                                    "Comparing {} Dash vs {} CLI resources (filtered from {})",
                                    dash_resources.len(),
                                    filtered_resource_ids.len(),
                                    cli_result.resource_ids.len()
                                ));
                            }
                        }

                        // Use detailed comparison with field-by-field checking
                        let result = compare_resources_detailed(
                            resource_type,
                            &dash_resources,
                            &filtered_resources_by_id,
                            &filtered_resource_ids,
                            cli_result.execution,
                        );

                        // Update progress: Comparison complete
                        {
                            if let Ok(mut p) = progress.lock() {
                                p.phase = VerificationPhase::Done;
                                let status = if result.matched { "OK" } else { "MISMATCH" };
                                p.status_detail = Some(format!(
                                    "{}: {} fields compared, {} matched",
                                    status,
                                    result.total_fields_compared,
                                    result.total_fields_matched
                                ));
                            }
                        }

                        // Log discovery gap scenario
                        if dash_resources.is_empty() && !cli_result.resource_ids.is_empty() {
                            info!(
                                "[Verification Background] DISCOVERY GAP: {} has {} CLI resources but 0 in Dash",
                                resource_type,
                                cli_result.resource_ids.len()
                            );
                        }

                        info!(
                            "[Verification Background] {} - {} Dash / {} CLI, {} fields compared",
                            resource_type,
                            result.dash_count,
                            result.cli_count,
                            result.total_fields_compared
                        );

                        results.add_result(result);
                    }
                }
                Err(e) => {
                    error!(
                        "[Verification Background] Failed to execute CLI for {}: {}",
                        resource_type, e
                    );
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
        }

        // Update progress after completing this type
        {
            if let Ok(mut p) = progress.lock() {
                p.completed = index + 1;
            }
        }
    }

    // Mark as completed with results
    info!(
        "[Verification Background] Completed. {} fields compared, {} matched",
        results.total_fields_compared(),
        results.total_fields_matched()
    );

    {
        if let Ok(mut p) = progress.lock() {
            p.state = VerificationProgressState::Completed;
            p.results = Some(results);
            p.current_type = None;
            p.current_command = None;
        }
    }
}

/// Filter CLI results to only include resources in the selected regions.
/// This is used for global services like S3 where CLI returns all resources
/// but we only want to compare resources in selected regions.
fn filter_cli_results_by_region(
    resource_type: &str,
    resources_by_id: &HashMap<String, serde_json::Value>,
    resource_ids: &[String],
    selected_regions: &[String],
) -> (HashMap<String, serde_json::Value>, Vec<String>) {
    use serde_json::Value;

    // Helper to convert LocationConstraint to region code
    fn location_to_region(location: Option<&Value>) -> String {
        match location {
            None => "us-east-1".to_string(),
            Some(Value::Null) => "us-east-1".to_string(),
            Some(Value::String(s)) if s.is_empty() => "us-east-1".to_string(),
            Some(Value::String(s)) if s == "EU" => "eu-west-1".to_string(),
            Some(Value::String(s)) => s.clone(),
            _ => "us-east-1".to_string(),
        }
    }

    let mut filtered_by_id = HashMap::new();
    let mut filtered_ids = Vec::new();

    for id in resource_ids {
        if let Some(resource) = resources_by_id.get(id) {
            // Determine the resource's region based on resource type
            let resource_region = if resource_type == "AWS::S3::Bucket" {
                // S3: Use LocationConstraint from get-bucket-location
                location_to_region(resource.get("LocationConstraint"))
            } else {
                // Other global services: may need different logic
                // For now, include all if we can't determine region
                "unknown".to_string()
            };

            // Include resource if its region matches one of the selected regions
            // or if we couldn't determine the region (be permissive)
            if resource_region == "unknown" || selected_regions.contains(&resource_region) {
                filtered_by_id.insert(id.clone(), resource.clone());
                filtered_ids.push(id.clone());
            }
        }
    }

    info!(
        "[CLI Filter] {} filtered from {} to {} resources for regions {:?}",
        resource_type,
        resource_ids.len(),
        filtered_ids.len(),
        selected_regions
    );

    (filtered_by_id, filtered_ids)
}
