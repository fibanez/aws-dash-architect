#![warn(clippy::all, rust_2018_idioms)]

use super::window_focus::FocusableWindow;
use eframe::egui;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_LOG_LINES: usize = 1000;
const UPDATE_INTERVAL_MS: u64 = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "ERROR" | "ERRO" => LogLevel::Error,
            "WARN" | "WARNING" => LogLevel::Warn,
            "INFO" => LogLevel::Info,
            "DEBUG" | "DEBG" => LogLevel::Debug,
            "TRACE" | "TRCE" => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    fn should_show(&self, filter_level: &LogLevel) -> bool {
        match filter_level {
            LogLevel::Error => matches!(self, LogLevel::Error),
            LogLevel::Warn => matches!(self, LogLevel::Error | LogLevel::Warn),
            LogLevel::Info => matches!(self, LogLevel::Error | LogLevel::Warn | LogLevel::Info),
            LogLevel::Debug => matches!(
                self,
                LogLevel::Error | LogLevel::Warn | LogLevel::Info | LogLevel::Debug
            ),
            LogLevel::Trace => true, // Show all
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

#[derive(Clone)]
pub struct LogMessage {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub full_line: String,
}

pub struct LogWindow {
    pub open: bool,
    log_path: PathBuf,
    log_messages: Arc<Mutex<VecDeque<LogMessage>>>,
    log_receiver: Option<Receiver<Vec<LogMessage>>>,
    log_sender: Option<Sender<Vec<LogMessage>>>,
    auto_scroll: bool,
    search_query: String,
    filter_level: LogLevel,
    watcher_thread: Option<thread::JoinHandle<()>>,
}

impl Default for LogWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl LogWindow {
    pub fn new() -> Self {
        // Get the log file path
        let log_path = Self::get_log_path();

        // Create channel for communication
        let (sender, receiver) = channel();

        let mut window = Self {
            open: false,
            log_path: log_path.clone(),
            log_messages: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES))),
            log_receiver: Some(receiver),
            log_sender: Some(sender),
            auto_scroll: true,
            search_query: String::new(),
            filter_level: LogLevel::Info, // Default to INFO level
            watcher_thread: None,
        };

        // Start the file watcher thread
        window.start_watcher();

        window
    }

    fn get_log_path() -> PathBuf {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
            let log_dir = proj_dirs.data_dir().join("logs");
            log_dir.join("awsdash.log")
        } else {
            // Fallback path
            PathBuf::from("./awsdash.log")
        }
    }

    fn start_watcher(&mut self) {
        let log_path = self.log_path.clone();
        let sender = self.log_sender.as_ref().unwrap().clone();
        let _messages = self.log_messages.clone();

        // Start the watcher thread
        let handle = thread::spawn(move || {
            let mut last_position = 0u64;

            loop {
                thread::sleep(Duration::from_millis(UPDATE_INTERVAL_MS));

                // Try to open the file
                let file = match File::open(&log_path) {
                    Ok(f) => f,
                    Err(_) => continue, // File doesn't exist yet
                };

                let mut reader = BufReader::new(file);

                // Get current file size
                if let Ok(metadata) = std::fs::metadata(&log_path) {
                    let current_size = metadata.len();

                    // If file was truncated or is new, reset position
                    if current_size < last_position {
                        last_position = 0;
                    }

                    // Seek to last position
                    if reader.seek(SeekFrom::Start(last_position)).is_ok() {
                        let mut new_messages = Vec::new();
                        let mut line = String::new();

                        // Read new lines
                        while reader.read_line(&mut line).unwrap_or(0) > 0 {
                            if !line.trim().is_empty() {
                                if let Some(msg) = Self::parse_log_line(&line) {
                                    new_messages.push(msg);
                                }
                            }
                            line.clear();
                        }

                        // Update position
                        if let Ok(pos) = reader.stream_position() {
                            last_position = pos;
                        }

                        // Send new messages
                        if !new_messages.is_empty() {
                            let _ = sender.send(new_messages);
                        }
                    }
                }
            }
        });

        self.watcher_thread = Some(handle);
    }

    fn parse_log_line(line: &str) -> Option<LogMessage> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Parse tracing/log format: TIMESTAMP LEVEL MODULE: MESSAGE
        // Example: 2025-05-30T00:20:07.991790Z DEBUG awsdash::app::dashui::menu: Log button clicked
        let parts: Vec<&str> = trimmed.splitn(4, ' ').collect();

        if parts.len() >= 3 {
            // Extract timestamp (first part)
            let timestamp = parts[0].to_string();

            // Extract log level (second part)
            let level = parts[1].to_string();

            // The rest is module and message
            if parts.len() >= 3 {
                // Find where the module ends and message begins (after the colon)
                let module_and_message = parts[2..].join(" ");

                // Split on first colon to separate module from message
                if let Some(colon_pos) = module_and_message.find(':') {
                    let module = module_and_message[..colon_pos].to_string();
                    let message = module_and_message[colon_pos + 1..].trim().to_string();

                    return Some(LogMessage {
                        timestamp,
                        level,
                        message: format!("{}: {}", module, message),
                        full_line: line.to_string(),
                    });
                }
            }
        }

        // Fallback for simple-logging format: TIMESTAMP [LEVEL] MESSAGE
        // Example: 2025-05-30 12:34:56 [INFO] Some message
        // Also handles: 2025-05-30 12:34:56.123 INFO Some message
        if let Some(bracket_start) = trimmed.find('[') {
            if let Some(bracket_end) = trimmed.find(']') {
                if bracket_end > bracket_start {
                    let timestamp = trimmed[..bracket_start].trim().to_string();
                    let level = trimmed[bracket_start + 1..bracket_end].to_string();
                    let message = trimmed[bracket_end + 1..].trim().to_string();

                    return Some(LogMessage {
                        timestamp,
                        level,
                        message,
                        full_line: line.to_string(),
                    });
                }
            }
        }

        // Another fallback for space-separated format without brackets
        // Example: 2025-05-30 12:34:56.123 INFO Some message
        let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            // Check if second part looks like a log level
            let potential_level = parts[1].to_uppercase();
            if matches!(
                potential_level.as_str(),
                "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE"
            ) {
                return Some(LogMessage {
                    timestamp: parts[0].to_string(),
                    level: potential_level,
                    message: parts[2].to_string(),
                    full_line: line.to_string(),
                });
            }
        }

        // Fallback: treat whole line as message
        Some(LogMessage {
            timestamp: String::new(),
            level: "INFO".to_string(),
            message: trimmed.to_string(),
            full_line: line.to_string(),
        })
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.show_with_focus(ctx, false);
    }

    pub fn show_with_focus(&mut self, ctx: &egui::Context, bring_to_front: bool) {
        if !self.open {
            return;
        }

        // Process any new messages
        if let Some(receiver) = &self.log_receiver {
            while let Ok(new_messages) = receiver.try_recv() {
                if let Ok(mut messages) = self.log_messages.lock() {
                    for msg in new_messages {
                        messages.push_back(msg);

                        // Remove old messages if we exceed the limit
                        while messages.len() > MAX_LOG_LINES {
                            messages.pop_front();
                        }
                    }
                }
            }
        }

        // Get the available screen rect to constrain window size
        let screen_rect = ctx.screen_rect();
        let max_width = screen_rect.width() * 0.9; // 90% of screen width
        let max_height = screen_rect.height() * 0.9; // 90% of screen height

        // Calculate default size that fits within screen bounds
        let default_width = 800.0_f32.min(max_width);
        let default_height = 400.0_f32.min(max_height);

        let mut window = egui::Window::new("Log Viewer")
            .open(&mut self.open)
            .default_size([default_width, default_height])
            .max_size([max_width, max_height])
            .constrain(true) // Ensure window stays within screen bounds
            .resizable(true)
            .movable(true);

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            // Top bar with file path and controls
            ui.horizontal(|ui| {
                ui.label("Log file:");
                ui.monospace(self.log_path.display().to_string());

                ui.separator();

                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");

                ui.separator();

                ui.label("Level:");
                egui::ComboBox::from_label("Filter Level")
                    .selected_text(self.filter_level.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filter_level, LogLevel::Error, "ERROR");
                        ui.selectable_value(&mut self.filter_level, LogLevel::Warn, "WARN");
                        ui.selectable_value(&mut self.filter_level, LogLevel::Info, "INFO");
                        ui.selectable_value(&mut self.filter_level, LogLevel::Debug, "DEBUG");
                        ui.selectable_value(&mut self.filter_level, LogLevel::Trace, "TRACE");
                    });

                ui.separator();

                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query);

                if ui.button("Clear").clicked() {
                    if let Ok(mut messages) = self.log_messages.lock() {
                        messages.clear();
                    }
                }
            });

            ui.separator();

            // Log content area
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .stick_to_bottom(self.auto_scroll)
                .show(ui, |ui| {
                    if let Ok(messages) = self.log_messages.lock() {
                        let total_messages = messages.len();
                        let mut shown_messages = 0;

                        for msg in messages.iter() {
                            // Filter by log level
                            let msg_level = LogLevel::from_str(&msg.level);
                            if !msg_level.should_show(&self.filter_level) {
                                continue;
                            }

                            // Filter by search query
                            if !self.search_query.is_empty()
                                && !msg
                                    .full_line
                                    .to_lowercase()
                                    .contains(&self.search_query.to_lowercase())
                            {
                                continue;
                            }

                            shown_messages += 1;

                            ui.horizontal(|ui| {
                                // Set smaller font size for log lines
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                ui.style_mut().text_styles.insert(
                                    egui::TextStyle::Monospace,
                                    egui::FontId::new(10.0, egui::FontFamily::Monospace),
                                );

                                // Timestamp
                                if !msg.timestamp.is_empty() {
                                    ui.monospace(&msg.timestamp);
                                }

                                // Level with color
                                let (level_color, level_text) = match msg.level.as_str() {
                                    "ERROR" | "ERRO" => {
                                        (egui::Color32::from_rgb(255, 100, 100), "ERROR")
                                    }
                                    "WARN" | "WARNING" => {
                                        (egui::Color32::from_rgb(255, 200, 100), "WARN")
                                    }
                                    "INFO" => (egui::Color32::from_rgb(100, 200, 255), "INFO"),
                                    "DEBUG" | "DEBG" => {
                                        (egui::Color32::from_rgb(150, 150, 150), "DEBUG")
                                    }
                                    "TRACE" | "TRCE" => {
                                        (egui::Color32::from_rgb(120, 120, 120), "TRACE")
                                    }
                                    _ => {
                                        (egui::Color32::from_rgb(200, 200, 200), msg.level.as_str())
                                    }
                                };

                                ui.colored_label(level_color, level_text);

                                // Message
                                ui.monospace(&msg.message);
                            });
                        }

                        // Show filter status at the bottom
                        if shown_messages < total_messages {
                            ui.separator();
                            ui.label(format!(
                                "Showing {} of {} messages (filtered by level: {})",
                                shown_messages,
                                total_messages,
                                self.filter_level.as_str()
                            ));
                        }
                    }
                });
        });

        // Request repaint to show updates
        ctx.request_repaint_after(Duration::from_millis(UPDATE_INTERVAL_MS));
    }

    pub fn show_with_offset(&mut self, ctx: &egui::Context, _offset: egui::Vec2) {
        if !self.open {
            return;
        }

        // Process any new messages
        if let Some(receiver) = &self.log_receiver {
            while let Ok(new_messages) = receiver.try_recv() {
                if let Ok(mut messages) = self.log_messages.lock() {
                    for msg in new_messages {
                        messages.push_back(msg);

                        // Remove old messages if we exceed the limit
                        while messages.len() > MAX_LOG_LINES {
                            messages.pop_front();
                        }
                    }
                }
            }
        }

        // Get the available screen rect to constrain window size
        let screen_rect = ctx.screen_rect();
        let max_width = screen_rect.width() * 0.9; // 90% of screen width
        let max_height = screen_rect.height() * 0.9; // 90% of screen height

        // Calculate default size that fits within screen bounds
        let default_width = 800.0_f32.min(max_width);
        let default_height = 400.0_f32.min(max_height);

        egui::Window::new("Log Viewer")
            .open(&mut self.open)
            .default_size([default_width, default_height])
            .max_size([max_width, max_height])
            .constrain(true) // Ensure window stays within screen bounds
            .resizable(true)
            .movable(true)
            .show(ctx, |ui| {
                // Top bar with file path and controls
                ui.horizontal(|ui| {
                    ui.label("Log file:");
                    ui.monospace(self.log_path.display().to_string());

                    ui.separator();

                    ui.checkbox(&mut self.auto_scroll, "Auto-scroll");

                    ui.separator();

                    ui.label("Level:");
                    egui::ComboBox::from_label("Filter Level")
                        .selected_text(self.filter_level.as_str())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.filter_level, LogLevel::Error, "ERROR");
                            ui.selectable_value(&mut self.filter_level, LogLevel::Warn, "WARN");
                            ui.selectable_value(&mut self.filter_level, LogLevel::Info, "INFO");
                            ui.selectable_value(&mut self.filter_level, LogLevel::Debug, "DEBUG");
                            ui.selectable_value(&mut self.filter_level, LogLevel::Trace, "TRACE");
                        });

                    ui.separator();

                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search_query);

                    if ui.button("Clear").clicked() {
                        if let Ok(mut messages) = self.log_messages.lock() {
                            messages.clear();
                        }
                    }
                });

                ui.separator();

                // Log content area
                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .stick_to_bottom(self.auto_scroll)
                    .show(ui, |ui| {
                        if let Ok(messages) = self.log_messages.lock() {
                            let total_messages = messages.len();
                            let mut shown_messages = 0;

                            for msg in messages.iter() {
                                // Filter by log level
                                let msg_level = LogLevel::from_str(&msg.level);
                                if !msg_level.should_show(&self.filter_level) {
                                    continue;
                                }

                                // Filter by search query
                                if !self.search_query.is_empty()
                                    && !msg
                                        .full_line
                                        .to_lowercase()
                                        .contains(&self.search_query.to_lowercase())
                                {
                                    continue;
                                }

                                shown_messages += 1;

                                ui.horizontal(|ui| {
                                    // Set smaller font size for log lines
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                    ui.style_mut().text_styles.insert(
                                        egui::TextStyle::Monospace,
                                        egui::FontId::new(10.0, egui::FontFamily::Monospace),
                                    );

                                    // Timestamp
                                    if !msg.timestamp.is_empty() {
                                        ui.monospace(&msg.timestamp);
                                    }

                                    // Level with color
                                    let (level_color, level_text) = match msg.level.as_str() {
                                        "ERROR" | "ERRO" => {
                                            (egui::Color32::from_rgb(255, 100, 100), "ERROR")
                                        }
                                        "WARN" | "WARNING" => {
                                            (egui::Color32::from_rgb(255, 200, 100), "WARN")
                                        }
                                        "INFO" => (egui::Color32::from_rgb(100, 200, 255), "INFO"),
                                        "DEBUG" | "DEBG" => {
                                            (egui::Color32::from_rgb(150, 150, 150), "DEBUG")
                                        }
                                        "TRACE" | "TRCE" => {
                                            (egui::Color32::from_rgb(120, 120, 120), "TRACE")
                                        }
                                        _ => (
                                            egui::Color32::from_rgb(200, 200, 200),
                                            msg.level.as_str(),
                                        ),
                                    };

                                    ui.colored_label(level_color, level_text);

                                    // Message
                                    ui.monospace(&msg.message);
                                });
                            }

                            // Show filter status at the bottom
                            if shown_messages < total_messages {
                                ui.separator();
                                ui.label(format!(
                                    "Showing {} of {} messages (filtered by level: {})",
                                    shown_messages,
                                    total_messages,
                                    self.filter_level.as_str()
                                ));
                            }
                        }
                    });
            });

        // Request repaint to show updates
        ctx.request_repaint_after(Duration::from_millis(UPDATE_INTERVAL_MS));
    }
}

impl FocusableWindow for LogWindow {
    type ShowParams = super::window_focus::SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "log_window"
    }

    fn window_title(&self) -> String {
        "Log Viewer".to_string()
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
        // Call the existing show_with_focus method
        LogWindow::show_with_focus(self, ctx, bring_to_front);
    }
}

impl Drop for LogWindow {
    fn drop(&mut self) {
        // The watcher thread will be automatically terminated when the sender is dropped
    }
}
