//! Keyboard Navigation System Core Traits
//!
//! This module provides the foundational trait system for Vimium-like keyboard navigation
//! throughout the AWS Dash application. It integrates with the existing `FocusableWindow`
//! trait system to provide consistent keyboard-driven interaction patterns.

use eframe::egui;
use std::collections::HashMap;

/// Navigation modes supported by the keyboard navigation system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NavigationMode {
    /// Normal mode - primary navigation mode with movement and window commands
    #[default]
    Normal,
    /// Insert mode - text input mode, navigation keys pass through to text fields
    Insert,
    /// Hint mode - display hints for clickable/focusable elements
    Hint,
    /// Visual mode - text/element selection mode
    Visual,
    /// Command mode - command palette and extended commands
    Command,
}

/// Types of navigable elements in the UI
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NavigableElementType {
    /// Clickable button
    Button,
    /// Text input field
    TextInput,
    /// Multi-line text area
    TextArea,
    /// Checkbox or toggle
    Checkbox,
    /// Radio button
    RadioButton,
    /// Dropdown/combo box
    ComboBox,
    /// Slider control
    Slider,
    /// Clickable link or label
    Link,
    /// Selectable list item
    ListItem,
    /// Tree node that can be expanded/collapsed
    TreeNode,
    /// Tab button
    Tab,
    /// Menu item
    MenuItem,
    /// Custom interactive element
    Custom(String),
}

/// Actions that can be performed on navigable elements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementAction {
    /// Click the element (buttons, links, etc.)
    Click,
    /// Focus the element for text input
    Focus,
    /// Toggle the element state (checkboxes, radio buttons)
    Toggle,
    /// Open/expand the element (dropdowns, tree nodes)
    Open,
    /// Close/collapse the element
    Close,
    /// Copy element text to clipboard
    Copy,
    /// Select element text
    Select,
    /// Activate element (generic activation)
    Activate,
    /// Smart action based on element type (universal hinting)
    Smart,
}

/// Result of processing a keyboard event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEventResult {
    /// Event was handled by navigation system
    Handled,
    /// Event should be passed through to underlying widget
    PassThrough,
    /// Event triggered a mode change
    ModeChanged(NavigationMode),
    /// Event triggered a navigation command
    Command(NavigationCommand),
    /// Event should close current mode/operation
    Cancel,
}

/// Navigation commands that can be triggered by keyboard input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationCommand {
    /// Scroll vertically (positive = down, negative = up)
    ScrollVertical(i32),
    /// Scroll horizontally (positive = right, negative = left)
    ScrollHorizontal(i32),
    /// Navigate to next window
    NextWindow,
    /// Navigate to previous window
    PreviousWindow,
    /// Close current window
    CloseWindow,
    /// Navigate to window by index (1-9)
    WindowByIndex(u8),
    /// Toggle to last active window
    LastWindow,
    /// Move to top of current view
    MoveToTop,
    /// Move to bottom of current view
    MoveToBottom,
    /// Open command palette
    OpenCommandPalette,
    /// Enter hint mode with specific action
    EnterHintMode(ElementAction),
    /// Navigate to next element
    NextElement,
    /// Navigate to previous element
    PreviousElement,
    /// Activate current element
    ActivateElement,
    /// Focus search/filter field
    FocusSearchField,
}

/// Core trait for keyboard navigation capabilities
pub trait KeyboardNavigable {
    /// Process a keyboard event in the current navigation context
    ///
    /// # Parameters
    /// - `key_event`: The keyboard input to process
    /// - `mode`: Current navigation mode
    /// - `modifiers`: Active keyboard modifiers
    ///
    /// # Returns
    /// Result indicating how the event was handled
    fn handle_key_event(
        &mut self,
        key_event: &egui::Event,
        mode: NavigationMode,
        modifiers: &egui::Modifiers,
    ) -> KeyEventResult;

    /// Get navigable elements in the current context
    ///
    /// Returns a collection of elements that can be navigated to or interacted with
    /// in the current UI state. Elements should be ordered by their logical
    /// navigation sequence (e.g., tab order).
    fn get_navigable_elements(&self) -> Vec<NavigableElement>;

