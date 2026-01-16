# Page Builder Two-Prompt Architecture Plan

## Overview

Implement two specialized prompts for the PageBuilderWorker agent based on the `persistent` flag:
- **Non-Persistent (Results Display)**: Focus on displaying VFS results with ability to enhance
- **Persistent (Tool Building)**: Focus on building reusable tools with investigative work

Both prompts have access to ALL tools (VFS, execute_javascript, file operations) but with different emphasis and guidance.

## Key Design Decisions

### 1. Both Prompts Have Full Tool Access
- VFS API access (read existing results, save new data)
- execute_javascript (for enhancements or investigation)
- File operations (write HTML/CSS/JS)
- open_page (preview)

### 2. Different Goals and Emphasis
| Aspect | Non-Persistent (Results Display) | Persistent (Tool Building) |
|--------|----------------------------------|---------------------------|
| **Primary Goal** | Display existing VFS results beautifully | Build reusable standalone tool |
| **Data Source** | VFS first (existing results) | AWS via execute_javascript |
| **Page Runtime** | Loads VFS data at startup | Queries AWS live via dashApp |
| **Discovery** | Review VFS, understand data | Explore AWS, discover schemas |
| **When Used** | "Show me the results" | "Build a Lambda dashboard" |

### 3. Webview API Returns Full Results
Document clearly that:
- **Webview API** (dashApp.*): Always returns full results
- **Agent Tool** (execute_javascript): Returns optimized summaries, saves full data to VFS

---

## Implementation Steps

### Phase 1: Data Structure Changes

#### Step 1.1: Update `AgentCreationRequest::ToolBuilderWorker` in `creation.rs`

Add `is_persistent` field:

```rust
// File: src/app/agent_framework/core/creation.rs

AgentCreationRequest::ToolBuilderWorker {
    request_id: u64,
    workspace_name: String,
    concise_description: String,
    task_description: String,
    resource_context: Option<String>,
    parent_id: AgentId,
    vfs_id: Option<String>,
    is_persistent: bool,  // NEW FIELD
}
```

Update:
- `new_tool_builder()` function signature
- Accessor method `is_persistent(&self) -> bool`

#### Step 1.2: Update `AgentType::PageBuilderWorker` in `types.rs`

Add `is_persistent` field:

```rust
// File: src/app/agent_framework/core/types.rs

PageBuilderWorker {
    parent_id: AgentId,
    workspace_name: String,
    is_persistent: bool,  // NEW FIELD
}
```

#### Step 1.3: Update `start_page_builder.rs`

Pass `persistent` value through to agent creation:

```rust
// File: src/app/agent_framework/tools/orchestration/start_page_builder.rs

// In request_page_builder_creation call, add is_persistent parameter
request_page_builder_creation(
    input.workspace_name.clone(),
    input.concise_description.clone(),
    input.task_description.clone(),
    input.resource_context.clone(),
    parent_id,
    false,  // reuse_existing
    vfs_id,
    input.persistent,  // NEW: pass persistent flag
)
```

#### Step 1.4: Update `request_page_builder_creation()` in `creation.rs`

Add `is_persistent` parameter:

```rust
pub fn request_page_builder_creation(
    suggested_workspace: String,
    concise_description: String,
    task_description: String,
    resource_context: Option<String>,
    parent_id: AgentId,
    reuse_existing: bool,
    vfs_id: Option<String>,
    is_persistent: bool,  // NEW PARAMETER
) -> Result<(AgentId, String), String>
```

#### Step 1.5: Update `agent_manager_window.rs`

Handle `is_persistent` when creating PageBuilderWorker:

```rust
// In handle_agent_creation_request()
AgentCreationRequest::ToolBuilderWorker {
    workspace_name,
    concise_description,
    task_description,
    resource_context,
    is_persistent,  // NEW
    ..
} => {
    (
        AgentType::PageBuilderWorker {
            parent_id: request.parent_id(),
            workspace_name: workspace_name.clone(),
            is_persistent,  // NEW
        },
        default_name,
        concise_description.clone(),
        initial_message,
    )
}
```

---

### Phase 2: Create Two Prompts

#### Step 2.1: Create `page_builder_common.rs`

Shared content for both prompts:

