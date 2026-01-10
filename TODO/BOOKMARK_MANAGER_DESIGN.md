# Bookmark Manager Design Document
## Multi-Pane Explorer Architecture

---

## 1. REQUIREMENTS ANALYSIS

### 1.1 Functional Requirements

**FR1: Folder Hierarchy Management**
- Create folders with custom names
- Organize folders in parent/child relationships
- Move folders between parents (drag-drop or dialog)
- Rename folders
- Delete folders (with cascade to child items)
- Expand/collapse folders in tree view

**FR2: Bookmark Management**
- Edit bookmark name and description
- Delete bookmarks
- Move bookmarks between folders
- Copy/Cut/Paste bookmarks via context menu
- Drag-drop bookmarks to folders
- Reorder bookmarks within a folder

**FR3: Visual Organization**
- Hierarchical folder tree with indent levels
- Drag handles (::) for draggable items
- Visual feedback during drag operations (highlight targets)
- Top-level "Top Folder" as root container
- Expand/collapse state persistence per session

**FR4: Context Menus**
- **Folder context menu**: Paste Bookmark, Rename, Delete
- **Bookmark context menu**: Copy, Cut, Edit, Delete

**FR5: Clipboard Operations**
- Copy bookmark: Creates duplicate when pasted
- Cut bookmark: Moves bookmark when pasted
- Paste to folder: Places clipboard bookmark in target folder

**FR6: Drag-Drop Operations**
- Drag bookmarks to folders (always moves, not copies)
- Drag folders to other folders (with circular reference prevention)
- Visual drop zone highlighting
- Top Folder as drop target

### 1.2 Non-Functional Requirements

**NFR1: Usability**
- Intuitive drag-drop interface
- Clear visual hierarchy
- Responsive interactions (no UI freezing)

**NFR2: Data Integrity**
- Prevent circular folder references
- Validate folder moves
- Safe bookmark deletion with confirmation

**NFR3: State Management**
- Persist folder expansion state during session
- Sync clipboard state across operations
- Handle concurrent access to bookmark data

---

## 2. CURRENT vs OLD SYSTEM COMPARISON

### 2.1 Current System (As of latest commit)
```
ExplorerInstance
├── Simple bookmark manager dialog
│   ├── Flat list of bookmarks
│   ├── Basic delete button
│   └── No folders, no drag-drop
└── State in panes (show_bookmark_manager flag)
```

### 2.2 Old System (Commit 3683a4d)
```
ResourceExplorerWindow
├── Full bookmark manager dialog
│   ├── Folder hierarchy with drag-drop
│   ├── Context menus (Copy/Cut/Paste/Edit/Delete)
│   ├── Folder management dialog
│   ├── Bookmark edit dialog
│   └── Clipboard operations
└── State in window (all dialog flags)
```

### 2.3 Target System (New Multi-Pane)
```
ExplorerInstance
├── Full bookmark manager (restored features)
│   ├── All old system features
│   ├── Adapted for multi-pane architecture
│   └── Shared across left/right panes
└── State in instance (dialog flags + clipboard)
```

---

## 3. ARCHITECTURE DESIGN

### 3.1 Data Structures

```rust
// Already exists in codebase
struct Bookmark {
    id: String,
    name: String,
    description: Option<String>,
    folder_id: Option<String>,  // Parent folder
    account_ids: Vec<String>,
    region_codes: Vec<String>,
    resource_type_ids: Vec<String>,
    // ... other fields
}

struct BookmarkFolder {
    id: String,
    name: String,
    parent_id: Option<String>,  // Parent folder
    created_at: DateTime<Utc>,
    modified_at: DateTime<Utc>,
}

// Drag-drop payload
enum DragData {
    Bookmark {
        id: String,
        source_folder: Option<String>,
    },
    Folder {
        id: String,
        parent_id: Option<String>,
    },
}
```

### 3.2 Instance State (Fields to Add)

```rust
struct ExplorerInstance {
    // ... existing fields ...

    // Bookmark edit dialog
    show_bookmark_edit_dialog: bool,
    editing_bookmark_id: Option<String>,
    bookmark_edit_name: String,
    bookmark_edit_description: String,

    // Folder management
    show_folder_dialog: bool,
    folder_dialog_name: String,
    folder_dialog_parent_id: Option<String>,
    editing_folder_id: Option<String>,
    expanded_folders: HashSet<String>,

    // Clipboard
    bookmark_clipboard: Option<String>,
    bookmark_clipboard_is_cut: bool,
}
```

### 3.3 Component Architecture

