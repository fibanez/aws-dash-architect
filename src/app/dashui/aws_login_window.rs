use crate::app::aws_identity::{AwsIdentityCenter, LoginState};
use crate::app::dashui::window_focus::{FocusableWindow, PositionShowParams};
use egui::{self, Context, RichText, ScrollArea, Vec2};
use std::sync::{Arc, Mutex};
use std::thread;

/// AWS Login window component
pub struct AwsLoginWindow {
    pub open: bool,
    pub identity_center_url: String,
    pub identity_center_region: String,
    pub default_role_name: String,
    login_in_progress: bool,
    completing_login: bool,
    error_message: Option<String>,
    aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>,
    pub accounts_window_open: bool,
    pub credentials_debug_window_open: bool,
    pub logged_out: bool, // Flag to indicate that user has just logged out
    first_open: bool,     // Track if this is the first time opening the window
}

impl Default for AwsLoginWindow {
    fn default() -> Self {
        Self {
            open: false,
            identity_center_url: "https://XXXXXXX.awsapps.com/start/".to_string(),
            identity_center_region: "us-east-1".to_string(),
            default_role_name: "awsdash".to_string(),
            login_in_progress: false,
            completing_login: false,
            error_message: None,
            aws_identity: None,
            accounts_window_open: false,
            credentials_debug_window_open: false,
            logged_out: false,
            first_open: true,
        }
    }
}

impl AwsLoginWindow {
    /// Reset the first_open flag to force centering on next open
    pub fn reset_position(&mut self) {
        self.first_open = true;
    }

    /// Show the AWS Login window
    pub fn show(
        &mut self,
        ctx: &Context,
        window_pos: Option<egui::Pos2>,
    ) -> (Option<Arc<Mutex<AwsIdentityCenter>>>, Option<egui::Rect>) {
        self.show_with_focus(ctx, window_pos, false)
    }

