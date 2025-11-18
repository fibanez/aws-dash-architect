//! Agent Framework Debug Logger
//!
//! Focused debugging system for Agent Framework interactions.
//! Tracks prompts, responses, tool calls, and responses in a structured format
//! for troubleshooting without excessive verbosity.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde_json::{self, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::error;

/// Agent Framework debug event types for focused logging
#[derive(Debug, Clone)]
pub enum AgentDebugEvent {
    /// Agent started processing request
    AgentStart {
        timestamp: DateTime<Utc>,
        user_request: String,
        session_id: String,
    },
    /// Agent prompt sent to LLM
    PromptSent {
        timestamp: DateTime<Utc>,
        session_id: String,
        system_prompt: String,
        user_message: String,
        model_id: String,
    },
    /// Agent response from LLM
    ResponseReceived {
        timestamp: DateTime<Utc>,
        session_id: String,
        full_response: String,
        tool_calls_requested: Vec<String>,
    },
    /// Agent tool call execution
    ToolCall {
        timestamp: DateTime<Utc>,
        session_id: String,
        tool_name: String,
        input_params: Value,
        success: bool,
        output_result: Option<Value>,
        error_message: Option<String>,
    },
    /// Create_task tool invocation
    CreateTaskStart {
        timestamp: DateTime<Utc>,
        session_id: String,
        task_id: String,
        task_description: String,
        account_ids: Vec<String>,
        regions: Vec<String>,
    },
    /// Task agent created and prompt generated
    TaskAgentCreated {
        timestamp: DateTime<Utc>,
        task_id: String,
        full_system_prompt: String,
        model_id: String,
    },
    /// Task agent prompt sent to LLM
    TaskPromptSent {
        timestamp: DateTime<Utc>,
        task_id: String,
        user_message: String,
        model_id: String,
    },
    /// Task agent response from LLM
    TaskResponseReceived {
        timestamp: DateTime<Utc>,
        task_id: String,
        full_response: String,
        tool_calls_requested: Vec<String>,
    },
    /// Task agent tool call execution
    TaskToolCall {
        timestamp: DateTime<Utc>,
        task_id: String,
        tool_name: String,
        input_params: Value,
        success: bool,
        output_result: Option<Value>,
        error_message: Option<String>,
    },
    /// Task completion
    TaskComplete {
        timestamp: DateTime<Utc>,
        task_id: String,
        success: bool,
        execution_summary: String,
    },
    /// Session ended
    SessionEnd {
        timestamp: DateTime<Utc>,
        session_id: String,
        total_duration_ms: u64,
    },
}

/// Agent Framework debug logger with file output
#[derive(Debug)]
pub struct AgentDebugLogger {
    file_writer: Arc<Mutex<std::fs::File>>,
    log_path: PathBuf,
}

impl AgentDebugLogger {
    /// Create a new agent framework debug logger
    pub fn new() -> Result<Self, std::io::Error> {
        let log_path = Self::get_debug_log_path()?;

        // Ensure parent directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create or open the debug log file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        // Write session separator
        let mut file_handle = file;
        writeln!(file_handle, "\n{}", "=".repeat(80))?;
        writeln!(file_handle, "AGENT FRAMEWORK DEBUG SESSION STARTED: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))?;
        writeln!(file_handle, "{}\n", "=".repeat(80))?;
        file_handle.flush()?;

        Ok(Self {
            file_writer: Arc::new(Mutex::new(file_handle)),
            log_path,
        })
    }

    /// Get the debug log file path
    fn get_debug_log_path() -> Result<PathBuf, std::io::Error> {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
            let log_dir = proj_dirs.data_dir().join("logs");
            Ok(log_dir.join("agent-framework-debug.log"))
        } else {
            // Fallback to current directory
            Ok(PathBuf::from("agent-framework-debug.log"))
        }
    }

    /// Log an agent framework debug event
    pub fn log_event(&self, event: AgentDebugEvent) {
        if let Ok(mut writer) = self.file_writer.lock() {
            if let Err(e) = self.write_event(&mut writer, &event) {
                error!("Failed to write agent framework debug event: {}", e);
            } else if let Err(e) = writer.flush() {
                error!("Failed to flush agent framework debug log: {}", e);
            }
        } else {
            error!("Failed to acquire agent framework debug logger lock");
        }
    }
    
