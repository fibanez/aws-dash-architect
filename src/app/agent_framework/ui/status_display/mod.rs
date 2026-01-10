//! Processing Status Display Engine
//!
//! Provides animated UI status display for agent processing with whimsical messages
//! and phase-based animations.
//!
//! ## Components
//!
//! - **ProcessingPhase**: Current phase of agent processing
//! - **StatusMessageGenerator**: Generates whimsical status messages
//! - **ProcessingAnimation**: Animated visual feedback (OrbitalDots, WaveBars)
//! - **ProcessingStatusWidget**: Complete widget combining message and animation
//!
//! ## Usage
//!
//! ```rust,ignore
//! use agent_framework::status_display::{ProcessingStatusWidget, ProcessingPhase};
//!
//! let mut widget = ProcessingStatusWidget::new();
//! widget.set_phase(ProcessingPhase::ExecutingTool("execute_javascript".into()));
//!
//! // In UI rendering:
//! widget.show(ui);
//! ```

#![warn(clippy::all, rust_2018_idioms)]

mod animation;
mod messages;
mod widget;

pub use animation::{AnimationStyle, ProcessingAnimation};
pub use messages::{ProcessingPhase, StatusMessageGenerator};
pub use widget::ProcessingStatusWidget;
