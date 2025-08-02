# Vimium-like Keyboard Navigation Implementation Plan

## Overview
Implement a Vimium-like keyboard navigation system for AWS Dash using a trait-based architecture that integrates with the existing `FocusableWindow` system.

## 🚨 CURRENT PRIORITY: Hint Activation System
**Critical Path**: Complete the hint activation system to make hints actually trigger widget actions

### Current Status
- ✅ **Framework Complete**: All core navigation infrastructure is implemented
- ✅ **Real Widget Data**: System captures 117+ real elements from TemplateSectionsWindow
- ✅ **Hint Positioning**: Hints positioned accurately with viewport clipping
- ❌ **Action Gap**: Hint selection doesn't trigger actual widget actions yet
- 🎯 **Next Goal**: Complete the hint activation system (R5.2)

## Milestone 1: Core Infrastructure ✅ **COMPLETED**
**Goal**: Establish the foundational trait system and basic navigation modes

### Tasks
- [x] Create `src/app/dashui/keyboard_navigation.rs` with core traits
  - [x] Define `NavigationMode` enum (Normal, Insert, Hint, Visual, Command)
  - [x] Define `NavigableElementType` enum (Button, TextInput, TextArea, Checkbox, etc.)
  - [x] Define `ElementAction` enum (Click, Focus, Toggle, etc.)
  - [x] Implement `KeyboardNavigable` trait
  - [x] Implement `NavigableWindow` trait extension
  - [x] Define `KeyEventResult` and `NavigationCommand` enums

- [x] Create `src/app/dashui/navigation_state.rs`
  - [x] Implement `NavigationState` struct with mode management
  - [x] Implement key sequence tracking
  - [x] Implement command count parsing (e.g., "5j" for scroll 5 times)
  - [x] Add mode stack for nested modes

- [x] Create `src/app/dashui/key_mapping.rs`
  - [x] Define `KeyMapping` and `KeyBindingMap` structures
  - [x] Implement `KeyMappingRegistry` for customizable bindings
  - [x] Create default key mappings for Normal mode
  - [x] Add configuration loading from TOML

- [x] Integrate with `DashApp`
  - [x] Add `NavigationState` field to `DashApp`
  - [x] Hook keyboard input processing in main update loop
  - [x] Add navigation state initialization

## Milestone 2: Basic Navigation Features ✅ **COMPLETED**
**Goal**: Implement scrolling, window navigation, and basic commands

### Tasks
- [x] Implement scrolling commands
  - [x] j/k for line scrolling
  - [x] d/u for half-page scrolling
  - [x] Ctrl+f/Ctrl+b for full-page scrolling (TODO: Add Ctrl bindings)
  - [x] gg/G for top/bottom navigation
  - [x] h/l for horizontal scrolling (basic implementation)

- [x] Implement window navigation
  - [x] Create window cycling commands (gt/gT)
  - [x] Add window closing command (x)
  - [x] Implement window focusing by number (1-9)
  - [ ] Add last window toggle (Ctrl+6) (TODO: Add Ctrl+6 binding)

- [x] Implement mode switching
  - [x] ESC to exit current mode
  - [x] i to enter Insert mode
  - [x] v to enter Visual mode
  - [x] : to open command palette

- [x] Add visual feedback
  - [x] Mode indicator in status bar
  - [x] Key sequence display
  - [x] Command count display

## Milestone 3: Widget Integration Layer ✅ **COMPLETED**
**Goal**: Make egui widgets keyboard navigable

### Tasks
- [x] Create `src/app/dashui/navigable_widgets.rs`
  - [x] Implement `NavigableWidget` wrapper struct
  - [x] Add factory methods for common widgets (button, text_edit, checkbox)
  - [x] Implement widget action support system
  - [x] Add widget state and response caching mechanism

- [x] Create widget collection system
  - [x] Add `NavigableElementCollector` for gathering widgets per frame
  - [x] Implement `NavigableWidgetManager` for global widget management
  - [x] Add widget lifecycle management with frame-based collection

