//! Per-Agent Tracing Layer for Stood Library Events
//!
//! This module provides a custom tracing subscriber layer that captures
//! `stood::*` events and routes them to the current agent's log file
//! based on thread-local context.
//!
//! ## Architecture
//!
//! When an agent executes in a background thread:
//! 1. Thread-local context is set with agent's logger and log level
//! 2. Agent calls stood library functions which emit tracing events
//! 3. This layer intercepts `stood::*` events
//! 4. Events are routed to the agent's log file via AgentLogger::log_stood_trace()
//!
//! This ensures stood library debug output appears in per-agent logs
//! instead of the global awsdash.log file.

#![warn(clippy::all, rust_2018_idioms)]

use std::cell::Cell;
use std::fmt;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

use crate::app::agent_framework::agent_logger::get_current_agent_logger;
use crate::app::agent_framework::StoodLogLevel;

// Thread-local storage for current agent's log level
thread_local! {
    static CURRENT_LOG_LEVEL: Cell<StoodLogLevel> = const { Cell::new(StoodLogLevel::Debug) };
}

/// Set the stood log level for the current thread
pub fn set_current_log_level(level: StoodLogLevel) {
    CURRENT_LOG_LEVEL.with(|cell| {
        cell.set(level);
    });
}

/// Get the stood log level for the current thread
pub fn get_current_log_level() -> StoodLogLevel {
    CURRENT_LOG_LEVEL.with(|cell| cell.get())
}

/// Custom tracing layer that routes stood::* events to per-agent logs
///
/// This layer:
/// - Only captures events from targets starting with "stood::"
/// - Checks the current thread's log level setting
/// - Routes matching events to the current agent's log file
pub struct AgentTracingLayer;

impl AgentTracingLayer {
    /// Create a new agent tracing layer
    pub fn new() -> Self {
        Self
    }
}

impl Default for AgentTracingLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Subscriber> Layer<S> for AgentTracingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Only capture stood::* events
        let target = event.metadata().target();
        if !target.starts_with("stood::") {
            return;
        }

        // Get current log level for this thread
        let log_level = get_current_log_level();

        // Check if event level meets threshold
        let event_level = *event.metadata().level();
        if !log_level.should_log(event_level) {
            return;
        }

        // Get current agent's logger
        let logger = match get_current_agent_logger() {
            Some(l) => l,
            None => return, // No agent context, skip
        };

        // Format the event
        let level_str = level_to_str(event_level);
        let message = format_event(event);

        // Write to agent log
        logger.log_stood_trace(level_str, target, &message);
    }
}

/// Convert tracing Level to string for logging
fn level_to_str(level: Level) -> &'static str {
    match level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARN",
        Level::INFO => "INFO",
        Level::DEBUG => "DEBUG",
        Level::TRACE => "TRACE",
    }
}

/// Format a tracing event into a message string
fn format_event(event: &Event<'_>) -> String {
    let mut visitor = MessageVisitor::default();
    event.record(&mut visitor);
    visitor.message
}

/// Visitor for extracting message from tracing events
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
            // Remove surrounding quotes if present
            if self.message.starts_with('"') && self.message.ends_with('"') {
                self.message = self.message[1..self.message.len() - 1].to_string();
            }
        } else if self.message.is_empty() {
            // Capture first field as message if no explicit message field
            self.message = format!("{}={:?}", field.name(), value);
        } else {
            // Append additional fields
            self.message
                .push_str(&format!(" {}={:?}", field.name(), value));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else if self.message.is_empty() {
            self.message = format!("{}={}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(" {}={}", field.name(), value));
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if self.message.is_empty() {
            self.message = format!("{}={}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(" {}={}", field.name(), value));
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if self.message.is_empty() {
            self.message = format!("{}={}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(" {}={}", field.name(), value));
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        if self.message.is_empty() {
            self.message = format!("{}={}", field.name(), value);
        } else {
            self.message
                .push_str(&format!(" {}={}", field.name(), value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_thread_local() {
        // Test default is Debug
        assert_eq!(get_current_log_level(), StoodLogLevel::Debug);

        // Test setting level
        set_current_log_level(StoodLogLevel::Trace);
        assert_eq!(get_current_log_level(), StoodLogLevel::Trace);

        // Test changing level
        set_current_log_level(StoodLogLevel::Info);
        assert_eq!(get_current_log_level(), StoodLogLevel::Info);

        // Reset to default
        set_current_log_level(StoodLogLevel::Debug);
    }

    #[test]
    fn test_level_to_str() {
        assert_eq!(level_to_str(Level::ERROR), "ERROR");
        assert_eq!(level_to_str(Level::WARN), "WARN");
        assert_eq!(level_to_str(Level::INFO), "INFO");
        assert_eq!(level_to_str(Level::DEBUG), "DEBUG");
        assert_eq!(level_to_str(Level::TRACE), "TRACE");
    }

    #[test]
    fn test_agent_tracing_layer_creation() {
        let layer = AgentTracingLayer::new();
        let layer_default = AgentTracingLayer::default();
        // Just verify they can be created
        drop(layer);
        drop(layer_default);
    }
}
