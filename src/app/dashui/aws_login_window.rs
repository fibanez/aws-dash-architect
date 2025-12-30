use crate::app::aws_identity::{AwsIdentityCenter, LoginState};
use crate::app::dashui::window_focus::{FocusableWindow, PositionShowParams};
use egui::{self, Context, RichText, ScrollArea, Vec2};
use std::sync::{Arc, Mutex};
use std::thread;

/// AWS Login window component
pub struct AwsLoginWindow {
    pub open: bool,
    identity_center_short_name: String, // Short name for Identity Center (e.g., "mycompany")
    identity_center_region: String,
    default_role_name: String,
    login_in_progress: bool,
    completing_login: bool,
    error_message: Option<String>,
    aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>,
    pub accounts_window_open: bool,
    pub logged_out: bool, // Flag to indicate that user has just logged out
    first_open: bool,     // Track if this is the first time opening the window
}

impl Default for AwsLoginWindow {
    fn default() -> Self {
        // Try to load defaults from sso.json (DEBUG builds only)
        #[cfg(debug_assertions)]
        let (short_name, region, role_name) = {
            use crate::app::sso_config::SsoConfig;
            if let Some(config) = SsoConfig::load() {
                tracing::info!("Loaded SSO defaults from sso.json");
                (config.short_name(), config.region, config.default_role_name)
            } else {
                ("your-org".to_string(), "us-east-1".to_string(), "awsdash".to_string())
            }
        };

        #[cfg(not(debug_assertions))]
        let (short_name, region, role_name) = {
            ("your-org".to_string(), "us-east-1".to_string(), "awsdash".to_string())
        };

        Self {
            open: false,
            identity_center_short_name: short_name,
            identity_center_region: region,
            default_role_name: role_name,
            login_in_progress: false,
            completing_login: false,
            error_message: None,
            aws_identity: None,
            accounts_window_open: false,
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

    /// Extract short name from full URL or validate input
    /// Quietly handles full URLs by extracting the short name
    /// Returns cleaned short name with only alphanumeric and hyphens
    fn validate_and_clean_short_name(input: &str) -> String {
        // If input looks like a full URL, extract the short name
        let cleaned = if input.starts_with("https://") || input.starts_with("http://") {
            // Extract short name from full URL
            if let Some(start) = input.find("://") {
                let after_protocol = &input[start + 3..];
                if let Some(dot_pos) = after_protocol.find('.') {
                    after_protocol[..dot_pos].to_string()
                } else {
                    input.to_string()
                }
            } else {
                input.to_string()
            }
        } else if input.contains(".awsapps.com") {
            // Handle case where user pastes without protocol
            if let Some(dot_pos) = input.find('.') {
                input[..dot_pos].to_string()
            } else {
                input.to_string()
            }
        } else {
            input.to_string()
        };

        // Only keep alphanumeric characters and hyphens
        cleaned
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect()
    }

    /// Build full Identity Center URL from short name
    fn build_full_url(&self) -> String {
        format!(
            "https://{}.awsapps.com/start/",
            self.identity_center_short_name
        )
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
            .default_size(egui::Vec2::new(450.0, 400.0))
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
                        ui.horizontal(|ui| {
                            // Text input for short name
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.identity_center_short_name)
                                    .desired_width(150.0)
                                    .hint_text("your-org"),
                            );

                            // Apply validation on text change
                            if response.changed() {
                                self.identity_center_short_name =
                                    Self::validate_and_clean_short_name(&self.identity_center_short_name);
                            }

                            // Static suffix label
                            ui.label(".awsapps.com/start/");
                        });
                        ui.end_row();

                        ui.label("Region:");
                        egui::ComboBox::from_label("")
                            .selected_text(&self.identity_center_region)
                            .width(300.0)
                            .show_ui(ui, |ui| {
                                ui.label(RichText::new("Default Regions").strong());
                                ui.selectable_value(&mut self.identity_center_region, "us-east-1".to_string(), "us-east-1 (US East N. Virginia)");
                                ui.selectable_value(&mut self.identity_center_region, "us-east-2".to_string(), "us-east-2 (US East Ohio)");
                                ui.selectable_value(&mut self.identity_center_region, "us-west-1".to_string(), "us-west-1 (US West N. California)");
                                ui.selectable_value(&mut self.identity_center_region, "us-west-2".to_string(), "us-west-2 (US West Oregon)");
                                ui.selectable_value(&mut self.identity_center_region, "ca-central-1".to_string(), "ca-central-1 (Canada Central)");
                                ui.selectable_value(&mut self.identity_center_region, "eu-central-1".to_string(), "eu-central-1 (Europe Frankfurt)");
                                ui.selectable_value(&mut self.identity_center_region, "eu-west-1".to_string(), "eu-west-1 (Europe Ireland)");
                                ui.selectable_value(&mut self.identity_center_region, "eu-west-2".to_string(), "eu-west-2 (Europe London)");
                                ui.selectable_value(&mut self.identity_center_region, "eu-west-3".to_string(), "eu-west-3 (Europe Paris)");
                                ui.selectable_value(&mut self.identity_center_region, "eu-north-1".to_string(), "eu-north-1 (Europe Stockholm)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-south-1".to_string(), "ap-south-1 (Asia Pacific Mumbai)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-northeast-1".to_string(), "ap-northeast-1 (Asia Pacific Tokyo)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-northeast-2".to_string(), "ap-northeast-2 (Asia Pacific Seoul)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-southeast-1".to_string(), "ap-southeast-1 (Asia Pacific Singapore)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-southeast-2".to_string(), "ap-southeast-2 (Asia Pacific Sydney)");
                                ui.selectable_value(&mut self.identity_center_region, "sa-east-1".to_string(), "sa-east-1 (South America Sao Paulo)");
                                ui.selectable_value(&mut self.identity_center_region, "us-gov-west-1".to_string(), "us-gov-west-1 (AWS GovCloud US West)");
                                ui.selectable_value(&mut self.identity_center_region, "us-gov-east-1".to_string(), "us-gov-east-1 (AWS GovCloud US East)");

                                ui.separator();
                                ui.label(RichText::new("Opt-in Regions").strong());
                                ui.selectable_value(&mut self.identity_center_region, "af-south-1".to_string(), "af-south-1 (Africa Cape Town)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-east-1".to_string(), "ap-east-1 (Asia Pacific Hong Kong)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-south-2".to_string(), "ap-south-2 (Asia Pacific Hyderabad)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-southeast-3".to_string(), "ap-southeast-3 (Asia Pacific Jakarta)");
                                ui.selectable_value(&mut self.identity_center_region, "ap-southeast-4".to_string(), "ap-southeast-4 (Asia Pacific Melbourne)");
                                ui.selectable_value(&mut self.identity_center_region, "eu-central-2".to_string(), "eu-central-2 (Europe Zurich)");
                                ui.selectable_value(&mut self.identity_center_region, "eu-south-1".to_string(), "eu-south-1 (Europe Milan)");
                                ui.selectable_value(&mut self.identity_center_region, "eu-south-2".to_string(), "eu-south-2 (Europe Spain)");
                                ui.selectable_value(&mut self.identity_center_region, "me-south-1".to_string(), "me-south-1 (Middle East Bahrain)");
                                ui.selectable_value(&mut self.identity_center_region, "me-central-1".to_string(), "me-central-1 (Middle East UAE)");
                                ui.selectable_value(&mut self.identity_center_region, "il-central-1".to_string(), "il-central-1 (Israel Tel Aviv)");
                                ui.selectable_value(&mut self.identity_center_region, "ca-west-1".to_string(), "ca-west-1 (Canada West Calgary)");
                            });
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
                        // Try to get login state without blocking
                        // If lock is held by background thread, show spinner
                        let login_state = match aws_identity.try_lock() {
                            Ok(guard) => Some(guard.login_state.clone()),
                            Err(_) => None, // Lock held by background thread
                        };

                        // If we couldn't get the lock, show spinner and wait
                        let login_state = match login_state {
                            Some(state) => state,
                            None => {
                                ui.label(
                                    RichText::new("Gathering credentials...")
                                        .strong(),
                                );
                                ui.add_space(5.0);
                                ui.spinner();
                                ctx.request_repaint();
                                return; // Exit the vertical_centered closure
                            }
                        };

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

                                        // Get the URL to use (prefer complete URI if available)
                                        let login_url = if let Some(uri_complete) = &auth_data.verification_uri_complete {
                                            uri_complete.clone()
                                        } else {
                                            auth_data.verification_uri.clone()
                                        };

                                        // Show buttons centered and close together using 4 columns
                                        // Buttons in middle two columns (1 and 2), leaving 0 and 3 empty
                                        ui.columns(4, |columns| {
                                            // Column 0: empty (provides left spacing)
                                            columns[1].vertical_centered(|ui| {
                                                if ui.button("Open login page").clicked() {
                                                    ui.ctx().open_url(egui::OpenUrl::new_tab(&login_url));
                                                }
                                            });
                                            columns[2].vertical_centered(|ui| {
                                                if ui.button("Copy Link").clicked() {
                                                    ui.ctx().copy_text(login_url);
                                                }
                                            });
                                            // Column 3: empty (provides right spacing)
                                        });

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
    fn start_login(&mut self) {
        let full_url = self.build_full_url();

        tracing::info!(
            "Starting login process with URL: {}, Region: {}, Role: {}",
            full_url,
            self.identity_center_region,
            self.default_role_name
        );

        if self.identity_center_short_name.is_empty() {
            self.error_message = Some("Identity Center short name is required".to_string());
            tracing::warn!("Login attempt with empty Identity Center short name");
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
            full_url,
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

                // Call complete_device_authorization WITHOUT holding the lock
                // This prevents UI freeze since the operation blocks for 30+ seconds
                let auth_result = {
                    let mut identity_center = match aws_identity_clone.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            tracing::error!(
                                "Failed to lock AWS identity center for authorization: {}",
                                e
                            );
                            return;
                        }
                    };

                    // Call the blocking operation - lock is released when guard drops
                    identity_center.complete_device_authorization()
                }; // Lock released here - UI can now render spinner smoothly

                // Process result and update state (re-acquire lock briefly)
                match auth_result {
                    Ok(_) => {
                        tracing::info!("Device authorization completed successfully");

                        // Re-acquire lock to get credentials
                        if let Ok(mut identity_center) = aws_identity_clone.lock() {
                            // Try to get default role credentials
                            match identity_center.get_default_role_credentials() {
                                Ok(creds) => {
                                    tracing::info!(
                                        "Successfully obtained default role credentials"
                                    );
                                    if let Some(exp) = creds.expiration {
                                        tracing::info!(
                                            "Default role credentials expire at: {}",
                                            exp.format("%Y-%m-%d %H:%M:%S UTC")
                                        );
                                    }

                                    // Store credentials directly
                                    identity_center.default_role_credentials = Some(creds);

                                    // Set LoggedIn state AFTER credentials are stored
                                    // This ensures credentials are available when state says "logged in"
                                    identity_center.login_state = LoginState::LoggedIn;
                                    tracing::info!(
                                        "Credentials stored and login state set to LoggedIn"
                                    );
                                }
                                Err(err) => {
                                    tracing::error!(
                                        "Failed to get default role credentials: {}",
                                        err
                                    );
                                    // Set logged in anyway - credentials are optional for some operations
                                    identity_center.login_state = LoginState::LoggedIn;
                                    tracing::warn!("Login completed but credentials unavailable");
                                }
                            }
                        } else {
                            tracing::error!("Failed to re-acquire lock for credential storage");
                        }
                    }
                    Err(err) => {
                        let error_msg = format!("Failed to complete login: {}", err);
                        tracing::error!("{}", error_msg);

                        // Re-acquire lock to set error state
                        if let Ok(mut identity_center) = aws_identity_clone.lock() {
                            identity_center.login_state = LoginState::Error(error_msg);
                        } else {
                            tracing::error!("Failed to re-acquire lock for error state");
                        }
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
                    ScrollArea::vertical().max_height(600.0).show(ui, |ui| {
                        for account in &accounts {
                            ui.collapsing(
                                format!("{} ({})", account.account_name, account.account_id),
                                |ui| {
                                    // Show email if available
                                    if let Some(email) = &account.account_email {
                                        ui.horizontal(|ui| {
                                            ui.label("Email:");
                                            ui.label(email);
                                        });
                                    }
                                },
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