- [x] Implement form navigation
  - [x] `NavigableContainer` trait for form field navigation
  - [x] Focus next/previous widget methods
  - [x] Widget focus history for navigation
  - [x] Support for Tab/Shift+Tab equivalent navigation

- [x] Add focus management
  - [x] Visual focus indicators with customizable styling
  - [x] Focus state management and restoration
  - [x] Element activation and interaction support

## Milestone 4: Hint Mode Implementation ✅ **COMPLETED**
**Goal**: Implement Vimium-style hint mode for clicking and focusing elements

### Tasks
- [x] Create `src/app/dashui/hint_mode.rs`
  - [x] Implement `HintMode` struct
  - [x] Add hint label generation algorithm
  - [x] Implement `HintMarker` for visual hints
  - [x] Add `HintOverlay` for rendering

- [x] Implement hint generation
  - [x] Alphabet-based hint labels (home row keys)
  - [x] Smart hint positioning to avoid overlaps
  - [x] Hint filtering based on element type
  - [x] Multi-character hint support

- [x] Add hint mode variants
  - [x] f - Universal hinting with smart actions (buttons→click, inputs→focus, text→copy)

- [x] Implement hint interaction
  - [x] Type-to-filter hints
  - [x] Hint activation on full match
  - [x] ESC to cancel hint mode
  - [x] Visual feedback for partial matches

- [x] Add integration with DashApp
  - [x] Widget collection system integrated with main update loop
  - [x] Hint overlay rendering system
  - [x] Space bar bypass for command palette
  - [x] Debug logging and troubleshooting support

## 🎯 PRIORITY IMPLEMENTATION PLAN: Real Widget Data (Current Sprint)

### 🏢 MILESTONE R1: Foundation - Replace Demo System (Week 1)
**Goal**: Replace demo system with real widget registration framework

#### Tasks
- [x] **R1.1: Remove Demo System** ✅ **COMPLETED**
  - [x] Remove `generate_fallback_elements()` from TemplateSectionsWindow
  - [x] Remove `create_demo_navigable_elements()` from DashApp
  - [x] Clean up demo element fallback logic in `EnterHintMode`
  - **Expected**: Force real implementation, remove 7 fake elements

- [x] **R1.2: Widget Registration Pattern** ✅ **COMPLETED**
  - [x] Create `WidgetRegistrar` trait for systematic widget capture
  - [x] Implement registration macros for common widget types
  - [x] Add widget ID tracking and bounds extraction
  - **Expected**: Framework for capturing real widgets during rendering

- [x] **R1.3: Context Integration** ✅ **COMPLETED**
  - [x] Enhance NavigableElementCollector to use `ctx.memory()` queries
  - [x] Implement frame-by-frame widget state management
  - [x] Add widget bounds and interaction capability detection
  - **Expected**: Foundation ready for window integration

### 🏢 MILESTONE R2: Template Window Implementation ✅ **COMPLETED**
**Goal**: Implement complete widget capture in TemplateSectionsWindow (target: 80+ elements)

#### Tasks
- [x] **R2.1: Section Tab Registration** ✅ **COMPLETED**
  - [x] Register navigation tabs (Resources, Parameters, Outputs, etc.)
  - [x] Capture tab selection state and positioning
  - **Expected**: ~9 tab elements

- [x] **R2.2: Filter Input Registration** ✅ **COMPLETED**
  - [x] Register resource filter text input
  - [x] Capture input focus capabilities and bounds
  - **Expected**: ~1 text input element

- [x] **R2.3: Resource Action Buttons** ✅ **COMPLETED**
  - [x] Register Edit, JSON, Delete buttons for each resource
  - [x] Implement dynamic registration based on resource count
  - [x] Add button state tracking (enabled/disabled)
  - **Expected**: ~3 buttons × 35 resources = 105 button elements