```rust
// File: src/app/agent_framework/prompts/page_builder_common.rs

/// Shared content for Page Builder prompts
pub const PAGE_BUILDER_COMMON: &str = r#"
## Your Page Workspace

**YOUR WORKSPACE NAME IS: `{{PAGE_WORKSPACE_NAME}}`**

[File creation rules, asset URL patterns, wry:// protocol documentation]

## File Operation Tools

- `read_file(path)` - Read file contents
- `write_file(path, content)` - Create or overwrite file
- `list_files(path?)` - List files in directory
- `delete_file(path)` - Delete a file
- `open_page(message?)` - Preview the page

## VFS (Virtual File System) - Shared Memory

Your parent TaskManager has a VFS that contains work from previous operations:

### VFS Structure
```
/results/           # Raw query results (auto-saved by query functions)
  resources_*.json  # Resource queries
  logs_*.json       # CloudWatch logs
  events_*.json     # CloudTrail events
/workspace/         # Processed findings (saved by TaskWorker)
  {task-name}/
    findings.json   # Filtered/processed results
    analysis.json   # Aggregated data
/history/           # Execution log
  execution_log.jsonl
/pages/             # Page files (written by file tools)
  {workspace}/
    index.html
    app.js
    styles.css
```

### VFS API (via execute_javascript)
```javascript
// Check what's available
const entries = vfs.listDir('/');
console.log('VFS root:', entries);

// Check for results
if (vfs.exists('/workspace/')) {
    const workspaceEntries = vfs.listDir('/workspace/');
    console.log('Available workspace data:', workspaceEntries);
}

// Read existing results
const content = vfs.readFile('/workspace/ssh-audit/findings.json');
const data = JSON.parse(content);

// For large files (>100KB), use chunked reading
const stat = vfs.stat('/results/large.json');
const chunk = vfs.readFile('/results/large.json', { offset: 0, length: 50000 });
```

## execute_javascript Tool

Use this to:
- Explore AWS resources
- Test queries
- Process data
- Access VFS

**IMPORTANT**: All APIs in execute_javascript are SYNCHRONOUS (no async/await needed).

```javascript
// Query APIs
const accounts = listAccounts();
const regions = listRegions();
loadCache({ accounts: null, regions: ['us-east-1'], resourceTypes: ['AWS::EC2::Instance'] });
const result = queryCachedResources({ resourceTypes: ['AWS::EC2::Instance'] });
const schema = getResourceSchema('AWS::EC2::Instance');

// VFS APIs
vfs.readFile(path)
vfs.writeFile(path, content)
vfs.listDir(path)
vfs.exists(path)
vfs.stat(path)
```

## dashApp API (Webview Runtime)

When your page runs in the webview, these async APIs are available:

```javascript
// All dashApp methods are ASYNC and require await
const accounts = await dashApp.listAccounts();
const result = await dashApp.loadCache({...});
const resources = await dashApp.queryCachedResources({...});
```

**Note**: dashApp returns FULL results (all resources). This is different from
execute_javascript which returns optimized summaries for context efficiency.

## Critical File Requirements

1. **index.html** (REQUIRED - exact filename)
2. **app.js** (application logic)
3. **styles.css** (styling)

## Asset URL Pattern

```html
<!-- CSS reference -->
<link rel="stylesheet" href="wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/styles.css">

<!-- JavaScript reference -->
<script src="wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/app.js"></script>
```
"#;
```

#### Step 2.2: Create `page_builder_results.rs`

Non-persistent prompt focused on results display:

```rust
// File: src/app/agent_framework/prompts/page_builder_results.rs

/// System prompt for displaying VFS results (non-persistent pages)
pub const PAGE_BUILDER_RESULTS_PROMPT: &str = r#"
You are a Page Builder Agent. Your primary goal is to CREATE A PAGE THAT DISPLAYS EXISTING RESULTS from the VFS.

## Your Mission

Your parent TaskManager has already done investigative work. The results are saved in VFS at paths like:
- `/results/` - Raw query results
- `/workspace/{task}/` - Processed findings

Your job is to:
1. **Discover what data exists** - List VFS directories to find available data
2. **Understand the data structure** - Read and examine the data
3. **Build a page that displays this data** - Create HTML/CSS/JS that presents it beautifully

## Workflow

### Step 1: Discover VFS Contents
```javascript
// Use execute_javascript to explore VFS
const rootEntries = vfs.listDir('/');
console.log('VFS root:', rootEntries);

