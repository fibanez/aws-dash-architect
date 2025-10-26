# Command Palette System

Keyboard-driven command palette providing quick access to core application functionality with context-aware command routing and authentication status awareness.

## Core Functionality

**Command Architecture:**
- **Main Command Palette**: System-level commands (Login, AWS Explorer, Control Bridge, Quit)
- Simple, focused interface with 4 primary commands
- Direct access to core features without hierarchical navigation

**Key Features:**
- Space bar global access with absolute priority over all navigation modes
- Context-aware authentication status (grayed-out commands when not logged in)
- Single-key activation for all commands with visual key indicators
- Click-outside-to-close and escape key cancellation
- Clean, minimal interface focused on essential operations

**Main Components:**
- **CommandPalette**: Main system command interface with 4 core commands (L, E, B, Q)
- **CommandRouter**: Action dispatch system with pattern matching
- **Visual Design**: Two-column layout with colored key indicators

**Integration Points:**
- Keyboard Navigation System (Space bar bypass for all modes)
- Window Focus System (FocusedWindow enum integration)
- AWS Identity Center (login status awareness)
- Resource Explorer (resource discovery and management)
- Control Bridge (AI-powered AWS operations)

## Implementation Details

**Key Files:**
- `src/app/dashui/command_palette.rs` - Main command palette with system-level commands
- `src/app/dashui/app.rs` - Command routing and palette integration

**Command Action Pattern:**
```rust
// Command palette defines 4 core actions
pub enum CommandAction {
    Login,      // L key - Login to AWS Identity Center
    AWSExplorer, // E key - Open AWS Resource Explorer
    ControlBridge, // B key - Open Control Bridge
    Quit        // Q key - Quit application
}
```

**Space Bar Priority System:**
- Space bar checked first in keyboard event handling
- Bypasses all Vimium navigation modes (Normal, Insert, Hint, Visual, Command)
- Opens main command palette regardless of current UI context
- Integrated with `NavigationCommand::OpenCommandPalette`

**Visual Design Pattern:**
- Two-column layout with calculated positioning (90% screen width, 25% height)
- Colored circular key indicators with single-letter shortcuts
- Color-coded commands by category
- Bottom-positioned overlay with consistent margin and styling
- Clean, minimal interface with 4 essential commands

**Authentication Integration:**
- Dynamic command availability based on AWS login status
- Login status checks integrated into command execution
- Visual feedback for authentication state

## Developer Notes

**Extension Points for Adding New Commands:**

1. **Add to Command Palette**:
   ```rust
   // In CommandAction enum
   pub enum CommandAction {
       Login, AWSExplorer, ControlBridge, Quit,
       NewCommand, // Add new action here
   }

   // In commands array
   let commands = [
       CommandEntry { key: egui::Key::L, label: "Login AWS", ... },
       CommandEntry { key: egui::Key::E, label: "AWS Explorer", ... },
       CommandEntry { key: egui::Key::B, label: "Control Bridge", ... },
       CommandEntry { key: egui::Key::N, label: "New Command", ... },
       CommandEntry { key: egui::Key::Q, label: "Quit", ... },
   ];
   ```

2. **Add Command Routing**:
   ```rust
   // In ui_command_palette method
   match action {
       CommandAction::NewCommand => {
           // Implement command handler
           self.show_command_palette = false;
       }
   }
   ```

**Authentication-Aware Commands:**
```rust
// Check login status for command availability
let logged_in = self.aws_identity_center.is_some();
let command_enabled = logged_in; // Control command availability
```

**Command Flow:**
1. Space Bar → Command Palette
2. Select Command → Execute action
3. Operation completion → Return to main UI

**Architectural Decisions:**
- **Space Bar Priority**: Ensures command palette is always accessible regardless of UI state
- **Minimal Interface**: Focus on 4 essential commands for clarity
- **Visual Consistency**: Unified styling with color-coded keys
- **Authentication Integration**: Commands respect login state to prevent errors
- **Direct Access**: No hierarchical navigation - immediate action execution

**Performance Considerations:**
- Palette only renders when active (state-based display)
- Command routing uses efficient pattern matching
- Minimal UI overhead with only 4 commands
- Visual updates throttled to avoid excessive redraws

**References:**
- [Keyboard Navigation System](keyboard-navigation-system.md) - Space bar priority integration
- [Window Focus System](window-focus-system.md) - FocusedWindow integration
- [Resource Explorer System](resource-explorer-system.md) - Explorer command integration