    pub fn show_with_focus(
        &mut self,
        ctx: &Context,
        window_pos: Option<egui::Pos2>,
        bring_to_front: bool,
    ) -> (Option<Arc<Mutex<AwsIdentityCenter>>>, Option<egui::Rect>) {
        // Check for logout flag and return None if set
        if self.logged_out {
            self.logged_out = false; // Reset the flag after use
            return (None, None);
        }

        // Always return the identity center reference if we have one, regardless of window state
        let result = self.aws_identity.clone();

        // If window is not open, just return the existing reference without rendering the window
        if !self.open {
            return (result, None);
        }

        let mut window_open = self.open;
        let mut window_rect = None;

        let mut window = egui::Window::new("AWS Identity Center Login")
            .open(&mut window_open)
            .resizable(true)
            .min_width(450.0)
            .collapsible(false);

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        // Apply position if provided, or center on first open
        if let Some(pos) = window_pos {
            window = window.current_pos(pos);
        } else if self.first_open {
            // Force center position on first open (override any saved position)
            let screen_rect = ctx.screen_rect();
            let window_size = Vec2::new(450.0, 400.0); // Estimated window size
            let center_pos = screen_rect.center() - window_size / 2.0;
            window = window.current_pos(center_pos);
            self.first_open = false; // Mark that we've positioned it once
        }

        if let Some(response) = window.show(ctx, |ui| {
            // Form fields in a table layout
            ui.add_enabled_ui(!self.login_in_progress, |ui| {
                egui::Grid::new("login_form_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .striped(false)
                    .show(ui, |ui| {
                        ui.label("Identity Center URL:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.identity_center_url)
                                .desired_width(300.0),
                        );
                        ui.end_row();

                        ui.label("Region:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.identity_center_region)
                                .desired_width(300.0),
                        );
                        ui.end_row();

                        ui.label("Default Role:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.default_role_name)
                                .desired_width(300.0),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            // Error message if any
            if let Some(error) = &self.error_message {
                ui.horizontal_centered(|ui| {
                    ui.add(egui::Label::new(
                        RichText::new(error).color(egui::Color32::from_rgb(220, 50, 50)),
                    ));
                });
                ui.add_space(10.0);
            }

            // Login buttons
            ui.vertical_centered(|ui| {
                match &self.aws_identity {
                    None => {
                        // Initial login button
                        let response = ui.add_enabled(
                            !self.login_in_progress,
                            egui::Button::new("Login with AWS Identity Center"),
                        );

                        if response.clicked() {
                            self.start_login();
                        }
                    }
                    Some(aws_identity) => {
                        // Check login state
                        let login_state = aws_identity.lock().unwrap().login_state.clone();

                        // Handle state transitions
                        match &login_state {
                            LoginState::LoggedIn => {
                                // Login completed successfully, but don't auto-open debug window
                                if self.completing_login {
                                    tracing::info!("Login completed successfully, debug window available via button");
                                }
                                self.completing_login = false;
                            }
                            LoginState::Error(_) => {
                                self.completing_login = false;
                            }
                            _ => {}
                        }

                        match login_state {
                            LoginState::DeviceAuthorization(auth_data) => {
                                ui.vertical_centered(|ui| {
                                    if self.completing_login {
                                        // Show spinner while completing login
                                        ui.label(
                                            RichText::new("Gathering credentials...")
                                                .strong(),
                                        );
                                        ui.add_space(5.0);
                                        ui.spinner();

                                        // Request continuous repaints while showing spinner
                                        ctx.request_repaint();
                                    } else {
                                        // Show normal device authorization UI
                                        ui.label(
                                            RichText::new("Please complete the login in your browser")
                                                .strong(),
                                        );
                                        ui.add_space(5.0);

                                        ui.label(format!("Verification code: {}", auth_data.user_code));

                                        if let Some(uri_complete) = &auth_data.verification_uri_complete
                                        {
                                            ui.hyperlink_to("Open login page", uri_complete);
                                        } else {
                                            ui.hyperlink_to(
                                                "Open login page",
                                                &auth_data.verification_uri,
                                            );
                                        }

                                        ui.add_space(10.0);

                                        if ui.button("I've completed the login").clicked() {
                                            self.complete_login(ctx);
                                        }
                                    }
                                });
                            }
                            LoginState::LoggedIn => {
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new("Successfully logged in!")
                                            .color(egui::Color32::from_rgb(50, 200, 80))
                                            .strong(),
                                    );
                                    ui.add_space(3.0);
                                    ui.label(
                                        RichText::new("It is safe to close this window")
                                            .color(egui::Color32::from_rgb(255, 165, 0))
                                            .size(14.0),
                                    );
                                    ui.add_space(5.0);

                                    ui.horizontal(|ui| {
                                        if ui.button("View Accounts").clicked() {
                                            self.accounts_window_open = true;
                                        }

                                        if ui.button("View Credentials").clicked() {
                                            self.credentials_debug_window_open = true;
                                            tracing::info!(
                                                "User manually opened credentials debug window"
                                            );
                                        }

                                        if ui.button("Logout").clicked() {
                                            tracing::info!("User clicked 'Logout'");
                                            self.logout();
                                        }
                                    });
                                });
                            }
                            LoginState::Error(error) => {
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(&error)
                                            .color(egui::Color32::from_rgb(220, 50, 50))
                                            .strong(),
                                    );
                                    ui.add_space(5.0);

                                    if ui.button("Try Again").clicked() {
                                        tracing::info!("User clicked 'Try Again' after error");
                                        self.aws_identity = None;
                                        self.error_message = None;
                                        self.login_in_progress = false;
                                        self.completing_login = false;
                                    }
                                });
                            }
                            LoginState::NotLoggedIn => {
                                // Reset and try again
                                self.aws_identity = None;
                            }
                        }
                    }
                }
            });
        }) {
            window_rect = Some(response.response.rect);
        }

        // Update window open state
        self.open = window_open;

        // Show accounts window if needed
        if self.accounts_window_open {
            if let Some(aws_identity) = &self.aws_identity {
                let aws_identity_clone = aws_identity.clone();
                // Pass through the bring_to_front parameter to the sub-window
                self.show_accounts_window_with_focus(ctx, &aws_identity_clone, bring_to_front);
            }
        }

        // Return the aws_identity reference and window rect
        (self.aws_identity.clone(), window_rect)
    }

