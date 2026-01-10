//! Per-Agent Logging System
//!
//! Each Agent Instance maintains its own dedicated log file for tracking:
//! - Conversation messages (user, assistant, system)
//! - Model interactions (requests, responses, token usage)
//! - Tool executions (start, success, failure with timing)
//! - Sub-task agent creation and progress
//! - Agent lifecycle events (creation, rename, termination)
//!
//! Log files are stored at: `~/.local/share/awsdash/logs/agents/{YYYYMMDDHHmm}-Agent-{uuid}.log`

#![warn(clippy::all, rust_2018_idioms)]

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::error;

use crate::app::agent_framework::{AgentId, AgentMetadata, AgentStatus, AgentType};

// Thread-local storage for current agent logger (used by tools to log to per-agent logs)
thread_local! {
    static CURRENT_AGENT_LOGGER: RefCell<Option<Arc<AgentLogger>>> = const { RefCell::new(None) };
}

/// Set the current agent logger for this thread (used by tools like execute_javascript)
pub fn set_current_agent_logger(logger: Option<Arc<AgentLogger>>) {
    CURRENT_AGENT_LOGGER.with(|cell| {
        *cell.borrow_mut() = logger;
    });
}

/// Get the current agent logger for this thread (if any)
pub fn get_current_agent_logger() -> Option<Arc<AgentLogger>> {
    CURRENT_AGENT_LOGGER.with(|cell| cell.borrow().clone())
}

/// Token usage statistics from model responses
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// Per-agent logger for tracking all agent activity
#[derive(Debug)]
pub struct AgentLogger {
    #[allow(dead_code)] // Stored for potential debugging/future use
    agent_id: AgentId,
    agent_name: Arc<Mutex<String>>,
    file_writer: Arc<Mutex<std::fs::File>>,
    log_path: PathBuf,
    session_start: DateTime<Utc>,
}

impl AgentLogger {
    /// Create a new agent logger with dedicated log file
    pub fn new(
        agent_id: AgentId,
        agent_name: String,
        agent_type: &AgentType,
    ) -> Result<Self, std::io::Error> {
        let log_path = Self::get_log_path(agent_id, agent_type)?;

        // Ensure parent directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create or open the log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let session_start = Utc::now();

        // Write session header
        let mut file_handle = file;
        writeln!(file_handle, "\n{}", "=".repeat(80))?;
        writeln!(
            file_handle,
            "🤖 AGENT SESSION STARTED: {}",
            session_start.format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        writeln!(file_handle, "Agent ID: {}", agent_id)?;
        writeln!(file_handle, "Agent Name: {}", agent_name)?;
        writeln!(file_handle, "Agent Type: {}", agent_type)?;
        writeln!(file_handle, "{}\n", "=".repeat(80))?;
        file_handle.flush()?;

        Ok(Self {
            agent_id,
            agent_name: Arc::new(Mutex::new(agent_name)),
            file_writer: Arc::new(Mutex::new(file_handle)),
            log_path,
            session_start,
        })
    }

    /// Get the log file path for an agent
    fn get_log_path(agent_id: AgentId, agent_type: &AgentType) -> Result<PathBuf, std::io::Error> {
        // Generate timestamp prefix: YYYYMMDDHHmm format
        let timestamp = Utc::now().format("%Y%m%d%H%M").to_string();

        // Generate filename based on agent type and parent relationship
        let filename = match agent_type {
            AgentType::TaskManager => {
                // Parent: 202601071622-Manager-a6e248e3.log
                format!("{}-Manager-{}.log", timestamp, Self::short_uuid(&agent_id))
            }
            AgentType::TaskWorker { parent_id } => {
                // Worker: 202601071622-Manager-a6e248e3-Worker-2f1cdcb1.log
                format!(
                    "{}-Manager-{}-Worker-{}.log",
                    timestamp,
                    Self::short_uuid(parent_id),
                    Self::short_uuid(&agent_id)
                )
            }
            AgentType::PageBuilderWorker { parent_id, .. } => {
                // ToolBuilder Worker: 202601071622-Manager-a6e248e3-ToolBuilder-fb624dd7.log
                format!(
                    "{}-Manager-{}-ToolBuilder-{}.log",
                    timestamp,
                    Self::short_uuid(parent_id),
                    Self::short_uuid(&agent_id)
                )
            }
        };

        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
            let log_dir = proj_dirs.data_dir().join("logs").join("agents");
            Ok(log_dir.join(filename))
        } else {
            // Fallback to current directory
            Ok(PathBuf::from(filename))
        }
    }