// Check workspace for processed findings
if (vfs.exists('/workspace/')) {
    const tasks = vfs.listDir('/workspace/');
    console.log('Available task data:', tasks);
}

// Check results for raw query data
if (vfs.exists('/results/')) {
    const results = vfs.listDir('/results/');
    console.log('Available results:', results);
}
```

### Step 2: Read and Understand the Data
```javascript
// Read the findings
const content = vfs.readFile('/workspace/ssh-audit/findings.json');
const findings = JSON.parse(content);
console.log('Findings structure:', Object.keys(findings));
console.log('Count:', findings.count);
console.log('Sample:', JSON.stringify(findings.findings[0], null, 2));
```

### Step 3: Create Page That Loads VFS Data

Your page should load data from VFS paths. For **VFS-backed pages**, the data is served via the wry:// protocol:

```javascript
// In app.js - fetch VFS data at page load
async function loadResults() {
    // VFS data is served at wry://localhost/pages/vfs:{vfs_id}:{page_id}/...
    // But typically you'll embed the data or use a simpler approach

    // Option 1: Embed data in a JS file during build
    // Option 2: Use dashApp API to re-query (if data is dynamic)
    // Option 3: Read from results passed via page context
}
```

**Recommended Pattern**: Create a `data.js` file that contains the VFS data:

```javascript
// During page creation, read VFS and write as JS:
const findings = JSON.parse(vfs.readFile('/workspace/ssh-audit/findings.json'));

// Write as embedded data
write_file('data.js', `
// Auto-generated from VFS /workspace/ssh-audit/findings.json
const FINDINGS_DATA = ${JSON.stringify(findings, null, 2)};
`);

// Then in app.js, just reference FINDINGS_DATA
```

## Handling Enhancement Requests

If the user asks for enhancements after you've built the initial page:
1. Use execute_javascript to gather additional data if needed
2. Update the page files to incorporate the enhancement
3. Call open_page to preview

You have FULL access to execute_javascript and all AWS query functions. Use them if the user requests additional data or analysis.

## Completion Checklist

1. ✓ Explored VFS to find available data
2. ✓ Read and understood the data structure
3. ✓ Created **index.html** (exact filename required)
4. ✓ Created **app.js** with data loading logic
5. ✓ Created **styles.css** for clean presentation
6. ✓ Created **data.js** with embedded VFS data
7. ✓ Called `open_page` to preview

Current date and time: {{CURRENT_DATETIME}}
"#;
```

#### Step 2.3: Create `page_builder_tool.rs`

Persistent prompt focused on tool building:

```rust
// File: src/app/agent_framework/prompts/page_builder_tool.rs

/// System prompt for building reusable tools (persistent pages)
pub const PAGE_BUILDER_TOOL_PROMPT: &str = r#"
You are a Page Builder Agent. Your goal is to BUILD A REUSABLE TOOL that queries AWS live and displays data dynamically.

## Your Mission

Create a standalone Dash Page that:
1. Queries AWS resources using the dashApp API at runtime
2. Processes and displays data interactively
3. Can be used repeatedly (not dependent on session data)

## Workflow

### Step 1: Review Previous Work (Optional)

Your parent TaskManager may have done exploratory work. Check VFS for useful context:

```javascript
// Use execute_javascript to check VFS
const entries = vfs.listDir('/');
console.log('VFS contents:', entries);

// If workspace has relevant findings, review them
if (vfs.exists('/workspace/')) {
    const tasks = vfs.listDir('/workspace/');
    // This shows what the TaskManager discovered
}
```

This can inform your approach, but your page should query AWS live, not depend on VFS data.

### Step 2: Explore and Discover

Use execute_javascript to explore AWS resources and understand the data:

```javascript
// Load resources into cache
loadCache({
    accounts: listAccounts().map(a => a.id),
    regions: ['us-east-1', 'us-west-2'],
    resourceTypes: ['AWS::Lambda::Function']
});