```
┌──────────────────────────────────────┐
│   ExplorerInstance::render()         │
│   (FocusableWindow impl)             │
└────────────┬─────────────────────────┘
             │
             ├─> render_bookmark_manager_dialog()
             │   ├─> render_folder_tree_level() [RECURSIVE]
             │   │   ├─> Render folders with drag handles
             │   │   ├─> Render bookmarks with drag handles
             │   │   └─> Handle context menus
             │   │
             │   ├─> Handle folder operations
             │   │   ├─> Delete folder
             │   │   ├─> Rename folder
             │   │   └─> Move folder
             │   │
             │   └─> Handle bookmark operations
             │       ├─> Delete bookmark
             │       ├─> Edit bookmark
             │       ├─> Move/Copy bookmark
             │       └─> Clipboard operations
             │
             ├─> render_folder_dialog()
             │   ├─> Create new folder
             │   ├─> Edit existing folder
             │   └─> Select parent folder
             │
             └─> render_bookmark_edit_dialog()
                 ├─> Edit name
                 └─> Edit description
```

---

## 4. EGUI INTEGRATION POINTS

### 4.1 Required egui Features

**Widgets:**
- `egui::Window` - Dialog windows
- `egui::CollapsingHeader` - Folder expand/collapse
- `egui::ScrollArea` - Scrollable tree view
- `egui::ComboBox` - Parent folder selection
- `egui::Label` with `Sense::click()` - Clickable labels
- `egui::Button` - Action buttons

**Drag-Drop API:**
- `ui.dnd_drag_source()` - Make elements draggable
- `response.dnd_hover_payload()` - Detect drag-over
- `response.dnd_release_payload()` - Handle drop
- `ui.painter().rect_stroke()` - Visual feedback

**Context Menus:**
- `response.context_menu()` - Right-click menus

**ID System:**
- `ui.id().with("suffix")` - Unique widget IDs

### 4.2 Third-Party Dependencies

**egui_dnd** (if not already included):
```toml
egui_dnd = "0.7"  # For list reordering within folders
```

**Note**: Check if `egui_dnd::dnd()` is available, otherwise use manual drag-drop with egui's built-in DnD.

---

## 5. IMPLEMENTATION PLAN

### Phase 1: Foundation (State & Data Structures)
**Task 1.1**: Add fields to `ExplorerInstance` ✓ (Already attempted)
**Task 1.2**: Add `DragData` enum
**Task 1.3**: Add import for `egui::Window`

### Phase 2: Bookmark Edit Dialog (Simplest Dialog)
**Task 2.1**: Implement `render_bookmark_edit_dialog()`
- Show dialog when `show_bookmark_edit_dialog == true`
- Text inputs for name and description
- Save button updates bookmark via `shared_context.bookmarks`
- Cancel button closes dialog

