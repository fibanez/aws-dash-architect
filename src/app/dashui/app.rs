use super::aws_login_window::AwsLoginWindow;

// Type aliases for complex types
type DeploymentTaskHandle =
    std::thread::JoinHandle<Result<(String, String, String), anyhow::Error>>;
// Validation task removed in Phase 2.2
// type ValidationTaskHandle =
//     std::thread::JoinHandle<Result<crate::app::cfn_guard::GuardValidation, anyhow::Error>>;
use super::command_palette::{CommandAction, CommandPalette};
use super::control_bridge_window::ControlBridgeWindow;
use super::credentials_debug_window::CredentialsDebugWindow;
// Resource/template editor windows removed
// use super::deployment_info_window::DeploymentInfoWindow;
use super::help_window::HelpWindow;
use super::log_window::LogWindow;
use super::menu;
// Project management removed
// use super::project_command_palette::{ProjectCommandAction, ProjectCommandPalette};
// Resource/template editor windows removed
// use super::property_type_window::PropertyTypeWindowManager;
// use super::resource_details_window::ResourceDetailsWindow;
// use super::resource_form_window::ResourceFormWindow;
// use super::resource_json_editor_window::ResourceJsonEditorWindow;
// use super::resource_types_window::ResourceTypesWindow;
// use super::template_sections_window::TemplateSectionsWindow;
use super::verification_window::VerificationWindow;
use super::window_focus::{
    FocusableWindow, IdentityShowParams, SimpleShowParams, WindowFocusManager,
};
use super::window_selector::{WindowSelector, WindowType};
use super::{
    ElementAction, HintMode, HintOverlay, KeyEventResult, KeyMappingRegistry,
    NavigableWidgetManager, NavigationCommand, NavigationMode, NavigationState,
};
use crate::app::aws_identity::AwsIdentityCenter;
use crate::app::bridge::set_global_aws_identity;
// CloudFormation resources removed
// use crate::app::cfn_resources::{
//     load_attribute_definitions, load_property_definitions, load_property_type_definitions,
//     CfnResourcesDownloader,
// };
// CloudFormation manager removed
// use crate::app::cloudformation_manager::{CloudFormationManager, ValidationResultsWindow};
use crate::app::fonts;
use crate::app::notifications::NotificationManager;
// Project management removed
// use crate::app::projects::CloudFormationResource;
use crate::app::resource_explorer::ResourceExplorer;
use crate::trace_info;
use eframe::egui;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tracing::info;

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Default)]
pub enum ThemeChoice {
    #[default]
    Latte,
    Frappe,
    Macchiato,
    Mocha,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct NavigationStatusBarSettings {
    pub show_status_bar: bool,
}

impl Default for NavigationStatusBarSettings {
    fn default() -> Self {
        Self {
            show_status_bar: false, // Hidden by default
        }
    }
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeChoice::Latte => write!(f, "Latte"),
            ThemeChoice::Frappe => write!(f, "Frappe"),
            ThemeChoice::Macchiato => write!(f, "Macchiato"),
            ThemeChoice::Mocha => write!(f, "Mocha"),
        }
    }
}

/// Pending actions to be executed on widgets
#[derive(Debug, Clone)]
pub enum PendingWidgetAction {
    ClickButton(String),     // Widget ID to click
    FocusTextInput(String),  // Widget ID to focus
    SelectListItem(String),  // Widget ID to select
    ActivateSection(String), // Section ID to activate
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedWindow {
    CommandPalette,
    // Resource/template editor windows removed
    // ResourceTypes,
    // ResourceDetails,
    // ResourceForm,
    // ResourceJsonEditor,
    // PropertyType(usize), // Index into property_type_manager.windows
    // TemplateSections,
    AwsLogin,
    AwsAccounts,
    StartupPopup,
    // Project management removed
    // ProjectCommandPalette,
    // ProjectForm,
    Help,
    Log,
    Chat,
    ControlBridge,
    CredentialsDebug,
    // DeploymentInfo,
    Verification,
    GuardViolations,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DashApp {
    pub theme: ThemeChoice,
    pub navigation_status_bar_settings: NavigationStatusBarSettings,

    #[serde(skip)]
    pub command_palette: CommandPalette,
    #[serde(skip)]
    pub show_command_palette: bool,
    // Resource/template editor windows removed
    // #[serde(skip)]
    // pub resource_types_window: ResourceTypesWindow,
    // #[serde(skip)]
    // pub resource_details_window: ResourceDetailsWindow,
    // #[serde(skip)]
    // pub resource_form_window: ResourceFormWindow,
    // #[serde(skip)]
    // pub resource_json_editor_window: ResourceJsonEditorWindow,
    // #[serde(skip)]
    // pub property_type_manager: PropertyTypeWindowManager,
    // #[serde(skip)]
    // pub template_sections_window: TemplateSectionsWindow,
    // Project management removed
    // #[serde(skip)]
    // pub project_command_palette: ProjectCommandPalette,
    #[serde(skip)]
    pub aws_login_window: AwsLoginWindow,
    #[serde(skip)]
    pub help_window: HelpWindow,
    #[serde(skip)]
    pub log_window: LogWindow,
    #[serde(skip)]
    pub control_bridge_window: ControlBridgeWindow,
    #[serde(skip)]
    pub credentials_debug_window: CredentialsDebugWindow,
    // Resource/template editor windows removed
    // #[serde(skip)]
    // pub deployment_info_window: DeploymentInfoWindow,
    #[serde(skip)]
    pub verification_window: VerificationWindow,
    #[serde(skip)]
    pub resource_explorer: ResourceExplorer,
    // CloudFormation manager removed
    // #[serde(skip)]
    // pub cloudformation_manager: Option<std::sync::Arc<CloudFormationManager>>,
    // CloudFormation manager windows removed
    // #[serde(skip)]
    // pub validation_results_window: ValidationResultsWindow,
    // #[serde(skip)]
    // pub parameter_dialog:
    //     crate::app::cloudformation_manager::parameter_dialog::ParameterInputDialog,
    // #[serde(skip)]
    // pub deployment_progress_window:
    //     crate::app::cloudformation_manager::deployment_progress_window::DeploymentProgressWindow,
    #[serde(skip)]
    pub pending_deployment_task: Option<DeploymentTaskHandle>,
    #[serde(skip)]
    pub notification_manager: NotificationManager,
    #[serde(skip)]
    current_template_hash: Option<u64>,
    #[serde(skip)]
    pub window_selector: WindowSelector,
    #[serde(skip)]
    pub aws_identity_center: Option<std::sync::Arc<std::sync::Mutex<AwsIdentityCenter>>>,
    #[serde(skip)]
    pub startup_popup_timer: Option<Instant>,
    #[serde(skip)]
    pub show_startup_popup: bool,
    #[serde(skip)]
    previous_screen_size: Option<egui::Vec2>,
    #[serde(skip)]
    previous_pixels_per_point: Option<f32>,
    #[serde(skip)]
    currently_focused_window: Option<FocusedWindow>,
    #[serde(skip)]
    window_focus_order: Vec<FocusedWindow>,
    #[serde(skip)]
    shake_windows: bool,
    #[serde(skip)]
    shake_start_time: Option<Instant>,
    #[serde(skip)]
    shake_duration: Duration,
    #[serde(skip)]
    window_positions: std::collections::HashMap<String, egui::Pos2>,
    #[serde(skip)]
    window_shake_offsets: std::collections::HashMap<String, egui::Vec2>,
    #[serde(skip)]
    pending_shake_timer: Option<Instant>,
    #[serde(skip)]
    logged_states: HashSet<String>,
    #[serde(skip)]
    window_focus_manager: WindowFocusManager,
    #[serde(skip)]
    navigation_state: NavigationState,
    #[serde(skip)]
    key_mapping_registry: KeyMappingRegistry,
    #[serde(skip)]
    pending_scroll_request: Option<f32>,
    #[serde(skip)]
    hint_mode: HintMode,
    #[serde(skip)]
    hint_overlay: HintOverlay,
    #[serde(skip)]
    /// Flag to skip the first input after activating hint mode (prevents double-processing)
    skip_next_hint_input: bool,
    #[serde(skip)]
    widget_manager: NavigableWidgetManager,
    #[serde(skip)]
    /// Queue of pending widget actions to execute
    pending_widget_actions: Vec<PendingWidgetAction>,
    #[serde(skip)]
    /// Flag to ensure enhanced fonts are configured only once
    fonts_configured: bool,
    #[serde(skip)]
    /// Current compliance validation status
    compliance_status: Option<crate::app::dashui::menu::ComplianceStatus>,
}

impl Default for DashApp {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::default(),
            navigation_status_bar_settings: NavigationStatusBarSettings::default(),
            command_palette: CommandPalette::new(),
            show_command_palette: false,
            // Resource/template editor windows removed
            // resource_types_window: ResourceTypesWindow::new(),
            // resource_details_window: ResourceDetailsWindow::new(),
            // resource_form_window: ResourceFormWindow::new(),
            // resource_json_editor_window: ResourceJsonEditorWindow::new(),
            // property_type_manager: PropertyTypeWindowManager::new(),
            // template_sections_window: TemplateSectionsWindow::new(),
            // Project management removed
            // project_command_palette: ProjectCommandPalette::new(),
            aws_login_window: AwsLoginWindow::default(),
            help_window: HelpWindow::new(),
            log_window: LogWindow::new(),
            control_bridge_window: ControlBridgeWindow::new(),
            credentials_debug_window: CredentialsDebugWindow::default(),
            // Resource/template editor windows removed
            // deployment_info_window: DeploymentInfoWindow::default(),
            verification_window: VerificationWindow::default(),
            resource_explorer: ResourceExplorer::new(),
            // CloudFormation manager removed
            // cloudformation_manager: None,
            // CloudFormation manager windows removed
            // validation_results_window: ValidationResultsWindow::new(),
            // parameter_dialog: crate::app::cloudformation_manager::parameter_dialog::ParameterInputDialog::new(),
            // deployment_progress_window: crate::app::cloudformation_manager::deployment_progress_window::DeploymentProgressWindow::new(),
            pending_deployment_task: None,
            notification_manager: NotificationManager::new(),
            current_template_hash: None,
            window_selector: WindowSelector::new(),
            aws_identity_center: None,
            startup_popup_timer: Some(Instant::now()),
            show_startup_popup: true,
            previous_screen_size: None,
            previous_pixels_per_point: None,
            currently_focused_window: None, // Default to no focus
            window_focus_order: Vec::new(),
            shake_windows: false,
            shake_start_time: None,
            shake_duration: Duration::from_millis(500),
            window_positions: std::collections::HashMap::new(),
            window_shake_offsets: std::collections::HashMap::new(),
            pending_shake_timer: None,
            logged_states: HashSet::new(),
            window_focus_manager: WindowFocusManager::new(),
            navigation_state: NavigationState::new(),
            key_mapping_registry: KeyMappingRegistry::new(),
            pending_scroll_request: None,
            hint_mode: HintMode::new(),
            hint_overlay: HintOverlay::new(),
            skip_next_hint_input: false,
            widget_manager: NavigableWidgetManager::new(),
            pending_widget_actions: Vec::new(),
            fonts_configured: false,
            compliance_status: None,
        }
    }
}

