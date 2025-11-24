# Reference Folder - Pseudocode Implementation Guide

## Purpose

This folder contains pseudocode reference documentation that mirrors the source code
structure. Each file provides implementation guidance, architectural patterns, and
algorithmic approaches for corresponding source files.

---

## Methodology

### File Organization

- **Mirror Source Structure**: Directory structure matches `src/` exactly
- **File Naming**: `source_file.rs` → `source_file.md`
- **Extension**: All reference files use `.md` (Markdown format)

### Content Structure

Each reference file should contain:

1. **Component Overview**: Brief description of the module's purpose
2. **Major Methods/Functions**: List of key functions with single-line descriptions
3. **Implementation Patterns**: Rust idioms, algorithms, design patterns used
4. **External Systems**: Dependencies, AWS services, third-party crates involved
5. **Pseudocode**: Algorithm outlines in 80-column format

### Pseudocode Guidelines

- **Line Length**: Maximum 80 characters per line
- **Format**: Single-line descriptions, concise and clear
- **Style**: Outline-based, hierarchical structure
- **Focus**: How things work, not what they do (implementation vs specification)
- **Rust Patterns**: Reference Rust-specific approaches (traits, lifetimes, async, etc.)
- **Algorithms**: Name specific algorithms (BFS, topological sort, caching, etc.)

### Example Format

```
## Function: parse_template()

Pattern: Builder pattern with error propagation
Algorithm: Recursive descent parser
External: serde_json, serde_yaml

Pseudocode:
  1. Detect format (JSON/YAML) by file extension or content sniffing
  2. Parse raw string into intermediate representation using serde
  3. Validate schema against CloudFormation specification
  4. Build Template struct using builder pattern
  5. Return Result<Template, anyhow::Error> with context
```

---

## Major Codebase Components

### AWS_IDENTITY
**AWS authentication and credential management**: Identity Center integration,
device authorization flow, multi-account credential handling, and SSO.

**Key Files**:
- `src/app/aws_identity.rs` - OAuth flow, credential caching, account enumeration

### EXPLORER
**AWS resource discovery system**: Multi-account resource querying, cross-region
discovery, service-specific clients, and data normalization.

**Key Files**:
- `src/app/resource_explorer/aws_client.rs` - Multi-account resource discovery
- `src/app/resource_explorer/services/*.rs` - AWS service clients
- `src/app/resource_explorer/normalizers/*.rs` - Service-specific data formatting
- `src/app/resource_explorer/window.rs` - Explorer UI
- `src/app/resource_explorer/tree.rs` - Hierarchical resource organization

### DASHUI
**User interface system**: Window management, egui-based UI, keyboard navigation,
command palettes, and focusable window trait system.

**Key Files**:
- `src/app/dashui/app.rs` - Main application coordinator
- `src/app/dashui/window_focus.rs` - Window focus management trait
- `src/app/dashui/keyboard_navigation.rs` - Vimium-like navigation
- `src/app/dashui/hint_mode.rs` - Visual hint system
- `src/app/dashui/command_palette.rs` - Global command search
- `src/app/dashui/*_window.rs` - Specialized windows

### AGENT FRAMEWORK
**AI agent system**: Task agents for AWS operations, tool registry, natural
language to AWS operation translation, and agent-tool communication.

**Key Files**:
- `src/app/agent_framework/agents/orchestration_agent.rs` - Main orchestration agent
- `src/app/agent_framework/agents/task_agent.rs` - Specialized AWS task agent
- `src/app/agent_framework/tools_registry.rs` - Tool registration and discovery
- `src/app/agent_framework/tools/*.rs` - AWS operation tools
- `src/app/agent_framework/model_config.rs` - Model configuration
- `src/app/agent_framework/agent_logger.rs` - Agent operation logging

### DATA_PLANE
**AWS data plane integration**: Direct AWS service API calls, resource
management, and service-specific operations.

**Key Files**:
- `src/app/data_plane/mod.rs` - Data plane module coordination
- `src/app/data_plane/*/` - Service-specific implementations

### NOTIFICATIONS
**Notification system**: User notifications, toast messages, and alert management.

**Key Files**:
- `src/app/notifications/mod.rs` - Notification system

---

## Usage Workflow

### For New Feature Development

1. **Review Spec**: Check `/docs/technical/` or TODOS for feature specifications
2. **Create Reference File**: Add `reference/src/app/feature.md` mirroring source
3. **Document Approach**: Write pseudocode outlining implementation strategy
4. **Identify Patterns**: Note Rust patterns, algorithms, external dependencies
5. **Implement**: Write actual code in `src/` referencing the pseudocode guide
6. **Update Reference**: Keep reference file in sync with implementation changes

### For Understanding Existing Code

1. **Locate Source File**: Find the module in `src/` you want to understand
2. **Check Reference**: Look for corresponding `.md` file in `reference/src/`
3. **Read Overview**: Understand the component's purpose and approach
4. **Study Pseudocode**: Review implementation patterns and algorithms
5. **Cross-Reference**: Use both source and reference together for full context

---

## Contributing Guidelines

- **Keep It Concise**: Single-line descriptions, not full documentation
- **Focus on "How"**: Implementation approach, not feature descriptions
- **Use Rust Idioms**: Reference traits, lifetimes, ownership, async patterns
- **80 Columns Max**: Keep pseudocode readable in any editor
- **Update Together**: When source changes, update reference documentation
- **External Systems**: Always list AWS services, crates, and dependencies

---

## Examples

See individual component reference files for concrete examples of the methodology
in practice. Start with commonly referenced modules like:

- `reference/src/app/resource_explorer/aws_client.md` - Resource discovery
- `reference/src/app/agent_framework/agents/orchestration_agent.md` - Agent orchestration
- `reference/src/app/dashui/window_focus.md` - Window focus trait system

---

**Last Updated**: 2025-10-24
**Maintainer**: Development Team
**Status**: Active Development
