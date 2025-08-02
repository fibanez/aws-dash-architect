//! Parameter passing tests for the window focus system
//!
//! These tests verify that different parameter types are correctly passed through
//! the FocusableWindow trait to window implementations.

use awsdash::app::dashui::window_focus::{
    FocusableWindow, PositionShowParams, ProjectShowParams, SimpleShowParams, ThemeShowParams,
};
use awsdash::app::projects::Project;

/// Mock window for testing simple parameters
struct MockSimpleWindow {
    pub show: bool,
    pub last_params: Option<SimpleShowParams>,
    pub last_bring_to_front: bool,
}

impl MockSimpleWindow {
    fn new() -> Self {
        Self {
            show: false,
            last_params: None,
            last_bring_to_front: false,
        }
    }
}

impl FocusableWindow for MockSimpleWindow {
    type ShowParams = SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "mock_simple"
    }

    fn window_title(&self) -> String {
        "Mock Simple Window".to_string()
    }

    fn is_open(&self) -> bool {
        self.show
    }

    fn show_with_focus(
        &mut self,
        _ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        self.last_params = Some(params);
        self.last_bring_to_front = bring_to_front;
    }
}

/// Mock window for testing project parameters
struct MockProjectWindow {
    pub show: bool,
    pub last_project: Option<Project>,
    pub last_window_pos: Option<egui::Pos2>,
    pub last_bring_to_front: bool,
}

impl MockProjectWindow {
    fn new() -> Self {
        Self {
            show: false,
            last_project: None,
            last_window_pos: None,
            last_bring_to_front: false,
        }
    }
}

impl FocusableWindow for MockProjectWindow {
    type ShowParams = ProjectShowParams;

    fn window_id(&self) -> &'static str {
        "mock_project"
    }

    fn window_title(&self) -> String {
        "Mock Project Window".to_string()
    }

    fn is_open(&self) -> bool {
        self.show
    }

    fn show_with_focus(
        &mut self,
        _ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        self.last_project = params.project;
        self.last_window_pos = params.window_pos;
        self.last_bring_to_front = bring_to_front;
    }
}

/// Mock window for testing position parameters
struct MockPositionWindow {
    pub show: bool,
    pub last_position: Option<egui::Pos2>,
    pub last_bring_to_front: bool,
}

impl MockPositionWindow {
    fn new() -> Self {
        Self {
            show: false,
            last_position: None,
            last_bring_to_front: false,
        }
    }
}

impl FocusableWindow for MockPositionWindow {
    type ShowParams = PositionShowParams;

    fn window_id(&self) -> &'static str {
        "mock_position"
    }

    fn window_title(&self) -> String {
        "Mock Position Window".to_string()
    }

    fn is_open(&self) -> bool {
        self.show
    }

    fn show_with_focus(
        &mut self,
        _ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        self.last_position = Some(params);
        self.last_bring_to_front = bring_to_front;
    }
}

/// Mock window for testing theme parameters
struct MockThemeWindow {
    pub show: bool,
    pub last_theme: Option<String>,
    pub last_bring_to_front: bool,
}

impl MockThemeWindow {
    fn new() -> Self {
        Self {
            show: false,
            last_theme: None,
            last_bring_to_front: false,
        }
    }
}

impl FocusableWindow for MockThemeWindow {
    type ShowParams = ThemeShowParams;