    /// Check if navigation is currently enabled for this component
    ///
    /// Allows components to temporarily disable navigation during certain operations
    /// (e.g., during drag and drop, modal dialogs, etc.)
    fn is_navigation_enabled(&self) -> bool {
        true
    }

    /// Get the currently focused element index
    ///
    /// Returns the index of the currently focused element in the navigable elements list,
    /// or None if no element is focused.
    fn get_focused_element_index(&self) -> Option<usize> {
        None
    }

    /// Set focus to element at given index
    ///
    /// # Parameters
    /// - `index`: Index of element to focus in the navigable elements list
    ///
    /// # Returns
    /// true if focus was successfully set, false otherwise
    fn set_focused_element(&mut self, _index: usize) -> bool {
        false
    }
}

/// Extended trait for windows that support keyboard navigation
pub trait NavigableWindow: super::window_focus::FocusableWindow {
    /// Get the navigation context for this window
    ///
    /// Provides window-specific navigation configuration and state
    fn get_navigation_context(&self) -> NavigationContext;

    /// Handle navigation command specific to this window
    ///
    /// Windows can override default command handling for window-specific behavior
    fn handle_navigation_command(&mut self, _command: NavigationCommand) -> KeyEventResult {
        // Default implementation returns PassThrough to use global handlers
        KeyEventResult::PassThrough
    }

    /// Get window-specific key bindings
    ///
    /// Allows windows to define custom key mappings that override global bindings
    fn get_custom_key_bindings(&self) -> HashMap<String, NavigationCommand> {
        HashMap::new()
    }

    /// Called when navigation mode changes within this window
    ///
    /// Allows windows to react to mode changes (e.g., show/hide mode indicators)
    fn on_navigation_mode_changed(&mut self, _old_mode: NavigationMode, _new_mode: NavigationMode) {
        // Default implementation does nothing
    }
}

/// Represents a navigable element in the UI
#[derive(Debug, Clone)]
pub struct NavigableElement {
    /// Unique identifier for this element within its container
    pub id: String,
    /// Type of element (button, text input, etc.)
    pub element_type: NavigableElementType,
    /// Screen rectangle occupied by this element
    pub rect: egui::Rect,
    /// Whether this element is currently visible and interactable
    pub enabled: bool,
    /// Optional label or description for accessibility
    pub label: Option<String>,
    /// Actions that can be performed on this element
    pub supported_actions: Vec<ElementAction>,
    /// Custom metadata for element-specific behavior
    pub metadata: HashMap<String, String>,
}

impl NavigableElement {
    /// Create a new navigable element
    pub fn new(id: String, element_type: NavigableElementType, rect: egui::Rect) -> Self {
        let mut supported_actions = match element_type {
            NavigableElementType::Button => vec![ElementAction::Click, ElementAction::Activate],
            NavigableElementType::TextInput | NavigableElementType::TextArea => {
                vec![
                    ElementAction::Focus,
                    ElementAction::Select,
                    ElementAction::Copy,
                ]
            }
            NavigableElementType::Checkbox => {
                vec![
                    ElementAction::Toggle,
                    ElementAction::Activate,
                    ElementAction::Click,
                ]
            }
            NavigableElementType::RadioButton => {
                vec![ElementAction::Activate, ElementAction::Click]
            }
            NavigableElementType::ComboBox => {
                vec![
                    ElementAction::Open,
                    ElementAction::Focus,
                    ElementAction::Activate,
                ]
            }
            NavigableElementType::Slider => {
                vec![ElementAction::Focus, ElementAction::Activate]
            }
            NavigableElementType::Link => {
                vec![
                    ElementAction::Click,
                    ElementAction::Copy,
                    ElementAction::Activate,
                ]
            }
            NavigableElementType::ListItem => {
                vec![
                    ElementAction::Click,
                    ElementAction::Select,
                    ElementAction::Activate,
                ]
            }
            NavigableElementType::TreeNode => {
                vec![
                    ElementAction::Open,
                    ElementAction::Close,
                    ElementAction::Activate,
                ]
            }
            NavigableElementType::Tab => {
                vec![ElementAction::Click, ElementAction::Activate]
            }
            NavigableElementType::MenuItem => {
                vec![ElementAction::Click, ElementAction::Activate]
            }
            NavigableElementType::Custom(_) => {
                vec![ElementAction::Activate]
            }
        };

        // All elements support smart action for universal hinting
        supported_actions.push(ElementAction::Smart);

        Self {
            id,
            element_type,
            rect,
            enabled: true,
            label: None,
            supported_actions,
            metadata: HashMap::new(),
        }
    }

