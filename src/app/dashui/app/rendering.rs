//! UI rendering for top menu, status bar, central panel, and overlays

use super::{DashApp, FocusedWindow};
use crate::app::dashui::command_palette::CommandAction;
use crate::app::dashui::menu;
use crate::app::dashui::NavigationMode;
use eframe::egui;
use std::time::Duration;

impl DashApp {
    /// Render the top menu bar
    pub(super) fn render_top_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // Project management removed
                let project_info = None;

                // Get resource count if a project is loaded
                let resource_count = None;

                // Get compliance programs from current project
                let compliance_programs = None;

                let menu_action = menu::build_menu(
                    ui,
                    ctx,
                    &mut self.theme,
                    &mut self.navigation_status_bar_settings,
                    project_info,
                    &mut self.log_window.open,
                    resource_count,
                    self.aws_identity_center.as_ref(), // Pass AWS identity center for login status
                    self.compliance_status.clone(),
                    compliance_programs,
                );

                // Handle menu actions
                match menu_action {
                    menu::MenuAction::ThemeChanged => {
                        tracing::info!("Theme changed");
                    }
                    menu::MenuAction::NavigationStatusBarChanged => {
                        tracing::info!(
                            "Navigation status bar setting changed to: {}",
                            self.navigation_status_bar_settings.show_status_bar
                        );
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
                    menu::MenuAction::LoginAWS => {
                        self.aws_login_window.open = true;
                        self.aws_login_window.reset_position();
                        tracing::info!("AWS Login window opened from Dash menu");
                    }
                    menu::MenuAction::AWSExplorer => {
                        // Check if logged in to AWS before opening Explorer
                        if self.is_aws_logged_in() {
                            self.resource_explorer.set_open(true);
                            tracing::info!("AWS Explorer opened from Dash menu");
                        } else {
                            self.show_login_required_notification("AWS Explorer");
                            tracing::warn!("AWS Explorer access denied - not logged in");
                        }
                    }
                    menu::MenuAction::AgentManager => {
                        // Check if logged in to AWS before opening Agent Manager
                        if self.is_aws_logged_in() {
                            if let Some(window) = &mut self.agent_manager_window {
                                window.open();
                                self.set_focused_window(FocusedWindow::AgentManager);
                            }
                            tracing::info!("Agent Manager opened from Dash menu");
                        } else {
                            self.show_login_required_notification("Agent Manager");
                            tracing::warn!("Agent Manager access denied - not logged in");
                        }
                    }
                    menu::MenuAction::Quit => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        tracing::info!("Quit requested from Dash menu");
                    }
                    menu::MenuAction::None => {}
                }
            });
        });
    }

    /// Render the navigation status bar showing current mode and key sequence
    pub(super) fn render_navigation_status_bar(&mut self, ctx: &egui::Context) {
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
                                FocusedWindow::Help => "Help",
                                FocusedWindow::Log => "Log",
                                FocusedWindow::Chat => "Chat",
                                _ => "Other",
                            };
                            ui.weak(format!("Focus: {}", window_name));
                        }
                    });
                });
            });
    }

    /// Render the central panel with content
    pub(super) fn render_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render the main content with resource grid
            self.render_main_content_area(ctx, ui);
        });
    }

    /// Render the main content area
    pub(super) fn render_main_content_area(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        // Show welcome message
        egui::Frame::default()
            .fill(ui.style().visuals.window_fill)
            .inner_margin(egui::vec2(10.0, 10.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("Press Space to open the command palette");
                });
            });
    }

    /// Show startup popup with tip
    pub(super) fn show_startup_popup(&mut self, ctx: &egui::Context) {
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
        let (title, content) = (
            "Tip",
            "Press the Space Bar\nto open the Command Window".to_string(),
        );

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

    /// Render the command palette
    pub(super) fn ui_command_palette(&mut self, ctx: &egui::Context) {
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
                        // Check if logged in to AWS before opening Explorer
                        if self.is_aws_logged_in() {
                            self.resource_explorer.set_open(true);
                        } else {
                            self.show_login_required_notification("AWS Explorer");
                            tracing::warn!("AWS Explorer access denied - not logged in");
                        }
                    }
                    CommandAction::AgentManager => {
                        // Check if logged in to AWS before opening Agent Manager
                        if self.is_aws_logged_in() {
                            if let Some(window) = &mut self.agent_manager_window {
                                window.open();
                                self.set_focused_window(FocusedWindow::AgentManager);
                            }
                        } else {
                            self.show_login_required_notification("Agent Manager");
                            tracing::warn!("Agent Manager access denied - not logged in");
                        }
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

    /// Render the hint overlay when hint mode is active
    pub(super) fn render_hint_overlay(&mut self, ctx: &egui::Context) {
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

    /// Render the debug panel with build information
    pub(super) fn render_debug_panel(&mut self, ctx: &egui::Context) {
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
                                git_branch, git_commit
                            ))
                            .small()
                            .color(egui::Color32::from_rgb(255, 165, 0)), // Orange color
                        );
                    }
                });
            });
    }

    /// Handle continuous repainting logic
    pub(super) fn handle_continuous_repainting(&mut self, ctx: &egui::Context) {
        // Request continuous redrawing when any window is open
        if self.show_command_palette
            || self.show_startup_popup
            || self.help_window.open
            || self.log_window.open
            || self
                .agent_manager_window
                .as_ref()
                .is_some_and(|w| w.is_open())
            || self.verification_window.visible
            || self.resource_explorer.is_open()
        {
            ctx.request_repaint();
        }
    }

    /// Check if user is logged in to AWS
    pub(super) fn is_aws_logged_in(&self) -> bool {
        if let Some(aws_identity) = &self.aws_identity_center {
            if let Ok(identity) = aws_identity.lock() {
                return matches!(
                    identity.login_state,
                    crate::app::aws_identity::LoginState::LoggedIn
                );
            }
        }
        false
    }

    /// Show a notification that the user must login to AWS first
    pub(super) fn show_login_required_notification(&mut self, feature_name: &str) {
        let notification = crate::app::notifications::Notification::new_warning(
            format!(
                "login_required_{}",
                feature_name.replace(' ', "_").to_lowercase()
            ),
            format!("Login Required for {}", feature_name),
            vec![crate::app::notifications::NotificationError {
                message: "You must login to AWS first before accessing this feature.".to_string(),
                code: None,
                details: Some(
                    "Use the Dash menu or Command Palette to open the AWS Login window."
                        .to_string(),
                ),
            }],
            feature_name.to_string(),
        );

        self.notification_manager.add_notification(notification);
        tracing::info!("Login required notification shown for {}", feature_name);
    }
}
