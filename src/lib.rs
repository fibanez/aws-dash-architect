//! AWS Dash Architect - CloudFormation Template Designer and AWS Resource Manager
//!
//! AWS Dash Architect is a desktop application for creating, editing, and managing AWS CloudFormation
//! templates with an intuitive visual interface. The application provides comprehensive support for
//! CloudFormation resource management, AWS Identity Center authentication, and project organization.
//!
//! # Core Features
//!
//! - **Visual CloudFormation Editor**: Create and edit CloudFormation templates with guided forms
//! - **Resource Dependency Visualization**: Interactive graph showing resource relationships
//! - **AWS Identity Center Integration**: Seamless authentication and multi-account support
//! - **Project Management**: Organize templates and resources across environments
//! - **Schema Validation**: Real-time validation against AWS CloudFormation specifications
//! - **Multi-format Support**: Import/export JSON and YAML CloudFormation templates
//!
//! # Architecture Overview
//!
//! The application follows a layered architecture with clear separation of concerns:
//!
//! - **UI Layer** ([`app::dashui`]): egui-based desktop interface with window management
//! - **Business Logic** ([`app`]): Core CloudFormation processing and AWS integration
//! - **Data Models**: Type-safe representations of CloudFormation templates and AWS resources
//! - **Integration Layer**: AWS SDK integration and external service communication
//!
//! ## Key Architectural Patterns
//!
//! - **Trait-based Window System**: Polymorphic window management with [`app::dashui::window_focus::FocusableWindow`]
//! - **Event-driven Processing**: Async operations with channels and state machines
//! - **Cache-first Architecture**: Aggressive caching for AWS resource specifications
//! - **Recovery-oriented Design**: Multiple fallback mechanisms for data integrity
//!
//! # Major Subsystems
//!
//! ## CloudFormation Template System
//!
//! Core template representation and manipulation via [`app::cfn_template::CloudFormationTemplate`].
//! Provides comprehensive support for all CloudFormation features including intrinsic functions,
//! dependencies, and cross-references.
//!
//! ## Resource Management
//!
//! AWS CloudFormation resource specifications managed by [`app::cfn_resources`] with automatic
//! downloading and caching of the latest AWS resource schemas for validation and form generation.
//!
//! ## Dependency Graph
//!
//! Resource relationships visualized and validated through [`app::cfn_dag::ResourceDag`] using
//! directed acyclic graph algorithms for deployment order optimization.
//!
//! ## Project Organization
//!
//! Multi-environment project management via [`app::projects`] enabling resource organization
//! across development, staging, and production environments.
//!
//! ## AWS Integration
//!
//! Identity Center authentication and credential management through [`app::aws_identity`]
//! supporting device authorization flow and multi-account access.
//!
//! # Getting Started
//!
//! The main application entry point is [`DashApp`] which coordinates all subsystems and provides
//! the primary user interface. See the [technical documentation](../docs/technical/index.wiki)
//! for detailed implementation guides and architectural patterns.
//!
//! # Development
//!
//! See [`CLAUDE.md`](../CLAUDE.md) for build commands, testing strategies, and development workflow.
//! The application uses chunked testing for context window management and smart verbosity controls
//! for efficient debugging.

#![warn(clippy::all, rust_2018_idioms)]

use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

/// Type alias for the tracing reload handle to reduce complexity
type TracingReloadHandle =
    tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, tracing_subscriber::Registry>;

/// Global handle for reloading tracing filter (enables/disables stood traces dynamically)
static TRACING_RELOAD_HANDLE: Lazy<Arc<Mutex<Option<TracingReloadHandle>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Set the tracing reload handle after initialization
/// Called from main.rs after setting up the tracing subscriber
pub fn set_tracing_reload_handle(handle: TracingReloadHandle) {
    *TRACING_RELOAD_HANDLE.lock().unwrap() = Some(handle);
}

/// Set stood tracing level dynamically
///
/// This updates the global tracing filter to capture stood library events
/// at the specified level. Note that per-agent log capture is handled by
/// the AgentTracingLayer which respects thread-local log level settings.
///
/// # Arguments
/// * `level` - The StoodLogLevel to set (Off, Info, Debug, Trace)
pub fn set_stood_log_level(level: app::agent_framework::StoodLogLevel) {
    if let Some(handle) = TRACING_RELOAD_HANDLE.lock().unwrap().as_ref() {
        let stood_level = level.to_filter_str();

        let new_filter = format!(
            "awsdash=trace,stood={},aws_sdk_cloudformation=trace,aws_sdk_bedrockruntime=trace,aws_config=trace,aws_sigv4=trace,aws_smithy_runtime=trace,aws_smithy_runtime_api=trace,hyper=trace,aws_smithy_http=trace,aws_endpoint=trace",
            stood_level
        );

        if let Ok(filter) = tracing_subscriber::EnvFilter::builder().parse(&new_filter) {
            if let Err(e) = handle.reload(filter) {
                eprintln!("Failed to reload tracing filter: {}", e);
            } else {
                tracing::info!("Stood log level set to: {}", level.display_name());
            }
        }
    }
}

/// Legacy function for backward compatibility
/// Toggles stood traces on (Debug level) or off
#[deprecated(since = "0.1.0", note = "Use set_stood_log_level instead")]
pub fn toggle_stood_traces(enable: bool) {
    use app::agent_framework::StoodLogLevel;
    let level = if enable {
        StoodLogLevel::Debug
    } else {
        StoodLogLevel::Off
    };
    set_stood_log_level(level);
}

// Include logging macros first
#[macro_use]
pub mod logging_macros;

pub mod app;
pub use app::DashApp;

// Temporarily disabled due to missing cfn_template module
// #[cfg(test)]
// mod test_cloudformation_import;
