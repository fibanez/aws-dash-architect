//! Navigable Widget Integration
//!
//! This module provides wrapper types and utilities to make egui widgets keyboard navigable.
//! It bridges the gap between egui's immediate mode GUI paradigm and the persistent navigation
//! state required for Vimium-like keyboard navigation.
//!
//! ## Debug Logging
//!
//! Internal debug logging for widget registration and collection is **disabled by default**
//! for performance. To enable detailed logging for debugging purposes, use:
//!
//! ```rust
//! widget_manager.set_debug_logging(true);  // Enable verbose logging
//! widget_manager.set_debug_logging(false); // Disable logging (default)
//! ```

use super::keyboard_navigation::{ElementAction, NavigableElement, NavigableElementType};
use eframe::egui;
use std::collections::HashMap;

/// Macro for registering a button widget with the navigation system
///
/// # Usage
/// ```rust
/// let response = ui.button("Click Me");
/// register_button!(widget_manager, response, "my_button", "Click Me");
/// ```
#[macro_export]
macro_rules! register_button {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_button(&$response, $id.to_string(), None)
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_button(&$response, $id.to_string(), Some($label.to_string()))
    };
}

/// Macro for registering a text input widget with the navigation system
///
/// # Usage
/// ```rust
/// let response = ui.text_edit_singleline(&mut text);
/// register_text_input!(widget_manager, response, "my_input", "Text Input");
/// ```
#[macro_export]
macro_rules! register_text_input {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_text_input(&$response, $id.to_string(), None)
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_text_input(&$response, $id.to_string(), Some($label.to_string()))
    };
}

/// Macro for registering a clickable element (labels, links, etc.) with the navigation system
///
/// # Usage
/// ```rust
/// let response = ui.label("Clickable Label").interact(egui::Sense::click());
/// register_clickable!(widget_manager, response, "my_label", "Clickable Label");
/// ```
#[macro_export]
macro_rules! register_clickable {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_clickable(&$response, $id.to_string(), None)
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_clickable(&$response, $id.to_string(), Some($label.to_string()))
    };
}

/// Macro for registering a list item widget with the navigation system
///
/// # Usage
/// ```rust
/// let response = ui.selectable_label(selected, "List Item");
/// register_list_item!(widget_manager, response, "my_item", "List Item");
/// ```
#[macro_export]
macro_rules! register_list_item {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_list_item(&$response, $id.to_string(), None)
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_list_item(&$response, $id.to_string(), Some($label.to_string()))
    };
}

/// Macro for registering a text area widget with the navigation system
///
/// # Usage
/// ```rust
/// let response = ui.text_edit_multiline(&mut text);
/// register_text_area!(widget_manager, response, "my_textarea", "Description");
/// ```
#[macro_export]
macro_rules! register_text_area {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_text_area(&$response, $id.to_string(), None)
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_text_area(&$response, $id.to_string(), Some($label.to_string()))
    };
}

/// Macro for registering a checkbox widget with the navigation system
///
/// # Usage
/// ```rust
/// let response = ui.checkbox(&mut value, "Enable feature");
/// register_checkbox!(widget_manager, response, "my_checkbox", "Enable feature");
/// ```
#[macro_export]
macro_rules! register_checkbox {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_checkbox(&$response, $id.to_string(), None)
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_checkbox(&$response, $id.to_string(), Some($label.to_string()))
    };
}

/// Macro for registering a combo box widget with the navigation system
///
/// # Usage
/// ```rust
/// let response = egui::ComboBox::from_label("Select option").show_ui(ui, |ui| { /* content */ });
/// register_combo_box!(widget_manager, response.response, "my_combo", "Select option");
/// ```
#[macro_export]
macro_rules! register_combo_box {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_combo_box(&$response, $id.to_string(), None)
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_combo_box(&$response, $id.to_string(), Some($label.to_string()))
    };
}

/// Macro for registering a slider widget with the navigation system
///
/// # Usage
/// ```rust
/// let response = ui.add(egui::Slider::new(&mut value, 0.0..=100.0));
/// register_slider!(widget_manager, response, "my_slider", "Volume");
/// ```
#[macro_export]
macro_rules! register_slider {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_custom_widget(
            &$response,
            $id.to_string(),
            NavigableElementType::Slider,
            vec![ElementAction::Activate, ElementAction::Focus],
            None,
            std::collections::HashMap::new(),
        )
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_custom_widget(
            &$response,
            $id.to_string(),
            NavigableElementType::Slider,
            vec![ElementAction::Activate, ElementAction::Focus],
            Some($label.to_string()),
            std::collections::HashMap::new(),
        )
    };
}

/// Macro for registering a radio button widget with the navigation system
///
/// # Usage
/// ```rust
/// let response = ui.radio_value(&mut selected, value, "Option");
/// register_radio_button!(widget_manager, response, "my_radio", "Option");
/// ```
#[macro_export]
macro_rules! register_radio_button {
    ($registrar:expr, $response:expr, $id:expr) => {
        $registrar.register_custom_widget(
            &$response,
            $id.to_string(),
            NavigableElementType::RadioButton,
            vec![
                ElementAction::Activate,
                ElementAction::Click,
                ElementAction::Smart,
            ],
            None,
            std::collections::HashMap::new(),
        )
    };
    ($registrar:expr, $response:expr, $id:expr, $label:expr) => {
        $registrar.register_custom_widget(
            &$response,
            $id.to_string(),
            NavigableElementType::RadioButton,
            vec![
                ElementAction::Activate,
                ElementAction::Click,
                ElementAction::Smart,
            ],
            Some($label.to_string()),
            std::collections::HashMap::new(),
        )
    };
}

/// Wrapper around egui widgets to make them keyboard navigable
#[derive(Debug)]
pub struct NavigableWidget {
    /// Unique identifier for this widget within its container
    pub id: String,
    /// Type of the underlying widget
    pub widget_type: NavigableElementType,
    /// Current widget state
    pub state: WidgetState,
    /// Screen rectangle occupied by this widget
    pub rect: Option<egui::Rect>,
    /// Whether this widget is currently enabled/interactable
    pub enabled: bool,
    /// Optional label for accessibility and hints
    pub label: Option<String>,
    /// Whether this widget currently has focus
    pub focused: bool,
    /// Custom metadata for widget-specific behavior
    pub metadata: HashMap<String, String>,
}