impl DashApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        // Apply the saved theme
        app.apply_theme(&cc.egui_ctx);

        // Start repository synchronization in background
        app.start_repository_sync();

        app
    }

    /// Start the shake animation for all windows
    pub fn start_shake_animation(&mut self) {
        self.shake_windows = true;
        self.shake_start_time = Some(Instant::now());
        self.window_shake_offsets.clear();
        // Add all tracked windows to shake offsets
        for window_id in self.window_positions.keys() {
            self.window_shake_offsets
                .insert(window_id.clone(), egui::Vec2::ZERO);
        }
    }

    /// Start a delayed shake animation (for automatic triggers)
    pub fn start_delayed_shake_animation(&mut self) {
        // Set a 100ms delay to allow window to settle
        self.pending_shake_timer = Some(Instant::now());
    }

    /// Trigger compliance validation for the current project (removed in Phase 2.1)
    fn trigger_compliance_validation(&mut self) {
        // Compliance/Guard system removed
    }

    /// Update shake offsets for tracked windows that are currently shaking
    fn update_window_shake_offsets(&mut self) {
        if let Some(start_time) = self.shake_start_time {
            let elapsed = start_time.elapsed();
            let progress = elapsed.as_secs_f32() / self.shake_duration.as_secs_f32();
            let intensity = (1.0 - progress) * 10.0; // Start at 10 pixels, decrease to 0
            let time = elapsed.as_millis() as f32;

            // Update offsets for windows that are currently in the shake list
            let windows_to_shake: Vec<String> = self.window_shake_offsets.keys().cloned().collect();
            for window_id in windows_to_shake {
                // Each window gets a slightly different shake pattern
                let hash = window_id.bytes().fold(0u8, |acc, b| acc.wrapping_add(b)) as f32;
                let x_offset = (time * (0.1 + hash * 0.001)).sin() * intensity;
                let y_offset = (time * (0.15 + hash * 0.001)).cos() * intensity;

                self.window_shake_offsets
                    .insert(window_id, egui::Vec2::new(x_offset, y_offset));
            }
        }
    }

    /// Get the current position for a window (including shake offset if active)
    pub fn get_window_position(&self, window_id: &str) -> Option<egui::Pos2> {
        if let Some(base_pos) = self.window_positions.get(window_id) {
            if let Some(offset) = self.window_shake_offsets.get(window_id) {
                Some(*base_pos + *offset)
            } else {
                Some(*base_pos)
            }
        } else {
            None
        }
    }

    /// Update the tracked position of a window
    pub fn update_window_position(&mut self, window_id: String, pos: egui::Pos2) {
        // Only update if we're not shaking (to preserve the original position)
        if !self.shake_windows {
            self.window_positions.insert(window_id, pos);
        }
    }

    /// Log a message only once (to prevent flooding)
    #[allow(dead_code)]
    fn log_once(&mut self, key: &str, message: &str) {
        if !self.logged_states.contains(key) {
            trace_info!("{}", message);
            self.logged_states.insert(key.to_string());
        }
    }

    /// Set the currently focused window
    fn set_focused_window(&mut self, window: FocusedWindow) {
        // Only do something if this is a different window
        if self.currently_focused_window != Some(window) {
            // Focus change - no logging to prevent potential flooding
            // If there was a previously focused window, update the order
            if let Some(prev_window) = self.currently_focused_window {
                // Remove the window from the order if it's already there
                self.window_focus_order.retain(|w| *w != prev_window);

                // Add the old window to the front of the order list
                self.window_focus_order.push(prev_window);
            }

            // Set the new focused window
            self.currently_focused_window = Some(window);

            // Remove the new window from the order if it was there
            self.window_focus_order.retain(|w| *w != window);
        }
    }

    /// Get the most recently focused window (other than the current one)
    fn get_previous_window(&self) -> Option<FocusedWindow> {
        self.window_focus_order.last().copied()
    }

    /// Close the currently focused window and focus the next available window
    fn close_focused_window(&mut self) {
        if let Some(window) = self.currently_focused_window {
            match window {
                FocusedWindow::CommandPalette => {
                    self.show_command_palette = false;
                    self.command_palette.show = false;
                }
                // Resource/template editor windows removed
                // FocusedWindow::ResourceTypes => {
                //     self.resource_types_window.show = false;
                // }
                // FocusedWindow::ResourceDetails => {
                //     self.resource_details_window.show = false;
                // }
                // FocusedWindow::ResourceForm => {
                //     self.resource_form_window.show = false;
                // }
                // CloudFormation scene graph removed
                // FocusedWindow::ResourceJsonEditor => {
                //     self.resource_json_editor_window.show = false;
                // }
                // FocusedWindow::PropertyType(idx) => {
                //     if idx < self.property_type_manager.windows.len() {
                //         self.property_type_manager.windows[idx].show = false;
                //     }
                // }
                FocusedWindow::AwsLogin => {
                    self.aws_login_window.open = false;
                }
                FocusedWindow::AwsAccounts => {
                    self.aws_login_window.accounts_window_open = false;
                }
                FocusedWindow::StartupPopup => {
                    self.show_startup_popup = false;
                    self.startup_popup_timer = None;
                }
                // Project management removed
                // FocusedWindow::ProjectCommandPalette => {
                //     self.project_command_palette.mode = ProjectCommandAction::Closed;
                // }
                // FocusedWindow::ProjectForm => {
                //     // Return to project command palette instead of closing completely
                //     self.project_command_palette.mode = ProjectCommandAction::CommandPalette;
                //     self.currently_focused_window = Some(FocusedWindow::ProjectCommandPalette);
                //     return;
                // }
                // CloudFormation command palette removed
                // CloudFormation form removed
                FocusedWindow::Help => {
                    self.help_window.open = false;
                }
                FocusedWindow::Log => {
                    self.log_window.open = false;
                }
                FocusedWindow::Chat => {
                    // Chat window removed
                }
                FocusedWindow::ControlBridge => {
                    self.control_bridge_window.open = false;
                }
                FocusedWindow::CredentialsDebug => {
                    self.credentials_debug_window.open = false;
                }
                // Resource/template editor windows removed
                // FocusedWindow::TemplateSections => {
                //     self.template_sections_window.show = false;
                // }
                FocusedWindow::Verification => {
                    self.verification_window.visible = false;
                }
                FocusedWindow::GuardViolations => {
                    // Guard violations window removed in Phase 2.1
                }
                // Resource/template editor windows removed
                // FocusedWindow::DeploymentInfo => {
                //     self.deployment_info_window.open = false;
                // }
            }

            // Remove the closed window from focus order
            self.window_focus_order.retain(|w| *w != window);

            // Set focus to the next available window
            self.currently_focused_window = self.get_previous_window();

            info!(
                "Closed window: {:?}, new focus: {:?}",
                window, self.currently_focused_window
            );
        }
    }

    // Apply the current theme to the context
    fn apply_theme(&self, ctx: &egui::Context) {
        // Apply the selected theme
        match self.theme {
            ThemeChoice::Latte => catppuccin_egui::set_theme(ctx, catppuccin_egui::LATTE),
            ThemeChoice::Frappe => catppuccin_egui::set_theme(ctx, catppuccin_egui::FRAPPE),
            ThemeChoice::Macchiato => catppuccin_egui::set_theme(ctx, catppuccin_egui::MACCHIATO),
            ThemeChoice::Mocha => catppuccin_egui::set_theme(ctx, catppuccin_egui::MOCHA),
        }

        // Make window corners more square by setting global window style
        let mut style = (*ctx.style()).clone();
        style.visuals.window_corner_radius = egui::CornerRadius::same(2); // Set window corner radius to 2 for a more square look
        ctx.set_style(style);
    }

    /// Check for UI dimension changes like window resize or font scale change
    fn check_ui_dimension_changes(&mut self, ctx: &egui::Context) {
        // Check for window size or font scale changes
        let current_screen_size = ctx.screen_rect().size();
        let current_pixels_per_point = ctx.pixels_per_point();

        // Detect window resize
        if self.previous_screen_size != Some(current_screen_size) {
            // Window size changed
            self.command_palette.on_window_resized();
            self.previous_screen_size = Some(current_screen_size);
        }

        // Detect font size change
        if self.previous_pixels_per_point != Some(current_pixels_per_point) {
            // Font scale changed
            self.command_palette.on_font_size_changed();
            self.previous_pixels_per_point = Some(current_pixels_per_point);
        }
    }

    /// Handle keyboard input for command palette and window closing
    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        // Process navigation system first (only when UI doesn't want keyboard input)
        if !ctx.wants_keyboard_input() {
            self.handle_navigation_input(ctx);
        }

        // Legacy keybindings for backwards compatibility
        self.handle_legacy_keyboard_input(ctx);
    }

    /// Handle keyboard input through the navigation system
    fn handle_navigation_input(&mut self, ctx: &egui::Context) {
        // Process all pending input events
        ctx.input(|input| {
            for event in &input.events {
                match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        // Handle space bar for command palette (bypasses all Vimium navigation)
                        // This works in any mode: Normal, Insert, Hint, Visual, or Command
                        if *key == egui::Key::Space && modifiers.is_none() {
                            info!("Space bar pressed - opening command palette");
                            self.show_command_palette = true;
                            self.set_focused_window(FocusedWindow::CommandPalette);
                            continue; // Skip further processing for space bar
                        }

                        // Handle hint mode input first if active
                        if self.hint_mode.is_active() {
                            self.handle_hint_mode_input(event);
                        } else {
                            let result = self.navigation_state.process_key_event(event, ctx);
                            self.handle_navigation_result(result);
                        }
                    }
                    egui::Event::Text(_) => {
                        // Handle hint mode input first if active
                        if self.hint_mode.is_active() {
                            self.handle_hint_mode_input(event);
                        } else {
                            let result = self.navigation_state.process_key_event(event, ctx);
                            self.handle_navigation_result(result);
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    /// Handle the result of navigation key processing
    fn handle_navigation_result(&mut self, result: KeyEventResult) {
        match result {
            KeyEventResult::Handled => {
                // Navigation system handled the key, do nothing
            }
            KeyEventResult::PassThrough => {
                // Let the UI handle the key normally
            }
            KeyEventResult::ModeChanged(new_mode) => {
                // Mode changed, update UI feedback if needed
                info!("Navigation mode changed to: {:?}", new_mode);
            }
            KeyEventResult::Command(command) => {
                self.execute_navigation_command(command);
            }
            KeyEventResult::Cancel => {
                // Return to normal mode
                self.navigation_state.set_mode(NavigationMode::Normal);
            }
        }
    }

    /// Handle input when hint mode is active
    fn handle_hint_mode_input(&mut self, event: &egui::Event) {
        // Check if we should skip this input (to prevent activation key double-processing)
        if self.skip_next_hint_input {
            self.skip_next_hint_input = false;
            info!("Skipping hint input to prevent activation key double-processing");
            return;
        }

        match event {
            egui::Event::Key {
                key, pressed: true, ..
            } => {
                match key {
                    egui::Key::Escape => {
                        // Exit hint mode
                        self.hint_mode.stop();
                        self.navigation_state.pop_mode();
                        info!("Exited hint mode");
                    }
                    egui::Key::Backspace => {
                        // Remove last filter character
                        self.hint_mode.remove_filter_char();
                    }
                    egui::Key::Enter => {
                        // Activate exact match if available
                        if let Some(element_id) = self.hint_mode.get_exact_match_element_id() {
                            self.activate_hint_element(&element_id);
                        }
                    }
                    _ => {
                        // Try to convert key to character for filtering
                        if let Some(ch) = self.key_to_char(*key) {
                            self.hint_mode.add_filter_char(ch);

                            // Check for exact match
                            if let Some(element_id) = self.hint_mode.get_exact_match_element_id() {
                                self.activate_hint_element(&element_id);
                            }
                        }
                    }
                }
            }
            egui::Event::Text(text) => {
                // Handle text input for hint filtering
                for ch in text.chars() {
                    self.hint_mode.add_filter_char(ch);
                }

                // Check for exact match after text input
                if let Some(element_id) = self.hint_mode.get_exact_match_element_id() {
                    self.activate_hint_element(&element_id);
                }
            }
            _ => {}
        }
    }

    /// Activate a hint element by its ID
    fn activate_hint_element(&mut self, element_id: &str) {
        info!("Activating hint element: {}", element_id);

        // Get the resolved action from the hint marker, not the current mode action
        let action = if let Some(hint) = self.hint_mode.has_exact_match() {
            hint.action // Use resolved action from hint (Smart actions are already resolved here)
        } else {
            self.hint_mode.current_action() // Fallback to current action
        };

        info!(
            "Using resolved action: {:?} for element: {}",
            action, element_id
        );

        // Handle ResourceFormWindow-specific elements (legacy prefix-based routing)
        if element_id.starts_with("resource_form_") {
            self.handle_resource_form_element_activation(element_id, action);
        } else if element_id.starts_with("template_sections_") {
            self.handle_template_sections_element_activation(element_id, action);
        } else if self.is_resource_form_element(element_id) {
            // Handle ResourceFormWindow elements (new: real widget integration)
            info!(
                "Element routing debug: id='{}' identified as ResourceForm element",
                element_id
            );
            self.queue_resource_form_action(element_id, action);
        } else if self.is_property_type_form_element(element_id) {
            // Handle PropertyTypeFormWindow elements (new: real widget integration)
            info!(
                "Element routing debug: id='{}' identified as PropertyTypeFormWindow element",
                element_id
            );
            self.queue_property_type_form_action(element_id, action);
        } else {
            // Handle real widget integration - queue action for execution (TemplateSections and others)
            info!("Queueing action {:?} for element: {}", action, element_id);

            // Resource/template editor windows removed
            // // Queue the action to be executed on the next frame when the widget is rendered
            // self.template_sections_window
            //     .queue_widget_action(element_id.to_string(), action);

            // Also handle immediate actions that don't require widget interaction
            match action {
                ElementAction::Copy => {
                    info!("Copying text from element: {}", element_id);
                    // Real implementation would copy actual element text to system clipboard
                    info!("Copied text from '{}' to clipboard", element_id);
                }
                _ => {
                    // Other actions will be handled when the widget is rendered
                }
            }
        }

        // Exit hint mode after activation
        self.hint_mode.stop();
        self.navigation_state.pop_mode();
    }

    /// Handle activation of ResourceFormWindow elements
    fn handle_resource_form_element_activation(&mut self, element_id: &str, action: ElementAction) {
        info!(
            "Activating ResourceFormWindow element: {} with action: {:?}",
            element_id, action
        );

        // Resolve Smart action to specific action based on element type
        let resolved_action = if action == ElementAction::Smart {
            // Determine appropriate action based on element ID patterns
            if element_id.contains("_button") {
                ElementAction::Click
            } else if element_id.contains("_input")
                || element_id.contains("_id")
                || element_id.contains("_field")
            {
                ElementAction::Focus
            } else {
                ElementAction::Activate
            }
        } else {
            action
        };

        info!(
            "Using resolved action: {:?} for ResourceForm element: {}",
            resolved_action, element_id
        );

        // Parse element type from ID
        if element_id.contains("_save_button") {
            match resolved_action {
                ElementAction::Click | ElementAction::Activate => {
                    info!("ResourceForm: Save button activated");
                    // In a full implementation, this would trigger the save logic
                    // For now, just log the action
                }
                _ => {
                    info!(
                        "ResourceForm: Save button - action {:?} not supported",
                        resolved_action
                    );
                }
            }
        } else if element_id.contains("_cancel_button") {
            match resolved_action {
                ElementAction::Click | ElementAction::Activate => {
                    info!("ResourceForm: Cancel button activated");
                    // Resource/template editor windows removed
                    // self.resource_form_window.show = false; // Close the form
                }
                _ => {
                    info!(
                        "ResourceForm: Cancel button - action {:?} not supported",
                        resolved_action
                    );
                }
            }
        } else if element_id.contains("_resource_id") {
            match resolved_action {
                ElementAction::Focus => {
                    info!("ResourceForm: Focusing Resource ID field");
                    // In a full implementation, this would focus the text input
                }
                ElementAction::Copy => {
                    // Resource/template editor windows removed
                    // info!(
                    //     "ResourceForm: Copying Resource ID: {}",
                    //     self.resource_form_window.resource_id
                    // );
                    // In a full implementation, this would copy to clipboard
                }
                _ => {
                    info!(
                        "ResourceForm: Resource ID field - action {:?} not supported",
                        resolved_action
                    );
                }
            }
        } else {
            // Handle property fields
            match resolved_action {
                ElementAction::Focus => {
                    info!("ResourceForm: Focusing property field: {}", element_id);
                    // In a full implementation, this would focus the specific property field
                }
                ElementAction::Copy => {
                    info!("ResourceForm: Copying property value from: {}", element_id);
                    // In a full implementation, this would copy the property value
                }
                _ => {
                    info!(
                        "ResourceForm: Property field {} - action {:?} not supported",
                        element_id, resolved_action
                    );
                }
            }
        }
    }

    /// Handle activation of TemplateSectionsWindow elements
    fn handle_template_sections_element_activation(
        &mut self,
        element_id: &str,
        action: ElementAction,
    ) {
        info!(
            "Activating TemplateSectionsWindow element: {} with action: {:?}",
            element_id, action
        );

        // Resolve Smart action to specific action based on element type
        let resolved_action = if action == ElementAction::Smart {
            // Determine appropriate action based on element ID patterns
            if element_id.contains("_button")
                || element_id.contains("_resource_")
                || element_id.contains("_section_")
            {
                ElementAction::Click
            } else if element_id.contains("_filter")
                || element_id.contains("_input")
                || element_id.contains("_field")
            {
                ElementAction::Focus
            } else {
                ElementAction::Activate
            }
        } else {
            action
        };

        info!(
            "Using resolved action: {:?} for TemplateSections element: {}",
            resolved_action, element_id
        );

        // Parse element type from ID and queue the appropriate action
        if element_id.contains("_edit_resource_") {
            // Extract resource ID from element_id (format: "template_sections_edit_resource_{resource_id}")
            if let Some(resource_id) = element_id.strip_prefix("template_sections_edit_resource_") {
                match resolved_action {
                    ElementAction::Click | ElementAction::Activate => {
                        info!(
                            "TemplateSections: Edit resource button activated for: {}",
                            resource_id
                        );
                        self.pending_widget_actions
                            .push(PendingWidgetAction::ClickButton(format!(
                                "edit_resource_{}",
                                resource_id
                            )));
                    }
                    _ => {
                        info!(
                            "TemplateSections: Edit resource button - action {:?} not supported",
                            resolved_action
                        );
                    }
                }
            }
        } else if element_id.contains("_json_resource_") {
            // Extract resource ID from element_id
            if let Some(resource_id) = element_id.strip_prefix("template_sections_json_resource_") {
                match resolved_action {
                    ElementAction::Click | ElementAction::Activate => {
                        info!(
                            "TemplateSections: JSON resource button activated for: {}",
                            resource_id
                        );
                        self.pending_widget_actions
                            .push(PendingWidgetAction::ClickButton(format!(
                                "json_resource_{}",
                                resource_id
                            )));
                    }
                    _ => {
                        info!(
                            "TemplateSections: JSON resource button - action {:?} not supported",
                            resolved_action
                        );
                    }
                }
            }
        } else if element_id.contains("_delete_resource_") {
            // Extract resource ID from element_id
            if let Some(resource_id) = element_id.strip_prefix("template_sections_delete_resource_")
            {
                match resolved_action {
                    ElementAction::Click | ElementAction::Activate => {
                        info!(
                            "TemplateSections: Delete resource button activated for: {}",
                            resource_id
                        );
                        self.pending_widget_actions
                            .push(PendingWidgetAction::ClickButton(format!(
                                "delete_resource_{}",
                                resource_id
                            )));
                    }
                    _ => {
                        info!(
                            "TemplateSections: Delete resource button - action {:?} not supported",
                            resolved_action
                        );
                    }
                }
            }
        } else if element_id.contains("_section_") {
            // Handle section tab activation
            if let Some(section_name) = element_id.strip_prefix("template_sections_section_") {
                match resolved_action {
                    ElementAction::Click | ElementAction::Activate => {
                        info!("TemplateSections: Section tab activated: {}", section_name);
                        self.pending_widget_actions
                            .push(PendingWidgetAction::ActivateSection(
                                section_name.to_string(),
                            ));
                    }
                    _ => {
                        info!(
                            "TemplateSections: Section tab - action {:?} not supported",
                            resolved_action
                        );
                    }
                }
            }
        } else if element_id.contains("_resource_filter") {
            match resolved_action {
                ElementAction::Focus => {
                    info!("TemplateSections: Focusing resource filter field");
                    self.pending_widget_actions
                        .push(PendingWidgetAction::FocusTextInput(
                            "resource_filter".to_string(),
                        ));
                }
                ElementAction::Copy => {
                    // Resource/template editor windows removed
                    // info!(
                    //     "TemplateSections: Copying resource filter text: {}",
                    //     self.template_sections_window.filter_text
                    // );
                    // In a full implementation, this would copy to clipboard
                }
                _ => {
                    info!(
                        "TemplateSections: Resource filter - action {:?} not supported",
                        resolved_action
                    );
                }
            }
        } else {
            info!(
                "TemplateSections: Unknown element activated: {}",
                element_id
            );
        }
    }

    /// Helper to detect if an element belongs to ResourceFormWindow
    fn is_resource_form_element(&self, _element_id: &str) -> bool {
        // Resource/template editor windows removed
        false
    }

    /// Helper to detect if an element belongs to PropertyTypeFormWindow
    fn is_property_type_form_element(&self, element_id: &str) -> bool {
        // PropertyTypeFormWindow elements include:
        // - property_form_cancel_button, property_form_apply_button (buttons)
        // - property_input_{prop_name} (text inputs)
        element_id == "property_form_cancel_button"
            || element_id == "property_form_apply_button"
            || element_id.starts_with("property_input_")
    }

    /// Queue action on ResourceFormWindow
fn queue_resource_form_action(&mut self, _element_id: &str, _action: ElementAction) {
        // Resource/template editor windows removed
    }

    /// Queue action on PropertyTypeFormWindow (via ResourceFormWindow)
    fn queue_property_type_form_action(&mut self, _element_id: &str, _action: ElementAction) {
        // Resource/template editor windows removed
        // info!(
        //     "Queueing action {:?} for PropertyTypeFormWindow element: {}",
        //     action, element_id
        // );
        // Since PropertyTypeFormWindow instances are managed within ResourceFormWindow,
        // we need to find and queue the action on the appropriate PropertyTypeFormWindow
        // For now, we'll queue it on all open PropertyTypeFormWindow instances
        // The widget manager will only activate it on the window that has that element
        // for form in &mut self.resource_form_window.property_type_forms {
        //     if form.is_open() {
        //         form.queue_widget_action(element_id.to_string(), action);
        //     }
        // }
    }

    /// Process pending widget actions queued from hint activation
    fn process_pending_widget_actions(&mut self) {
        let actions = std::mem::take(&mut self.pending_widget_actions);

        for action in actions {
            match action {
                PendingWidgetAction::ClickButton(button_id) => {
                    info!("Processing pending click action for button: {}", button_id);
                    // This will be handled by the actual UI rendering when it creates the button
                    // For now, we store it for the template sections window to check
                    if button_id.starts_with("edit_resource_") {
                        // Extract resource ID and trigger edit
                        if let Some(resource_id) = button_id.strip_prefix("edit_resource_") {
                            info!("Triggering edit for resource: {}", resource_id);
                            // TODO: Actually trigger the edit action in template sections window
                        }
                    }
                }
                PendingWidgetAction::FocusTextInput(input_id) => {
                    info!(
                        "Processing pending focus action for text input: {}",
                        input_id
                    );
                    // This will be handled when the UI renders the text input
                }
                PendingWidgetAction::SelectListItem(item_id) => {
                    info!(
                        "Processing pending select action for list item: {}",
                        item_id
                    );
                    // This will be handled when the UI renders the list
                }
                PendingWidgetAction::ActivateSection(section_name) => {
                    info!("Processing pending section activation: {}", section_name);
                    // TODO: Actually change the selected section in template sections window
                }
            }
        }
    }

    /// Convert a key to character for hint filtering
    fn key_to_char(&self, key: egui::Key) -> Option<char> {
        match key {
            egui::Key::A => Some('a'),
            egui::Key::B => Some('b'),
            egui::Key::C => Some('c'),
            egui::Key::D => Some('d'),
            egui::Key::E => Some('e'),
            egui::Key::F => Some('f'),
            egui::Key::G => Some('g'),
            egui::Key::H => Some('h'),
            egui::Key::I => Some('i'),
            egui::Key::J => Some('j'),
            egui::Key::K => Some('k'),
            egui::Key::L => Some('l'),
            egui::Key::M => Some('m'),
            egui::Key::N => Some('n'),
            egui::Key::O => Some('o'),
            egui::Key::P => Some('p'),
            egui::Key::Q => Some('q'),
            egui::Key::R => Some('r'),
            egui::Key::S => Some('s'),
            egui::Key::T => Some('t'),
            egui::Key::U => Some('u'),
            egui::Key::V => Some('v'),
            egui::Key::W => Some('w'),
            egui::Key::X => Some('x'),
            egui::Key::Y => Some('y'),
            egui::Key::Z => Some('z'),
            _ => None,
        }
    }

    /// Execute a navigation command
    fn execute_navigation_command(&mut self, command: NavigationCommand) {
        match command {
            NavigationCommand::ScrollVertical(amount) => {
                self.handle_scroll_command(amount, false);
            }
            NavigationCommand::ScrollHorizontal(amount) => {
                self.handle_scroll_command(amount, true);
            }
            NavigationCommand::NextWindow => {
                self.focus_next_window();
            }
            NavigationCommand::PreviousWindow => {
                self.focus_previous_window();
            }
            NavigationCommand::CloseWindow => {
                self.close_focused_window();
            }
            NavigationCommand::WindowByIndex(index) => {
                self.focus_window_by_index(index);
            }
            NavigationCommand::LastWindow => {
                self.focus_last_window();
            }
            NavigationCommand::MoveToTop => {
                // Scroll to top (large negative scroll)
                self.handle_scroll_command(-1000, false);
            }
            NavigationCommand::MoveToBottom => {
                // Scroll to bottom (large positive scroll)
                self.handle_scroll_command(1000, false);
            }
            NavigationCommand::OpenCommandPalette => {
                self.show_command_palette = true;
                self.set_focused_window(FocusedWindow::CommandPalette);
            }
            NavigationCommand::EnterHintMode(action) => {
                // Enter hint mode with specified action
                self.navigation_state.push_mode(NavigationMode::Hint);

                // Get collected elements from the widget manager
                let collected_elements = self
                    .widget_manager
                    .collector()
                    .get_enabled_elements()
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                tracing::info!(
                    "EnterHintMode: Widget manager provided {} elements",
                    collected_elements.len()
                );

                // Collect elements from all open windows
                let elements = collected_elements;

                // Resource/template editor windows removed
                // // Add elements from ResourceFormWindow if it's open
                // if self.resource_form_window.is_open() {
                //     let form_elements = self.resource_form_window.collect_navigable_elements();
                //     tracing::info!(
                //         "EnterHintMode: ResourceFormWindow is open, added {} elements",
                //         form_elements.len()
                //     );
                //     elements.extend(form_elements);
                // } else {
                //     tracing::info!("EnterHintMode: ResourceFormWindow is not open");
                // }

                // Resource/template editor windows removed
                // // Add elements from ResourceTypesWindow if it's open
                // if self.resource_types_window.is_open() {
                //     let types_elements = self.resource_types_window.collect_navigable_elements();
                //     tracing::info!(
                //         "EnterHintMode: ResourceTypesWindow is open, added {} elements",
                //         types_elements.len()
                //     );
                //     elements.extend(types_elements);
                // } else {
                //     tracing::info!("EnterHintMode: ResourceTypesWindow is not open");
                // }

                // Resource/template editor windows removed
                // // Add elements from TemplateSectionsWindow if it's open
                // if self.template_sections_window.is_open() {
                //     let template_elements =
                //         self.template_sections_window.collect_navigable_elements();
                //     // R3.2 testing logs - only show if debug logging would be enabled (simplified check)
                //     #[cfg(debug_assertions)]
                //     {
                //         tracing::debug!("🎯 R3.2 HINT TESTING - EnterHintMode: TemplateSectionsWindow is open, collected {} real elements", template_elements.len());
                //     }
                //     elements.extend(template_elements);
                // }

                // Add elements from other windows
                if self.help_window.is_open() {
                    tracing::info!("EnterHintMode: HelpWindow is open - but collect_navigable_elements is not implemented yet");
                    // TODO: Add collect_navigable_elements to HelpWindow when implemented
                } else {
                    tracing::info!("EnterHintMode: HelpWindow is not open");
                }

                if self.log_window.is_open() {
                    tracing::info!("EnterHintMode: LogWindow is open - but collect_navigable_elements is not implemented yet");
                    // TODO: Add collect_navigable_elements to LogWindow when implemented
                } else {
                    tracing::info!("EnterHintMode: LogWindow is not open");
                }

                if self.show_command_palette {
                    tracing::info!("EnterHintMode: CommandPalette is open - but collect_navigable_elements is not implemented yet");
                    // TODO: Add collect_navigable_elements to CommandPalette when implemented
                } else {
                    tracing::info!("EnterHintMode: CommandPalette is not open");
                }

                // R3.2 testing logs - only in debug builds
                #[cfg(debug_assertions)]
                {
                    tracing::debug!("🎯 R3.2 HINT TESTING - EnterHintMode: Total collected elements from all sources: {}", elements.len());

                    // R3.2 validation logging
                    if elements.len() >= 80 {
                        tracing::info!("✅ R3.2 SUCCESS: Hint mode activated with {} elements (exceeding 80+ target!)", elements.len());
                    } else if !elements.is_empty() {
                        tracing::info!("⚠️ R3.2 PARTIAL: Hint mode activated with {} elements (below 80+ target)", elements.len());
                    } else {
                        tracing::warn!("❌ R3.2 FAILURE: Hint mode activated with 0 elements - no widgets to navigate!");
                    }
                }

                // Start hint mode - logging happens in hint_mode.start()
                self.hint_mode.start(action, &elements);
                // Set flag to skip the next hint input (prevents activation key double-processing)
                self.skip_next_hint_input = true;
            }
            NavigationCommand::NextElement => {
                // Navigate to next focusable element
                info!("Next element");
            }
            NavigationCommand::PreviousElement => {
                // Navigate to previous focusable element
                info!("Previous element");
            }
            NavigationCommand::ActivateElement => {
                // Activate currently focused element
                info!("Activate element");
            }
            NavigationCommand::FocusSearchField => {
                // Focus search field in current window
                info!("Focus search field");
                // This is typically handled by individual windows
            }
        }
    }

    /// Process pending scroll requests from navigation commands
    fn process_pending_scroll_requests(&mut self, _ctx: &egui::Context) {
        if let Some(scroll_amount) = self.pending_scroll_request.take() {
            // For now, we'll store the scroll request and apply it during window rendering
            // egui doesn't allow injecting scroll events directly, so we'll need to
            // coordinate with individual windows to handle scrolling
            info!("Processing scroll request: {} pixels", scroll_amount);

            // Store scroll request for windows to consume
            self.apply_scroll_to_focused_window(scroll_amount);
        }
    }

    /// Apply scroll to the currently focused window
    fn apply_scroll_to_focused_window(&mut self, scroll_amount: f32) {
        // Based on the currently focused window, apply scrolling
        match self.currently_focused_window {
            // Resource/template editor windows removed
            // Some(FocusedWindow::ResourceTypes) => {
            //     // For now, just log that we would scroll the resource types window
            //     info!("Scrolling ResourceTypes window by {} pixels", scroll_amount);
            //     // TODO: Add scroll state to ResourceTypesWindow
            // }
            // Resource/template editor windows removed
            // Some(FocusedWindow::ResourceForm) => {
            //     info!("Scrolling ResourceForm window by {} pixels", scroll_amount);
            //     // TODO: Add scroll state to ResourceFormWindow
            // }
            // CloudFormation graph removed
            Some(FocusedWindow::Help) => {
                info!("Scrolling Help window by {} pixels", scroll_amount);
                // TODO: Add scroll state to HelpWindow
            }
            Some(FocusedWindow::Log) => {
                info!("Scrolling Log window by {} pixels", scroll_amount);
                // TODO: Add scroll state to LogWindow
            }
            _ => {
                info!("No focused window or unsupported window for scrolling");
            }
        }
    }

    /// Handle scroll commands by sending scroll events to egui
    fn handle_scroll_command(&mut self, amount: i32, horizontal: bool) {
        // Store scroll command to be applied on next frame
        // egui handles scrolling through input events, so we'll simulate scroll wheel events
        if horizontal {
            info!("Horizontal scroll: {}", amount);
            // For now, just log horizontal scrolling
            // TODO: Implement horizontal scrolling when needed
        } else {
            // Vertical scrolling - simulate mouse wheel
            let scroll_amount = amount as f32 * 20.0; // Convert to pixels
            info!("Vertical scroll: {} pixels", scroll_amount);

            // Store the scroll request for the next frame
            // egui will process this during the next update cycle
            self.pending_scroll_request = Some(scroll_amount);
        }
    }

    /// Focus the next window in the window order
    fn focus_next_window(&mut self) {
        // Implement window cycling logic
        // Resource/template editor windows removed
        if let Some(current) = self.currently_focused_window {
            // For now, just cycle through a few common windows
            match current {
                // Resource/template editor windows removed
                // FocusedWindow::ResourceTypes => {
                //     self.resource_form_window.show = true;
                //     self.set_focused_window(FocusedWindow::ResourceForm);
                // }
                // Resource/template editor windows removed
                // FocusedWindow::ResourceForm => {
                //     self.help_window.open = true;
                //     self.set_focused_window(FocusedWindow::Help);
                // }
                // CloudFormation graph removed
                FocusedWindow::Help => {
                    // Resource/template editor windows removed
                    // self.resource_types_window.show = true;
                    // self.set_focused_window(FocusedWindow::ResourceTypes);
                }
                _ => {
                    // Resource/template editor windows removed
                    // self.resource_types_window.show = true;
                    // self.set_focused_window(FocusedWindow::ResourceTypes);
                }
            }
        } else {
            // Resource/template editor windows removed
            // self.resource_types_window.show = true;
            // self.set_focused_window(FocusedWindow::ResourceTypes);
        }
    }

    /// Focus the previous window in the window order
    fn focus_previous_window(&mut self) {
        // Implement reverse window cycling logic
        // Resource/template editor windows removed
        if let Some(current) = self.currently_focused_window {
            match current {
                // Resource/template editor windows removed
                // FocusedWindow::ResourceTypes => {
                //     self.help_window.open = true;
                //     self.set_focused_window(FocusedWindow::Help);
                // }
                // Resource/template editor windows removed
                // FocusedWindow::ResourceForm => {
                //     self.resource_types_window.show = true;
                //     self.set_focused_window(FocusedWindow::ResourceTypes);
                // }
                // CloudFormation graph removed
                FocusedWindow::Help => {
                    // Resource/template editor windows removed
                    // self.resource_form_window.show = true;
                    // self.set_focused_window(FocusedWindow::ResourceForm);
                }
                _ => {
                    self.help_window.open = true;
                    self.set_focused_window(FocusedWindow::Help);
                }
            }
        } else {
            self.help_window.open = true;
            self.set_focused_window(FocusedWindow::Help);
        }
    }

    /// Focus a window by its index (1-9)
    fn focus_window_by_index(&mut self, index: u8) {
        let window = match index {
            // Resource/template editor windows removed
            // 1 => {
            //     self.resource_types_window.show = true;
            //     FocusedWindow::ResourceTypes
            // }
            // 2 => {
            //     self.resource_form_window.show = true;
            //     FocusedWindow::ResourceForm
            // }
            // 3 => {
            //     self.resource_json_editor_window.show = true;
            //     FocusedWindow::ResourceJsonEditor
            // }
            // 4 => {
            //     // CloudFormation scene graph removed - skip to next window
            //     self.template_sections_window.show = true;
            //     FocusedWindow::TemplateSections
            // }
            // 5 => {
            //     self.template_sections_window.show = true;
            //     FocusedWindow::TemplateSections
            // }
            6 => {
                self.help_window.open = true;
                FocusedWindow::Help
            }
            7 => {
                self.log_window.open = true;
                FocusedWindow::Log
            }
            8 => {
                // Chat window removed
                FocusedWindow::Chat
            }
            9 => {
                self.credentials_debug_window.open = true;
                FocusedWindow::CredentialsDebug
            }
            _ => return,
        };

        self.set_focused_window(window);
    }

    /// Focus the last active window
    fn focus_last_window(&mut self) {
        if let Some(last_window) = self.window_focus_order.last().copied() {
            self.set_focused_window(last_window);
        }
    }

    /// Legacy keyboard input handling for backwards compatibility
    fn handle_legacy_keyboard_input(&mut self, ctx: &egui::Context) {
        // Space to open command palette (when navigation is in insert mode or disabled)
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) && !ctx.wants_keyboard_input() {
            match self.navigation_state.current_mode() {
                NavigationMode::Insert | NavigationMode::Command => {
                    self.show_command_palette = true;
                    self.set_focused_window(FocusedWindow::CommandPalette);
                }
                _ => {
                    // In normal/hint/visual modes, space is handled by navigation system
                }
            }
        }

        // F1 to open chat window - REMOVED (chat window deleted)

        // Ctrl+G to open CloudFormation graph window
        // Project management removed
        // if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::G))
        //     && !ctx.wants_keyboard_input()
        // {
        //     info!("Ctrl+G pressed - opening CloudFormation scene graph");
        //     if let Some(project) = &self.project_command_palette.current_project {
        //         info!(
        //             "Current project: {} has {} CloudFormation resources",
        //             project.name,
        //             project.get_resources().len()
        //         );
        //
        //         // CloudFormation scene graph removed
        //     } else {
        //         warn!("No project loaded - CloudFormation graph not available. Load a CloudFormation template first.");
        //     }
        // }

        // Windows+C to close windows (legacy code kept for compatibility)
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::C)) {
            // Resource/template editor windows removed
            // self.resource_types_window.show = false;
            // Resource/template editor windows removed
            // self.resource_details_window.show = false;
            // Resource/template editor windows removed
            // self.resource_types_window.selected_resource_index = None;
        }
    }

    /// Render the top menu bar
    fn render_top_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // Project management removed
                // let project_info = self.project_command_palette.get_current_project_summary();
                let project_info = None;

                // Get resource count if a project is loaded
                // Project management removed
                // let resource_count = self
                //     .project_command_palette
                //     .current_project
                //     .as_ref()
                //     .map(|project| project.get_resources().len());
                let resource_count = None;

                // Get compliance programs from current project
                // Project management removed
                // let compliance_programs = self.project_command_palette
                //     .current_project
                //     .as_ref()
                //     .map(|project| &project.compliance_programs);
                let compliance_programs = None;

                let (menu_action, selected_window) = menu::build_menu(
                    ui,
                    ctx,
                    &mut self.theme,
                    &mut self.navigation_status_bar_settings,
                    project_info,
                    &mut self.help_window.open,
                    &mut self.log_window.open,
                    resource_count,
                    self.aws_identity_center.as_ref(), // Pass AWS identity center for login status
                    &mut self.window_selector,
                    self.compliance_status.clone(),
                    compliance_programs,
                );

                // Handle menu actions
                match menu_action {
                    menu::MenuAction::ThemeChanged => {
                        tracing::info!("Theme changed");
                    }
                    menu::MenuAction::NavigationStatusBarChanged => {
                        tracing::info!("Navigation status bar setting changed to: {}", 
                                     self.navigation_status_bar_settings.show_status_bar);
                    }
                    menu::MenuAction::ShakeWindows => {
                        self.start_shake_animation();
                        tracing::info!("Shake animation started");
                    }
                    menu::MenuAction::ShowWindowSelector => {
                        // No longer needed, handled directly in menu
                    }
                    menu::MenuAction::ShowComplianceDetails => {
                        // Open the Guard Violations window
                        self.focus_window("guard_violations");
                        tracing::info!("Compliance details window opened");
                    }
                    menu::MenuAction::ValidateCompliance => {
                        // Trigger compliance validation
                        self.trigger_compliance_validation();
                        tracing::info!("Compliance validation triggered");
                    }
                    menu::MenuAction::None => {}
                }

                // Handle window selection from the window selector menu
                if let Some(window_id) = selected_window {
                    self.focus_window(&window_id);
                }
            });
        });
    }

    /// Render the navigation status bar showing current mode and key sequence
    fn render_navigation_status_bar(&mut self, ctx: &egui::Context) {
        if !self.navigation_status_bar_settings.show_status_bar {
            return;
        }
        
        egui::TopBottomPanel::top("navigation_status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Navigation mode indicator
                    let mode_text = match self.navigation_state.current_mode() {
                        NavigationMode::Normal => "NORMAL",
                        NavigationMode::Insert => "INSERT",
                        NavigationMode::Hint => "HINT",
                        NavigationMode::Visual => "VISUAL",
                        NavigationMode::Command => "COMMAND",
                    };

                    let mode_color = match self.navigation_state.current_mode() {
                        NavigationMode::Normal => egui::Color32::from_rgb(100, 150, 255), // Blue
                        NavigationMode::Insert => egui::Color32::from_rgb(100, 255, 100), // Green
                        NavigationMode::Hint => egui::Color32::from_rgb(255, 200, 100),   // Orange
                        NavigationMode::Visual => egui::Color32::from_rgb(255, 150, 255), // Magenta
                        NavigationMode::Command => egui::Color32::from_rgb(255, 255, 100), // Yellow
                    };

                    ui.colored_label(mode_color, format!("-- {} --", mode_text));

                    // Show current key sequence if any
                    let key_sequence = self.navigation_state.current_key_sequence();
                    if !key_sequence.is_empty() {
                        ui.separator();
                        ui.label(format!("Keys: {}", key_sequence));
                    }

                    // Show command count if any
                    if let Some(count) = self.navigation_state.current_command_count() {
                        ui.separator();
                        ui.label(format!("Count: {}", count));
                    }

                    // Show hint mode information
                    if self.hint_mode.is_active() {
                        ui.separator();
                        let hint_filter = self.hint_mode.current_filter();
                        if !hint_filter.is_empty() {
                            ui.label(format!("Filter: {}", hint_filter));
                        }

                        let visible_hints = self.hint_mode.visible_hints().len();
                        ui.label(format!("Hints: {}", visible_hints));
                    }

                    // Error/warning notifications
                    self.notification_manager.render_status_bar_indicator(ui);

                    // Add some spacing to push the next element to the right
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Show focused window info
                        if let Some(focused) = self.currently_focused_window {
                            let window_name = match focused {
                                // Resource/template editor windows removed
                                // FocusedWindow::ResourceTypes => "Resource Types",
                                // Resource/template editor windows removed
                                // FocusedWindow::ResourceForm => "Resource Form",
                                // Resource/template editor windows removed
                                // FocusedWindow::ResourceJsonEditor => "JSON Editor",
                                // CloudFormation graph removed
                                // Resource/template editor windows removed
                                // FocusedWindow::TemplateSections => "Template Sections",
                                FocusedWindow::Help => "Help",
                                FocusedWindow::Log => "Log",
                                FocusedWindow::Chat => "Chat",
                                FocusedWindow::CredentialsDebug => "Credentials",
                                _ => "Other",
                            };
                            ui.weak(format!("Focus: {}", window_name));
                        }
                    });
                });
            });
    }

    /// Handle download-related operations
    fn handle_downloads(&mut self) {
        // Only update the download status, no auto-download
        // Download manager removed
    }

    /// Render the central panel with content
    fn render_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render the main content with resource grid
            self.render_main_content_area(ctx, ui);
        });
    }

    /// Handle resource types and details windows
    fn handle_resource_windows(&mut self, _ctx: &egui::Context) {
        // Resource/template editor windows removed
        // self.handle_resource_types_window(ctx);
        // self.handle_resource_details_window(ctx);
    }

    /// Handle the resource types window
    // Resource/template editor windows removed
    /*
    fn handle_resource_types_window(&mut self, ctx: &egui::Context) {
        // Resource/template editor windows removed
        // if self.resource_types_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            // Resource/template editor windows removed
            // if self.currently_focused_window != Some(FocusedWindow::ResourceTypes) {
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::ResourceTypes);
            }

            // Check if this window should be brought to the front
            // Resource/template editor windows removed
            // let window_id = self.resource_types_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window with focus management and check if a resource was selected
            if let Some(resource_type) = self
                .resource_types_window
                .show_with_focus(ctx, bring_to_front)
            {
                // Close the resource types window after selection
                // Resource/template editor windows removed
                // self.resource_types_window.show = false;

                if self.project_command_palette.current_project.is_some() {
                    // Open the resource form window for a new resource
                    let project = self
                        .project_command_palette
                        .current_project
                        .as_ref()
                        .unwrap()
                        .clone();

                    // Reset previous form and open a new one
                    self.resource_form_window = ResourceFormWindow::new();
                    // Resource/template editor windows removed
                    // self.resource_form_window.open_new(
                        resource_type.clone(),
                        &project,
                        |_| {}, // Empty closure for now, we'll handle saving in the show method
                    );
                    // Resource/template editor windows removed
                    // self.set_focused_window(FocusedWindow::ResourceForm);
                } else {
                    // Fallback to showing resource details if no project is open
                    self.open_resource_details(&resource_type);
                }
            }
        }
    }
    */

    /// Handle the resource details window
    // Resource/template editor windows removed
    /*
    fn handle_resource_details_window(&mut self, ctx: &egui::Context) {
        // Resource/template editor windows removed
        // if self.resource_details_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            // Resource/template editor windows removed
            // if self.currently_focused_window != Some(FocusedWindow::ResourceDetails) {
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::ResourceDetails);
            }

            // Check if this window should be brought to the front
            // Resource/template editor windows removed
            // let window_id = self.resource_details_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window with focus management and check if a property was selected
            if let Some(property_type) = self
                .resource_details_window
                .show_with_focus(ctx, bring_to_front)
            {
                self.open_property_type(&property_type);
            }
        }
    }
    */

    /// Handle property type windows
    // Resource/template editor windows removed
    /*
    fn handle_property_windows(&mut self, ctx: &egui::Context) {
        // Display all property type windows and get property types to open
        let (property_types_to_open, focused_window_idx) =
            // Resource/template editor windows removed
            // self.property_type_manager.show_windows(ctx);

        // Update focus if one of the property type windows was interacted with
        if let Some(idx) = focused_window_idx {
            // Resource/template editor windows removed
            // self.set_focused_window(FocusedWindow::PropertyType(idx));
        }

        // Open any properties that were requested from the property windows
        for property_type in property_types_to_open {
            self.open_property_type(&property_type);
        }

        // Always handle resource graph keyboard interaction
        if ctx.input(|i| {
            i.key_pressed(egui::Key::ArrowUp)
                || i.key_pressed(egui::Key::ArrowDown)
                || i.key_pressed(egui::Key::ArrowLeft)
                || i.key_pressed(egui::Key::ArrowRight)
                || i.key_down(egui::Key::ArrowUp)
                || i.key_down(egui::Key::ArrowDown)
                || i.key_down(egui::Key::ArrowLeft)
                || i.key_down(egui::Key::ArrowRight)
        }) {
            // Ensure continuous repainting for smooth panning
            ctx.request_repaint();
        }
    }
    */

    /// Handle the command palettes (main, project, and cloudformation)
    // Project management removed - this method heavily relied on project_command_palette
    fn handle_command_palettes(&mut self, ctx: &egui::Context) {
        // Display command palette if open
        self.ui_command_palette(ctx);

        // Project command palette handling removed
        /*

        // Show project command palette if open
        if self.project_command_palette.mode != ProjectCommandAction::Closed {
            // Set focus based on mode - only if not already focused to avoid stealing focus every frame
            if self.project_command_palette.mode == ProjectCommandAction::CommandPalette {
                if self.currently_focused_window != Some(FocusedWindow::ProjectCommandPalette) {
                    self.set_focused_window(FocusedWindow::ProjectCommandPalette);
                }
            } else if self.currently_focused_window != Some(FocusedWindow::ProjectForm) {
                self.set_focused_window(FocusedWindow::ProjectForm);
            }

            // Set AWS Identity Center if available
            self.project_command_palette
                .set_aws_identity_center(self.aws_identity_center.clone());

            // Track whether there was a project before showing
            let had_project = self.project_command_palette.current_project.is_some();

            // Show the window and handle project save
            let project_saved = self.project_command_palette.show(ctx);

            // Check if a project is now loaded (or was saved)
            let has_project = self.project_command_palette.current_project.is_some();

            // Update global project state for bridge tools when project state changes
            if has_project != had_project || project_saved {
                info!("🔥 APP DEBUG: Project state changed - updating global bridge state");
                info!("🔥 APP DEBUG: had_project: {}, has_project: {}, project_saved: {}", 
                    had_project, has_project, project_saved);
                    
                if let Some(project) = &self.project_command_palette.current_project {
                    // Project is loaded - update global state
                    info!("🔥 APP DEBUG: Setting global project: '{}'", project.name);
                    info!("🔥 APP DEBUG: Project has template: {}", project.cfn_template.is_some());
                    let project_clone = project.clone();
                    set_global_current_project(Some(Arc::new(Mutex::new(project_clone))));
                    info!("🔥 APP DEBUG: Global project state updated successfully");
                } else {
                    // No project loaded - clear global state
                    info!("🔥 APP DEBUG: No project loaded - clearing global state");
                    set_global_current_project(None);
                    info!("🔥 APP DEBUG: Global project state cleared");
                }
            } else {
                info!("🔥 APP DEBUG: No project state change detected - not updating global state");
            }

            // If project state changed (loaded or saved), trigger appropriate actions
            if (has_project && !had_project) || project_saved {
                // Check if we should trigger validation before borrowing the project
                let should_trigger_validation = if let Some(project) = &self.project_command_palette.current_project {
                    project_saved && !project.compliance_programs.is_empty() && project.guard_rules_enabled
                } else {
                    false
                };

                if let Some(project) = &self.project_command_palette.current_project {
                    // Project loaded/saved
                    tracing::info!("🔄 APP_RESPONSE: Project loaded/saved successfully");

                    // Resource/template editor windows removed
                    // // Load the CloudFormation template if available
                    // if let Some(cfn_template) = &project.cfn_template {
                    //     self.template_sections_window
                    //         .set_template(cfn_template.clone());
                    //     tracing::info!(
                    //         "🔄 APP_RESPONSE: CloudFormation template loaded into template sections window"
                    //     );
                    // }

                    // CloudFormation scene graph removed

                    // Show the template sections window (focus will be set by handle_template_sections_window)
                    // Resource/template editor windows removed
                    // self.template_sections_window.show = true;
                    // Resource/template editor windows removed
                    // self.template_sections_window.selected_section =
                        // Resource/template editor windows removed
                        // super::template_sections_window::TemplateSection::Resources;
                }
                
                // Trigger validation if project was saved with compliance programs
                if should_trigger_validation {
                    tracing::info!("Project saved with compliance programs - triggering automatic validation");
                    self.trigger_compliance_validation();
                }
            }
        }
        */
    }

    /// Handle authentication windows
    fn handle_auth_windows(&mut self, ctx: &egui::Context) {
        // Show AWS login window if open
        if self.aws_login_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(FocusedWindow::AwsLogin) {
                self.set_focused_window(FocusedWindow::AwsLogin);
            }

            // Check if this window should be brought to the front
            let window_id = self.aws_login_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Get position for window
            let window_pos = self.get_window_position("aws_login").unwrap_or_default();

            // Show the window with focus management and get results
            let (aws_identity, window_rect) =
                self.aws_login_window
                    .show_with_focus(ctx, Some(window_pos), bring_to_front);

            // Update position tracking
            if let Some(rect) = window_rect {
                self.update_window_position("aws_login".to_string(), rect.min);
            }

            if let Some(aws_identity) = aws_identity {
                // Check if this is a new successful login
                let was_logged_in_before =
                    if let Some(existing_identity) = &self.aws_identity_center {
                        if let Ok(identity) = existing_identity.lock() {
                            matches!(
                                identity.login_state,
                                crate::app::aws_identity::LoginState::LoggedIn
                            )
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                // Update the identity center reference
                self.aws_identity_center = Some(aws_identity.clone());

                // Set global AwsIdentity for bridge tools
                set_global_aws_identity(Some(aws_identity.clone()));

                // Proactively initialize ResourceExplorer with AWS Identity Center
                // This ensures the AWS client is available for bridge tools even if the window isn't open
                self.resource_explorer
                    .set_aws_identity_center(Some(aws_identity.clone()));
                tracing::info!("🚀 ResourceExplorer proactively initialized for bridge tools");

                // CloudFormation manager removed
                // Initialize CloudFormation Manager with shared AWS Explorer infrastructure
                // if self.cloudformation_manager.is_none() {
                //     if let Ok(identity) = aws_identity.lock() {
                //         let default_role_name = identity.default_role_name.clone();
                //         drop(identity); // Release the lock early
                //
                //         let credential_coordinator = std::sync::Arc::new(
                //             crate::app::resource_explorer::credentials::CredentialCoordinator::new(
                //                 aws_identity.clone(),
                //                 default_role_name,
                //             ),
                //         );
                //         let mut manager = CloudFormationManager::new(credential_coordinator);
                //
                //         // Set AWS client from ResourceExplorer if available
                //         if let Some(aws_client) = self.resource_explorer.get_aws_client() {
                //             manager.set_aws_client(Some(aws_client));
                //         }
                //
                //         self.cloudformation_manager = Some(std::sync::Arc::new(manager));
                //     }
                // }

                // Check if we just completed login
                let is_logged_in_now = if let Ok(identity) = aws_identity.lock() {
                    matches!(
                        identity.login_state,
                        crate::app::aws_identity::LoginState::LoggedIn
                    )
                } else {
                    false
                };

                // Log when transitioning from not logged in to logged in
                if !was_logged_in_before && is_logged_in_now {
                    tracing::info!("AWS login successful");
                    // Note: Shake animation now triggered when credentials debug window opens
                }
            } else if self.aws_identity_center.is_some() && self.aws_login_window.logged_out {
                // If login window returns None and we previously had an identity,
                // and the logged_out flag is set, it means user logged out
                tracing::info!("Clearing AWS Identity Center reference due to logout");
                self.aws_identity_center = None;
                // CloudFormation manager removed
                // self.cloudformation_manager = None;

                // Clear global AwsIdentity for bridge tools
                set_global_aws_identity(None);

                // Clear ResourceExplorer AWS client
                self.resource_explorer.set_aws_identity_center(None);
                tracing::info!("🧹 ResourceExplorer cleared on logout");
            }

            // Check if the accounts window is open and set focus
            if self.aws_login_window.accounts_window_open {
                self.set_focused_window(FocusedWindow::AwsAccounts);
            }

            // Check if the credentials debug window should be opened
            if self.aws_login_window.credentials_debug_window_open {
                self.credentials_debug_window.open = true;
                self.set_focused_window(FocusedWindow::CredentialsDebug);
                // Trigger shake animation when credentials are ready and debug window opens
                tracing::info!("AWS credentials obtained and debug window opened");
                self.start_delayed_shake_animation();
                // Reset the flag as we've handled it
                self.aws_login_window.credentials_debug_window_open = false;
            }
        }
    }

    /// Handle startup popup
    fn handle_startup_popup(&mut self, ctx: &egui::Context) {
        // Show startup popup if needed
        if self.show_startup_popup {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(FocusedWindow::StartupPopup) {
                self.set_focused_window(FocusedWindow::StartupPopup);
            }
            self.show_startup_popup(ctx);
        }
    }

    /// Render debug panel
    fn render_debug_panel(&mut self, ctx: &egui::Context) {
        // Add debug build warning to bottom right corner
        egui::TopBottomPanel::bottom("bottom_panel")
            .show_separator_line(false)
            .resizable(false)
            .min_height(0.0)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    // Show custom debug info with git information
                    if cfg!(debug_assertions) {
                        let git_branch = env!("GIT_BRANCH");
                        let git_commit = env!("GIT_COMMIT");
                        ui.label(
                            egui::RichText::new(format!(
                                "Debug Build - {}@{}",
                                git_branch,
                                git_commit
                            ))
                            .small()
                            .color(egui::Color32::from_rgb(255, 165, 0)) // Orange color
                        );
                    }
                });
            });
    }

    /// Handle continuous repainting logic
    fn handle_continuous_repainting(&mut self, ctx: &egui::Context) {
        // Request continuous redrawing when any window is open
        // Resource/template editor windows removed
        // if self.resource_types_window.show
        //     || self.resource_details_window.show
        //     || self.resource_form_window.show
        //     || !self.property_type_manager.windows.is_empty()
        if self.show_command_palette
            || self.show_startup_popup
            // Project management removed
            // || self.project_command_palette.mode != ProjectCommandAction::Closed
            || self.help_window.open
            || self.log_window.open
            || self.credentials_debug_window.open
            || self.verification_window.visible
            || self.resource_explorer.is_open()
        {
            ctx.request_repaint();
        }

        // Always repaint when we have a project loaded to keep the resource graph responsive
        // Project management removed
        // if self.project_command_palette.current_project.is_some() {
        //     ctx.request_repaint();
        // }
    }

    /// Handle the chat window - REMOVED (chat window deleted)
    fn handle_chat_window(&mut self, _ctx: &egui::Context) {
        // Chat window removed in Phase 1.2
    }

    /// Handle the Control Bridge window - closable like other windows
    fn handle_control_bridge_window(&mut self, ctx: &egui::Context) {
        // Only show if window is open
        if !self.control_bridge_window.open {
            return;
        }

        // Only set focus if this window is not already focused to avoid stealing focus every frame
        if self.currently_focused_window != Some(FocusedWindow::ControlBridge) {
            self.set_focused_window(FocusedWindow::ControlBridge);
        }

        // Check if this window should be brought to the front
        let window_id = self.control_bridge_window.window_id();
        let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
        if bring_to_front {
            self.window_focus_manager.clear_bring_to_front(window_id);
        }

        // Control Bridge requires AWS Identity Center login
        if let Some(aws_identity) = &self.aws_identity_center {
            let params = IdentityShowParams {
                aws_identity: Some(aws_identity.clone()),
            };
            FocusableWindow::show_with_focus(
                &mut self.control_bridge_window,
                ctx,
                params,
                bring_to_front,
            );
        } else {
            // Show login requirement message when not logged in
            let mut is_open = self.control_bridge_window.open;
            let mut open_login = false;
            egui::Window::new("🚢 Control Bridge")
                .open(&mut is_open)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label("🔐 AWS Identity Center login required");
                        ui.add_space(10.0);
                        ui.label("Please log in to AWS Identity Center to use the Control Bridge.");
                        ui.add_space(10.0);
                        if ui.button("Open Login Window").clicked() {
                            open_login = true;
                        }
                        ui.add_space(20.0);
                    });
                });
            self.control_bridge_window.open = is_open;

            if open_login {
                self.aws_login_window.open = true;
                self.set_focused_window(FocusedWindow::AwsLogin);
            }
        }
    }

    /// Handle the credentials debug window
    fn handle_credentials_debug_window(&mut self, ctx: &egui::Context) {
        if self.credentials_debug_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(FocusedWindow::CredentialsDebug) {
                self.set_focused_window(FocusedWindow::CredentialsDebug);
            }

            // Check if this window should be brought to the front
            let window_id = self.credentials_debug_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window with AWS identity information
            self.credentials_debug_window.show_with_focus(
                ctx,
                self.aws_identity_center.as_ref(),
                None, // window position
                bring_to_front,
            );
        }
    }

    /// Handle the deployment info window
