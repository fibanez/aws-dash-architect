use egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub mod error_window;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Error,
    Warning,
    Info,
    Success,
    /// Persistent deployment status notification
    DeploymentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationError {
    pub message: String,
    pub code: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub notification_type: NotificationType,
    pub errors: Vec<NotificationError>,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
    #[serde(skip, default)]
    pub expires_at: Option<Instant>,
    pub dismissible: bool,
    pub source: String, // e.g., "CloudFormation Validation", "Compliance Check"

    /// Additional data for deployment status notifications
    pub deployment_data: Option<DeploymentNotificationData>,
}

/// Additional data for deployment status notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentNotificationData {
    pub environment_name: String,
    pub stack_name: String,
    pub deployment_id: String,
    pub is_polling: bool,
}

impl Notification {
    pub fn new_error(
        id: String,
        title: String,
        errors: Vec<NotificationError>,
        source: String,
    ) -> Self {
        Self {
            id,
            title,
            notification_type: NotificationType::Error,
            errors,
            created_at: Instant::now(),
            expires_at: None, // Errors don't auto-expire
            dismissible: true,
            source,
            deployment_data: None,
        }
    }

    pub fn new_warning(
        id: String,
        title: String,
        errors: Vec<NotificationError>,
        source: String,
    ) -> Self {
        Self {
            id,
            title,
            notification_type: NotificationType::Warning,
            errors,
            created_at: Instant::now(),
            expires_at: Some(Instant::now() + Duration::from_secs(30)),
            dismissible: true,
            source,
            deployment_data: None,
        }
    }

    pub fn new_info(id: String, title: String, message: String, source: String) -> Self {
        Self {
            id,
            title,
            notification_type: NotificationType::Info,
            errors: vec![NotificationError {
                message,
                code: None,
                details: None,
            }],
            created_at: Instant::now(),
            expires_at: Some(Instant::now() + Duration::from_secs(10)),
            dismissible: true,
            source,
            deployment_data: None,
        }
    }

    pub fn new_success(id: String, title: String, message: String, source: String) -> Self {
        Self {
            id,
            title,
            notification_type: NotificationType::Success,
            errors: vec![NotificationError {
                message,
                code: None,
                details: None,
            }],
            created_at: Instant::now(),
            expires_at: Some(Instant::now() + Duration::from_secs(5)),
            dismissible: true,
            source,
            deployment_data: None,
        }
    }

    pub fn new_deployment_status(
        id: String,
        environment_name: String,
        stack_name: String,
        deployment_id: String,
        message: String,
        is_polling: bool,
    ) -> Self {
        Self {
            id,
            title: format!("Deployment Status - {}", environment_name),
            notification_type: NotificationType::DeploymentStatus,
            errors: vec![NotificationError {
                message,
                code: None,
                details: None,
            }],
            created_at: Instant::now(),
            expires_at: None,   // Deployment status notifications are persistent
            dismissible: false, // Cannot be dismissed, only updated
            source: "CloudFormation Deployment".to_string(),
            deployment_data: Some(DeploymentNotificationData {
                environment_name,
                stack_name,
                deployment_id,
                is_polling,
            }),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() > expires_at
        } else {
            false
        }
    }

    pub fn get_color(&self) -> Color32 {
        match self.notification_type {
            NotificationType::Error => Color32::from_rgb(220, 50, 50),
            NotificationType::Warning => Color32::from_rgb(255, 150, 0),
            NotificationType::Info => Color32::from_rgb(70, 130, 200),
            NotificationType::Success => Color32::from_rgb(40, 180, 40),
            NotificationType::DeploymentStatus => {
                // Color based on deployment data if available
                if let Some(deployment_data) = &self.deployment_data {
                    if deployment_data.is_polling {
                        Color32::from_rgb(70, 130, 200) // Blue for in-progress
                    } else {
                        Color32::from_rgb(40, 180, 40) // Green for completed/stable
                    }
                } else {
                    Color32::from_rgb(70, 130, 200) // Default blue
                }
            }
        }
    }

    pub fn get_icon(&self) -> &'static str {
        match self.notification_type {
            NotificationType::Error => "✗",
            NotificationType::Warning => "⚠",
            NotificationType::Info => "ℹ",
            NotificationType::Success => "✓",
            NotificationType::DeploymentStatus => {
                if let Some(deployment_data) = &self.deployment_data {
                    if deployment_data.is_polling {
                        "⟳" // Rotating/polling icon
                    } else {
                        "☁" // Cloud icon for deployment
                    }
                } else {
                    "☁"
                }
            }
        }
    }
}

