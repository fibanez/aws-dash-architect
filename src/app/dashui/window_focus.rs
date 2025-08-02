//! Window Focus Management System
//!
//! This module provides a trait-based system for bringing windows to the foreground
//! when selected from the window selector menu. It replaces ad-hoc focus implementations
//! with a consistent, maintainable approach.

use eframe::egui;

/// Trait for windows that can be brought to the foreground
///
/// This trait provides a consistent interface for focus behavior across all windows.
/// Windows implement this trait to support being brought to the front when selected
/// from the window selector menu.
///
/// # Example
/// ```rust
/// impl FocusableWindow for HelpWindow {
///     type ShowParams = ();
///
///     fn window_id(&self) -> &'static str { "help" }
///     fn window_title(&self) -> String { "Help".to_string() }
///     fn is_open(&self) -> bool { self.show }
///
///     fn show_with_focus(&mut self, ctx: &egui::Context, _params: (), bring_to_front: bool) {
///         let mut window = egui::Window::new("Help")
///             .resizable(true)
///             .default_width(400.0);
///
///         if bring_to_front {
///             window = window.order(egui::Order::Foreground);
///         }
///
///         window.show(ctx, |ui| {
///             // Window content...
///         });
///     }
/// }
/// ```
pub trait FocusableWindow {
    /// Parameters required for the show method
    ///
    /// Different windows require different parameters:
    /// - Simple windows: `()`
    /// - Position-based windows: `egui::Pos2`
    /// - Complex windows: Custom parameter structs
    type ShowParams;

    /// Unique identifier for this window type
    ///
    /// This should match the window ID used in the window selector menu.
    /// Must be unique across all windows in the application.
    fn window_id(&self) -> &'static str;

    /// Human-readable title for this window
    ///
    /// This should match exactly what appears in the window's title bar
    /// and in the window selector menu for consistency.
    fn window_title(&self) -> String;

    /// Whether this window is currently open/visible
    ///
    /// Used to determine if the window should be shown in the selector menu
    /// and for state tracking.
    fn is_open(&self) -> bool;

    /// Show the window with optional focus behavior
    ///
    /// This is the main method that renders the window. When `bring_to_front` is true,
    /// the window should be displayed with `egui::Order::Foreground` to ensure it
    /// appears above other windows.
    ///
    /// # Parameters
    /// - `ctx`: egui context for rendering
    /// - `params`: Window-specific parameters (defined by `ShowParams`)
    /// - `bring_to_front`: Whether to bring window to foreground
    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    );
}

/// Manages focus requests and window ordering
///
/// This struct centralizes the logic for bringing windows to the front when selected
/// from the window selector menu. It tracks focus requests and provides utilities
/// for applying focus ordering to egui windows.
pub struct WindowFocusManager {
    /// The ID of the window that should be brought to the front on the next frame
    bring_to_front_window: Option<String>,
}

impl WindowFocusManager {
    /// Create a new window focus manager
    pub fn new() -> Self {
        Self {
            bring_to_front_window: None,
        }
    }

    /// Request that a window be brought to the front
    ///
    /// This sets a flag that will be checked by window handlers on the next frame.
    /// The window with the matching ID will be displayed with foreground ordering.
    ///
    /// # Parameters
    /// - `window_id`: The unique identifier of the window to focus
    pub fn request_focus(&mut self, window_id: String) {
        self.bring_to_front_window = Some(window_id);
    }

    /// Check if a specific window should be brought to the front
    ///
    /// Window handlers should call this method to determine if they should
    /// apply foreground ordering to their window.
    ///
    /// # Parameters
    /// - `window_id`: The unique identifier of the window to check
    ///
    /// # Returns
    /// `true` if this window should be brought to the front
    pub fn should_bring_to_front(&self, window_id: &str) -> bool {
        self.bring_to_front_window.as_ref() == Some(&window_id.to_string())
    }

    /// Clear the focus request for a specific window
    ///
    /// This should be called by window handlers after they have processed
    /// the focus request to prevent the window from staying in foreground
    /// mode indefinitely.
    ///
    /// # Parameters
    /// - `window_id`: The unique identifier of the window that processed the focus
    pub fn clear_bring_to_front(&mut self, window_id: &str) {
        if self.should_bring_to_front(window_id) {
            self.bring_to_front_window = None;
        }
    }