// Resource/template editor windows removed
    /*
    fn handle_deployment_info_window(&mut self, ctx: &egui::Context) {
        // Resource/template editor windows removed
        // if self.deployment_info_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            // Resource/template editor windows removed
            // if self.currently_focused_window != Some(FocusedWindow::DeploymentInfo) {
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::DeploymentInfo);
            }

            // Check if this window should be brought to the front
            // Resource/template editor windows removed
            // let window_id = self.deployment_info_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Get notification and deployment status data
            // Project management removed
            // let current_environment = if let Some(project) = &self.project_command_palette.current_project {
            //     project
            //         .environments
            //         .iter()
            //         .find(|env| env.deployment_status.is_some())
            //         .map(|env| env.name.clone())
            // } else {
            //     None
            // };
            let current_environment = None;

            let notification = if let Some(env_name) = &current_environment {
                self.notification_manager.get_deployment_status(env_name)
            } else {
                None
            };

            // Project management removed
            // let deployment_status = if let (Some(project), Some(env_name)) = (
            //     &self.project_command_palette.current_project,
            //     &current_environment,
            // ) {
            //     project
            //         .environments
            //         .iter()
            //         .find(|env| env.name == *env_name)
            //         .and_then(|env| env.deployment_status.as_ref())
            // } else {
            //     None
            // };
            let deployment_status = None;

            // Show the window with deployment information
            // Resource/template editor windows removed
            // self.deployment_info_window.show_with_focus(
                ctx,
                notification,
                deployment_status,
                bring_to_front,
            );
        }
    }
    */

    /// Handle the verification window
    fn handle_verification_window(&mut self, ctx: &egui::Context) {
        if self.verification_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(FocusedWindow::Verification) {
                self.set_focused_window(FocusedWindow::Verification);
            }

            // Check if this window should be brought to the front
            let window_id = self.verification_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window using the trait
            FocusableWindow::show_with_focus(
                &mut self.verification_window,
                ctx,
                (),
                bring_to_front,
            );
        }
    }

    /// Handle the AWS resource explorer window
    fn handle_resource_explorer_window(&mut self, ctx: &egui::Context) {
        if self.resource_explorer.is_open() {
            // Ensure resource explorer has access to AWS Identity Center for real account data
            self.resource_explorer
                .set_aws_identity_center(self.aws_identity_center.clone());

            // Show the resource explorer window
            self.resource_explorer.show(ctx);
        }
    }

    /// Handle the window selector
    fn handle_window_selector(&mut self, _ctx: &egui::Context) {
        // Update window tracking - the menu selection is handled in render_top_panel
        self.update_window_tracking();
    }

    /// Update the window tracking to reflect current window states
    fn update_window_tracking(&mut self) {
        // Resource/template editor windows removed
        // // Track Resource Form Window
        // if self.resource_form_window.show {
        //     let window_id = format!("resource_form_{}", self.resource_form_window.resource_id);
        //     let title = format!("Edit Resource: {}", self.resource_form_window.resource_id);
        //     self.window_selector
        //         .register_window(window_id, title, WindowType::ResourceForm);
        // } else {
        //     let window_id = format!("resource_form_{}", self.resource_form_window.resource_id);
        //     self.window_selector.unregister_window(&window_id);
        // }

        // Track Help Window
        if self.help_window.open {
            self.window_selector.register_window(
                "help_window".to_string(),
                "Help".to_string(),
                WindowType::HelpWindow,
            );
        } else {
            self.window_selector.unregister_window("help_window");
        }

        // Track Log Window
        if self.log_window.open {
            self.window_selector.register_window(
                "log_window".to_string(),
                "Log Viewer".to_string(),
                WindowType::LogWindow,
            );
        } else {
            self.window_selector.unregister_window("log_window");
        }

        // Track AWS Login Window
        if self.aws_login_window.open {
            self.window_selector.register_window(
                "aws_login_window".to_string(),
                "AWS Identity Center Login".to_string(),
                WindowType::Other("AWS Login".to_string()),
            );
        } else {
            self.window_selector.unregister_window("aws_login_window");
        }

        // Resource/template editor windows removed
        // // Track Resource Types Window
        // if self.resource_types_window.show {
        //     self.window_selector.register_window(
        //         "resource_types".to_string(),
        //         "CloudFormation Resource Types".to_string(),
        //         WindowType::ResourceTypes,
        //     );
        // } else {
        //     self.window_selector.unregister_window("resource_types");
        // }

        // Resource/template editor windows removed
        // // Track Resource Details Window
        // if self.resource_details_window.show {
        //     self.window_selector.register_window(
        //         "resource_details".to_string(),
        //         self.resource_details_window.window_title(),
        //         WindowType::ResourceDetails,
        //     );
        // } else {
        //     self.window_selector.unregister_window("resource_details");
        // }

        // Track CloudFormation Scene Graph - removed

        // Track Chat Window - REMOVED (chat window deleted)

        // Track Control Bridge Window - always open
        self.window_selector.register_window(
            "control_bridge".to_string(),
            "🚢 Control Bridge".to_string(),
            WindowType::Other("Control Bridge".to_string()),
        );

        // Track Credentials Debug Window
        if self.credentials_debug_window.open {
            self.window_selector.register_window(
                "credentials_debug".to_string(),
                "AWS Credentials Debug".to_string(),
                WindowType::CredentialsDebug,
            );
        } else {
            self.window_selector.unregister_window("credentials_debug");
        }

        // Resource/template editor windows removed
        // // Track Template Sections Window
        // if self.template_sections_window.show {
        //     self.window_selector.register_window(
        //         "template_sections".to_string(),
        //         "CloudFormation Template".to_string(),
        //         WindowType::TemplateSection,
        //     );
        // } else {
        //     self.window_selector.unregister_window("template_sections");
        // }

        // Resource/template editor windows removed
        // // Track Resource JSON Editor Window
        // if self.resource_json_editor_window.show {
        //     self.window_selector.register_window(
        //         "resource_json_editor".to_string(),
        //         "Resource JSON Editor".to_string(),
        //         WindowType::ResourceJsonEditor,
        //     );
        // } else {
        //     self.window_selector
        //         .unregister_window("resource_json_editor");
        // }

        // Track Verification Window
        if self.verification_window.visible {
            self.window_selector.register_window(
                "verification_window".to_string(),
                self.verification_window.window_title(),
                WindowType::Other("Verification".to_string()),
            );
        } else {
            self.window_selector
                .unregister_window("verification_window");
        }

        // TODO: Track Property Type windows from property_type_manager
        // This would require accessing the PropertyTypeWindowManager's windows
    }

    /// Focus a specific window by ID
    fn focus_window(&mut self, window_id: &str) {
        // Request focus through the focus manager
        self.window_focus_manager
            .request_focus(window_id.to_string());

        match window_id {
            "help_window" => {
                self.help_window.open = true;
                self.set_focused_window(FocusedWindow::Help);
            }
            "log_window" => {
                self.log_window.open = true;
                self.set_focused_window(FocusedWindow::Log);
            }
            "aws_login_window" => {
                self.aws_login_window.open = true;
                self.aws_login_window.reset_position(); // Reset to center window
                self.set_focused_window(FocusedWindow::AwsLogin);
            }
            "resource_types" => {
                // Resource/template editor windows removed
                // self.resource_types_window.show = true;
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::ResourceTypes);
            }
            "resource_details" => {
                // Resource/template editor windows removed
                // self.resource_details_window.show = true;
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::ResourceDetails);
            }
            "cloudformation_scene" => {
                // CloudFormation scene graph removed
            }
            "chat_window" => {
                // Chat window removed
            }
            "control_bridge" => {
                self.control_bridge_window.open = true;
                self.set_focused_window(FocusedWindow::ControlBridge);
            }
            "credentials_debug" => {
                self.credentials_debug_window.open = true;
                self.set_focused_window(FocusedWindow::CredentialsDebug);
            }
            "template_sections" => {
                // Resource/template editor windows removed
                // self.template_sections_window.show = true;
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::TemplateSections);
            }
            "resource_json_editor" => {
                // Resource/template editor windows removed
                // self.resource_json_editor_window.show = true;
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::ResourceJsonEditor);
            }
            "verification_window" => {
                self.verification_window.visible = true;
                self.set_focused_window(FocusedWindow::Verification);
            }
            "guard_violations" => {
                // Guard violations window removed in Phase 2.1
            }
            _ => {
                // Handle resource form windows with dynamic IDs
                if window_id.starts_with("resource_form_") {
                    // Resource/template editor windows removed
                    // self.resource_form_window.show = true;
                    // Resource/template editor windows removed
                    // self.set_focused_window(FocusedWindow::ResourceForm);
                }
                // TODO: Handle property type windows and other dynamic windows
            }
        }
    }

    /// Handle the template sections window
