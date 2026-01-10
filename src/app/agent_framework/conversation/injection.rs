//! Message Injection Engine
//!
//! Provides programmatic message injection into agent conversations without
//! requiring user input. This enables automated follow-ups, context management,
//! and multi-agent coordination.
//!
//! ## Architecture
//!
//! Messages can be injected:
//! - Immediately (bypass user input)
//! - After specific tool completions
//! - After LLM responses
//! - When token thresholds are exceeded
//!
//! ## Usage
//!
//! ```rust,ignore
//! use agent_framework::message_injection::{MessageInjector, InjectionType, InjectionTrigger};
//!
//! let mut injector = MessageInjector::new();
//!
//! // Queue a follow-up after tool execution
//! injector.queue_injection(
//!     InjectionType::ToolFollowUp {
//!         tool_name: "execute_javascript".into(),
//!         context: "Analyze these results".into(),
//!     },
//!     InjectionTrigger::AfterToolComplete("execute_javascript".into()),
//! );
//!
//! // Check for ready injections
//! let context = InjectionContext::after_tool("execute_javascript", true);
//! if let Some(message) = injector.check_triggers(&context) {
//!     agent.inject_message(message);
//! }
//! ```

#![warn(clippy::all, rust_2018_idioms)]

use std::collections::VecDeque;

/// Type of message injection
///
/// Describes the semantic purpose of an injected message, which affects
/// how it's formatted and logged.
#[derive(Debug, Clone, PartialEq)]
pub enum InjectionType {
    /// System context to prepend to conversation
    ///
    /// Used for providing background information or constraints
    /// that should influence the agent's behavior.
    SystemContext(String),

    /// Follow-up after tool execution
    ///
    /// Automatically prompts the agent to analyze or act on
    /// the results of a tool call.
    ToolFollowUp {
        /// Name of the tool that was executed
        tool_name: String,
        /// Context or instructions for follow-up
        context: String,
    },

    /// Memory/summary injection for long conversations
    ///
    /// Used when conversation context exceeds token limits and
    /// older messages need to be summarized.
    MemorySummary(String),

    /// Redirect or correction
    ///
    /// Used to steer the agent in a different direction or
    /// correct a misunderstanding.
    Correction(String),

    /// Worker agent result
    ///
    /// Injects results from a completed worker agent back
    /// into the parent agent's conversation.
    WorkerResult {
        /// ID of the worker agent
        worker_id: String,
        /// Result from the worker
        result: String,
    },

    /// Error recovery prompt
    ///
    /// Prompts the agent to try an alternative approach after
    /// an error occurred.
    ErrorRecovery {
        /// Description of the error
        error: String,
        /// Suggested alternative approach
        suggestion: Option<String>,
    },
}

impl InjectionType {
    /// Get a label for logging purposes
    pub fn label(&self) -> &'static str {
        match self {
            InjectionType::SystemContext(_) => "SystemContext",
            InjectionType::ToolFollowUp { .. } => "ToolFollowUp",
            InjectionType::MemorySummary(_) => "MemorySummary",
            InjectionType::Correction(_) => "Correction",
            InjectionType::WorkerResult { .. } => "WorkerResult",
            InjectionType::ErrorRecovery { .. } => "ErrorRecovery",
        }
    }

    /// Format the injection as a message string
    pub fn format_message(&self) -> String {
        match self {
            InjectionType::SystemContext(context) => {
                format!("[System Context]\n{}", context)
            }
            InjectionType::ToolFollowUp { tool_name, context } => {
                format!("[Follow-up after {}]\n{}", tool_name, context)
            }
            InjectionType::MemorySummary(summary) => {
                format!("[Context Summary]\n{}", summary)
            }
            InjectionType::Correction(correction) => {
                format!("[Correction]\n{}", correction)
            }
            InjectionType::WorkerResult { worker_id, result } => {
                format!("[Worker {} Result]\n{}", worker_id, result)
            }
            InjectionType::ErrorRecovery { error, suggestion } => {
                let base = format!("[Error Recovery]\nThe previous operation failed: {}", error);
                match suggestion {
                    Some(s) => format!("{}\n\nSuggested approach: {}", base, s),
                    None => format!("{}\n\nPlease try an alternative approach.", base),
                }
            }
        }
    }
}

