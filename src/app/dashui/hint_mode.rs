//! Hint Mode Implementation
//!
//! This module implements Vimium-style hint mode for keyboard navigation.
//! It provides visual hints over clickable/focusable elements and allows
//! users to interact with elements by typing the corresponding hint labels.

use super::keyboard_navigation::{ElementAction, NavigableElement};
use eframe::egui;
use std::collections::HashMap;

/// Configuration for hint generation and display
#[derive(Debug, Clone)]
pub struct HintConfig {
    /// Characters used for hint labels (home row keys preferred)
    pub hint_chars: Vec<char>,
    /// Regular font size for hint labels
    pub font_size: f32,
    /// Minimum font size (never go smaller than this)
    pub min_font_size: f32,
    /// Small font size for tight spaces
    pub small_font_size: f32,
    /// Background color for hint labels
    pub background_color: egui::Color32,
    /// Text color for hint labels
    pub text_color: egui::Color32,
    /// Border color for hint labels
    pub border_color: egui::Color32,
    /// Border width for hint labels
    pub border_width: f32,
    /// Padding around hint text
    pub padding: f32,
    /// Minimum distance between hints to avoid overlaps
    pub min_hint_distance: f32,
}

impl Default for HintConfig {
    fn default() -> Self {
        Self {
            // Vimium-style home row keys + additional letters
            hint_chars: vec![
                'f', 'j', 'd', 'k', 's', 'l', 'a', ';', 'g', 'h', 'q', 'w', 'e', 'r', 't', 'y',
                'u', 'i', 'o', 'p', 'z', 'x', 'c', 'v', 'b', 'n', 'm',
            ],
            font_size: 11.0,
            min_font_size: 8.0,
            small_font_size: 10.0,
            background_color: egui::Color32::from_rgb(255, 247, 15), // Yellow background
            text_color: egui::Color32::BLACK,
            border_color: egui::Color32::from_rgb(200, 200, 200),
            border_width: 1.0,
            padding: 4.0,
            min_hint_distance: 10.0,
        }
    }
}

/// Visual marker for hint display
#[derive(Debug, Clone)]
pub struct HintMarker {
    /// Unique identifier for this hint
    pub id: String,
    /// The hint label text (e.g., "f", "fd", "abc")
    pub label: String,
    /// Screen position for the hint
    pub position: egui::Pos2,
    /// Size of the hint rectangle
    pub size: egui::Vec2,
    /// Whether this hint is currently highlighted
    pub highlighted: bool,
    /// Whether this hint matches the current filter
    pub visible: bool,
    /// Associated navigable element
    pub element_id: String,
    /// Action to perform when this hint is activated
    pub action: ElementAction,
    /// Original element bounds for visibility checking
    pub element_rect: egui::Rect,
    /// Clipping rectangle when this hint was generated (for scroll area clipping)
    pub clip_rect: egui::Rect,
}

impl HintMarker {
    /// Create a new hint marker
    pub fn new(
        id: String,
        label: String,
        position: egui::Pos2,
        element_id: String,
        action: ElementAction,
        element_rect: egui::Rect,
        clip_rect: egui::Rect,
    ) -> Self {
        Self {
            id,
            label: label.clone(),
            position,
            size: egui::Vec2::ZERO, // Will be calculated during rendering
            highlighted: false,
            visible: true,
            element_id,
            action,
            element_rect,
            clip_rect,
        }
    }

    /// Check if this hint matches the given filter
    pub fn matches_filter(&self, filter: &str) -> bool {
        if filter.is_empty() {
            true
        } else {
            self.label.starts_with(filter)
        }
    }

    /// Check if this hint is an exact match for the filter
    pub fn is_exact_match(&self, filter: &str) -> bool {
        self.label == filter
    }

    /// Get the rectangle occupied by this hint
    pub fn rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(self.position, self.size)
    }
}