    /// Start the login process
    pub fn start_login(&mut self) {
        tracing::info!(
            "Starting login process with URL: {}, Region: {}, Role: {}",
            self.identity_center_url,
            self.identity_center_region,
            self.default_role_name
        );

        if self.identity_center_url.is_empty() {
            self.error_message = Some("Identity Center URL is required".to_string());
            tracing::warn!("Login attempt with empty Identity Center URL");
            return;
        }

        if self.identity_center_region.is_empty() {
            self.error_message = Some("Region is required".to_string());
            tracing::warn!("Login attempt with empty Region");
            return;
        }

        if self.default_role_name.is_empty() {
            self.error_message = Some("Default Role is required".to_string());
            tracing::warn!("Login attempt with empty Default Role");
            return;
        }

        self.error_message = None;
        self.login_in_progress = true;
        tracing::info!("Login validation successful, proceeding with login");

        // Create a new identity center instance
        let mut identity_center = AwsIdentityCenter::new(
            self.identity_center_url.clone(),
            self.default_role_name.clone(),
            self.identity_center_region.clone(),
        );

        // Initialize (mock implementation)
        match identity_center.initialize() {
            Ok(_) => {
                // Create the shared reference for the UI to access
                let identity_center = Arc::new(Mutex::new(identity_center));

                // Start device authorization in a separate thread to not block the UI
                let identity_center_clone = identity_center.clone();

                thread::spawn(move || {
                    let mut identity_center = identity_center_clone.lock().unwrap();
                    if let Err(err) = identity_center.start_device_authorization() {
                        let error_msg = format!("Failed to start device authorization: {}", err);
                        identity_center.login_state = LoginState::Error(error_msg);
                    }
                });

                self.aws_identity = Some(identity_center);
            }
            Err(err) => {
                self.error_message = Some(format!("Failed to initialize: {}", err));
                self.login_in_progress = false;
            }
        }
    }