// Resource/template editor windows removed
    /*
    fn handle_template_sections_window(&mut self, ctx: &egui::Context) {
        // Resource/template editor windows removed
        // if self.template_sections_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            // Resource/template editor windows removed
            // if self.currently_focused_window != Some(FocusedWindow::TemplateSections) {
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::TemplateSections);
            }

            // Check if this window should be brought to the front
            // Resource/template editor windows removed
            // let window_id = self.template_sections_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Get position for window
            let window_pos = self.get_window_position(window_id);

            // Resource/template editor windows removed
            // let (command_result, window_rect) = self.template_sections_window.show_with_focus(
                ctx,
                self.project_command_palette.current_project.as_ref(),
                window_pos,
                bring_to_front,
            );

            // Update position tracking
            if let Some(rect) = window_rect {
                self.update_window_position(window_id.to_string(), rect.min);
            }

            if let Some(command_result) = command_result {
                match command_result {
                    // Resource/template editor windows removed
                    // super::template_sections_window::CommandResult::TemplateUpdated(
                        updated_template,
                    ) => {
                        // Save the updated template back to the project
                        if let Some(project) = &mut self.project_command_palette.current_project {
                            project.cfn_template = Some(*updated_template);
                            // Save the project to disk
                            if let Err(e) = project.save_all_resources() {
                                tracing::error!("Failed to save CloudFormation template: {}", e);
                            } else {
                                tracing::info!("CloudFormation template saved successfully");
                            }
                        }
                    }
                    // Resource/template editor windows removed
                    // super::template_sections_window::CommandResult::EditResource(resource_id) => {
                        // Open the resource form window for editing
                        let resource_id_owned = resource_id.clone();

                        // Get resource from project
                        let project_resource =
                            if let Some(project) = &self.project_command_palette.current_project {
                                project.get_resource(&resource_id_owned)
                            } else {
                                None
                            };

                        if let Some(project_resource) = project_resource {
                            // Use the resource from project (has current state including metadata)
                            if let Some(project) = &self.project_command_palette.current_project {
                                // Resource/template editor windows removed
                                // self.resource_form_window.open_edit(
                                    project_resource,
                                    project,
                                    |_resource| {
                                        // Callback handled in handle_resource_form_window
                                    },
                                );
                            }
                        } else {
                            // Fallback to template resource
                            tracing::warn!(
                                "Resource {} not found in DAG, falling back to template",
                                resource_id_owned
                            );
                            self.fallback_to_template_resource_by_id(&resource_id_owned);
                        }
                    }
                    // Resource/template editor windows removed
                    // super::template_sections_window::CommandResult::DeleteResource(resource_id) => {
                        // Delete the resource
                        if let Some(project) = &mut self.project_command_palette.current_project {
                            match project.remove_resource(&resource_id) {
                                Ok(_) => {
                                    tracing::info!(
                                        "Successfully deleted resource: {}",
                                        resource_id
                                    );
                                    // Clear any error message
                                    // Resource/template editor windows removed
                                    // self.template_sections_window.error_message = None;

                                    // Update the template sections window with the latest template
                                    if let Some(cfn_template) = &project.cfn_template {
                                        self.template_sections_window
                                            .set_template(cfn_template.clone());
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to delete resource {}: {}",
                                        resource_id,
                                        e
                                    );
                                    // Resource/template editor windows removed
                                    // self.template_sections_window.error_message = Some(format!(
                                        "Failed to delete resource '{}': {}",
                                        resource_id, e
                                    ));
                                }
                            }
                        }
                    }
                    // Resource/template editor windows removed
                    // super::template_sections_window::CommandResult::JsonEditResource(
                        resource_id,
                    ) => {
                        // Open the JSON editor for the resource
                        let resource_id_owned = resource_id.clone();

                        // Try to get resource from DAG first (preferred - has current state)
                        let resource_for_editor = if let Some(project) =
                            &self.project_command_palette.current_project
                        {
                            if let Some(project_resource) = project.get_resource(&resource_id_owned)
                            {
                                // Use project resource (convert to cfn_template::Resource)
                                Some(project_resource.to_cfn_resource())
                            } else {
                                // Fallback to template resource
                                tracing::warn!("Resource {} not found in project for JSON editing, falling back to template", resource_id_owned);
                                project
                                    .cfn_template
                                    .as_ref()
                                    .and_then(|template| template.resources.get(&resource_id_owned))
                                    .cloned()
                            }
                        } else {
                            None
                        };

                        if let Some(resource) = resource_for_editor {
                            // Open the JSON editor with a simple save handler
                            // Resource/template editor windows removed
                            // self.resource_json_editor_window.open_for_resource(
                                resource_id_owned,
                                resource,
                                move |res_id, _updated_resource| {
                                    // The actual save will be handled in handle_resource_json_editor_window
                                    info!("JSON editor save requested for resource: {}", res_id);
                                },
                            );

                            // Resource/template editor windows removed
                            // self.set_focused_window(FocusedWindow::ResourceJsonEditor);
                        } else {
                            tracing::error!(
                                "Resource {} not found for JSON editing",
                                resource_id_owned
                            );
                        }
                    }
                }
            }
        }
    }
    */

    /// Fallback method to load resource from template when DAG is not available
    // Project management removed
    #[allow(dead_code)]
    fn fallback_to_template_resource_by_id(&mut self, _resource_id: &str) {
        // Project management removed
        // if let Some(project) = &self.project_command_palette.current_project {
        //     if let Some(cfn_template) = &project.cfn_template {
        //         if let Some(resource) = cfn_template.resources.get(resource_id) {
        //             // Create a CloudFormationResource from the template resource
        //             // Resource/template editor windows removed
        //             // let cfn_resource =
        //             //     crate::app::projects::CloudFormationResource::from_cfn_resource(
        //             //         resource_id.to_string(),
        //             //         resource,
        //             //     );
        //
        //             // // Open the resource form window for editing
        //             // self.resource_form_window
        //             //     .open_edit(cfn_resource, project, |_resource| {
        //             //         // Callback handled in handle_resource_form_window
        //             //     });
        //         } else {
        //             tracing::error!("Resource {} not found in template either", resource_id);
        //         }
        //     } else {
        //         tracing::error!(
        //             "No CloudFormation template available for resource {}",
        //             resource_id
        //         );
        //     }
        // } else {
        //     tracing::error!("No project available for resource {}", resource_id);
        // }
    }

    /// Generic handler for simple focusable windows
    ///
    /// This method provides a consistent pattern for handling windows that implement
    /// the FocusableWindow trait with SimpleShowParams. It handles focus requests,
    /// window state management, and calls the trait's show_with_focus method.
    ///
    /// Note: Currently not used due to borrowing constraints, but kept as reference
    /// for the pattern and potential future use with different architecture.
    #[allow(dead_code)]
    fn handle_simple_focusable_window<W>(
        &mut self,
        window: &mut W,
        focused_window_type: FocusedWindow,
        ctx: &egui::Context,
    ) where
        W: FocusableWindow<ShowParams = SimpleShowParams>,
    {
        if window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(focused_window_type) {
                self.set_focused_window(focused_window_type);
            }

            // Check if this window should be brought to the front
            let window_id = window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window using the trait
            window.show_with_focus(ctx, (), bring_to_front);
        }
    }

    /// Handle the help window
    fn handle_help_window(&mut self, ctx: &egui::Context) {
        if self.help_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(FocusedWindow::Help) {
                self.set_focused_window(FocusedWindow::Help);
            }

            // Check if this window should be brought to the front
            let window_id = self.help_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window using the trait
            FocusableWindow::show_with_focus(&mut self.help_window, ctx, (), bring_to_front);
        }
    }

    /// Handle the log window
    fn handle_log_window(&mut self, ctx: &egui::Context) {
        if self.log_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(FocusedWindow::Log) {
                self.set_focused_window(FocusedWindow::Log);
            }

            // Check if this window should be brought to the front
            let window_id = self.log_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window using the trait
            FocusableWindow::show_with_focus(&mut self.log_window, ctx, (), bring_to_front);
        }
    }

    /// Handle the resource JSON editor window
