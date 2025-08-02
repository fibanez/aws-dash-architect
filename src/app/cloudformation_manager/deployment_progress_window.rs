#![allow(clippy::collapsible_if)]

use crate::app::cloudformation_manager::{DeploymentOperation, DeploymentState, StackEvent};
use egui::{Color32, Context, RichText, ScrollArea, Ui, Window};

/// UI window for displaying CloudFormation deployment progress
pub struct DeploymentProgressWindow {
    /// Current deployment being monitored
    deployment: Option<DeploymentOperation>,
    /// Whether the window is open
    pub is_open: bool,
    /// Whether to auto-scroll the event log
    auto_scroll: bool,
    /// Filter for event log display
    event_filter: EventFilter,
    /// Whether to show only failed events
    show_only_failures: bool,
    /// Export format selection
    export_format: ExportFormat,
}

/// Event filtering options
#[derive(Debug, Clone, PartialEq)]
pub enum EventFilter {
    All,
    StackOnly,
    ResourcesOnly,
    InProgress,
    Failed,
}

/// Export format options
#[derive(Debug, Clone, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
    Text,
}

impl Default for DeploymentProgressWindow {
    fn default() -> Self {
        Self {
            deployment: None,
            is_open: false,
            auto_scroll: true,
            event_filter: EventFilter::All,
            show_only_failures: false,
            export_format: ExportFormat::Json,
        }
    }
}

impl DeploymentProgressWindow {
    /// Create a new deployment progress window
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the window with a specific deployment
    pub fn open_with_deployment(&mut self, deployment: DeploymentOperation) {
        self.deployment = Some(deployment);
        self.is_open = true;
    }

    /// Update the deployment data
    pub fn update_deployment(&mut self, deployment: DeploymentOperation) {
        self.deployment = Some(deployment);
    }

    /// Close the window
    pub fn close(&mut self) {
        self.is_open = false;
        self.deployment = None;
    }

    /// Show the deployment progress window
    pub fn show(&mut self, ctx: &Context) {
        if !self.is_open {
            return;
        }

        let Some(deployment) = self.deployment.clone() else {
            self.is_open = false;
            return;
        };

        let window_title = format!("Deployment Progress - {}", deployment.stack_name);

        let mut is_open = self.is_open;
        let _response = Window::new(window_title)
            .default_size([800.0, 600.0])
            .resizable(true)
            .open(&mut is_open)
            .show(ctx, |ui| {
                self.show_deployment_content(ui, ctx, &deployment);
            });

        self.is_open = is_open;
    }

    /// Show the main deployment content
    fn show_deployment_content(
        &mut self,
        ui: &mut Ui,
        ctx: &Context,
        deployment: &DeploymentOperation,
    ) {
        // Header with basic deployment info
        self.show_deployment_header(ui, deployment);

        ui.separator();

        // Progress section
        self.show_progress_section(ui, deployment);

        ui.separator();

        // Event log section
        self.show_event_log_section(ui, deployment);

        ui.separator();

        // Stack outputs section (if deployment is complete)
        if deployment.state.is_terminal() && !deployment.stack_outputs.is_empty() {
            self.show_stack_outputs_section(ui, ctx, deployment);
            ui.separator();
        }

        // Action buttons
        self.show_action_buttons(ui, deployment);
    }

    /// Show deployment header information
    fn show_deployment_header(&self, ui: &mut Ui, deployment: &DeploymentOperation) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Stack:").strong());
            ui.label(&deployment.stack_name);

            ui.separator();

            ui.label(RichText::new("Type:").strong());
            let type_text = match deployment.deployment_type {
                crate::app::cloudformation_manager::DeploymentType::Create => "Create",
                crate::app::cloudformation_manager::DeploymentType::Update => "Update",
                crate::app::cloudformation_manager::DeploymentType::Delete => "Delete",
            };
            ui.label(type_text);

            ui.separator();

            ui.label(RichText::new("Environment:").strong());
            ui.label(&deployment.environment);

