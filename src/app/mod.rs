//! Core application modules for AWS Dash Architect.
//!
//! This module contains the business logic and data models that power the AWS CloudFormation
//! template designer and resource management system. Each submodule handles a specific aspect
//! of CloudFormation template processing, AWS integration, or user interface management.
//!
//! # Module Organization
//!
//! ## CloudFormation Processing
//! - [`cfn_template`] - Core CloudFormation template representation and manipulation
//! - [`cfn_resources`] - AWS resource specifications and schema management
//! - [`cfn_dag`] - Resource dependency graph analysis and validation
//! - [`cfn_intrinsic_functions`] - CloudFormation function detection and processing
//! - [`cfn_resource_icons`] - AWS resource type to icon mappings
//! - [`cfn_resource_policies`] - Resource policy management and validation
//!
//! ## AWS Integration
//! - [`aws_identity`] - AWS Identity Center authentication and credential management
//! - [`bedrock_client`] - AWS Bedrock integration for AI-powered features
//!
//! ## Project and Data Management
//! - [`projects`] - Multi-environment project organization and persistence
//! - [`dashui`] - Complete user interface implementation with window management
//!
//! # Integration Patterns
//!
//! The modules follow a layered architecture where:
//! - Data models ([`cfn_template`], [`projects`]) provide core representations
//! - Processing modules ([`cfn_dag`], [`cfn_resources`]) handle business logic
//! - Integration modules ([`aws_identity`], [`bedrock_client`]) connect to external services
//! - UI modules ([`dashui`]) coordinate user interactions and visual presentation
//!
//! See the [technical documentation](../../docs/technical/index.wiki) for detailed
//! implementation guides and architectural patterns for each subsystem.

pub mod aws_identity;
pub mod bedrock_client;
pub mod bridge;
pub mod cf_syntax;
pub mod cfn_dag;
pub mod cfn_intrinsic_functions;
pub mod cfn_resource_icons;
pub mod cfn_resource_policies;
pub mod cfn_resources;
pub mod cfn_template;
pub mod cloudformation_manager;
pub mod dashui;
pub mod notifications;
pub mod projects;
pub mod resource_explorer;

pub use dashui::app::DashApp;