/// State information for navigable widgets
#[derive(Debug, Clone, Default)]
pub struct WidgetState {
    /// Whether the widget was interacted with this frame
    pub interacted: bool,
    /// The response from the last frame (if any)
    pub last_response: Option<WidgetResponse>,
    /// Whether the widget should be activated on next frame
    pub pending_activation: bool,
    /// Focus-related state
    pub focus_state: FocusState,
}

/// Simplified widget response for navigation purposes
#[derive(Debug, Clone)]
pub struct WidgetResponse {
    /// Whether the widget was clicked
    pub clicked: bool,
    /// Whether the widget gained focus
    pub gained_focus: bool,
    /// Whether the widget lost focus
    pub lost_focus: bool,
    /// Whether the widget's value changed
    pub changed: bool,
    /// Rectangle occupied by the widget
    pub rect: egui::Rect,
}

/// Focus state for navigable widgets
#[derive(Debug, Clone, Default)]
pub struct FocusState {
    /// Whether this widget should be focused
    pub should_focus: bool,
    /// Whether this widget had focus in the last frame
    pub had_focus: bool,
    /// Visual focus style to apply
    pub focus_style: FocusStyle,
}

/// Visual styling for focused widgets
#[derive(Debug, Clone)]
pub struct FocusStyle {
    /// Border color for focused widgets
    pub border_color: egui::Color32,
    /// Border width for focused widgets
    pub border_width: f32,
    /// Background highlight color
    pub highlight_color: Option<egui::Color32>,
}

impl Default for FocusStyle {
    fn default() -> Self {
        Self {
            border_color: egui::Color32::from_rgb(100, 150, 255),
            border_width: 2.0,
            highlight_color: Some(egui::Color32::from_rgba_unmultiplied(100, 150, 255, 30)),
        }
    }
}

impl NavigableWidget {
    /// Create a new navigable widget wrapper
    pub fn new(id: String, widget_type: NavigableElementType) -> Self {
        Self {
            id,
            widget_type,
            state: WidgetState::default(),
            rect: None,
            enabled: true,
            label: None,
            focused: false,
            metadata: HashMap::new(),
        }
    }

    /// Set the label for this widget
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    /// Set whether this widget is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add metadata to this widget
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set focus state for this widget
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.state.focus_state.should_focus = focused;
    }

    /// Check if this widget supports a specific action
    pub fn supports_action(&self, action: &ElementAction) -> bool {
        // All widgets support Smart action for universal hinting
        if matches!(action, ElementAction::Smart) {
            return true;
        }

        match self.widget_type {
            NavigableElementType::Button => {
                matches!(action, ElementAction::Click | ElementAction::Activate)
            }
            NavigableElementType::TextInput | NavigableElementType::TextArea => {
                matches!(
                    action,
                    ElementAction::Focus | ElementAction::Select | ElementAction::Copy
                )
            }
            NavigableElementType::Checkbox => {
                matches!(
                    action,
                    ElementAction::Toggle | ElementAction::Activate | ElementAction::Click
                )
            }
            NavigableElementType::RadioButton => {
                matches!(action, ElementAction::Activate | ElementAction::Click)
            }
            NavigableElementType::ComboBox => {
                matches!(
                    action,
                    ElementAction::Open | ElementAction::Focus | ElementAction::Activate
                )
            }
            _ => matches!(action, ElementAction::Activate),
        }
    }

    /// Convert to NavigableElement for use with navigation system
    pub fn to_navigable_element(&self) -> NavigableElement {
        let mut supported_actions = match self.widget_type {
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
            _ => vec![ElementAction::Activate],
        };

        // All elements support smart action for universal hinting
        supported_actions.push(ElementAction::Smart);

        NavigableElement {
            id: self.id.clone(),
            element_type: self.widget_type.clone(),
            rect: self.rect.unwrap_or(egui::Rect::NOTHING),
            enabled: self.enabled,
            label: self.label.clone(),
            supported_actions,
            metadata: self.metadata.clone(),
        }
    }

    /// Apply focus styling to a widget if it's focused
    pub fn apply_focus_styling(&self, ui: &mut egui::Ui, response: &egui::Response) {
        if self.focused && self.enabled {
            let focus_style = &self.state.focus_state.focus_style;

            // Draw focus border using a simple rectangle outline
            let stroke = egui::Stroke::new(focus_style.border_width, focus_style.border_color);

            // Draw border by drawing four lines
            let rect = response.rect;
            let painter = ui.painter();

            // Top line
            painter.line_segment([rect.left_top(), rect.right_top()], stroke);
            // Bottom line
            painter.line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
            // Left line
            painter.line_segment([rect.left_top(), rect.left_bottom()], stroke);
            // Right line
            painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
        }
    }

    /// Update widget state based on egui response
    pub fn update_from_response(&mut self, response: &egui::Response) {
        self.rect = Some(response.rect);

        let widget_response = WidgetResponse {
            clicked: response.clicked(),
            gained_focus: response.gained_focus(),
            lost_focus: response.lost_focus(),
            changed: response.changed(),
            rect: response.rect,
        };

        self.state.last_response = Some(widget_response);
        self.state.interacted = response.clicked() || response.changed() || response.gained_focus();

        // Update focus state
        self.state.focus_state.had_focus = self.focused;
        if response.gained_focus() {
            self.focused = true;
        } else if response.lost_focus() {
            self.focused = false;
        }
    }

    /// Execute an action on this widget
    pub fn execute_action(&mut self, action: ElementAction) -> bool {
        if !self.enabled || !self.supports_action(&action) {
            return false;
        }

        match action {
            ElementAction::Click | ElementAction::Activate => {
                self.state.pending_activation = true;
                true
            }
            ElementAction::Focus => {
                self.set_focused(true);
                true
            }
            ElementAction::Toggle => {
                // For checkboxes and toggles
                self.state.pending_activation = true;
                true
            }
            _ => false,
        }
    }
}

/// Factory functions for creating common navigable widgets
impl NavigableWidget {
    /// Create a navigable button wrapper
    pub fn button(id: String, label: String) -> Self {
        Self::new(id, NavigableElementType::Button).with_label(label)
    }

    /// Create a navigable text input wrapper
    pub fn text_input(id: String, label: String) -> Self {
        Self::new(id, NavigableElementType::TextInput).with_label(label)
    }

    /// Create a navigable text area wrapper
    pub fn text_area(id: String, label: String) -> Self {
        Self::new(id, NavigableElementType::TextArea).with_label(label)
    }

