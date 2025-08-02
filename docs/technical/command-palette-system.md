# Command Palette System

Multi-context command palette system providing keyboard-driven access to application functionality through hierarchical command organization with fuzzy search and context-aware command routing.

## Core Functionality

**Multi-Context Architecture:**
- **Main Command Palette**: System-level commands (Search, Login, AWS Explorer, Graph View, Quit)
- **Project Command Palette**: Project management operations (New, Open, Edit project)
- **CloudFormation Command Palette**: CloudFormation-specific operations (Deploy, Import, Validate, Add Resource, Edit Sections)

**Key Features:**
- Space bar global access with absolute priority over all navigation modes
- Fuzzy search integration for file and resource selection
- Context-aware authentication status (grayed-out commands when not logged in)
- Hierarchical command flow from main palette to specialized sub-palettes
- Single-key activation for all commands with visual key indicators
- Click-outside-to-close and escape key cancellation

**Main Components:**
- **CommandPalette**: Main system command interface with 7 core commands
- **ProjectCommandPalette**: Project lifecycle management (New/Open/Edit)
- **CloudFormationCommandPalette**: CloudFormation workflow commands (Deploy/Import/Validate)
- **FuzzyFilePicker**: File selection with fuzzy matching and directory navigation
- **CommandRouter**: Action dispatch system with pattern matching

**Integration Points:**
- Keyboard Navigation System (Space bar bypass for all modes)
- Window Focus System (FocusedWindow enum integration)
- AWS Identity Center (login status awareness)
- Project Management System (project operations)
- CloudFormation Manager (template and deployment operations)

## Implementation Details

**Key Files:**
- `src/app/dashui/command_palette.rs` - Main command palette with system-level commands
- `src/app/dashui/project_command_palette.rs` - Project management command interface
- `src/app/dashui/cloudformation_command_palette.rs` - CloudFormation operations palette
- `src/app/dashui/app.rs` - Command routing and palette integration

**Command Action Pattern:**
```rust
// Each palette defines its own action enum
pub enum CommandAction { Search, Login, Project, CloudFormation, GraphView, AWSExplorer, Quit }
pub enum ProjectCommandAction { NewProject, OpenProject, EditProject }
pub enum CloudFormationCommandAction { AddResource, Deploy, Import, Validate, EditSections }
```

**Space Bar Priority System:**
- Space bar checked first in keyboard event handling
- Bypasses all Vimium navigation modes (Normal, Insert, Hint, Visual, Command)
- Opens main command palette regardless of current UI context
- Integrated with `NavigationCommand::OpenCommandPalette`

**Visual Design Pattern:**
- Two-column layout with calculated positioning (90% screen width, 25% height)
- Colored circular key indicators with single-letter shortcuts
- Color-coded commands by category (Green=Create, Blue=Import, Orange=Auth)
- Bottom-positioned overlay with consistent margin and styling
- Responsive sizing with scroll support for longer command lists

**Authentication Integration:**
- Dynamic command availability based on AWS login status
- Grayed-out CloudFormation commands when not authenticated
- Login status checks integrated into command execution

## Developer Notes

**Extension Points for Adding New Commands:**

1. **Add to Existing Palette**:
   ```rust
   // In CommandAction enum
   pub enum CommandAction {
       Search, Login, Project, CloudFormation, GraphView, AWSExplorer, Quit,
       NewCommand, // Add new action here
   }
   
   // In commands array
   commands.push(CommandEntry {
       key: "N",
       label: "New Command",
       color: Color32::from_rgb(70, 130, 180),
       description: "Description of new command",
       action: CommandAction::NewCommand,
   });
   ```

2. **Create New Sub-Palette**:
   ```rust
   // Define new palette state enum
   pub enum NewPaletteState { Closed, CommandPalette }
   
   // Implement palette display method
   pub fn show_new_palette(ui: &mut Ui, state: &mut NewPaletteState) {
       // Follow existing palette pattern
   }
   
   // Add to main app integration
   // Add FocusedWindow enum variant
   ```

3. **Add Command Routing**:
   ```rust
   // In ui_command_palette method
   match action {
       CommandAction::NewCommand => {
           // Implement command handler
           self.show_command_palette = false;
       }
   }
   ```

**File Selection Integration Pattern:**
- Use `FuzzyFilePicker` for file-based operations
- Support directory navigation with Ctrl+Y and Left Arrow keys
- Implement file type filtering (JSON/YAML for CloudFormation)
- Provide visual feedback for directory vs. file selection

**Authentication-Aware Commands:**
```rust
// Check login status for command availability
let logged_in = self.aws_identity_center.is_some();
let command_color = if logged_in {
    Color32::from_rgb(70, 130, 180)
} else {
    Color32::GRAY // Grayed out when not authenticated
};
```

**Hierarchical Command Flow:**
1. Space Bar → Main Command Palette
2. Main Palette Action → Context-specific sub-palette
3. Sub-palette Action → Specialized dialogs/operations
4. Operation completion → Return to previous context

**Architectural Decisions:**
- **Space Bar Priority**: Ensures command palette is always accessible regardless of UI state
- **Context Separation**: Each palette handles its domain-specific commands
- **Visual Consistency**: Unified styling across all palette types
- **Authentication Integration**: Commands respect login state to prevent errors
- **Fuzzy Search**: Provides efficient file selection without native dialogs

**Performance Considerations:**
- Palettes only render when active (state-based display)
- Command routing uses efficient pattern matching
- File picker uses incremental filtering for large directories
- Visual updates throttled to avoid excessive redraws

**References:**
- [Keyboard Navigation System](keyboard-navigation-system.md) - Space bar priority integration
- [Window Focus System](window-focus-system.md) - FocusedWindow integration
- [Project Management](project-management.md) - Project command operations
- [CloudFormation Manager](cloudformation-manager.md) - CloudFormation command integration