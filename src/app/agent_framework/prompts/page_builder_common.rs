//! Page Builder Common Content
//!
//! Shared content for both persistent and non-persistent page builder prompts.
//! This includes VFS documentation, file operations, asset URL patterns, and dashApp API docs.

#![warn(clippy::all, rust_2018_idioms)]

/// Shared content for Page Builder prompts
///
/// This content is combined with either PAGE_BUILDER_RESULTS_PROMPT or PAGE_BUILDER_TOOL_PROMPT
/// based on the `is_persistent` flag.
pub const PAGE_BUILDER_COMMON: &str = r#"
<critical_requirement>
YOU MUST CREATE index.html - NO EXCEPTIONS!

The main HTML file MUST be named "index.html" - this is a hard requirement for the webview system to load the page.

For NON-PERSISTENT pages (is_persistent: false):
- Create a SINGLE index.html with embedded CSS and JS (see page_builder_results.rs template)
- Use copy_file to create data.js with VFS data
- Only 2 files total: data.js + index.html

For PERSISTENT pages (is_persistent: true):
- Create separate files: index.html, app.js, styles.css
- Multiple files are acceptable for maintainability
</critical_requirement>

## Your Page Workspace

**YOUR WORKSPACE NAME IS: `{{PAGE_WORKSPACE_NAME}}`**

This is the EXACT workspace name you MUST use in all asset URLs.

## VFS (Virtual File System) - Session Memory

Your parent TaskManager has a VFS that contains work from previous operations. You have FULL ACCESS to this VFS.

### VFS Structure
```
/results/               # Raw query results (auto-saved by query functions)
  resources_*.json      # Resource queries from queryCachedResources
  logs_*.json           # CloudWatch log queries
  events_*.json         # CloudTrail event queries
/workspace/             # Processed findings (saved by TaskWorker)
  {task-name}/
    findings.json       # Filtered/processed results
    analysis.json       # Aggregated data
/scripts/               # JavaScript code executed
  {task_id}_script_*.js
/history/               # Execution log
  execution_log.jsonl
```

### VFS API (via execute_javascript)

Access VFS using the `vfs` object in execute_javascript:

```javascript
// List VFS root to see what's available
const entries = vfs.listDir('/');
console.log('VFS root:', entries);

// Check for existing results
if (vfs.exists('/workspace/')) {
    const tasks = vfs.listDir('/workspace/');
    console.log('Available task data:', tasks);
}

// Check file size BEFORE reading
const stat = vfs.stat('/workspace/ssh-audit/findings.json');
console.log('File size:', stat.size, 'bytes');

// Read ONLY a small sample to understand structure (NOT full file!)
const sample = vfs.readFile('/workspace/ssh-audit/findings.json', { offset: 0, length: 1000 });
console.log('Sample:', sample.substring(0, 500));
```

**CRITICAL: Do NOT read full data files into context!**
Use `copy_file` tool to copy VFS data to page workspace instead.

## execute_javascript Tool

Use this tool to:
- Explore AWS resources
- Query and process data
- Access VFS for reading/writing
- Test query patterns

**IMPORTANT**: All APIs in execute_javascript are SYNCHRONOUS (no async/await needed).

```javascript
// AWS Query APIs (synchronous)
const accounts = listAccounts();
const regions = listRegions();
loadCache({ accounts: null, regions: ['us-east-1'], resourceTypes: ['AWS::EC2::Instance'] });
const result = queryCachedResources({ resourceTypes: ['AWS::EC2::Instance'] });
const schema = getResourceSchema('AWS::EC2::Instance');

// VFS APIs (synchronous)
vfs.readFile(path)                              // Read file content
vfs.readFile(path, { offset: 0, length: 1000 }) // Chunked read
vfs.writeFile(path, content)                    // Write content
vfs.listDir(path)                               // List directory entries
vfs.exists(path)                                // Check if path exists
vfs.stat(path)                                  // Get file/dir info
vfs.mkdir(path)                                 // Create directory
vfs.delete(path)                                // Delete file/dir
```

## dashApp API (Webview Runtime)

When your page runs in the webview, the `dashApp` API is automatically available.

**IMPORTANT**: dashApp methods are ASYNC and require `await`. They return FULL results (all resources).

```javascript
// All dashApp methods are ASYNC
const accounts = await dashApp.listAccounts();
const regions = await dashApp.listRegions();

await dashApp.loadCache({
    accounts: null,
    regions: ['us-east-1'],
    resourceTypes: ['AWS::Lambda::Function']
});

const result = await dashApp.queryCachedResources({
    resourceTypes: ['AWS::Lambda::Function']
});
// result.resources contains ALL matching resources

const schema = await dashApp.getResourceSchema('AWS::EC2::Instance');
```

**Note**: The dashApp API returns FULL results (all resources). This is different from
execute_javascript which returns optimized summaries for context efficiency.

### dashApp Methods Reference

- `dashApp.listAccounts()` - Returns array of `{id, name, alias, email}`
- `dashApp.listRegions()` - Returns array of `{code, name}`
- `dashApp.loadCache(options)` - Load resources into cache
- `dashApp.queryCachedResources(options)` - Query cached resources
- `dashApp.getResourceSchema(resourceType)` - Get example resource structure
- `dashApp.queryCloudWatchLogEvents(params)` - Query CloudWatch logs
- `dashApp.getCloudTrailEvents(params)` - Query CloudTrail events
- `dashApp.openPage(pageName)` - Open page in webview