/// Hint generation algorithm
#[derive(Debug)]
pub struct HintGenerator {
    config: HintConfig,
    next_hint_index: usize,
}

impl HintGenerator {
    /// Create a new hint generator with default configuration
    pub fn new() -> Self {
        Self {
            config: HintConfig::default(),
            next_hint_index: 0,
        }
    }

    /// Create a hint generator with custom configuration
    pub fn with_config(config: HintConfig) -> Self {
        Self {
            config,
            next_hint_index: 0,
        }
    }

    /// Generate hint labels for the given number of elements
    pub fn generate_hint_labels(&mut self, element_count: usize) -> Vec<String> {
        let mut labels = Vec::new();
        self.next_hint_index = 0;

        if element_count == 0 {
            return labels;
        }

        // Use single characters if we have enough
        if element_count <= self.config.hint_chars.len() {
            for i in 0..element_count {
                labels.push(self.config.hint_chars[i].to_string());
            }
        } else {
            // Use multi-character hints
            for i in 0..element_count {
                labels.push(self.generate_multi_char_hint(i));
            }
        }

        labels
    }

    /// Generate a multi-character hint for the given index
    fn generate_multi_char_hint(&self, index: usize) -> String {
        let base = self.config.hint_chars.len();
        let mut result = String::new();
        let mut idx = index;

        loop {
            result.insert(0, self.config.hint_chars[idx % base]);
            idx /= base;
            if idx == 0 {
                break;
            }
            idx -= 1; // Adjust for 0-based indexing
        }

        result
    }

    /// Reset the hint generator state
    pub fn reset(&mut self) {
        self.next_hint_index = 0;
    }

    /// Get the current hint configuration
    pub fn config(&self) -> &HintConfig {
        &self.config
    }

    /// Update the hint configuration
    pub fn set_config(&mut self, config: HintConfig) {
        self.config = config;
    }
}

impl Default for HintGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Main hint mode manager
#[derive(Debug)]
pub struct HintMode {
    /// Current action mode for hints
    current_action: ElementAction,
    /// Generated hint markers
    hints: Vec<HintMarker>,
    /// Current filter string
    filter: String,
    /// Hint generator
    generator: HintGenerator,
    /// Whether hint mode is currently active
    active: bool,
    /// Map of element IDs to their hints
    element_to_hint: HashMap<String, String>,
    /// Whether to show hints for all elements or just matching ones
    #[allow(dead_code)]
    show_all_hints: bool,
}

impl HintMode {
    /// Create a new hint mode manager
    pub fn new() -> Self {
        Self {
            current_action: ElementAction::Click,
            hints: Vec::new(),
            filter: String::new(),
            generator: HintGenerator::new(),
            active: false,
            element_to_hint: HashMap::new(),
            show_all_hints: true,
        }
    }

    /// Start hint mode with the specified action
    pub fn start(&mut self, action: ElementAction, elements: &[NavigableElement]) {
        tracing::info!(
            "start: Entering hint mode with action {:?}, {} elements provided",
            action,
            elements.len()
        );
        self.current_action = action;
        self.active = true;
        self.filter.clear();
        self.generate_hints(elements);
        tracing::info!(
            "start: Hint mode active with {} total hints generated",
            self.hints.len()
        );
    }

    /// Stop hint mode and clear all hints
    pub fn stop(&mut self) {
        tracing::info!(
            "stop: Exiting hint mode, clearing {} hints",
            self.hints.len()
        );
        self.active = false;
        self.hints.clear();
        self.filter.clear();
        self.element_to_hint.clear();
    }

    /// Check if hint mode is currently active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get the current action mode
    pub fn current_action(&self) -> ElementAction {
        self.current_action
    }

    /// Add a character to the filter
    pub fn add_filter_char(&mut self, ch: char) {
        self.filter.push(ch);
        tracing::debug!(
            "add_filter_char: Filter updated to '{}', updating hint visibility",
            self.filter
        );
        self.update_hint_visibility();
        let visible_count = self.visible_hints().len();
        tracing::debug!(
            "add_filter_char: {} hints now visible after filtering",
            visible_count
        );
    }

