# Multi-Pane Architecture

The Resource Explorer supports a multi-pane interface where each window can display one or two independent resource views side-by-side.

## Overview

The multi-pane architecture replaces the previous tab-based system with a direct window-to-panes structure. Each Explorer window contains:
- **Left pane** - Always visible, displays primary resource view
- **Right pane** - Optional, toggled via button, displays secondary resource view

Each pane operates independently with its own query state, progress tracking, and resource display.

## How to Use

### Opening a Split Pane View

1. Open the Resource Explorer window
2. Click the split pane toggle button in the toolbar
3. The right pane appears with an independent resource view
4. Click the toggle button again to close the right pane

### Working with Multiple Panes

Each pane maintains independent state for:
- **Resource selection** - Select different accounts, regions, or resource types per pane
- **Query progress** - Each pane tracks its own Phase 1/Phase 2 loading status
- **Search filter** - Filter resources independently in each pane
- **Grouping mode** - Organize resources differently per pane

You can compare resources across different scopes by configuring each pane with different selections.

### Loading Bookmarks into Panes

When you load a bookmark, it applies to the currently focused pane. This allows you to:
1. Load one bookmark into the left pane
2. Load a different bookmark into the right pane
3. Compare the results side-by-side

## How it Works

### Architecture Overview

```
ResourceExplorerWindow
├── ExplorerInstance (left pane)
│   ├── PaneRenderer
│   ├── ResourceExplorerState
│   └── QueryEngine
└── ExplorerInstance (right pane, optional)
    ├── PaneRenderer
    ├── ResourceExplorerState
    └── QueryEngine
```

### Key Components

**ResourceExplorerWindow** (`src/app/resource_explorer/window.rs`)
- Top-level window container
- Manages split pane toggle state
- Routes actions to the appropriate pane
- Handles window-level dialogs (bookmarks, filters)

**ExplorerInstance** (`src/app/resource_explorer/instances/mod.rs`)
- Represents a single pane
- Contains independent state and renderer
- Executes queries via dedicated query engine
- Tracks progress and failed queries

**PaneRenderer** (`src/app/resource_explorer/instances/pane_renderer.rs`)
- Renders tree view of resources
- Displays search bar and active selection tags
- Shows per-pane status bar with progress indicators
- Returns actions for window-level processing

**PaneAction** (`src/app/resource_explorer/instances/pane_renderer.rs:54`)
- Actions triggered by pane rendering (remove account, apply bookmark, etc.)
- Includes source pane ID for pane-aware routing
- Processed by window or manager to update state

### Pane-Aware Action Routing

When a user clicks "Remove Account" in the right pane, the system:
1. PaneRenderer returns `PaneAction::RemoveAccount { account_id, source_pane_id }`
2. Window receives the action with source pane identification
3. Window routes the action to the correct pane's state
4. Only that pane's query refreshes

This prevents actions in one pane from affecting the other pane.

### Independent Progress Tracking

Each pane displays its own status bar showing:
- **Phase 1 progress** - Resource listing (e.g., "45/50 complete")
- **Phase 2 progress** - Property enrichment (e.g., "12/45 enriching")
- **Failed queries** - Count of queries that failed
- **Cache stats** - Hit rate and entry count

The status bars update independently as each pane's queries complete.

## Visual Design

### Split Pane Separator

A visible 2px gray separator divides the left and right panes, making it clear which resources belong to which pane.

### Per-Pane Status Bars

Each pane has its own status bar at the bottom showing:
```
Phase 1: 45/50 | Phase 2: 12/45 | Failed: 2 | Cache: 85% hit (1234 entries)
```

### Active Selection Tags

Selection tags (accounts, regions, resource types) display with colored backgrounds:
- Yellow background for accounts
- Green background for regions
- Blue background for resource types

Each tag includes an "X" button to remove that selection from the current pane.

## Integration Points

### Query Engine

Each pane creates its own QueryEngine instance for executing Phase 1 and Phase 2 queries. See [Resource Explorer System](resource-explorer-system.md) for query execution details.

### Bookmark System

Bookmarks apply to the focused pane. When loading a bookmark:
1. Determine which pane has focus
2. Apply bookmark selections to that pane's state
3. Trigger query execution for only that pane

### Cache Sharing

Both panes share the same resource cache. When one pane loads EC2 instances for us-east-1, the cache entry is available to the other pane. See [Resource Explorer Caching](resource-explorer-caching.md).

### Memory Budget

Memory limit enforcement applies globally across all panes. If total memory usage exceeds 80% of system RAM, queries stop in all panes. See [Memory Management System](memory-management-system.md).

## Code Example

### Creating a Split Pane View

```rust
// In ResourceExplorerWindow
pub fn show(&mut self, ctx: &Context, shared_context: &SharedContext) {
    Window::new("Resource Explorer")
        .show(ctx, |ui| {
            // Toolbar with split pane toggle
            ui.horizontal(|ui| {
                if ui.button("Split Pane").clicked() {
                    self.toggle_split_pane();
                }
            });

            // Render panes
            ui.horizontal(|ui| {
                // Left pane (always visible)
                self.render_pane(ui, &mut self.left_pane, ctx);

                // Right pane (optional)
                if let Some(ref mut right_pane) = self.right_pane {
                    ui.separator();  // 2px gray separator
                    self.render_pane(ui, right_pane, ctx);
                }
            });
        });
}
```

### Processing Pane Actions

```rust
// Route action to correct pane based on source_pane_id
match action {
    PaneAction::RemoveAccount { account_id, source_pane_id } => {
        if left_pane.id == source_pane_id {
            left_pane.state.remove_account(&account_id);
            left_pane.execute_query();
        } else if let Some(ref mut right) = right_pane {
            if right.id == source_pane_id {
                right.state.remove_account(&account_id);
                right.execute_query();
            }
        }
    }
}
```

## Testing

Test multi-pane functionality using egui_kittest:

```rust
#[test]
fn test_split_pane_independent_queries() {
    let mut harness = kittest::Harness::new_ui(|ui| {
        let mut window = ResourceExplorerWindow::new();

        // Enable split pane
        window.toggle_split_pane();

        // Configure left pane for EC2 in us-east-1
        window.left_pane.state.select_account("123456789012");
        window.left_pane.state.select_region("us-east-1");
        window.left_pane.state.select_resource_type("AWS::EC2::Instance");

        // Configure right pane for RDS in us-west-2
        if let Some(ref mut right) = window.right_pane {
            right.state.select_account("123456789012");
            right.state.select_region("us-west-2");
            right.state.select_resource_type("AWS::RDS::DBInstance");
        }

        window.show(ui.ctx(), &shared_context);
    });

    // Verify left pane shows EC2 instances
    // Verify right pane shows RDS instances
}
```

## Migration from Tab System

The previous tab-based architecture had an `ExplorerTab` layer between the window and panes. This caused:
- Complex state management with tab switching
- Difficulty tracking which tab had focus
- Lock contention when switching tabs during queries

The multi-pane architecture removes the tab layer, resulting in:
- Simpler code structure (window → pane)
- Clear focus model (left vs right pane)
- No tab switching overhead
- Better performance for split-pane workflows

## Related Documentation

- [Resource Explorer System](resource-explorer-system.md) - Query execution and two-phase loading
- [Resource Explorer Caching](resource-explorer-caching.md) - Shared cache across panes
- [Memory Management System](memory-management-system.md) - Memory budget enforcement
- [Performance Monitoring Infrastructure](performance-monitoring-infrastructure.md) - Query timing and diagnostics