    /// Create a navigable checkbox wrapper
    pub fn checkbox(id: String, label: String) -> Self {
        Self::new(id, NavigableElementType::Checkbox).with_label(label)
    }

    /// Create a navigable combo box wrapper
    pub fn combo_box(id: String, label: String) -> Self {
        Self::new(id, NavigableElementType::ComboBox).with_label(label)
    }

    /// Create a navigable slider wrapper
    pub fn slider(id: String, label: String) -> Self {
        Self::new(id, NavigableElementType::Slider).with_label(label)
    }
}

/// Trait for UI containers that support navigable widgets
pub trait NavigableContainer {
    /// Register a navigable widget with this container
    fn register_widget(&mut self, widget: NavigableWidget);

    /// Get all registered widgets
    fn get_widgets(&self) -> &[NavigableWidget];

    /// Get a mutable reference to all widgets
    fn get_widgets_mut(&mut self) -> &mut Vec<NavigableWidget>;

    /// Find a widget by its ID
    fn find_widget(&self, id: &str) -> Option<&NavigableWidget> {
        self.get_widgets().iter().find(|w| w.id == id)
    }

    /// Find a mutable widget by its ID
    fn find_widget_mut(&mut self, id: &str) -> Option<&mut NavigableWidget> {
        self.get_widgets_mut().iter_mut().find(|w| w.id == id)
    }

    /// Get the currently focused widget
    fn get_focused_widget(&self) -> Option<&NavigableWidget> {
        self.get_widgets().iter().find(|w| w.focused)
    }

    /// Set focus to a widget by ID
    fn set_widget_focus(&mut self, id: &str) -> bool {
        // Clear focus from all widgets first
        for widget in self.get_widgets_mut() {
            widget.set_focused(false);
        }

        // Set focus to the requested widget
        if let Some(widget) = self.find_widget_mut(id) {
            widget.set_focused(true);
            true
        } else {
            false
        }
    }

    /// Move focus to the next widget
    fn focus_next_widget(&mut self) -> bool {
        let widgets = self.get_widgets();
        if widgets.is_empty() {
            return false;
        }

        let current_index = widgets.iter().position(|w| w.focused);
        let next_index = match current_index {
            Some(idx) => (idx + 1) % widgets.len(),
            None => 0,
        };

        let next_widget_id = widgets[next_index].id.clone();
        self.set_widget_focus(&next_widget_id)
    }

    /// Move focus to the previous widget
    fn focus_previous_widget(&mut self) -> bool {
        let widgets = self.get_widgets();
        if widgets.is_empty() {
            return false;
        }

        let current_index = widgets.iter().position(|w| w.focused);
        let prev_index = match current_index {
            Some(idx) => {
                if idx == 0 {
                    widgets.len() - 1
                } else {
                    idx - 1
                }
            }
            None => widgets.len() - 1,
        };

        let prev_widget_id = widgets[prev_index].id.clone();
        self.set_widget_focus(&prev_widget_id)
    }
}

/// Default implementation of NavigableContainer
#[derive(Debug, Default)]
pub struct DefaultNavigableContainer {
    widgets: Vec<NavigableWidget>,
}

impl NavigableContainer for DefaultNavigableContainer {
    fn register_widget(&mut self, widget: NavigableWidget) {
        // Remove any existing widget with the same ID
        self.widgets.retain(|w| w.id != widget.id);
        self.widgets.push(widget);
    }

    fn get_widgets(&self) -> &[NavigableWidget] {
        &self.widgets
    }

    fn get_widgets_mut(&mut self) -> &mut Vec<NavigableWidget> {
        &mut self.widgets
    }
}

impl DefaultNavigableContainer {
    /// Create a new navigable container
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all widgets from this container
    pub fn clear(&mut self) {
        self.widgets.clear();
    }

    /// Get the number of widgets in this container
    pub fn widget_count(&self) -> usize {
        self.widgets.len()
    }
}

/// Context information for widget collection
#[derive(Debug, Clone)]
pub struct CollectorContext {
    /// Current window identifier being rendered
    pub window_id: Option<String>,
    /// Current window title
    pub window_title: Option<String>,
    /// Current section or container name
    pub section_name: Option<String>,
    /// Screen bounds for position validation
    pub screen_bounds: egui::Rect,
    /// Whether to perform strict bounds checking
    pub strict_bounds_checking: bool,
    /// Last logged element count to prevent spam
    pub last_logged_count: usize,
    /// Frame count when last logged to prevent per-frame spam
    pub last_logged_frame: u64,
    /// Current UI clipping rectangle for this frame
    pub ui_clip_rect: egui::Rect,
}

impl Default for CollectorContext {
    fn default() -> Self {
        Self {
            window_id: None,
            window_title: None,
            section_name: None,
            screen_bounds: egui::Rect::NOTHING,
            strict_bounds_checking: true,
            last_logged_count: 0,
            last_logged_frame: 0,
            ui_clip_rect: egui::Rect::EVERYTHING, // Default to no clipping
        }
    }
}

/// Collector for gathering navigable elements during UI rendering
#[derive(Debug)]
pub struct NavigableElementCollector {
    /// Elements collected in the current frame
    elements: Vec<NavigableElement>,
    /// Current container being processed
    current_container: Option<String>,
    /// Frame counter for lifecycle management
    frame_count: u64,
    /// Widget ID counter for unique ID generation
    widget_id_counter: u32,
    /// Context information for tracking bounds and interactions
    context_info: CollectorContext,
    /// Track if we've logged bounds errors this frame to prevent spam
    logged_bounds_error_this_frame: bool,
    /// Count of bounds errors in current frame
    bounds_error_count_this_frame: usize,
    /// Flag to enable/disable internal debug logging (disabled by default for performance)
    debug_logging_enabled: bool,
}

impl NavigableElementCollector {
    /// Enable or disable internal debug logging for this collector
    /// Useful for debugging widget registration without flooding logs in production
    pub fn set_debug_logging(&mut self, enabled: bool) {
        self.debug_logging_enabled = enabled;
    }