// Resource/template editor windows removed
    /*
    fn handle_resource_json_editor_window(&mut self, ctx: &egui::Context) {
        // Resource/template editor windows removed
        // if self.resource_json_editor_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            // Resource/template editor windows removed
            // if self.currently_focused_window != Some(FocusedWindow::ResourceJsonEditor) {
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::ResourceJsonEditor);
            }

            // Check if this window should be brought to the front
            // Resource/template editor windows removed
            // let window_id = self.resource_json_editor_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window using the trait
            let theme_params = ThemeShowParams {
                theme: self.theme.to_string(),
            };
            FocusableWindow::show_with_focus(
                &mut self.resource_json_editor_window,
                ctx,
                theme_params,
                bring_to_front,
            );
        }

        // Check if a save was requested
        // Resource/template editor windows removed
        // if self.resource_json_editor_window.save_requested {
            // Resource/template editor windows removed
            // self.resource_json_editor_window.save_requested = false;

            // Get the saved resource
            // Resource/template editor windows removed
            // if let Some(saved_resource) = self.resource_json_editor_window.saved_resource.take() {
                // Resource/template editor windows removed
                // let resource_id = self.resource_json_editor_window.resource_id.clone();

                // Update the project
                if let Some(project) = &mut self.project_command_palette.current_project {
                    if let Some(cfn_template) = &mut project.cfn_template {
                        // Update the resource in the template
                        cfn_template
                            .resources
                            .insert(resource_id.clone(), saved_resource);

                        // Update the template sections window
                        self.template_sections_window
                            .set_template(cfn_template.clone());

                        // Save the project
                        if let Err(e) = project.save_all_resources() {
                            tracing::error!(
                                "Failed to save CloudFormation template after JSON edit: {}",
                                e
                            );
                        } else {
                            tracing::info!(
                                "CloudFormation template saved successfully after JSON edit"
                            );
                        }
                    }
                }
            }
        }
    }
    */

    /// Handle the resource form window