    /// Complete the login process after user has authorized in browser
    fn complete_login(&mut self, ctx: &Context) {
        tracing::info!("User clicked 'I've completed the login'");
        if let Some(aws_identity) = &self.aws_identity {
            // Set the local flag to show spinner
            self.completing_login = true;

            // Request immediate repaint to show the spinner
            ctx.request_repaint();

            // Clone the identity reference for the thread
            let aws_identity_clone = aws_identity.clone();

            // We need to run this in a separate thread to not block the UI
            thread::spawn(move || {
                // Small delay to ensure UI has time to render the spinner
                thread::sleep(std::time::Duration::from_millis(100));

                tracing::info!("Starting thread to complete device authorization");
                let mut identity_center = match aws_identity_clone.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        tracing::error!("Failed to lock AWS identity center: {}", e);
                        return;
                    }
                };

                match identity_center.complete_device_authorization() {
                    Ok(_) => {
                        tracing::info!("Successfully completed login");

                        // Try to get default role credentials
                        match identity_center.get_default_role_credentials() {
                            Ok(creds) => {
                                tracing::info!("Successfully obtained default role credentials");
                                if let Some(exp) = creds.expiration {
                                    tracing::info!(
                                        "Default role credentials expire at: {}",
                                        exp.format("%Y-%m-%d %H:%M:%S UTC")
                                    );
                                }

                                // Store credentials directly
                                identity_center.default_role_credentials = Some(creds);
                                tracing::info!(
                                    "Credentials stored and debug window should open automatically"
                                );
                            }
                            Err(err) => {
                                tracing::error!("Failed to get default role credentials: {}", err);
                                // Continue anyway, we still have a successful login
                            }
                        }
                    }
                    Err(err) => {
                        let error_msg = format!("Failed to complete login: {}", err);
                        tracing::error!("{}", error_msg);
                        identity_center.login_state = LoginState::Error(error_msg);
                    }
                }
            });
        } else {
            tracing::error!("Called complete_login but aws_identity is None");
            self.error_message = Some("Internal error: AWS identity not initialized".to_string());
        }
    }

    /// Logout from AWS Identity Center and reset window state
    fn logout(&mut self) {
        if let Some(aws_identity) = &self.aws_identity {
            // Create a clone for the thread
            let aws_identity_clone = aws_identity.clone();

            // We need to run this in a separate thread to avoid UI lock
            thread::spawn(move || {
                if let Ok(mut identity_center) = aws_identity_clone.lock() {
                    identity_center.logout();
                } else {
                    tracing::error!("Failed to lock AWS identity center for logout");
                }
            });

            // Reset window state
            self.aws_identity = None;
            self.error_message = None;
            self.login_in_progress = false;
            self.completing_login = false;
            self.accounts_window_open = false;
            self.credentials_debug_window_open = false;
            self.logged_out = true; // Set logged out flag so app.rs can detect logout
        } else {
            tracing::error!("Called logout but aws_identity is None");
        }
    }

    /// Show accounts window with account list and roles
    #[allow(dead_code)]
    fn show_accounts_window(
        &mut self,
        ctx: &Context,
        aws_identity: &Arc<Mutex<AwsIdentityCenter>>,
    ) {
        self.show_accounts_window_with_focus(ctx, aws_identity, false)
    }

    /// Show accounts window with focus capability
    fn show_accounts_window_with_focus(
        &mut self,
        ctx: &Context,
        aws_identity: &Arc<Mutex<AwsIdentityCenter>>,
        bring_to_front: bool,
    ) {
        // Clone the AWS Identity Center state for UI display
        let accounts = {
            let identity_center = aws_identity.lock().unwrap();
            identity_center.accounts.clone()
        };

        let mut window_open = self.accounts_window_open;

        let mut window = egui::Window::new("AWS Accounts")
            .open(&mut window_open)
            .resizable(true)
            .default_size(Vec2::new(600.0, 400.0));

        // Bring to front if requested
        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
                ui.vertical(|ui| {
                    if accounts.is_empty() {
                        ui.label("No accounts found");
                    } else {
                        ScrollArea::vertical()
                            .max_height(600.0)
                            .show(ui, |ui| {
                                for account in &accounts {
                                    ui.collapsing(
                                        format!("{} ({})", account.account_name, account.account_id),
                                        |ui| {
                                            // Account details with console button
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Role:");
                                                        ui.label(RichText::new(&account.role_name).strong());
                                                    });
                                                });
                                            });

                                            if let Some(email) = &account.account_email {
                                                ui.horizontal(|ui| {
                                                    ui.label("Email:");
                                                    ui.label(email);
                                                });
                                            }

                                            // Show available roles
                                            let roles = {
                                                let identity_center = aws_identity.lock().unwrap();
                                                identity_center.get_account_roles(&account.account_id)
                                            };

                                            ui.collapsing("Available Roles", |ui| {
                                                if roles.is_empty() {
                                                    ui.label("No roles found");
                                                } else {
                                                    // Create a grid for role, assume button, and console button
                                                    egui::Grid::new("available_roles_grid")
                                                        .num_columns(3)
                                                        .spacing([10.0, 6.0])
                                                        .min_col_width(150.0)
                                                        .show(ui, |ui| {
                                                            // Add header row
                                                            ui.label(RichText::new("Role Name").strong());
                                                            ui.label(""); // Empty cell for assume button
                                                            ui.label(""); // Empty cell for console button
                                                            ui.end_row();

                                                            for role in roles {
                                                                // Role name in first column
                                                                ui.label(&role);

                                                                // Assume button in second column
                                                                if ui.button("Assume").clicked() {
                                                                    // Clone what we need for the thread
                                                                    let aws_identity_clone = aws_identity.clone();
                                                                    let account_id = account.account_id.clone();
                                                                    let role_name = role.clone();

                                                                    // Get credentials in a separate thread
                                                                    tracing::info!("Attempting to assume role {} for account {}",
                                                                                 role_name, account_id);
                                                                    thread::spawn(move || {
                                                                        let mut identity_center = match aws_identity_clone.lock() {
                                                                            Ok(guard) => guard,
                                                                            Err(e) => {
                                                                                tracing::error!("Failed to lock AWS identity center for role assumption: {}", e);
                                                                                return;
                                                                            }
                                                                        };

                                                                        tracing::info!("Requesting credentials for account {} with role {}",
                                                                                     account_id, role_name);
                                                                        match identity_center.get_account_credentials(&account_id, &role_name) {
                                                                            Ok(creds) => {
                                                                                tracing::info!("Successfully assumed role {} for account {}",
                                                                                             role_name, account_id);
                                                                                if let Some(exp) = creds.expiration {
                                                                                    tracing::info!("Credentials expire at: {}",
                                                                                                 exp.format("%Y-%m-%d %H:%M:%S UTC"));
                                                                                }
                                                                            },
                                                                            Err(err) => {
                                                                                tracing::error!("Failed to get credentials: {}", err);
                                                                            }
                                                                        }
                                                                    });
                                                                }

                                                                // Console button in third column
                                                                if ui.button("🌐 Console").clicked() {
                                                                    // Clone what we need for the thread
                                                                    let aws_identity_clone = aws_identity.clone();
                                                                    let account_id = account.account_id.clone();
                                                                    let role_name = role.clone();

                                                                    // Open AWS console in a separate thread
                                                                    tracing::info!("Opening AWS Console for account {} with role {}",
                                                                                 account_id, role_name);
                                                                    thread::spawn(move || {
                                                                        let mut identity_center = match aws_identity_clone.lock() {
                                                                            Ok(guard) => guard,
                                                                            Err(e) => {
                                                                                tracing::error!("Failed to lock AWS identity center for console access: {}", e);
                                                                                return;
                                                                            }
                                                                        };

                                                                        match identity_center.open_aws_console(&account_id, &role_name) {
                                                                            Ok(_) => {
                                                                                tracing::info!("Successfully opened AWS Console for account {} with role {}",
                                                                                             account_id, role_name);
                                                                            },
                                                                            Err(err) => {
                                                                                tracing::error!("Failed to open AWS Console: {}", err);
                                                                            }
                                                                        }
                                                                    });
                                                                }

                                                                ui.end_row();
                                                            }
                                                        });
                                                }

                                            });

                                            // Show credentials if available
                                            if let Some(credentials) = &account.credentials {
                                                ui.collapsing("Credentials", |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Access Key ID:");
                                                        ui.label(RichText::new(&credentials.access_key_id).monospace());
                                                    });

                                                    ui.horizontal(|ui| {
                                                        ui.label("Secret Access Key:");
                                                        // Only show first few characters
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
                                                });
                                            }
                                        }
                                    );
                                    ui.add_space(5.0);
                                }
                            });
                    }
                });
            });

        self.accounts_window_open = window_open;
    }
}

impl FocusableWindow for AwsLoginWindow {
    type ShowParams = PositionShowParams;

    fn window_id(&self) -> &'static str {
        "aws_login_window"
    }

    fn window_title(&self) -> String {
        "AWS Identity Center Login".to_string()
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        params: Self::ShowParams,
        bring_to_front: bool,
    ) {
        self.show_with_focus(ctx, Some(params), bring_to_front);
    }
}
