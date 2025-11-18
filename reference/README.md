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

### CLOUDFORMATION
**Core template system**: Template parsing, resource definitions, intrinsic
functions, dependency graphs (DAG), and resource type specifications.

**Key Files**:
- `src/app/cfn_template.rs` - Template structure and validation
- `src/app/cfn_resources.rs` - AWS resource type management
- `src/app/cfn_dag.rs` - Dependency graph and topological analysis
- `src/app/cfn_intrinsic_functions.rs` - Ref, GetAtt, Sub, etc.
- `src/app/cfn_resource_icons.rs` - Icon mappings for 86+ AWS services

### PROJECTS
**Multi-environment project management**: Project persistence, environment
configurations (Dev/Staging/Prod), resource tracking, and template import/export.

**Key Files**:
- `src/app/projects.rs` - Project structure, environments, serialization

### AWS_IDENTITY
**AWS authentication and credential management**: Identity Center integration,
device authorization flow, multi-account credential handling, and SSO.

**Key Files**:
- `src/app/aws_identity.rs` - OAuth flow, credential caching, account enumeration

### BEDROCK
**AI integration via AWS Bedrock**: Model configuration, request/response handling,
and chat interface support.

**Key Files**:
- `src/app/bedrock_client.rs` - Bedrock API client and model management

### EXPLORER
**AWS resource discovery system**: Multi-account resource querying, cross-region
discovery, service-specific clients (86 AWS services), and data normalization.

**Key Files**:
- [[src/app/resource_explorer/aws_client|src/app/resource_explorer/aws_client.rs]] - Multi-account resource discovery
- `src/app/resource_explorer/services/*.rs` - 86 AWS service clients
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
- `src/app/dashui/*_window.rs` - 38+ specialized windows

### CFN_MANAGER
**CloudFormation deployment system**: Stack deployment orchestration, parameter
management, validation, and resource lookup integration.

**Key Files**:
- `src/app/cloudformation_manager/manager.rs` - Stack deployment orchestration
- `src/app/cloudformation_manager/deployment.rs` - Stack event tracking
- `src/app/cloudformation_manager/parameters.rs` - Parameter discovery
- `src/app/cloudformation_manager/parameter_dialog.rs` - User input collection
- `src/app/cloudformation_manager/secrets_manager.rs` - Secrets integration
- `src/app/cloudformation_manager/resource_lookup.rs` - Existing resource discovery

### AGENT FRAMEWORK
**AI agent system**: Task agents for AWS operations, tool registry, natural
language to AWS operation translation, and agent-tool communication.

**Key Files**:
- `src/app/agent_framework/agents/task_agent.rs` - Specialized AWS task agent
- `src/app/agent_framework/tools_registry.rs` - Tool registration and discovery
- `src/app/agent_framework/tools/*.rs` - 12 AWS operation tools
- `src/app/agent_framework/model_config.rs` - Model configuration
- `src/app/agent_framework/debug_logger.rs` - AI operation logging

### GUARD
**Compliance validation system**: CloudFormation Guard integration, rule
repository management, compliance program support (NIST, PCI-DSS, HIPAA, etc.),
and violation reporting.

**Key Files**:
- `src/app/cfn_guard.rs` - Guard validation engine integration
- `src/app/guard_repository_manager.rs` - Git-based rule repository
- `src/app/compliance_discovery.rs` - Compliance program discovery
- `src/app/repository_recovery.rs` - Error recovery mechanisms

### EVALUATION
**Agent evaluation framework**: Template-based evaluations, session logging,
hot prompt loading, and bridge message monitoring.

**Key Files**:
- `src/app/evaluation/template.rs` - Evaluation template definitions
- `src/app/evaluation/session_logging.rs` - Session-based logging
- `src/app/evaluation/prompt_loader.rs` - Hot prompt reloading
- `src/app/evaluation/bridge_message_monitor.rs` - Pattern matching

### NAVIGATION
**Keyboard navigation system**: Vimium-like shortcuts, hint mode, key mapping,
and navigable widget system.

**Key Files**:
- `src/app/dashui/keyboard_navigation.rs` - Navigation state machine
- `src/app/dashui/hint_mode.rs` - Visual element hints
- `src/app/dashui/key_mapping.rs` - Key binding configuration
- `src/app/dashui/navigable_widgets.rs` - Focusable widget trait

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

- `reference/src/app/cfn_template.md` - CloudFormation template parsing
- `reference/src/app/projects.md` - Project serialization and persistence
- `reference/src/app/dashui/window_focus.md` - Window focus trait system

---

**Last Updated**: 2025-10-24
**Maintainer**: Development Team
**Status**: Active Development