    /// Apply focus ordering to an egui window
    ///
    /// This is a utility method that applies `egui::Order::Foreground` to a window
    /// when it should be brought to the front. This ensures consistent behavior
    /// across all windows.
    ///
    /// # Parameters
    /// - `window`: The egui window to potentially modify
    /// - `bring_to_front`: Whether to apply foreground ordering
    ///
    /// # Returns
    /// The window with or without foreground ordering applied
    pub fn apply_focus_order(window: egui::Window<'_>, bring_to_front: bool) -> egui::Window<'_> {
        if bring_to_front {
            window.order(egui::Order::Foreground)
        } else {
            window
        }
    }
}

impl Default for WindowFocusManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Common parameter types for different window patterns
///
/// These types provide standardized parameter patterns for common window types,
/// reducing boilerplate and ensuring consistency.
///
/// Parameters for simple windows that don't need additional data
///
/// Simple windows are those that can be shown without any additional context
/// or parameters. They typically just need to know whether to bring themselves
/// to the front. Examples include Help, Log, Credentials Debug, and Verification windows.
pub type SimpleShowParams = ();

/// Helper macro for implementing FocusableWindow trait for simple windows
///
/// This macro generates a standard implementation for windows that:
/// - Use `SimpleShowParams` (no additional parameters)
/// - Have a boolean field indicating if they're open/visible
/// - Use a standard egui::Window for display
///
/// # Usage
/// ```rust
/// impl_simple_focusable_window!(
///     HelpWindow,           // Window struct name
///     "help_window",        // Window ID (must match focus_window switch)
///     "Help",              // Window title
///     open,                // Boolean field name for visibility
///     show                 // Method name that takes (ctx, bring_to_front)
/// );
/// ```
#[allow(unused_macros)]
macro_rules! impl_simple_focusable_window {
    ($window_type:ty, $window_id:expr, $window_title:expr, $open_field:ident, $show_method:ident) => {
        impl crate::app::dashui::window_focus::FocusableWindow for $window_type {
            type ShowParams = crate::app::dashui::window_focus::SimpleShowParams;

            fn window_id(&self) -> &'static str {
                $window_id
            }

            fn window_title(&self) -> String {
                $window_title.to_string()
            }

            fn is_open(&self) -> bool {
                self.$open_field
            }

            fn show_with_focus(
                &mut self,
                ctx: &eframe::egui::Context,
                _params: Self::ShowParams,
                bring_to_front: bool,
            ) {
                self.$show_method(ctx, bring_to_front);
            }
        }
    };
}

#[allow(unused_imports)]
pub(crate) use impl_simple_focusable_window;

/// Parameters for windows that need positioning information
pub type PositionShowParams = egui::Pos2;

/// Parameters for windows that need project context
#[derive(Clone)]
pub struct ProjectShowParams {
    pub project: Option<crate::app::projects::Project>,
    pub window_pos: Option<egui::Pos2>,
}

/// Parameters for windows that need theme information
#[derive(Clone)]
pub struct ThemeShowParams {
    pub theme: String,
}

/// Parameters for windows that need AWS identity context
#[derive(Clone)]
pub struct IdentityShowParams {
    pub aws_identity:
        Option<std::sync::Arc<std::sync::Mutex<crate::app::aws_identity::AwsIdentityCenter>>>,
}