    /// Remove the last character from the filter
    pub fn remove_filter_char(&mut self) {
        let old_filter = self.filter.clone();
        self.filter.pop();
        tracing::debug!(
            "remove_filter_char: Filter changed from '{}' to '{}'",
            old_filter,
            self.filter
        );
        self.update_hint_visibility();
    }

    /// Clear the filter
    pub fn clear_filter(&mut self) {
        let old_filter = self.filter.clone();
        self.filter.clear();
        tracing::debug!("clear_filter: Filter cleared from '{}'", old_filter);
        self.update_hint_visibility();
    }

    /// Get the current filter string
    pub fn current_filter(&self) -> &str {
        &self.filter
    }

    /// Check if there's an exact match for the current filter
    pub fn has_exact_match(&self) -> Option<&HintMarker> {
        let exact_match = self
            .hints
            .iter()
            .find(|hint| hint.is_exact_match(&self.filter));
        if let Some(hint) = exact_match {
            tracing::debug!(
                "has_exact_match: Found exact match for filter '{}' -> element_id='{}' label='{}'",
                self.filter,
                hint.element_id,
                hint.label
            );
        }
        exact_match
    }

    /// Get the element ID for the exact match, if any
    pub fn get_exact_match_element_id(&self) -> Option<String> {
        let element_id = self.has_exact_match().map(|hint| hint.element_id.clone());
        if let Some(ref id) = element_id {
            tracing::info!(
                "get_exact_match_element_id: Exact match found for filter '{}' -> element_id='{}'",
                self.filter,
                id
            );
        }
        element_id
    }

    /// Get all visible hints
    pub fn visible_hints(&self) -> Vec<&HintMarker> {
        self.hints.iter().filter(|hint| hint.visible).collect()
    }

    /// Get all hints
    pub fn all_hints(&self) -> &[HintMarker] {
        &self.hints
    }

    /// Generate hints for the given elements
    fn generate_hints(&mut self, elements: &[NavigableElement]) {
        self.hints.clear();
        self.element_to_hint.clear();

        // Filter elements that support the current action
        let actionable_elements: Vec<&NavigableElement> = elements
            .iter()
            .filter(|e| e.enabled && e.supports_action(&self.current_action))
            .collect();

        tracing::info!(
            "generate_hints: Processing {} total elements, {} actionable for action {:?}",
            elements.len(),
            actionable_elements.len(),
            self.current_action
        );

        if actionable_elements.is_empty() {
            tracing::warn!(
                "generate_hints: No actionable elements found for action {:?}",
                self.current_action
            );
            return;
        }

        // Generate hint labels
        let labels = self
            .generator
            .generate_hint_labels(actionable_elements.len());

        // Create hint markers with detailed logging
        for (i, element) in actionable_elements.iter().enumerate() {
            if i < labels.len() {
                let label = &labels[i];
                let position = self.calculate_hint_position(&element.rect, i);

                // Extract detailed widget information from metadata
                let window_id = element
                    .metadata
                    .get("window_id")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                let field_type = element
                    .metadata
                    .get("field_type")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                let widget_label = element.label.as_deref().unwrap_or("unlabeled");

                tracing::debug!("generate_hints: Creating hint[{}] label='{}' for element_id='{}' type={:?} field_type='{}' widget_label='{}' window_id='{}' rect={:?} supported_actions={:?}",
                               i, label, element.id, element.element_type, field_type, widget_label, window_id, element.rect, element.supported_actions);

                // Extract clipping context from element metadata
                let clip_rect = {
                    let min_x = element
                        .metadata
                        .get("clip_min_x")
                        .and_then(|s| s.parse::<f32>().ok());
                    let min_y = element
                        .metadata
                        .get("clip_min_y")
                        .and_then(|s| s.parse::<f32>().ok());
                    let max_x = element
                        .metadata
                        .get("clip_max_x")
                        .and_then(|s| s.parse::<f32>().ok());
                    let max_y = element
                        .metadata
                        .get("clip_max_y")
                        .and_then(|s| s.parse::<f32>().ok());

                    if let (Some(min_x), Some(min_y), Some(max_x), Some(max_y)) =
                        (min_x, min_y, max_x, max_y)
                    {
                        egui::Rect::from_min_max(
                            egui::Pos2::new(min_x, min_y),
                            egui::Pos2::new(max_x, max_y),
                        )
                    } else {
                        egui::Rect::EVERYTHING // Fallback if parsing fails
                    }
                };

                // Determine the actual action to use
                let actual_action = if self.current_action == ElementAction::Smart {
                    element.get_smart_action()
                } else {
                    self.current_action
                };

                let hint = HintMarker::new(
                    format!("hint_{}", i),
                    label.clone(),
                    position,
                    element.id.clone(),
                    actual_action,
                    element.rect,
                    clip_rect,
                );

                self.element_to_hint
                    .insert(element.id.clone(), label.clone());
                self.hints.push(hint);
            }
        }

        tracing::info!(
            "generate_hints: Created {} hints for {} actionable elements",
            self.hints.len(),
            actionable_elements.len()
        );
        self.update_hint_visibility();
    }

