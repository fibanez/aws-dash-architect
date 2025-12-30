//! Built-in Middleware Layers
//!
//! This module provides pre-built middleware layers for common use cases.
//!
//! ## Available Layers
//!
//! - [`TokenTrackingLayer`] - Tracks token usage and can inject summaries
//! - [`AutoAnalysisLayer`] - Automatically triggers follow-up analysis
//! - [`LoggingLayer`] - Logs all message flow for debugging

#![warn(clippy::all, rust_2018_idioms)]

mod auto_analysis;
mod logging;
mod token_tracking;

pub use auto_analysis::AutoAnalysisLayer;
pub use logging::LoggingLayer;
pub use token_tracking::TokenTrackingLayer;
