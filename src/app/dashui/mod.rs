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
//! - **Resource Explorer**: Tree-based visualization of AWS resources across accounts and regions
//! - **Agent Control**: AI-powered AWS operations interface (Agent Framework)
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
//! - **CloudFormation Picker**: [`cloudformation_file_picker::CloudFormationFilePicker`] optimized for templates
//! - **Drag and Drop**: Support for template files and project imports
//!
//! ## Development Tools
//! - **Log Viewer**: [`log_window::LogWindow`] for real-time application logging
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

pub mod agent_log_window;
pub mod agent_manager_window;
pub mod app;
pub mod aws_login_window;
pub mod cloudwatch_logs_window;
pub mod cloudtrail_events_window;
pub mod command_palette;
pub mod help_window;
pub mod hint_mode;
pub mod key_mapping;
pub mod keyboard_navigation;
pub mod log_window;
pub mod menu;
pub mod navigable_widgets;
pub mod navigation_state;
pub mod verification_window;
pub mod window_focus;
pub mod window_selector;

pub use agent_log_window::AgentLogWindow;
pub use agent_manager_window::AgentManagerWindow;
pub use app::DashApp;
pub use aws_login_window::AwsLoginWindow;
pub use cloudwatch_logs_window::{CloudWatchLogsShowParams, CloudWatchLogsWindow};
pub use cloudtrail_events_window::{CloudTrailEventsShowParams, CloudTrailEventsWindow};
pub use command_palette::CommandPalette;
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
pub use verification_window::VerificationWindow;
pub use window_focus::{
    FocusableWindow, IdentityShowParams, PositionShowParams, ProjectShowParams, SimpleShowParams,
    ThemeShowParams, WindowFocusManager,
};
pub use window_selector::{WindowInfo, WindowSelector, WindowType};