    /// Set the label for this element
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    /// Set whether this element is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add metadata to this element
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add custom supported actions
    pub fn with_actions(mut self, actions: Vec<ElementAction>) -> Self {
        self.supported_actions = actions;
        self
    }

    /// Check if this element supports a specific action
    pub fn supports_action(&self, action: &ElementAction) -> bool {
        self.supported_actions.contains(action)
    }

    /// Get the smart action for this element type (used with universal hinting)
    pub fn get_smart_action(&self) -> ElementAction {
        match self.element_type {
            NavigableElementType::Button => ElementAction::Click,
            NavigableElementType::TextInput | NavigableElementType::TextArea => {
                ElementAction::Focus
            }
            NavigableElementType::Checkbox => ElementAction::Toggle,
            NavigableElementType::RadioButton => ElementAction::Click,
            NavigableElementType::ComboBox => ElementAction::Open,
            NavigableElementType::Slider => ElementAction::Focus,
            NavigableElementType::Link => ElementAction::Click,
            NavigableElementType::ListItem => ElementAction::Click,
            NavigableElementType::TreeNode => ElementAction::Open,
            NavigableElementType::Tab => ElementAction::Click,
            NavigableElementType::MenuItem => ElementAction::Click,
            NavigableElementType::Custom(_) => ElementAction::Activate,
        }
    }

    /// Get the center point of this element for hint positioning
    pub fn center(&self) -> egui::Pos2 {
        self.rect.center()
    }

    /// Check if a point is within this element's bounds
    pub fn contains_point(&self, point: egui::Pos2) -> bool {
        self.rect.contains(point)
    }
}

/// Navigation context for a window or component
#[derive(Debug, Clone)]
pub struct NavigationContext {
    /// Whether this context supports hint mode
    pub supports_hints: bool,
    /// Whether this context supports visual selection
    pub supports_visual_mode: bool,
    /// Whether this context handles scrolling
    pub handles_scrolling: bool,
    /// Custom navigation settings
    pub settings: HashMap<String, String>,
}

impl Default for NavigationContext {
    fn default() -> Self {
        Self {
            supports_hints: true,
            supports_visual_mode: false,
            handles_scrolling: true,
            settings: HashMap::new(),
        }
    }
}

impl NavigationContext {
    /// Create a new navigation context with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a navigation context for a simple window
    pub fn simple() -> Self {
        Self {
            supports_hints: true,
            supports_visual_mode: false,
            handles_scrolling: false,
            settings: HashMap::new(),
        }
    }

    /// Create a navigation context for a form window
    pub fn form() -> Self {
        Self {
            supports_hints: true,
            supports_visual_mode: true,
            handles_scrolling: true,
            settings: HashMap::new(),
        }
    }

    /// Create a navigation context for a graph/visualization window
    pub fn graph() -> Self {
        Self {
            supports_hints: true,
            supports_visual_mode: false,
            handles_scrolling: true,
            settings: HashMap::new(),
        }
    }

    /// Add a custom setting to this context
    pub fn with_setting(mut self, key: String, value: String) -> Self {
        self.settings.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_mode_default() {
        assert_eq!(NavigationMode::default(), NavigationMode::Normal);
    }

    #[test]
    fn test_navigable_element_creation() {
        let rect =
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::Vec2::new(100.0, 30.0));
        let element = NavigableElement::new(
            "test_button".to_string(),
            NavigableElementType::Button,
            rect,
        );

        assert_eq!(element.id, "test_button");
        assert_eq!(element.element_type, NavigableElementType::Button);
        assert_eq!(element.rect, rect);
        assert!(element.enabled);
        assert!(element.supports_action(&ElementAction::Click));
        assert!(element.supports_action(&ElementAction::Activate));
        assert!(!element.supports_action(&ElementAction::Focus));
    }

    #[test]
    fn test_navigable_element_with_label() {
        let rect =
            egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::Vec2::new(100.0, 30.0));
        let element = NavigableElement::new(
            "test_input".to_string(),
            NavigableElementType::TextInput,
            rect,
        )
        .with_label("Name Field".to_string());