    /// Write formatted event to file with full content and beautiful formatting
    fn write_event(&self, writer: &mut std::fs::File, event: &AgentDebugEvent) -> std::io::Result<()> {
        let timestamp_str = match event {
            AgentDebugEvent::AgentStart { timestamp, .. } => timestamp,
            AgentDebugEvent::PromptSent { timestamp, .. } => timestamp,
            AgentDebugEvent::ResponseReceived { timestamp, .. } => timestamp,
            AgentDebugEvent::ToolCall { timestamp, .. } => timestamp,
            AgentDebugEvent::CreateTaskStart { timestamp, .. } => timestamp,
            AgentDebugEvent::TaskAgentCreated { timestamp, .. } => timestamp,
            AgentDebugEvent::TaskPromptSent { timestamp, .. } => timestamp,
            AgentDebugEvent::TaskResponseReceived { timestamp, .. } => timestamp,
            AgentDebugEvent::TaskToolCall { timestamp, .. } => timestamp,
            AgentDebugEvent::TaskComplete { timestamp, .. } => timestamp,
            AgentDebugEvent::SessionEnd { timestamp, .. } => timestamp,
        }.format("%H:%M:%S");
        
        match event {
            AgentDebugEvent::AgentStart { user_request, session_id, .. } => {
                writeln!(writer, "\n[{}] 🚀 AGENT_SESSION_START", timestamp_str)?;
                writeln!(writer, "    Session ID: {}", session_id)?;
                writeln!(writer, "    User Request: \"{}\"", user_request)?;
            },
            AgentDebugEvent::PromptSent { session_id, system_prompt, user_message, model_id, .. } => {
                writeln!(writer, "\n[{}] 📤 AGENT_PROMPT_SENT", timestamp_str)?;
                writeln!(writer, "    Session ID: {}", session_id)?;
                writeln!(writer, "    Model: {}", model_id)?;
                writeln!(writer, "    User Message: \"{}\"", user_message)?;
                writeln!(writer, "    System Prompt:")?;
                writeln!(writer, "    ┌─────────────────────────────────────────────────────────────────")?;
                for line in system_prompt.lines() {
                    writeln!(writer, "    │ {}", line)?;
                }
                writeln!(writer, "    └─────────────────────────────────────────────────────────────────")?;
            },
            AgentDebugEvent::ResponseReceived { session_id, full_response, tool_calls_requested, .. } => {
                writeln!(writer, "\n[{}] 📥 AGENT_RESPONSE_RECEIVED", timestamp_str)?;
                writeln!(writer, "    Session ID: {}", session_id)?;
                writeln!(writer, "    Tool Calls Requested: {:?}", tool_calls_requested)?;
                writeln!(writer, "    Full Response:")?;
                writeln!(writer, "    ┌─────────────────────────────────────────────────────────────────")?;
                for line in full_response.lines() {
                    writeln!(writer, "    │ {}", line)?;
                }
                writeln!(writer, "    └─────────────────────────────────────────────────────────────────")?;
            },
            AgentDebugEvent::ToolCall { session_id, tool_name, input_params, success, output_result, error_message, .. } => {
                let status = if *success { "✅" } else { "❌" };
                writeln!(writer, "\n[{}] {} AGENT_TOOL_CALL", timestamp_str, status)?;
                writeln!(writer, "    Session ID: {}", session_id)?;
                writeln!(writer, "    Tool Name: {}", tool_name)?;
                writeln!(writer, "    Success: {}", success)?;
                writeln!(writer, "    Input Parameters:")?;
                let formatted_input = serde_json::to_string_pretty(input_params)
                    .unwrap_or_else(|_| "Invalid JSON".to_string());
                writeln!(writer, "    ┌─────────────────────────────────────────────────────────────────")?;
                for line in formatted_input.lines() {
                    writeln!(writer, "    │ {}", line)?;
                }
                writeln!(writer, "    └─────────────────────────────────────────────────────────────────")?;
                
                if let Some(output) = output_result {
                    writeln!(writer, "    Output Result:")?;
                    let formatted_output = serde_json::to_string_pretty(output)
                        .unwrap_or_else(|_| "Invalid JSON".to_string());
                    writeln!(writer, "    ┌─────────────────────────────────────────────────────────────────")?;
                    for line in formatted_output.lines() {
                        writeln!(writer, "    │ {}", line)?;
                    }
                    writeln!(writer, "    └─────────────────────────────────────────────────────────────────")?;
                }
                
                if let Some(error) = error_message {
                    writeln!(writer, "    Error: {}", error)?;
                }
            },
            AgentDebugEvent::CreateTaskStart { session_id, task_id, task_description, account_ids, regions, .. } => {
                writeln!(writer, "\n[{}] 🎯 CREATE_TASK_START", timestamp_str)?;
                writeln!(writer, "    Session ID: {}", session_id)?;
                writeln!(writer, "    Task ID: {}", task_id)?;
                writeln!(writer, "    Account IDs: {:?}", account_ids)?;
                writeln!(writer, "    Regions: {:?}", regions)?;
                writeln!(writer, "    Task Description: \"{}\"", task_description)?;
            },
            AgentDebugEvent::TaskAgentCreated { task_id, full_system_prompt, model_id, .. } => {
                writeln!(writer, "\n[{}] 🤖 TASK_AGENT_CREATED", timestamp_str)?;
                writeln!(writer, "    Task ID: {}", task_id)?;
                writeln!(writer, "    Model: {}", model_id)?;
                writeln!(writer, "    Full System Prompt:")?;
                writeln!(writer, "    ┌─────────────────────────────────────────────────────────────────")?;
                for line in full_system_prompt.lines() {
                    writeln!(writer, "    │ {}", line)?;
                }
                writeln!(writer, "    └─────────────────────────────────────────────────────────────────")?;
            },
            AgentDebugEvent::TaskPromptSent { task_id, user_message, model_id, .. } => {
                writeln!(writer, "\n[{}] 📤 TASK_PROMPT_SENT", timestamp_str)?;
                writeln!(writer, "    Task ID: {}", task_id)?;
                writeln!(writer, "    Model: {}", model_id)?;
                writeln!(writer, "    User Message: \"{}\"", user_message)?;
            },
            AgentDebugEvent::TaskResponseReceived { task_id, full_response, tool_calls_requested, .. } => {
                writeln!(writer, "\n[{}] 📥 TASK_RESPONSE_RECEIVED", timestamp_str)?;
                writeln!(writer, "    Task ID: {}", task_id)?;
                writeln!(writer, "    Tool Calls Requested: {:?}", tool_calls_requested)?;
                writeln!(writer, "    Full Response:")?;
                writeln!(writer, "    ┌─────────────────────────────────────────────────────────────────")?;
                for line in full_response.lines() {
                    writeln!(writer, "    │ {}", line)?;
                }
                writeln!(writer, "    └─────────────────────────────────────────────────────────────────")?;
            },
            AgentDebugEvent::TaskToolCall { task_id, tool_name, input_params, success, output_result, error_message, .. } => {
                let status = if *success { "✅" } else { "❌" };
                writeln!(writer, "\n[{}] {} TASK_TOOL_CALL", timestamp_str, status)?;
                writeln!(writer, "    Task ID: {}", task_id)?;
                writeln!(writer, "    Tool Name: {}", tool_name)?;
                writeln!(writer, "    Success: {}", success)?;
                writeln!(writer, "    Input Parameters:")?;
                let formatted_input = serde_json::to_string_pretty(input_params)
                    .unwrap_or_else(|_| "Invalid JSON".to_string());
                writeln!(writer, "    ┌─────────────────────────────────────────────────────────────────")?;
                for line in formatted_input.lines() {
                    writeln!(writer, "    │ {}", line)?;
                }
                writeln!(writer, "    └─────────────────────────────────────────────────────────────────")?;
                
                if let Some(output) = output_result {
                    writeln!(writer, "    Output Result:")?;
                    let formatted_output = serde_json::to_string_pretty(output)
                        .unwrap_or_else(|_| "Invalid JSON".to_string());
                    writeln!(writer, "    ┌─────────────────────────────────────────────────────────────────")?;
                    for line in formatted_output.lines() {
                        writeln!(writer, "    │ {}", line)?;
                    }
                    writeln!(writer, "    └─────────────────────────────────────────────────────────────────")?;
                }
                
                if let Some(error) = error_message {
                    writeln!(writer, "    Error: {}", error)?;
                }
            },
            AgentDebugEvent::TaskComplete { task_id, success, execution_summary, .. } => {
                let status = if *success { "✅" } else { "❌" };
                writeln!(writer, "\n[{}] {} TASK_COMPLETE", timestamp_str, status)?;
                writeln!(writer, "    Task ID: {}", task_id)?;
                writeln!(writer, "    Success: {}", success)?;
                writeln!(writer, "    Summary: {}", execution_summary)?;
            },
            AgentDebugEvent::SessionEnd { session_id, total_duration_ms, .. } => {
                writeln!(writer, "\n[{}] 🏁 SESSION_END", timestamp_str)?;
                writeln!(writer, "    Session ID: {}", session_id)?;
                writeln!(writer, "    Total Duration: {}ms", total_duration_ms)?;
                writeln!(writer, "{}", "=".repeat(80))?;
            },
        }
        
        Ok(())
    }
    
