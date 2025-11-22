//! Window focus management, shake animations, and positioning

use super::{DashApp, FocusedWindow};
use crate::app::dashui::window_focus::FocusableWindow;
use crate::app::dashui::window_selector::WindowType;
use crate::trace_info;
use eframe::egui;
use std::time::Instant;
use tracing::info;

impl DashApp {
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

    /// Compliance validation feature removed
    pub(super) fn trigger_compliance_validation(&mut self) {
        // Compliance/Guard system removed
    }

    /// Update shake offsets for tracked windows that are currently shaking
    pub(super) fn update_window_shake_offsets(&mut self) {
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
    pub(super) fn log_once(&mut self, key: &str, message: &str) {
        if !self.logged_states.contains(key) {
            trace_info!("{}", message);
            self.logged_states.insert(key.to_string());
        }
    }

    /// Set the currently focused window
    pub(super) fn set_focused_window(&mut self, window: FocusedWindow) {
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
    pub(super) fn get_previous_window(&self) -> Option<FocusedWindow> {
        self.window_focus_order.last().copied()
    }

    /// Close the currently focused window and focus the next available window
    pub(super) fn close_focused_window(&mut self) {
        if let Some(window) = self.currently_focused_window {
            match window {
                FocusedWindow::CommandPalette => {
                    self.show_command_palette = false;
                    self.command_palette.show = false;
                }
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
                FocusedWindow::Help => {
                    self.help_window.open = false;
                }
                FocusedWindow::Log => {
                    self.log_window.open = false;
                }
                FocusedWindow::Chat => {
                    // Chat window removed
                }
                FocusedWindow::AgentManager => {
                    if let Some(window) = &mut self.agent_manager_window {
                        window.close();
                    }
                }
                FocusedWindow::Verification => {
                    self.verification_window.visible = false;
                }
                FocusedWindow::GuardViolations => {
                    // Guard violations window removed
                }
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

    /// Update the window tracking to reflect current window states
    pub(super) fn update_window_tracking(&mut self) {
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
    }

    /// Focus a specific window by ID
    pub(super) fn focus_window(&mut self, window_id: &str) {
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
            }
            "resource_details" => {
                // Resource/template editor windows removed
            }
            "cloudformation_scene" => {
                // CloudFormation scene graph removed
            }
            "chat_window" => {
                // Chat window removed
            }
            "control_bridge" => {
                // AgentControlWindow removed - using AgentManagerWindow instead
            }
            "template_sections" => {
                // Resource/template editor windows removed
            }
            "resource_json_editor" => {
                // Resource/template editor windows removed
            }
            "verification_window" => {
                self.verification_window.visible = true;
                self.set_focused_window(FocusedWindow::Verification);
            }
            "guard_violations" => {
                // Guard violations window removed
            }
            _ => {
                // Handle resource form windows with dynamic IDs
                if window_id.starts_with("resource_form_") {
                    // Resource/template editor windows removed
                }
                // TODO: Handle property type windows and other dynamic windows
            }
        }
    }
}