### Phase 3: Folder Management Dialog
**Task 3.1**: Implement `render_folder_dialog()`
- Create new folder or edit existing
- ComboBox for parent folder selection
- Prevent circular references (can't be parent of itself)

### Phase 4: Bookmark Manager Main Dialog (Core)
**Task 4.1**: Implement basic bookmark manager structure
- Window with stats (bookmark count, folder count)
- "New Folder" button
- ScrollArea for tree view

**Task 4.2**: Implement `render_folder_tree_level()` - Read-only first
- Recursive folder rendering
- CollapsingHeader for folders
- List bookmarks in each folder
- No drag-drop yet

**Task 4.3**: Add context menus
- Folder: Rename, Delete, Paste Bookmark
- Bookmark: Copy, Cut, Edit, Delete

**Task 4.4**: Implement clipboard operations
- Copy/Cut bookmark to clipboard
- Paste bookmark to folder
- Move vs Copy logic

### Phase 5: Drag-Drop (Most Complex)
**Task 5.1**: Add drag handles for folders
- Drag source with `DragData::Folder`
- Visual feedback on hover

**Task 5.2**: Add drag handles for bookmarks
- Drag source with `DragData::Bookmark`
- Track source folder

**Task 5.3**: Implement drop targets
- Folders accept bookmarks and subfolders
- "Top Folder" accepts everything
- Prevent circular folder moves

**Task 5.4**: Add visual drop zone highlighting
- Stroke rectangle on valid drop targets

### Phase 6: Bookmark Reordering
**Task 6.1**: Use egui_dnd for bookmark reordering within folders
- Allow drag-to-reorder bookmarks in same folder
- Update bookmark order in BookmarkManager

### Phase 7: State Synchronization
**Task 7.1**: Sync dialog states with pane states
- Check `show_bookmark_manager` from both panes
- Close dialog updates both panes

**Task 7.2**: Persist expanded folder state
- Track in `expanded_folders: HashSet<String>`

---

## 6. PSEUDOCODE

### 6.1 Main Dialog Structure

```rust
fn render_bookmark_manager_dialog(&mut self, ctx: &Context, shared_context: &ExplorerSharedContext) {
    // Check if any pane wants to show the manager
    let show = check_pane_states_for_show_bookmark_manager();
    if !show { return; }

    // State for pending operations
    let mut bookmark_to_delete = None;
    let mut bookmark_to_edit = None;
    let mut folder_to_delete = None;
    let mut folder_to_rename = None;
    let mut move_bookmark_to_folder = None;
    let mut is_drag_drop_move = false;

    // Main dialog window
    Window::new("Manage Bookmarks")
        .show(ctx, |ui| {
            // Stats row
            show_bookmark_and_folder_counts();

            // Toolbar
            if "New Folder" button clicked {
                open_folder_dialog();
            }

            // Tree view in ScrollArea
            ScrollArea::vertical().show(|ui| {
                // Top Folder drop zone
                render_top_folder_drop_zone();

                // Recursive tree from root (parent_id = None)
                render_folder_tree_level(
                    parent_id: None,
                    &mut bookmark_to_delete,
                    &mut bookmark_to_edit,
                    // ... other out params
                );
            });
        });

    // Process pending operations (after dialog closes)
    if let Some(id) = bookmark_to_delete {
        delete_bookmark(id);
    }
    if let Some(id) = bookmark_to_edit {
        open_edit_dialog(id);
    }
    // ... handle other operations

    // Sync close state with panes
    if dialog_closed {
        close_in_both_panes();
    }
}
```

### 6.2 Recursive Folder Tree

```rust
fn render_folder_tree_level(
    &mut self,
    ui: &mut Ui,
    parent_id: Option<String>,
    shared_context: &ExplorerSharedContext,
    bookmark_to_delete: &mut Option<String>,
    bookmark_to_edit: &mut Option<String>,
    folder_to_delete: &mut Option<String>,
    folder_to_rename: &mut Option<String>,
    move_bookmark_to_folder: &mut Option<(String, Option<String>)>,
    is_drag_drop_move: &mut bool,
) {
    // Get folders at this level
    let folders = get_subfolders(parent_id);

    // Render each folder
    for folder in folders {
        // Drag handle for folder
        let drag_id = ui.id().with("folder_drag").with(&folder.id);
        ui.dnd_drag_source(drag_id, DragData::Folder { id, parent_id }, |ui| {
            ui.label(":: ");  // Drag handle
        });

        // Collapsing header for folder
        CollapsingHeader::new(&folder.name)
            .show(ui, |ui| {
                // RECURSE: Render subfolders and bookmarks
                render_folder_tree_level(
                    parent_id: Some(folder.id),
                    // ... pass through out params
                );
            });

        // Context menu for folder
        response.context_menu(|ui| {
            if "Paste Bookmark" clicked {
                *move_bookmark_to_folder = Some((clipboard_id, Some(folder.id)));
            }
            if "Rename" clicked {
                *folder_to_rename = Some(folder.id);
            }
            if "Delete" clicked {
                *folder_to_delete = Some(folder.id);
            }
        });

        // Handle drop on folder
        if let Some(dropped) = response.dnd_release_payload::<DragData>() {
            match dropped {
                DragData::Bookmark { id, source_folder } => {
                    if source_folder != Some(folder.id) {
                        *move_bookmark_to_folder = Some((id, Some(folder.id)));
                        *is_drag_drop_move = true;
                    }
                }
                DragData::Folder { id, parent_id } => {
                    if !is_circular_reference(folder.id, id) {
                        move_folder_to_parent(id, Some(folder.id));
                    }
                }
            }
        }
    }

    // Get bookmarks at this level
    let bookmarks = get_bookmarks_in_folder(parent_id);

    // Use egui_dnd for reordering within folder
    dnd(ui, "bookmarks").show_vec(&mut bookmarks, |ui, bookmark, handle| {
        // Drag handle
        handle.ui(|ui| { ui.label(":: "); });

        // Bookmark info
        ui.label(&bookmark.name);
        ui.label(&bookmark.description);
        ui.label(format!("{} accounts, {} regions", ...));

        // Context menu
        response.context_menu(|ui| {
            if "Copy" clicked {
                self.bookmark_clipboard = Some(bookmark.id);
                self.bookmark_clipboard_is_cut = false;
            }
            if "Cut" clicked {
                self.bookmark_clipboard = Some(bookmark.id);
                self.bookmark_clipboard_is_cut = true;
            }
            if "Edit" clicked {
                *bookmark_to_edit = Some(bookmark.id);
            }
            if "Delete" clicked {
                *bookmark_to_delete = Some(bookmark.id);
            }
        });
    });
}
```

### 6.3 Folder Dialog

```rust
fn render_folder_dialog(&mut self, ctx: &Context, shared_context: &ExplorerSharedContext) {
    if !self.show_folder_dialog { return; }

    let is_editing = self.editing_folder_id.is_some();
    let title = if is_editing { "Edit Folder" } else { "New Folder" };

    Window::new(title).show(ctx, |ui| {
        // Name input
        ui.text_edit_singleline(&mut self.folder_dialog_name);

        // Parent folder dropdown
        ComboBox::from_label("Parent folder")
            .show_ui(|ui| {
                if select "Top Folder" {
                    self.folder_dialog_parent_id = None;
                }
                for folder in all_folders {
                    // Don't allow selecting self as parent
                    if folder.id != self.editing_folder_id {
                        if select folder {
                            self.folder_dialog_parent_id = Some(folder.id);
                        }
                    }
                }
            });

        // Buttons
        if "Create/Update" clicked {
            if is_editing {
                update_folder(editing_folder_id, name, parent_id);
            } else {
                create_folder(name, parent_id);
            }
            close_dialog();
        }
        if "Cancel" clicked {
            close_dialog();
        }
    });
}
```

### 6.4 Bookmark Edit Dialog

```rust
fn render_bookmark_edit_dialog(&mut self, ctx: &Context, shared_context: &ExplorerSharedContext) {
    if !self.show_bookmark_edit_dialog { return; }

    Window::new("Edit Bookmark").show(ctx, |ui| {
        // Name input
        ui.label("Name:");
        ui.text_edit_singleline(&mut self.bookmark_edit_name);

        // Description input
        ui.label("Description:");
        ui.text_edit_singleline(&mut self.bookmark_edit_description);

        // Buttons
        if "Save" clicked {
            let bookmark = get_bookmark_mut(self.editing_bookmark_id);
            bookmark.name = self.bookmark_edit_name.clone();
            bookmark.description = if empty { None } else { Some(...) };
            bookmark.modified_at = now();
            save_bookmarks();
            close_dialog();
        }
        if "Cancel" clicked {
            close_dialog();
        }
    });
}
```

---

## 7. EDGE CASES & VALIDATION

### 7.1 Circular Reference Prevention
```
Before moving folder A to folder B:
  - Check if B is a descendant of A
  - Use recursive function: is_descendant(A, B)
  - If true, reject move
```

### 7.2 Same-Folder Drops
```
When dropping bookmark:
  - Check if source_folder == target_folder
  - If same, ignore drop (no-op)
```

### 7.3 Folder Deletion
```
When deleting folder:
  - Move child bookmarks to Top Folder (or delete?)
  - Move child folders to Top Folder
  - Update expanded_folders set
```

### 7.4 Clipboard State
```
After paste:
  - Clear clipboard if Cut operation
  - Keep clipboard if Copy operation
```

---

## 8. TESTING STRATEGY

### Manual Testing Checklist

**Folder Operations:**
- [ ] Create folder at top level
- [ ] Create subfolder
- [ ] Rename folder
- [ ] Delete empty folder
- [ ] Delete folder with bookmarks
- [ ] Move folder via drag-drop
- [ ] Prevent circular folder reference

**Bookmark Operations:**
- [ ] Edit bookmark name and description
- [ ] Delete bookmark
- [ ] Copy bookmark to clipboard
- [ ] Cut bookmark to clipboard
- [ ] Paste bookmark to folder
- [ ] Drag bookmark to folder
- [ ] Reorder bookmarks in same folder

**UI Interactions:**
- [ ] Expand/collapse folders persists during session
- [ ] Context menus work on right-click
- [ ] Visual feedback during drag operations
- [ ] Dialog closes sync with pane states
- [ ] Drag handles are clickable

---

## 9. IMPLEMENTATION PRIORITY

**Priority 1 (Essential):**
1. Bookmark Edit Dialog - Simplest, high value
2. Folder Management Dialog - Required for hierarchy
3. Bookmark Manager Main Structure - Core UI

**Priority 2 (Important):**
4. Folder Tree Rendering (read-only) - Visual hierarchy
5. Context Menus - User convenience
6. Clipboard Operations - Power user feature

**Priority 3 (Nice to Have):**
7. Drag-Drop for bookmarks - Intuitive but can use context menu instead
8. Drag-Drop for folders - Same as above
9. Bookmark Reordering - Polish feature

---

## 10. MIGRATION NOTES

**From Old System to New:**
- Old: `self.bookmark_manager` → New: `shared_context.bookmarks`
- Old: Window-level state → New: Instance-level state
- Old: Single window → New: Shared across panes

**Key Differences:**
- New system must check both panes for `show_bookmark_manager` flag
- New system must close dialog in both panes
- New system has access to `ExplorerSharedContext` instead of direct bookmark manager

---

## 11. NEXT STEPS

1. **Review this document** - Ensure requirements match expectations
2. **Start with Phase 1** - Add fields to ExplorerInstance
3. **Implement Phase 2** - Bookmark Edit Dialog (quick win)
4. **Iterate through phases** - Build incrementally, test each phase
5. **Document as we go** - Add comments explaining how each part works

---

END OF DESIGN DOCUMENT
