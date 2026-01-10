# AWS Explorer UI Map - Original Implementation Reference

**Purpose**: Complete reference map of the original AWS Explorer UI layout from `window.rs` to guide the multi-pane architecture implementation.

**Source**: `/src/app/resource_explorer/window.rs` (lines 313-2057+)

---

## Window Structure (egui::Window)

```rust
Window::new("AWS Explorer")
    .title_bar(false)           // Custom title bar
    .default_size([800.0, height]) // height = screen - 60 - 40
    .min_size([600.0, 400.0])
    .resizable(true)
    .movable(true)
    .collapsible(true)
    .constrain(true)
```

---

## Layout Hierarchy

```
Window
├── Custom Title Bar (horizontal)
│   ├── Heading "AWS Explorer"
│   └── Right-to-left layout:
│       ├── X button (Close/Terminate)
│       └── _ button (Minimize)
├── Separator
├── TopBottomPanel::bottom("explorer_status_bar")
│   └── Status Bar (see below)
├── SidePanel::left("explorer_sidebar")
│   └── Sidebar Controls (see below)
└── CentralPanel::default()
    ├── Unified Toolbar (horizontal, see below)
    ├── Separator
    ├── Active Selection Tags (render_active_tags)
    ├── Space (10.0)
    ├── Search Bar (render_search_bar)
    ├── Separator
    └── Tree View (render_tree_view in ScrollArea)
```

---

## 1. Custom Title Bar

**Layout**: `ui.horizontal()` → `ui.with_layout(egui::Layout::right_to_left())`

**Location**: Lines 426-441

**Components**:
- **Heading**: `ui.heading("AWS Explorer")` (left-aligned)
- **X Button** (rightmost):
  - Text: "X"
  - Tooltip: "Close window"
  - Action: `close_clicked = true` → `WindowAction::Terminate`
- **Minimize Button** (next to X):
  - Text: "_"
  - Tooltip: "Minimize - Hide window (keep state)"
  - Action: `minimize_requested = true` → `WindowAction::Minimize`

---

## 2. Status Bar (Bottom Panel)

**Panel Type**: `egui::TopBottomPanel::bottom("explorer_status_bar")`

**Location**: Lines 448-837

**Configuration**:
- `show_separator_line(true)`
- `show_inside(ui, |ui| { ... })`

**Layout**: `ui.horizontal()` with two sections:

### Left Section (Scrollable)
**Wrapped in**: `egui::ScrollArea::horizontal().scroll_bar_visibility(AlwaysHidden)`

**Content** (priority order):
1. **Phase 1 Progress** (if loading):
   - Spinner + text: "Phase 1: Loading resources... ({pending}/{total})"
   - Color: `Color32::from_rgb(100, 180, 255)` (light blue)
   - Font: `.small()`
   - Shows failed count if any

2. **Phase 2 Progress** (if phase2_enrichment_in_progress):
   - Spinner + text: "Phase 2: Enriching details... ({pending}/{total})"
   - Color: `Color32::from_rgb(255, 180, 100)` (light orange)
   - Font: `.small()`

3. **Active Operation Indicator** (if global_status active):
   - Pulsing "*" character: `pulse = ((time * 3.0).sin() * 0.3 + 0.7)`
   - Color: `Color32::from_rgba_unmultiplied(100, 180, 255, pulse)`
   - Text color: `Color32::from_rgb(100, 180, 255)`
   - Font: `.small()`

4. **Ready State**:
   - Text: "Ready"
   - Color: `Color32::GRAY`
   - Font: `.small()`

5. **Failed Queries Indicator** (persistent, after other status):
   - Text: "[{count} queries failed]"
   - Color: `Color32::from_rgb(255, 150, 50)` (orange)
   - Font: `.small()`
   - Clickable: Opens service availability dialog
   - Tooltip: Multi-line explanation

### Right Section (right_to_left layout)
**Content**: Memory and cache stats

