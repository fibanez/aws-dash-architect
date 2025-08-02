use super::{Notification, NotificationManager, NotificationType};
use egui::{Context, RichText, ScrollArea};

pub struct NotificationDetailsWindow;

impl NotificationDetailsWindow {
    pub fn show(manager: &mut NotificationManager, ctx: &Context) {
        if !manager.show_details_window {
            return;
        }

        let mut open = manager.show_details_window;
        let selected_id = manager.selected_notification_id.clone();

        if let Some(notification_id) = selected_id {
            if let Some(notification) = manager.get_notification(&notification_id).cloned() {
                egui::Window::new(format!(
                    "{} - {}",
                    notification.get_icon(),
                    notification.title
                ))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .min_width(500.0)
                .min_height(300.0)
                .show(ctx, |ui| {
                    Self::show_notification_details(ui, &notification, manager);
                });
            } else {
                // Notification no longer exists, close the window
                open = false;
            }
        } else {
            open = false;
        }

        manager.show_details_window = open;
        if !open {
            manager.selected_notification_id = None;
        }
    }

    fn show_notification_details(
        ui: &mut egui::Ui,
        notification: &Notification,
        manager: &mut NotificationManager,
    ) {
        // Header with status and source
        ui.horizontal(|ui| {
            ui.colored_label(
                notification.get_color(),
                format!("{} {}", notification.get_icon(), notification.title),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Copy to Clipboard").clicked() {
                    let summary = Self::create_clipboard_summary(notification);
                    ui.ctx().copy_text(summary);
                }
            });
        });

        ui.separator();

        // Source information
        ui.horizontal(|ui| {
            ui.label(RichText::new("Source:").strong());
            ui.label(&notification.source);
        });

        // Timestamp
        ui.horizontal(|ui| {
            ui.label(RichText::new("Time:").strong());
            ui.label(format!("{:?} ago", notification.created_at.elapsed()));
        });

        if notification.expires_at.is_some() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Type:").strong());
                ui.label("Auto-dismissing");
            });
        }

        ui.separator();

        // Error/message section
        if !notification.errors.is_empty() {
            let section_title = match notification.notification_type {
                NotificationType::Error => format!("Errors ({})", notification.errors.len()),
                NotificationType::Warning => format!("Warnings ({})", notification.errors.len()),
                NotificationType::Info => "Information".to_string(),
                NotificationType::Success => "Success".to_string(),
                NotificationType::DeploymentStatus => "Deployment Status".to_string(),
            };

            ui.group(|ui| {
                ui.label(
                    RichText::new(section_title)
                        .color(notification.get_color())
                        .strong(),
                );

                ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for (i, error) in notification.errors.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.colored_label(notification.get_color(), "•");
                            ui.vertical(|ui| {
                                ui.label(&error.message);
                                if let Some(code) = &error.code {
                                    ui.label(
                                        RichText::new(format!("Code: {}", code)).size(11.0).weak(),
                                    );
                                }
                                if let Some(details) = &error.details {
                                    ui.label(RichText::new(details).size(11.0).weak());
                                }
                            });
                        });

                        if i < notification.errors.len() - 1 {
                            ui.separator();
                        }
                    }
                });
            });
        }

        // Action buttons
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Close").clicked() {
                manager.show_details_window = false;
                manager.selected_notification_id = None;
            }

            if notification.dismissible && ui.button("Dismiss").clicked() {
                manager.dismiss_notification(&notification.id);
            }

            // Additional actions based on notification type and source
            match notification.source.as_str() {
                "CloudFormation Validation" => {
                    if matches!(notification.notification_type, NotificationType::Error)
                        && ui.button("Fix Template").clicked()
                    {
                        // TODO: Navigate to template editor or show template fixing suggestions
                        tracing::info!(
                            "Fix Template button clicked - TODO: implement template fixing"
                        );
                    }
                }
                "Compliance Check" => {
                    if ui.button("View Compliance Report").clicked() {
                        // TODO: Navigate to full compliance report
                        tracing::info!(
                            "View Compliance Report button clicked - TODO: implement navigation"
                        );
                    }
                }
                _ => {}
            }
        });
    }

    fn create_clipboard_summary(notification: &Notification) -> String {
        let mut summary = String::new();

        summary.push_str(&format!(
            "{} - {}\n",
            notification.get_icon(),
            notification.title
        ));
        summary.push_str("==========================================\n\n");

        summary.push_str(&format!("Source: {}\n", notification.source));
        summary.push_str(&format!(
            "Time: {:?} ago\n",
            notification.created_at.elapsed()
        ));
        summary.push_str(&format!("Type: {:?}\n\n", notification.notification_type));

        if !notification.errors.is_empty() {
            let section_title = match notification.notification_type {
                NotificationType::Error => "Errors",
                NotificationType::Warning => "Warnings",
                NotificationType::Info => "Information",
                NotificationType::Success => "Success",
                NotificationType::DeploymentStatus => "Deployment Status",
            };

            summary.push_str(&format!(
                "{} ({}):\n",
                section_title,
                notification.errors.len()
            ));
            for error in &notification.errors {
                summary.push_str(&format!("• {}\n", error.message));
                if let Some(code) = &error.code {
                    summary.push_str(&format!("  Code: {}\n", code));
                }
                if let Some(details) = &error.details {
                    summary.push_str(&format!("  Details: {}\n", details));
                }
            }
            summary.push('\n');
        }

        summary
    }
}
