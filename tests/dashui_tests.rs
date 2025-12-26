#[cfg(test)]
mod tests {
    use awsdash::app::dashui::app::{DashApp, FocusedWindow, ThemeChoice};

    #[test]
    fn test_dashapp_default() {
        let app = DashApp::default();

        // Check default theme (using match since ThemeChoice doesn't have Debug)
        assert!(matches!(app.theme, ThemeChoice::Latte));

        // Check all windows are properly initialized
        assert!(!app.show_command_palette);
        assert!(app.aws_identity_center.is_none());
        assert!(app.startup_popup_timer.is_some());
        assert!(app.show_startup_popup);
    }

    #[test]
    fn test_theme_choice_default() {
        let theme = ThemeChoice::default();
        assert!(matches!(theme, ThemeChoice::Latte));
    }

    #[test]
    fn test_theme_choices() {
        // Test all theme variants
        let _latte = ThemeChoice::Latte;
        let _frappe = ThemeChoice::Frappe;
        let _macchiato = ThemeChoice::Macchiato;
        let _mocha = ThemeChoice::Mocha;

        // Test equality (using manual comparison since Debug trait not available)
        assert!(ThemeChoice::Latte == ThemeChoice::Latte);
        assert!(ThemeChoice::Latte != ThemeChoice::Mocha);
    }

    #[test]
    fn test_focused_window_equality() {
        assert_eq!(FocusedWindow::CommandPalette, FocusedWindow::CommandPalette);
        assert_ne!(FocusedWindow::CommandPalette, FocusedWindow::Help);
    }

    #[test]
    fn test_dashapp_theme_serialization() {
        let mut app = DashApp::default();
        app.theme = ThemeChoice::Mocha;

        // Test that theme is correctly set
        assert!(matches!(app.theme, ThemeChoice::Mocha));

        // Serialize
        let serialized = serde_json::to_string(&app).unwrap();

        // Deserialize
        let deserialized: DashApp = serde_json::from_str(&serialized).unwrap();

        // Check theme is preserved
        assert!(matches!(deserialized.theme, ThemeChoice::Mocha));

        // Check that skipped fields are reset to defaults
        assert!(!deserialized.show_command_palette);
        assert!(deserialized.aws_identity_center.is_none());
    }

    #[test]
    fn test_focused_window_variants() {
        // Test all existing window types
        let windows = vec![
            FocusedWindow::CommandPalette,
            FocusedWindow::AwsLogin,
            FocusedWindow::AwsAccounts,
            FocusedWindow::StartupPopup,
            FocusedWindow::Help,
            FocusedWindow::Log,
            FocusedWindow::Chat,
            FocusedWindow::AgentManager,
            FocusedWindow::Verification,
            FocusedWindow::GuardViolations,
        ];

        // Ensure we can create and compare all variants
        for (i, window) in windows.iter().enumerate() {
            for (j, other) in windows.iter().enumerate() {
                if i == j {
                    assert_eq!(window, other);
                } else {
                    assert_ne!(window, other);
                }
            }
        }
    }

    #[test]
    fn test_dashapp_startup_timer() {
        let mut app = DashApp::default();

        // Should have a startup timer initially
        assert!(app.startup_popup_timer.is_some());
        assert!(app.show_startup_popup);

        // Can disable popup
        app.show_startup_popup = false;
        assert!(!app.show_startup_popup);

        // Can clear timer
        app.startup_popup_timer = None;
        assert!(app.startup_popup_timer.is_none());
    }
}
