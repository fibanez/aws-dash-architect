use super::window_focus::FocusableWindow;
use crate::app::aws_identity::{AwsCredentials, AwsIdentityCenter};
use egui::{self, Context, RichText, Vec2};
use std::sync::{Arc, Mutex};

/// Credentials Debug window component
#[derive(Default)]
pub struct CredentialsDebugWindow {
    pub open: bool,
}

impl CredentialsDebugWindow {
    /// Show the Credentials Debug window
    pub fn show(
        &mut self,
        ctx: &Context,
        aws_identity: Option<&Arc<Mutex<AwsIdentityCenter>>>,
        window_pos: Option<egui::Pos2>,
    ) -> Option<egui::Rect> {
        // If window is not open, don't render
        if !self.open {
            return None;
        }

        let mut window_open = self.open;
        let mut window_rect = None;

        let mut window = egui::Window::new("AWS Credentials Debug")
            .open(&mut window_open)
            .resizable(true)
            .default_size(Vec2::new(600.0, 400.0));

        // Apply position if provided
        if let Some(pos) = window_pos {
            window = window.current_pos(pos);
        } else {
            window = window.default_pos(ctx.screen_rect().center());
        }

        if let Some(response) = window.show(ctx, |ui| {
            ui.vertical(|ui| {
                match aws_identity {
                    Some(aws_id) => {
                        let identity_center = aws_id.lock().unwrap();

                        // Show basic AWS Identity information
                        ui.heading("AWS Identity Information");
                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            ui.label("Identity Center URL:");
                            ui.label(RichText::new(&identity_center.identity_center_url).strong());
                        });

                        ui.horizontal(|ui| {
                            ui.label("Region:");
                            ui.label(
                                RichText::new(&identity_center.identity_center_region).strong(),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("Default Role:");
                            ui.label(RichText::new(&identity_center.default_role_name).strong());
                        });

                        ui.horizontal(|ui| {
                            ui.label("Login State:");
                            ui.label(
                                RichText::new(format!("{:?}", identity_center.login_state))
                                    .strong(),
                            );
                        });

                        ui.add_space(10.0);

                        // Default Role Credentials
                        if let Some(credentials) = &identity_center.default_role_credentials {
                            ui.heading("Default Role Credentials");
                            ui.add_space(5.0);
                            Self::display_credentials(ui, credentials);
                        } else {
                            ui.heading("Default Role Credentials");
                            ui.add_space(5.0);
                            ui.label(
                                RichText::new("No default role credentials available").italics(),
                            );
                        }

                        ui.add_space(10.0);

                        // Account information in tree-like structure
                        ui.collapsing(
                            RichText::new(format!("📁 AWS Accounts ({})", identity_center.accounts.len())).strong(),
                            |ui| {
                                if identity_center.accounts.is_empty() {
                                    ui.label(RichText::new("No accounts found").italics());
                                } else {
                                    for account in &identity_center.accounts {
                                        ui.collapsing(
                                            format!("🏢 {} ({})", account.account_name, account.account_id),
                                            |ui| {
                                                // Account details in tree structure
                                                ui.collapsing("📋 Account Details", |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Role:");
                                                        ui.label(RichText::new(&account.role_name).strong());
                                                    });

                                                    if let Some(email) = &account.account_email {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Email:");
                                                            ui.label(RichText::new(email).monospace());
                                                        });
                                                    }
                                                });

                                                // Credentials section
                                                if let Some(credentials) = &account.credentials {
                                                    ui.collapsing("🔐 Credentials", |ui| {
                                                        Self::display_credentials(ui, credentials);
                                                    });
                                                } else {
                                                    ui.collapsing("🔐 Credentials", |ui| {
                                                        ui.label(RichText::new("No credentials available for this account").color(egui::Color32::GRAY));
                                                    });
                                                }
                                            },
                                        );
                                    }
                                }
                            }
                        );

                        ui.add_space(10.0);

                        // CloudFormation Deployment Role Information in tree structure
                        ui.collapsing(
                            RichText::new("CloudFormation Deployment Role").strong(),
                            |ui| {
                                match &identity_center.cloudformation_deployment_role_name {
                                    Some(role_name) => {
                                        ui.horizontal(|ui| {
                                            ui.label("Role Name:");
                                            ui.label(RichText::new(role_name).strong());
                                        });

                                        ui.add_space(5.0);
                                        ui.label(RichText::new("This role will be used for CloudFormation deployment").italics());
                                    }
                                    None => {
                                        ui.label(RichText::new("No CloudFormation deployment role discovered").color(egui::Color32::GRAY));
                                    }
                                }
                            }
                        );
                    }
                    None => {
                        ui.label("Not logged in to AWS Identity Center");
                    }
                }
            });
        }) {
            window_rect = Some(response.response.rect);
        }

        // Update window open state
        self.open = window_open;
        window_rect
    }

    // Helper function to display credentials
    fn display_credentials(ui: &mut egui::Ui, credentials: &AwsCredentials) {
        ui.horizontal(|ui| {
            ui.label("Access Key ID:");
            ui.label(RichText::new(&credentials.access_key_id).monospace());
        });

        ui.horizontal(|ui| {
            ui.label("Secret Access Key:");
            let secret = if credentials.secret_access_key.len() > 8 {
                format!("{}...", &credentials.secret_access_key[..8])
            } else {
                credentials.secret_access_key.clone()
            };
            ui.label(RichText::new(secret).monospace());
        });

        if let Some(expiration) = credentials.expiration {
            ui.horizontal(|ui| {
                ui.label("Expires:");
                ui.label(format!("{}", expiration.format("%Y-%m-%d %H:%M:%S UTC")));
            });
        }

        if let Some(token) = &credentials.session_token {
            ui.collapsing("Session Token", |ui| {
                // Show shortened token
                let short_token = if token.len() > 20 {
                    format!("{}...", &token[..20])
                } else {
                    token.clone()
                };
                ui.label(RichText::new(short_token).monospace());
            });
        }
    }

    /// Show the Credentials Debug window with focus capability
    pub fn show_with_focus(
        &mut self,
        ctx: &Context,
        aws_identity: Option<&Arc<Mutex<AwsIdentityCenter>>>,
        window_pos: Option<egui::Pos2>,
        bring_to_front: bool,
    ) -> Option<egui::Rect> {
        // If window is not open, don't render
        if !self.open {
            return None;
        }

        let mut window_open = self.open;
        let mut window_rect = None;

        let mut window = egui::Window::new("AWS Credentials Debug")
            .open(&mut window_open)
            .resizable(true)
            .default_size(Vec2::new(600.0, 400.0));

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        // Apply position if provided
        if let Some(pos) = window_pos {
            window = window.current_pos(pos);
        } else {
            window = window.default_pos(ctx.screen_rect().center());
        }

        if let Some(response) = window.show(ctx, |ui| {
            ui.vertical(|ui| {
                match aws_identity {
                    Some(aws_id) => {
                        let identity_center = aws_id.lock().unwrap();

                        // Show basic AWS Identity information
                        ui.heading("AWS Identity Information");
                        ui.add_space(5.0);

                        ui.horizontal(|ui| {
                            ui.label("Identity Center URL:");
                            ui.label(RichText::new(&identity_center.identity_center_url).monospace());
                        });

                        ui.horizontal(|ui| {
                            ui.label("Region:");
                            ui.label(RichText::new(&identity_center.identity_center_region).monospace());
                        });

                        ui.horizontal(|ui| {
                            ui.label("Default Role:");
                            ui.label(RichText::new(&identity_center.default_role_name).monospace());
                        });

                        ui.add_space(10.0);

                        // Account information in tree-like structure
                        ui.collapsing(
                            RichText::new(format!("📁 AWS Accounts ({})", identity_center.accounts.len())).strong(),
                            |ui| {
                                if identity_center.accounts.is_empty() {
                                    ui.label(RichText::new("No accounts loaded").italics());
                                } else {
                                    for account in &identity_center.accounts {
                                        ui.collapsing(
                                            format!("🏢 {} ({})", account.account_name, account.account_id),
                                            |ui| {
                                                // Account details in tree structure
                                                ui.collapsing("📋 Account Details", |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Role:");
                                                        ui.label(RichText::new(&account.role_name).strong());
                                                    });

                                                    if let Some(email) = &account.account_email {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Email:");
                                                            ui.label(RichText::new(email).monospace());
                                                        });
                                                    }
                                                });

                                                // Credentials section
                                                match &account.credentials {
                                                    Some(creds) => {
                                                        ui.collapsing("🔐 Credentials", |ui| {
                                                            Self::display_credentials(ui, creds);
                                                        });
                                                    }
                                                    None => {
                                                        ui.collapsing("🔐 Credentials", |ui| {
                                                            ui.label(RichText::new("No credentials available").color(egui::Color32::GRAY));
                                                        });
                                                    }
                                                }
                                            },
                                        );
                                    }
                                }
                            }
                        );

                        ui.add_space(10.0);

                        // CloudFormation Deployment Role Information in tree structure
                        ui.collapsing(
                            RichText::new("CloudFormation Deployment Role").strong(),
                            |ui| {
                                match &identity_center.cloudformation_deployment_role_name {
                                    Some(role_name) => {
                                        ui.horizontal(|ui| {
                                            ui.label("Role Name:");
                                            ui.label(RichText::new(role_name).strong());
                                        });

                                        ui.add_space(5.0);
                                        ui.label(RichText::new("This role will be used for CloudFormation deployment").italics());
                                    }
                                    None => {
                                        ui.label(RichText::new("No CloudFormation deployment role discovered").color(egui::Color32::GRAY));
                                        ui.add_space(5.0);
                                    }
                                }
                            }
                        );

                        ui.add_space(10.0);

                        // Infrastructure Information in tree structure
                        ui.collapsing(
                            RichText::new("Infrastructure Information").strong(),
                            |ui| {
                                match &identity_center.infrastructure_info {
                                    Some(infrastructure_info) => {
                                        ui.collapsing("Database Details", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label("Table Name:");
                                                ui.label(RichText::new(&infrastructure_info.table_name).strong());
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Region:");
                                                ui.label(RichText::new(&infrastructure_info.db_region).monospace());
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Account:");
                                                ui.label(RichText::new(&infrastructure_info.db_account).monospace());
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Source Role:");
                                                ui.label(RichText::new(&infrastructure_info.source_role).monospace());
                                            });

                                            ui.collapsing("📋 DynamoDB Table ARN", |ui| {
                                                ui.label(RichText::new(&infrastructure_info.dynamodb_table_arn).monospace());
                                            });
                                        });

                                        ui.collapsing(
                                            format!("☁️ CloudFormation Roles ({})", infrastructure_info.cloudformation_role_arns.len()),
                                            |ui| {
                                                if infrastructure_info.cloudformation_role_arns.is_empty() {
                                                    ui.label(RichText::new("No CloudFormation roles found").color(egui::Color32::GRAY));
                                                } else {
                                                    for (i, role_arn) in infrastructure_info.cloudformation_role_arns.iter().enumerate() {
                                                        ui.horizontal(|ui| {
                                                            ui.label(format!("{}:", i + 1));
                                                            ui.label(RichText::new(role_arn).monospace());
                                                        });
                                                    }
                                                }
                                            }
                                        );
                                    }
                                    None => {
                                        ui.label(RichText::new("No infrastructure information extracted").color(egui::Color32::GRAY));
                                        ui.add_space(5.0);
                                        ui.label("To extract infrastructure information:");
                                        ui.label("1. Login to AWS Identity Center");
                                        ui.label("2. Use extract_infrastructure_info() with CloudFormation role name");
                                    }
                                }
                            }
                        );
                    }
                    None => {
                        ui.label("No AWS Identity Center configured");
                        ui.add_space(10.0);
                        ui.label("To see credential information:");
                        ui.label("1. Set up AWS Identity Center");
                        ui.label("2. Log in through the login window");
                    }
                }
            });
        }) {
            window_rect = Some(response.response.rect);
        }

        // Update window state
        self.open = window_open;

        window_rect
    }
}

impl FocusableWindow for CredentialsDebugWindow {
    type ShowParams = super::window_focus::SimpleShowParams;

    fn window_id(&self) -> &'static str {
        "credentials_debug"
    }

    fn window_title(&self) -> String {
        "AWS Credentials Debug".to_string()
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
        // For the trait implementation, we'll show without AWS identity and positioning
        // This provides a basic interface - the actual app handler will use the full show_with_focus method
        self.show_with_focus(ctx, None, None, bring_to_front);
    }
}
