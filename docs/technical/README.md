# AWS Dash Technical Documentation

This directory contains modular technical documentation for the AWS Dash project.

## Core Systems

* [Resource Explorer System](resource-explorer-system.md) - Multi-account AWS resource discovery with 80+ resource types
* [Keyboard Navigation System](keyboard-navigation-system.md) - Vimium-style navigation with hint mode and multi-modal interaction
* [Notifications System](notifications-system.md) - Comprehensive notification management
* [UI Testing Framework](ui-testing-framework.md) - Automated testing with egui_kittest
* [Agent Framework](agent-framework-v2.md) - AI agent system using stood library for AWS operations
* [Agent Feedback Systems](agent-feedback-systems.md) - Status display, message injection, and conversation middleware for agents
* [Multi-Agent System](multi-agent-system.md) - Task manager and worker agent orchestration for parallel AWS operations
* [Code Execution Tool](code-execution-tool.md) - V8-based JavaScript execution for AI agents

## UI Features

* [AWS Login Window](aws-login-window.md) - Identity Center authentication with simplified URL input and region selection
* [Command Palette System](command-palette-system.md) - Command palette with keyboard shortcuts

## Architecture Patterns

* [Testing Patterns](testing-patterns.md) - Testing strategies and frameworks

## Implementation Guides

* [Adding New Windows](adding-new-windows.md) - How to add focusable windows
* [UI Component Testing](ui-component-testing.md) - Writing UI tests
* [AWS Data Plane Integration](aws-data-plane-integration-guide.md) - Integrating AWS data plane services (CloudWatch Logs, Metrics, etc.)
* [Agent Middleware Guide](agent-middleware-guide.md) - Creating custom middleware layers for agent message processing

## Extension Patterns

* [Resource Normalizers](resource-normalizers.md) - Adding support for new AWS resource types
* [Credential Management](credential-management.md) - Multi-account security and authentication patterns
* [AWS Service Integration Patterns](aws-service-integration-patterns.md) - Templates for integrating new AWS services

## Reference

* [AWS API Calls Inventory](aws-api-calls-inventory.md) - Complete inventory of AWS SDK calls per resource type (for security/compliance gap analysis)
* [Agent Injection Patterns Research](agent-injection-patterns-research.md) - Industry research on message injection, context management, and memory patterns for AI agents

For user-facing documentation including troubleshooting guides, see the [User Guide](../userguide/) section.
