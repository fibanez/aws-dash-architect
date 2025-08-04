//! AWS Bridge Specialized Agents
//!
//! This module contains specialized agent implementations for the agent-on-demand
//! architecture. Each agent is designed for specific AWS tasks with focused toolsets.

pub mod aws_log_analyzer;
pub mod aws_resource_auditor;
pub mod aws_security_scanner;

// Re-export agents for easy access
pub use aws_log_analyzer::AwsLogAnalyzerAgent;
pub use aws_resource_auditor::AwsResourceAuditorAgent;
pub use aws_security_scanner::AwsSecurityScannerAgent;