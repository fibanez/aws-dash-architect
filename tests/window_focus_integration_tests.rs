//! Integration test for window focus functionality
//!
//! This test verifies that the FocusableWindow trait implementations work correctly
//! and that the WindowFocusManager integrates properly with the application.

#[cfg(test)]
mod window_focus_integration_tests {
    use awsdash::app::dashui::{CredentialsDebugWindow, HelpWindow, LogWindow, VerificationWindow};
    use awsdash::app::dashui::{FocusableWindow, WindowFocusManager};

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_help_window_trait_implementation() {
        let mut help_window = HelpWindow::new();
        help_window.open = true;

        // Test trait methods
        assert_eq!(help_window.window_id(), "help_window");
        assert_eq!(help_window.window_title(), "Help");
        assert!(help_window.is_open());

        // Note: We can't easily test show_with_focus without a full egui context
        // but we can verify the trait is implemented correctly
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_log_window_trait_implementation() {
        let mut log_window = LogWindow::new();
        log_window.open = true;

        // Test trait methods
        assert_eq!(log_window.window_id(), "log_window");
        assert_eq!(log_window.window_title(), "Log Viewer");
        assert!(log_window.is_open());
    }

    #[test]
    fn test_credentials_debug_window_trait_implementation() {
        let creds_window = CredentialsDebugWindow { open: true };

        // Test trait methods
        assert_eq!(creds_window.window_id(), "credentials_debug");
        assert_eq!(creds_window.window_title(), "AWS Credentials Debug");
        assert!(creds_window.is_open());
    }

    #[test]
    fn test_verification_window_trait_implementation() {
        let mut verification_window = VerificationWindow::default();
        verification_window.visible = true;

        // Test trait methods
        assert_eq!(verification_window.window_id(), "verification_window");
        assert_eq!(
            verification_window.window_title(),
            "CloudFormation Import Verification Results"
        );
        assert!(verification_window.is_open());
    }

    #[test]
    fn test_window_focus_manager_integration() {
        let mut focus_manager = WindowFocusManager::new();

        // Test focus requests for different windows
        focus_manager.request_focus("help_window".to_string());
        assert!(focus_manager.should_bring_to_front("help_window"));
        assert!(!focus_manager.should_bring_to_front("log_window"));

        focus_manager.clear_bring_to_front("help_window");
        assert!(!focus_manager.should_bring_to_front("help_window"));

        // Test switching focus between windows
        focus_manager.request_focus("log_window".to_string());
        assert!(focus_manager.should_bring_to_front("log_window"));
        assert!(!focus_manager.should_bring_to_front("help_window"));

        focus_manager.clear_bring_to_front("log_window");
        assert!(!focus_manager.should_bring_to_front("log_window"));
    }

    #[test]
    fn test_window_trait_consistency() {
        // Test that all windows implement the trait consistently
        let help_window = HelpWindow::new();
        let log_window = LogWindow::new();
        let creds_window = CredentialsDebugWindow::default();
        let verification_window = VerificationWindow::default();

        // All windows should have unique IDs
        let mut window_ids = std::collections::HashSet::new();
        window_ids.insert(help_window.window_id());
        window_ids.insert(log_window.window_id());
        window_ids.insert(creds_window.window_id());
        window_ids.insert(verification_window.window_id());

        assert_eq!(window_ids.len(), 4, "All windows should have unique IDs");

        // All windows should have non-empty titles
        assert!(!help_window.window_title().is_empty());
        assert!(!log_window.window_title().is_empty());
        assert!(!creds_window.window_title().is_empty());
        assert!(!verification_window.window_title().is_empty());
    }
}