        assert_eq!(element.label, Some("Name Field".to_string()));
        assert!(element.supports_action(&ElementAction::Focus));
        assert!(element.supports_action(&ElementAction::Select));
    }

    #[test]
    fn test_navigable_element_center() {
        let rect =
            egui::Rect::from_min_size(egui::Pos2::new(10.0, 20.0), egui::Vec2::new(100.0, 50.0));
        let element = NavigableElement::new("test".to_string(), NavigableElementType::Button, rect);

        let center = element.center();
        assert_eq!(center, egui::Pos2::new(60.0, 45.0)); // 10 + 100/2, 20 + 50/2
    }

    #[test]
    fn test_navigable_element_contains_point() {
        let rect =
            egui::Rect::from_min_size(egui::Pos2::new(10.0, 20.0), egui::Vec2::new(100.0, 50.0));
        let element = NavigableElement::new("test".to_string(), NavigableElementType::Button, rect);

        assert!(element.contains_point(egui::Pos2::new(50.0, 40.0))); // Inside
        assert!(element.contains_point(egui::Pos2::new(10.0, 20.0))); // Top-left corner
        assert!(element.contains_point(egui::Pos2::new(110.0, 70.0))); // Bottom-right corner
        assert!(!element.contains_point(egui::Pos2::new(5.0, 40.0))); // Left of rect
        assert!(!element.contains_point(egui::Pos2::new(50.0, 15.0))); // Above rect
    }

    #[test]
    fn test_navigation_context_default() {
        let context = NavigationContext::default();
        assert!(context.supports_hints);
        assert!(!context.supports_visual_mode);
        assert!(context.handles_scrolling);
        assert!(context.settings.is_empty());
    }

    #[test]
    fn test_navigation_context_simple() {
        let context = NavigationContext::simple();
        assert!(context.supports_hints);
        assert!(!context.supports_visual_mode);
        assert!(!context.handles_scrolling);
    }

    #[test]
    fn test_navigation_context_form() {
        let context = NavigationContext::form();
        assert!(context.supports_hints);
        assert!(context.supports_visual_mode);
        assert!(context.handles_scrolling);
    }

    #[test]
    fn test_element_action_support() {
        // Test button actions
        let button_element = NavigableElement::new(
            "btn".to_string(),
            NavigableElementType::Button,
            egui::Rect::ZERO,
        );
        assert!(button_element.supports_action(&ElementAction::Click));
        assert!(!button_element.supports_action(&ElementAction::Focus));

        // Test text input actions
        let text_element = NavigableElement::new(
            "txt".to_string(),
            NavigableElementType::TextInput,
            egui::Rect::ZERO,
        );
        assert!(text_element.supports_action(&ElementAction::Focus));
        assert!(!text_element.supports_action(&ElementAction::Click));

        // Test checkbox actions
        let checkbox_element = NavigableElement::new(
            "chk".to_string(),
            NavigableElementType::Checkbox,
            egui::Rect::ZERO,
        );
        assert!(checkbox_element.supports_action(&ElementAction::Toggle));
        assert!(checkbox_element.supports_action(&ElementAction::Click));
    }

    #[test]
    fn test_element_with_metadata() {
        let element = NavigableElement::new(
            "test".to_string(),
            NavigableElementType::Button,
            egui::Rect::ZERO,
        )
        .with_metadata("tooltip".to_string(), "Click me".to_string())
        .with_metadata("group".to_string(), "toolbar".to_string());

        assert_eq!(
            element.metadata.get("tooltip"),
            Some(&"Click me".to_string())
        );
        assert_eq!(element.metadata.get("group"), Some(&"toolbar".to_string()));
        assert_eq!(element.metadata.get("nonexistent"), None);
    }

    #[test]
    fn test_element_with_custom_actions() {
        let custom_actions = vec![ElementAction::Copy, ElementAction::Select];
        let element = NavigableElement::new(
            "test".to_string(),
            NavigableElementType::Custom("special".to_string()),
            egui::Rect::ZERO,
        )
        .with_actions(custom_actions);

        assert!(element.supports_action(&ElementAction::Copy));
        assert!(element.supports_action(&ElementAction::Select));
        assert!(!element.supports_action(&ElementAction::Click));
    }
}
