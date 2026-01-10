# Pages Manager

Webview-based interface for viewing, renaming, and deleting pages created by the Page Builder system.

## Overview

The Pages Manager provides a web UI for managing all pages stored in `~/.local/share/awsdash/pages/`. It lists pages with metadata, supports fuzzy search, sortable columns, and allows viewing, renaming, or deleting pages.

**Key features:**
- Ant Design styled table interface
- Fuzzy search with Fuse.js
- Sortable columns (name, created, modified, size)
- Rename with automatic wry:// URL updates
- Click page name to view in new window

## How to Use

### Accessing the Pages Manager

Access via the application menu:
- **Menu**: Dash > Pages

Or via Command Palette:
- Press **Space** to open palette
- Press **P** for Pages

### Viewing Pages

Click any page name in the table to open it in a new webview window.

### Searching Pages

Type in the search box to filter pages using fuzzy matching. The search finds partial matches and handles typos.

### Sorting Pages

Click column headers to sort:
- **Name**: Alphabetical
- **Created**: By creation date
- **Last Modified**: By modification date
- **Size**: By total file size

Click again to toggle ascending/descending.

### Renaming Pages

1. Click **Rename** button on the page row
2. Enter new name in modal dialog
3. Click **Rename** to confirm

The rename operation:
- Renames the folder
- Updates all `wry://localhost/pages/{old-name}/` URLs in files to use the new name
- Recursively processes .html, .js, .css, .json, and other text files

### Deleting Pages

1. Click the trash icon button on the page row
2. Confirm deletion in the modal dialog
3. The page folder and all contents are permanently removed

## How it Works

### Architecture

```
┌─────────────────────────────────────────────┐
│  Pages Manager Window (Webview)             │
│  ┌───────────────────────────────────────┐  │
│  │  HTML/CSS/JS (Ant Design styled)      │  │
│  │  dashApp.listPages()                  │──┼──> API Server
│  │  dashApp.viewPage()                   │  │    ↓
│  │  dashApp.renamePage()                 │  │    commands.rs
│  │  dashApp.deletePage()                 │  │    ↓
│  └───────────────────────────────────────┘  │    File System
└─────────────────────────────────────────────┘    ~/.local/share/awsdash/pages/
```

### Page Metadata

Each page entry includes:

| Field | Description |
|-------|-------------|
| `name` | Folder name (workspace name) |
| `createdAt` | Unix timestamp of folder creation |
| `lastModified` | Most recent file modification time |
| `totalSize` | Sum of all file sizes in bytes |
| `fileCount` | Number of files in the folder |

### Rename URL Update Logic

When renaming, the system updates internal URLs:

```rust
fn update_wry_urls_in_directory(dir: &Path, old_name: &str, new_name: &str) {
    // For each text file (.html, .js, .css, .json, etc.)
    // Replace: wry://localhost/pages/{old_name}/
    // With:    wry://localhost/pages/{new_name}/
}
```

This ensures page assets (scripts, styles, images) continue to load correctly after rename.

### Fuzzy Search Implementation

Uses Fuse.js for client-side fuzzy matching:

```javascript
const fuse = new Fuse(pages, {
  keys: ['name'],
  threshold: 0.4,   // Tolerance for typos
  distance: 100,    // Character distance for matching
  includeScore: true
});

const results = fuse.search(query);
```

## API Methods Used

The Pages Manager uses these dashApp methods:

| Method | Description |
|--------|-------------|
| `listPages()` | Get all pages with metadata |
| `viewPage(name)` | Open page in new webview window |
| `renamePage(old, new)` | Rename folder and update URLs |
| `deletePage(name)` | Delete page folder permanently |

## UI Components

### Header

- Title: "AWS Dash Pages" with bolt icon
- Refresh button to reload page list

### Search Bar

- Fuzzy search input
- Results count indicator
- Hint text: "Use an Agent to create or edit pages"

### Table

| Column | Sortable | Content |
|--------|----------|---------|
| Name | Yes | Clickable page name, file count |
| Created | Yes | Formatted date |
| Last Modified | Yes | Formatted date |
| Size | Yes | Human-readable size badge |
| Actions | No | Rename and Delete buttons |

### Modals

- **Delete Confirmation**: Warns about permanent deletion
- **Rename Dialog**: Input field for new name, Enter to submit

## Styling

The UI uses Ant Design CSS from CDN:

```html
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/antd/5.12.8/reset.min.css">
```

Custom CSS variables provide consistent theming:

```css
:root {
  --primary-color: #1890ff;
  --success-color: #52c41a;
  --warning-color: #faad14;
  --error-color: #ff4d4f;
  --text-color: rgba(0, 0, 0, 0.85);
  --border-color: #d9d9d9;
}
```

## Key Source Files

- [`src/app/webview/pages_manager_window.rs`](../src/app/webview/pages_manager_window.rs) - HTML generation
- [`src/app/webview/commands.rs`](../src/app/webview/commands.rs) - listPages, viewPage, renamePage, deletePage
- [`src/app/dashui/menu.rs`](../src/app/dashui/menu.rs) - Menu integration
- [`src/app/dashui/command_palette.rs`](../src/app/dashui/command_palette.rs) - Command palette entry

## Related Documentation

- [Page Builder System](page-builder-system.md) - How pages are created
- [Webview API System](webview-api-system.md) - dashApp API details
- [Command Palette System](command-palette-system.md) - Keyboard navigation