- **Format**: "{physical_mb}MB | {cache_mb}MB cache ({ratio}x) | {active} active, {queries} queries"
- **Font**: `.small().color(Color32::GRAY)`
- **Example**: "407MB | 0.1MB cache (86.2x) | 300 active, 13 queries"

---

## 3. Left Sidebar

**Panel Type**: `egui::SidePanel::left("explorer_sidebar")`

**Location**: Lines 840-869

**Configuration**:
- `default_width(180.0)`
- `min_width(150.0)`
- `resizable(true)`
- `show_inside(ui, |ui| { ... })`

**Rendering**: Delegates to `PaneRenderer::render_sidebar(ui, &mut state)`

**Components** (from pane_renderer.rs lines 186-416):

### 3.1 Group By Section
- **Label**: "Group by:" (plain text)
- **Spacing**: `ui.add_space(4.0)`
- **Dropdown**: `egui::ComboBox::from_label("")`
  - Shows `state.primary_grouping.display_name()`
  - Three sections:
    1. **Built-in** (weak, small header):
       - Account
       - Region
       - Resource Type
       - None
    2. **Tag Groupings** (dynamic, weak/small header):
       - Format: "Tag: {key} ({resource_count} resources, {value_count} values)"
       - Limited to 20 tags
       - Filtered by `min_tag_resources_for_grouping`
       - Tooltip shows value preview
    3. **Advanced** (weak/small header):
       - Button: "Tag Hierarchy..."
       - Button: "Property Hierarchy..."

### 3.2 Min Resources Control
- **Spacing**: `ui.add_space(8.0)`
- **Label**: "Min res:"
- **Control**: `egui::DragValue`
  - Range: `1..=100`
  - Speed: `1.0`
  - Tooltip: "Minimum number of resources for tags to appear in GroupBy dropdown. Drag to adjust or click to type."

### 3.3 Tag Presence Filters
- **Separator** + **Spacing**: `ui.add_space(8.0)`
- **Checkbox 1**: "Show only tagged"
  - Tooltip: "Show only resources with any tags"
  - Mutually exclusive with untagged
- **Checkbox 2**: "Show only untagged"
  - Tooltip: "Show only resources with no tags"
  - Mutually exclusive with tagged

### 3.4 Filter Buttons
- **Spacing**: `ui.add_space(8.0)`
- **Button 1**: "Tag Filters..."
  - Tooltip: "Open advanced tag filter builder"
  - Action: `PaneAction::ShowTagFilterBuilder`
  - If active: Shows badge with count (red text on yellow background)
    - Format: "(1 filter active)" or "({n} filters active)"
    - Colors: text=`rgb(200,40,40)`, bg=`rgb(255,255,200)`
    - Font: `.strong()`
    - Frame: `corner_radius(3.0)`, `inner_margin(6,2)`
- **Spacing**: `ui.add_space(4.0)`
- **Button 2**: "Property Filters..."
  - Tooltip: "Open property filter builder"
  - Action: `PaneAction::ShowPropertyFilterBuilder`
  - If active: Shows badge (same styling as Tag Filters)
- **Spacing**: `ui.add_space(4.0)`
- **Button 3**: "Clear Filters" (only if filters active)
  - Tooltip: "Clear all tag and property filters"
  - Action: Clears all filters in state

---

## 4. Unified Toolbar (Central Panel Top)

**Location**: Lines 1808-1942 (`render_unified_toolbar`)

**Layout**: `ui.horizontal()`

**Components** (left to right):

### 4.1 Bookmarks Menu Button
- **Button**: `ui.menu_button("Bookmarks", |ui| { ... })`
- **Content**:
  - Recursive bookmark/folder hierarchy
  - Active bookmark highlighted: `[Active] {name}` with selection background
  - Tooltips show: name, description, counts, grouping, usage stats
  - Separator before management actions
  - Bottom actions:
    - "Add Bookmark"
    - "Manage Bookmarks"

