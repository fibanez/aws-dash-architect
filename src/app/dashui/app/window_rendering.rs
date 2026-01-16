//! Window rendering methods for all UI windows

use super::{DashApp, FocusedWindow};
use crate::app::agent_framework::v8_bindings::set_global_aws_identity;
use crate::app::agent_framework::utils::registry::set_global_aws_client;
use crate::app::dashui::window_focus::FocusableWindow;
use crate::app::resource_explorer::set_global_bookmark_manager;
use eframe::egui;
use std::sync::Arc;

impl DashApp {
    /// Handle command palettes
    pub(super) fn handle_command_palettes(&mut self, ctx: &egui::Context) {
        // Display command palette if open
        self.ui_command_palette(ctx);

        // Project command palette handling removed
    }

    /// Handle authentication windows
    pub(super) fn handle_auth_windows(&mut self, ctx: &egui::Context) {
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
                // Use try_lock() to avoid blocking UI when login thread holds the mutex
                let was_logged_in_before =
                    if let Some(existing_identity) = &self.aws_identity_center {
                        if let Ok(identity) = existing_identity.try_lock() {
                            matches!(
                                identity.login_state,
                                crate::app::aws_identity::LoginState::LoggedIn
                            )
                        } else {
                            false // Lock held - assume not logged in yet
                        }
                    } else {
                        false
                    };

                // Update the identity center reference
                self.aws_identity_center = Some(aws_identity.clone());

                // Set global AwsIdentity for agent framework tools
                set_global_aws_identity(Some(aws_identity.clone()));

                // Initialize AgentManagerWindow (V2 only - no V1 AgentManager needed)
                if self.agent_manager_window.is_none() {
                    self.agent_manager_window = Some(crate::app::dashui::AgentManagerWindow::new());
                    tracing::info!("🚀 AgentManagerWindow (V2) initialized");
                }

                // Check if credentials are actually available (prevents race condition)
                // Use try_lock() to avoid blocking UI when login thread holds the mutex
                let has_credentials = if let Ok(identity) = aws_identity.try_lock() {
                    identity.default_role_credentials.is_some()
                } else {
                    false // Lock held by login thread - assume not ready yet
                };

                if has_credentials {
                    // Set AWS identity on AgentManagerWindow
                    if let Some(agent_window) = &mut self.agent_manager_window {
                        agent_window.set_aws_identity(aws_identity.clone());
                        tracing::debug!("AgentManagerWindow AWS identity set");
                    }

                    // Proactively initialize ExplorerManager with AWS Identity Center
                    // This ensures the AWS client is available for agent framework tools
                    self.explorer_manager
                        .set_aws_identity_center(Some(aws_identity.clone()));

                    // Create AWS client from identity center
                    if let Ok(identity_center) = aws_identity.lock() {
                        let default_role = identity_center.default_role_name.clone();
                        let credential_coordinator = Arc::new(
                            crate::app::resource_explorer::credentials::CredentialCoordinator::new(
                                aws_identity.clone(),
                                default_role,
                            ),
                        );
                        let aws_client = Arc::new(
                            crate::app::resource_explorer::AWSResourceClient::new(credential_coordinator)
                        );
                        self.explorer_manager.set_aws_client(Some(aws_client.clone()));

                        // Set global AWS client for bridge tools
                        set_global_aws_client(Some(aws_client));

                        tracing::info!("ExplorerManager AWS client created and set");
                    }

                    // Set global bookmark manager for V8 bindings
                    // Note: Global explorer state will be set when first window is opened
                    set_global_bookmark_manager(Some(
                        self.explorer_manager.get_bookmark_manager(),
                    ));
                    tracing::debug!(
                        "ExplorerManager AWS identity and global bookmark manager set for V8 bindings"
                    );
                } else {
                    tracing::debug!("Waiting for credentials before initializing windows");
                }

                // Check if we just completed login
                // Use try_lock() to avoid blocking UI when login thread holds the mutex
                let is_logged_in_now = if let Ok(identity) = aws_identity.try_lock() {
                    matches!(
                        identity.login_state,
                        crate::app::aws_identity::LoginState::LoggedIn
                    )
                } else {
                    false // Lock held by login thread - assume not logged in yet
                };

                // Log when transitioning from not logged in to logged in
                if !was_logged_in_before && is_logged_in_now {
                    tracing::info!("AWS login successful");
                    tracing::info!(
                        "ResourceExplorer and AgentManagerWindow initialized with credentials"
                    );
                    // Note: Shake animation now triggered when credentials debug window opens
                }
            } else if self.aws_identity_center.is_some() && self.aws_login_window.logged_out {
                // If login window returns None and we previously had an identity,
                // and the logged_out flag is set, it means user logged out
                tracing::info!("Clearing AWS Identity Center reference due to logout");
                self.aws_identity_center = None;

                // Clear global AwsIdentity for agent framework tools
                set_global_aws_identity(None);

                // Close all Explorer windows and clear AWS client
                // Note: close_all_windows() also clears global explorer state
                self.explorer_manager.set_aws_identity_center(None);
                self.explorer_manager.set_aws_client(None);
                self.explorer_manager.close_all_windows();

                // Clear global AWS client for bridge tools
                set_global_aws_client(None);

                // Clear global bookmark manager for V8 bindings
                set_global_bookmark_manager(None);
                tracing::info!("ExplorerManager cleared and all windows closed on logout");

                // Reset log groups initialization check for next login
                self.reset_log_groups_init_check();
            }

