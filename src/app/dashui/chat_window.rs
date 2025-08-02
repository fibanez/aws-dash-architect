use crate::app::aws_identity::{AwsCredentials, AwsIdentityCenter};
use crate::app::bedrock_client::{BedrockApiClient, ChatMessage as BedrockChatMessage};
use crate::app::dashui::window_focus::{FocusableWindow, IdentityShowParams};
use egui::{Color32, Context, RichText, ScrollArea, Ui, Window};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

// Claude models
const CLAUDE_3_HAIKU: &str = "Claude 3 Haiku";
const CLAUDE_3_SONNET: &str = "Claude 3.5 Sonnet";
const CLAUDE_3_OPUS: &str = "Claude 3 Opus";
const CLAUDE_3_7_SONNET: &str = "Claude 3.7 Sonnet"; // Adding Claude 3.7 Sonnet
                                                     // Llama models
const LLAMA_3_8B: &str = "Llama 3 8B Instruct";
const LLAMA_3_70B: &str = "Llama 3 70B Instruct";
// Amazon models
const NOVA_PRO: &str = "Amazon Nova Pro"; // Preferred default model

// Model ID mappings
lazy_static::lazy_static! {
    static ref MODEL_ID_MAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        // Amazon models (first for priority)
        m.insert(NOVA_PRO, "amazon.nova-pro-v1:0"); // Nova Pro model ID
        // Claude models
        m.insert(CLAUDE_3_HAIKU, "anthropic.claude-3-haiku-20240307-v1:0");
        m.insert(CLAUDE_3_SONNET, "anthropic.claude-3-5-sonnet-20240620-v1:0");
        m.insert(CLAUDE_3_OPUS, "anthropic.claude-3-opus-20240229-v1:0");
        m.insert(CLAUDE_3_7_SONNET, "anthropic.claude-3-7-sonnet-20250219-v1:0"); // Updated Claude 3.7 Sonnet model ID
        // Meta models
        m.insert(LLAMA_3_8B, "meta.llama3-8b-instruct-v1:0");
        m.insert(LLAMA_3_70B, "meta.llama3-70b-instruct-v1:0");
        m
    };
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChatStatus {
    Idle,
    Loading,
    Error(String),
}

// Note: Now using BedrockChatMessage from bedrock_client.rs

#[derive(Debug)]
pub struct ChatWindow {
    pub open: bool,
    pub messages: Vec<ChatMessage>,
    pub input_text: String,
    pub status: ChatStatus,
    pub selected_model: String,
    available_models: Vec<String>,
    scrolled_to_bottom: bool,
    model_id_map: HashMap<String, String>,
    initialized: bool,
    initialized_region: Option<String>,
    last_token_refresh: Option<chrono::DateTime<chrono::Utc>>,
    // Add a bedrock client to query available models
    bedrock_client: Option<BedrockApiClient>,
    // Track whether models have been fetched
    models_fetched: bool,
}

impl Default for ChatWindow {
    fn default() -> Self {
        // Initial fallback models for before we can query the API
        let fallback_models = vec![
            NOVA_PRO.to_string(),          // Amazon Nova Pro first in the list (preferred)
            CLAUDE_3_7_SONNET.to_string(), // Claude 3.7 second tier
            CLAUDE_3_SONNET.to_string(),
            CLAUDE_3_HAIKU.to_string(),
            CLAUDE_3_OPUS.to_string(),
            LLAMA_3_8B.to_string(),
            LLAMA_3_70B.to_string(),
        ];

        // Create an initial fallback model ID map from the static MODEL_ID_MAP
        let mut model_id_map = HashMap::new();
        for (model_name, model_id) in MODEL_ID_MAP.iter() {
            model_id_map.insert(model_name.to_string(), model_id.to_string());
        }

        Self {
            open: false,
            messages: vec![ChatMessage {
                role: "assistant".to_string(),
                content: "Hello! I'm an assistant powered by Amazon Bedrock. How can I help you today?\n\nNote: AWS Bedrock integration is ready to use. You need to be logged in to AWS with proper Bedrock access to use these models.".to_string(),
                timestamp: chrono::Utc::now(),
            }],
            input_text: String::new(),
            status: ChatStatus::Idle,
            selected_model: NOVA_PRO.to_string(), // Default to Amazon Nova Pro as preferred model
            available_models: fallback_models,
            scrolled_to_bottom: false,
            model_id_map,
            initialized: false,
            initialized_region: None,
            last_token_refresh: None,
            bedrock_client: None,
            models_fetched: false,
        }
    }
}