/// Parameters for resource form windows
#[derive(Clone)]
pub struct FormShowParams {
    // Currently empty - form windows manage their own state
    // This is a placeholder for potential future form parameters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_manager_creation() {
        let manager = WindowFocusManager::new();
        assert!(manager.bring_to_front_window.is_none());
    }

    #[test]
    fn test_focus_request_and_check() {
        let mut manager = WindowFocusManager::new();

        // Initially no window should be focused
        assert!(!manager.should_bring_to_front("help"));

        // Request focus for help window
        manager.request_focus("help".to_string());
        assert!(manager.should_bring_to_front("help"));
        assert!(!manager.should_bring_to_front("log"));
    }

    #[test]
    fn test_focus_clear() {
        let mut manager = WindowFocusManager::new();

        // Request and then clear focus
        manager.request_focus("help".to_string());
        assert!(manager.should_bring_to_front("help"));

        manager.clear_bring_to_front("help");
        assert!(!manager.should_bring_to_front("help"));
    }

    #[test]
    fn test_clear_wrong_window() {
        let mut manager = WindowFocusManager::new();

        // Request focus for one window, try to clear another
        manager.request_focus("help".to_string());
        manager.clear_bring_to_front("log");

        // Help should still be focused
        assert!(manager.should_bring_to_front("help"));
    }

    #[test]
    fn test_apply_focus_order() {
        let window = egui::Window::new("Test");

        // Without focus
        let _window_no_focus = WindowFocusManager::apply_focus_order(window, false);
        // Note: We can't easily test the actual order since it's internal to egui
        // But we can verify the method doesn't panic

        let window = egui::Window::new("Test");

        // With focus
        let _window_with_focus = WindowFocusManager::apply_focus_order(window, true);
        // Method should complete without panicking
    }

    #[test]
    fn test_default_manager() {
        let manager = WindowFocusManager::default();
        assert!(manager.bring_to_front_window.is_none());
    }

    #[test]
    fn test_multiple_focus_requests() {
        let mut manager = WindowFocusManager::new();

        // Request focus for first window
        manager.request_focus("window1".to_string());
        assert!(manager.should_bring_to_front("window1"));
        assert!(!manager.should_bring_to_front("window2"));

        // Request focus for second window (should replace first)
        manager.request_focus("window2".to_string());
        assert!(!manager.should_bring_to_front("window1"));
        assert!(manager.should_bring_to_front("window2"));

        // Request focus for third window
        manager.request_focus("window3".to_string());
        assert!(!manager.should_bring_to_front("window1"));
        assert!(!manager.should_bring_to_front("window2"));
        assert!(manager.should_bring_to_front("window3"));
    }

    #[test]
    fn test_clear_focus_after_multiple_requests() {
        let mut manager = WindowFocusManager::new();

        // Request focus for multiple windows
        manager.request_focus("window1".to_string());
        manager.request_focus("window2".to_string());
        manager.request_focus("window3".to_string());

        // Only the last one should be focused
        assert!(manager.should_bring_to_front("window3"));

        // Clear the focused window
        manager.clear_bring_to_front("window3");
        assert!(!manager.should_bring_to_front("window3"));

        // Previous windows should not be focused either
        assert!(!manager.should_bring_to_front("window1"));
        assert!(!manager.should_bring_to_front("window2"));
    }

    #[test]
    fn test_empty_window_id() {
        let mut manager = WindowFocusManager::new();

        // Test with empty string
        manager.request_focus("".to_string());
        assert!(manager.should_bring_to_front(""));
        assert!(!manager.should_bring_to_front("non_empty"));

        // Clear empty string
        manager.clear_bring_to_front("");
        assert!(!manager.should_bring_to_front(""));
    }

    #[test]
    fn test_case_sensitive_window_ids() {
        let mut manager = WindowFocusManager::new();

        // Request focus with lowercase
        manager.request_focus("window".to_string());
        assert!(manager.should_bring_to_front("window"));
        assert!(!manager.should_bring_to_front("WINDOW"));
        assert!(!manager.should_bring_to_front("Window"));

        // Clear with different case should not work
        manager.clear_bring_to_front("WINDOW");
        assert!(manager.should_bring_to_front("window")); // Should still be focused

        // Clear with correct case
        manager.clear_bring_to_front("window");
        assert!(!manager.should_bring_to_front("window"));
    }

    #[test]
    fn test_unicode_window_ids() {
        let mut manager = WindowFocusManager::new();

        // Test with unicode characters
        let unicode_id = "窗口测试🪟";
        manager.request_focus(unicode_id.to_string());
        assert!(manager.should_bring_to_front(unicode_id));

        manager.clear_bring_to_front(unicode_id);
        assert!(!manager.should_bring_to_front(unicode_id));
    }

    #[test]
    fn test_very_long_window_id() {
        let mut manager = WindowFocusManager::new();

        // Test with very long window ID
        let long_id = "a".repeat(1000);
        manager.request_focus(long_id.clone());
        assert!(manager.should_bring_to_front(&long_id));

        manager.clear_bring_to_front(&long_id);
        assert!(!manager.should_bring_to_front(&long_id));
    }

    #[test]
    fn test_focus_request_idempotency() {
        let mut manager = WindowFocusManager::new();

        // Request focus multiple times for same window
        manager.request_focus("window".to_string());
        assert!(manager.should_bring_to_front("window"));

        manager.request_focus("window".to_string());
        assert!(manager.should_bring_to_front("window")); // Should still be focused

        manager.request_focus("window".to_string());
        assert!(manager.should_bring_to_front("window")); // Should still be focused

        // Clear once should remove focus
        manager.clear_bring_to_front("window");
        assert!(!manager.should_bring_to_front("window"));
    }

    #[test]
    fn test_clear_non_existent_window() {
        let mut manager = WindowFocusManager::new();

        // Try to clear focus for window that was never focused
        manager.clear_bring_to_front("non_existent");
        assert!(!manager.should_bring_to_front("non_existent"));

        // Focus a window, then try to clear different window
        manager.request_focus("actual_window".to_string());
        manager.clear_bring_to_front("non_existent");
        assert!(manager.should_bring_to_front("actual_window")); // Should still be focused
    }

    #[test]
    fn test_should_bring_to_front_with_no_focus_request() {
        let manager = WindowFocusManager::new();

        // Test various window IDs when no focus has been requested
        assert!(!manager.should_bring_to_front("help"));
        assert!(!manager.should_bring_to_front("log"));
        assert!(!manager.should_bring_to_front(""));
        assert!(!manager.should_bring_to_front("non_existent"));
    }

    #[test]
    fn test_apply_focus_order_with_different_window_types() {
        // Test with different window configurations
        let window1 = egui::Window::new("Simple Window");
        let _result1 = WindowFocusManager::apply_focus_order(window1, true);

        let window2 = egui::Window::new("Resizable Window").resizable(true);
        let _result2 = WindowFocusManager::apply_focus_order(window2, false);

        let window3 = egui::Window::new("Collapsible Window").collapsible(true);
        let _result3 = WindowFocusManager::apply_focus_order(window3, true);

        // Test with window that has multiple properties
        let window4 = egui::Window::new("Complex Window")
            .resizable(true)
            .collapsible(true)
            .default_width(400.0)
            .default_height(300.0);
        let _result4 = WindowFocusManager::apply_focus_order(window4, false);
    }

    // Test helper struct to test trait implementation
    struct MockWindow {
        id: &'static str,
        title: String,
        open: bool,
        last_bring_to_front: bool,
    }

    impl MockWindow {
        fn new(id: &'static str, title: &str) -> Self {
            Self {
                id,
                title: title.to_string(),
                open: false,
                last_bring_to_front: false,
            }
        }
    }

    impl FocusableWindow for MockWindow {
        type ShowParams = SimpleShowParams;

        fn window_id(&self) -> &'static str {
            self.id
        }

        fn window_title(&self) -> String {
            self.title.clone()
        }

        fn is_open(&self) -> bool {
            self.open
        }

        fn show_with_focus(
            &mut self,
            _ctx: &egui::Context,
            _params: Self::ShowParams,
            bring_to_front: bool,
        ) {
            self.last_bring_to_front = bring_to_front;
        }
    }

    #[test]
    fn test_focusable_window_trait_basic() {
        let mut window = MockWindow::new("test_window", "Test Window");

        // Test basic trait methods
        assert_eq!(window.window_id(), "test_window");
        assert_eq!(window.window_title(), "Test Window");
        assert!(!window.is_open());

        // Open window and test
        window.open = true;
        assert!(window.is_open());
    }

    #[test]
    fn test_focusable_window_trait_with_focus_manager() {
        let mut manager = WindowFocusManager::new();
        let mut window = MockWindow::new("test_window", "Test Window");
        window.open = true;

        // Test initial state
        assert!(!manager.should_bring_to_front(window.window_id()));

        // Request focus
        manager.request_focus(window.window_id().to_string());
        assert!(manager.should_bring_to_front(window.window_id()));

        // Clear focus
        manager.clear_bring_to_front(window.window_id());
        assert!(!manager.should_bring_to_front(window.window_id()));
    }
}
