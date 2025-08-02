# Keyboard Navigation System

Vimium-style keyboard navigation system providing comprehensive keyboard-only interaction with multi-modal navigation, visual hint overlays, and element targeting throughout the application.

## Core Functionality

**Multi-Modal Navigation System:**
- **Normal Mode**: Primary navigation with movement and window commands
- **Insert Mode**: Text input mode, navigation keys pass through to text fields
- **Hint Mode**: Visual hints for clickable/focusable elements
- **Visual Mode**: Text/element selection mode
- **Command Mode**: Command palette and extended commands

**Hint System Features:**
- Visual hint overlays with home row key labels (`f`, `j`, `d`, `k`, `s`, `l`, `a`, `;`)
- Smart element targeting across all open windows
- Adaptive hint generation for any number of elements
- Element filtering and viewport-aware positioning
- Support for multiple action types (Click, Focus, Toggle, Smart, etc.)

**Key Components:**
- **NavigationState**: Global state management with key sequence processing
- **HintMode**: Visual hint overlay system with element targeting
- **NavigableWidgetManager**: Widget registration and collection system
- **HintOverlay**: Renders yellow hint labels positioned over UI elements
- **ElementActions**: Supports 9 different interaction types with smart action resolution

**Integration Points:**
- Window Focus System for application-wide navigation context
- Command Palette System (Space bar always opens palette, bypassing navigation)
- All focusable windows implement NavigableWindow trait
- egui widget system through registration macros

## Implementation Details

**Key Files:**
- `src/app/dashui/keyboard_navigation.rs` - Core traits and element definitions
- `src/app/dashui/hint_mode.rs` - Visual hint overlay system with element targeting
- `src/app/dashui/navigation_state.rs` - Global state management and key processing
- `src/app/dashui/navigable_widgets.rs` - Widget integration and registration system
- `src/app/dashui/key_mapping.rs` - Configurable key bindings and command mapping

**Navigation Commands:**
- Movement: `j/k` (vertical scroll), `h/l` (horizontal), `gg/G` (top/bottom)
- Mode switching: `i` (Insert), `v` (Visual), `:` (Command), `f` (Hint)
- Window navigation: `1-9` keys for window by index
- Element interaction: Hint mode with `f` key + home row targeting

**Element Registration Pattern:**
```rust
register_button!(widget_manager, response, "my_button", "Click Me");
register_text_input!(widget_manager, response, "my_input", "Text Input");
register_clickable!(widget_manager, response, "my_label", "Clickable Label");
```

**Configuration Requirements:**
- NavigableWidgetManager must be active in main app loop
- All windows should implement NavigableWindow trait for hint integration
- Key sequence timeout configured (default: 2 seconds)
- Debug logging controls for performance (disabled by default)

## Developer Notes

**Extension Points for New Navigation Modes:**

1. **Add New NavigationMode**:
   ```rust
   // In navigation_state.rs
   #[derive(Debug, Clone, PartialEq)]
   pub enum NavigationMode {
       Normal, Insert, Hint, Visual, Command,
       NewMode, // Add new mode here
   }
   ```

2. **Implement Mode-Specific Key Handlers**:
   ```rust
   // In NavigationState::handle_input()
   NavigationMode::NewMode => {
       match key {
           // Handle new mode key bindings
       }
   }
   ```

3. **Add Mode Display**: Update navigation status bar rendering

**Adding New Element Types:**
1. Extend `NavigableElementType` enum with new widget types
2. Define supported actions in `NavigableElement::new()`
3. Add smart action resolution logic for automatic action selection
4. Create registration macro for consistent widget integration

**Window Integration Pattern:**
- Implement `NavigableWindow` trait extending `FocusableWindow`
- Override `get_navigation_context()` for window-specific navigation settings
- Implement `collect_navigable_elements()` to provide elements for hint mode
- Register elements during UI rendering using provided macros

**Architectural Decisions:**
- **Home Row Keys**: Optimized for touch typing efficiency with `fjdk` primary keys
- **Smart Actions**: Automatically resolve appropriate action per element type
- **Frame-Based Collection**: Elements collected during UI rendering for accuracy
- **Viewport Clipping**: Only show hints for visible elements to reduce visual noise
- **Space Key Priority**: Command palette always accessible regardless of navigation mode

**Performance Considerations:**
- Element collection happens per-frame but with efficient filtering
- Debug logging disabled by default for production performance
- Hint generation uses efficient algorithms for large element sets
- Viewport bounds checking prevents off-screen hint rendering

**References:**
- [Window Focus System](window-focus-system.md) - Integration with application focus management
- [Command Palette System](command-palette-system.md) - Command mode integration
- [Trait Patterns](trait-patterns.md) - NavigableWindow trait implementation patterns