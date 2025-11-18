//! CloudWatch Logs Viewer Window
//!
//! Displays CloudWatch Logs for AWS resources with fuzzy search filtering.

#![warn(clippy::all, rust_2018_idioms)]

use super::window_focus::FocusableWindow;
use crate::app::data_plane::cloudwatch_logs::{CloudWatchLogsClient, LogEvent, LogQueryResult};
use crate::app::resource_explorer::credentials::CredentialCoordinator;
use chrono::{DateTime, Utc};
use eframe::egui;
use egui::{Color32, Context, RichText, Ui};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::sync::Arc;
use std::sync::mpsc;

/// Maximum number of log events to display in the UI
const MAX_DISPLAY_EVENTS: usize = 1000;

/// Parameters for showing the CloudWatch Logs window
#[derive(Clone)]
pub struct CloudWatchLogsShowParams {
    pub log_group_name: String,
    pub resource_name: String,
    pub account_id: String,
    pub region: String,
}

/// Result from background log loading
type LogLoadResult = Result<LogQueryResult, String>;

pub struct CloudWatchLogsWindow {
    pub open: bool,
    // Display parameters
    log_group_name: String,
    resource_name: String,
    account_id: String,
    region: String,

    // State
    logs: Vec<LogEvent>,
    search_filter: String,
    loading: bool,
    error_message: Option<String>,

    // Services
    client: Arc<CloudWatchLogsClient>,
    fuzzy_matcher: SkimMatcherV2,

    // Channel for receiving log results from background thread
    log_receiver: mpsc::Receiver<LogLoadResult>,
    log_sender: mpsc::Sender<LogLoadResult>,
}

impl CloudWatchLogsWindow {
    pub fn new(credential_coordinator: Arc<CredentialCoordinator>) -> Self {
        let (log_sender, log_receiver) = mpsc::channel();

        Self {
            open: false,
            log_group_name: String::new(),
            resource_name: String::new(),
            account_id: String::new(),
            region: String::new(),
            logs: Vec::new(),
            search_filter: String::new(),
            loading: false,
            error_message: None,
            client: Arc::new(CloudWatchLogsClient::new(credential_coordinator)),
            fuzzy_matcher: SkimMatcherV2::default(),
            log_receiver,
            log_sender,
        }
    }

    /// Open the window and load logs for a specific resource
    pub fn open_for_resource(&mut self, params: CloudWatchLogsShowParams) {
        self.log_group_name = params.log_group_name;
        self.resource_name = params.resource_name;
        self.account_id = params.account_id;
        self.region = params.region;
        self.search_filter.clear();
        self.error_message = None;
        self.open = true;

        // Start loading logs
        self.refresh_logs();
    }

    /// Refresh logs from CloudWatch
    fn refresh_logs(&mut self) {
        self.loading = true;
        self.error_message = None;

        // Spawn async task to fetch logs
        let client = Arc::clone(&self.client);
        let account_id = self.account_id.clone();
        let region = self.region.clone();
        let log_group_name = self.log_group_name.clone();
        let sender = self.log_sender.clone();

        // Create a new thread (since egui runs on a blocking thread) and run tokio inside it
        std::thread::spawn(move || {
            // Create a new tokio runtime for this thread
            let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

            // Run the async operation
            runtime.block_on(async move {
                let result = match client.get_latest_log_events(&account_id, &region, &log_group_name, 100).await {
                    Ok(result) => {
                        log::info!("Loaded {} log events from {}", result.events.len(), log_group_name);
                        Ok(result)
                    }
                    Err(e) => {
                        log::error!("Failed to load logs: {}", e);
                        Err(e.to_string())
                    }
                };

                // Send result back through channel
                let _ = sender.send(result);
            });
        });
    }

    /// Poll for log results from background thread
    fn poll_log_results(&mut self) {
        // Check for results from background thread
        while let Ok(result) = self.log_receiver.try_recv() {
            self.loading = false;

            match result {
                Ok(log_result) => {
                    self.logs = log_result.events;
                    self.error_message = None;
                }
                Err(error_msg) => {
                    self.error_message = Some(error_msg);
                    self.logs.clear();
                }
            }
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        self.show_with_offset(ctx, egui::Vec2::ZERO);
    }

    pub fn show_with_focus(&mut self, ctx: &Context, bring_to_front: bool) {
        self.show_with_offset_and_focus(ctx, egui::Vec2::ZERO, bring_to_front);
    }

    pub fn show_with_offset(&mut self, ctx: &Context, offset: egui::Vec2) {
        self.show_with_offset_and_focus(ctx, offset, false);
    }

    fn show_with_offset_and_focus(
        &mut self,
        ctx: &Context,
        _offset: egui::Vec2,
        bring_to_front: bool,
    ) {
        if !self.open {
            return;
        }

        // Poll for log results from background thread
        self.poll_log_results();

        // Request continuous repaint while loading to show spinner and update logs as they arrive
        if self.loading {
            ctx.request_repaint();
        }

        let title = format!("CloudWatch Logs: {}", self.resource_name);

        // Store open state locally to avoid borrow checker issues
        let mut is_open = self.open;

        let mut window = egui::Window::new(title)
            .open(&mut is_open)
            .default_size([800.0, 600.0])
            .resizable(true)
            .collapsible(true);

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            self.ui_content(ui);
        });

