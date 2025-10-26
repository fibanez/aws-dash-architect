# File Picker System

Custom file selection system replacing native dialogs with fuzzy search capabilities, directory navigation, and specialized pickers for different file types with folder creation functionality.

## Core Functionality

**File Picker Types:**
- **FuzzyFilePicker**: General-purpose directory and file selection with Project.json auto-detection
- **CloudFormationFilePicker**: Specialized for CloudFormation template selection (JSON/YAML files)

**Key Features:**
- Fuzzy search filtering with real-time matching as user types
- Directory-first organization with alphabetical sorting within each category
- Keyboard navigation with arrow keys and Enter/Escape handling
- Parent directory navigation with Left Arrow key
- Folder creation with Ctrl+N keyboard shortcut
- Visual distinction between directories (📁) and files
- Error handling with user-friendly error messages
- Auto-focus on search field for immediate typing

**Navigation Controls:**
- **Type to filter**: Real-time fuzzy matching of file/directory names
- **Ctrl+Y**: Navigate into selected directory
- **Left Arrow (←)**: Go up one level in directory hierarchy  
- **Enter**: Accept current selection (directory or file)
- **Escape**: Cancel and close picker
- **Ctrl+N**: Create new folder with dialog
- **Arrow Keys**: Navigate through filtered results

**Main Components:**
- **FuzzyFilePicker**: Core picker with directory navigation and folder creation
- **CloudFormationFilePicker**: Template-specific picker with JSON/YAML filtering
- **FilePickerStatus**: State management enum (Open/Closed/Selected)
- **FuzzyMatcher**: Search algorithm integration for relevance ranking

**Integration Points:**
- Command Palette System for file selection operations
- Project Management System for Project.json discovery
- CloudFormation Manager for template import operations
- Application data directory navigation

## Implementation Details

**Key Files:**
- `src/app/dashui/fuzzy_file_picker.rs` - General-purpose file picker with folder creation
- `src/app/dashui/cloudformation_file_picker.rs` - CloudFormation template specific picker
- `src/app/dashui/app.rs` - Fuzzy matching algorithm (`fuzzy_match_score`)

**Status Management Pattern:**
```rust
pub enum FuzzyFilePickerStatus {
    Open,                    // Picker is active and waiting for input
    Closed,                  // Picker was cancelled or closed
    Selected(PathBuf),       // Path was selected by user
}
```

**Directory Processing Logic:**
- Hidden files/directories (starting with '.') are automatically filtered out
- Directories are sorted and displayed first, followed by files
- Both categories use case-insensitive alphabetical sorting
- Real-time filtering applied based on fuzzy search query

**Project.json Auto-Detection:**
- When navigating into directories, automatically checks for Project.json files
- If Project.json found, immediately selects that directory
- Enables project discovery workflow without manual file selection

**Fuzzy Search Algorithm:**
- Uses `fuzzy_match_score()` function for relevance ranking
- Supports partial matches and character-order-independent searching
- Empty query displays all non-hidden files and directories
- Real-time filtering updates as user types

**Window Layout:**
- 60% of screen width and height for optimal usability
- Centered on screen with proper spacing
- Search field auto-focused on display
- Scrollable results area for large directories

## Developer Notes

**Extension Points for Custom File Types:**

1. **Create Specialized Picker**:
   ```rust
   pub struct CustomFilePicker {
       pub status: CustomFilePickerStatus,
       current_dir: PathBuf,
       query: String,
       // Add file type specific filtering
   }
   ```

2. **Implement File Type Filtering**:
   ```rust
   // In update_entries method
   if let Some(extension) = path.extension() {
       match extension.to_str() {
           Some("json") | Some("yaml") | Some("yml") => {
               // Include file
           }
           _ => continue, // Skip non-matching files
       }
   }
   ```

3. **Add Custom Navigation Behaviors**:
   ```rust
   // Override accept_selection for specialized behaviors
   fn accept_selection(&mut self) {
       // Custom logic for file type handling
   }
   ```

**Integration Pattern for New Use Cases:**
- Follow the status enum pattern for state management
- Implement show() method for UI rendering with consistent styling
- Add keyboard event handling for navigation shortcuts
- Integrate with main application loop through status polling

**Folder Creation Workflow:**
```rust
// Triggered by Ctrl+N
self.show_new_folder_dialog = true;

// Dialog collects folder name
if ui.button("Create").clicked() {
    let new_folder_path = self.current_dir.join(&self.new_folder_name);
    std::fs::create_dir_all(&new_folder_path)?;
    self.update_entries();
}
```

**Error Handling Strategy:**
- Directory access errors display user-friendly messages
- Permission errors handled gracefully without crashing
- Invalid directory paths trigger fallback to parent directory
- File system errors logged and displayed in picker UI

**Performance Optimizations:**
- Directory listings cached until navigation changes
- Fuzzy search algorithm optimized for real-time filtering
- Hidden file filtering applied during directory read
- Sorting performed once per directory change

**Visual Design Patterns:**
- Consistent with application's overall UI theme
- Directory and file icons for visual distinction
- Selected item highlighting with keyboard navigation
- Error messages displayed within picker context
- Modal overlay behavior with click-outside-to-close

**Architectural Decisions:**
- **No Native Dialogs**: Custom implementation provides consistent UX across platforms
- **Fuzzy Search**: Enables efficient navigation in large directories
- **Keyboard First**: Optimized for keyboard-driven workflows
- **File Type Awareness**: Specialized pickers filter relevant files
- **Auto-Detection**: Smart behaviors reduce manual selection steps

**References:**
- [Command Palette System](command-palette-system.md) - File picker integration
- [Keyboard Navigation System](keyboard-navigation-system.md) - Keyboard shortcut integration
- [UI Testing Framework](ui-testing-framework.md) - Testing file picker interactions