            ui.separator();

            ui.label(RichText::new("Region:").strong());
            ui.label(&deployment.region);
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Started:").strong());
            ui.label(
                deployment
                    .start_time
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
            );

            if let Some(end_time) = deployment.end_time {
                ui.separator();
                ui.label(RichText::new("Ended:").strong());
                ui.label(end_time.format("%Y-%m-%d %H:%M:%S UTC").to_string());

                ui.separator();
                ui.label(RichText::new("Duration:").strong());
                let duration = deployment.duration();
                let total_seconds = duration.as_secs();
                let minutes = total_seconds / 60;
                let seconds = total_seconds % 60;
                ui.label(format!("{}m {}s", minutes, seconds));
            }
        });
    }

    /// Show the progress section
    fn show_progress_section(&mut self, ui: &mut Ui, deployment: &DeploymentOperation) {
        ui.group(|ui| {
            ui.label(RichText::new("Progress").heading());

            // Status with color coding
            ui.horizontal(|ui| {
                ui.label(RichText::new("Status:").strong());

                let (status_text, status_color) = match &deployment.state {
                    DeploymentState::Collecting => ("Collecting Parameters", Color32::BLUE),
                    DeploymentState::Validating => ("Validating Template", Color32::BLUE),
                    DeploymentState::Deploying => ("Deploying Stack", Color32::YELLOW),
                    DeploymentState::Monitoring => ("Monitoring Progress", Color32::YELLOW),
                    DeploymentState::Complete(true) => ("Deployment Successful", Color32::GREEN),
                    DeploymentState::Complete(false) => ("Deployment Failed", Color32::RED),
                    DeploymentState::Cancelled => ("Deployment Cancelled", Color32::GRAY),
                    DeploymentState::Failed(error) => {
                        ui.label(RichText::new("Failed:").strong().color(Color32::RED));
                        ui.label(RichText::new(error).color(Color32::RED));
                        return;
                    }
                };

                ui.label(RichText::new(status_text).color(status_color));
            });

            // Progress bar
            let progress = deployment.progress_percent as f32 / 100.0;
            let progress_bar =
                egui::ProgressBar::new(progress).text(format!("{}%", deployment.progress_percent));
            ui.add(progress_bar);

            // Resource progress summary
            if !deployment.events.is_empty() {
                ui.horizontal(|ui| {
                    let resource_ids = deployment.get_resource_ids();
                    let total_resources = resource_ids.len();
                    let completed_resources = resource_ids
                        .iter()
                        .filter(|resource_id| {
                            if let Some(event) = deployment.get_latest_resource_event(resource_id) {
                                event.resource_status.ends_with("_COMPLETE")
                                    || event.resource_status.ends_with("_FAILED")
                            } else {
                                false
                            }
                        })
                        .count();

                    ui.label(RichText::new("Resources:").strong());
                    ui.label(format!(
                        "{}/{} completed",
                        completed_resources, total_resources
                    ));
                });
            }
        });
    }

    /// Show the event log section
    fn show_event_log_section(&mut self, ui: &mut Ui, deployment: &DeploymentOperation) {
        ui.group(|ui| {
            // Event log header with controls
            ui.horizontal(|ui| {
                ui.label(RichText::new("Event Log").heading());

                ui.separator();

                // Filter controls
                ui.label("Filter:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", self.event_filter))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.event_filter, EventFilter::All, "All Events");
                        ui.selectable_value(
                            &mut self.event_filter,
                            EventFilter::StackOnly,
                            "Stack Only",
                        );
                        ui.selectable_value(
                            &mut self.event_filter,
                            EventFilter::ResourcesOnly,
                            "Resources Only",
                        );
                        ui.selectable_value(
                            &mut self.event_filter,
                            EventFilter::InProgress,
                            "In Progress",
                        );
                        ui.selectable_value(
                            &mut self.event_filter,
                            EventFilter::Failed,
                            "Failed Only",
                        );
                    });

                ui.separator();

                ui.checkbox(&mut self.show_only_failures, "Show Only Failures");
                ui.checkbox(&mut self.auto_scroll, "Auto Scroll");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑 Clear").clicked() {
                        // In a real implementation, this might clear local display without affecting the deployment
                    }
                });
            });

            ui.separator();

            // Event list
            let filtered_events = self.filter_events(&deployment.events);

            ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(self.auto_scroll)
                .max_height(300.0)
                .show(ui, |ui| {
                    if filtered_events.is_empty() {
                        ui.label("No events to display");
                        return;
                    }

                    for event in filtered_events.iter().rev() {
                        // Show newest first
                        self.show_event_item(ui, event);
                    }
                });
        });
    }

    /// Show the stack outputs section
    fn show_stack_outputs_section(
        &self,
        ui: &mut Ui,
        ctx: &Context,
        deployment: &DeploymentOperation,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Stack Outputs").heading());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📋 Copy All").clicked() {
                        let outputs_text = deployment
                            .stack_outputs
                            .iter()
                            .map(|(key, value)| format!("{}: {}", key, value))
                            .collect::<Vec<_>>()
                            .join("\n");
                        ctx.copy_text(outputs_text);
                    }
                });
            });

            ui.separator();

            if deployment.stack_outputs.is_empty() {
                ui.label("No outputs available");
            } else {
                for (key, value) in &deployment.stack_outputs {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{}:", key)).strong());
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut value.clone()));
                        if ui.button("📋").clicked() {
                            ctx.copy_text(value.clone());
                        }
                    });
                }
            }
        });
    }

    /// Show action buttons
    fn show_action_buttons(&mut self, ui: &mut Ui, deployment: &DeploymentOperation) {
        ui.horizontal(|ui| {
            // Cancel button (only for active deployments)
            if deployment.can_cancel()
                && ui
                    .button(RichText::new("⏹ Cancel Deployment").color(Color32::RED))
                    .clicked()
            {
                // In a real implementation, this would call the deployment manager to cancel
                // deployment_manager.cancel_deployment(&deployment.id).await;
            }

            ui.separator();

            // Export logs button
            ui.menu_button("📁 Export Logs", |ui| {
                ui.label("Export Format:");
                ui.radio_value(&mut self.export_format, ExportFormat::Json, "JSON");
                ui.radio_value(&mut self.export_format, ExportFormat::Csv, "CSV");
                ui.radio_value(&mut self.export_format, ExportFormat::Text, "Text");

                ui.separator();

                if ui.button("Export").clicked() {
                    self.export_deployment_logs(deployment);
                }
            });

            ui.separator();

            // Refresh button
            if ui.button("🔄 Refresh").clicked() {
                // In a real implementation, this would trigger a refresh of deployment data
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Close").clicked() {
                    self.close();
                }
            });
        });
    }

    /// Show a single event item
    fn show_event_item(&self, ui: &mut Ui, event: &StackEvent) {
        ui.horizontal(|ui| {
            // Timestamp
            ui.label(
                RichText::new(event.timestamp.format("%H:%M:%S").to_string())
                    .monospace()
                    .color(Color32::GRAY),
            );

            // Status with color coding
            let status_color = if event.resource_status.contains("FAILED") {
                Color32::RED
            } else if event.resource_status.contains("COMPLETE") {
                Color32::GREEN
            } else if event.resource_status.contains("IN_PROGRESS") {
                Color32::YELLOW
            } else {
                Color32::WHITE
            };

            ui.label(
                RichText::new(&event.resource_status)
                    .monospace()
                    .color(status_color),
            );

            // Resource info
            if let Some(logical_id) = &event.logical_resource_id {
                ui.label(logical_id);

                if let Some(resource_type) = &event.resource_type {
                    ui.label(
                        RichText::new(format!("({})", resource_type))
                            .italics()
                            .color(Color32::GRAY),
                    );
                }
            } else {
                ui.label(RichText::new("Stack Event").italics());
            }

            // Status reason (if available)
            if let Some(reason) = &event.resource_status_reason {
                ui.label(RichText::new(reason).color(Color32::GRAY));
            }
        });
    }

    /// Filter events based on current filter settings
    fn filter_events<'a>(&self, events: &'a [StackEvent]) -> Vec<&'a StackEvent> {
        events
            .iter()
            .filter(|event| {
                // Apply show_only_failures filter
                if self.show_only_failures && !event.resource_status.contains("FAILED") {
                    return false;
                }

                // Apply event filter
                match self.event_filter {
                    EventFilter::All => true,
                    EventFilter::StackOnly => event.logical_resource_id.is_none(),
                    EventFilter::ResourcesOnly => event.logical_resource_id.is_some(),
                    EventFilter::InProgress => event.resource_status.contains("IN_PROGRESS"),
                    EventFilter::Failed => event.resource_status.contains("FAILED"),
                }
            })
            .collect()
    }

    /// Export deployment logs to file
    fn export_deployment_logs(&self, deployment: &DeploymentOperation) {
        // In a real implementation, this would:
        // 1. Generate the export data based on format
        // 2. Open a file save dialog
        // 3. Write the data to the selected file

        let _export_data = match self.export_format {
            ExportFormat::Json => self.generate_json_export(deployment),
            ExportFormat::Csv => self.generate_csv_export(deployment),
            ExportFormat::Text => self.generate_text_export(deployment),
        };

        // For now, just log that export was requested
        tracing::info!(
            "Export requested for deployment {} in {:?} format",
            deployment.id,
            self.export_format
        );
    }

    /// Generate JSON export data
    fn generate_json_export(&self, deployment: &DeploymentOperation) -> String {
        serde_json::to_string_pretty(deployment).unwrap_or_default()
    }

    /// Generate CSV export data
    fn generate_csv_export(&self, deployment: &DeploymentOperation) -> String {
        let mut csv = "Timestamp,Event ID,Resource ID,Resource Type,Status,Reason\n".to_string();

        for event in &deployment.events {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                event.timestamp.format("%Y-%m-%d %H:%M:%S"),
                event.event_id,
                event.logical_resource_id.as_deref().unwrap_or(""),
                event.resource_type.as_deref().unwrap_or(""),
                event.resource_status,
                event.resource_status_reason.as_deref().unwrap_or("")
            ));
        }

        csv
    }

    /// Generate text export data
    fn generate_text_export(&self, deployment: &DeploymentOperation) -> String {
        let mut text = "CloudFormation Deployment Log\n".to_string();
        text.push_str(&format!("Stack: {}\n", deployment.stack_name));
        text.push_str(&format!("Type: {:?}\n", deployment.deployment_type));
        text.push_str(&format!("Environment: {}\n", deployment.environment));
        text.push_str(&format!("Region: {}\n", deployment.region));
        text.push_str(&format!(
            "Started: {}\n",
            deployment.start_time.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        if let Some(end_time) = deployment.end_time {
            text.push_str(&format!(
                "Ended: {}\n",
                end_time.format("%Y-%m-%d %H:%M:%S UTC")
            ));
        }

        text.push_str(&format!("Status: {}\n", deployment.state.description()));
        text.push_str(&format!("Progress: {}%\n\n", deployment.progress_percent));

        text.push_str("Events:\n");
        text.push_str("--------\n");

        for event in &deployment.events {
            text.push_str(&format!(
                "{} | {} | {} | {} | {}\n",
                event.timestamp.format("%H:%M:%S"),
                event.resource_status,
                event.logical_resource_id.as_deref().unwrap_or("Stack"),
                event.resource_type.as_deref().unwrap_or(""),
                event.resource_status_reason.as_deref().unwrap_or("")
            ));
        }

        if !deployment.stack_outputs.is_empty() {
            text.push_str("\nStack Outputs:\n");
            text.push_str("--------------\n");
            for (key, value) in &deployment.stack_outputs {
                text.push_str(&format!("{}: {}\n", key, value));
            }
        }

        text
    }
}
