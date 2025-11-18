#![warn(clippy::all, rust_2018_idioms)]

//! Shared message types for agent communication
//!
//! This module contains the message data structures used for agent-to-UI communication.
//! Extracted from AgentControlWindow to enable reuse across multiple agent instances.

use chrono::{DateTime, Utc};
use egui::Color32;
use stood::agent::result::AgentResult;
use uuid::Uuid;

// ============================================================================
// MESSAGE SYSTEM
// ============================================================================

/// Message types for the GUI
#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Debug,
    JsonRequest,  // Model request JSON data
    JsonResponse, // Model response JSON data
}

impl MessageRole {
    pub fn icon(&self) -> &'static str {
        match self {
            MessageRole::User => "👤",
            MessageRole::Assistant => "⚡", // Lightning bolt - supported by egui
            MessageRole::System => "ℹ",
            MessageRole::Debug => "🔧",
            MessageRole::JsonRequest => "📤",  // Outgoing request
            MessageRole::JsonResponse => "📥", // Incoming response
        }
    }

    pub fn color(&self, dark_mode: bool) -> Color32 {
        match (self, dark_mode) {
            // Dark mode: bright colors for visibility on dark background
            (MessageRole::User, true) => Color32::from_rgb(100, 150, 255), // Bright blue
            (MessageRole::Assistant, true) => Color32::from_rgb(100, 255, 150), // Bright green
            (MessageRole::System, true) => Color32::from_rgb(255, 200, 100), // Bright orange
            (MessageRole::Debug, true) => Color32::from_rgb(180, 180, 180), // Light gray
            (MessageRole::JsonRequest, true) => Color32::from_rgb(255, 140, 0), // Bright orange for JSON
            (MessageRole::JsonResponse, true) => Color32::from_rgb(255, 140, 0), // Bright orange for JSON

            // Light mode: darker colors for visibility on light background
            (MessageRole::User, false) => Color32::from_rgb(50, 75, 150), // Dark blue
            (MessageRole::Assistant, false) => Color32::from_rgb(50, 150, 75), // Dark green
            (MessageRole::System, false) => Color32::from_rgb(180, 120, 50), // Dark orange
            (MessageRole::Debug, false) => Color32::from_rgb(100, 100, 100), // Dark gray
            (MessageRole::JsonRequest, false) => Color32::from_rgb(200, 100, 0), // Dark orange for JSON
            (MessageRole::JsonResponse, false) => Color32::from_rgb(200, 100, 0), // Dark orange for JSON
        }
    }
}

/// JSON debug data captured from model interactions
#[derive(Debug, Clone)]
pub struct JsonDebugData {
    pub json_type: JsonDebugType,
    pub json_content: String, // Composed JSON (our reconstruction)
    pub raw_json_content: Option<String>, // Raw JSON from provider API (if available)
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum JsonDebugType {
    Request,
    Response,
}

/// A single message in the conversation
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String, // Unique identifier for this message
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub summary: Option<String>,
    pub debug_info: Option<String>,
    pub nested_messages: Vec<Message>,
    pub agent_source: Option<String>, // Track which agent/tool generated this message
    pub json_debug_data: Vec<JsonDebugData>, // JSON capture data for this message/agent
}

impl Message {
    pub fn new(role: MessageRole, content: String) -> Self {
        let summary = Self::generate_summary(&content);
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: Utc::now(),
            summary: Some(summary),
            debug_info: None,
            nested_messages: Vec::new(),
            agent_source: None,
            json_debug_data: Vec::new(),
        }
    }

    pub fn new_with_agent(role: MessageRole, content: String, agent_source: String) -> Self {
        let summary = Self::generate_summary(&content);
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: Utc::now(),
            summary: Some(summary),
            debug_info: None,
            nested_messages: Vec::new(),
            agent_source: Some(agent_source),
            json_debug_data: Vec::new(),
        }
    }

    pub fn with_debug(mut self, debug_info: String) -> Self {
        self.debug_info = Some(debug_info);
        self
    }

    pub fn add_nested_message(&mut self, message: Message) {
        self.nested_messages.push(message);
    }

    fn generate_summary(content: &str) -> String {
        let words: Vec<&str> = content.split_whitespace().take(5).collect();
        if words.len() < 5 && content.len() < 30 {
            content.to_string()
        } else {
            format!("{}...", words.join(" "))
        }
    }
}

// ============================================================================
// AGENT RESPONSE SYSTEM
// ============================================================================

/// Response from agent execution in separate thread
#[derive(Debug)]
pub enum AgentResponse {
    Success(AgentResult),
    Error(String),
    JsonDebug(JsonDebugData),
    ModelChanged {
        model_id: String,
    },
    // Tool callback responses for creating tree structure
    ToolCallStart {
        parent_message: Message,
    },
    ToolCallComplete {
        parent_message_id: String,
        child_message: Message,
    },
}