    /// Check if debug logging is currently enabled
    pub fn is_debug_logging_enabled(&self) -> bool {
        self.debug_logging_enabled
    }
    /// Create a new element collector
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            current_container: None,
            frame_count: 0,
            widget_id_counter: 0,
            context_info: CollectorContext::default(),
            logged_bounds_error_this_frame: false,
            bounds_error_count_this_frame: 0,
            debug_logging_enabled: false, // Disabled by default for performance
        }
    }

    /// Start a new frame collection cycle
    pub fn start_frame(&mut self) {
        self.elements.clear();
        self.frame_count += 1;
        self.widget_id_counter = 0; // Reset counter for new frame

        // Reset bounds error tracking for new frame
        self.logged_bounds_error_this_frame = false;
        self.bounds_error_count_this_frame = 0;
    }

    /// Update the collector context with window information
    pub fn set_context(
        &mut self,
        window_id: String,
        window_title: String,
        screen_bounds: egui::Rect,
    ) {
        self.context_info.window_id = Some(window_id);
        self.context_info.window_title = Some(window_title);
        self.context_info.screen_bounds = screen_bounds;
    }

    /// Set the UI clipping context for this frame
    pub fn set_ui_clipping_context(&mut self, clip_rect: egui::Rect) {
        self.context_info.ui_clip_rect = clip_rect;
    }

    /// Generate a unique widget ID based on context and counter
    pub fn generate_widget_id(&mut self, prefix: &str, user_id: Option<&str>) -> String {
        let id = if let Some(user_id) = user_id {
            format!("{}_{}", prefix, user_id)
        } else {
            self.widget_id_counter += 1;
            format!(
                "{}_{}_auto_{}",
                self.context_info.window_id.as_deref().unwrap_or("window"),
                prefix,
                self.widget_id_counter
            )
        };

        // Ensure uniqueness by checking existing elements
        let mut unique_id = id.clone();
        let mut suffix = 1;
        while self.elements.iter().any(|e| e.id == unique_id) {
            unique_id = format!("{}_{}", id, suffix);
            suffix += 1;
        }

        unique_id
    }

    /// Validate element bounds are reasonable
    pub fn validate_element_bounds(&self, rect: &egui::Rect) -> bool {
        if !self.context_info.strict_bounds_checking {
            return true;
        }

        // Check if rect has positive size
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return false;
        }

        // Check if rect is within reasonable screen bounds (allow some tolerance)
        let screen = &self.context_info.screen_bounds;
        if screen != &egui::Rect::NOTHING {
            let tolerance = 100.0; // Allow elements slightly outside screen
            let expanded_screen = screen.expand(tolerance);
            if !expanded_screen.intersects(*rect) {
                return false;
            }
        }

        true
    }

    /// Set the current container context
    pub fn set_container(&mut self, container_id: String) {
        self.current_container = Some(container_id);
    }

    /// Add a navigable element to the collection
    pub fn add_element(&mut self, mut element: NavigableElement) {
        // Validate element bounds before adding (with proper debounced logging)
        if !self.validate_element_bounds(&element.rect) {
            self.bounds_error_count_this_frame += 1;

            // Only log bounds issues once per frame and only occasionally (if debug logging enabled)
            if self.debug_logging_enabled
                && !self.logged_bounds_error_this_frame
                && self.frame_count % 300 == 0
            {
                // Every 5 seconds at 60fps
                tracing::debug!("Frame {}: Skipping elements with invalid bounds (example: '{}' {:?}) - will batch additional errors this frame",
                               self.frame_count, element.id, element.rect);
                self.logged_bounds_error_this_frame = true;
            }
            return;
        }

        // Enhance metadata with context information
        if let Some(window_id) = &self.context_info.window_id {
            element
                .metadata
                .insert("window_id".to_string(), window_id.clone());
        }
        if let Some(window_title) = &self.context_info.window_title {
            element
                .metadata
                .insert("window_title".to_string(), window_title.clone());
        }
        if let Some(container) = &self.current_container {
            element
                .metadata
                .insert("container_id".to_string(), container.clone());
        }
        element
            .metadata
            .insert("frame_count".to_string(), self.frame_count.to_string());
        element.metadata.insert(
            "collection_order".to_string(),
            self.elements.len().to_string(),
        );

        // Store clipping context in a parseable format
        element.metadata.insert(
            "clip_min_x".to_string(),
            self.context_info.ui_clip_rect.min.x.to_string(),
        );
        element.metadata.insert(
            "clip_min_y".to_string(),
            self.context_info.ui_clip_rect.min.y.to_string(),
        );
        element.metadata.insert(
            "clip_max_x".to_string(),
            self.context_info.ui_clip_rect.max.x.to_string(),
        );
        element.metadata.insert(
            "clip_max_y".to_string(),
            self.context_info.ui_clip_rect.max.y.to_string(),
        );

        // Only trace log elements very occasionally to avoid spam (if debug logging enabled)
        if self.debug_logging_enabled && self.elements.len() < 2 && self.frame_count % 300 == 0 {
            // Only first 2 elements, every 5 seconds
            tracing::trace!(
                "Frame {}: Added element '{}' type={:?} enabled={} (sample logging)",
                self.frame_count,
                element.id,
                element.element_type,
                element.enabled
            );
        }

        self.elements.push(element);
    }

    /// Register a widget and convert it to an element
    pub fn register_widget(&mut self, widget: &NavigableWidget) {
        let element = widget.to_navigable_element();
        self.add_element(element);
    }

    /// Get all collected elements for the current frame
    pub fn get_elements(&self) -> &[NavigableElement] {
        &self.elements
    }

    /// Find an element by ID
    pub fn find_element(&self, id: &str) -> Option<&NavigableElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    /// Get elements by type
    pub fn get_elements_by_type(
        &self,
        element_type: &NavigableElementType,
    ) -> Vec<&NavigableElement> {
        self.elements
            .iter()
            .filter(|e| &e.element_type == element_type)
            .collect()
    }

    /// Get enabled elements only
    pub fn get_enabled_elements(&self) -> Vec<&NavigableElement> {
        self.elements.iter().filter(|e| e.enabled).collect()
    }

    /// Get elements that support a specific action
    pub fn get_actionable_elements(&self, action: &ElementAction) -> Vec<&NavigableElement> {
        self.elements
            .iter()
            .filter(|e| e.supports_action(action))
            .collect()
    }

    /// Clear the current collection
    pub fn clear(&mut self) {
        self.elements.clear();
        self.current_container = None;
    }

    /// Get the current frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Query egui context memory for widget interaction state
    pub fn query_context_memory(&mut self, ctx: &egui::Context) {
        // Query interaction state from egui's memory system
        let memory = ctx.memory(|mem| {
            // Check for focused widget
            let focused_id = mem.focused();

            // Get interaction information
            let mut interaction_info = HashMap::new();
            if let Some(id) = focused_id {
                interaction_info.insert("focused_widget".to_string(), format!("{:?}", id));
            }

            interaction_info
        });

        // Store interaction state in context (if debug logging enabled)
        if self.debug_logging_enabled {
            for (key, value) in memory {
                tracing::trace!(
                    "NavigableElementCollector: Context memory {} = {}",
                    key,
                    value
                );
            }
        }

        // Query input state separately using the input API (reduced logging)
        ctx.input(|i| {
            if let Some(_hover_pos) = i.pointer.hover_pos() {
                // Only log hover position occasionally to avoid spam (if debug logging enabled)
                if self.debug_logging_enabled && self.frame_count % 120 == 0 {
                    // Every 2 seconds at 60fps
                    tracing::trace!(
                        "NavigableElementCollector: Hover position tracking active (frame {})",
                        self.frame_count
                    );
                }
            }
        });
    }

    /// Update widget states based on egui context queries
    pub fn update_widget_states(&mut self, ctx: &egui::Context) {
        // Query current interaction state
        self.query_context_memory(ctx);

        // Update element metadata with current frame interaction state
        let screen_rect = ctx.screen_rect();
        self.context_info.screen_bounds = screen_rect;

        // Check for elements that may have changed interaction state
        for element in &mut self.elements {
            // Update metadata with current frame information
            element
                .metadata
                .insert("screen_width".to_string(), screen_rect.width().to_string());
            element.metadata.insert(
                "screen_height".to_string(),
                screen_rect.height().to_string(),
            );

            // Mark elements that are within current viewport
            let is_visible = screen_rect.intersects(element.rect);
            element
                .metadata
                .insert("viewport_visible".to_string(), is_visible.to_string());
        }
    }
}

