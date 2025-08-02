//! Trait implementation tests for all window types
//!
//! These tests verify that all windows correctly implement the FocusableWindow trait
//! and that their implementations are consistent and correct.

use awsdash::app::dashui::{
    aws_login_window::AwsLoginWindow, chat_window::ChatWindow,
    cloudformation_scene_graph::CloudFormationSceneGraph,
    credentials_debug_window::CredentialsDebugWindow, help_window::HelpWindow,
    log_window::LogWindow, resource_details_window::ResourceDetailsWindow,
    resource_form_window::ResourceFormWindow,
    resource_json_editor_window::ResourceJsonEditorWindow,
    resource_types_window::ResourceTypesWindow, template_sections_window::TemplateSectionsWindow,
    verification_window::VerificationWindow, window_focus::FocusableWindow,
};

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    #[test]
    fn test_help_window_trait_implementation() {
        let mut window = HelpWindow::new();

        // Test trait methods
        assert_eq!(window.window_id(), "help_window");
        assert_eq!(window.window_title(), "Help");
        assert!(!window.is_open());

        // Open window and test
        window.open = true;
        assert!(window.is_open());

        // Test show_with_focus doesn't panic
        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), true);
    }

    #[test]
    fn test_log_window_trait_implementation() {
        let mut window = LogWindow::new();

        assert_eq!(window.window_id(), "log_window");
        assert_eq!(window.window_title(), "Log Viewer");
        assert!(!window.is_open());

        window.open = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), true);
    }

    #[test]
    fn test_verification_window_trait_implementation() {
        let mut window = VerificationWindow::default();

        assert_eq!(window.window_id(), "verification_window");
        assert_eq!(
            window.window_title(),
            "CloudFormation Import Verification Results"
        );
        assert!(!window.is_open());

        window.visible = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), true);
    }

    #[test]
    fn test_credentials_debug_window_trait_implementation() {
        let mut window = CredentialsDebugWindow::default();

        assert_eq!(window.window_id(), "credentials_debug");
        assert_eq!(window.window_title(), "AWS Credentials Debug");
        assert!(!window.is_open());

        window.open = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), true);
    }

    #[test]
    fn test_resource_types_window_trait_implementation() {
        let mut window = ResourceTypesWindow::new();

        assert_eq!(window.window_id(), "resource_types");
        assert_eq!(window.window_title(), "CloudFormation Resource Types");
        assert!(!window.is_open());

        window.show = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), true);
    }

    #[test]
    fn test_resource_details_window_trait_implementation() {
        let mut window = ResourceDetailsWindow::new();

        assert_eq!(window.window_id(), "resource_details");
        // Title should be "Resource Details" when no resource type is selected
        assert_eq!(window.window_title(), "Resource Details");
        assert!(!window.is_open());

        window.show = true;
        assert!(window.is_open());

        // Test with resource type selected
        window.selected_resource_type = "AWS::S3::Bucket".to_string();
        assert_eq!(window.window_title(), "Resource Details: AWS::S3::Bucket");

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), true);
    }

    #[test]
    fn test_template_sections_window_trait_implementation() {
        let mut window = TemplateSectionsWindow::new();

        assert_eq!(window.window_id(), "template_sections");
        assert_eq!(window.window_title(), "CloudFormation Template");
        assert!(!window.is_open());

        window.show = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        // Test with project parameters
        use awsdash::app::dashui::window_focus::ProjectShowParams;
        let params = ProjectShowParams {
            project: None,
            window_pos: None,
        };

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params.clone(), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params, true);
    }

    #[test]
    fn test_aws_login_window_trait_implementation() {
        let mut window = AwsLoginWindow::default();

        assert_eq!(window.window_id(), "aws_login_window");
        assert_eq!(window.window_title(), "AWS Identity Center Login");
        assert!(!window.is_open());

        window.open = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        // Test with position parameters
        // use awsdash::app::dashui::window_focus::PositionShowParams; // Not needed
        let position = egui::Pos2::new(100.0, 200.0);

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, position, false);

        let position2 = egui::Pos2::new(50.0, 75.0);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, position2, true);
    }

    #[test]
    fn test_resource_form_window_trait_implementation() {
        let mut window = ResourceFormWindow::new();

        assert_eq!(window.window_id(), "resource_form");
        // Title should reflect empty state initially
        assert_eq!(window.window_title(), "New  Resource"); // Empty resource type
        assert!(!window.is_open());

        window.show = true;
        window.resource_type = "AWS::S3::Bucket".to_string();
        window.resource_id = "MyBucket".to_string();
        window.is_new = false;

        assert!(window.is_open());
        assert_eq!(window.window_title(), "Edit Resource: MyBucket");

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        // Test with form parameters
        use awsdash::app::dashui::window_focus::FormShowParams;
        let params = FormShowParams {};

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params.clone(), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params, true);
    }

    #[test]
    fn test_resource_json_editor_window_trait_implementation() {
        let mut window = ResourceJsonEditorWindow::new();

        assert_eq!(window.window_id(), "resource_json_editor");
        assert_eq!(window.window_title(), "Resource JSON Editor");
        assert!(!window.is_open());

        window.show = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        // Test with theme parameters
        use awsdash::app::dashui::window_focus::ThemeShowParams;
        let params = ThemeShowParams {
            theme: "Mocha".to_string(),
        };

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params.clone(), false);

        let params2 = ThemeShowParams {
            theme: "Latte".to_string(),
        };
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params2, true);
    }

    #[test]
    fn test_chat_window_trait_implementation() {
        let mut window = ChatWindow::new();

        assert_eq!(window.window_id(), "chat_window");
        assert_eq!(window.window_title(), "AWS Q Chat");
        assert!(!window.is_open());

        window.open = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        // Test with identity parameters
        use awsdash::app::dashui::window_focus::IdentityShowParams;
        let params = IdentityShowParams {
            aws_identity: None, // Test with None
        };

        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params, false);

        // Test with Some identity (would need actual AwsIdentityCenter instance)
        let params2 = IdentityShowParams {
            aws_identity: None, // Keep as None for test simplicity
        };
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, params2, true);
    }

    #[test]
    fn test_cloudformation_scene_graph_trait_implementation() {
        let mut window = CloudFormationSceneGraph::new();

        assert_eq!(window.window_id(), "cloudformation_scene");
        assert_eq!(window.window_title(), "CloudFormation Graph");
        assert!(!window.is_open());

        window.show = true;
        assert!(window.is_open());

        let harness = Harness::new_ui(|ui| {
            ui.label("Test");
        });

        // Note: CloudFormation Scene Graph doesn't use traditional window ordering
        // so bring_to_front parameter doesn't affect the rendering
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), false);
        FocusableWindow::show_with_focus(&mut window, &harness.ctx, (), true);
    }

    #[test]
    fn test_all_window_ids_are_unique() {
        let window_ids = [
            HelpWindow::new().window_id(),
            LogWindow::new().window_id(),
            VerificationWindow::default().window_id(),
            CredentialsDebugWindow::default().window_id(),
            ResourceTypesWindow::new().window_id(),
            ResourceDetailsWindow::new().window_id(),
            TemplateSectionsWindow::new().window_id(),
            AwsLoginWindow::default().window_id(),
            ResourceFormWindow::new().window_id(),
            ResourceJsonEditorWindow::new().window_id(),
            ChatWindow::new().window_id(),
            CloudFormationSceneGraph::new().window_id(),
        ];

        // Check for uniqueness
        for (i, id1) in window_ids.iter().enumerate() {
            for (j, id2) in window_ids.iter().enumerate() {
                if i != j {
                    assert_ne!(id1, id2, "Window IDs must be unique: {} == {}", id1, id2);
                }
            }
        }
    }

    #[test]
    fn test_all_window_titles_are_meaningful() {
        let windows_and_titles = [
            (HelpWindow::new().window_title(), "Help"),
            (LogWindow::new().window_title(), "Log Viewer"),
            (
                VerificationWindow::default().window_title(),
                "CloudFormation Import Verification Results",
            ),
            (
                CredentialsDebugWindow::default().window_title(),
                "AWS Credentials Debug",
            ),
            (
                ResourceTypesWindow::new().window_title(),
                "CloudFormation Resource Types",
            ),
            (
                ResourceDetailsWindow::new().window_title(),
                "Resource Details",
            ),
            (
                TemplateSectionsWindow::new().window_title(),
                "CloudFormation Template",
            ),
            (
                AwsLoginWindow::default().window_title(),
                "AWS Identity Center Login",
            ),
            (ResourceFormWindow::new().window_title(), "New  Resource"), // Empty resource type
            (
                ResourceJsonEditorWindow::new().window_title(),
                "Resource JSON Editor",
            ),
            (ChatWindow::new().window_title(), "AWS Q Chat"),
            (
                CloudFormationSceneGraph::new().window_title(),
                "CloudFormation Graph",
            ),
        ];

        for (actual_title, expected_title) in &windows_and_titles {
            assert_eq!(actual_title, expected_title);
            assert!(!actual_title.is_empty(), "Window title should not be empty");
            assert!(
                actual_title.len() > 3,
                "Window title should be meaningful (>3 chars)"
            );
        }
    }

    #[test]
    fn test_window_open_state_consistency() {
        // Test that is_open() correctly reflects the window's open state
        let mut help_window = HelpWindow::new();
        assert!(!help_window.is_open());
        help_window.open = true;
        assert!(help_window.is_open());

        let mut log_window = LogWindow::new();
        assert!(!log_window.is_open());
        log_window.open = true;
        assert!(log_window.is_open());

        let mut verification_window = VerificationWindow::default();
        assert!(!verification_window.is_open());
        verification_window.visible = true;
        assert!(verification_window.is_open());

        let mut credentials_window = CredentialsDebugWindow::default();
        assert!(!credentials_window.is_open());
        credentials_window.open = true;
        assert!(credentials_window.is_open());

        // Test a few more to ensure consistency across different field names
        let mut resource_types_window = ResourceTypesWindow::new();
        assert!(!resource_types_window.is_open());
        resource_types_window.show = true;
        assert!(resource_types_window.is_open());

        let mut aws_login_window = AwsLoginWindow::default();
        assert!(!aws_login_window.is_open());
        aws_login_window.open = true;
        assert!(aws_login_window.is_open());
    }
}