impl ChatWindow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.scrolled_to_bottom = false;
        }
    }

    // Fetch available models from the AWS Bedrock API
    fn fetch_available_models(&mut self, aws_creds: &AwsCredentials) -> Result<(), String> {
        info!("Fetching available models from AWS Bedrock API");

        // Create Bedrock API client if it doesn't exist
        if self.bedrock_client.is_none() {
            let mut client = BedrockApiClient::new("us-east-1".to_string());

            // Convert AWS credentials to the format expected by the Bedrock SDK
            let credentials = aws_sdk_bedrock::config::Credentials::new(
                &aws_creds.access_key_id,
                &aws_creds.secret_access_key,
                aws_creds.session_token.clone(),
                None, // expiration time is None for this use case
                "AWSIdentityCenter",
            );

            // Initialize the client
            if let Err(e) = client.initialize_with_credentials(credentials.clone()) {
                return Err(format!("Failed to initialize Bedrock client: {}", e));
            }

            self.bedrock_client = Some(client);
        }

        // Fetch available models using the client
        if let Some(client) = &mut self.bedrock_client {
            match client.list_foundation_models() {
                Ok(models) => {
                    info!(
                        "Successfully fetched {} models from AWS Bedrock",
                        models.len()
                    );

                    // Build a new model list and map based on the fetched models
                    let mut model_names = Vec::new();
                    let mut model_id_map = HashMap::new();

                    for model in models {
                        model_names.push(model.model_name.clone());
                        model_id_map.insert(model.model_name.clone(), model.model_id.clone());
                        info!("Found model: {} ({})", model.model_name, model.model_id);
                    }

                    // Sort models by provider and name
                    model_names.sort_by(|a, b| {
                        // First group by provider prefix
                        let a_prefix = a.split_whitespace().next().unwrap_or("");
                        let b_prefix = b.split_whitespace().next().unwrap_or("");

                        if a_prefix == b_prefix {
                            // Within the same provider, sort by name
                            a.cmp(b)
                        } else {
                            // Put Claude models first, then Amazon, then others
                            if a_prefix == "Claude" {
                                std::cmp::Ordering::Less
                            } else if b_prefix == "Claude" {
                                std::cmp::Ordering::Greater
                            } else if a_prefix == "Amazon" {
                                std::cmp::Ordering::Less
                            } else if b_prefix == "Amazon" {
                                std::cmp::Ordering::Greater
                            } else {
                                a.cmp(b)
                            }
                        }
                    });

                    // Update the available models and map
                    self.available_models = model_names;
                    self.model_id_map = model_id_map;

                    // If we have models but no current selection, or current selection isn't available,
                    // set a default model (preferring Amazon Nova Pro)
                    if !self.available_models.is_empty()
                        && (!self.model_id_map.contains_key(&self.selected_model))
                    {
                        // Try to find Amazon Nova Pro
                        let nova_pro = self
                            .available_models
                            .iter()
                            .find(|name| name.contains("Nova Pro"));

                        if let Some(nova_pro_model) = nova_pro {
                            self.selected_model = nova_pro_model.clone();
                        } else {
                            // Or any Amazon model
                            let amazon = self
                                .available_models
                                .iter()
                                .find(|name| name.contains("Amazon"));

                            if let Some(amazon_model) = amazon {
                                self.selected_model = amazon_model.clone();
                            } else {
                                // Try Claude as fallback
                                let claude = self
                                    .available_models
                                    .iter()
                                    .find(|name| name.contains("Claude"));

                                if let Some(claude_model) = claude {
                                    self.selected_model = claude_model.clone();
                                } else {
                                    // Or just the first available model
                                    self.selected_model = self.available_models[0].clone();
                                }
                            }
                        }
                    }

                    // Mark models as fetched
                    self.models_fetched = true;
                    info!(
                        "Updated available models with {} models from AWS Bedrock",
                        self.available_models.len()
                    );

                    Ok(())
                }
                Err(e) => {
                    error!("Failed to fetch models from AWS Bedrock: {}", e);
                    Err(format!("Failed to fetch models: {}", e))
                }
            }
        } else {
            Err("Bedrock client not initialized".to_string())
        }
    }

    // Call Bedrock API using the AWS SDK
    fn call_bedrock_api(
        &self,
        model_id: &str,
        messages: &[BedrockChatMessage],
        aws_creds: &AwsCredentials,
    ) -> Result<String, String> {
        // Use the existing client if available, otherwise create a new one
        // Create or get a client to use
        // Create a client to use for this API call
        // We store the client in this variable to keep it alive while using its reference
        #[allow(unused_assignments)]
        let mut temp_client: Option<BedrockApiClient> = None;

        // Determine which client to use
        let client_ref = if let Some(client) = &self.bedrock_client {
            // Use the existing client
            client
        } else {
            // Create a new temporary client
            let mut new_client = BedrockApiClient::new("us-east-1".to_string());

            // Convert our AWS credentials to the format expected by the Bedrock SDK
            let credentials = aws_sdk_bedrock::config::Credentials::new(
                &aws_creds.access_key_id,
                &aws_creds.secret_access_key,
                aws_creds.session_token.clone(),
                None, // expiration time is None for this use case
                "AWSIdentityCenter",
            );

            // Initialize the client
            if let Err(e) = new_client.initialize_with_credentials(credentials) {
                return Err(format!("Failed to initialize Bedrock client: {}", e));
            }

            // Store the client in our temporary variable
            temp_client = Some(new_client);

            // Return a reference to the temporary client
            temp_client.as_ref().unwrap()
        };

        // Call the converse API
        match client_ref.converse_with_model(model_id, messages) {
            Ok(response) => Ok(response),
            Err(e) => Err(format!("Failed to call Bedrock API: {}", e)),
        }
    }

    // Store whether this is the first time opening the window
    fn is_first_open(&self) -> bool {
        // If we have only the default welcome message, it's probably the first open
        self.messages.len() == 1 && self.messages[0].role == "assistant"
    }

    pub fn send_message(&mut self, aws_identity: &Arc<Mutex<AwsIdentityCenter>>) {
        if self.input_text.trim().is_empty() {
            return;
        }

        // Add user message to chat history
        let user_message = ChatMessage {
            role: "user".to_string(),
            content: self.input_text.clone(),
            timestamp: chrono::Utc::now(),
        };
        self.messages.push(user_message);

        // Clear input field
        let user_input = std::mem::take(&mut self.input_text);

        // Update status to loading
        self.status = ChatStatus::Loading;
        self.scrolled_to_bottom = false;

        // Check if AWS Identity Center is authenticated and get credentials
        let credentials = match aws_identity.lock() {
            Ok(mut identity) => {
                // Get default role credentials for Bedrock API
                if identity.default_role_credentials.is_none() {
                    // No default role credentials, try to get them
                    info!("No default role credentials found, trying to get them now");
                    match identity.get_default_role_credentials() {
                        Ok(_) => {
                            info!("Successfully obtained default role credentials");
                            // The credentials are now stored in identity.default_role_credentials
                        }
                        Err(e) => {
                            error!("Failed to get default role credentials: {}", e);
                            let error_message = ChatMessage {
                                role: "system".to_string(),
                                content: format!("Failed to get AWS credentials: {}", e),
                                timestamp: chrono::Utc::now(),
                            };
                            self.messages.push(error_message);
                            self.status =
                                ChatStatus::Error(format!("Failed to get AWS credentials: {}", e));
                            return;
                        }
                    }
                }

                // Get credentials to use for Bedrock API
                match &identity.default_role_credentials {
                    Some(creds) => {
                        // If we haven't fetched models yet, try to fetch them now
                        if !self.models_fetched {
                            info!("Before sending message, trying to fetch Bedrock models");
                            match self.fetch_available_models(creds) {
                                Ok(_) => {
                                    info!("Successfully fetched models before sending message");
                                }
                                Err(e) => {
                                    info!(
                                        "Failed to fetch models, will continue with defaults: {}",
                                        e
                                    );
                                    // Continue with default models
                                }
                            }
                        }
                        creds.clone()
                    }
                    None => {
                        error!("No AWS credentials available");
                        let error_message = ChatMessage {
                            role: "system".to_string(),
                            content: "To use AWS Bedrock, you need to log in to AWS Identity Center first. Press Space to open the command palette, then select 'Login'.".to_string(),
                            timestamp: chrono::Utc::now(),
                        };
                        self.messages.push(error_message);
                        self.status = ChatStatus::Error("No AWS credentials available".to_string());
                        return;
                    }
                }
            }
            Err(e) => {
                error!("Failed to lock AWS identity center: {}", e);
                self.status =
                    ChatStatus::Error(format!("Failed to lock AWS identity center: {}", e));
                return;
            }
        };

        // Get model ID for the selected model name
        info!("Selected model from UI: {}", self.selected_model);
        let model_id = match self.model_id_map.get(&self.selected_model) {
            Some(id) => {
                info!(
                    "Found model ID: {} for selected model: {}",
                    id, self.selected_model
                );
                id.clone()
            }
            None => {
                error!("Model ID not found for model: {}", self.selected_model);
                error!(
                    "Available models in map: {:?}",
                    self.model_id_map.keys().collect::<Vec<_>>()
                );
                let error_message = ChatMessage {
                    role: "system".to_string(),
                    content: format!("Model ID not found for the selected model: {}. Please select a different model.", self.selected_model),
                    timestamp: chrono::Utc::now(),
                };
                self.messages.push(error_message);
                self.status =
                    ChatStatus::Error(format!("Model ID not found: {}", self.selected_model));
                return;
            }
        };

        // Prepare messages for the Bedrock API
        let mut bedrock_messages = Vec::new();

        // IMPORTANT: Bedrock API must begin with a USER message, not an assistant message
        // Skip our initial assistant welcome message for the first API call
        // For Bedrock API, we need to follow a strict alternating pattern of messages
        // where the first message MUST be from the USER, followed by ASSISTANT, then USER, etc.

        if self.messages.len() <= 2 {
            // For the first user message (with just welcome + first user message)
            // Don't add any previous context - just the user's message will be added below
            info!("First API call: Adding only the user message, no context");
            // No messages are added here - the user message will be added outside this block
        } else {
            // For existing conversations with multiple messages
            info!("Building conversation history for Bedrock API");

            // Include context from previous messages, limited to the last 10 for performance
            // but keeping the essential structure for the API
            let mut context_messages = Vec::new();

            // Get a window of recent messages, but skip the most recent one (the user's input we just added)
            // Calculate start_idx safely to avoid underflow
            let start_idx = if self.messages.len() <= 11 {
                0
            } else {
                self.messages.len() - 11
            };
            let end_idx = self.messages.len() - 1;
            info!(
                "Including context messages from index {} to {} (total messages: {})",
                start_idx,
                end_idx,
                self.messages.len()
            );

            // Copy messages to our temporary collection, skipping system messages
            for msg in &self.messages[start_idx..end_idx] {
                if msg.role != "system" {
                    // Skip system messages - AWS Bedrock doesn't support them
                    context_messages.push(msg);
                }
            }

            // CRITICAL: Ensure we start with a USER message and maintain strict alternation
            // Find the first USER message to start with
            let mut found_first_user = false;
            let mut current_role = "user"; // We must start with a user message

            // First, find and add a user message to start
            for msg in &context_messages {
                if !found_first_user && msg.role == "user" {
                    // Found first user message - add it
                    found_first_user = true;
                    bedrock_messages.push(BedrockChatMessage {
                        role: "user".to_string(),
                        content: msg.content.clone(),
                    });
                    current_role = "user";
                    break; // We found our starting user message
                }
            }

            // Now add alternating messages, continuing from where we left off
            // Only include messages that follow the proper alternating pattern
            if found_first_user {
                // Continue adding messages after the first user message, enforcing alternation
                let mut should_add = false; // Start looking for assistant after finding user

                for msg in &context_messages {
                    // Skip messages until we find the first user message again
                    if !should_add {
                        if msg.role == "user" && msg.content == bedrock_messages[0].content {
                            should_add = true; // Start adding messages after this one
                        }
                        continue;
                    }

                    // Only add messages that follow strict alternation
                    if (current_role == "user" && msg.role == "assistant")
                        || (current_role == "assistant" && msg.role == "user")
                    {
                        bedrock_messages.push(BedrockChatMessage {
                            role: msg.role.clone(),
                            content: msg.content.clone(),
                        });
                        current_role = msg.role.as_str();
                    }
                    // Skip any messages that would break the alternating pattern
                }
            } else {
                // If we couldn't find a user message to start with, use the latest user message
                // (This should be rare, but we handle it for robustness)
                info!("No prior user message found - using just current message");
                // We'll add the current user message below, outside this block
            }

            // Make sure we end with an assistant message so we can add the user message next
            // This ensures proper alternation of user -> assistant -> user (new message)
            if current_role == "user" || bedrock_messages.is_empty() {
                info!("Message sequence doesn't end with assistant - adjusting pattern");
                // If we don't have any messages yet or end with a user,
                // start fresh with just the current user message (added below)
                bedrock_messages.clear();
            }

            // Log the pattern we're sending
            let roles: Vec<&str> = bedrock_messages.iter().map(|m| m.role.as_str()).collect();
            info!(
                "Message roles prepared for API (excluding new user message): {:?}",
                roles
            );
        }

        // Count message roles for logging
        let mut user_msgs = 0;
        let mut assistant_msgs = 0;

        for msg in &bedrock_messages {
            if msg.role == "user" {
                user_msgs += 1;
            } else if msg.role == "assistant" {
                assistant_msgs += 1;
            }
        }

        // Add user's message (always ending with a user message for the model to respond to)
        bedrock_messages.push(BedrockChatMessage {
            role: "user".to_string(),
            content: user_input.clone(),
        });
        user_msgs += 1;

        info!(
            "Message count by role (in API request): user={}, assistant={}, total={}",
            user_msgs,
            assistant_msgs,
            bedrock_messages.len()
        );

        info!(
            "Final message count for Bedrock API: {} (including new user message)",
            bedrock_messages.len()
        );

        // Log first few characters of the user input for debugging (without exposing full content)
        let preview_len = user_input.len().min(20);
        info!(
            "User message preview: \"{}{}\"",
            &user_input[..preview_len],
            if user_input.len() > preview_len {
                "..."
            } else {
                ""
            }
        );

        // Call Bedrock API
        info!("Calling Bedrock API with model: {}", model_id);
        info!(
            "AWS credentials present: {}",
            !credentials.access_key_id.is_empty()
        );

        match self.call_bedrock_api(&model_id, &bedrock_messages, &credentials) {
            Ok(response) => {
                info!(
                    "Received response from Bedrock API, length: {}",
                    response.len()
                );

                // Log a preview of the response for debugging
                let preview_len = response.len().min(30);
                info!(
                    "Response preview: \"{}{}\"",
                    &response[..preview_len],
                    if response.len() > preview_len {
                        "..."
                    } else {
                        ""
                    }
                );

                // Add assistant's response to chat history
                let assistant_message = ChatMessage {
                    role: "assistant".to_string(),
                    content: response,
                    timestamp: chrono::Utc::now(),
                };
                self.messages.push(assistant_message);
                self.status = ChatStatus::Idle;

                // Update last initialization time
                self.initialized = true;
                self.initialized_region = Some("us-east-1".to_string());
                self.last_token_refresh = Some(chrono::Utc::now());
                info!(
                    "Chat state updated with response, new message count: {}",
                    self.messages.len()
                );
            }
            Err(e) => {
                error!("Failed to call Bedrock API: {}", e);
                error!(
                    "Error occurred with model: {}, message count: {}",
                    model_id,
                    bedrock_messages.len()
                );

                // Add error message to chat history
                let error_message = ChatMessage {
                    role: "system".to_string(),
                    content: format!("Failed to generate response: {}. This could be due to Bedrock service limitations or permission issues. Make sure your AWS account has access to the selected model in Bedrock.", e),
                    timestamp: chrono::Utc::now(),
                };
                self.messages.push(error_message);
                self.status = ChatStatus::Error(format!("Failed to generate response: {}", e));

                // Reset initialization status
                self.initialized = false;
                error!("Chat initialization status reset due to error");
            }
        }
    }

    pub fn show(&mut self, ctx: &Context, aws_identity: &Arc<Mutex<AwsIdentityCenter>>) {
        self.show_with_offset(ctx, aws_identity);
    }

    pub fn show_with_offset(
        &mut self,
        ctx: &Context,
        aws_identity: &Arc<Mutex<AwsIdentityCenter>>,
    ) {
        if !self.open {
            return;
        }

        // Calculate size based on screen
        let screen_rect = ctx.screen_rect();
        // Use fixed initial width of 400px, and make sure height doesn't exceed screen
        let chat_width = 400.0;
        let chat_height = screen_rect.height() * 0.85;

        // Create a mutable copy of self.open for the window to modify
        let mut open = self.open;

        // Check if this is the first time opening the window
        let _is_first = self.is_first_open();

        // If we're logged in to AWS but haven't fetched models yet, try to fetch them
        if !self.models_fetched {
            if let Ok(identity) = aws_identity.lock() {
                if let Some(creds) = &identity.default_role_credentials {
                    // Log that we're going to fetch models
                    info!("Chat window opened, fetching available models from AWS Bedrock");

                    // Try to fetch models - if it fails, we'll use fallback models
                    match self.fetch_available_models(creds) {
                        Ok(_) => {
                            info!("Successfully updated models from AWS Bedrock API");
                        }
                        Err(e) => {
                            error!("Failed to fetch models from AWS Bedrock: {}", e);
                            // Keep using fallback models
                        }
                    }
                }
            }
        }

        // Create the window with higher z-order to ensure visibility, including for dropdowns
        let window = Window::new("AI Assistant")
            .open(&mut open)
            .movable(true)
            .default_size([chat_width, chat_height]) // Initial size
            .min_width(360.0) // Slightly wider minimum for better dropdown visibility
            .max_width(600.0) // Allow larger width for comfort
            .min_height(400.0) // Taller minimum height
            .max_height(screen_rect.height() * 0.95) // Allow more height
            .order(egui::Order::Foreground) // Use foreground order to ensure it's on top of everything
            .collapsible(true)
            .title_bar(true); // Ensure title bar is visible

        // For all windows, including first open, allow resizing
        // This ensures the dropdown will be properly displayed regardless of window state
        window
            .resizable(true) // Allow resizing for better usability
            .show(ctx, |ui| {
                self.ui_content(ui, aws_identity);
            });

        // Update self.open based on the window's state
        self.open = open;
    }

    fn ui_content(&mut self, ui: &mut Ui, aws_identity: &Arc<Mutex<AwsIdentityCenter>>) {
        // Create a vertical layout with better spacing
        ui.vertical(|ui| {
            // Chat history area with improved styling and fixed dimensions
            // Respect user's window size but cap the growth to prevent auto-expanding
            let current_window_height = ui.available_height();
            let input_area_height = 70.0;
            let max_scroll_height = (current_window_height - input_area_height).min(500.0); // Cap at 500px

            ScrollArea::vertical()
                .auto_shrink([false, true])
                .max_height(max_scroll_height)
                .stick_to_bottom(!self.scrolled_to_bottom)
                .id_salt("chat_scroll_area") // Fixed ID to prevent auto-resize issues
                .show(ui, |ui| {
                    // Set a fixed width to prevent content from expanding the window
                    let available_width = ui.available_width();
                    ui.set_max_width(available_width);
                    self.render_chat_messages(ui);
                });

            // Status indicator with better spacing
            match &self.status {
                ChatStatus::Idle => {}
                ChatStatus::Loading => {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Waiting for response...").italics());
                    });
                }
                ChatStatus::Error(err) => {
                    ui.add_space(4.0);
                    ui.colored_label(Color32::RED, RichText::new(err).strong());
                }
            }

            // Always add space before input area
            ui.add_space(8.0);

            // Input area with improved styling
            ui.horizontal(|ui| {
                // Create a frame for the text input for better visual separation
                let frame = egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .inner_margin(egui::vec2(8.0, 6.0))
                    .corner_radius(2.0);

                frame.show(ui, |ui| {
                    let available_width = ui.available_width();
                    let text_width = available_width.min(350.0); // Prevent text area from getting too wide
                    let text_edit = egui::TextEdit::multiline(&mut self.input_text)
                        .hint_text("Type a message...")
                        .desired_width(text_width)
                        .min_size(egui::vec2(text_width, 36.0)); // Minimum size for better usability

                    let response = ui.add(text_edit);

                    // Focus on the text input when the window is opened
                    if !self.scrolled_to_bottom {
                        response.request_focus();
                        self.scrolled_to_bottom = true;
                    }

                    // Handle Enter key (without shift) to send
                    let mut send_message = false;
                    if response.has_focus() {
                        let ctx = ui.ctx();
                        let input = ctx.input(|i| {
                            (
                                i.key_pressed(egui::Key::Enter) && !i.modifiers.shift,
                                i.modifiers, // No need to clone, as Modifiers implements Copy
                            )
                        });

                        if input.0 {
                            send_message = true;
                        }
                    }

                    // Return the send message flag for use outside the frame
                    if send_message {
                        self.send_message(aws_identity);
                    }
                });

                ui.add_space(4.0);

                // Styled send button
                let send_button = egui::Button::new(RichText::new("Send").strong())
                    .min_size(egui::vec2(60.0, 36.0))
                    .fill(ui.visuals().selection.bg_fill); // Use accent color

                if ui.add(send_button).clicked() {
                    self.send_message(aws_identity);
                }
            });
        });
    }

    fn render_chat_messages(&self, ui: &mut Ui) {
        // Add an initial space at the top of the chat for better visual balance
        ui.add_space(8.0);

        for message in &self.messages {
            // Create a different style based on the role
            let (bg_color, text_color, align) = match message.role.as_str() {
                "user" => (
                    ui.visuals().selection.bg_fill,
                    Color32::WHITE, // Use white text for better contrast on accent color background
                    egui::Align::RIGHT,
                ),
                "assistant" => (
                    ui.visuals().widgets.noninteractive.bg_fill,
                    ui.visuals().widgets.noninteractive.fg_stroke.color,
                    egui::Align::LEFT,
                ),
                "system" => (
                    Color32::from_rgb(180, 0, 0), // Darker red for system messages
                    Color32::WHITE,
                    egui::Align::Center,
                ),
                _ => (
                    ui.visuals().widgets.noninteractive.bg_fill,
                    ui.visuals().widgets.noninteractive.fg_stroke.color,
                    egui::Align::LEFT,
                ),
            };

            // Simplify message layout to prevent overflow
            match align {
                egui::Align::RIGHT => {
                    ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                        self.render_single_message(ui, message, bg_color, text_color);
                    });
                }
                egui::Align::Center => {
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        self.render_single_message(ui, message, bg_color, text_color);
                    });
                }
                _ => {
                    self.render_single_message(ui, message, bg_color, text_color);
                }
            };

            // Add spacing between messages
            ui.add_space(12.0);
        }

        // Add some space at the bottom of the chat history for visual balance
        ui.add_space(8.0);
    }

    fn render_single_message(
        &self,
        ui: &mut Ui,
        message: &ChatMessage,
        bg_color: Color32,
        text_color: Color32,
    ) {
        // Calculate a maximum width for the message bubble (70% of available width)
        let max_width = ui.available_width() * 0.7;

        // Create a frame for the message that looks like a chat bubble with consistent styling
        egui::Frame::default()
            .fill(bg_color)
            .inner_margin(egui::vec2(10.0, 8.0))
            .corner_radius(2.0)
            .stroke(egui::Stroke::new(1.0, text_color.linear_multiply(0.3)))
            .show(ui, |ui| {
                // Constrain the width to ensure proper wrapping
                ui.set_max_width(max_width);

                // Add timestamp
                let time = message.timestamp.format("%H:%M:%S").to_string();

                // Different header styling for user vs assistant
                ui.horizontal(|ui| {
                    // Use strong text for the role with proper spacing
                    ui.label(
                        RichText::new(&message.role)
                            .strong()
                            .color(text_color)
                            .size(10.0),
                    );

                    // Push the time to the right side
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(time)
                                .small()
                                .color(text_color.linear_multiply(0.7)),
                        );
                    });
                });

                ui.add_space(4.0); // Space between header and separator

                // Add a subtle separator line
                let separator_color = text_color.linear_multiply(0.2);
                let cursor_pos = ui.cursor();
                ui.painter().line_segment(
                    [
                        egui::pos2(ui.min_rect().left(), cursor_pos.min.y),
                        egui::pos2(ui.min_rect().right(), cursor_pos.min.y),
                    ],
                    egui::Stroke::new(1.0, separator_color),
                );

                ui.add_space(6.0); // Space after separator

                // Format the message content to handle newlines properly
                let content_parts: Vec<&str> = message.content.split('\n').collect();

                // Use a vertical layout with reduced spacing for message content
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0; // Tighter spacing between paragraphs

                    for (i, part) in content_parts.iter().enumerate() {
                        if i > 0 && part.is_empty() {
                            // Empty line indicates paragraph break
                            ui.add_space(4.0);
                        } else if !part.is_empty() {
                            // Add the text with proper wrapping
                            ui.label(
                                RichText::new(*part)
                                    .color(text_color)
                                    .text_style(egui::TextStyle::Body),
                            );
                        }
                    }
                });
            });
    }

    /// Show the chat window with focus capability
    pub fn show_with_focus(
        &mut self,
        ctx: &Context,
        aws_identity: &Arc<Mutex<AwsIdentityCenter>>,
        _bring_to_front: bool,
    ) {
        // For now, just delegate to the existing show method
        // Note: bring_to_front parameter is not used yet but could be implemented
        // by modifying the Window creation in the show method
        self.show(ctx, aws_identity);
    }
}

impl FocusableWindow for ChatWindow {
    type ShowParams = IdentityShowParams;

    fn window_id(&self) -> &'static str {
        "chat_window"
    }

    fn window_title(&self) -> String {
        "AWS Q Chat".to_string()
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
        if let Some(aws_identity) = &params.aws_identity {
            self.show_with_focus(ctx, aws_identity, bring_to_front);
        }
    }
}
