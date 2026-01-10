//! Processing Status Widget
//!
//! Complete widget combining animated visual feedback with whimsical status messages.
//!
//! ## Layout
//!
//! ```text
//! [ Animation ] Message text... (detail)
//! ```
//!
//! The widget displays horizontally with the animation on the left and the
//! status message on the right, styled with italic weak text.

#![warn(clippy::all, rust_2018_idioms)]

use egui::{RichText, Ui};

use super::animation::ProcessingAnimation;
use super::messages::{ProcessingPhase, StatusMessageGenerator};

/// Complete processing status widget
///
/// Combines animation and message display for agent processing status.
pub struct ProcessingStatusWidget {
    /// Current processing phase
    phase: ProcessingPhase,
    /// Animated visual indicator
    animation: ProcessingAnimation,
    /// Whimsical message generator
    message_generator: StatusMessageGenerator,
    /// Optional detail to display (e.g., token count)
    detail: Option<String>,
    /// Previous phase for detecting changes
    previous_phase: ProcessingPhase,
}

impl Default for ProcessingStatusWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessingStatusWidget {
    /// Create a new processing status widget
    pub fn new() -> Self {
        Self {
            phase: ProcessingPhase::Idle,
            animation: ProcessingAnimation::new(),
            message_generator: StatusMessageGenerator::new(),
            detail: None,
            previous_phase: ProcessingPhase::Idle,
        }
    }

    /// Set the current processing phase
    ///
    /// This updates both the animation style and resets the message rotation
    /// if the phase changes.
    pub fn set_phase(&mut self, phase: ProcessingPhase) {
        // Detect phase change for reset
        let phase_changed = match (&self.previous_phase, &phase) {
            (ProcessingPhase::Thinking, ProcessingPhase::Thinking) => false,
            (ProcessingPhase::AnalyzingResults, ProcessingPhase::AnalyzingResults) => false,
            (ProcessingPhase::Idle, ProcessingPhase::Idle) => false,
            (ProcessingPhase::ExecutingTool(a), ProcessingPhase::ExecutingTool(b)) => a != b,
            _ => true,
        };

        if phase_changed {
            self.message_generator.reset();
        }

        self.animation.set_phase(&phase);
        self.previous_phase = self.phase.clone();
        self.phase = phase;
    }

    /// Set optional detail text (e.g., "2,500 tokens", "15 resources")
    pub fn set_detail(&mut self, detail: Option<String>) {
        self.detail = detail;
    }

    /// Set processing state from agent status
    ///
    /// Convenience method that sets phase to Thinking if processing,
    /// Idle if not.
    pub fn set_processing(&mut self, is_processing: bool) {
        if is_processing {
            if matches!(self.phase, ProcessingPhase::Idle) {
                self.set_phase(ProcessingPhase::Thinking);
            }
        } else {
            self.set_phase(ProcessingPhase::Idle);
        }
    }

    /// Update phase for tool execution
    pub fn set_executing_tool(&mut self, tool_name: &str) {
        self.set_phase(ProcessingPhase::ExecutingTool(tool_name.to_string()));
    }

    /// Update phase for result analysis
    pub fn set_analyzing(&mut self) {
        self.set_phase(ProcessingPhase::AnalyzingResults);
    }

    /// Check if currently showing an active state
    pub fn is_active(&self) -> bool {
        self.phase.is_active()
    }

    /// Get the current phase
    pub fn phase(&self) -> &ProcessingPhase {
        &self.phase
    }

    /// Render the widget
    ///
    /// Displays animation + message horizontally. If idle, displays nothing
    /// to maintain consistent layout.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    ///
    /// # Returns
    /// The height used by the widget (for layout consistency)
    pub fn show(&mut self, ui: &mut Ui) -> f32 {
        let is_active = self.phase.is_active();

        if !is_active {
            // Reserve space even when idle to prevent layout shift
            let text_height = ui.text_style_height(&egui::TextStyle::Body);
            ui.add_space(text_height);
            return text_height;
        }

        // Horizontal layout: animation + message
        ui.horizontal(|ui| {
            // Animation
            self.animation.show(ui);

            ui.add_space(8.0);

            // Message
            let message = self
                .message_generator
                .generate(&self.phase, self.detail.as_deref());

            ui.label(RichText::new(message).italics().weak());
        });

        // Return approximate height
        ui.text_style_height(&egui::TextStyle::Body).max(24.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_creation() {
        let widget = ProcessingStatusWidget::new();
        assert!(!widget.is_active());
        assert!(matches!(widget.phase(), ProcessingPhase::Idle));
    }

    #[test]
    fn test_set_phase() {
        let mut widget = ProcessingStatusWidget::new();

        widget.set_phase(ProcessingPhase::Thinking);
        assert!(widget.is_active());
        assert!(matches!(widget.phase(), ProcessingPhase::Thinking));

        widget.set_phase(ProcessingPhase::ExecutingTool("test_tool".into()));
        assert!(matches!(widget.phase(), ProcessingPhase::ExecutingTool(_)));
    }

    #[test]
    fn test_set_processing() {
        let mut widget = ProcessingStatusWidget::new();

        widget.set_processing(true);
        assert!(widget.is_active());

        widget.set_processing(false);
        assert!(!widget.is_active());
    }

    #[test]
    fn test_set_detail() {
        let mut widget = ProcessingStatusWidget::new();

        widget.set_detail(Some("2,500 tokens".into()));
        assert_eq!(widget.detail, Some("2,500 tokens".to_string()));

        widget.set_detail(None);
        assert!(widget.detail.is_none());
    }

    #[test]
    fn test_set_executing_tool() {
        let mut widget = ProcessingStatusWidget::new();
        widget.set_executing_tool("execute_javascript");

        if let ProcessingPhase::ExecutingTool(name) = widget.phase() {
            assert_eq!(name, "execute_javascript");
        } else {
            panic!("Expected ExecutingTool phase");
        }
    }

    #[test]
    fn test_set_analyzing() {
        let mut widget = ProcessingStatusWidget::new();
        widget.set_analyzing();
        assert!(matches!(widget.phase(), ProcessingPhase::AnalyzingResults));
    }

    #[test]
    fn test_phase_change_behavior() {
        let mut widget = ProcessingStatusWidget::new();

        // Set to thinking
        widget.set_phase(ProcessingPhase::Thinking);
        assert!(widget.is_active());

        // Change to tool execution
        widget.set_phase(ProcessingPhase::ExecutingTool("test".into()));
        assert!(widget.is_active());

        // Verify phase updated correctly
        if let ProcessingPhase::ExecutingTool(name) = widget.phase() {
            assert_eq!(name, "test");
        } else {
            panic!("Expected ExecutingTool phase");
        }

        // Change to analyzing
        widget.set_phase(ProcessingPhase::AnalyzingResults);
        assert!(matches!(widget.phase(), ProcessingPhase::AnalyzingResults));

        // Change to idle
        widget.set_phase(ProcessingPhase::Idle);
        assert!(!widget.is_active());
    }
}