    fn window_id(&self) -> &'static str {
        "mock_theme"
    }

    fn window_title(&self) -> String {
        "Mock Theme Window".to_string()
    }

    fn is_open(&self) -> bool {
        self.show
    }

    fn show_with_focus(
        &mut self,
        _ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        self.last_theme = Some(params.theme);
        self.last_bring_to_front = bring_to_front;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    #[test]
    fn test_simple_params_passing() {
        let harness = Harness::new_ui(|ui| {
            ui.label("Test UI");
        });

        let mut window = MockSimpleWindow::new();
        window.show = true;

        // Test with simple parameters (empty tuple)
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), false);

        assert!(window.last_params.is_some());
        assert!(!window.last_bring_to_front);

        // Test with bring_to_front = true
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), true);
        assert!(window.last_bring_to_front);
    }

    #[test]
    fn test_project_params_passing() {
        let harness = Harness::new_ui(|ui| {
            ui.label("Test UI");
        });

        let mut window = MockProjectWindow::new();
        window.show = true;

        // Create test project
        let project = Project::new(
            "test_project".to_string(),
            "Test project description".to_string(),
            "test".to_string(),
        );
        let position = egui::Pos2::new(100.0, 200.0);

        let params = ProjectShowParams {
            project: Some(project.clone()),
            window_pos: Some(position),
        };

        // Test parameter passing
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params, false);

        assert!(window.last_project.is_some());
        assert_eq!(window.last_project.as_ref().unwrap().name, "test_project");
        assert!(window.last_window_pos.is_some());
        assert_eq!(window.last_window_pos.unwrap(), position);
        assert!(!window.last_bring_to_front);
    }

    #[test]
    fn test_project_params_with_none_values() {
        let harness = Harness::new_ui(|ui| {
            ui.label("Test UI");
        });

        let mut window = MockProjectWindow::new();
        window.show = true;

        let params = ProjectShowParams {
            project: None,
            window_pos: None,
        };

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params, true);

        assert!(window.last_project.is_none());
        assert!(window.last_window_pos.is_none());
        assert!(window.last_bring_to_front);
    }

    #[test]
    fn test_position_params_passing() {
        let harness = Harness::new_ui(|ui| {
            ui.label("Test UI");
        });

        let mut window = MockPositionWindow::new();
        window.show = true;

        let position = egui::Pos2::new(50.0, 75.0);

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, position, false);

        assert!(window.last_position.is_some());
        assert_eq!(window.last_position.unwrap(), position);
        assert!(!window.last_bring_to_front);
    }

    #[test]
    fn test_theme_params_passing() {
        let harness = Harness::new_ui(|ui| {
            ui.label("Test UI");
        });

        let mut window = MockThemeWindow::new();
        window.show = true;

        let params = ThemeShowParams {
            theme: "Mocha".to_string(),
        };

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params, true);

        assert!(window.last_theme.is_some());
        assert_eq!(window.last_theme.as_ref().unwrap(), "Mocha");
        assert!(window.last_bring_to_front);
    }

    #[test]
    fn test_theme_params_with_different_themes() {
        let harness = Harness::new_ui(|ui| {
            ui.label("Test UI");
        });

        let mut window = MockThemeWindow::new();
        window.show = true;

        let themes = ["Latte", "Frappe", "Macchiato", "Mocha"];

        for theme in &themes {
            let params = ThemeShowParams {
                theme: theme.to_string(),
            };

            FocusableWindow::show_with_focus(&mut window, &harness.ctx, params, false);

            assert!(window.last_theme.is_some());
            assert_eq!(window.last_theme.as_ref().unwrap(), theme);
        }
    }

    #[test]
    fn test_parameter_passing_preserves_focus_flag() {
        let harness = Harness::new_ui(|ui| {
            ui.label("Test UI");
        });

        // Test with different window types and focus flags
        let mut simple_window = MockSimpleWindow::new();
        simple_window.show = true;

        let mut position_window = MockPositionWindow::new();
        position_window.show = true;

        let mut theme_window = MockThemeWindow::new();
        theme_window.show = true;

        // Test bring_to_front = false
        FocusableWindow::show_with_focus(&mut simple_window, &harness.ctx, (), false);
        FocusableWindow::show_with_focus(
            &mut position_window,
            &harness.ctx,
            egui::Pos2::ZERO,
            false,
        );
        FocusableWindow::show_with_focus(
            &mut theme_window,
            &harness.ctx,
            ThemeShowParams {
                theme: "Test".to_string(),
            },
            false,
        );

        assert!(!simple_window.last_bring_to_front);
        assert!(!position_window.last_bring_to_front);
        assert!(!theme_window.last_bring_to_front);

        // Test bring_to_front = true
        FocusableWindow::show_with_focus(&mut simple_window, &harness.ctx, (), true);
        FocusableWindow::show_with_focus(
            &mut position_window,
            &harness.ctx,
            egui::Pos2::ZERO,
            true,
        );
        FocusableWindow::show_with_focus(
            &mut theme_window,
            &harness.ctx,
            ThemeShowParams {
                theme: "Test".to_string(),
            },
            true,
        );

        assert!(simple_window.last_bring_to_front);
        assert!(position_window.last_bring_to_front);
        assert!(theme_window.last_bring_to_front);
    }

    #[test]
    fn test_parameter_cloning_and_ownership() {
        let harness = Harness::new_ui(|ui| {
            ui.label("Test UI");
        });

        let mut window = MockProjectWindow::new();
        window.show = true;

        // Create project that will be moved
        let project = Project::new(
            "ownership_test".to_string(),
            "Test project description".to_string(),
            "test".to_string(),
        );
        let position = egui::Pos2::new(10.0, 20.0);

        let params = ProjectShowParams {
            project: Some(project.clone()),
            window_pos: Some(position),
        };

        // Use the parameters (they should be cloned, not moved)
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params.clone(), false);

        // Should still be able to use original parameters
        assert!(params.project.is_some());
        assert_eq!(params.project.as_ref().unwrap().name, "ownership_test");

        // Window should have received the parameters
        assert!(window.last_project.is_some());
        assert_eq!(window.last_project.as_ref().unwrap().name, "ownership_test");
    }
}