/// Trigger condition for when an injection should fire
#[derive(Debug, Clone, PartialEq)]
pub enum InjectionTrigger {
    /// Inject immediately on next check
    Immediate,

    /// Inject after a specific tool completes
    AfterToolComplete {
        /// Name of the tool to wait for
        tool_name: String,
        /// Only trigger on successful completion
        on_success_only: bool,
    },

    /// Inject after the next LLM response
    AfterResponse,

    /// Inject when token count exceeds threshold
    OnTokenThreshold(usize),

    /// Inject after a specific number of turns
    AfterTurns(usize),
}

impl InjectionTrigger {
    /// Create a trigger for after tool completion (success only)
    pub fn after_tool(tool_name: impl Into<String>) -> Self {
        InjectionTrigger::AfterToolComplete {
            tool_name: tool_name.into(),
            on_success_only: true,
        }
    }

    /// Create a trigger for after tool completion (any result)
    pub fn after_tool_any(tool_name: impl Into<String>) -> Self {
        InjectionTrigger::AfterToolComplete {
            tool_name: tool_name.into(),
            on_success_only: false,
        }
    }
}

/// A pending injection waiting for its trigger condition
#[derive(Debug, Clone)]
pub struct PendingInjection {
    /// Type of injection (determines message format)
    pub injection_type: InjectionType,
    /// When this injection should fire
    pub trigger: InjectionTrigger,
    /// Priority (higher = fires first when multiple ready)
    pub priority: u8,
}

impl PendingInjection {
    /// Create a new pending injection
    pub fn new(injection_type: InjectionType, trigger: InjectionTrigger) -> Self {
        Self {
            injection_type,
            trigger,
            priority: 0,
        }
    }

    /// Set priority (higher = fires first)
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Format as message string
    pub fn format_message(&self) -> String {
        self.injection_type.format_message()
    }
}

/// Context for checking injection triggers
///
/// Provides information about the current conversation state
/// so triggers can determine if they should fire.
#[derive(Debug, Clone, Default)]
pub struct InjectionContext {
    /// Last tool that completed (if any)
    pub last_tool_completed: Option<String>,
    /// Whether the last tool succeeded
    pub last_tool_success: bool,
    /// Whether an LLM response just completed
    pub response_completed: bool,
    /// Current estimated token count
    pub token_count: usize,
    /// Number of conversation turns
    pub turn_count: usize,
}

impl InjectionContext {
    /// Create context for after a tool completes
    pub fn after_tool(tool_name: impl Into<String>, success: bool) -> Self {
        Self {
            last_tool_completed: Some(tool_name.into()),
            last_tool_success: success,
            ..Default::default()
        }
    }

    /// Create context for after an LLM response
    pub fn after_response() -> Self {
        Self {
            response_completed: true,
            ..Default::default()
        }
    }

    /// Create context with token count
    pub fn with_tokens(token_count: usize) -> Self {
        Self {
            token_count,
            ..Default::default()
        }
    }

    /// Create context with turn count
    pub fn with_turns(turn_count: usize) -> Self {
        Self {
            turn_count,
            ..Default::default()
        }
    }
}

/// Message injection coordinator
///
/// Manages a queue of pending injections and checks trigger conditions
/// to determine when messages should be injected into the conversation.
#[derive(Debug, Default)]
pub struct MessageInjector {
    /// Queue of pending injections
    pending: VecDeque<PendingInjection>,
    /// Whether injections are enabled
    enabled: bool,
}