#[derive(Default)]
pub struct NotificationManager {
    notifications: HashMap<String, Notification>,
    pub show_details_window: bool,
    pub selected_notification_id: Option<String>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            notifications: HashMap::new(),
            show_details_window: false,
            selected_notification_id: None,
        }
    }

    pub fn add_notification(&mut self, notification: Notification) {
        self.notifications
            .insert(notification.id.clone(), notification);
    }

    pub fn dismiss_notification(&mut self, id: &str) {
        self.notifications.remove(id);
        if let Some(selected_id) = &self.selected_notification_id {
            if selected_id == id {
                self.selected_notification_id = None;
                self.show_details_window = false;
            }
        }
    }

    pub fn clear_expired(&mut self) {
        // Don't clear deployment status notifications as they are persistent
        self.notifications.retain(|_, notification| {
            !notification.is_expired()
                || matches!(
                    notification.notification_type,
                    NotificationType::DeploymentStatus
                )
        });
    }

    pub fn get_active_notifications(&self) -> Vec<&Notification> {
        let mut notifications: Vec<&Notification> = self.notifications.values().collect();
        notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        notifications
    }

    pub fn get_notification(&self, id: &str) -> Option<&Notification> {
        self.notifications.get(id)
    }

    pub fn has_errors(&self) -> bool {
        self.notifications
            .values()
            .any(|n| matches!(n.notification_type, NotificationType::Error))
    }

    pub fn has_warnings(&self) -> bool {
        self.notifications
            .values()
            .any(|n| matches!(n.notification_type, NotificationType::Warning))
    }

    pub fn get_error_count(&self) -> usize {
        self.notifications
            .values()
            .filter(|n| matches!(n.notification_type, NotificationType::Error))
            .count()
    }

    pub fn get_warning_count(&self) -> usize {
        self.notifications
            .values()
            .filter(|n| matches!(n.notification_type, NotificationType::Warning))
            .count()
    }

    pub fn show_notification_details(&mut self, notification_id: String) {
        self.selected_notification_id = Some(notification_id);
        self.show_details_window = true;
    }

    /// Update or create a deployment status notification
    pub fn update_deployment_status(
        &mut self,
        environment_name: &str,
        stack_name: String,
        deployment_id: String,
        message: String,
        is_polling: bool,
    ) {
        let id = format!("deployment_status_{}", environment_name);

        if let Some(existing) = self.notifications.get_mut(&id) {
            // Update existing notification
            existing.errors[0].message = message;
            if let Some(deployment_data) = &mut existing.deployment_data {
                deployment_data.stack_name = stack_name;
                deployment_data.deployment_id = deployment_id;
                deployment_data.is_polling = is_polling;
            }
        } else {
            // Create new deployment status notification
            let notification = Notification::new_deployment_status(
                id,
                environment_name.to_string(),
                stack_name,
                deployment_id,
                message,
                is_polling,
            );
            self.add_notification(notification);
        }
    }

    /// Get deployment status for a specific environment
    pub fn get_deployment_status(&self, environment_name: &str) -> Option<&Notification> {
        let id = format!("deployment_status_{}", environment_name);
        self.notifications.get(&id)
    }

    /// Remove deployment status notification for an environment
    pub fn clear_deployment_status(&mut self, environment_name: &str) {
        let id = format!("deployment_status_{}", environment_name);
        self.notifications.remove(&id);
    }

    pub fn render_status_bar_indicator(&mut self, ui: &mut egui::Ui) {
        self.clear_expired();

        let error_count = self.get_error_count();
        let warning_count = self.get_warning_count();

        // Check if we have deployment status notifications
        let has_deployment_notifications = self
            .notifications
            .values()
            .any(|n| matches!(n.notification_type, NotificationType::DeploymentStatus));

        // Always show if we have deployment status, errors, or warnings
        if error_count > 0 || warning_count > 0 || has_deployment_notifications {
            ui.separator();

            // Show deployment status notifications first (permanent)
            // Collect deployment notification IDs first to avoid borrow checker issues
            let deployment_notification_ids: Vec<String> = self
                .notifications
                .iter()
                .filter_map(|(id, n)| {
                    if matches!(n.notification_type, NotificationType::DeploymentStatus) {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Collect deployment notification data to avoid borrow checker issues
            let deployment_data: Vec<(String, &'static str, egui::Color32, String, bool)> =
                deployment_notification_ids
                    .iter()
                    .filter_map(|notification_id| {
                        self.notifications.get(notification_id).map(|notification| {
                            let icon = notification.get_icon();
                            let color = notification.get_color();
                            let message = notification.errors[0].message.clone();
                            let is_polling = notification
                                .deployment_data
                                .as_ref()
                                .map(|data| data.is_polling)
                                .unwrap_or(false);
                            (notification_id.clone(), icon, color, message, is_polling)
                        })
                    })
                    .collect();

            // Now render the deployment notifications
            for (notification_id, icon, color, message, is_polling) in deployment_data {
                let clicked = ui
                    .horizontal(|ui| {
                        if is_polling {
                            // Use egui's built-in spinner for polling deployments
                            ui.add(egui::Spinner::new().size(16.0));
                        } else {
                            // Show static icon for completed deployments
                            ui.colored_label(color, icon);
                        }

                        // Clickable message text
                        ui.colored_label(color, &message).clicked()
                    })
                    .inner;

                if clicked {
                    self.show_notification_details(notification_id);
                }
            }

            if error_count > 0 {
                let error_text = if error_count == 1 {
                    "1 error".to_string()
                } else {
                    format!("{} errors", error_count)
                };

                if ui
                    .colored_label(Color32::from_rgb(220, 50, 50), format!("✗ {}", error_text))
                    .clicked()
                {
                    // Find the first error notification and show it
                    if let Some(error_notification) = self
                        .get_active_notifications()
                        .iter()
                        .find(|n| matches!(n.notification_type, NotificationType::Error))
                    {
                        self.show_notification_details(error_notification.id.clone());
                    }
                }
            }

            if warning_count > 0 {
                let warning_text = if warning_count == 1 {
                    "1 warning".to_string()
                } else {
                    format!("{} warnings", warning_count)
                };

                if ui
                    .colored_label(
                        Color32::from_rgb(255, 150, 0),
                        format!("⚠ {}", warning_text),
                    )
                    .clicked()
                {
                    // Find the first warning notification and show it
                    if let Some(warning_notification) = self
                        .get_active_notifications()
                        .iter()
                        .find(|n| matches!(n.notification_type, NotificationType::Warning))
                    {
                        self.show_notification_details(warning_notification.id.clone());
                    }
                }
            }
        }
    }
}
