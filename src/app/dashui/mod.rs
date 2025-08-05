//! Desktop user interface implementation for AWS Dash Architect.
//!
//! This module provides a comprehensive egui-based desktop interface for CloudFormation
//! template editing, AWS resource management, and project organization. The UI follows
//! a window-based architecture where each functional area is implemented as a focusable
//! window component.
//!
//! # UI Architecture
//!
//! The interface is built around several key architectural patterns:
//!
//! ## Window Management System
//! - **Trait-based Windows**: All windows implement [`window_focus::FocusableWindow`] for consistent behavior
//! - **Focus Coordination**: [`window_focus::WindowFocusManager`] provides centralized focus management
//! - **Type-safe Parameters**: Different parameter types for different window categories
//! - **Persistent State**: Window positions and configurations survive application restarts
//!
//! ## Command Palette System
//! - **General Commands**: [`command_palette::CommandPalette`] for application-wide operations
//! - **Project Commands**: [`project_command_palette::ProjectCommandPalette`] for project management
//! - **CloudFormation Commands**: [`cloudformation_command_palette::CloudFormationCommandPalette`] for template operations
//! - **Fuzzy Search**: All palettes include intelligent search and filtering
//!
//! ## Resource Management Interface
//! - **Resource Browser**: [`resource_types_window::ResourceTypesWindow`] for AWS resource discovery
//! - **Form-based Editor**: [`resource_form_window::ResourceFormWindow`] for guided resource creation
//! - **JSON Editor**: [`resource_json_editor_window::ResourceJsonEditorWindow`] for advanced editing
//! - **Property Types**: [`property_type_window::PropertyTypeWindowManager`] for complex property editing
//!
//! ## Visualization Components
//! - **Dependency Graph**: [`cloudformation_scene_graph::CloudFormationSceneGraph`] for resource relationships
//! - **Template Structure**: [`template_sections_window::TemplateSectionsWindow`] for template organization
//! - **AWS Resource Icons**: [`aws_icon_manager::AwsIconManager`] for visual resource identification
//!
//! # Integration with Core Systems
//!
//! The UI layer integrates seamlessly with the application's core systems:
//! - **Template System**: Direct integration with [`crate::app::cfn_template`] for real-time editing
//! - **Resource Specifications**: Uses [`crate::app::cfn_resources`] for schema-driven form generation
//! - **Project Management**: Coordinates with [`crate::app::projects`] for data persistence
//! - **AWS Integration**: Leverages [`crate::app::aws_identity`] for authentication and account management
//!
//! # User Experience Features
//!
//! ## Theme Support
//! - **Multiple Themes**: Latte, Frappe, Macchiato, and Mocha color schemes via Catppuccin
//! - **Consistent Styling**: Theme-aware components throughout the application
//! - **User Preference Persistence**: Theme choices saved across sessions
//!
//! ## File Operations
//! - **Custom File Picker**: [`fuzzy_file_picker::FuzzyFilePicker`] with fuzzy search capabilities
//! - **CloudFormation Picker**: [`cloudformation_file_picker::CloudFormationFilePicker`] optimized for templates
//! - **Drag and Drop**: Support for template files and project imports
//!
//! ## Development Tools
//! - **Log Viewer**: [`log_window::LogWindow`] for real-time application logging
//! - **Credentials Debug**: [`credentials_debug_window::CredentialsDebugWindow`] for AWS credential troubleshooting
//! - **Verification Tools**: [`verification_window::VerificationWindow`] for template validation
//!
//! # Window Categories
//!
//! Windows are organized into functional categories:
//!
//! ## Core Application Windows
//! - [`app::DashApp`] - Main application coordinator and state manager
//! - [`help_window::HelpWindow`] - User documentation and guidance
//! - [`download_manager::DownloadManager`] - Background download coordination
//!
//! ## AWS Integration Windows
//! - [`aws_login_window::AwsLoginWindow`] - AWS Identity Center authentication
//! - [`chat_window::ChatWindow`] - AI-powered assistance via AWS Bedrock
//!
//! ## CloudFormation Editing Windows
//! - Resource management: [`resource_details_window`], [`resource_form_window`], [`resource_json_editor_window`]
//! - Template organization: [`template_sections_window`], [`cloudformation_scene_graph`]
//! - Property editing: [`property_type_window`], [`value_editor_window`], [`reference_picker_window`]
//!
//! See the [window focus documentation](../../../docs/technical/window-focus-system.wiki) for
//! detailed implementation patterns and the [UI testing guide](../../../docs/technical/ui-component-testing.wiki)
//! for testing strategies.