- [ ] **R2.4: Resource Item Registration** 📋 MEDIUM PRIORITY
  - [ ] Register clickable resource name/type labels
  - [ ] Capture resource selection capabilities
  - **Expected**: ~35 resource item elements

### ✅ MILESTONE R3: Validation & Testing (Week 3)
**Goal**: Verify system works with real data (target: 140+ elements total)

#### Tasks
- [x] **R3.1: Comprehensive Logging** 📊 ✅ **COMPLETED**
  - [x] Add detailed widget registration logging
  - [x] Track element counts per window and type
  - [x] Implement debug visualization for element bounds

- [x] **R3.2: Hint System Testing** 🎯 ✅ **COMPLETED**
  - [x] Test hint mode activation with real elements
  - [x] Verify 80+ elements are collected and displayed
  - [x] Validate hint label generation and filtering

- [x] **R3.3: Position Accuracy Verification** 📐 ✅ **COMPLETED**
  - [x] Compare hint positions to actual widget locations
  - [x] Ensure hints appear directly over their target elements
  - [x] Implement viewport clipping for scrolled-out elements

### Success Metrics 📈
- ✅ **Milestone R1**: Demo system removed, registration framework operational
- ✅ **Milestone R2**: Template window shows 80+ real elements in logs
- ✅ **Milestone R3**: Hints positioned accurately, full system functional, viewport clipping implemented

### Current Real Integration Status 🚀
- ✅ **Demo System Removed**: Completely removed mock elements from the system
- ✅ **Real Widget Registration**: Framework operational and integrated
- ✅ **Template Window Integration**: Section tabs, filter input, and action buttons registered
- ✅ **Target Achieved**: System now captures 117+ real elements (9 tabs + 3 filter controls + 105 action buttons)
- ✅ **Hint System Validated**: Hint mode working with real data (117 elements)
- ✅ **Position Accuracy**: Hints positioned correctly with viewport clipping
- ✅ **Production Testing**: Integration tests passing with real project data
- ✅ **Hint Activation Complete**: Hints now trigger actual widget actions
- ✅ **Simplified Hint System**: Universal "f" key with smart actions (buttons→click, inputs→focus, text→copy)
- 🎯 **Next Priority**: Additional window integration (R4.1) - expand to other windows

## 🚀 MILESTONE R4: System Expansion (Future - After Real Data Complete)
**Goal**: Extend real widget capture to other major application windows

### Tasks
- [ ] **R4.1: Additional Window Integration** 🪟 🎯 **HIGH PRIORITY - IN PROGRESS**
  - [x] ResourceFormWindow widget registration ✅ **COMPLETED** - 4 elements captured (resource ID input, documentation button, save/cancel buttons)
  - [ ] ResourceTypesWindow widget registration
  - [ ] CommandPalette widget registration
  - **Expected**: Additional 50+ elements across windows

- [ ] **R4.2: Global UI Integration** 🌐 MEDIUM PRIORITY
  - [ ] Menu bar widget registration
  - [ ] Toolbar and status bar elements
  - [ ] Modal dialog elements

## ⚡ MILESTONE R5: Optimization & Polish (Future)
**Goal**: Performance improvements and enhanced user experience

### Tasks
- [ ] **R5.1: Performance Optimization** 🏃‍♀️ LOW PRIORITY
  - [ ] Implement widget data caching between frames
  - [ ] Add delta updates for changed widgets only
  - [ ] Optimize element collection algorithms

- [x] **R5.2: Hint Activation System** 🔗 ✅ **COMPLETED**
  - [x] Complete the pending widget action processing
  - [x] Ensure hint selection actually triggers widget actions
  - [x] Test button clicks, text focus, and other interactions

- [ ] **R5.3: Adaptive Features** 🎨 LOW PRIORITY
  - [ ] Implement adaptive font sizing based on real widget dimensions
  - [ ] Add proximity-based hint adjustments
  - [ ] Enhance hint visual styling