        // Update open state after window is shown
        self.open = is_open;
    }

    fn ui_content(&mut self, ui: &mut Ui) {
        // Header: Log group name
        ui.horizontal(|ui| {
            ui.label(RichText::new("Log Group:").strong());
            ui.label(&self.log_group_name);
        });

        ui.separator();

        // Search and Refresh controls
        ui.horizontal(|ui| {
            ui.label("Search (fuzzy):");
            let response = ui.text_edit_singleline(&mut self.search_filter);

            // Auto-focus search box when window opens
            if self.open && self.logs.is_empty() && !self.loading {
                response.request_focus();
            }

            if ui.button("Refresh").clicked() {
                self.refresh_logs();
            }
        });

        ui.separator();

        // Status message
        if self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading logs...");
            });
        } else if let Some(error) = &self.error_message {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
        }

        ui.separator();

        // Log events display
        let available_height = ui.available_height() - 100.0; // Reserve space for footer

        egui::ScrollArea::vertical()
            .max_height(available_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.render_log_events(ui);
            });

        ui.separator();

        // Footer: Statistics
        let filtered_count = self.get_filtered_logs().len();
        let total_count = self.logs.len().min(MAX_DISPLAY_EVENTS);
        ui.label(format!(
            "Showing {} of {} events (latest)",
            filtered_count, total_count
        ));

        ui.horizontal(|ui| {
            if ui.button("Export to File").clicked() {
                self.export_logs_to_file();
            }
        });
    }

    fn render_log_events(&self, ui: &mut Ui) {
        if self.logs.is_empty() && !self.loading {
            ui.label(RichText::new("No log events available").italics());
            ui.label("Click 'Refresh' to load the latest logs.");
            return;
        }

        let filtered_logs = self.get_filtered_logs();

        if filtered_logs.is_empty() && !self.search_filter.is_empty() {
            ui.label(RichText::new("No logs match your search filter").italics());
            return;
        }

        for event in filtered_logs {
            self.render_log_event(ui, event);
            ui.add_space(8.0);
        }
    }

    fn render_log_event(&self, ui: &mut Ui, event: &LogEvent) {
        // Format timestamp
        let timestamp = DateTime::from_timestamp_millis(event.timestamp)
            .unwrap_or_else(|| Utc::now());
        let timestamp_str = timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        // Event header: timestamp and stream name
        ui.horizontal(|ui| {
            ui.label(RichText::new(timestamp_str).monospace().weak());
            ui.label(RichText::new(&format!("[{}]", event.log_stream_name)).monospace().weak());
        });

        // Event message - try to format as JSON if possible
        let formatted_message = self.try_format_json(&event.message);

        // Render message with search highlighting if filter is active
        if self.search_filter.is_empty() {
            // Render each line separately for proper formatting
            for line in formatted_message.lines() {
                ui.label(RichText::new(line).monospace());
            }
        } else {
            self.render_highlighted_text(ui, &formatted_message);
        }
    }

    /// Try to detect and format JSON in log message
    fn try_format_json(&self, message: &str) -> String {
        let trimmed = message.trim();

        // First, try to parse the entire message as JSON
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']')) {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Ok(formatted) = serde_json::to_string_pretty(&json_value) {
                    return formatted;
                }
            }
        }

        // If not, look for JSON embedded within the message
        // Find the first { or [ and try to extract and format JSON from there
        if let Some(start_brace) = message.find('{') {
            if let Some(json_str) = Self::extract_json_object(&message[start_brace..]) {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Ok(formatted) = serde_json::to_string_pretty(&json_value) {
                        // Replace the JSON portion with formatted version
                        let before = &message[..start_brace];
                        let after_end = start_brace + json_str.len();
                        let after = if after_end < message.len() {
                            &message[after_end..]
                        } else {
                            ""
                        };
                        return format!("{}\n{}\n{}", before.trim(), formatted, after.trim());
                    }
                }
            }
        }

        // Same for arrays
        if let Some(start_bracket) = message.find('[') {
            if let Some(json_str) = Self::extract_json_array(&message[start_bracket..]) {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Ok(formatted) = serde_json::to_string_pretty(&json_value) {
                        let before = &message[..start_bracket];
                        let after_end = start_bracket + json_str.len();
                        let after = if after_end < message.len() {
                            &message[after_end..]
                        } else {
                            ""
                        };
                        return format!("{}\n{}\n{}", before.trim(), formatted, after.trim());
                    }
                }
            }
        }

        // Return original message if no JSON found or parsing failed
        message.to_string()
    }

    /// Extract a JSON object from a string starting with '{'
    fn extract_json_object(s: &str) -> Option<&str> {
        if !s.starts_with('{') {
            return None;
        }

        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (idx, ch) in s.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&s[..=idx]);
                    }
                }
                _ => {}
            }
        }

        None
    }

    /// Extract a JSON array from a string starting with '['
    fn extract_json_array(s: &str) -> Option<&str> {
        if !s.starts_with('[') {
            return None;
        }

        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (idx, ch) in s.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '[' if !in_string => depth += 1,
                ']' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&s[..=idx]);
                    }
                }
                _ => {}
            }
        }

        None
    }

    /// Render text with yellow highlighting for search matches
    fn render_highlighted_text(&self, ui: &mut Ui, text: &str) {
        let highlight_bg = Color32::from_rgb(255, 255, 200); // Light yellow background
        let search_lower = self.search_filter.to_lowercase();

        // Split text into lines to handle newlines (from formatted JSON)
        for (line_idx, line) in text.lines().enumerate() {
            if line_idx > 0 {
                // Add spacing between lines
                ui.add_space(2.0);
            }

            let line_lower = line.to_lowercase();

            // Find all occurrences in this line and highlight them
            let mut current_pos = 0;
            let mut found_match = false;

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0; // No spacing between text segments

                while current_pos < line.len() {
                    let remaining = &line[current_pos..];
                    let remaining_lower = &line_lower[current_pos..];

                    if let Some(match_start) = remaining_lower.find(&search_lower) {
                        found_match = true;
                        let match_end = match_start + self.search_filter.len();

                        // Render text before match
                        if match_start > 0 {
                            let before = &remaining[..match_start];
                            ui.label(RichText::new(before).monospace());
                        }

                        // Render highlighted match
                        let matched = &remaining[match_start..match_end];
                        ui.label(
                            RichText::new(matched)
                                .monospace()
                                .background_color(highlight_bg)
                                .color(Color32::BLACK)
                        );

                        current_pos += match_end;
                    } else {
                        // No more matches, render the rest
                        ui.label(RichText::new(remaining).monospace());
                        break;
                    }
                }

                // If no matches found on this line, render the whole line
                if !found_match {
                    ui.label(RichText::new(line).monospace());
                }
            });
        }
    }

    fn get_filtered_logs(&self) -> Vec<&LogEvent> {
        if self.search_filter.is_empty() {
            self.logs.iter().take(MAX_DISPLAY_EVENTS).collect()
        } else {
            self.logs
                .iter()
                .filter(|event| {
                    // Fuzzy match against message content
                    self.fuzzy_matcher
                        .fuzzy_match(&event.message, &self.search_filter)
                        .is_some()
                })
                .take(MAX_DISPLAY_EVENTS)
                .collect()
        }
    }

    fn export_logs_to_file(&self) {
        // TODO: Implement export to file functionality
        // This would use a file picker dialog and write logs to a text file
        log::info!("Export logs to file - not yet implemented");
    }
}

impl FocusableWindow for CloudWatchLogsWindow {
    type ShowParams = CloudWatchLogsShowParams;

    fn window_id(&self) -> &'static str {
        "cloudwatch_logs_window"
    }

    fn window_title(&self) -> String {
        format!("CloudWatch Logs: {}", self.resource_name)
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        // Open for the resource first
        self.open_for_resource(params);

        // Then show with focus
        CloudWatchLogsWindow::show_with_focus(self, ctx, bring_to_front);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full UI testing would require egui_kittest
    // These are basic structure tests

    #[test]
    fn test_window_creation() {
        // This is a placeholder - actual tests would need a CredentialCoordinator
        // For now, just verify the structure compiles
    }

    #[test]
    fn test_max_display_events() {
        assert_eq!(MAX_DISPLAY_EVENTS, 1000);
    }
}