    /// Calculate the position for a hint based on element rectangle and index
    fn calculate_hint_position(&self, element_rect: &egui::Rect, index: usize) -> egui::Pos2 {
        // Place hint at top-left corner of element with small offset
        let position = egui::Pos2::new(element_rect.min.x + 2.0, element_rect.min.y + 2.0);

        // Debug logging for position verification (only for first few hints to avoid spam)
        if index < 3 {
            tracing::debug!(
                "calculate_hint_position[{}]: element_rect={:?} -> hint_pos={:?}",
                index,
                element_rect,
                position
            );
        }

        position
    }

    /// Update visibility of hints based on current filter
    fn update_hint_visibility(&mut self) {
        let mut visible_count = 0;
        let mut highlighted_count = 0;

        for hint in &mut self.hints {
            let was_visible = hint.visible;
            let was_highlighted = hint.highlighted;

            hint.visible = hint.matches_filter(&self.filter);
            hint.highlighted = hint.is_exact_match(&self.filter);

            if hint.visible {
                visible_count += 1;
            }
            if hint.highlighted {
                highlighted_count += 1;
            }

            // Log visibility changes for debugging
            if was_visible != hint.visible || was_highlighted != hint.highlighted {
                tracing::trace!("update_hint_visibility: hint '{}' element_id='{}' visible: {} -> {}, highlighted: {} -> {}",
                               hint.label, hint.element_id, was_visible, hint.visible, was_highlighted, hint.highlighted);
            }
        }

        tracing::debug!("update_hint_visibility: Filter '{}' results: {} visible, {} highlighted out of {} total hints",
                       self.filter, visible_count, highlighted_count, self.hints.len());
    }

    /// Get the hint generator for configuration
    pub fn generator(&self) -> &HintGenerator {
        &self.generator
    }

    /// Get mutable hint generator for configuration
    pub fn generator_mut(&mut self) -> &mut HintGenerator {
        &mut self.generator
    }
}

impl Default for HintMode {
    fn default() -> Self {
        Self::new()
    }
}

/// Overlay for rendering hints on top of the UI
#[derive(Debug)]
pub struct HintOverlay {
    /// Font to use for hint rendering
    font_id: egui::FontId,
}

impl HintOverlay {
    /// Create a new hint overlay
    pub fn new() -> Self {
        Self {
            font_id: egui::FontId::monospace(10.0),
        }
    }