            // Check if the accounts window is open and set focus
            if self.aws_login_window.accounts_window_open {
                self.set_focused_window(FocusedWindow::AwsAccounts);
            }
        }
    }

    /// Handle startup popup
    pub(super) fn handle_startup_popup(&mut self, ctx: &egui::Context) {
        // Show startup popup if needed
        if self.show_startup_popup {
            // Only set focus if this window is not already focused to avoid stealing focus every frame
            if self.currently_focused_window != Some(FocusedWindow::StartupPopup) {
                self.set_focused_window(FocusedWindow::StartupPopup);
            }
            self.show_startup_popup(ctx);
        }
    }

    /// Handle the chat window - REMOVED (chat window deleted)
    pub(super) fn handle_chat_window(&mut self, _ctx: &egui::Context) {
        // Chat window removed
    }

    /// Handle the credentials debug window - removed
    pub(super) fn handle_credentials_debug_window(&mut self, _ctx: &egui::Context) {
        // Credentials debug window removed
    }

    /// Handle the verification window
    pub(super) fn handle_verification_window(&mut self, ctx: &egui::Context) {
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

/// Open the Pages Manager window in a new webview
    pub fn open_pages_manager_window(&self) {
        use super::ThemeChoice;
        let is_dark_theme = !matches!(self.theme, ThemeChoice::Latte);
        match crate::app::webview::spawn_pages_manager_window(is_dark_theme) {
            Ok(()) => {
                tracing::info!("Pages Manager window spawned successfully");
            }
            Err(e) => {
                tracing::error!("Failed to spawn Pages Manager window: {}", e);
            }
        }
    }

    /// Handle all AWS Explorer window instances
    ///
    /// This method manages multiple Explorer windows, each with independent state but shared cache/bookmarks
    pub(super) fn handle_explorer_windows(&mut self, ctx: &egui::Context) {
        use crate::app::dashui::window_focus::FocusableWindow;

        // Get list of all instance IDs to avoid borrow conflicts
        let instance_ids: Vec<uuid::Uuid> = self
            .explorer_manager
            .instances
            .iter()
            .map(|i| i.id())
            .collect();

        // Track which instances should be closed (after rendering)
        let mut instances_to_close = Vec::new();

        // Render each Explorer instance
        for instance_id in instance_ids {
            if let Some(instance) = self
                .explorer_manager
                .instances
                .iter_mut()
                .find(|i| i.id() == instance_id)
            {
                // Check if this window should be brought to the front
                let window_id_str = instance.window_id();
                let bring_to_front = self.window_focus_manager.should_bring_to_front(window_id_str);

                if bring_to_front {
                    self.window_focus_manager
                        .clear_bring_to_front(window_id_str);
                }

                // Render the instance using FocusableWindow trait
                FocusableWindow::show_with_focus(
                    instance,
                    ctx,
                    self.explorer_manager.shared_context.clone(),
                    bring_to_front,
                );

                // Check if window was closed
                if !instance.is_open {
                    instances_to_close.push(instance_id);
                    tracing::info!("Explorer instance {} marked for closure", instance.instance_number());
                }
            }
        }

        // Close instances that were marked for closure
        for instance_id in instances_to_close {
            self.explorer_manager.close_window(instance_id);
        }

        // Process V8 ExplorerAction queue (agent scripts requesting Explorer windows)
        let v8_actions = crate::app::resource_explorer::drain_explorer_actions();
        for v8_action in v8_actions {
            match v8_action {
                crate::app::resource_explorer::ExplorerAction::OpenWithConfig(config) => {
                    tracing::info!("V8 agent requested new Explorer window with config");
                    let instance = self.explorer_manager.open_new_window();
                    // TODO: Apply config to the new instance's state
                    // For now, just open an empty window
                    tracing::warn!("V8 config application not yet implemented: {:?}", config);
                    let _ = instance; // Suppress unused warning
                }
            }
        }

        // Collect pending actions from all Explorer instances
        let actions = self.explorer_manager.take_pending_actions();
        for action in actions {
            match action {
                crate::app::resource_explorer::ResourceExplorerAction::OpenCloudWatchLogs {
                    log_group_name,
                    resource_name,
                    account_id,
                    region,
                } => {
                    // Create a new CloudWatch Logs window for this resource
                    if let Some(aws_client) = self.explorer_manager.shared_context.get_aws_client() {
                        let credential_coordinator = aws_client.get_credential_coordinator();
                        let mut new_window = crate::app::dashui::CloudWatchLogsWindow::new(credential_coordinator);

                        // Open the window with the resource's log group
                        new_window.open_for_resource(
                            crate::app::dashui::CloudWatchLogsShowParams {
                                log_group_name,
                                resource_name,
                                account_id,
                                region,
                            },
                        );

                        // Add to the list of open windows
                        self.cloudwatch_logs_windows.push(new_window);
                    }
                }
                crate::app::resource_explorer::ResourceExplorerAction::OpenAwsConsole {
                    resource_type,
                    resource_id,
                    resource_name,
                    resource_arn,
                    account_id,
                    region,
                } => {
                    let aws_identity = self.aws_identity_center.clone();
                    std::thread::spawn(move || {
                        let Some(aws_identity) = aws_identity else {
                            tracing::warn!("AWS Console requested but no identity center available");
                            return;
                        };

                        let mut identity_center = match aws_identity.lock() {
                            Ok(identity) => identity,
                            Err(_) => {
                                tracing::warn!("Failed to lock AWS identity center for console launch");
                                return;
                            }
                        };

                        let destination =
                            crate::app::resource_explorer::console_links::build_console_destination(
                                &resource_type,
                                &resource_id,
                                &region,
                                resource_arn.as_deref(),
                            );
                        let role_name = identity_center.default_role_name.clone();
                        let console_url = match identity_center.generate_console_signin_url(
                            &account_id,
                            &role_name,
                            &destination,
                        ) {
                            Ok(url) => url,
                            Err(err) => {
                                tracing::warn!("Failed to generate AWS console URL: {}", err);
                                return;
                            }
                        };

                        let title = format!("AWS Console: {}", resource_name);
                        if let Err(err) =
                            crate::app::webview::spawn_webview_process(console_url, title)
                        {
                            tracing::warn!("Failed to spawn AWS console webview: {}", err);
                        }
                    });
                }
                crate::app::resource_explorer::ResourceExplorerAction::RequestAwsConsoleRoles {
                    request_id,
                    account_id,
                } => {
                    let aws_identity = self.aws_identity_center.clone();
                    let updates = self.explorer_manager.shared_context.console_role_menu_updates();
                    std::thread::spawn(move || {
                        let result = match aws_identity {
                            Some(aws_identity) => match aws_identity.lock() {
                                Ok(identity_center) => {
                                    identity_center.fetch_console_menu_roles(&account_id)
                                }
                                Err(_) => Err("Failed to lock AWS Identity Center".to_string()),
                            },
                            None => Err("AWS Identity Center not available".to_string()),
                        };

                        if let Ok(mut queue) = updates.lock() {
                            queue.push(crate::app::resource_explorer::ConsoleRoleMenuUpdate {
                                request_id,
                                account_id,
                                result,
                            });
                        }
                    });
                }
                crate::app::resource_explorer::ResourceExplorerAction::OpenAwsConsoleWithRole {
                    resource_type,
                    resource_id,
                    resource_name,
                    resource_arn,
                    account_id,
                    region,
                    role_name,
                } => {
                    let aws_identity = self.aws_identity_center.clone();
                    std::thread::spawn(move || {
                        let Some(aws_identity) = aws_identity else {
                            tracing::warn!("AWS Console requested but no identity center available");
                            return;
                        };

                        let identity_center = match aws_identity.lock() {
                            Ok(identity) => identity,
                            Err(_) => {
                                tracing::warn!("Failed to lock AWS identity center for console launch");
                                return;
                            }
                        };

                        let destination =
                            crate::app::resource_explorer::console_links::build_console_destination(
                                &resource_type,
                                &resource_id,
                                &region,
                                resource_arn.as_deref(),
                            );
                        let console_url = match identity_center
                            .generate_console_signin_url_ephemeral(&account_id, &role_name, &destination)
                        {
                            Ok(url) => url,
                            Err(err) => {
                                tracing::warn!("Failed to generate AWS console URL: {}", err);
                                return;
                            }
                        };

                        let title = format!("AWS Console: {}", resource_name);
                        if let Err(err) =
                            crate::app::webview::spawn_webview_process(console_url, title)
                        {
                            tracing::warn!("Failed to spawn AWS console webview: {}", err);
                        }
                    });
                }
                crate::app::resource_explorer::ResourceExplorerAction::OpenCloudTrailEvents {
                    resource_type,
                    resource_name,
                    resource_arn,
                    account_id,
                    region,
                } => {
                    // Create a new CloudTrail Events window for this resource
                    if let Some(aws_client) = self.explorer_manager.shared_context.get_aws_client() {
                        let credential_coordinator = aws_client.get_credential_coordinator();
                        let mut new_window = crate::app::dashui::CloudTrailEventsWindow::new(credential_coordinator);

                        // Open the window with the resource's parameters
                        new_window.open_for_resource(
                            crate::app::dashui::CloudTrailEventsShowParams {
                                resource_type,
                                resource_name,
                                resource_arn,
                                account_id,
                                region,
                            },
                        );

                        // Add to the list of open windows
                        self.cloudtrail_events_windows.push(new_window);
                    }
                }
            }
        }

        // Handle all CloudWatch Logs windows
        for logs_window in &mut self.cloudwatch_logs_windows {
            if logs_window.is_open() {
                logs_window.show(ctx);
            }
        }

        // Remove closed windows from the list
        self.cloudwatch_logs_windows.retain(|w| w.is_open());

        // Handle all CloudTrail Events windows
        for events_window in &mut self.cloudtrail_events_windows {
            if events_window.is_open() {
                events_window.show(ctx);
            }
        }

        // Remove closed windows from the list
        self.cloudtrail_events_windows.retain(|w| w.is_open());
    }

    /// Handle the window selector
    pub(super) fn handle_window_selector(&mut self, _ctx: &egui::Context) {
        // Update window tracking - the menu selection is handled in render_top_panel
        self.update_window_tracking();
    }

    /// Handle the help window
    pub(super) fn handle_help_window(&mut self, ctx: &egui::Context) {
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
    pub(super) fn handle_log_window(&mut self, ctx: &egui::Context) {
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

    /// Handle the agent manager window
    pub(super) fn handle_agent_manager_window(&mut self, ctx: &egui::Context) {
        // Sync agent logging setting to agent manager window
        if let Some(window) = &mut self.agent_manager_window {
            window.set_agent_logging_enabled(self.agent_logging_enabled);
        }

        // Check if window exists and is open before borrowing
        let is_open = self
            .agent_manager_window
            .as_ref()
            .is_some_and(|w| w.is_open());
        if !is_open {
            return;
        }

        // Set focus if needed
        if self.currently_focused_window != Some(FocusedWindow::AgentManager) {
            self.set_focused_window(FocusedWindow::AgentManager);
        }

        // Get window_id and bring_to_front status
        let (window_id, bring_to_front) = if let Some(window) = &self.agent_manager_window {
            let id = window.window_id();
            let bring = self.window_focus_manager.should_bring_to_front(id);
            (id, bring)
        } else {
            return;
        };

        // Clear bring to front flag
        if bring_to_front {
            self.window_focus_manager.clear_bring_to_front(window_id);
        }

        // Show the window using the trait
        if let Some(window) = &mut self.agent_manager_window {
            FocusableWindow::show_with_focus(window, ctx, (), bring_to_front);
        }
    }

    /// Handle validation results window (removed)
    pub(super) fn handle_validation_results_window(&mut self, _ctx: &egui::Context) {
        // CloudFormation manager removed
    }

    /// Guard violations window removed
    pub(super) fn handle_guard_violations_window(&mut self, _ctx: &egui::Context) {
        // Guard violations window removed
    }

    /// Compliance error window removed
    pub(super) fn handle_compliance_error_window(&mut self, _ctx: &egui::Context) {
        // Compliance error window removed
    }

    /// Handle parameter dialog (removed)
    pub(super) fn handle_parameter_dialog(&mut self, _ctx: &egui::Context) {
        // CloudFormation manager removed
    }

    /// Handle deployment progress window (removed)
    pub(super) fn handle_deployment_progress_window(&mut self, _ctx: &egui::Context) {
        // CloudFormation manager removed
    }

    /// Handle notification details window
    pub(super) fn handle_notification_details_window(&mut self, ctx: &egui::Context) {
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
                    self.notification_manager.show_details_window = false;
                    self.notification_manager.selected_notification_id = None;
                    return;
                }
            }
        }

        // Show regular notification details window for other notifications
        NotificationDetailsWindow::show(&mut self.notification_manager, ctx);
    }
}
