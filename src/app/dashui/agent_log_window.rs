#![warn(clippy::all, rust_2018_idioms)]

//! Agent Log Viewer Window
//!
//! Displays the per-agent log file with search, filtering, and navigation capabilities.
//! Shows all agent activity: conversations, model interactions, tool calls, and lifecycle events.

use super::window_focus::FocusableWindow;
use crate::app::agent_framework::{AgentId, AgentLogger};
use eframe::egui;
use egui::{Color32, Context, RichText, ScrollArea, TextEdit, Ui};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// Log filtering options
#[derive(Debug, Clone, Copy, PartialEq)]
enum LogFilter {
    All,
    Messages,      // User/Assistant/System messages
    ModelCalls,    // Model requests/responses
    Tools,         // Tool executions
    Errors,        // Error messages only
    Lifecycle,     // Agent creation/termination
}

pub struct AgentLogWindow {
    open: bool,
    agent_id: Option<AgentId>,
    agent_name: String,
    log_content: String,
    log_path: PathBuf,

    // UI state
    search_query: String,
    filter: LogFilter,
    auto_refresh: bool,
    scroll_to_bottom: bool,

    // For auto-refresh
    last_refresh: std::time::Instant,
}

impl AgentLogWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            agent_id: None,
            agent_name: String::new(),
            log_content: String::new(),
            log_path: PathBuf::new(),
            search_query: String::new(),
            filter: LogFilter::All,
            auto_refresh: false,
            scroll_to_bottom: false,
            last_refresh: std::time::Instant::now(),
        }
    }

    /// Open the log viewer for a specific agent
    pub fn show_log_for_agent(&mut self, agent_id: AgentId, agent_name: String, logger: &Arc<AgentLogger>) {
        tracing::info!("🔍 Opening agent log window for agent {} ({})", agent_id, agent_name);
        tracing::info!("🔍 Log path: {}", logger.log_path().display());
        self.open = true;
        self.agent_id = Some(agent_id);
        self.agent_name = agent_name.clone();
        self.log_path = logger.log_path().clone();
        self.refresh_log_content();
        self.scroll_to_bottom = true;
        tracing::info!("🔍 Agent log window opened: open={}, agent_name={}", self.open, self.agent_name);
    }

    /// Refresh log content from file
    fn refresh_log_content(&mut self) {
        match fs::read_to_string(&self.log_path) {
            Ok(content) => {
                self.log_content = content;
                self.last_refresh = std::time::Instant::now();
            }
            Err(e) => {
                self.log_content = format!("Error reading log file: {}", e);
            }
        }
    }

    /// Filter log content based on current filter setting
    fn get_filtered_content(&self) -> String {
        if self.filter == LogFilter::All && self.search_query.is_empty() {
            return self.log_content.clone();
        }

        let mut filtered_lines = Vec::new();
        let mut in_relevant_block = false;
        let mut current_block = Vec::new();

        for line in self.log_content.lines() {
            // Check if this line starts a new log entry
            let is_event_start = line.starts_with('[') || line.starts_with("===") || line.starts_with("🤖");

            if is_event_start {
                // Process previous block
                if in_relevant_block && !current_block.is_empty() {
                    filtered_lines.extend(current_block.clone());
                }

                // Start new block
                current_block.clear();
                in_relevant_block = self.matches_filter(line);
            }

            current_block.push(line);
        }

        // Don't forget the last block
        if in_relevant_block && !current_block.is_empty() {
            filtered_lines.extend(current_block);
        }

        filtered_lines.join("\n")
    }

    /// Check if a log line matches the current filter
    fn matches_filter(&self, line: &str) -> bool {
        // Apply filter
        let filter_match = match self.filter {
            LogFilter::All => true,
            LogFilter::Messages => {
                line.contains("USER_MESSAGE")
                    || line.contains("ASSISTANT_RESPONSE")
                    || line.contains("SYSTEM_MESSAGE")
            }
            LogFilter::ModelCalls => {
                line.contains("MODEL_REQUEST")
                    || line.contains("MODEL_RESPONSE")
            }
            LogFilter::Tools => {
                line.contains("TOOL_START")
                    || line.contains("TOOL_COMPLETE")
                    || line.contains("TOOL_FAILED")
                    || line.contains("SUBTASK_CREATED")
            }
            LogFilter::Errors => {
                line.contains("❌ ERROR") || line.contains("TOOL_FAILED")
            }
            LogFilter::Lifecycle => {
                line.contains("AGENT_CREATED")
                    || line.contains("AGENT_RENAMED")
                    || line.contains("MODEL_CHANGED")
                    || line.contains("AGENT_TERMINATED")
                    || line.contains("AGENT SESSION STARTED")
            }
        };

        // Apply search query if present
        if !self.search_query.is_empty() {
            filter_match && line.to_lowercase().contains(&self.search_query.to_lowercase())
        } else {
            filter_match
        }
    }

    pub fn show(&mut self, ctx: &Context, bring_to_front: bool) {
        // Removed: floods logs on every frame
        // tracing::debug!("🔍 AgentLogWindow::show called: open={}, agent_name={}", self.open, self.agent_name);

        // Auto-refresh if enabled (every 2 seconds)
        if self.auto_refresh && self.last_refresh.elapsed().as_secs() >= 2 {
            self.refresh_log_content();
            self.scroll_to_bottom = true;
        }

        let mut is_open = self.open;

        let mut window = egui::Window::new(format!("Agent Log - {}", self.agent_name))
            .open(&mut is_open)
            .default_size([900.0, 700.0])
            .resizable(true);

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            self.render_toolbar(ui);
            ui.separator();
            self.render_log_content(ui);
        });

        self.open = is_open;
    }

    fn render_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Search box
            ui.label("Search:");
            let search_response = TextEdit::singleline(&mut self.search_query)
                .desired_width(200.0)
                .hint_text("🔍 Search logs...")
                .show(ui);

            if search_response.response.changed() {
                // Search query changed, no need to scroll
            }

            ui.separator();

            // Filter dropdown
            ui.label("Filter:");
            egui::ComboBox::from_id_salt("log_filter")
                .selected_text(format!("{:?}", self.filter))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filter, LogFilter::All, "All Events");
                    ui.selectable_value(&mut self.filter, LogFilter::Messages, "Messages");
                    ui.selectable_value(&mut self.filter, LogFilter::ModelCalls, "Model Calls");
                    ui.selectable_value(&mut self.filter, LogFilter::Tools, "Tools");
                    ui.selectable_value(&mut self.filter, LogFilter::Errors, "Errors");
                    ui.selectable_value(&mut self.filter, LogFilter::Lifecycle, "Lifecycle");
                });

            ui.separator();

            // Refresh button
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_log_content();
                self.scroll_to_bottom = true;
            }

            // Auto-refresh toggle
            ui.checkbox(&mut self.auto_refresh, "Auto-refresh");

            ui.separator();

            // Scroll to bottom button
            if ui.button("⬇ Bottom").clicked() {
                self.scroll_to_bottom = true;
            }

            // Copy path button
            if ui.button("📋 Copy Path").clicked() {
                ui.ctx().copy_text(self.log_path.display().to_string());
            }
        });

        // Second row: Log file path
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("Log file: {}", self.log_path.display()))
                    .small()
                    .color(Color32::GRAY),
            );
        });
    }

    fn render_log_content(&mut self, ui: &mut Ui) {
        let filtered_content = self.get_filtered_content();

        // Calculate available height
        let available_height = ui.available_height();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(available_height)
            .show(ui, |ui| {
                // Use monospace font for log content
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                if filtered_content.is_empty() {
                    ui.label(
                        RichText::new("No log entries match the current filter.")
                            .color(Color32::GRAY)
                            .italics(),
                    );
                } else {
                    // Highlight search matches
                    if self.search_query.is_empty() {
                        ui.label(&filtered_content);
                    } else {
                        // Simple highlight by splitting on search term
                        let query_lower = self.search_query.to_lowercase();
                        for line in filtered_content.lines() {
                            let line_lower = line.to_lowercase();
                            if line_lower.contains(&query_lower) {
                                ui.label(
                                    RichText::new(line)
                                        .background_color(Color32::from_rgb(80, 80, 0))
                                );
                            } else {
                                ui.label(line);
                            }
                        }
                    }
                }

                // Auto-scroll to bottom if requested
                if self.scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    self.scroll_to_bottom = false;
                }
            });
    }
}

impl FocusableWindow for AgentLogWindow {
    type ShowParams = ();

    fn window_id(&self) -> &'static str {
        "agent_log"
    }

    fn window_title(&self) -> String {
        format!("Agent Log - {}", self.agent_name)
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        _params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        self.show(ctx, bring_to_front);
    }
}

impl Default for AgentLogWindow {
    fn default() -> Self {
        Self::new()
    }
}