### 4.2 Separator
- **Widget**: `ui.separator()`

### 4.3 Select Button
- **Button**: `ui.button("Select")`
- **Action**: `state.show_unified_selection_dialog = true`

### 4.4 Dropdown Menu ("v")
- **Button**: `ui.menu_button("v", |ui| { ... })`
- **Content**:
  - "Add Account" → `state.show_account_dialog = true`
  - "Add Region" → `state.show_region_dialog = true`
  - "Add Resource" → `state.show_resource_type_dialog = true`

### 4.5 Separator
- **Widget**: `ui.separator()`

### 4.6 Refresh Button
- **Button**: `ui.button("Refresh")`
- **Action**: `state.show_refresh_dialog = true`

### 4.7 Reset Button
- **Button**: `ui.button("Reset")`
- **Tooltip**: "Reset all selections to default state"
- **Action**: `clear_clicked = true` → calls `state.clear_all_selections()`

### 4.8 Verify with CLI Button (DEBUG only)
- **Conditional**: `#[cfg(debug_assertions)]`
- **Separator** before button
- **Button**: `ui.button("Verify with CLI")`
- **Tooltip**: "Compare cached resources with AWS CLI output"
- **Action**: `verification_window.open = true`

### 4.9 Cache Menu Button
- **Separator** before button
- **Button**: `ui.menu_button("Cache", |ui| { ... })`
- **Content**:
  - Label: "Resource queries: {count}"
  - Label: "Detailed entries: {count}"
  - Separator
  - Label: "Compressed: {mb:.1} MB"
  - Label: "Uncompressed: {mb:.1} MB"
  - Label: "Compression: {ratio:.1}x"
  - Separator
  - Button: "Clear Cache" → clears shared cache

### 4.10 Loading Indicator (conditional)
- **Conditional**: `if state.is_loading()`
- **Separator** before indicator
- **Widgets**:
  - `ui.spinner()`
  - Label: "Loading... ({count} queries)"

---

## 5. Active Selection Tags

**Location**: Lines 495-549+ (pane_renderer.rs `render_active_tags`)

**Layout**: `ui.horizontal()` then `ui.horizontal_wrapped()`

**Components**:

### 5.1 Header Row
- **Label**: "Selection:" (small font)
- **Expand/Collapse Button** (if > 5 tags):
  - Text: `[-]` if expanded, `[+{count}]` if collapsed
  - `ui.small_button()`
  - Toggles `state.active_selection_expanded`

### 5.2 Tag Display (wrapped)
- **Layout**: `ui.horizontal_wrapped()` with `set_max_width(ui.available_width())`
- **Limit**: Show 5 tags when collapsed, all when expanded
- **Order**: Accounts → Regions → Resource Types

**Tag Rendering** (closeable colored tags):
- **Accounts**: Yellow background (`Color32::from_rgb(255, 220, 100)`)
  - Format: "Account: {display_name} ({account_id}) x"
  - Tooltip: Account details
  - X button removes account
- **Regions**: Light green background (`Color32::from_rgb(144, 238, 144)`)
  - Format: "Region: {name} ({code}) x"
  - Tooltip: Region details
  - X button removes region
- **Resource Types**: Light blue background (`Color32::from_rgb(173, 216, 230)`)
  - Format: "{resource_type} x"
  - Tooltip: Resource type
  - X button removes resource type

**Tag Style** (consistent across all types):
```rust
egui::Frame::new()
    .fill(background_color)  // Yellow/Green/Blue
    .inner_margin(egui::Margin::symmetric(6, 2))
    .corner_radius(3.0)
    .show(ui, |ui| {
        ui.label(egui::RichText::new(label_text).small().strong());
        if ui.small_button("x").clicked() {
            // Remove action
        }
    });
ui.add_space(2.0);  // Between tags
```

---

## 6. Search Bar