impl Default for NavigableElementCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for systematic widget registration during UI rendering
///
/// This trait provides a consistent interface for capturing UI widgets as they're rendered,
/// converting them to NavigableElements for use with the keyboard navigation system.
pub trait WidgetRegistrar {
    /// Register a button widget with the navigation system
    fn register_button<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response;

    /// Register a text input widget with the navigation system
    fn register_text_input<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response;

    /// Register a text area widget with the navigation system
    fn register_text_area<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response;

    /// Register a checkbox widget with the navigation system
    fn register_checkbox<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response;

    /// Register a combo box widget with the navigation system
    fn register_combo_box<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response;

    /// Register a generic clickable element (links, labels, etc.)
    fn register_clickable<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response;

    /// Register a list item widget with the navigation system
    fn register_list_item<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response;

    /// Register a custom widget with specific type and actions
    fn register_custom_widget<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        widget_type: NavigableElementType,
        supported_actions: Vec<ElementAction>,
        label: Option<String>,
        metadata: HashMap<String, String>,
    ) -> &'a egui::Response;

    /// Start a new frame for widget collection
    fn start_widget_frame(&mut self);

    /// Get the current widget count for debugging
    fn widget_count(&self) -> usize;
}

/// Context information for widget registration
#[derive(Debug, Clone)]
pub struct WidgetRegistrationContext {
    /// ID of the window containing this widget
    pub window_id: String,
    /// Title of the window containing this widget
    pub window_title: String,
    /// Optional section or container within the window
    pub section_id: Option<String>,
    /// Frame number for lifecycle tracking
    pub frame_number: u64,
}

impl WidgetRegistrationContext {
    /// Create a new widget registration context
    pub fn new(window_id: String, window_title: String) -> Self {
        Self {
            window_id,
            window_title,
            section_id: None,
            frame_number: 0,
        }
    }

    /// Set the section context for widgets
    pub fn with_section(mut self, section_id: String) -> Self {
        self.section_id = Some(section_id);
        self
    }

    /// Update the frame number
    pub fn with_frame(mut self, frame_number: u64) -> Self {
        self.frame_number = frame_number;
        self
    }
}

/// Pending action to be executed on a widget
#[derive(Debug, Clone)]
pub struct PendingWidgetAction {
    pub element_id: String,
    pub action: ElementAction,
    pub timestamp: std::time::Instant,
}

/// Manager for navigable widgets across the entire application
#[derive(Debug)]
pub struct NavigableWidgetManager {
    /// Global element collector
    collector: NavigableElementCollector,
    /// Currently focused element ID
    focused_element_id: Option<String>,
    /// Navigation state for widget-level operations
    widget_navigation_enabled: bool,
    /// Element focus history for navigation
    focus_history: Vec<String>,
    /// Maximum history size
    max_history_size: usize,
    /// Queue of pending actions to execute on widgets
    pending_actions: Vec<PendingWidgetAction>,
}

impl NavigableWidgetManager {
    /// Enable or disable internal debug logging for widget management
    /// This controls logging for widget registration, element collection, and frame processing
    pub fn set_debug_logging(&mut self, enabled: bool) {
        self.collector.set_debug_logging(enabled);
    }

    /// Check if debug logging is currently enabled
    pub fn is_debug_logging_enabled(&self) -> bool {
        self.collector.is_debug_logging_enabled()
    }

    /// Check if an element should be clicked due to keyboard navigation
    pub fn should_element_be_clicked(&mut self, element_id: &str) -> bool {
        let actions = self.get_pending_actions(element_id);
        actions.iter().any(|action| {
            match action.action {
                ElementAction::Click | ElementAction::Activate => true,
                ElementAction::Smart => {
                    // Resolve Smart action to specific action based on element type
                    if let Some(element) = self.collector.find_element(element_id) {
                        matches!(
                            element.get_smart_action(),
                            ElementAction::Click | ElementAction::Activate
                        )
                    } else {
                        false
                    }
                }
                _ => false,
            }
        })
    }

    /// Check if an element should be focused due to keyboard navigation
    pub fn should_element_be_focused(&mut self, element_id: &str) -> bool {
        let actions = self.get_pending_actions(element_id);
        actions.iter().any(|action| {
            match action.action {
                ElementAction::Focus => true,
                ElementAction::Smart => {
                    // Resolve Smart action to specific action based on element type
                    if let Some(element) = self.collector.find_element(element_id) {
                        matches!(element.get_smart_action(), ElementAction::Focus)
                    } else {
                        false
                    }
                }
                _ => false,
            }
        })
    }
    /// Create a new widget manager
    pub fn new() -> Self {
        Self {
            collector: NavigableElementCollector::new(),
            focused_element_id: None,
            widget_navigation_enabled: true,
            focus_history: Vec::new(),
            max_history_size: 50,
            pending_actions: Vec::new(),
        }
    }