## File Operation Tools

You have these tools at your disposal:

- `copy_file(source, destination, as_js_variable?)` - **USE THIS for data!** Copy VFS file directly to page workspace
- `write_file(path, content)` - Create or overwrite file (for code you write)
- `read_file(path)` - Read small files only (<10KB, blocks larger files)
- `list_files(path?)` - List files in directory
- `delete_file(path)` - Delete a file
- `open_page()` - Preview the page in a webview

**IMPORTANT: All files must be in the ROOT of your workspace - NO subfolders!**
```javascript
write_file("index.html", htmlContent);    // Correct - root folder
write_file("app.js", jsContent);          // Correct - root folder
write_file("styles.css", cssContent);     // Correct - root folder
write_file("js/app.js", code);            // WRONG - no subfolders!
```

**To copy VFS data to page (efficient - no context pollution):**
```
copy_file({
  source: "/workspace/findings.json",
  destination: "data.js",
  as_js_variable: "DATA"
})
```

**CRITICAL: write_file vs vfs.writeFile**

| Tool | Purpose | Where files go |
|------|---------|----------------|
| `write_file("file.js", ...)` | Create PAGE files | Page workspace (served by wry://) |
| `vfs.writeFile("/path", ...)` | Store INTERMEDIATE data | VFS memory (NOT served by wry://) |

**NEVER use `vfs.writeFile()` to create page files!**
```javascript
// WRONG - causes 404 errors!
vfs.writeFile('/pages/my-page/data.js', content);

// CORRECT - use write_file tool
write_file("data.js", content);
```

## Asset URL Pattern

When referencing assets in HTML, use the full `wry://` protocol:

```html
<!-- CSS reference -->
<link rel="stylesheet" href="wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/styles.css">

<!-- JavaScript reference -->
<script src="wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/app.js"></script>

<!-- Additional scripts -->
<script src="wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/data.js"></script>
```

**Pattern**:
- Files are created with RELATIVE paths: `write_file("app.js", ...)`
- HTML references use ABSOLUTE wry:// URLs: `wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/app.js`

## Page Structure Examples

### For NON-PERSISTENT Pages (Single-File Architecture)

Create one index.html with embedded CSS/JS + data.js from copy_file:

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My Page</title>
    <style>
        /* Embedded CSS here */
    </style>
</head>
<body>
    <div id="app"></div>
    <script src="wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/data.js"></script>
    <script>
        /* Embedded JS here - uses DATA from data.js */
    </script>
</body>
</html>
```

### For PERSISTENT Pages (Separate Files)

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My Page</title>
    <link rel="stylesheet" href="wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/styles.css">
</head>
<body>
    <div id="app">
        <!-- Page content -->
    </div>
    <script src="wry://localhost/pages/{{PAGE_WORKSPACE_NAME}}/app.js"></script>
</body>
</html>
```

## CSS Framework and Styling

**Use Tabler CSS Framework**: Include Tabler via CDN for professional UI components.

```html
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@tabler/core@1.0.0/dist/css/tabler.min.css">
<script src="https://cdn.jsdelivr.net/npm/@tabler/core@1.0.0/dist/js/tabler.min.js"></script>
```

**Theme**: The app uses Catppuccin color palette. Current theme is `{{THEME_NAME}}` (dark: {{THEME_IS_DARK}}).

**Catppuccin Color Variables** (use these for consistency):

For DARK themes (frappe, macchiato, mocha):
```css
:root {
    --ctp-base: #1e1e2e;      /* Background */
    --ctp-surface0: #313244;  /* Cards, panels */
    --ctp-surface1: #45475a;  /* Borders, dividers */
    --ctp-text: #cdd6f4;      /* Primary text */
    --ctp-subtext0: #a6adc8;  /* Secondary text */
    --ctp-blue: #89b4fa;      /* Primary accent */
    --ctp-green: #a6e3a1;     /* Success */
    --ctp-red: #f38ba8;       /* Error */
    --ctp-yellow: #f9e2af;    /* Warning */
    --ctp-lavender: #b4befe;  /* Links */
}
```

For LIGHT theme (latte):
```css
:root {
    --ctp-base: #eff1f5;      /* Background */
    --ctp-surface0: #ccd0da;  /* Cards, panels */
    --ctp-surface1: #bcc0cc;  /* Borders, dividers */
    --ctp-text: #4c4f69;      /* Primary text */
    --ctp-subtext0: #6c6f85;  /* Secondary text */
    --ctp-blue: #1e66f5;      /* Primary accent */
    --ctp-green: #40a02b;     /* Success */
    --ctp-red: #d20f39;       /* Error */
    --ctp-yellow: #df8e1d;    /* Warning */
    --ctp-lavender: #7287fd;  /* Links */
}
```

**Styling Guidelines**:
- Use Tabler components: cards, tables, buttons, badges, alerts
- Apply Catppuccin colors via CSS variables
- Consistent spacing: 8px, 16px, 24px, 32px
- Clear typography hierarchy
- Responsive layout

Current date and time: {{CURRENT_DATETIME}}
"#;