    /// Render hints on the given UI context
    pub fn render(&mut self, ui: &mut egui::Ui, hint_mode: &mut HintMode) {
        if !hint_mode.is_active() {
            return;
        }

        // Hint mode is active - render hints directly without overlay message

        let visible_hints = hint_mode.visible_hints();

        // Only log once per hint mode session, not every frame
        // Detailed logging happens in hint_mode.start() and generate_hints()

        // Render all visible hints, but check viewport visibility first
        let mut rendered_hints = 0;
        let mut clipped_hints = 0;

        for hint in visible_hints.iter() {
            if self.is_hint_in_viewport(ui, hint) {
                self.render_hint(ui, hint, hint_mode.generator().config());
                rendered_hints += 1;
            } else {
                clipped_hints += 1;
            }
        }

        // Debug logging for visibility filtering
        if rendered_hints > 0 || clipped_hints > 0 {
            tracing::debug!(
                "HintOverlay: Rendered {} hints, clipped {} hints outside viewport",
                rendered_hints,
                clipped_hints
            );
        }

        // Show filter status
        if !hint_mode.current_filter().is_empty() {
            self.render_filter_status(ui, hint_mode);
        }
    }

    /// Render a single hint marker
    fn render_hint(&self, ui: &mut egui::Ui, hint: &HintMarker, config: &HintConfig) {
        let painter = ui.painter();

        // Calculate adaptive font size based on available space
        let adaptive_font_size = self.calculate_adaptive_font_size(hint, config);
        let adaptive_font_id = egui::FontId::monospace(adaptive_font_size);

        // Calculate text size with adaptive font
        let text_galley =
            painter.layout_no_wrap(hint.label.clone(), adaptive_font_id, config.text_color);

        // Calculate hint rectangle with padding
        let text_size = text_galley.size();
        let hint_size = egui::Vec2::new(
            text_size.x + config.padding * 2.0,
            text_size.y + config.padding * 2.0,
        );

        // Position hint rectangle at top-left alignment
        let hint_rect = egui::Rect::from_min_size(hint.position, hint_size);

        // Choose colors based on highlight state
        let bg_color = if hint.highlighted {
            egui::Color32::from_rgb(255, 100, 100) // Red highlight for exact match
        } else {
            config.background_color
        };

        // Draw background
        painter.rect_filled(hint_rect, 2.0, bg_color);

        // Draw border using lines instead of rect_stroke
        let stroke = egui::Stroke::new(config.border_width, config.border_color);

        // Draw border by drawing four lines
        painter.line_segment([hint_rect.left_top(), hint_rect.right_top()], stroke);
        painter.line_segment([hint_rect.left_bottom(), hint_rect.right_bottom()], stroke);
        painter.line_segment([hint_rect.left_top(), hint_rect.left_bottom()], stroke);
        painter.line_segment([hint_rect.right_top(), hint_rect.right_bottom()], stroke);

        // Draw text at top-left of hint rectangle with padding
        let text_pos = egui::Pos2::new(
            hint_rect.min.x + config.padding,
            hint_rect.min.y + config.padding,
        );
        painter.galley(text_pos, text_galley, config.text_color);
    }