impl MessageInjector {
    /// Create a new message injector
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            enabled: true,
        }
    }

    /// Enable or disable injection processing
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if injections are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Queue a new injection
    pub fn queue(&mut self, injection: PendingInjection) {
        self.pending.push_back(injection);
    }

    /// Queue an injection with specific type and trigger
    pub fn queue_injection(&mut self, injection_type: InjectionType, trigger: InjectionTrigger) {
        self.queue(PendingInjection::new(injection_type, trigger));
    }

    /// Queue an immediate injection
    pub fn queue_immediate(&mut self, injection_type: InjectionType) {
        self.queue(PendingInjection::new(
            injection_type,
            InjectionTrigger::Immediate,
        ));
    }

    /// Get number of pending injections
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if there are pending injections
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Clear all pending injections
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Check if a trigger condition is met
    fn trigger_matches(trigger: &InjectionTrigger, context: &InjectionContext) -> bool {
        match trigger {
            InjectionTrigger::Immediate => true,

            InjectionTrigger::AfterToolComplete {
                tool_name,
                on_success_only,
            } => {
                if let Some(ref completed_tool) = context.last_tool_completed {
                    if completed_tool == tool_name {
                        return !*on_success_only || context.last_tool_success;
                    }
                }
                false
            }

            InjectionTrigger::AfterResponse => context.response_completed,

            InjectionTrigger::OnTokenThreshold(threshold) => context.token_count >= *threshold,

            InjectionTrigger::AfterTurns(turns) => context.turn_count >= *turns,
        }
    }

    /// Check triggers and return the next ready injection message
    ///
    /// Returns the formatted message string if an injection is ready,
    /// removing it from the queue.
    pub fn check_triggers(&mut self, context: &InjectionContext) -> Option<String> {
        if !self.enabled {
            return None;
        }

        // Find the first matching injection (considering priority)
        let mut best_index: Option<usize> = None;
        let mut best_priority: u8 = 0;

        for (i, injection) in self.pending.iter().enumerate() {
            if Self::trigger_matches(&injection.trigger, context)
                && (best_index.is_none() || injection.priority > best_priority)
            {
                best_index = Some(i);
                best_priority = injection.priority;
            }
        }

        // Remove and return the best matching injection
        if let Some(index) = best_index {
            let injection = self.pending.remove(index)?;
            Some(injection.format_message())
        } else {
            None
        }
    }

    /// Check triggers and return all ready injections
    ///
    /// Returns all injection messages that are ready, sorted by priority.
    pub fn check_all_triggers(&mut self, context: &InjectionContext) -> Vec<String> {
        if !self.enabled {
            return Vec::new();
        }

        // Collect all matching injections
        let mut ready: Vec<(usize, u8)> = self
            .pending
            .iter()
            .enumerate()
            .filter(|(_, inj)| Self::trigger_matches(&inj.trigger, context))
            .map(|(i, inj)| (i, inj.priority))
            .collect();

        // Sort by priority (descending) then by index (ascending for FIFO within priority)
        ready.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        // Remove in reverse order to maintain valid indices
        let mut messages = Vec::new();
        let indices: Vec<usize> = ready.iter().map(|(i, _)| *i).collect();

        for &index in indices.iter().rev() {
            if let Some(injection) = self.pending.remove(index) {
                messages.push(injection.format_message());
            }
        }

        // Reverse to get correct priority order
        messages.reverse();
        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injection_type_labels() {
        assert_eq!(
            InjectionType::SystemContext("test".into()).label(),
            "SystemContext"
        );
        assert_eq!(
            InjectionType::ToolFollowUp {
                tool_name: "test".into(),
                context: "ctx".into()
            }
            .label(),
            "ToolFollowUp"
        );
        assert_eq!(
            InjectionType::MemorySummary("test".into()).label(),
            "MemorySummary"
        );
        assert_eq!(
            InjectionType::Correction("test".into()).label(),
            "Correction"
        );
    }

    #[test]
    fn test_injection_type_format() {
        let sys = InjectionType::SystemContext("Be concise".into());
        assert!(sys.format_message().contains("System Context"));
        assert!(sys.format_message().contains("Be concise"));

        let follow = InjectionType::ToolFollowUp {
            tool_name: "execute_javascript".into(),
            context: "Analyze the results".into(),
        };
        assert!(follow.format_message().contains("execute_javascript"));
        assert!(follow.format_message().contains("Analyze the results"));
    }

    #[test]
    fn test_error_recovery_format() {
        let err = InjectionType::ErrorRecovery {
            error: "Connection timeout".into(),
            suggestion: Some("Try a different region".into()),
        };
        let msg = err.format_message();
        assert!(msg.contains("Connection timeout"));
        assert!(msg.contains("Try a different region"));

        let err_no_suggestion = InjectionType::ErrorRecovery {
            error: "Unknown error".into(),
            suggestion: None,
        };
        let msg2 = err_no_suggestion.format_message();
        assert!(msg2.contains("Unknown error"));
        assert!(msg2.contains("alternative approach"));
    }

    #[test]
    fn test_injection_trigger_after_tool() {
        let trigger = InjectionTrigger::after_tool("my_tool");
        assert!(matches!(
            trigger,
            InjectionTrigger::AfterToolComplete {
                tool_name: _,
                on_success_only: true
            }
        ));

        let trigger_any = InjectionTrigger::after_tool_any("my_tool");
        assert!(matches!(
            trigger_any,
            InjectionTrigger::AfterToolComplete {
                tool_name: _,
                on_success_only: false
            }
        ));
    }

    #[test]
    fn test_pending_injection_priority() {
        let inj = PendingInjection::new(
            InjectionType::SystemContext("test".into()),
            InjectionTrigger::Immediate,
        )
        .with_priority(5);

        assert_eq!(inj.priority, 5);
    }

    #[test]
    fn test_injection_context_builders() {
        let ctx = InjectionContext::after_tool("my_tool", true);
        assert_eq!(ctx.last_tool_completed, Some("my_tool".to_string()));
        assert!(ctx.last_tool_success);

        let ctx2 = InjectionContext::after_response();
        assert!(ctx2.response_completed);

        let ctx3 = InjectionContext::with_tokens(5000);
        assert_eq!(ctx3.token_count, 5000);
    }

    #[test]
    fn test_injector_queue() {
        let mut injector = MessageInjector::new();
        assert_eq!(injector.pending_count(), 0);
        assert!(!injector.has_pending());

        injector.queue_immediate(InjectionType::SystemContext("test".into()));
        assert_eq!(injector.pending_count(), 1);
        assert!(injector.has_pending());

        injector.clear();
        assert_eq!(injector.pending_count(), 0);
    }

    #[test]
    fn test_immediate_trigger() {
        let mut injector = MessageInjector::new();
        injector.queue_immediate(InjectionType::SystemContext("Hello".into()));

        let context = InjectionContext::default();
        let msg = injector.check_triggers(&context);

        assert!(msg.is_some());
        assert!(msg.unwrap().contains("Hello"));
        assert_eq!(injector.pending_count(), 0);
    }

    #[test]
    fn test_tool_trigger_success_only() {
        let mut injector = MessageInjector::new();
        injector.queue_injection(
            InjectionType::ToolFollowUp {
                tool_name: "execute_javascript".into(),
                context: "Analyze".into(),
            },
            InjectionTrigger::after_tool("execute_javascript"),
        );

        // Wrong tool - should not trigger
        let ctx1 = InjectionContext::after_tool("other_tool", true);
        assert!(injector.check_triggers(&ctx1).is_none());
        assert_eq!(injector.pending_count(), 1);

        // Right tool but failed - should not trigger (success_only)
        let ctx2 = InjectionContext::after_tool("execute_javascript", false);
        assert!(injector.check_triggers(&ctx2).is_none());
        assert_eq!(injector.pending_count(), 1);

        // Right tool and success - should trigger
        let ctx3 = InjectionContext::after_tool("execute_javascript", true);
        let msg = injector.check_triggers(&ctx3);
        assert!(msg.is_some());
        assert_eq!(injector.pending_count(), 0);
    }

    #[test]
    fn test_tool_trigger_any_result() {
        let mut injector = MessageInjector::new();
        injector.queue_injection(
            InjectionType::ErrorRecovery {
                error: "Failed".into(),
                suggestion: None,
            },
            InjectionTrigger::after_tool_any("execute_javascript"),
        );

        // Failed tool - should trigger with after_tool_any
        let ctx = InjectionContext::after_tool("execute_javascript", false);
        let msg = injector.check_triggers(&ctx);
        assert!(msg.is_some());
    }

    #[test]
    fn test_response_trigger() {
        let mut injector = MessageInjector::new();
        injector.queue_injection(
            InjectionType::Correction("Focus on security".into()),
            InjectionTrigger::AfterResponse,
        );

        // No response yet
        let ctx1 = InjectionContext::default();
        assert!(injector.check_triggers(&ctx1).is_none());

        // Response completed
        let ctx2 = InjectionContext::after_response();
        assert!(injector.check_triggers(&ctx2).is_some());
    }

    #[test]
    fn test_token_threshold_trigger() {
        let mut injector = MessageInjector::new();
        injector.queue_injection(
            InjectionType::MemorySummary("Previous context...".into()),
            InjectionTrigger::OnTokenThreshold(10000),
        );

        // Below threshold
        let ctx1 = InjectionContext::with_tokens(5000);
        assert!(injector.check_triggers(&ctx1).is_none());

        // At threshold
        let ctx2 = InjectionContext::with_tokens(10000);
        assert!(injector.check_triggers(&ctx2).is_some());
    }

    #[test]
    fn test_priority_ordering() {
        let mut injector = MessageInjector::new();

        // Add low priority first
        injector.queue(
            PendingInjection::new(
                InjectionType::SystemContext("Low".into()),
                InjectionTrigger::Immediate,
            )
            .with_priority(1),
        );

        // Add high priority second
        injector.queue(
            PendingInjection::new(
                InjectionType::SystemContext("High".into()),
                InjectionTrigger::Immediate,
            )
            .with_priority(10),
        );

        let ctx = InjectionContext::default();

        // High priority should fire first
        let msg1 = injector.check_triggers(&ctx);
        assert!(msg1.unwrap().contains("High"));

        // Low priority fires second
        let msg2 = injector.check_triggers(&ctx);
        assert!(msg2.unwrap().contains("Low"));
    }

    #[test]
    fn test_disabled_injector() {
        let mut injector = MessageInjector::new();
        injector.queue_immediate(InjectionType::SystemContext("test".into()));

        injector.set_enabled(false);
        assert!(!injector.is_enabled());

        let ctx = InjectionContext::default();
        assert!(injector.check_triggers(&ctx).is_none());

        // Re-enable
        injector.set_enabled(true);
        assert!(injector.check_triggers(&ctx).is_some());
    }

    #[test]
    fn test_check_all_triggers() {
        let mut injector = MessageInjector::new();

        injector.queue_immediate(InjectionType::SystemContext("First".into()));
        injector.queue_immediate(InjectionType::SystemContext("Second".into()));
        injector.queue_injection(
            InjectionType::Correction("Not ready".into()),
            InjectionTrigger::AfterResponse,
        );

        let ctx = InjectionContext::default();
        let messages = injector.check_all_triggers(&ctx);

        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("First"));
        assert!(messages[1].contains("Second"));

        // Third injection should still be pending
        assert_eq!(injector.pending_count(), 1);
    }
}