    /// Start a new frame for widget collection
    pub fn start_frame(&mut self) {
        self.collector.start_frame();
    }

    /// Start a new frame with context integration
    pub fn start_frame_with_context(
        &mut self,
        ctx: &egui::Context,
        window_id: String,
        window_title: String,
    ) {
        self.collector.start_frame();

        // Set context information
        let screen_bounds = ctx.screen_rect();
        self.collector
            .set_context(window_id, window_title, screen_bounds);

        // Query context memory for interaction state
        self.collector.query_context_memory(ctx);
    }

    /// Start a new frame with UI context for clipping information
    pub fn start_frame_with_ui_context(
        &mut self,
        ui: &egui::Ui,
        window_id: String,
        window_title: String,
    ) {
        self.collector.start_frame();

        // Set context information including current clipping rectangle
        let screen_bounds = ui.ctx().screen_rect();
        self.collector
            .set_context(window_id, window_title, screen_bounds);

        // Store the current UI clipping context for this frame
        self.collector.set_ui_clipping_context(ui.clip_rect());

        // Query context memory for interaction state
        self.collector.query_context_memory(ui.ctx());
    }

    /// Complete frame processing with context updates
    pub fn complete_frame(&mut self, ctx: &egui::Context) {
        // Update all widget states based on current context
        self.collector.update_widget_states(ctx);

        // Log frame completion statistics only when count changes significantly
        let element_count = self.collector.get_elements().len();
        let enabled_count = self.collector.get_enabled_elements().len();

        // Only log when element count changes by more than 5 or every 60 frames (1 second at 60fps)
        let should_log = element_count > 0
            && (self
                .collector
                .context_info
                .last_logged_count
                .abs_diff(element_count)
                > 5
                || self.collector.frame_count - self.collector.context_info.last_logged_frame > 60);

        if should_log && self.collector.debug_logging_enabled {
            tracing::debug!("NavigableWidgetManager: Frame {} - {} elements ({} enabled) - change from {} elements (skipped {} invalid bounds this frame)",
                           self.collector.frame_count(), element_count, enabled_count,
                           self.collector.context_info.last_logged_count, self.collector.bounds_error_count_this_frame);

            // Update the last logged values
            self.collector.context_info.last_logged_count = element_count;
            self.collector.context_info.last_logged_frame = self.collector.frame_count;
        }
    }

    /// Register a widget with the manager
    pub fn register_widget(&mut self, widget: &NavigableWidget) {
        self.collector.register_widget(widget);
    }

    /// Get the collector for direct access
    pub fn collector(&self) -> &NavigableElementCollector {
        &self.collector
    }

    /// Get mutable collector access
    pub fn collector_mut(&mut self) -> &mut NavigableElementCollector {
        &mut self.collector
    }

    /// Set focus to an element by ID
    pub fn set_focus(&mut self, element_id: String) -> bool {
        if self.collector.find_element(&element_id).is_some() {
            // Add previous focus to history
            if let Some(prev_id) = &self.focused_element_id {
                if prev_id != &element_id {
                    self.add_to_history(prev_id.clone());
                }
            }

            self.focused_element_id = Some(element_id);
            true
        } else {
            false
        }
    }

    /// Get the currently focused element ID
    pub fn get_focused_element_id(&self) -> Option<&String> {
        self.focused_element_id.as_ref()
    }

    /// Get the currently focused element
    pub fn get_focused_element(&self) -> Option<&NavigableElement> {
        self.focused_element_id
            .as_ref()
            .and_then(|id| self.collector.find_element(id))
    }

    /// Clear focus
    pub fn clear_focus(&mut self) {
        if let Some(prev_id) = self.focused_element_id.take() {
            self.add_to_history(prev_id);
        }
    }

    /// Focus next element in tab order
    pub fn focus_next_element(&mut self) -> bool {
        let enabled_elements = self.collector.get_enabled_elements();
        if enabled_elements.is_empty() {
            return false;
        }

        let current_index = self
            .focused_element_id
            .as_ref()
            .and_then(|id| enabled_elements.iter().position(|e| &e.id == id));

        let next_index = match current_index {
            Some(idx) => (idx + 1) % enabled_elements.len(),
            None => 0,
        };

        let next_element_id = enabled_elements[next_index].id.clone();
        self.set_focus(next_element_id)
    }

    /// Focus previous element in tab order
    pub fn focus_previous_element(&mut self) -> bool {
        let enabled_elements = self.collector.get_enabled_elements();
        if enabled_elements.is_empty() {
            return false;
        }

        let current_index = self
            .focused_element_id
            .as_ref()
            .and_then(|id| enabled_elements.iter().position(|e| &e.id == id));

        let prev_index = match current_index {
            Some(idx) => {
                if idx == 0 {
                    enabled_elements.len() - 1
                } else {
                    idx - 1
                }
            }
            None => enabled_elements.len() - 1,
        };

        let prev_element_id = enabled_elements[prev_index].id.clone();
        self.set_focus(prev_element_id)
    }

    /// Focus the last focused element (from history)
    pub fn focus_last_element(&mut self) -> bool {
        if let Some(last_id) = self.focus_history.pop() {
            self.set_focus(last_id)
        } else {
            false
        }
    }

    /// Enable or disable widget navigation
    pub fn set_navigation_enabled(&mut self, enabled: bool) {
        self.widget_navigation_enabled = enabled;
        if !enabled {
            self.clear_focus();
        }
    }

    /// Check if widget navigation is enabled
    pub fn is_navigation_enabled(&self) -> bool {
        self.widget_navigation_enabled
    }

    /// Add an element ID to focus history
    fn add_to_history(&mut self, element_id: String) {
        // Remove existing entry if present
        self.focus_history.retain(|id| id != &element_id);

        // Add to end of history
        self.focus_history.push(element_id);

        // Trim history if too long
        if self.focus_history.len() > self.max_history_size {
            self.focus_history.remove(0);
        }
    }

    /// Get focus history
    pub fn get_focus_history(&self) -> &[String] {
        &self.focus_history
    }