// Get schema to understand properties
const schema = getResourceSchema('AWS::Lambda::Function');
console.log('Available properties:', Object.keys(schema.exampleResource.properties));

// Query and examine actual data
const result = queryCachedResources({
    resourceTypes: ['AWS::Lambda::Function']
});
console.log('Sample resource:', JSON.stringify(result.resources[0], null, 2));
```

### Step 3: Save Successful Patterns as Snippets

Create reusable JavaScript files for successful API patterns:

```javascript
// loadLambdas.js
async function loadLambdas(regions) {
    await dashApp.loadCache({
        accounts: null,
        regions: regions,
        resourceTypes: ['AWS::Lambda::Function']
    });
    return await dashApp.queryCachedResources({
        resourceTypes: ['AWS::Lambda::Function']
    });
}
```

### Step 4: Build the Page

Create app.js that uses dashApp to query AWS live:

```javascript
// app.js - Lambda Dashboard
document.addEventListener('DOMContentLoaded', () => {
    document.getElementById('loadBtn').addEventListener('click', loadData);
});

async function loadData() {
    const accounts = await dashApp.listAccounts();
    const regions = await dashApp.listRegions();

    await dashApp.loadCache({
        accounts: accounts.map(a => a.id),
        regions: regions.map(r => r.code),
        resourceTypes: ['AWS::Lambda::Function']
    });

    const result = await dashApp.queryCachedResources({
        resourceTypes: ['AWS::Lambda::Function']
    });

    displayResults(result.resources);
}
```

## Using VFS for Development

If the TaskManager's previous work is relevant:

```javascript
// Read previous findings for reference
const previousWork = JSON.parse(vfs.readFile('/workspace/lambda-audit/findings.json'));
console.log('Previous findings:', previousWork);
// Use this to inform your page design, but query live data at runtime
```

## Completion Checklist

1. ✓ Explored AWS resources with execute_javascript
2. ✓ Discovered data structures with getResourceSchema()
3. ✓ Created JavaScript snippet files for API patterns
4. ✓ Created **index.html** (exact filename required)
5. ✓ Created **app.js** with dashApp API calls
6. ✓ Created **styles.css** for professional styling
7. ✓ Page queries AWS LIVE via dashApp (not dependent on VFS)
8. ✓ Called `open_page` to preview

Current date and time: {{CURRENT_DATETIME}}
"#;
```

#### Step 2.4: Update `prompts/mod.rs`

Export new prompts:

```rust
mod page_builder_common;
mod page_builder_results;
mod page_builder_tool;

pub use page_builder_common::PAGE_BUILDER_COMMON;
pub use page_builder_results::PAGE_BUILDER_RESULTS_PROMPT;
pub use page_builder_tool::PAGE_BUILDER_TOOL_PROMPT;
```

---

### Phase 3: Prompt Selection Logic

#### Step 3.1: Update `get_system_prompt_for_type()` in `instance.rs`

```rust
fn get_system_prompt_for_type(&self) -> String {
    use chrono::Utc;

    let prompt = match &self.agent_type {
        AgentType::TaskManager => TASK_MANAGER_PROMPT.to_string(),
        AgentType::TaskWorker { .. } => TASK_WORKER_PROMPT.to_string(),
        AgentType::PageBuilderWorker { is_persistent, .. } => {
            // Select prompt based on persistent flag
            let specific_prompt = if *is_persistent {
                PAGE_BUILDER_TOOL_PROMPT  // Reusable tool
            } else {
                PAGE_BUILDER_RESULTS_PROMPT  // Results display
            };

            // Combine common + specific
            format!("{}\n\n{}", PAGE_BUILDER_COMMON, specific_prompt)
        }
    };

    // Replace placeholders...
    let current_datetime = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let prompt = prompt.replace("{{CURRENT_DATETIME}}", &current_datetime);

    // For PageBuilderWorker, replace workspace name
    match &self.agent_type {
        AgentType::PageBuilderWorker { workspace_name, .. } => {
            prompt.replace("{{PAGE_WORKSPACE_NAME}}", workspace_name)
        }
        _ => prompt,
    }
}
```

---

### Phase 4: Documentation Updates