    /// Check if a hint should be visible in the current viewport
    fn is_hint_in_viewport(&self, ui: &egui::Ui, hint: &HintMarker) -> bool {
        // Use the saved clipping rectangle from when the hint was generated
        // This preserves the original scroll area context even though we're rendering in a global overlay
        let effective_clip_rect = if hint.clip_rect == egui::Rect::EVERYTHING {
            // Fallback to current UI clip rect if no specific clipping was saved
            ui.clip_rect()
        } else {
            hint.clip_rect
        };

        // Check if the original element intersects with the effective clipping area
        let element_visible = effective_clip_rect.intersects(hint.element_rect);

        // Also check that the element is reasonably within the screen bounds
        let screen_rect = ui.ctx().screen_rect();
        let element_on_screen = screen_rect.intersects(hint.element_rect);

        // Additional check: element should have reasonable size (not zero-sized)
        let element_has_size = hint.element_rect.width() > 0.0 && hint.element_rect.height() > 0.0;

        // Element must be visible in effective clip area, on screen, and have valid size
        let is_visible = element_visible && element_on_screen && element_has_size;

        // Debug logging for the first few hints to understand clipping behavior
        if hint.element_id.contains("_0")
            || hint.element_id.contains("_1")
            || hint.element_id.contains("_2")
        {
            tracing::trace!("is_hint_in_viewport: hint '{}' element_rect={:?} effective_clip_rect={:?} saved_clip_rect={:?} -> element_visible={} element_on_screen={} element_has_size={} -> visible={}",
                           hint.label, hint.element_rect, effective_clip_rect, hint.clip_rect, element_visible, element_on_screen, element_has_size, is_visible);
        }

        is_visible
    }

    /// Render filter status
    fn render_filter_status(&self, ui: &mut egui::Ui, hint_mode: &HintMode) {
        let screen_rect = ui.max_rect();
        let status_text = format!("Filter: {}", hint_mode.current_filter());

        // Show filter in bottom-left corner
        let text_galley =
            ui.painter()
                .layout_no_wrap(status_text, self.font_id.clone(), egui::Color32::WHITE);

        let status_pos = egui::Pos2::new(
            screen_rect.min.x + 10.0,
            screen_rect.max.y - text_galley.size().y - 10.0,
        );

        // Draw background for status
        let status_rect = egui::Rect::from_min_size(
            egui::Pos2::new(status_pos.x - 5.0, status_pos.y - 2.0),
            egui::Vec2::new(text_galley.size().x + 10.0, text_galley.size().y + 4.0),
        );

        ui.painter().rect_filled(
            status_rect,
            2.0,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
        );

        ui.painter()
            .galley(status_pos, text_galley, egui::Color32::WHITE);
    }

    /// Calculate adaptive font size based on widget size and proximity to other hints
    fn calculate_adaptive_font_size(&self, hint: &HintMarker, config: &HintConfig) -> f32 {
        // For now, we'll use the associated element's rect if available to determine size
        // This is a placeholder - in the real implementation, we'd need access to the
        // original widget size and nearby hints

        // Start with regular font size
        let mut font_size = config.font_size;

        // If we have access to the element rectangle (this would come from the NavigableElement)
        // For now, we'll estimate based on hint position and use some heuristics

        // Simple heuristic: if the hint label is long, consider using smaller font
        let label_length = hint.label.len();
        if label_length > 2 {
            font_size = config.small_font_size;
        }

        // Ensure we never go below minimum font size
        font_size.max(config.min_font_size)
    }

    /// Set the font for hint rendering
    pub fn set_font(&mut self, font_id: egui::FontId) {
        self.font_id = font_id;
    }
}