    /// Queue an action to be executed on a widget
    pub fn queue_action(&mut self, element_id: String, action: ElementAction) {
        // Resolve Smart action to specific action based on element type
        let resolved_action = if action == ElementAction::Smart {
            if let Some(element) = self.collector.find_element(&element_id) {
                let smart_action = element.get_smart_action();
                tracing::info!(
                    "Resolved Smart action to {:?} for element '{}' (type: {:?})",
                    smart_action,
                    element_id,
                    element.element_type
                );
                smart_action
            } else {
                tracing::warn!(
                    "Could not find element '{}' to resolve Smart action, keeping Smart",
                    element_id
                );
                action // Keep Smart if element not found
            }
        } else {
            action
        };

        self.pending_actions.push(PendingWidgetAction {
            element_id: element_id.clone(),
            action: resolved_action,
            timestamp: std::time::Instant::now(),
        });
        tracing::info!(
            "Queued action {:?} for element '{}'",
            resolved_action,
            element_id
        );
    }

    /// Get pending actions for a specific element ID
    pub fn get_pending_actions(&self, element_id: &str) -> Vec<&PendingWidgetAction> {
        self.pending_actions
            .iter()
            .filter(|action| action.element_id == element_id)
            .collect()
    }

    /// Remove and return all pending actions for a specific element ID
    pub fn consume_pending_actions(&mut self, element_id: &str) -> Vec<PendingWidgetAction> {
        let (matching, remaining): (Vec<_>, Vec<_>) = self
            .pending_actions
            .drain(..)
            .partition(|action| action.element_id == element_id);

        self.pending_actions = remaining;
        matching
    }

    /// Clear all pending actions (call this at frame start to prevent stale actions)
    pub fn clear_stale_actions(&mut self, max_age_ms: u64) {
        let now = std::time::Instant::now();
        self.pending_actions.retain(|action| {
            now.duration_since(action.timestamp).as_millis() <= max_age_ms as u128
        });
    }

    /// Get count of pending actions
    pub fn pending_action_count(&self) -> usize {
        self.pending_actions.len()
    }
}

impl WidgetRegistrar for NavigableWidgetManager {
    fn register_button<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response {
        let mut metadata = HashMap::new();
        metadata.insert("widget_type".to_string(), "button".to_string());
        metadata.insert("clicked".to_string(), response.clicked().to_string());
        metadata.insert("hovered".to_string(), response.hovered().to_string());
        metadata.insert("rect".to_string(), format!("{:?}", response.rect));

        let element = NavigableElement {
            id: id.clone(),
            element_type: NavigableElementType::Button,
            rect: response.rect,
            enabled: response.enabled(),
            label: label.clone(),
            supported_actions: vec![
                ElementAction::Click,
                ElementAction::Activate,
                ElementAction::Smart,
            ],
            metadata,
        };

        // Reduced logging - only log occasionally (if debug logging enabled)
        if self.collector.debug_logging_enabled
            && self.collector.frame_count % 30 == 0
            && self.collector.get_elements().len() < 3
        {
            tracing::trace!(
                "Frame {}: Registered button widget: id='{}', label={:?}",
                self.collector.frame_count(),
                id,
                label
            );
        }

        self.collector.add_element(element);
        response
    }

    fn register_text_input<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response {
        let mut metadata = HashMap::new();
        metadata.insert("widget_type".to_string(), "text_input".to_string());
        metadata.insert("has_focus".to_string(), response.has_focus().to_string());
        metadata.insert("changed".to_string(), response.changed().to_string());
        metadata.insert("rect".to_string(), format!("{:?}", response.rect));

        let element = NavigableElement {
            id: id.clone(),
            element_type: NavigableElementType::TextInput,
            rect: response.rect,
            enabled: response.enabled(),
            label: label.clone(),
            supported_actions: vec![
                ElementAction::Focus,
                ElementAction::Select,
                ElementAction::Copy,
                ElementAction::Smart,
            ],
            metadata,
        };

        // Reduced logging - only log occasionally (if debug logging enabled)
        if self.collector.debug_logging_enabled
            && self.collector.frame_count % 60 == 0
            && self.collector.get_elements().len() < 2
        {
            tracing::trace!(
                "Frame {}: Registered text_input widget: id='{}', label={:?}",
                self.collector.frame_count(),
                id,
                label
            );
        }

        self.collector.add_element(element);
        response
    }

    fn register_text_area<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response {
        let mut metadata = HashMap::new();
        metadata.insert("widget_type".to_string(), "text_area".to_string());
        metadata.insert("has_focus".to_string(), response.has_focus().to_string());
        metadata.insert("changed".to_string(), response.changed().to_string());
        metadata.insert("rect".to_string(), format!("{:?}", response.rect));

        let element = NavigableElement {
            id: id.clone(),
            element_type: NavigableElementType::TextArea,
            rect: response.rect,
            enabled: response.enabled(),
            label: label.clone(),
            supported_actions: vec![
                ElementAction::Focus,
                ElementAction::Select,
                ElementAction::Copy,
                ElementAction::Smart,
            ],
            metadata,
        };

        // Minimal logging to avoid spam

        self.collector.add_element(element);
        response
    }

    fn register_checkbox<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response {
        let mut metadata = HashMap::new();
        metadata.insert("widget_type".to_string(), "checkbox".to_string());
        metadata.insert("clicked".to_string(), response.clicked().to_string());
        metadata.insert("changed".to_string(), response.changed().to_string());
        metadata.insert("rect".to_string(), format!("{:?}", response.rect));

        let element = NavigableElement {
            id: id.clone(),
            element_type: NavigableElementType::Checkbox,
            rect: response.rect,
            enabled: response.enabled(),
            label: label.clone(),
            supported_actions: vec![
                ElementAction::Toggle,
                ElementAction::Activate,
                ElementAction::Click,
                ElementAction::Smart,
            ],
            metadata,
        };

        // Minimal logging to avoid spam

        self.collector.add_element(element);
        response
    }

    fn register_combo_box<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response {
        let mut metadata = HashMap::new();
        metadata.insert("widget_type".to_string(), "combo_box".to_string());
        metadata.insert("clicked".to_string(), response.clicked().to_string());
        metadata.insert("has_focus".to_string(), response.has_focus().to_string());
        metadata.insert("rect".to_string(), format!("{:?}", response.rect));

        let element = NavigableElement {
            id: id.clone(),
            element_type: NavigableElementType::ComboBox,
            rect: response.rect,
            enabled: response.enabled(),
            label: label.clone(),
            supported_actions: vec![
                ElementAction::Open,
                ElementAction::Focus,
                ElementAction::Activate,
                ElementAction::Smart,
            ],
            metadata,
        };

        // Minimal logging to avoid spam

        self.collector.add_element(element);
        response
    }

