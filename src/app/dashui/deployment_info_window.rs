use crate::app::dashui::window_focus::{FocusableWindow, SimpleShowParams};
use crate::app::notifications::Notification;
use crate::app::projects::{DeploymentState, DeploymentStatus};
use chrono::{DateTime, Local, Utc};
use egui::{self, Context, RichText, ScrollArea};

/// Deployment Information Window showing detailed deployment status
#[derive(Default)]
pub struct DeploymentInfoWindow {
    pub open: bool,
    /// Currently selected tab (0 = Status, 1 = Events)
    pub selected_tab: usize,
}

impl DeploymentInfoWindow {
    /// Helper function to format UTC time as local time
    fn format_local_time(utc_time: &DateTime<Utc>) -> String {
        let local_time: DateTime<Local> = utc_time.with_timezone(&Local);
        local_time.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Helper function to format duration between two times
    fn format_duration_between(start: &DateTime<Utc>, end: &DateTime<Utc>) -> String {
        let duration = end.signed_duration_since(*start);
        if duration.num_hours() > 0 {
            format!(
                "{}h {}m {}s",
                duration.num_hours(),
                duration.num_minutes() % 60,
                duration.num_seconds() % 60
            )
        } else if duration.num_minutes() > 0 {
            format!(
                "{}m {}s",
                duration.num_minutes(),
                duration.num_seconds() % 60
            )
        } else {
            format!("{}s", duration.num_seconds())
        }
    }

    /// Helper function to format duration from start time to now
    fn format_duration_from_now(start: &DateTime<Utc>) -> String {
        let now = Utc::now();
        Self::format_duration_between(start, &now)
    }
}

impl DeploymentInfoWindow {
    /// Show the deployment information window
    pub fn show(
        &mut self,
        ctx: &Context,
        notification: Option<&Notification>,
        deployment_status: Option<&DeploymentStatus>,
    ) -> Option<egui::Rect> {
        self.show_with_focus(ctx, notification, deployment_status, false)
    }

    pub fn show_with_focus(
        &mut self,
        ctx: &Context,
        notification: Option<&Notification>,
        deployment_status: Option<&DeploymentStatus>,
        bring_to_front: bool,
    ) -> Option<egui::Rect> {
        if !self.open {
            return None;
        }

        let mut window_open = self.open;
        let mut window_rect = None;

        let mut window = egui::Window::new("Deployment Information")
            .open(&mut window_open)
            .resizable(true)
            .min_width(500.0)
            .min_height(400.0)
            .collapsible(false);

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        if let Some(response) = window.show(ctx, |ui| {
            // Tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.selected_tab, 0, "📊 Status");
                ui.selectable_value(&mut self.selected_tab, 1, "📋 Events");
            });

            ui.separator();

            // Tab content
            ScrollArea::vertical().show(ui, |ui| match self.selected_tab {
                0 => self.render_deployment_content(ui, notification, deployment_status),
                1 => self.render_events_content(ui, deployment_status),
                _ => self.render_deployment_content(ui, notification, deployment_status),
            });
        }) {
            window_rect = Some(response.response.rect);
        }

        // Update window open state
        self.open = window_open;