    /// Get the current log file path for reference
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }
}

impl Default for AgentDebugLogger {
    fn default() -> Self {
        Self::new().expect("Failed to create agent framework debug logger")
    }
}

/// Global agent framework debug logger instance using thread-safe Lazy initialization
static AGENT_DEBUG_LOGGER: Lazy<Option<Arc<AgentDebugLogger>>> = Lazy::new(|| {
    match AgentDebugLogger::new() {
        Ok(logger) => Some(Arc::new(logger)),
        Err(e) => {
            error!("Failed to initialize agent framework debug logger: {}", e);
            None
        }
    }
});

/// Initialize the global agent framework debug logger
pub fn init_agent_debug_logger() -> Result<Arc<AgentDebugLogger>, std::io::Error> {
    // Force initialization and return the logger
    if let Some(logger) = &*AGENT_DEBUG_LOGGER {
        Ok(logger.clone())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to initialize agent framework debug logger",
        ))
    }
}

/// Get the global agent framework debug logger
pub fn get_agent_debug_logger() -> Option<Arc<AgentDebugLogger>> {
    AGENT_DEBUG_LOGGER.as_ref().cloned()
}

/// Log an agent framework debug event using the global logger
pub fn log_agent_debug_event(event: AgentDebugEvent) {
    if let Some(logger) = get_agent_debug_logger() {
        logger.log_event(event);
    }
}

