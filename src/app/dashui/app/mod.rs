//! Modular implementation of DashApp
//!
//! This module contains the implementation of DashApp split into logical components:
//! - initialization: App creation and font configuration
//! - theme: Theme management and UI dimension tracking
//! - window_management: Window focus, shake animations, and positioning
//! - keyboard_input: Keyboard, navigation, and hint mode input handling
//! - event_handling: Download handling, widget actions, and element activation
//! - rendering: Top menu bar, status bar, and central panel rendering
//! - window_rendering: All individual window rendering methods

use super::aws_login_window::AwsLoginWindow;
use super::cloudwatch_logs_window::CloudWatchLogsWindow;
use super::cloudtrail_events_window::CloudTrailEventsWindow;
use super::command_palette::CommandPalette;
use super::help_window::HelpWindow;
use super::log_window::LogWindow;
use super::verification_window::VerificationWindow;
use super::window_focus::WindowFocusManager;
use super::window_selector::WindowSelector;
use super::{
    HintMode, HintOverlay, KeyMappingRegistry,
    NavigableWidgetManager, NavigationState,
};
use crate::app::aws_identity::AwsIdentityCenter;
use crate::app::notifications::NotificationManager;
use crate::app::resource_explorer::ResourceExplorer;
use eframe::egui;
use std::collections::HashSet;
use std::time::{Duration, Instant};

// Type aliases for complex types
type DeploymentTaskHandle =
    std::thread::JoinHandle<Result<(String, String, String), anyhow::Error>>;

// Module declarations
mod event_handling;
mod initialization;
mod keyboard_input;
mod rendering;
mod theme;
mod window_management;
mod window_rendering;

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
    AwsLogin,
    AwsAccounts,
    StartupPopup,
    Help,
    Log,
    Chat,
    AgentManager,
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
    #[serde(skip)]
    pub aws_login_window: AwsLoginWindow,
    #[serde(skip)]
    pub help_window: HelpWindow,
    #[serde(skip)]
    pub log_window: LogWindow,
    // V1 AgentManager removed - V2 agents managed directly in AgentManagerWindow
    #[serde(skip)]
    pub agent_manager_window: Option<crate::app::dashui::AgentManagerWindow>,
    #[serde(skip)]
    pub verification_window: VerificationWindow,
    #[serde(skip)]
    pub cloudwatch_logs_windows: Vec<CloudWatchLogsWindow>,
    #[serde(skip)]
    pub cloudtrail_events_windows: Vec<CloudTrailEventsWindow>,
    #[serde(skip)]
    pub resource_explorer: ResourceExplorer,
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
            aws_login_window: AwsLoginWindow::default(),
            help_window: HelpWindow::new(),
            log_window: LogWindow::new(),
            agent_manager_window: None,
            verification_window: VerificationWindow::default(),
            cloudwatch_logs_windows: Vec::new(),
            cloudtrail_events_windows: Vec::new(),
            resource_explorer: ResourceExplorer::new(),
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

impl eframe::App for DashApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Frame timing instrumentation
        let frame_start = std::time::Instant::now();

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

        // Poll agent responses BEFORE rendering windows
        // This ensures agents are polled every frame regardless of window visibility
        if let Some(agent_window) = &mut self.agent_manager_window {
            agent_window.poll_agent_responses_global();
        }

        // Render UI components
        self.render_top_menu_bar(ctx);
        self.render_navigation_status_bar(ctx);
        self.render_central_panel(ctx);
        self.render_debug_panel(ctx);

        // Handle different windows
        // Resource/template editor windows removed
        self.handle_command_palettes(ctx);
        self.handle_auth_windows(ctx);
        self.handle_startup_popup(ctx);
        self.handle_help_window(ctx);
        self.handle_log_window(ctx);
        self.handle_chat_window(ctx);
        self.handle_agent_manager_window(ctx);
        self.handle_credentials_debug_window(ctx);
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

        // Frame timing instrumentation
        let frame_duration = frame_start.elapsed();
        if frame_duration.as_millis() > 16 {
            log::warn!(
                "⏱️ SLOW FRAME: {:?} (target: 16ms for 60fps)",
                frame_duration
            );
        }
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

pub fn fuzzy_match_score(pattern: &str, text: &str) -> Option<usize> {
    if pattern.is_empty() {
        return Some(0);
    }

    let pattern = pattern.to_lowercase();
    let text = text.to_lowercase();

    let mut score = 0;
    let mut pattern_idx = 0;
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let mut last_match_idx = None;

    for (idx, c) in text.chars().enumerate() {
        if pattern_idx < pattern_chars.len() && c == pattern_chars[pattern_idx] {
            // Bonus points for consecutive matches
            if let Some(last_idx) = last_match_idx {
                if idx == last_idx + 1 {
                    score += 5;
                }
            }

            // Bonus points for matching at word boundaries
            if idx == 0 || text.chars().nth(idx - 1).unwrap().is_whitespace() {
                score += 10;
            }

            score += 1;
            pattern_idx += 1;
            last_match_idx = Some(idx);
        }
    }

    if pattern_idx == pattern_chars.len() {
        Some(score)
    } else {
        None
    }
}