// Resource/template editor windows removed
    /*
    fn handle_resource_form_window(&mut self, ctx: &egui::Context) {
        // Resource/template editor windows removed
        // if self.resource_form_window.is_open() {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            // Resource/template editor windows removed
            // if self.currently_focused_window != Some(FocusedWindow::ResourceForm) {
                // Resource/template editor windows removed
                // self.set_focused_window(FocusedWindow::ResourceForm);
            }

            // Check if this window should be brought to the front
            // Resource/template editor windows removed
            // let window_id = self.resource_form_window.window_id();
            let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id);
            if bring_to_front {
                self.window_focus_manager.clear_bring_to_front(window_id);
            }

            // Show the window and check if a resource was saved
            let resource_saved = self
                .resource_form_window
                .show_with_focus_logic(ctx, bring_to_front);
            if resource_saved {
                // A resource was saved, get the resource data
                if let Some(project) = &mut self.project_command_palette.current_project {
                    // Create the CloudFormation resource
                    let mut resource = CloudFormationResource::new(
                        // Resource/template editor windows removed
                        // self.resource_form_window.resource_id.clone(),
                        // Resource/template editor windows removed
                        // self.resource_form_window.resource_type.clone(),
                    );
                    // Resource/template editor windows removed
                    // resource.properties = self.resource_form_window.properties.clone();

                    // Add the resource to the project
                    match project.add_resource(resource.clone(), Vec::new()) {
                        Ok(_) => {
                            info!("Added resource {} to project", resource.resource_id);

                            // Save all resources using the modern approach
                            if let Err(e) = project.save_all_resources() {
                                error!("Failed to save CloudFormation resources: {}", e);
                            } else {
                                info!("CloudFormation resources saved successfully");
                            }

                            // After saving, go back to the template sections window
                            // Resource/template editor windows removed
                            // self.template_sections_window.show = true;
                            // Resource/template editor windows removed
                            // self.template_sections_window.selected_section =
                                // Resource/template editor windows removed
                                // super::template_sections_window::TemplateSection::Resources;
                            // Resource/template editor windows removed
                            // self.set_focused_window(FocusedWindow::TemplateSections);
                        }
                        Err(e) => {
                            error!("Failed to add resource to project: {}", e);
                        }
                    }
                }
            }
        }
    }
    */

    // CloudFormation scene graph removed

    /// Handle the validation results window
    fn handle_validation_results_window(&mut self, _ctx: &egui::Context) {
        // CloudFormation manager removed
        // if self.validation_results_window.open {
        //     self.validation_results_window.show(ctx);
        // }
    }

    /// Handle the guard violations window (removed in Phase 2.1)
    fn handle_guard_violations_window(&mut self, _ctx: &egui::Context) {
        // Guard violations window removed
    }

    fn handle_compliance_error_window(&mut self, _ctx: &egui::Context) {
        // Compliance error window removed in Phase 2.1
    }

    #[allow(dead_code)]
    fn retry_compliance_discovery(&mut self) {
        // Compliance discovery removed in Phase 2.1
    }

    /// Start repository synchronization in background (removed in Phase 2.2)
    fn start_repository_sync(&mut self) {
        // Guard repository system removed
    }

    /// Update repository sync status from background thread (removed in Phase 2.2)
    fn update_repository_sync_status(&mut self, _ctx: &egui::Context) {
        // Guard repository system removed
    }

    fn handle_notification_details_window(&mut self, ctx: &egui::Context) {
        use crate::app::notifications::{
            error_window::NotificationDetailsWindow, NotificationType,
        };

        // Check if a deployment status notification was clicked
        if let Some(selected_id) = &self.notification_manager.selected_notification_id.clone() {
            if let Some(notification) = self.notification_manager.get_notification(selected_id) {
                if matches!(
                    notification.notification_type,
                    NotificationType::DeploymentStatus
                ) {
                    // Open the deployment info window instead of the generic details window
                    // Resource/template editor windows removed
                    // self.deployment_info_window.open = true;
                    self.notification_manager.show_details_window = false;
                    self.notification_manager.selected_notification_id = None;
                    return;
                }
            }
        }

        // Show regular notification details window for other notifications
        NotificationDetailsWindow::show(&mut self.notification_manager, ctx);
    }

    fn handle_parameter_dialog(&mut self, _ctx: &egui::Context) {
        // CloudFormation manager removed
        // if self.parameter_dialog.is_open {
        //     let parameters_confirmed = self.parameter_dialog.show(ctx);
        //     if parameters_confirmed {
        //         self.initiate_deployment();
        //     }
        // }
    }

    fn handle_deployment_progress_window(&mut self, _ctx: &egui::Context) {
        // CloudFormation manager removed
        // if self.deployment_progress_window.is_open {
        //     self.deployment_progress_window.show(ctx);
        // }
    }

    /// Check for new validation results and display them in UI
    fn handle_validation_results(&mut self) {
        // CloudFormation manager removed
        /*
        if let Some(cloudformation_manager) = &self.cloudformation_manager {
            let manager = cloudformation_manager.clone();

            // Extract validation result if available
            let validation_result = {
                let validation_lock = manager.get_validation_result_lock();
                if let Ok(result_guard) = validation_lock.try_read() {
                    if let Some(result) = result_guard.clone() {
                        // Drop read lock and clear the result
                        drop(result_guard);
                        if let Ok(mut write_guard) = validation_lock.try_write() {
                            *write_guard = None;
                        }
                        Some(result)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            // Process the validation result if we have one
            if let Some(validation_result) = validation_result {
                tracing::info!("=== PROCESSING VALIDATION RESULT ===");
                if validation_result.is_valid {
                    tracing::info!("Validation successful - showing results window");

                    // CloudFormation command palette removed

                    // Create success notification
                    let notification = crate::app::notifications::Notification::new_success(
                        format!("validation_success_{}", chrono::Utc::now().timestamp()),
                        "CloudFormation Validation Successful".to_string(),
                        format!(
                            "Template validation passed with {} parameters",
                            validation_result.parameters.len()
                        ),
                        "CloudFormation Validation".to_string(),
                    );
                    self.notification_manager.add_notification(notification);

                    // Show results window
                    self.validation_results_window
                        .show_result(validation_result);
                } else {
                    tracing::warn!("Validation failed - creating error notification");

                    // CloudFormation command palette removed - error_summary no longer needed

                    // Create error notification from validation errors
                    let notification_errors = validation_result
                        .errors
                        .iter()
                        .map(|e| crate::app::notifications::NotificationError {
                            message: e.message.clone(),
                            code: e.code.clone(),
                            details: None,
                        })
                        .collect();

                    let notification = crate::app::notifications::Notification::new_error(
                        format!("validation_error_{}", chrono::Utc::now().timestamp()),
                        "CloudFormation Validation Failed".to_string(),
                        notification_errors,
                        "CloudFormation Validation".to_string(),
                    );

                    self.notification_manager.add_notification(notification);

                    // Also show in validation results window for detailed view
                    self.validation_results_window
                        .show_result(validation_result);
                }
            }
        }
        */
    }

    /// Reset font scaling after scene graph is closed
    #[allow(dead_code)]
    fn reset_font_scaling_for_scene_graph(&self, ctx: &egui::Context) {
        // Reset to default font scaling
        let base_font_size = 14.0;
        let mut style = (*ctx.style()).clone();

        // Reset all text styles to default sizes
        for (text_style, font_id) in style.text_styles.iter_mut() {
            font_id.size = match text_style {
                egui::TextStyle::Heading => base_font_size * 1.2,
                egui::TextStyle::Body => base_font_size,
                egui::TextStyle::Button => base_font_size,
                egui::TextStyle::Small => base_font_size * 0.9,
                egui::TextStyle::Monospace => base_font_size,
                _ => base_font_size,
            };
        }

        ctx.set_style(style);
        tracing::debug!("Reset font scaling after scene graph closed");
    }

    /// Render the main content area
    fn render_main_content_area(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        // Show welcome message
        egui::Frame::default()
            .fill(ui.style().visuals.window_fill)
            .inner_margin(egui::vec2(10.0, 10.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("AWS Resource Explorer");
                    ui.add_space(10.0);
                    ui.label("Press Space to open the command palette");
                    ui.add_space(5.0);
                    ui.label("Use the Bridge window to explore AWS resources with AI assistance");
                });
            });
    }

    // Open resource details window
    #[allow(dead_code)]
    fn open_resource_details(&mut self, _type_name: &str) {
        // Resource/template editor windows removed
    }

    // Open property type details in a new window
    #[allow(dead_code)]
    fn open_property_type(&mut self, _property_type: &str) {
        // Resource/template editor windows removed
    }

    // Show command palette
    fn show_startup_popup(&mut self, ctx: &egui::Context) {
        // Check if we should stop showing the popup using timer
        if let Some(start_time) = self.startup_popup_timer {
            if start_time.elapsed() > Duration::from_secs(3) {
                self.show_startup_popup = false;
                self.startup_popup_timer = None;
                return;
            }
        } else {
            return; // Timer is None, so we don't show the popup
        }

        if !self.show_startup_popup {
            return;
        }

        // Center the popup in the screen
        let screen_rect = ctx.screen_rect();

        // Show tip about command palette
        let (title, content) = ("Tip", "Press the Space Bar\nto open the Command Window".to_string());

        egui::Window::new(title)
            .fixed_pos(egui::pos2(
                screen_rect.center().x - 150.0,
                screen_rect.center().y - 40.0,
            ))
            .fixed_size(egui::vec2(300.0, 80.0))
            .collapsible(false)
            .resizable(false)
            .title_bar(true)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    // Show content (could be multi-line)
                    for line in content.lines() {
                        ui.label(line);
                    }
                    ui.add_space(10.0);
                });
            });

        // Ensure continuous repainting while popup is shown
        ctx.request_repaint();
    }

    fn ui_command_palette(&mut self, ctx: &egui::Context) {
        // Use the command_palette component instead of reimplementing the palette here
        if self.show_command_palette {
            self.command_palette.show = true;

            // Only set focus if command palette is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(FocusedWindow::CommandPalette) {
                self.set_focused_window(FocusedWindow::CommandPalette);
            }

            // Now we use the command palette's action return value
            if let Some(action) = self.command_palette.show(ctx) {
                // When an action is returned, the command palette closes itself
                self.show_command_palette = false;
                match action {
                    CommandAction::Login => {
                        self.aws_login_window.open = true;
                        self.aws_login_window.reset_position();
                    }
                    CommandAction::AWSExplorer => {
                        self.resource_explorer.set_open(true);
                    }
                    CommandAction::ControlBridge => {
                        self.control_bridge_window.open = true;
                        self.set_focused_window(FocusedWindow::ControlBridge);
                    }
                    CommandAction::Quit => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }

            // If command palette was closed, update our state
            if !self.command_palette.show {
                self.show_command_palette = false;
                // We don't need to clear focus here because we'll focus the next window
            }
        } else {
            self.command_palette.show = false;
        }
    }

    // No longer needed as we're using the command_palette component

    /// Configure the font sizes and rendering for the application with enhanced emoji support
    fn configure_fonts(&mut self, ctx: &egui::Context) {
        // Configure enhanced fonts only once for performance
        if !self.fonts_configured {
            info!("🎨 Initializing enhanced fonts with emoji support");
            fonts::configure_enhanced_fonts(ctx);
            self.fonts_configured = true;
        }

        // Continue with basic font size configuration (disabled to prevent Scene zoom conflicts)
        // The Scene container should handle its own font scaling independently
        let base_font_size = 14.0; // Use consistent base size
        self.configure_font_definitions(ctx, base_font_size);
    }

    /// Configure font definitions for optimal text rendering at all zoom levels
    fn configure_font_definitions(&self, ctx: &egui::Context, _base_font_size: f32) {
        // Get current font definitions
        ctx.fonts(|_fonts| {
            // Configure font rasterization settings for crisp text
            // Note: egui uses ab_glyph for font rasterization
            // The font atlas is automatically managed, but we can influence quality
            // by ensuring consistent sizing and avoiding sub-pixel positioning issues
        });
    }

    /// Render the hint overlay when hint mode is active
    fn render_hint_overlay(&mut self, ctx: &egui::Context) {
        if self.hint_mode.is_active() {
            // Render the hint overlay on top of everything using Area for proper overlay behavior
            // Note: No logging here to avoid flooding - logging happens in hint_mode.start() and hint_overlay.render()
            egui::Area::new(egui::Id::new("vimium_hints_overlay"))
                .movable(false)
                .order(egui::Order::Foreground) // Ensures it's on top of all other UI
                .show(ctx, |ui| {
                    // Make the area cover the entire screen for proper input handling
                    ui.allocate_exact_size(ctx.screen_rect().size(), egui::Sense::hover());

                    // Render the hint overlay
                    self.hint_overlay.render(ui, &mut self.hint_mode);
                });
        }
    }

    /// Poll CloudFormation stack status and events for active deployments
    /// This is a simplified implementation that simulates progress for demonstration
    // Project management removed
    fn poll_deployment_status(&mut self) {
        // Project management removed
        /*
        use chrono::Utc;

        // Only poll if we have a current project
        if let Some(project) = &mut self.project_command_palette.current_project {
            let mut project_updated = false;

            // Check each environment for deployments that need polling
            for environment in &mut project.environments {
                if let Some(deployment_status) = &mut environment.deployment_status {
                    if deployment_status.should_poll_events() {
                        // Simulate deployment progress
                        let elapsed_seconds = Utc::now()
                            .signed_duration_since(deployment_status.initiated_at)
                            .num_seconds();

                        // Simulate events being added over time
                        if elapsed_seconds > 10 && deployment_status.stack_events.is_empty() {
                            // Add initial stack event
                            let initial_event = crate::app::projects::StackEvent {
                                timestamp: deployment_status.initiated_at
                                    + chrono::Duration::seconds(5),
                                logical_resource_id: deployment_status.stack_name.clone(),
                                physical_resource_id: None,
                                resource_type: "AWS::CloudFormation::Stack".to_string(),
                                resource_status: "CREATE_IN_PROGRESS".to_string(),
                                resource_status_reason: Some("User Initiated".to_string()),
                                event_id: uuid::Uuid::new_v4().to_string(),
                            };
                            deployment_status.add_events(vec![initial_event]);
                            project_updated = true;

                            // Update notification
                            self.notification_manager.update_deployment_status(
                                &environment.name,
                                deployment_status.stack_name.clone(),
                                deployment_status.deployment_id.clone(),
                                "Resources are being created...".to_string(),
                                true, // Still polling
                            );
                        }

                        // Add more events over time
                        if elapsed_seconds > 30 && deployment_status.stack_events.len() < 3 {
                            let resource_event = crate::app::projects::StackEvent {
                                timestamp: Utc::now(),
                                logical_resource_id: "myDynamoDBTable".to_string(),
                                physical_resource_id: Some("dynamodb-table-12345".to_string()),
                                resource_type: "AWS::DynamoDB::Table".to_string(),
                                resource_status: "CREATE_IN_PROGRESS".to_string(),
                                resource_status_reason: Some(
                                    "Resource creation Initiated".to_string(),
                                ),
                                event_id: uuid::Uuid::new_v4().to_string(),
                            };
                            deployment_status.add_events(vec![resource_event]);
                            project_updated = true;
                        }

                        // Simulate completion after 60 seconds
                        if elapsed_seconds > 60
                            && deployment_status.status
                                == crate::app::projects::DeploymentState::InProgress
                        {
                            // Add completion events
                            let completion_events = vec![
                                crate::app::projects::StackEvent {
                                    timestamp: Utc::now() - chrono::Duration::seconds(5),
                                    logical_resource_id: "myDynamoDBTable".to_string(),
                                    physical_resource_id: Some("dynamodb-table-12345".to_string()),
                                    resource_type: "AWS::DynamoDB::Table".to_string(),
                                    resource_status: "CREATE_COMPLETE".to_string(),
                                    resource_status_reason: None,
                                    event_id: uuid::Uuid::new_v4().to_string(),
                                },
                                crate::app::projects::StackEvent {
                                    timestamp: Utc::now(),
                                    logical_resource_id: deployment_status.stack_name.clone(),
                                    physical_resource_id: Some(format!(
                                        "arn:aws:cloudformation:{}:{}:stack/{}/",
                                        deployment_status.region,
                                        deployment_status.account_id,
                                        deployment_status.stack_name
                                    )),
                                    resource_type: "AWS::CloudFormation::Stack".to_string(),
                                    resource_status: "CREATE_COMPLETE".to_string(),
                                    resource_status_reason: None,
                                    event_id: uuid::Uuid::new_v4().to_string(),
                                },
                            ];

                            deployment_status.add_events(completion_events);
                            deployment_status.set_stack_status("CREATE_COMPLETE".to_string());
                            project_updated = true;

                            // Update notification to show completion
                            let local_time =
                                deployment_status.last_updated.with_timezone(&chrono::Local);
                            self.notification_manager.update_deployment_status(
                                &environment.name,
                                deployment_status.stack_name.clone(),
                                deployment_status.deployment_id.clone(),
                                format!(
                                    "Deployed successfully at {}",
                                    local_time.format("%H:%M:%S")
                                ),
                                false, // Not polling anymore
                            );
                        }
                    }
                }
            }

            // Save project if any deployment status was updated
            if project_updated {
                if let Err(e) = project.save_to_file() {
                    tracing::warn!(
                        "Failed to save project with updated deployment status: {}",
                        e
                    );
                }
            }
        }
        */
    }

    /// Initialize deployment status notifications when a project is loaded
    // Project management removed
    fn initialize_deployment_status_notifications(&mut self) {
        // Project management removed
        /*
        if let Some(project) = &self.project_command_palette.current_project {
            for environment in &project.environments {
                if let Some(deployment_status) = &environment.deployment_status {
                    let message = match deployment_status.status {
                        crate::app::projects::DeploymentState::InProgress => {
                            "Deployment in progress".to_string()
                        }
                        crate::app::projects::DeploymentState::Completed => {
                            let local_time =
                                deployment_status.last_updated.with_timezone(&chrono::Local);
                            format!("Last deployed: {}", local_time.format("%Y-%m-%d %H:%M"))
                        }
                        crate::app::projects::DeploymentState::Failed => {
                            format!(
                                "Deployment failed: {}",
                                deployment_status
                                    .error_message
                                    .as_deref()
                                    .unwrap_or("Unknown error")
                            )
                        }
                        crate::app::projects::DeploymentState::Cancelled => {
                            "Deployment was cancelled".to_string()
                        }
                    };

                    let is_polling = matches!(
                        deployment_status.status,
                        crate::app::projects::DeploymentState::InProgress
                    );

                    self.notification_manager.update_deployment_status(
                        &environment.name,
                        deployment_status.stack_name.clone(),
                        deployment_status.deployment_id.clone(),
                        message,
                        is_polling,
                    );
                }
            }
        }
        */
    }

    /// Handle validation task monitoring for async compliance validation operations (removed in Phase 2.1)
    fn handle_validation_task_monitoring(&mut self) {
        // Compliance validation removed
    }

    /// Handle deployment task monitoring for async deployment operations
    fn handle_deployment_task_monitoring(&mut self) {
        if let Some(task) = &self.pending_deployment_task {
            if task.is_finished() {
                let completed_task = self.pending_deployment_task.take().unwrap();

                // Process the deployment result - std::thread::JoinHandle::join() blocks until completion
                match completed_task.join() {
                    Ok(Ok((deployment_id, stack_name, environment))) => {
                        tracing::info!("Deployment task completed successfully: {}", deployment_id);

                        // Update deployment status notification and project data
                        self.notification_manager.update_deployment_status(
                            &environment,
                            stack_name.clone(),
                            deployment_id.clone(),
                            "Deployment successfully initiated, monitoring progress...".to_string(),
                            true, // Still polling
                        );

                        // Update project deployment status with real deployment ID
                        // Project management removed
                        // if let Some(project) = &mut self.project_command_palette.current_project {
                        //     if let Some(env) = project
                        //         .environments
                        //         .iter_mut()
                        //         .find(|e| e.name == environment)
                        //     {
                        //         if let Some(status) = &mut env.deployment_status {
                        //             status.deployment_id = deployment_id.clone();
                        //             status.set_stack_status("CREATE_IN_PROGRESS".to_string());
                        //
                        //             // Save project to persist updated deployment status
                        //             if let Err(e) = project.save_to_file() {
                        //                 tracing::warn!("Failed to save project with updated deployment status: {}", e);
                        //             }
                        //         }
                        //     }
                        // }

                        // TODO: Start polling CloudFormation stack status here
                        // For now, simulate eventual completion after some time
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Deployment task failed: {}", e);

                        // Update deployment status to failed
                        // CloudFormation command palette removed - using empty environment
                        let environment_name = "";
                        if !environment_name.is_empty() {
                            self.notification_manager.update_deployment_status(
                                environment_name,
                                "Unknown".to_string(), // We don't have stack name in error case
                                "failed".to_string(),
                                format!("Deployment failed: {}", e),
                                false, // Not polling anymore
                            );

                            // Update project deployment status
                            // Project management removed
                            // if let Some(project) = &mut self.project_command_palette.current_project
                            // {
                            //     if let Some(env) = project
                            //         .environments
                            //         .iter_mut()
                            //         .find(|e| e.name == *environment_name)
                            //     {
                            //         if let Some(status) = &mut env.deployment_status {
                            //             status.set_error(format!(
                            //                 "Failed to initiate deployment: {}",
                            //                 e
                            //             ));
                            //
                            //                     // Save project to persist failed deployment status
                            //                     if let Err(save_err) = project.save_to_file() {
                            //                         tracing::warn!("Failed to save project with failed deployment status: {}", save_err);
                            //                     }
                            //                 }
                            //             }
                            // }
                        }

                        self.notification_manager.add_notification(
                            crate::app::notifications::Notification::new_error(
                                uuid::Uuid::new_v4().to_string(),
                                "Deployment Failed".to_string(),
                                vec![crate::app::notifications::NotificationError {
                                    message: format!("Failed to initiate deployment: {}", e),
                                    code: None,
                                    details: None,
                                }],
                                "CloudFormation Deployment".to_string(),
                            ),
                        );
                    }
                    Err(join_error) => {
                        tracing::error!("Deployment task failed to join: {:#?}", join_error);

                        // std::thread::JoinError doesn't have is_cancelled/is_panic methods
                        // It only indicates that the thread panicked
                        tracing::error!(
                            "🚨 THREAD PANIC: The deployment thread panicked during execution"
                        );
                        tracing::error!("This often indicates:");
                        tracing::error!("  • Panic in the deployment code");
                        tracing::error!("  • Runtime issues during AWS API calls");
                        tracing::error!("  • Memory or resource exhaustion");
                        tracing::error!("  • Unhandled error in async context");

                        let error_message = "Deployment thread panicked during execution";

                        self.notification_manager.add_notification(
                            crate::app::notifications::Notification::new_error(
                                uuid::Uuid::new_v4().to_string(),
                                "Deployment Task Error".to_string(),
                                vec![crate::app::notifications::NotificationError {
                                    message: error_message.to_string(),
                                    code: Some("TASK_JOIN_ERROR".to_string()),
                                    details: Some(format!("Join Error Debug: {:#?}", join_error)),
                                }],
                                "CloudFormation Deployment".to_string(),
                            ),
                        );
                    }
                }
            }
        }
    }

    /// Ensure CloudFormation manager is initialized if AWS credentials are available
    #[allow(dead_code)]
    fn ensure_cloudformation_manager_initialized(&mut self) {
        // CloudFormation manager removed
        /*
        // Skip if already initialized
        if self.cloudformation_manager.is_some() {
            return;
        }

        // Try to initialize from existing AWS identity
        if let Some(aws_identity) = &self.aws_identity_center {
            if let Ok(identity) = aws_identity.lock() {
                // Check if we're actually logged in
                if matches!(
                    identity.login_state,
                    crate::app::aws_identity::LoginState::LoggedIn
                ) {
                    tracing::info!(
                        "Initializing CloudFormation manager with existing AWS credentials"
                    );

                    let credential_coordinator = std::sync::Arc::new(
                        crate::app::resource_explorer::credentials::CredentialCoordinator::new(
                            aws_identity.clone(),
                            identity.default_role_name.clone(),
                        ),
                    );

                    // CloudFormation manager removed
                    // let mut manager =
                    //     crate::app::cloudformation_manager::CloudFormationManager::new(
                    //         credential_coordinator,
                    //     );
                    //
                    // // Set AWS client from ResourceExplorer if available
                    // if let Some(aws_client) = self.resource_explorer.get_aws_client() {
                    //     manager.set_aws_client(Some(aws_client));
                    // }

                    self.cloudformation_manager = Some(std::sync::Arc::new(manager));
                    tracing::info!("CloudFormation manager initialized successfully");
                } else {
                    tracing::warn!("AWS Identity Center is available but user is not logged in");
                }
            } else {
                tracing::warn!(
                    "Failed to lock AWS Identity Center for CloudFormation manager initialization"
                );
            }
        } else {
            tracing::warn!(
                "No AWS Identity Center available for CloudFormation manager initialization"
            );
        }
        */
    }

    /// Initiate deployment when parameters are confirmed from parameter dialog
    #[allow(dead_code)]
    fn initiate_deployment(&mut self) {
        // CloudFormation manager removed
        /*
        tracing::info!("Initiating deployment with confirmed parameters");

        if let Some(_project) = &self.project_command_palette.current_project {
            let parameter_values = self.parameter_dialog.get_parameter_values();
            // CloudFormation command palette removed - deployment data unavailable
            let stack_name = String::new();
            let account_id = String::new();
            let region = String::new();

            self.initiate_deployment_with_parameters(
                stack_name,
                account_id,
                region,
                parameter_values,
            );
        }
        */
    }

    /// Initiate deployment with specific parameters
    #[allow(dead_code)]
    fn initiate_deployment_with_parameters(
        &mut self,
        _stack_name: String,
        _account_id: String,
        _region: String,
        _parameters: std::collections::HashMap<String, String>,
    ) {
        // CloudFormation manager removed
        /*
        tracing::info!(
            "Initiating deployment with parameters for stack: {}",
            stack_name
        );

        if let (Some(cloudformation_manager), Some(project)) = (
            &mut self.cloudformation_manager,
            &self.project_command_palette.current_project,
        ) {
            if let Some(template) = &project.cfn_template {
                // Ensure CloudFormation Manager has fresh AWS client before deployment
                if let Some(fresh_aws_client) = self.resource_explorer.get_aws_client() {
                    tracing::info!(
                        "Updating CloudFormation Manager with fresh AWS client for deployment"
                    );
                    std::sync::Arc::get_mut(cloudformation_manager)
                        .unwrap()
                        .set_aws_client(Some(fresh_aws_client));
                } else {
                    tracing::warn!("No AWS client available from Resource Explorer for CloudFormation deployment");
                }

                let manager = cloudformation_manager.clone();
                let stack_name_clone = stack_name.clone(); // Clone for the notification
                                                           // Serialize the template to string for deployment
                let template_string = match serde_json::to_string_pretty(template) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to serialize template: {}", e);
                        self.notification_manager.add_notification(
                            crate::app::notifications::Notification::new_error(
                                uuid::Uuid::new_v4().to_string(),
                                "Template Serialization Error".to_string(),
                                vec![crate::app::notifications::NotificationError {
                                    message: format!("Failed to serialize template: {}", e),
                                    code: None,
                                    details: None,
                                }],
                                "CloudFormation Deployment".to_string(),
                            ),
                        );
                        return;
                    }
                };
                let project_clone = project.clone();
                // CloudFormation command palette removed - environment unavailable
                let environment = String::new();

                // Create deployment task using std::thread to avoid tokio runtime conflicts
                let deployment_task = std::thread::spawn(move || {
                    tracing::info!("Starting deployment task for stack: {}", stack_name);

                    // Create a dedicated tokio runtime with extended configuration to prevent early shutdown
                    let runtime = match tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2) // Ensure sufficient worker threads
                        .thread_name("cloudformation-deployment")
                        .thread_stack_size(3 * 1024 * 1024) // 3MB stack size
                        .enable_all() // Enable all tokio features (I/O, time, etc.)
                        .build()
                    {
                        Ok(rt) => {
                            tracing::info!(
                                "✓ Created dedicated Tokio runtime for CloudFormation deployment"
                            );
                            rt
                        }
                        Err(e) => {
                            tracing::error!("Failed to create tokio runtime: {}", e);
                            return Err(anyhow::anyhow!("Failed to create tokio runtime: {}", e));
                        }
                    };

                    // Execute the async deployment operation with comprehensive lifecycle logging
                    tracing::info!(
                        "🚀 Starting CloudFormation deployment execution in dedicated runtime"
                    );
                    let result = runtime.block_on(async move {
                        tracing::info!(
                            "📡 Inside async deployment context for stack: {}",
                            stack_name
                        );
                        match manager
                            .deploy_stack(
                                template_string,
                                stack_name.clone(),
                                &project_clone,
                                environment.clone(),
                                if parameters.is_empty() {
                                    None
                                } else {
                                    Some(parameters)
                                },
                            )
                            .await
                        {
                            Ok(deployment_id) => {
                                tracing::info!(
                                    "✅ Deployment initiated successfully with ID: {}",
                                    deployment_id
                                );
                                Ok((deployment_id, stack_name, environment))
                            }
                            Err(e) => {
                                tracing::error!("❌ Failed to initiate deployment: {}", e);
                                Err(e)
                            }
                        }
                    });

                    tracing::info!(
                        "🏁 CloudFormation deployment execution completed, shutting down runtime"
                    );
                    // Explicitly shutdown the runtime to ensure clean termination
                    runtime.shutdown_background();
                    tracing::info!("✅ Runtime shutdown completed");

                    result
                });

                // Store the deployment task for monitoring
                self.pending_deployment_task = Some(deployment_task);

                // Create deployment status notification
                // CloudFormation command palette removed
                if false {
                    let environment_name = "";
                    // Generate a temporary deployment ID for UI tracking
                    let ui_deployment_id = uuid::Uuid::new_v4().to_string();

                    self.notification_manager.update_deployment_status(
                        environment_name,
                        stack_name_clone.clone(),
                        ui_deployment_id.clone(),
                        "Deployment in progress...".to_string(),
                        true, // is_polling
                    );

                    // Update project deployment status
                    if let Some(project) = &mut self.project_command_palette.current_project {
                        if let Some(env) = project
                            .environments
                            .iter_mut()
                            .find(|e| e.name == *environment_name)
                        {
                            let deployment_status = crate::app::projects::DeploymentStatus::new(
                                stack_name_clone.clone(),
                                _account_id.clone(),
                                _region.clone(),
                                ui_deployment_id,
                            );
                            env.deployment_status = Some(deployment_status);

                            // Save project to persist deployment status
                            if let Err(e) = project.save_to_file() {
                                tracing::warn!(
                                    "Failed to save project with deployment status: {}",
                                    e
                                );
                            }
                        }
                    }
                }
            } else {
                tracing::error!("No CloudFormation template in project");
                self.notification_manager.add_notification(
                    crate::app::notifications::Notification::new_error(
                        uuid::Uuid::new_v4().to_string(),
                        "No Template".to_string(),
                        vec![crate::app::notifications::NotificationError {
                            message: "Project does not have a CloudFormation template loaded"
                                .to_string(),
                            code: None,
                            details: None,
                        }],
                        "CloudFormation Deployment".to_string(),
                    ),
                );
            }
        } else {
            tracing::error!("CloudFormation manager or project not available");
            self.notification_manager.add_notification(
                crate::app::notifications::Notification::new_error(
                    uuid::Uuid::new_v4().to_string(),
                    "Deployment Error".to_string(),
                    vec![crate::app::notifications::NotificationError {
                        message: "CloudFormation manager or project not available".to_string(),
                        code: None,
                        details: None,
                    }],
                    "CloudFormation Deployment".to_string(),
                ),
            );
        }
        */
    }
}