    /// Get last 8 characters of UUID for short display
    fn short_uuid(agent_id: &AgentId) -> String {
        let uuid_str = agent_id.to_string();
        uuid_str.chars().rev().take(8).collect::<String>().chars().rev().collect()
    }

    /// Get the current log file path (for UI display)
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Get the debug log file path for an agent (for Stood library traces)
    /// NOTE: Debug logs are deprecated - stood traces now go to per-agent logs via AgentTracingLayer
    pub fn get_debug_log_path(agent_id: AgentId, agent_type: &AgentType) -> Result<PathBuf, std::io::Error> {
        // Generate timestamp prefix: YYYYMMDDHHmm format
        let timestamp = Utc::now().format("%Y%m%d%H%M").to_string();

        // Use same naming pattern as main logs but with -debug suffix
        let filename = match agent_type {
            AgentType::TaskManager => {
                format!("{}-Manager-{}-debug.log", timestamp, Self::short_uuid(&agent_id))
            }
            AgentType::TaskWorker { parent_id } => {
                format!(
                    "{}-Manager-{}-Worker-{}-debug.log",
                    timestamp,
                    Self::short_uuid(parent_id),
                    Self::short_uuid(&agent_id)
                )
            }
            AgentType::PageBuilderWorker { parent_id, .. } => {
                format!(
                    "{}-Manager-{}-ToolBuilder-{}-debug.log",
                    timestamp,
                    Self::short_uuid(parent_id),
                    Self::short_uuid(&agent_id)
                )
            }
        };

        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
            let log_dir = proj_dirs.data_dir().join("logs").join("agents");
            Ok(log_dir.join(filename))
        } else {
            // Fallback to current directory
            Ok(PathBuf::from(filename))
        }
    }

    /// Update agent name (when renamed via UI)
    pub fn update_agent_name(&self, agent_type: &AgentType, new_name: String) {
        if let Ok(mut name) = self.agent_name.lock() {
            *name = new_name.clone();
        }

        self.write_event(&format!(
            "{} 📝 AGENT_RENAMED\n    New Name: {}",
            Self::timestamp_with_type(agent_type),
            new_name
        ));
    }

    /// Log agent creation event
    pub fn log_agent_created(&self, agent_type: &AgentType, metadata: &AgentMetadata) {
        self.write_event(&format!(
            "{} 🚀 AGENT_CREATED\n    Description: {}",
            Self::timestamp_with_type(agent_type),
            metadata.description
        ));
    }

    /// Log user message
    pub fn log_user_message(&self, agent_type: &AgentType, message: &str) {
        self.write_event(&format!(
            "{} 👤 USER_MESSAGE\n    Message: \"{}\"",
            Self::timestamp_with_type(agent_type),
            message
        ));
    }

    /// Log assistant response
    pub fn log_assistant_response(&self, agent_type: &AgentType, response: &str) {
        self.write_event(&format!(
            "{} ⚡ ASSISTANT_RESPONSE\n    Response:\n    ┌─────────────────────────────────────────────────────────────────\n{}\n    └─────────────────────────────────────────────────────────────────",
            Self::timestamp_with_type(agent_type),
            Self::indent_lines(response, 4)
        ));
    }

    /// Log system message
    pub fn log_system_message(&self, agent_type: &AgentType, message: &str) {
        self.write_event(&format!(
            "{} ℹ SYSTEM_MESSAGE\n    Message: \"{}\"",
            Self::timestamp_with_type(agent_type),
            message
        ));
    }

    /// Log error message
    pub fn log_error(&self, agent_type: &AgentType, error: &str) {
        self.write_event(&format!(
            "{} ❌ ERROR\n    Error: {}",
            Self::timestamp_with_type(agent_type),
            error
        ));

        // Flush immediately for errors - we want these persisted right away
        if let Ok(mut writer) = self.file_writer.lock() {
            let _ = writer.flush();
        }
    }

    /// Log model request sent
    pub fn log_model_request(
        &self,
        agent_type: &AgentType,
        system_prompt: &str,
        user_message: &str,
        model_id: &str,
    ) {
        self.write_event(&format!(
            "{} 📤 MODEL_REQUEST_SENT\n    Model: {}\n    User Message: \"{}\"\n    System Prompt:\n    ┌─────────────────────────────────────────────────────────────────\n{}\n    └─────────────────────────────────────────────────────────────────",
            Self::timestamp_with_type(agent_type),
            model_id,
            user_message,
            Self::indent_lines(system_prompt, 4) // No truncation - show full prompt
        ));
    }

    /// Log model response received
    pub fn log_model_response(
        &self,
        agent_type: &AgentType,
        response: &str,
        stop_reason: &str,
        duration_ms: u64,
        tokens: Option<TokenUsage>,
    ) {
        let token_info = if let Some(t) = tokens {
            format!(
                "\n    Tokens: input={}, output={}, total={}",
                t.input_tokens, t.output_tokens, t.total_tokens
            )
        } else {
            String::new()
        };

        self.write_event(&format!(
            "{} 📥 MODEL_RESPONSE_RECEIVED\n    Stop Reason: {}\n    Duration: {}ms{}\n    Full Response:\n    ┌─────────────────────────────────────────────────────────────────\n{}\n    └─────────────────────────────────────────────────────────────────",
            Self::timestamp_with_type(agent_type),
            stop_reason,
            duration_ms,
            token_info,
            Self::indent_lines(response, 4)
        ));
    }

    /// Log tool execution start
    pub fn log_tool_start(&self, agent_type: &AgentType, tool_name: &str, input: &Value) {
        let formatted_input =
            serde_json::to_string_pretty(input).unwrap_or_else(|_| "Invalid JSON".to_string());

        // Prettify JavaScript code to display with actual newlines
        let prettified_input = Self::prettify_json_for_display(&formatted_input, tool_name);

        self.write_event(&format!(
            "{} 🔧 TOOL_START\n    Tool Name: {}\n    Input Parameters:\n    ┌─────────────────────────────────────────────────────────────────\n{}\n    └─────────────────────────────────────────────────────────────────",
            Self::timestamp_with_type(agent_type),
            tool_name,
            Self::indent_lines(&prettified_input, 4)
        ));
    }

    /// Log tool execution completion
    pub fn log_tool_complete(
        &self,
        agent_type: &AgentType,
        tool_name: &str,
        output: Option<&Value>,
        duration: Duration,
    ) {
        let formatted_output = if let Some(out) = output {
            serde_json::to_string_pretty(out).unwrap_or_else(|_| "Invalid JSON".to_string())
        } else {
            "null".to_string()
        };

        self.write_event(&format!(
            "{} ✅ TOOL_COMPLETE\n    Tool Name: {}\n    Duration: {:.2}s\n    Output Result:\n    ┌─────────────────────────────────────────────────────────────────\n{}\n    └─────────────────────────────────────────────────────────────────",
            Self::timestamp_with_type(agent_type),
            tool_name,
            duration.as_secs_f64(),
            Self::indent_lines(&formatted_output, 4)
        ));
    }

    /// Log tool execution failure
    pub fn log_tool_failed(
        &self,
        agent_type: &AgentType,
        tool_name: &str,
        error: &str,
        duration: Duration,
    ) {
        self.write_event(&format!(
            "{} ❌ TOOL_FAILED\n    Tool Name: {}\n    Duration: {:.2}s\n    Error: {}",
            Self::timestamp_with_type(agent_type),
            tool_name,
            duration.as_secs_f64(),
            error
        ));
    }

    /// Log sub-task creation (create_task tool)
    pub fn log_subtask_created(
        &self,
        agent_type: &AgentType,
        task_id: &str,
        description: &str,
        accounts: &[String],
        regions: &[String],
    ) {
        self.write_event(&format!(
            "{} 🎯 SUBTASK_CREATED\n    Task ID: {}\n    Description: \"{}\"\n    Accounts: {:?}\n    Regions: {:?}",
            Self::timestamp_with_type(agent_type),
            task_id,
            description,
            accounts,
            regions
        ));
    }

    /// Log model change
    pub fn log_model_changed(&self, agent_type: &AgentType, old_model: &str, new_model: &str) {
        self.write_event(&format!(
            "{} MODEL_CHANGED\n    Old Model: {}\n    New Model: {}",
            Self::timestamp_with_type(agent_type),
            old_model,
            new_model
        ));
    }

    /// Log a stood library trace message
    ///
    /// Used by the custom tracing layer to capture stood::* events
    /// directly to the per-agent log file.
    ///
    /// # Arguments
    /// * `level` - The tracing level (TRACE, DEBUG, INFO, WARN, ERROR)
    /// * `target` - The tracing target (e.g., "stood::agent::execution")
    /// * `message` - The formatted trace message
    pub fn log_stood_trace(&self, level: &str, target: &str, message: &str) {
        // Use a more compact format for stood traces to avoid excessive verbosity
        let timestamp = Utc::now().format("%H:%M:%S%.3f").to_string();
        let formatted = format!(
            "[{}] [STOOD] [{}] {}: {}",
            timestamp, level, target, message
        );

        // Write directly without the extra newline prefix (stood traces can be frequent)
        if let Ok(mut writer) = self.file_writer.lock() {
            if let Err(e) = writeln!(writer, "{}", formatted) {
                error!("Failed to write stood trace to agent log: {}", e);
            }
            // No flush for stood traces - they're frequent and buffering is fine
        }
    }

    /// Log stood log level change
    pub fn log_stood_level_changed(
        &self,
        agent_type: &AgentType,
        old_level: &str,
        new_level: &str,
    ) {
        self.write_event(&format!(
            "{} STOOD_LOG_LEVEL_CHANGED\n    Old Level: {}\n    New Level: {}",
            Self::timestamp_with_type(agent_type),
            old_level,
            new_level
        ));
    }

    /// Log agent termination
    pub fn log_agent_terminated(&self, agent_type: &AgentType, final_status: &AgentStatus) {
        let status_text = match final_status {
            AgentStatus::Completed => "Completed Successfully",
            AgentStatus::Failed(err) => {
                self.write_event(&format!(
                    "{} 🏁 AGENT_TERMINATED\n    Status: Failed\n    Error: {}",
                    Self::timestamp_with_type(agent_type),
                    err
                ));

                // Flush immediately on termination
                if let Ok(mut writer) = self.file_writer.lock() {
                    let _ = writer.flush();
                }
                return;
            }
            AgentStatus::Cancelled => "Cancelled by User",
            _ => "Unknown",
        };

        let session_duration = Utc::now()
            .signed_duration_since(self.session_start)
            .num_milliseconds();

        self.write_event(&format!(
            "{} 🏁 AGENT_TERMINATED\n    Status: {}\n    Total Duration: {}ms\n    {}",
            Self::timestamp_with_type(agent_type),
            status_text,
            session_duration,
            "=".repeat(80)
        ));

        // Flush immediately on termination
        if let Ok(mut writer) = self.file_writer.lock() {
            let _ = writer.flush();
        }
    }

    // ============================================================================
    // HELPER METHODS
    // ============================================================================

    /// Write an event to the log file
    ///
    /// Note: Does NOT flush on every write to avoid blocking the UI thread.
    /// The OS will buffer writes and flush periodically, which is much faster.
    /// Flush only happens on critical events (errors, termination) or when the file is closed.
    fn write_event(&self, event: &str) {
        if let Ok(mut writer) = self.file_writer.lock() {
            if let Err(e) = writeln!(writer, "\n{}", event) {
                error!("Failed to write to agent log: {}", e);
            }
            // NO flush() here - let OS buffer the writes for performance
            // This prevents blocking the UI thread on disk I/O (can take 100-2000ms!)
        } else {
            error!("Failed to acquire agent logger lock");
        }
    }

    /// Get agent type label for log entries
    fn agent_type_label(agent_type: &AgentType) -> &str {
        match agent_type {
            AgentType::TaskManager => "MANAGER",
            AgentType::TaskWorker { .. } => "TASK WORKER",
            AgentType::PageBuilderWorker { .. } => "PAGE BUILDER WORKER",
        }
    }

    /// Get current timestamp for logging
    fn timestamp() -> String {
        Utc::now().format("%H:%M:%S").to_string()
    }

    /// Get timestamp with agent type label
    fn timestamp_with_type(agent_type: &AgentType) -> String {
        format!(
            "[{}] [{}]",
            Self::timestamp(),
            Self::agent_type_label(agent_type)
        )
    }

    /// Prettify JSON for display - specifically handles JavaScript code with escaped newlines
    ///
    /// For execute_javascript tool, this extracts the "code" field and replaces escaped
    /// newlines (\n) with actual newlines so the JavaScript displays as formatted code.
    fn prettify_json_for_display(json_str: &str, tool_name: &str) -> String {
        if tool_name != "execute_javascript" {
            return json_str.to_string();
        }

        // Try to parse as JSON and extract code field
        if let Ok(json_value) = serde_json::from_str::<Value>(json_str) {
            if let Some(code_str) = json_value.get("code").and_then(|v| v.as_str()) {
                // Found JavaScript code - replace escaped newlines with actual newlines
                // The code is already unescaped by serde_json, so we just need to format it nicely
                return format!("{{\n  \"code\": \"\"\"\n{}\n  \"\"\"\n}}", code_str);
            }
        }

        // Fallback to original if parsing fails or no code field found
        json_str.to_string()
    }

    /// Indent all lines in a string by N spaces, with 80-column wrapping
    fn indent_lines(text: &str, spaces: usize) -> String {
        let indent = " ".repeat(spaces);
        let prefix_len = spaces + 2; // spaces + "│ "
        let max_content_width = 80 - prefix_len;

        let mut result = Vec::new();

        for line in text.lines() {
            let content_len = line.len();
            if content_len <= max_content_width {
                result.push(format!("{}│ {}", indent, line));
            } else {
                // Wrap long lines
                let mut remaining = line;
                while remaining.len() > max_content_width {
                    // Find last space before max_content_width
                    if let Some(wrap_pos) = remaining[..max_content_width].rfind(' ') {
                        result.push(format!("{}│ {}", indent, &remaining[..wrap_pos]));
                        remaining = remaining[wrap_pos + 1..].trim_start();
                    } else {
                        // No space found, hard wrap
                        result.push(format!("{}│ {}", indent, &remaining[..max_content_width]));
                        remaining = &remaining[max_content_width..];
                    }
                }
                if !remaining.is_empty() {
                    result.push(format!("{}│ {}", indent, remaining));
                }
            }
        }

        result.join("\n")
    }

    /// Clean up old agent log files, keeping only the N most recent
    ///
    /// This helps prevent the logs directory from growing indefinitely.
    /// By default, keeps the 50 most recent agent log files.
    ///
    /// Returns the number of files deleted.
    pub fn cleanup_old_logs(keep_count: usize) -> Result<usize, std::io::Error> {
        let log_dir = if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash")
        {
            proj_dirs.data_dir().join("logs").join("agents")
        } else {
            return Ok(0); // Can't determine log directory
        };

        if !log_dir.exists() {
            return Ok(0); // Nothing to clean up
        }

        // Collect all agent log files with their metadata
        let mut log_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        for entry in std::fs::read_dir(&log_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Only process agent log files (not debug logs)
            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.ends_with(".log")
                        && !filename.ends_with("-debug.log")
                        && filename.contains("Agent-")
                    {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                log_files.push((path, modified));
                            }
                        }
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        log_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Delete files beyond keep_count
        let mut deleted = 0;
        for (path, _) in log_files.iter().skip(keep_count) {
            match std::fs::remove_file(path) {
                Ok(_) => {
                    deleted += 1;
                    tracing::info!(
                        "Deleted old agent log: {}",
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to delete old agent log {}: {}", path.display(), e);
                }
            }
        }

        Ok(deleted)
    }

    /// Clean up old debug log files, keeping only the N most recent
    ///
    /// Similar to cleanup_old_logs but specifically for debug logs.
    pub fn cleanup_old_debug_logs(keep_count: usize) -> Result<usize, std::io::Error> {
        let log_dir = if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash")
        {
            proj_dirs.data_dir().join("logs").join("agents")
        } else {
            return Ok(0);
        };

        if !log_dir.exists() {
            return Ok(0);
        }

        // Collect all debug log files with their metadata
        let mut debug_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

        for entry in std::fs::read_dir(&log_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.ends_with("-debug.log") && filename.contains("Agent-") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                debug_files.push((path, modified));
                            }
                        }
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        debug_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Delete files beyond keep_count
        let mut deleted = 0;
        for (path, _) in debug_files.iter().skip(keep_count) {
            match std::fs::remove_file(path) {
                Ok(_) => {
                    deleted += 1;
                    tracing::info!(
                        "Deleted old debug log: {}",
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to delete old debug log {}: {}", path.display(), e);
                }
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_logger_creation() {
        let agent_id = AgentId::new();
        let agent_type = AgentType::TaskManager;
        let logger = AgentLogger::new(agent_id, "Test Agent".to_string(), &agent_type)
            .expect("Failed to create logger");

        // Verify log path exists
        assert!(logger.log_path().exists());

        println!("Agent logger created successfully: {:?}", logger.log_path());
    }

    #[test]
    fn test_indent_lines() {
        let text = "line1\nline2\nline3";
        let indented = AgentLogger::indent_lines(text, 4);

        assert!(indented.contains("    │ line1"));
        assert!(indented.contains("    │ line2"));
        assert!(indented.contains("    │ line3"));
    }
}