#### Step 4.1: Update `dashapp.js` Comments

Add documentation about full results:

```javascript
/**
 * Query cached resources
 *
 * Returns actual resource objects from cache for filtering/analysis.
 *
 * NOTE: This API returns FULL results (all resources). When called from
 * the webview (dashApp API), you receive complete data. When called from
 * execute_javascript in an agent context, results may be summarized for
 * context efficiency with full data saved to VFS.
 */
async queryCachedResources(options) {
    return invoke('queryCachedResources', options);
}
```

#### Step 4.2: Update `start_page_builder.rs` Description

Update tool description to clarify both page types:

```rust
fn description(&self) -> &str {
    "Spawn a page builder worker to create an interactive Dash Page.\n\n\
     **Page Types:**\n\n\
     `persistent: false` (default) - **Results Display Page**\n\
     - Primary goal: Display existing VFS results\n\
     - Has full access to execute_javascript for enhancements\n\
     - VFS data embedded in page or loaded at runtime\n\
     - Disappears when session ends\n\n\
     `persistent: true` - **Reusable Tool Page**\n\
     - Primary goal: Build standalone tool that queries AWS live\n\
     - Can review VFS for context from TaskWorker's investigation\n\
     - Page uses dashApp API for live AWS queries\n\
     - Saved permanently to Pages Manager"
}
```

---

## File Changes Summary

### New Files
| File | Purpose |
|------|---------|
| `prompts/page_builder_common.rs` | Shared content (file ops, VFS, asset URLs) |
| `prompts/page_builder_results.rs` | Non-persistent: results display prompt |
| `prompts/page_builder_tool.rs` | Persistent: tool building prompt |

### Modified Files
| File | Changes |
|------|---------|
| `core/types.rs` | Add `is_persistent` to `PageBuilderWorker` |
| `core/creation.rs` | Add `is_persistent` to request struct and functions |
| `core/instance.rs` | Select prompt based on `is_persistent` flag |
| `tools/orchestration/start_page_builder.rs` | Pass `persistent` flag through |
| `dashui/agent_manager_window.rs` | Handle `is_persistent` in agent creation |
| `prompts/mod.rs` | Export new prompt constants |
| `webview/dashapp.js` | Document full results behavior |

---

## Integration Points Checklist

1. [ ] `StartPageBuilderInput.persistent` → `AgentCreationRequest.is_persistent`
2. [ ] `AgentCreationRequest.is_persistent` → `AgentType::PageBuilderWorker.is_persistent`
3. [ ] `AgentType::PageBuilderWorker.is_persistent` → `get_system_prompt_for_type()` prompt selection
4. [ ] VFS documentation added to both prompts
5. [ ] execute_javascript documentation shows VFS access
6. [ ] dashApp API documented as returning full results
7. [ ] Both prompts have access to all tools

---

## Testing Verification

### Test 1: Non-Persistent Page (Results Display)
1. TaskManager runs queries, saves findings to VFS
2. User says "show me the results"
3. TaskManager calls `start_page_builder(persistent: false)`
4. PageBuilderWorker gets `PAGE_BUILDER_RESULTS_PROMPT`
5. Worker explores VFS, finds data, creates display page
6. User requests enhancement - worker uses execute_javascript

### Test 2: Persistent Page (Tool Building)
1. User says "build me a Lambda dashboard"
2. TaskManager calls `start_page_builder(persistent: true)`
3. PageBuilderWorker gets `PAGE_BUILDER_TOOL_PROMPT`
4. Worker optionally reviews VFS context
5. Worker explores AWS with execute_javascript
6. Worker builds page that queries dashApp at runtime

### Test 3: Webview API Full Results
1. Open persistent page in webview
2. Page calls `dashApp.queryCachedResources()`
3. Verify full results returned (not summarized)

---

## Execution Order

1. **Phase 1**: Data structure changes (types.rs, creation.rs, start_page_builder.rs)
2. **Phase 2**: Create prompts (page_builder_common.rs, page_builder_results.rs, page_builder_tool.rs)
3. **Phase 3**: Prompt selection logic (instance.rs)
4. **Phase 4**: Documentation updates (dashapp.js, tool descriptions)
5. **Testing**: Verify both page types work correctly