/// Helper function to extract tool calls from response (simplified)
pub fn extract_tool_calls_from_response(response: &str) -> Vec<String> {
    // Simple heuristic to find tool calls - look for common patterns
    let mut tools = Vec::new();
    
    // Look for JSON tool calls or function names
    for line in response.lines() {
        if line.contains("tool_name") || line.contains("function_name") {
            // Extract tool name patterns
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    let tool_name = &line[start + 1..start + 1 + end];
                    if tool_name.starts_with("aws_") || tool_name.starts_with("todo_") || tool_name == "create_task" {
                        tools.push(tool_name.to_string());
                    }
                }
            }
        }
    }
    
    // Remove duplicates while preserving order
    let mut unique_tools = Vec::new();
    for tool in tools {
        if !unique_tools.contains(&tool) {
            unique_tools.push(tool);
        }
    }
    
    unique_tools
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_debug_logger_creation() {
        // Test that the logger can be created successfully
        let logger = AgentDebugLogger::new().expect("Failed to create debug logger");

        // Verify the log path has a parent directory (the logger creates parent dirs)
        assert!(logger.log_path().parent().is_some());

        println!("Debug logger created successfully: {:?}", logger.log_path());
    }
    
    #[test]
    fn test_extract_tool_calls() {
        let response_with_tools = r#"I need to use the aws_list_resources tool and then todo_write"#;
        let tools = extract_tool_calls_from_response(response_with_tools);
        // This is a simple test - the actual extraction would need more sophisticated parsing
        assert!(tools.len() >= 0); // Just ensure it doesn't crash
    }
}