# Page Builder System

AI agent system for creating and editing interactive HTML/CSS/JS pages that display AWS data using the dashApp API.

## Overview

The Page Builder system enables Task Manager agents to spawn specialized PageBuilder worker agents that create complete web applications. These pages are stored in `~/.local/share/awsdash/pages/` and served via the `wry://localhost` custom protocol.

**Key capabilities:**
- Create interactive dashboards for AWS resources
- Edit existing pages based on user feedback
- Automatic workspace management with collision detection
- Progress tracking in parent agent's conversation

## How to Use

### Creating a New Page

Users request pages through natural language:

```
User: "Create a dashboard showing my Lambda functions"

TaskManager: Spawns PageBuilder worker with:
- workspace_name: "lambda-dashboard"
- task_description: "Build an interactive dashboard..."
```

The TaskManager uses the `start_page_builder` tool:

```json
{
  "workspace_name": "Lambda Dashboard",
  "concise_description": "Building Lambda dashboard",
  "task_description": "Create an interactive dashboard showing Lambda functions with filters for runtime and memory size.",
  "resource_context": "Lambda functions from all accounts with runtime, memory, timeout, last invoked"
}
```

### Editing an Existing Page

Users can request modifications:

```
User: "Add a search box to my Lambda dashboard"

TaskManager: Uses edit_page tool with:
- page_name: "lambda-dashboard"
- task_description: "Add a search box to filter functions by name"
```

The `edit_page` tool:
1. Validates the page exists
2. Passes `reuse_existing: true` to skip collision detection
3. PageBuilder reads existing files and applies changes

## How it Works

### Agent Hierarchy

```
User Request
    ↓
TaskManager Agent
    ├── start_page_builder tool → PageBuilder Worker
    │                               ↓
    │                            Creates HTML/CSS/JS
    │                               ↓
    │                            Files saved to workspace
    └── edit_page tool ─────────→ PageBuilder Worker (reuse workspace)
                                    ↓
                                 Reads existing files
                                    ↓
                                 Applies modifications
```

### Workspace Management

**Creation (start_page_builder):**
1. Suggested name sanitized to kebab-case (e.g., "Lambda Dashboard" → "lambda-dashboard")
2. Collision detection adds suffix if folder exists ("lambda-dashboard-2")
3. Directory created at `~/.local/share/awsdash/pages/{workspace}/`
4. PageBuilder receives sanitized workspace name

**Editing (edit_page):**
1. Validates page folder exists
2. Skips collision detection (`reuse_existing: true`)
3. Uses existing workspace name directly
4. PageBuilder reads and modifies existing files

### PageBuilder Worker Capabilities

PageBuilder workers have access to:

| Tool | Description |
|------|-------------|
| `read_file` | Read existing file contents |
| `write_file` | Create or overwrite files |
| `edit_file` | Make targeted changes to files |
| `list_files` | List files in workspace |

### File Structure

A typical page workspace:

```
~/.local/share/awsdash/pages/lambda-dashboard/
├── index.html      # Main HTML entry point
├── styles.css      # Styling
├── app.js          # Application logic
└── page.log        # Console.log output from runtime
```

### Progress Tracking

Worker progress displays inline in TaskManager's conversation:

```
[PageBuilder: Building Lambda dashboard]
    └── write_file: index.html (completed)
    └── write_file: styles.css (completed)
    └── write_file: app.js (running...)
```

## Tool Reference

### start_page_builder

Spawns a new PageBuilder worker to create a page.

**Parameters:**
- `workspace_name` (required): Suggested name for the workspace
- `concise_description` (required): 4-5 word description for UI display
- `task_description` (required): Full description of what to build
- `resource_context` (optional): What AWS resources/data to display

**Returns:**
```json
{
  "workspace_name": "lambda-dashboard",
  "result": "Page created successfully",
  "execution_time_ms": 45000
}
```

### edit_page

Modifies an existing page.

**Parameters:**
- `page_name` (required): Existing page folder name
- `concise_description` (required): 4-5 word description
- `task_description` (required): What changes to make

**Returns:**
```json
{
  "page_name": "lambda-dashboard",
  "result": "Changes applied successfully",
  "execution_time_ms": 30000
}
```

## TaskManager Guidelines

The TaskManager prompt includes specific guidance for page creation:

**When to create pages:**
- User explicitly requests a dashboard, tool, or view
- Large dataset benefits from interactive visualization
- User wants something persistent for repeated use

**When NOT to create pages:**
- Simple data queries ("show me S3 buckets")
- One-time lookups ("list Lambda functions")

**Important rules:**
- Never call `start_page_builder` unless user explicitly requests it
- Ask user if unsure whether they want a page or just data
- Provide comprehensive context to PageBuilder workers

## Configuration

### Workspace Directory

Pages are stored at:
```
~/.local/share/awsdash/pages/
```

This location is determined by `dirs::data_local_dir()`.

### Worker Timeout

PageBuilder workers have a 10-minute timeout (600 seconds) to complete their work, as page creation involves multiple file operations.

## Testing

### Verify Page Creation

1. Ask TaskManager to create a simple page
2. Check workspace was created in pages directory
3. Open page via Pages Manager
4. Verify dashApp API calls work

### Verify Edit Flow

1. Create a page
2. Request a modification
3. Verify existing files are preserved
4. Verify changes are applied correctly

## Key Source Files

- [`src/app/agent_framework/tools/orchestration/start_page_builder.rs`](../src/app/agent_framework/tools/orchestration/start_page_builder.rs) - Page creation tool
- [`src/app/agent_framework/tools/orchestration/edit_page.rs`](../src/app/agent_framework/tools/orchestration/edit_page.rs) - Page editing tool
- [`src/app/agent_framework/core/creation.rs`](../src/app/agent_framework/core/creation.rs) - Worker creation with reuse_existing logic
- [`src/app/agent_framework/utils/workspace.rs`](../src/app/agent_framework/utils/workspace.rs) - Workspace sanitization
- [`src/app/agent_framework/prompts/page_builder_worker.rs`](../src/app/agent_framework/prompts/page_builder_worker.rs) - PageBuilder system prompt

## Related Documentation

- [Multi-Agent System](multi-agent-system.md) - Task manager and worker orchestration
- [Webview API System](webview-api-system.md) - dashApp API that pages use
- [Pages Manager](pages-manager.md) - UI for managing created pages
- [Code Execution Tool](code-execution-tool.md) - V8 JavaScript execution