impl Default for HintOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::dashui::keyboard_navigation::{
        KeyEventResult, NavigableElementType, NavigationCommand,
    };
    use crate::app::dashui::navigation_state::NavigationState;

    #[test]
    fn test_hint_config_default() {
        let config = HintConfig::default();
        assert!(!config.hint_chars.is_empty());
        assert_eq!(config.hint_chars[0], 'f'); // Home row starts with 'f'
        assert!(config.font_size > 0.0);
    }

    #[test]
    fn test_hint_marker_creation() {
        let marker = HintMarker::new(
            "test_hint".to_string(),
            "f".to_string(),
            egui::Pos2::new(10.0, 20.0),
            "test_element".to_string(),
            ElementAction::Click,
            egui::Rect::from_min_size(egui::Pos2::new(5.0, 15.0), egui::Vec2::new(50.0, 30.0)),
            egui::Rect::EVERYTHING,
        );

        assert_eq!(marker.id, "test_hint");
        assert_eq!(marker.label, "f");
        assert_eq!(marker.element_id, "test_element");
        assert!(marker.visible);
        assert!(!marker.highlighted);
    }

    #[test]
    fn test_hint_marker_filter_matching() {
        let marker = HintMarker::new(
            "test".to_string(),
            "fd".to_string(),
            egui::Pos2::ZERO,
            "element".to_string(),
            ElementAction::Click,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(100.0, 50.0)),
            egui::Rect::EVERYTHING,
        );

        assert!(marker.matches_filter(""));
        assert!(marker.matches_filter("f"));
        assert!(marker.matches_filter("fd"));
        assert!(!marker.matches_filter("g"));
        assert!(!marker.matches_filter("fdk"));

        assert!(!marker.is_exact_match("f"));
        assert!(marker.is_exact_match("fd"));
    }

    #[test]
    fn test_hint_generator_single_char() {
        let mut generator = HintGenerator::new();
        let labels = generator.generate_hint_labels(3);

        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0], "f");
        assert_eq!(labels[1], "j");
        assert_eq!(labels[2], "d");
    }

    #[test]
    fn test_hint_generator_multi_char() {
        let mut generator = HintGenerator::new();
        let hint_chars_count = generator.config().hint_chars.len();

        // Request more hints than available single characters
        let labels = generator.generate_hint_labels(hint_chars_count + 2);

        assert_eq!(labels.len(), hint_chars_count + 2);
        // First hints should be single characters
        assert_eq!(labels[0], "f");
        // Later hints should be multi-character
        assert!(labels[hint_chars_count].len() > 1);
    }

    #[test]
    fn test_hint_mode_lifecycle() {
        let mut hint_mode = HintMode::new();
        assert!(!hint_mode.is_active());

        // Create a test element
        let element = NavigableElement {
            id: "test_btn".to_string(),
            element_type: super::super::keyboard_navigation::NavigableElementType::Button,
            rect: egui::Rect::from_min_size(
                egui::Pos2::new(10.0, 10.0),
                egui::Vec2::new(50.0, 20.0),
            ),
            enabled: true,
            label: Some("Test Button".to_string()),
            supported_actions: vec![ElementAction::Click],
            metadata: HashMap::new(),
        };

        hint_mode.start(ElementAction::Click, &[element]);
        assert!(hint_mode.is_active());
        assert_eq!(hint_mode.current_action(), ElementAction::Click);
        assert_eq!(hint_mode.all_hints().len(), 1);

        hint_mode.stop();
        assert!(!hint_mode.is_active());
        assert_eq!(hint_mode.all_hints().len(), 0);
    }

    #[test]
    fn test_debug_hint_flow() {
        // Test to debug the complete hint flow similar to user pressing 'f'
        // Initialize tracing for this test
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_test_writer()
            .try_init();

        println!("🔍 Debugging hint system flow");

        // 1. Test navigation state key processing
        println!("1️⃣ Testing NavigationState key processing...");
        let mut nav_state = NavigationState::new();
        let ctx = egui::Context::default();

        // Simulate pressing 'f' key
        let f_key_event = egui::Event::Key {
            key: egui::Key::F,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };

        let result = nav_state.process_key_event(&f_key_event, &ctx);
        println!("   'f' key processing result: {:?}", result);

        // 2. Check if hint mode is entered
        match result {
            KeyEventResult::Command(NavigationCommand::EnterHintMode(action)) => {
                println!(
                    "   ✅ Correctly parsed 'f' key as EnterHintMode({:?})",
                    action
                );
                assert_eq!(action, ElementAction::Smart);
            }
            _ => {
                panic!("   ❌ Expected EnterHintMode command, got: {:?}", result);
            }
        }

        // 3. Test hint mode element collection
        println!("2️⃣ Testing HintMode element collection...");
        let mut hint_mode = HintMode::new();

        // Create a few test elements
        let elements = vec![
            NavigableElement {
                id: "test_button_1".to_string(),
                element_type: NavigableElementType::Button,
                rect: egui::Rect::from_min_size(
                    egui::Pos2::new(10.0, 10.0),
                    egui::Vec2::new(100.0, 30.0),
                ),
                enabled: true,
                label: Some("Test Button 1".to_string()),
                supported_actions: vec![ElementAction::Click, ElementAction::Smart],
                metadata: HashMap::new(),
            },
            NavigableElement {
                id: "test_button_2".to_string(),
                element_type: NavigableElementType::Button,
                rect: egui::Rect::from_min_size(
                    egui::Pos2::new(10.0, 50.0),
                    egui::Vec2::new(100.0, 30.0),
                ),
                enabled: true,
                label: Some("Test Button 2".to_string()),
                supported_actions: vec![ElementAction::Click, ElementAction::Smart],
                metadata: HashMap::new(),
            },
        ];

        hint_mode.start(ElementAction::Smart, &elements);

        println!("   Elements provided: {}", elements.len());
        println!("   Hints generated: {}", hint_mode.all_hints().len());
        println!("   Visible hints: {}", hint_mode.visible_hints().len());
        println!("   Hint mode active: {}", hint_mode.is_active());

        assert!(hint_mode.is_active(), "Hint mode should be active");
        assert_eq!(
            hint_mode.all_hints().len(),
            2,
            "Should generate 2 hints for 2 elements"
        );
        assert_eq!(
            hint_mode.visible_hints().len(),
            2,
            "Both hints should be visible"
        );

        if !hint_mode.all_hints().is_empty() {
            println!("   ✅ Hint generation working correctly");
            for (i, hint) in hint_mode.all_hints().iter().enumerate() {
                println!(
                    "     Hint {}: label='{}' element_id='{}' visible={}",
                    i, hint.label, hint.element_id, hint.visible
                );
            }
        }

        println!("3️⃣ Testing hint filtering...");
        // Test filtering
        hint_mode.add_filter_char('f');
        let visible_after_filter = hint_mode.visible_hints().len();
        println!(
            "   Visible hints after 'f' filter: {}",
            visible_after_filter
        );

        // The first hint should have label 'f', so it should still be visible
        assert!(
            visible_after_filter >= 1,
            "At least one hint should match 'f' filter"
        );

        println!("🎉 Hint flow debugging complete - all components work individually!");
    }

    #[test]
    fn test_hint_mode_filtering() {
        let mut hint_mode = HintMode::new();

        // Create test elements
        let elements = vec![
            NavigableElement {
                id: "btn1".to_string(),
                element_type: super::super::keyboard_navigation::NavigableElementType::Button,
                rect: egui::Rect::from_min_size(
                    egui::Pos2::new(10.0, 10.0),
                    egui::Vec2::new(50.0, 20.0),
                ),
                enabled: true,
                label: Some("Button 1".to_string()),
                supported_actions: vec![ElementAction::Click],
                metadata: HashMap::new(),
            },
            NavigableElement {
                id: "btn2".to_string(),
                element_type: super::super::keyboard_navigation::NavigableElementType::Button,
                rect: egui::Rect::from_min_size(
                    egui::Pos2::new(70.0, 10.0),
                    egui::Vec2::new(50.0, 20.0),
                ),
                enabled: true,
                label: Some("Button 2".to_string()),
                supported_actions: vec![ElementAction::Click],
                metadata: HashMap::new(),
            },
        ];

        hint_mode.start(ElementAction::Click, &elements);
        assert_eq!(hint_mode.visible_hints().len(), 2);

        // Add filter character
        hint_mode.add_filter_char('f');
        let visible_after_filter = hint_mode.visible_hints().len();
        assert!(visible_after_filter <= 2);

        // Test exact match
        hint_mode.clear_filter();
        hint_mode.add_filter_char('f');
        if hint_mode.has_exact_match().is_some() {
            assert!(hint_mode.get_exact_match_element_id().is_some());
        }
    }
}