impl eframe::App for DashApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Configure fonts with enhanced emoji support
        self.configure_fonts(ctx);

        // Update repository sync status from background thread
        self.update_repository_sync_status(ctx);

        // Update shake animation state
        if self.shake_windows {
            if let Some(start_time) = self.shake_start_time {
                if start_time.elapsed() >= self.shake_duration {
                    // Animation finished
                    self.shake_windows = false;
                    self.shake_start_time = None;
                    self.window_shake_offsets.clear();
                } else {
                    // Update shake offsets for each window
                    self.update_window_shake_offsets();
                    // Request continuous repaint during shake animation
                    ctx.request_repaint();
                }
            }
        }

        // Handle pending delayed shake animation
        if let Some(timer) = self.pending_shake_timer {
            if timer.elapsed() >= Duration::from_millis(500) {
                self.start_shake_animation();
                self.pending_shake_timer = None;
            }
        }

        // Handle UI changes and input
        self.check_ui_dimension_changes(ctx);

        // Start widget collection for this frame
        self.widget_manager.start_frame();

        // Start widget capture for template sections window
        // Resource/template editor windows removed
        // self.template_sections_window.start_widget_capture();

        // Process pending widget actions from previous frame
        self.process_pending_widget_actions();

        self.handle_keyboard_input(ctx);

        // Process pending scroll requests
        self.process_pending_scroll_requests(ctx);

        // Handle downloads
        self.handle_downloads();

        // Check for new validation results
        self.handle_validation_results();

        // Check for compliance validation task updates
        self.handle_validation_task_monitoring();

        // Check for deployment task updates
        self.handle_deployment_task_monitoring();

        // Initialize deployment status notifications when project loads
        // This is checked every frame but only creates notifications when they don't exist
        self.initialize_deployment_status_notifications();

        // Poll CloudFormation stack status and events for active deployments
        self.poll_deployment_status();

        // Render UI components
        self.render_top_menu_bar(ctx);
        self.render_navigation_status_bar(ctx);
        self.render_central_panel(ctx);
        self.render_debug_panel(ctx);

        // Handle different windows - move resource list window before command palette
        // to ensure it's processed before any potential command palette action handling
        self.handle_resource_windows(ctx);
        // Resource/template editor windows removed
        // self.handle_property_windows(ctx);
        // Resource/template editor windows removed
        // self.handle_resource_form_window(ctx);
        // self.handle_resource_json_editor_window(ctx);
        // CloudFormation scene graph removed
        self.handle_command_palettes(ctx);
        self.handle_auth_windows(ctx);
        self.handle_startup_popup(ctx);
        self.handle_help_window(ctx);
        self.handle_log_window(ctx);
        self.handle_chat_window(ctx);
        self.handle_control_bridge_window(ctx);
        self.handle_credentials_debug_window(ctx);
        // Resource/template editor windows removed
        // self.handle_deployment_info_window(ctx);
        // self.handle_template_sections_window(ctx);
        self.handle_verification_window(ctx);
        self.handle_guard_violations_window(ctx);
        self.handle_compliance_error_window(ctx);
        self.handle_validation_results_window(ctx);
        self.handle_parameter_dialog(ctx);
        self.handle_deployment_progress_window(ctx);
        self.handle_notification_details_window(ctx);
        self.handle_resource_explorer_window(ctx);
        self.handle_window_selector(ctx);

        // Render hint overlay on top of everything
        self.render_hint_overlay(ctx);

        // Handle continuous repainting
        self.handle_continuous_repainting(ctx);
    }
}