pub mod app;
pub mod aws_icon_manager;
pub mod aws_login_window;
pub mod chat_window;
pub mod cloudformation_command_palette;
pub mod control_bridge_window;
pub mod cloudformation_file_picker;
pub mod cloudformation_scene_graph;
pub mod cloudformation_window_node_widget;
pub mod command_palette;
pub mod credentials_debug_window;
pub mod deployment_info_window;
pub mod download_manager;
pub mod fuzzy_file_picker;
pub mod guard_violations_window;
pub mod help_window;
pub mod hint_mode;
pub mod key_mapping;
pub mod keyboard_navigation;
pub mod log_window;
pub mod menu;
pub mod navigable_widgets;
pub mod navigation_state;
pub mod project_command_palette;
pub mod property_type_form_window;
pub mod property_type_window;
pub mod reference_picker_window;
pub mod resource_details_window;
pub mod resource_form_window;
pub mod resource_json_editor_window;
pub mod resource_types_window;
pub mod resource_windows;
pub mod template_sections_window;
pub mod value_editor_window;
pub mod verification_window;
pub mod window_focus;
pub mod window_selector;

pub use app::DashApp;
pub use aws_icon_manager::AwsIconManager;
pub use aws_login_window::AwsLoginWindow;
pub use chat_window::ChatWindow;
pub use cloudformation_command_palette::{
    CloudFormationCommandAction, CloudFormationCommandPalette,
};
pub use control_bridge_window::ControlBridgeWindow;
pub use cloudformation_file_picker::{CloudFormationFilePicker, CloudFormationFilePickerStatus};
pub use cloudformation_scene_graph::CloudFormationSceneGraph;
pub use cloudformation_window_node_widget::{CloudFormationWindowNodeWidget, NodeWindowManager};
pub use command_palette::CommandPalette;
pub use credentials_debug_window::CredentialsDebugWindow;
pub use download_manager::DownloadManager;
pub use fuzzy_file_picker::{FuzzyFilePicker, FuzzyFilePickerStatus};
pub use guard_violations_window::GuardViolationsWindow;
pub use help_window::HelpWindow;
pub use hint_mode::{HintConfig, HintGenerator, HintMarker, HintMode, HintOverlay};
pub use key_mapping::{KeyBindingMap, KeyBindingSettings, KeyMapping, KeyMappingRegistry};
pub use keyboard_navigation::{
    ElementAction, KeyEventResult, KeyboardNavigable, NavigableElement, NavigableElementType,
    NavigableWindow, NavigationCommand, NavigationContext, NavigationMode,
};
pub use log_window::LogWindow;
pub use navigable_widgets::{
    DefaultNavigableContainer, FocusState, FocusStyle, NavigableContainer,
    NavigableElementCollector, NavigableWidget, NavigableWidgetManager, WidgetState,
};
pub use navigation_state::NavigationState;
pub use project_command_palette::{ProjectCommandAction, ProjectCommandPalette};
pub use property_type_window::{PropertyTypeWindow, PropertyTypeWindowManager};
pub use reference_picker_window::ReferencePickerWindow;
pub use resource_details_window::ResourceDetailsWindow;
pub use resource_form_window::ResourceFormWindow;
pub use resource_json_editor_window::ResourceJsonEditorWindow;
pub use resource_types_window::ResourceTypesWindow;
pub use value_editor_window::ValueEditorWindow;
pub use verification_window::VerificationWindow;
pub use window_focus::{
    FocusableWindow, IdentityShowParams, PositionShowParams, ProjectShowParams, SimpleShowParams,
    ThemeShowParams, WindowFocusManager,
};
pub use window_selector::{WindowInfo, WindowSelector, WindowType};
