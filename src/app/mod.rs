//! Core application modules for AWS Dash.
//!
//! This module contains the business logic and data models for AWS resource
//! exploration and AI-powered operations through the Bridge Agent system.
//!
//! # Module Organization
//!
//! ## AWS Integration
//! - [`aws_identity`] - AWS Identity Center authentication and credential management
//! - [`resource_explorer`] - Multi-account AWS resource discovery and visualization
//!
//! ## AI Agent System
//! - [`bridge`] - AI agent tools for AWS resource operations and analysis
//!
//! ## UI and Infrastructure
//! - [`dashui`] - Complete user interface implementation with window management
//! - [`fonts`] - Font loading and management
//! - [`notifications`] - Notification system for user feedback
//!
//! # Architecture
//!
//! The application follows a simple layered architecture:
//! - [`aws_identity`] provides authentication and credential management
//! - [`resource_explorer`] handles AWS resource discovery across accounts and regions
//! - [`bridge`] provides AI agent capabilities for resource analysis and operations
//! - [`dashui`] coordinates the user interface and window management

pub mod aws_identity;
pub mod aws_regions;
pub mod bridge;
pub mod dashui;
pub mod fonts;
pub mod notifications;
pub mod resource_explorer;

pub use dashui::app::DashApp;