// Fuzzy search utilities
pub fn fuzzy_match(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }

    let pattern = pattern.to_lowercase();
    let text = text.to_lowercase();

    let mut pattern_idx = 0;
    let pattern_chars: Vec<char> = pattern.chars().collect();

    for c in text.chars() {
        if pattern_idx < pattern_chars.len() && c == pattern_chars[pattern_idx] {
            pattern_idx += 1;
        }
    }

    pattern_idx == pattern_chars.len()
}

// CommandEntry struct removed - now defined in command_palette.rs

pub fn fuzzy_match_score(pattern: &str, text: &str) -> Option<usize> {
    if pattern.is_empty() {
        return Some(0);
    }

    let pattern = pattern.to_lowercase();
    let text = text.to_lowercase();

    let mut score = 0;
    let mut pattern_idx = 0;
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let mut consecutive_matches = 0;

    for c in text.chars() {
        if pattern_idx < pattern_chars.len() && c == pattern_chars[pattern_idx] {
            pattern_idx += 1;
            consecutive_matches += 1;
            // Bonus for consecutive matches
            score += consecutive_matches;
        } else {
            consecutive_matches = 0;
        }
    }

    if pattern_idx == pattern_chars.len() {
        // Bonus for shorter text (more precise match)
        let length_ratio = pattern.len() as f32 / text.len() as f32;
        score = (score as f32 * (1.0 + length_ratio)) as usize;
        Some(score)
    } else {
        None
    }
}