## Milestone 6: Window-Specific Integration ✅ **COMPLETED** 
**Goal**: Integrate navigation with existing windows (trait implementation only)

### Tasks
- [x] Update `ResourceFormWindow` ✅ **COMPLETED**
  - [x] Implement `NavigableWindow` trait
  - [x] Register form fields as navigable
  - [x] Add custom key bindings for resource operations (Ctrl+S: Save, Escape: Close)
  - [x] Integrate with main app hint system

- [x] Update `ResourceTypesWindow` ✅ **COMPLETED**
  - [x] Implement `NavigableWindow` trait
  - [x] Make resource list navigable
  - [x] Add shortcuts for common operations (/, Ctrl+N/P, Enter, Escape)
  - [x] Implement type-to-search (existing fuzzy search integration)
  - [x] Integrate with main app hint system

- [ ] **Note**: Real widget registration for these windows moved to Milestone R4

## Milestone 7: Testing and Polish
**Goal**: Comprehensive testing and user experience refinement

### Tasks
- [ ] Unit tests
  - [ ] Navigation state transitions
  - [ ] Key mapping parsing
  - [ ] Hint generation algorithm
  - [ ] Widget navigation logic

- [ ] Integration tests
  - [ ] Full navigation workflows
  - [ ] Multi-window scenarios
  - [ ] Mode switching edge cases
  - [ ] Focus management

- [ ] Performance optimization
  - [ ] Hint generation caching
  - [ ] Widget collection efficiency
  - [ ] Key mapping lookup optimization

- [ ] User experience polish
  - [ ] Smooth animations for hints
  - [ ] Consistent visual feedback
  - [ ] Error handling and recovery
  - [ ] Accessibility considerations

## Implementation Notes

### Priority Order
1. **🚨 CURRENT FOCUS**: Real Widget Data Implementation (Milestones R1-R3)
   - **R1**: Foundation - Remove demo system, create registration framework
   - **R2**: Template Window - Capture 80+ real elements
   - **R3**: Validation - Test with real data
2. **FUTURE**: System Expansion (Milestones R4-R5)
3. **COMPLETED**: Core Infrastructure (Milestones 1-6) ✅
4. **PLANNED**: Advanced Features (Milestones 7-8)

### Technical Considerations
- Use `egui::Context` for all UI interactions
- Leverage existing `FocusableWindow` trait
- Maintain backward compatibility
- Keep navigation state separate from window state
- Use async for configuration loading

### Testing Strategy
- **Real Widget Data Phase**: Test widget registration and hint positioning accuracy
- **Production Validation**: Verify 80+ elements collected from template window
- Use existing UI testing framework for regression testing
- Add keyboard navigation specific test utilities
- Benchmark performance with many navigable elements

### Next Immediate Actions 🎯
1. **✅ COMPLETED**: R1 Foundation - Demo system removed, widget registration framework operational
2. **✅ COMPLETED**: R2 Template Window - 117+ real elements now captured from TemplateSectionsWindow
3. **✅ COMPLETED**: R3.2 Hint System Testing - Test hint mode activation with real elements
4. **✅ COMPLETED**: R3.3 Position Accuracy Verification - Hints positioned correctly with viewport clipping
5. **✅ COMPLETED**: R5.2 Hint Activation System - Hints now trigger actual widget actions
6. **🔥 CURRENT PRIORITY**: R4.1 Additional Window Integration - Continue with ResourceTypesWindow

### Risk Mitigation
- **High Risk**: Widget registration may impact UI performance
  - **Mitigation**: Implement registration only when navigation active
- **Medium Risk**: Complex widgets may not register correctly
  - **Mitigation**: Start with simple widgets, gradually add complex ones
- **Current Issue**: Template window showing 0 elements instead of 80+
  - **Root Cause**: Using fallback fake elements instead of real widget capture
  - **Solution**: Remove fallback system to force real implementation