        window_rect
    }

    fn render_deployment_content(
        &self,
        ui: &mut egui::Ui,
        notification: Option<&Notification>,
        deployment_status: Option<&DeploymentStatus>,
    ) {
        ui.heading("CloudFormation Deployment Status");
        ui.add_space(10.0);

        match (notification, deployment_status) {
            (Some(notif), Some(status)) => {
                // We have both notification and deployment status
                self.render_full_deployment_info(ui, notif, status);
            }
            (Some(notif), None) => {
                // Only have notification (deployment in progress or failed without project data)
                self.render_notification_only(ui, notif);
            }
            (None, Some(status)) => {
                // Only have deployment status (from project file)
                self.render_status_only(ui, status);
            }
            (None, None) => {
                // No deployment information available
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label(
                        RichText::new("No deployment information available")
                            .size(16.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label("This environment has not been deployed yet.");
                });
            }
        }
    }

    fn render_full_deployment_info(
        &self,
        ui: &mut egui::Ui,
        notification: &Notification,
        status: &DeploymentStatus,
    ) {
        // Environment and stack information
        if let Some(deployment_data) = &notification.deployment_data {
            ui.group(|ui| {
                ui.label(RichText::new("Deployment Details").strong().size(14.0));
                ui.separator();

                egui::Grid::new("deployment_details_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Environment:");
                        ui.label(RichText::new(&deployment_data.environment_name).strong());
                        ui.end_row();

                        ui.label("Stack Name:");
                        ui.label(&deployment_data.stack_name);
                        ui.end_row();

                        ui.label("Account ID:");
                        ui.label(&status.account_id);
                        ui.end_row();

                        ui.label("Region:");
                        ui.label(&status.region);
                        ui.end_row();

                        ui.label("Deployment ID:");
                        ui.label(RichText::new(&status.deployment_id).monospace());
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);
        }

        // Current status
        ui.group(|ui| {
            ui.label(RichText::new("Current Status").strong().size(14.0));
            ui.separator();

            let (status_text, status_color) = match status.status {
                DeploymentState::InProgress => {
                    ("In Progress", egui::Color32::from_rgb(70, 130, 200))
                }
                DeploymentState::Completed => ("Completed", egui::Color32::from_rgb(40, 180, 40)),
                DeploymentState::Failed => ("Failed", egui::Color32::from_rgb(220, 50, 50)),
                DeploymentState::Cancelled => ("Cancelled", egui::Color32::from_rgb(255, 150, 0)),
            };

            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.colored_label(status_color, RichText::new(status_text).strong());
            });

            if let Some(stack_status) = &status.stack_status {
                ui.horizontal(|ui| {
                    ui.label("CloudFormation Status:");
                    ui.label(RichText::new(stack_status).monospace());
                });
            }

            ui.horizontal(|ui| {
                ui.label("Last Updated:");
                ui.label(Self::format_local_time(&status.last_updated));
            });
        });

        ui.add_space(10.0);

        // Error message if any
        if let Some(error_message) = &status.error_message {
            ui.group(|ui| {
                ui.label(
                    RichText::new("Error Details")
                        .strong()
                        .size(14.0)
                        .color(egui::Color32::from_rgb(220, 50, 50)),
                );
                ui.separator();
                ui.label(RichText::new(error_message).color(egui::Color32::from_rgb(220, 50, 50)));
            });
            ui.add_space(10.0);
        }

        // Timeline
        ui.group(|ui| {
            ui.label(RichText::new("Timeline").strong().size(14.0));
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Initiated:");
                ui.label(Self::format_local_time(&status.initiated_at));
            });

            ui.horizontal(|ui| {
                ui.label("Duration:");
                // Show live duration if still in progress, otherwise show final duration
                let duration_text = if matches!(status.status, DeploymentState::InProgress) {
                    Self::format_duration_from_now(&status.initiated_at)
                } else {
                    Self::format_duration_between(&status.initiated_at, &status.last_updated)
                };
                ui.label(duration_text);
            });
        });
    }

    fn render_notification_only(&self, ui: &mut egui::Ui, notification: &Notification) {
        ui.group(|ui| {
            ui.label(RichText::new("Current Deployment").strong().size(14.0));
            ui.separator();

            if let Some(deployment_data) = &notification.deployment_data {
                egui::Grid::new("notification_details_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Environment:");
                        ui.label(RichText::new(&deployment_data.environment_name).strong());
                        ui.end_row();

                        ui.label("Stack Name:");
                        ui.label(&deployment_data.stack_name);
                        ui.end_row();

                        ui.label("Deployment ID:");
                        ui.label(RichText::new(&deployment_data.deployment_id).monospace());
                        ui.end_row();

                        ui.label("Status:");
                        if deployment_data.is_polling {
                            ui.colored_label(
                                egui::Color32::from_rgb(70, 130, 200),
                                "In Progress (Polling)",
                            );
                        } else {
                            ui.colored_label(egui::Color32::from_rgb(40, 180, 40), "Stable");
                        }
                        ui.end_row();
                    });
            }

            ui.add_space(10.0);
            ui.label("Message:");
            ui.label(&notification.errors[0].message);
        });
    }

    fn render_status_only(&self, ui: &mut egui::Ui, status: &DeploymentStatus) {
        ui.group(|ui| {
            ui.label(RichText::new("Last Deployment").strong().size(14.0));
            ui.separator();

            egui::Grid::new("status_details_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Stack Name:");
                    ui.label(&status.stack_name);
                    ui.end_row();

                    ui.label("Account ID:");
                    ui.label(&status.account_id);
                    ui.end_row();

                    ui.label("Region:");
                    ui.label(&status.region);
                    ui.end_row();

                    ui.label("Deployment ID:");
                    ui.label(RichText::new(&status.deployment_id).monospace());
                    ui.end_row();
                });
        });

        ui.add_space(10.0);

        // Status information
        ui.group(|ui| {
            ui.label(RichText::new("Status").strong().size(14.0));
            ui.separator();

            let (status_text, status_color) = match status.status {
                DeploymentState::InProgress => {
                    ("In Progress", egui::Color32::from_rgb(70, 130, 200))
                }
                DeploymentState::Completed => ("Completed", egui::Color32::from_rgb(40, 180, 40)),
                DeploymentState::Failed => ("Failed", egui::Color32::from_rgb(220, 50, 50)),
                DeploymentState::Cancelled => ("Cancelled", egui::Color32::from_rgb(255, 150, 0)),
            };

            ui.horizontal(|ui| {
                ui.label("Final Status:");
                ui.colored_label(status_color, RichText::new(status_text).strong());
            });

            if let Some(stack_status) = &status.stack_status {
                ui.horizontal(|ui| {
                    ui.label("CloudFormation Status:");
                    ui.label(RichText::new(stack_status).monospace());
                });
            }

            ui.horizontal(|ui| {
                ui.label("Completed:");
                ui.label(Self::format_local_time(&status.last_updated));
            });
        });

        if let Some(error_message) = &status.error_message {
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.label(
                    RichText::new("Error Details")
                        .strong()
                        .size(14.0)
                        .color(egui::Color32::from_rgb(220, 50, 50)),
                );
                ui.separator();
                ui.label(RichText::new(error_message).color(egui::Color32::from_rgb(220, 50, 50)));
            });
        }
    }

    fn render_events_content(
        &self,
        ui: &mut egui::Ui,
        deployment_status: Option<&DeploymentStatus>,
    ) {
        ui.heading("CloudFormation Stack Events");
        ui.add_space(10.0);

        match deployment_status {
            Some(status) if !status.stack_events.is_empty() => {
                ui.horizontal(|ui| {
                    ui.label(format!("Total Events: {}", status.stack_events.len()));
                    if let Some(last_poll) = &status.last_event_poll {
                        ui.separator();
                        ui.label(format!(
                            "Last Updated: {}",
                            Self::format_local_time(last_poll)
                        ));
                    }
                });

                ui.add_space(10.0);

                // Events table
                egui::Grid::new("events_grid")
                    .num_columns(5)
                    .spacing([8.0, 4.0])
                    .min_col_width(80.0)
                    .striped(true)
                    .show(ui, |ui| {
                        // Header row
                        ui.label(RichText::new("Time").strong());
                        ui.label(RichText::new("Resource").strong());
                        ui.label(RichText::new("Type").strong());
                        ui.label(RichText::new("Status").strong());
                        ui.label(RichText::new("Reason").strong());
                        ui.end_row();

                        // Event rows (already sorted newest first)
                        for event in &status.stack_events {
                            ui.label(Self::format_local_time(&event.timestamp));

                            // Resource ID with truncation for long names
                            let resource_display = if event.logical_resource_id.len() > 20 {
                                format!("{}...", &event.logical_resource_id[..17])
                            } else {
                                event.logical_resource_id.clone()
                            };
                            ui.label(RichText::new(resource_display).monospace());

                            // Resource type with truncation
                            let type_display = if event.resource_type.len() > 25 {
                                format!("{}...", &event.resource_type[..22])
                            } else {
                                event.resource_type.clone()
                            };
                            ui.label(RichText::new(type_display).monospace());

                            // Status with color coding
                            let status_color = if event.resource_status.contains("FAILED") {
                                egui::Color32::from_rgb(220, 50, 50)
                            } else if event.resource_status.contains("COMPLETE") {
                                egui::Color32::from_rgb(40, 180, 40)
                            } else if event.resource_status.contains("IN_PROGRESS") {
                                egui::Color32::from_rgb(70, 130, 200)
                            } else {
                                egui::Color32::GRAY
                            };
                            ui.colored_label(status_color, &event.resource_status);

                            // Reason with truncation
                            let reason_display = event
                                .resource_status_reason
                                .as_ref()
                                .map(|r| {
                                    if r.len() > 40 {
                                        format!("{}...", &r[..37])
                                    } else {
                                        r.clone()
                                    }
                                })
                                .unwrap_or_else(|| "-".to_string());
                            ui.label(reason_display);

                            ui.end_row();
                        }
                    });
            }
            Some(_) => {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label(
                        RichText::new("No events available")
                            .size(16.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label("Events will appear here as the deployment progresses.");
                });
            }
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label(
                        RichText::new("No deployment information available")
                            .size(16.0)
                            .color(egui::Color32::GRAY),
                    );
                    ui.add_space(10.0);
                    ui.label("Deploy a CloudFormation stack to see events here.");
                });
            }
        }
    }
}

impl FocusableWindow for DeploymentInfoWindow {
    type ShowParams = SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "deployment_info_window"
    }

    fn window_title(&self) -> String {
        "Deployment Information".to_string()
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        _params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        // This implementation doesn't use the simple params, as we need deployment data
        // The main show_with_focus method with deployment data should be used instead
        self.show_with_focus(ctx, None, None, bring_to_front);
    }
}