    fn register_clickable<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response {
        let element = NavigableElement {
            id,
            element_type: NavigableElementType::Link,
            rect: response.rect,
            enabled: response.enabled(),
            label,
            supported_actions: vec![
                ElementAction::Click,
                ElementAction::Activate,
                ElementAction::Smart,
            ],
            metadata: HashMap::new(),
        };
        self.collector.add_element(element);
        response
    }

    fn register_list_item<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        label: Option<String>,
    ) -> &'a egui::Response {
        let mut metadata = HashMap::new();
        metadata.insert("widget_type".to_string(), "list_item".to_string());
        metadata.insert("clicked".to_string(), response.clicked().to_string());
        metadata.insert("hovered".to_string(), response.hovered().to_string());
        metadata.insert("rect".to_string(), format!("{:?}", response.rect));

        let element = NavigableElement {
            id: id.clone(),
            element_type: NavigableElementType::ListItem,
            rect: response.rect,
            enabled: response.enabled(),
            label: label.clone(),
            supported_actions: vec![
                ElementAction::Click,
                ElementAction::Select,
                ElementAction::Smart,
            ],
            metadata,
        };

        // Minimal logging to avoid spam

        self.collector.add_element(element);
        response
    }

    fn register_custom_widget<'a>(
        &mut self,
        response: &'a egui::Response,
        id: String,
        widget_type: NavigableElementType,
        supported_actions: Vec<ElementAction>,
        label: Option<String>,
        metadata: HashMap<String, String>,
    ) -> &'a egui::Response {
        let element = NavigableElement {
            id,
            element_type: widget_type,
            rect: response.rect,
            enabled: response.enabled(),
            label,
            supported_actions,
            metadata,
        };
        self.collector.add_element(element);
        response
    }

    fn start_widget_frame(&mut self) {
        self.start_frame();
    }

    fn widget_count(&self) -> usize {
        self.collector.get_elements().len()
    }
}

impl Default for NavigableWidgetManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigable_widget_creation() {
        let widget = NavigableWidget::button("test_btn".to_string(), "Test Button".to_string());

        assert_eq!(widget.id, "test_btn");
        assert_eq!(widget.widget_type, NavigableElementType::Button);
        assert_eq!(widget.label, Some("Test Button".to_string()));
        assert!(widget.enabled);
        assert!(!widget.focused);
    }

    #[test]
    fn test_widget_action_support() {
        let button = NavigableWidget::button("btn".to_string(), "Button".to_string());
        assert!(button.supports_action(&ElementAction::Click));
        assert!(button.supports_action(&ElementAction::Activate));
        assert!(!button.supports_action(&ElementAction::Focus));
        // All widgets should support Smart action for universal hinting
        assert!(button.supports_action(&ElementAction::Smart));

        let text_input = NavigableWidget::text_input("txt".to_string(), "Text".to_string());
        assert!(text_input.supports_action(&ElementAction::Focus));
        assert!(!text_input.supports_action(&ElementAction::Click));
        // All widgets should support Smart action for universal hinting
        assert!(text_input.supports_action(&ElementAction::Smart));
    }

    #[test]
    fn test_widget_focus() {
        let mut widget = NavigableWidget::button("btn".to_string(), "Button".to_string());

        assert!(!widget.focused);
        assert!(!widget.state.focus_state.should_focus);

        widget.set_focused(true);
        assert!(widget.focused);
        assert!(widget.state.focus_state.should_focus);
    }

    #[test]
    fn test_navigable_container() {
        let mut container = DefaultNavigableContainer::new();

        let widget1 = NavigableWidget::button("btn1".to_string(), "Button 1".to_string());
        let widget2 = NavigableWidget::button("btn2".to_string(), "Button 2".to_string());

        container.register_widget(widget1);
        container.register_widget(widget2);

        assert_eq!(container.widget_count(), 2);
        assert!(container.find_widget("btn1").is_some());
        assert!(container.find_widget("btn2").is_some());
        assert!(container.find_widget("btn3").is_none());
    }

    #[test]
    fn test_container_focus_navigation() {
        let mut container = DefaultNavigableContainer::new();

        let widget1 = NavigableWidget::button("btn1".to_string(), "Button 1".to_string());
        let widget2 = NavigableWidget::button("btn2".to_string(), "Button 2".to_string());

        container.register_widget(widget1);
        container.register_widget(widget2);

        // Test focus next
        assert!(container.focus_next_widget());
        assert!(container.find_widget("btn1").unwrap().focused);

        assert!(container.focus_next_widget());
        assert!(container.find_widget("btn2").unwrap().focused);
        assert!(!container.find_widget("btn1").unwrap().focused);

        // Test focus previous
        assert!(container.focus_previous_widget());
        assert!(container.find_widget("btn1").unwrap().focused);
        assert!(!container.find_widget("btn2").unwrap().focused);
    }

    #[test]
    fn test_widget_to_navigable_element() {
        let widget = NavigableWidget::button("btn".to_string(), "Test Button".to_string())
            .with_metadata("group".to_string(), "toolbar".to_string());

        let element = widget.to_navigable_element();

        assert_eq!(element.id, "btn");
        assert_eq!(element.element_type, NavigableElementType::Button);
        assert_eq!(element.label, Some("Test Button".to_string()));
        assert!(element.supports_action(&ElementAction::Click));
        // Verify Smart action is included in NavigableElement
        assert!(element.supports_action(&ElementAction::Smart));
        assert!(element.supported_actions.contains(&ElementAction::Smart));
        assert_eq!(element.metadata.get("group"), Some(&"toolbar".to_string()));
    }

    #[test]
    fn test_widget_action_execution() {
        let mut widget = NavigableWidget::button("btn".to_string(), "Button".to_string());

        // Test valid action
        assert!(widget.execute_action(ElementAction::Click));
        assert!(widget.state.pending_activation);

        // Test invalid action
        widget.state.pending_activation = false;
        assert!(!widget.execute_action(ElementAction::Focus));
        assert!(!widget.state.pending_activation);

        // Test disabled widget
        widget.enabled = false;
        assert!(!widget.execute_action(ElementAction::Click));
    }
}