**Location**: Lines 173-181 (pane_renderer.rs `render_search_bar`)

**Layout**: `ui.horizontal()`

**Components**:
- **Label**: "Search:"
- **Text Edit**: `ui.text_edit_singleline(&mut state.search_filter)`
- **Button**: "Clear"
  - Action: `state.search_filter.clear()`

---

## 7. Tree View

**Location**: Lines 418-492 (pane_renderer.rs `render_tree_view`)

**Container**: `egui::ScrollArea::vertical().auto_shrink([false, false])`

**States**:

### 7.1 Empty Selection
```
"Select accounts, regions, and resource types to begin exploring"
```
- **Layout**: `ui.centered_and_justified()`
- **Condition**: `state.query_scope.is_empty()`

### 7.2 No Resources Found
```
"No resources found for the current selection"
```
- **Layout**: `ui.centered_and_justified()`
- **Condition**: `state.resources.is_empty() && !state.is_loading()`

### 7.3 No Matches After Filtering
```
"No resources match the active tag filters"
```
- **Layout**: `ui.centered_and_justified()`
- **Condition**: `filtered_resources.is_empty()` (after tag/property filters applied)

### 7.4 Loading State
```
[Spinner] "Loading resources..."
```
- **Layout**: `ui.centered_and_justified()`
- **Widgets**: `ui.spinner()` + label
- **Condition**: `state.is_loading()`

### 7.5 Resource Tree Display
**Pre-tree Header** (if filters active):
```
"Showing {filtered} of {total} resources ({count} filter[s])"
```
- **Layout**: `ui.horizontal()`
- **Separator** after

**Tree Rendering**:
- Delegates to `tree_renderer.render_tree_cached()`
- Parameters:
  - `&filtered_resources` (after tag + property filters)
  - `primary_grouping` (clone)
  - `&search_filter`
  - `&badge_selector`
  - `&tag_popularity`
  - `enrichment_version`

---

## Dialogs (Modal Windows)

**Note**: Dialogs are rendered OUTSIDE the main window, in separate egui::Window calls. They are controlled by flags in window.rs (show_bookmark_dialog, show_unified_selection_dialog, etc.) and state.rs (show_account_dialog, show_region_dialog, etc.).

### Dialog List (from window.rs struct fields):
1. **Bookmark Creation**: `show_bookmark_dialog`
2. **Bookmark Manager**: `show_bookmark_manager`
3. **Bookmark Edit**: `show_bookmark_edit_dialog`
4. **Folder Creation/Edit**: `show_folder_dialog`
5. **Unified Selection** (Account/Region/Resource): `state.show_unified_selection_dialog`
6. **Account Selection**: `state.show_account_dialog`
7. **Region Selection**: `state.show_region_dialog`
8. **Resource Type Selection**: `state.show_resource_type_dialog`
9. **Refresh Dialog**: `show_refresh_dialog`
10. **Tag Filter Builder**: `show_filter_builder`
11. **Tag Hierarchy Builder**: `show_hierarchy_builder`
12. **Property Filter Builder**: `show_property_filter_builder`
13. **Property Hierarchy Builder**: `show_property_hierarchy_builder`
14. **Service Availability Dialog**: `show_service_availability_dialog`
15. **Verification Window** (DEBUG): `verification_window.open`

**Handled by**: `FuzzySearchDialog` (for selection dialogs), `TagFilterBuilderWidget`, etc.

---

## Colors Used

| Element | Color | RGB | Usage |
|---------|-------|-----|-------|
| Account Tag BG | Yellow | `(255, 220, 100)` | Account closeable tags |
| Region Tag BG | Light Green | `(144, 238, 144)` | Region closeable tags |
| Resource Tag BG | Light Blue | `(173, 216, 230)` | Resource type tags |
| Active Status | Light Blue | `(100, 180, 255)` | Phase 1/active operations |
| Phase 2 Status | Light Orange | `(255, 180, 100)` | Phase 2 enrichment |
| Failed Queries | Orange | `(255, 150, 50)` | Failed query indicator |
| Filter Badge Text | Dark Red | `(200, 40, 40)` | Active filter count |
| Filter Badge BG | Light Yellow | `(255, 255, 200)` | Active filter background |
| Status Ready | Gray | `GRAY` | Ready state text |
| Memory Stats | Gray | `GRAY` | Status bar right section |

