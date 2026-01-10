//! Window Maximize Helper
//!
//! Provides reusable soft-maximize functionality for egui windows.
//! When maximized, windows fill the available screen area minus the menu bar height.
//!
//! ## Soft-Maximize vs Hard-Maximize
//!
//! This module implements "soft-maximize" which means:
//! - Window is resized and repositioned to fill the screen (below menu bar)
//! - Window remains resizable, movable, and collapsible after maximizing
//! - User can manually adjust the window even in "maximized" state
//!
//! ## Size Forcing Mechanism
//!
//! egui remembers window sizes by ID. Simply setting `default_size()` won't resize
//! an already-open window. We use a `needs_resize` flag to force size for one frame
//! after maximize/restore is toggled. This triggers egui to apply the new size.

use egui::{Context, Pos2, Vec2};

/// Height reserved for the top menu bar (in pixels)
pub const MENU_BAR_HEIGHT: f32 = 28.0;

/// State for window maximize/restore functionality
#[derive(Debug, Clone, Default)]
pub struct WindowMaximizeState {
    /// Whether the window is currently maximized
    pub is_maximized: bool,
    /// Saved position before maximizing (for restore)
    restore_pos: Option<Pos2>,
    /// Saved size before maximizing (for restore)
    restore_size: Option<Vec2>,
    /// Target size to apply (set when maximize/restore is toggled)
    target_size: Option<Vec2>,
    /// Target position to apply (set when maximize/restore is toggled)
    target_pos: Option<Pos2>,
    /// Flag indicating size should be forced on next frame
    needs_resize: bool,
}

impl WindowMaximizeState {
    /// Create a new maximize state (starts not maximized)
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle between maximized and restored state
    ///
    /// This calculates the target size at the moment of toggle based on the
    /// current application window size. The size is then forced on the next frame.
    ///
    /// When maximizing: uses screen_rect to calculate full available space
    /// When restoring: uses saved restore_pos/restore_size
    pub fn toggle(&mut self, ctx: &Context) {
        self.is_maximized = !self.is_maximized;
        self.needs_resize = true;

        if self.is_maximized {
            // Calculate maximized size based on current app window dimensions
            let screen_rect = ctx.screen_rect();
            self.target_pos = Some(Pos2::new(0.0, MENU_BAR_HEIGHT));
            self.target_size = Some(Vec2::new(
                screen_rect.width(),
                screen_rect.height() - MENU_BAR_HEIGHT,
            ));
        } else {
            // Restore to saved position/size
            self.target_pos = self.restore_pos;
            self.target_size = self.restore_size;
        }
    }

    /// Check if the window needs to be resized this frame
    pub fn needs_resize(&self) -> bool {
        self.needs_resize
    }

    /// Clear the resize flag after applying the size
    pub fn clear_resize_flag(&mut self) {
        self.needs_resize = false;
    }

    /// Get the target size if set (for forced resize)
    pub fn target_size(&self) -> Option<Vec2> {
        self.target_size
    }

    /// Get the target position if set (for forced resize)
    pub fn target_pos(&self) -> Option<Pos2> {
        self.target_pos
    }

    /// Get the maximize button label
    pub fn button_label(&self) -> &'static str {
        if self.is_maximized {
            // Restore icon (two overlapping squares) - using ASCII
            "[_]"
        } else {
            // Maximize icon (single square) - using ASCII
            "[ ]"
        }
    }

    /// Get tooltip for the maximize button
    pub fn button_tooltip(&self) -> &'static str {
        if self.is_maximized {
            "Restore window"
        } else {
            "Maximize window"
        }
    }

    /// Save current window position and size before maximizing
    pub fn save_restore_state(&mut self, pos: Pos2, size: Vec2) {
        self.restore_pos = Some(pos);
        self.restore_size = Some(size);
    }

    /// Get the window configuration based on maximize state
    ///
    /// Returns (position, size, should_set_pos) tuple.
    /// - When maximized: fills screen below menu bar
    /// - When restored: uses saved position/size or defaults
    pub fn get_window_config(
        &self,
        ctx: &Context,
        default_size: Vec2,
    ) -> (Option<Pos2>, Vec2, bool) {
        let screen_rect = ctx.screen_rect();

        if self.is_maximized {
            // Maximized: fill screen below menu bar
            let pos = Pos2::new(0.0, MENU_BAR_HEIGHT);
            let size = Vec2::new(screen_rect.width(), screen_rect.height() - MENU_BAR_HEIGHT);
            (Some(pos), size, true)
        } else {
            // Restored: use saved state or defaults
            let size = self.restore_size.unwrap_or(default_size);
            // Don't force position when not maximized - let egui handle it
            // unless we have a saved restore position
            (self.restore_pos, size, self.restore_pos.is_some())
        }
    }

    /// Calculate maximized size for the current screen
    pub fn maximized_size(ctx: &Context) -> Vec2 {
        let screen_rect = ctx.screen_rect();
        Vec2::new(screen_rect.width(), screen_rect.height() - MENU_BAR_HEIGHT)
    }

    /// Calculate maximized position (top-left, below menu)
    pub fn maximized_pos() -> Pos2 {
        Pos2::new(0.0, MENU_BAR_HEIGHT)
    }
}

/// Render a maximize/restore button
///
/// Returns true if the button was clicked.
/// Place this in a horizontal layout at the top-right of your window content.
pub fn maximize_button(ui: &mut egui::Ui, state: &WindowMaximizeState) -> bool {
    ui.add(egui::Button::new(state.button_label()).small())
        .on_hover_text(state.button_tooltip())
        .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = WindowMaximizeState::new();
        assert!(!state.is_maximized);
        assert!(state.restore_pos.is_none());
        assert!(state.restore_size.is_none());
        assert!(!state.needs_resize);
    }

    #[test]
    fn test_maximize_state_fields() {
        let mut state = WindowMaximizeState::new();
        assert!(!state.is_maximized);

        // Manually toggle (toggle() requires Context in real use)
        state.is_maximized = true;
        state.needs_resize = true;
        state.target_size = Some(Vec2::new(1920.0, 1052.0));
        state.target_pos = Some(Pos2::new(0.0, MENU_BAR_HEIGHT));

        assert!(state.is_maximized);
        assert!(state.needs_resize());
        assert_eq!(state.target_size(), Some(Vec2::new(1920.0, 1052.0)));
        assert_eq!(state.target_pos(), Some(Pos2::new(0.0, MENU_BAR_HEIGHT)));

        // Clear resize flag
        state.clear_resize_flag();
        assert!(!state.needs_resize());
    }

    #[test]
    fn test_button_labels() {
        let mut state = WindowMaximizeState::new();
        assert_eq!(state.button_label(), "[ ]");
        assert_eq!(state.button_tooltip(), "Maximize window");

        // Manually set maximized
        state.is_maximized = true;
        assert_eq!(state.button_label(), "[_]");
        assert_eq!(state.button_tooltip(), "Restore window");
    }

    #[test]
    fn test_save_restore_state() {
        let mut state = WindowMaximizeState::new();
        let pos = Pos2::new(100.0, 200.0);
        let size = Vec2::new(800.0, 600.0);

        state.save_restore_state(pos, size);

        assert_eq!(state.restore_pos, Some(pos));
        assert_eq!(state.restore_size, Some(size));
    }
}