---

## Fonts/Text Styling

| Element | Style | Code |
|---------|-------|------|
| Window Title | Heading | `ui.heading("AWS Explorer")` |
| Status Bar | Small | `egui::RichText::new(text).small()` |
| Status Active | Small + Strong | `.small().strong()` |
| Selection Tags | Small + Strong | `.small().strong()` |
| Tag Count Badge | Strong | `.strong()` |
| Sidebar Labels | Plain | Default font |
| Dropdown Headers | Small + Weak | `.small().weak()` |
| Empty Messages | Italics + Weak | `.italics().weak()` |

---

## Spacing Constants

| Location | Spacing | Code |
|----------|---------|------|
| After Group By label | 4.0 | `ui.add_space(4.0)` |
| Before Min Resources | 8.0 | `ui.add_space(8.0)` |
| Before Tag Checkboxes | 8.0 (after separator) | `ui.separator(); ui.add_space(8.0)` |
| Before Filter Buttons | 8.0 | `ui.add_space(8.0)` |
| Between Filter Buttons | 4.0 | `ui.add_space(4.0)` |
| Between Tags | 2.0 | `ui.add_space(2.0)` |
| Before Search Bar | 10.0 | `ui.add_space(10.0)` |

---

## Widget Interactions

### Clickable Elements:
1. **Title Bar Buttons**: X (terminate), _ (minimize)
2. **Toolbar Buttons**: Bookmarks, Select, v dropdown, Refresh, Reset, Verify CLI, Cache
3. **Tag X Buttons**: Remove account/region/resource type
4. **Expand/Collapse Button**: Show/hide extra selection tags
5. **Filter Buttons**: Tag Filters, Property Filters, Clear Filters
6. **Dropdown Items**: Group by modes, bookmarks
7. **Failed Queries Indicator**: Opens service availability dialog
8. **Search Clear Button**: Clears search filter

### Tooltips:
- All buttons have tooltips (via `.on_hover_text()`)
- Tag labels have detailed tooltips (via `.on_hover_ui()`)
- Bookmarks show full details on hover
- Grouping options show value previews on hover

---

## TODO for Multi-Pane Implementation

Reference this map when implementing each pane to ensure:
1. ✅ **Sidebar**: Matches exact layout (Group By, Min Res, Checkboxes, Filter Buttons)
2. ❌ **Toolbar**: Add Bookmarks button, Select button, v dropdown, Refresh, Reset, Cache menu
3. ❌ **Active Tags**: Render colored, closeable tags with expand/collapse
4. ✅ **Search Bar**: Already implemented
5. ✅ **Tree View**: Already implemented via PaneRenderer
6. ❌ **Dialogs**: Add FuzzySearchDialog to each pane for unified selection
7. ❌ **Status Indicators**: Wire up loading spinners in toolbar

**Key Missing Components in Current Pane Implementation**:
- Toolbar buttons (Bookmarks, Select, Refresh, Reset, Cache)
- FuzzySearchDialog integration for selection UI
- Active selection tag rendering with colored backgrounds
- Expand/collapse functionality for tags
- Bookmark system per-pane
- Refresh dialog
- Cache menu in toolbar

---

**Created**: 2026-01-04
**Reference Commit**: Multi-Pane-Explorer worktree (current)
**Source File**: `src/app/resource_explorer/window.rs`
**Implementation Target**: `src/app/resource_explorer/instances/pane.rs` + `pane_renderer.rs`
