//! Control Bridge Window - UI Component for AI Command Center
//!
//! This window provides the user interface for the Bridge Agent, a persistent AI assistant
//! that helps manage AWS infrastructure. The UI handles message display, user input,
//! and agent interaction, while business logic is delegated to:
//!
//! - `bridge::agents::bridge_agent::BridgeAgent` - Agent creation and system prompt
//! - `bridge::callback_handlers` - Event handling and JSON capture
//! - `bridge::agents::task_agent::TaskAgent` - Specialized task execution
//!
//! ## Architecture
//!
//! - **UI Layer** (this file): Message rendering, input handling, window management
//! - **Business Layer**: Agent orchestration, tool execution, credential management
//! - **Integration**: AWS Identity Center for authentication and credential injection

use crate::app::aws_identity::AwsIdentityCenter;
use crate::app::bridge::{
    clear_global_bridge_sender, get_global_cancellation_manager, set_global_bridge_sender,
    set_global_model, BridgeDebugEvent, log_bridge_debug_event, ModelConfig, ModelSettings,
};
use crate::app::bridge::agents::bridge_agent::{AwsCredentials, BridgeAgent};
use crate::app::dashui::window_focus::{FocusableWindow, IdentityShowParams};
use chrono::{DateTime, Utc};
use egui::{CollapsingHeader, Color32, RichText, ScrollArea, TextEdit, Window};
use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Arc, Mutex};
use stood::agent::{result::AgentResult, Agent};
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

// ============================================================================
// MESSAGE SYSTEM FOR egui
// ============================================================================

// Removed SubAgentEvent - agents now handle their own event loops without streaming

/// Response from agent execution in separate thread
#[derive(Debug)]
pub enum AgentResponse {
    Success(AgentResult),
    Error(String),
    JsonDebug(JsonDebugData),
    ModelChanged {
        model_id: String,
    },
    // Tool callback responses for creating tree structure
    ToolCallStart {
        parent_message: Message,
    },
    ToolCallComplete {
        parent_message_id: String,
        child_message: Message,
    },
}

// Real-time streaming updates from agent execution
// Removed StreamingUpdate - using blocking execution

/// JSON debug data captured from model interactions
#[derive(Debug, Clone)]
pub struct JsonDebugData {
    pub json_type: JsonDebugType,
    pub json_content: String, // Composed JSON (our reconstruction)
    pub raw_json_content: Option<String>, // Raw JSON from provider API (if available)
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum JsonDebugType {
    Request,
    Response,
}

/// Message types for the GUI
#[derive(Debug, Clone)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Debug,
    JsonRequest,  // Model request JSON data
    JsonResponse, // Model response JSON data
}

impl MessageRole {
    fn icon(&self) -> &'static str {
        match self {
            MessageRole::User => "👤",
            MessageRole::Assistant => "⚡", // Lightning bolt - supported by egui
            MessageRole::System => "ℹ",
            MessageRole::Debug => "🔧",
            MessageRole::JsonRequest => "📤",  // Outgoing request
            MessageRole::JsonResponse => "📥", // Incoming response
        }
    }

    fn color(&self, dark_mode: bool) -> Color32 {
        match (self, dark_mode) {
            // Dark mode: bright colors for visibility on dark background
            (MessageRole::User, true) => Color32::from_rgb(100, 150, 255), // Bright blue
            (MessageRole::Assistant, true) => Color32::from_rgb(100, 255, 150), // Bright green
            (MessageRole::System, true) => Color32::from_rgb(255, 200, 100), // Bright orange
            (MessageRole::Debug, true) => Color32::from_rgb(180, 180, 180), // Light gray
            (MessageRole::JsonRequest, true) => Color32::from_rgb(255, 140, 0), // Bright orange for JSON
            (MessageRole::JsonResponse, true) => Color32::from_rgb(255, 140, 0), // Bright orange for JSON

            // Light mode: darker colors for visibility on light background
            (MessageRole::User, false) => Color32::from_rgb(50, 75, 150), // Dark blue
            (MessageRole::Assistant, false) => Color32::from_rgb(50, 150, 75), // Dark green
            (MessageRole::System, false) => Color32::from_rgb(180, 120, 50), // Dark orange
            (MessageRole::Debug, false) => Color32::from_rgb(100, 100, 100), // Dark gray
            (MessageRole::JsonRequest, false) => Color32::from_rgb(200, 100, 0), // Dark orange for JSON
            (MessageRole::JsonResponse, false) => Color32::from_rgb(200, 100, 0), // Dark orange for JSON
        }
    }
}

/// A single message in the conversation
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String, // Unique identifier for this message
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub summary: Option<String>,
    pub debug_info: Option<String>,
    pub nested_messages: Vec<Message>,
    pub agent_source: Option<String>, // Track which agent/tool generated this message
    pub json_debug_data: Vec<JsonDebugData>, // JSON capture data for this message/agent
}

impl Message {
    pub fn new(role: MessageRole, content: String) -> Self {
        let summary = Self::generate_summary(&content);
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: Utc::now(),
            summary: Some(summary),
            debug_info: None,
            nested_messages: Vec::new(),
            agent_source: None,
            json_debug_data: Vec::new(),
        }
    }

    pub fn new_with_agent(role: MessageRole, content: String, agent_source: String) -> Self {
        let summary = Self::generate_summary(&content);
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: Utc::now(),
            summary: Some(summary),
            debug_info: None,
            nested_messages: Vec::new(),
            agent_source: Some(agent_source),
            json_debug_data: Vec::new(),
        }
    }

    pub fn with_debug(mut self, debug_info: String) -> Self {
        self.debug_info = Some(debug_info);
        self
    }

    pub fn add_nested_message(&mut self, message: Message) {
        self.nested_messages.push(message);
    }

    fn generate_summary(content: &str) -> String {
        let words: Vec<&str> = content.split_whitespace().take(5).collect();
        if words.len() < 5 && content.len() < 30 {
            content.to_string()
        } else {
            format!("{}...", words.join(" "))
        }
    }
}

// ============================================================================
// CONTROL BRIDGE WINDOW
// ============================================================================

/// Control Bridge - Always available AI command center
#[derive(Debug)]
pub struct ControlBridgeWindow {
    messages: VecDeque<Message>,
    input_text: String,
    agent: Arc<Mutex<Option<Agent>>>,
    response_receiver: mpsc::Receiver<AgentResponse>,
    response_sender: mpsc::Sender<AgentResponse>,
    debug_mode: bool,
    show_json_debug: bool,
    dark_mode: bool,
    pub open: bool, // Window can be opened and closed like other windows

    // UI state
    auto_scroll: bool,
    scroll_to_bottom: bool,
    last_message_time: Option<std::time::Instant>, // Time when last message was added

    // Agent processing
    processing_message: bool,

    // Option change tracking
    #[allow(dead_code)] // For future state tracking functionality
    prev_debug_mode: bool,
    #[allow(dead_code)] // For future state tracking functionality
    prev_json_debug: bool,

    // UI Debug information
    #[allow(dead_code)] // For future debug UI functionality
    show_debug_panel: bool,
    last_processing_time: Option<std::time::Duration>,
    #[allow(dead_code)] // For future statistics functionality
    message_count_stats: (usize, usize, usize, usize), // user, assistant, system, debug
    processing_start_time: Option<std::time::Instant>,

    // Expand/Collapse state management
    force_expand_all: bool,
    force_collapse_all: bool,
    message_expand_states: HashMap<String, bool>, // Track expand state by message ID
    // Active agent tracking for cancellation
    active_agents: Arc<Mutex<HashMap<String, String>>>, // agent_id -> agent_type mapping
    main_agent_active: bool,                            // Track if main Bridge Agent is processing
    // Parent-child node relationships for sub-agent visibility
    active_agent_nodes: HashMap<String, String>, // agent_id -> parent_message_id mapping

    // Model selection and configuration
    available_models: Vec<ModelConfig>, // Available AI models
    model_settings: ModelSettings,      // Model preferences and selection
    model_changed: bool,                // Flag to trigger agent recreation when model changes
}

impl Default for ControlBridgeWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ControlBridgeWindow {
    fn drop(&mut self) {
        info!("🚢 Control Bridge window dropped - clearing global Bridge sender");
        clear_global_bridge_sender();
    }
}

impl ControlBridgeWindow {
    pub fn new() -> Self {
        info!("🚢 Initializing Control Bridge (Agent will be created on first message)");

        // Create response channel for thread communication
        let (response_sender, response_receiver) = mpsc::channel();

        // Set global Bridge sender for log analysis event bubbling
        set_global_bridge_sender(response_sender.clone());
        info!("📡 Global Bridge sender configured for log analysis event bubbling");

        let mut app = Self {
            messages: VecDeque::new(),
            input_text: String::new(),
            agent: Arc::new(Mutex::new(None)), // Agent created lazily on first message
            response_receiver,
            response_sender,
            debug_mode: false,
            show_json_debug: false, // Disable JSON debug by default
            dark_mode: true,        // Default to dark mode
            open: false,            // Start closed by default
            auto_scroll: true,
            scroll_to_bottom: false,
            last_message_time: None,
            processing_message: false,
            prev_debug_mode: false,
            prev_json_debug: false,
            show_debug_panel: false,
            last_processing_time: None,
            message_count_stats: (0, 0, 0, 0),
            processing_start_time: None,
            force_expand_all: false,
            force_collapse_all: false,
            message_expand_states: HashMap::new(),
            active_agents: Arc::new(Mutex::new(HashMap::new())),
            main_agent_active: false,
            active_agent_nodes: HashMap::new(),

            // Initialize model configuration
            available_models: ModelConfig::default_models(),
            model_settings: ModelSettings::default(),
            model_changed: false,
        };

        // Set initial global model configuration
        set_global_model(app.model_settings.selected_model.clone());

        // Add welcome message
        app.add_message(Message::new_with_agent(
            MessageRole::System,
            "🚢 Ready".to_string(),
            "ControlBridge".to_string(),
        ));

        app
    }

    /// Cancel all active agents (main Bridge Agent and specialized agents)
    fn cancel_all_agents(&mut self) {
        info!("🛑 Cancelling all active agents with proper cancellation tokens");

        // Cancel main Bridge Agent
        if self.main_agent_active {
            info!("🛑 Cancelling main Bridge Agent");
            self.processing_message = false;
            self.main_agent_active = false;

            // Add cancellation message
            self.add_message(Message::new_with_agent(
                MessageRole::System,
                "🛑 Main Bridge Agent stopped by user.".to_string(),
                "ControlBridge".to_string(),
            ));
        }

        // Actually cancel all running agents using the global cancellation manager
        let cancelled_count = if let Some(cancellation_manager) = get_global_cancellation_manager()
        {
            let count = cancellation_manager.cancel_all();
            if count > 0 {
                info!(
                    "🛑 Cancelled {} running agents via cancellation tokens",
                    count
                );
                count
            } else {
                info!("🛑 No running agents to cancel");
                0
            }
        } else {
            warn!("❌ No global cancellation manager available - cannot cancel running agents");
            0
        };

        // Update UI tracking for active agents
        let ui_active_agents = {
            let mut agents = self.active_agents.lock().unwrap();
            let count = agents.len();
            let agent_types: Vec<String> = agents.values().cloned().collect();
            agents.clear(); // Clear all active agents from UI tracking
            (count, agent_types)
        };

        // Show appropriate cancellation messages
        if cancelled_count > 0 || ui_active_agents.0 > 0 {
            let total_cancelled = std::cmp::max(cancelled_count, ui_active_agents.0);
            self.add_message(Message::new_with_agent(
                MessageRole::System,
                format!("🛑 Cancelled {} active agents", total_cancelled),
                "ControlBridge".to_string(),
            ));
        }

        // Reset UI state
        self.scroll_to_bottom = true;

        info!("🛑 All agents cancelled - UI reset complete");
    }

    fn add_message(&mut self, message: Message) {
        debug!(
            "Adding message: role={:?}, agent_source={:?}, content_len={}",
            message.role,
            message.agent_source,
            message.content.len()
        );
        trace!(
            "Message content preview: {}",
            &message.content[..std::cmp::min(100, message.content.len())]
        );

        self.messages.push_back(message);
        // Keep only last 100 messages to prevent memory issues
        if self.messages.len() > 100 {
            let removed = self.messages.pop_front();
            debug!(
                "Removed oldest message due to 100 message limit: {:?}",
                removed.map(|m| m.role)
            );
        }
        // Set flag to scroll to bottom when auto-scroll is enabled
        if self.auto_scroll {
            self.scroll_to_bottom = true;
            self.last_message_time = Some(std::time::Instant::now()); // Track when message was added
            trace!("Auto-scroll enabled, setting scroll_to_bottom flag");
        }

        info!("Message added. Total messages: {}", self.messages.len());
    }

    fn process_user_input(&mut self, input: String, aws_identity: &Arc<Mutex<AwsIdentityCenter>>) {
        info!("🚢 Processing user input: '{}'", input);
        debug!(
            "Input length: {} chars, processing_message: {}",
            input.len(),
            self.processing_message
        );

        if input.trim().is_empty() {
            warn!("Empty input received, ignoring");
            return;
        }

        // Add user message to display
        self.add_message(Message::new_with_agent(
            MessageRole::User,
            input.clone(),
            "User".to_string(),
        ));

        debug!("Processing user input with agent");

        // Process with Control Bridge Agent
        info!("🤖 Starting agent processing for input");
        
        // Log Bridge session start for debugging
        let session_id = format!("bridge-session-{}", chrono::Utc::now().timestamp_millis());
        log_bridge_debug_event(BridgeDebugEvent::BridgeAgentStart {
            timestamp: Utc::now(),
            user_request: input.clone(),
            session_id: session_id.clone(),
        });
        
        self.processing_message = true;
        self.main_agent_active = true; // Track main agent activity
        self.processing_start_time = Some(std::time::Instant::now());

        // No streaming - agent will execute until complete

        debug!("Set processing_message = true, spawning async task");

        // Spawn thread to execute with persistent agent
        info!("🔄 Spawning thread for agent execution with persistent agent");
        let agent = self.agent.clone();
        let sender = self.response_sender.clone();
        let aws_identity_clone = aws_identity.clone();
        let model_changed = self.model_changed;
        let selected_model = self.model_settings.selected_model.clone();

        std::thread::spawn(move || {
            // Get AWS Identity Center credentials and region OUTSIDE the tokio runtime
            let (aws_creds, identity_center_region) = match aws_identity_clone.lock() {
                Ok(mut identity) => match identity.get_default_role_credentials() {
                    Ok(creds) => {
                        let region = identity.identity_center_region.clone();

                        (creds, region)
                    }
                    Err(e) => {
                        let response = AgentResponse::Error(format!(
                            "Failed to get AWS Identity Center credentials: {}",
                            e
                        ));
                        if let Err(e) = sender.send(response) {
                            error!("Failed to send credential error back to UI: {}", e);
                        }
                        return;
                    }
                },
                Err(e) => {
                    let response = AgentResponse::Error(format!(
                        "Failed to access AWS Identity Center: {}",
                        e
                    ));
                    if let Err(e) = sender.send(response) {
                        error!("Failed to send identity center error back to UI: {}", e);
                    }
                    return;
                }
            };

            let result = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime.block_on(async {
                    let mut agent_guard = agent.lock().unwrap();

                    // Create agent on first use or recreate if model changed
                    if agent_guard.is_none() || model_changed {
                        // Create AWS credentials struct
                        let credentials = AwsCredentials {
                            access_key_id: aws_creds.access_key_id.clone(),
                            secret_access_key: aws_creds.secret_access_key.clone(),
                            session_token: aws_creds.session_token.clone(),
                        };

                        // Create the Bridge Agent
                        match BridgeAgent::create(
                            selected_model.clone(),
                            credentials,
                            identity_center_region.clone(),
                            sender.clone(),
                            input.clone(),
                        ).await {
                            Ok(new_agent) => {
                                let action = if model_changed {
                                    "recreated due to model change"
                                } else {
                                    "created"
                                };
                                info!(
                                    "✅ Control Bridge Agent {} successfully (model: {})",
                                    action, selected_model
                                );
                                *agent_guard = Some(new_agent);

                                // Notify UI thread that model was successfully changed
                                if model_changed {
                                    info!("✅ Model successfully changed to: {}", selected_model);
                                    if let Err(e) = sender.send(AgentResponse::ModelChanged {
                                        model_id: selected_model.clone()
                                    }) {
                                        error!("Failed to send model change notification to UI: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("❌ Failed to create Control Bridge Agent: {}", e);
                                return Err(e);
                            }
                        }
                    }

                    // Execute with the agent - let the event loop handle all tool calls naturally
                    if let Some(ref mut agent) = agent_guard.as_mut() {
                        info!("🚀 Executing Bridge agent with natural event loop (no streaming)");
                        
                        // Log the prompt being sent
                        log_bridge_debug_event(BridgeDebugEvent::BridgePromptSent {
                            timestamp: Utc::now(),
                            session_id: session_id.clone(),
                            model_id: selected_model.clone(),
                            system_prompt: "Bridge system prompt".to_string(),
                            user_message: input.clone(),
                        });
                        
                        let _start_time = std::time::Instant::now();
                        match agent.execute(&input).await {
                            Ok(result) => {
                                // Log detailed response information
                                info!("🔍 Bridge Agent Response Details:");
                                info!("  Response length: {} chars", result.response.len());
                                info!("  Response is empty: {}", result.response.trim().is_empty());
                                info!("  First 200 chars: {}", &result.response.chars().take(200).collect::<String>());
                                info!("  Tools used: {}", result.used_tools);
                                info!("  Tools called: {:?}", result.tools_called);
                                info!("  Success: {}", result.success);
                                
                                // Log the response
                                log_bridge_debug_event(BridgeDebugEvent::BridgeResponseReceived {
                                    timestamp: Utc::now(),
                                    session_id: session_id.clone(),
                                    full_response: result.response.clone(),
                                    tool_calls_requested: result.tools_called.clone(),
                                });
                                Ok(result)
                            },
                            Err(e) => {
                                log_bridge_debug_event(BridgeDebugEvent::BridgeToolCall {
                                    timestamp: Utc::now(),
                                    session_id: session_id.clone(),
                                    tool_name: "bridge-execution".to_string(),
                                    input_params: serde_json::json!({}),
                                    success: false,
                                    output_result: None,
                                    error_message: Some(e.to_string()),
                                });
                                Err(format!("Control Bridge Agent execution failed: {}", e))
                            }
                        }
                    } else {
                        Err("Control Bridge Agent not initialized".to_string())
                    }
                }),
                Err(e) => {
                    error!("Failed to create tokio runtime for Control Bridge: {}", e);
                    Err(format!("Failed to create tokio runtime: {}", e))
                }
            };

            // Send result back to UI thread via channel
            let response = match result {
                Ok(agent_result) => AgentResponse::Success(agent_result),
                Err(e) => AgentResponse::Error(e), // e is already a String
            };

            if let Err(e) = sender.send(response) {
                error!("Failed to send agent response back to UI: {}", e);
            }
        });

        // Note: We don't wait for the result here - it will come back via the channel
        // The UI update loop will handle the response when it arrives
        debug!("Agent execution thread spawned, processing_message remains true until response received");
    }

    pub fn show(&mut self, ctx: &egui::Context, aws_identity: &Arc<Mutex<AwsIdentityCenter>>) {
        // Check for agent responses from background threads (non-blocking)
        while let Ok(response) = self.response_receiver.try_recv() {
            self.handle_agent_response(response);
        }

        // Calculate size constraints based on available area (main window)
        let available_rect = ctx.available_rect();

        // Control Bridge window - closable like other windows with stable title
        let window_title = "🚢 Control Bridge";

        let window = Window::new(window_title)
            .movable(true)
            .resizable(true)
            .default_size([500.0, 600.0])
            .min_width(400.0)
            .min_height(300.0)
            .max_height(available_rect.height() * 0.95) // Limit height to 95% of main window
            .collapsible(true);

        let mut is_open = self.open;
        let _result = window.open(&mut is_open).show(ctx, |ui| {
            self.ui_content(ui, aws_identity, ctx);
        });
        self.open = is_open;
    }

    fn handle_agent_response(&mut self, response: AgentResponse) {
        let duration = self
            .processing_start_time
            .map(|start| start.elapsed())
            .unwrap_or_default();

        match response {
            AgentResponse::Success(agent_result) => {
                info!(
                    "✅ Control Bridge Agent response received in {:?}. Response length: {} chars",
                    duration,
                    agent_result.response.len()
                );

                debug!("[DEBUG] Response received in {:?}", duration);
                debug!(
                    "[DEBUG] Response length: {} chars",
                    agent_result.response.len()
                );
                debug!("[DEBUG] Used tools: {}", agent_result.used_tools);
                if agent_result.used_tools {
                    debug!("[DEBUG] Tools called: {:?}", agent_result.tools_called);
                }
                debug!("[DEBUG] Success: {}", agent_result.success);

                // Add agent response to conversation
                if agent_result.response.trim().is_empty() {
                    warn!("❌ [ERROR] Empty response received!");
                    warn!("Response was: '{}'", agent_result.response);
                    warn!("Response length: {}", agent_result.response.len());
                    warn!("Tools used: {}", agent_result.used_tools);
                    warn!("Tools called: {:?}", agent_result.tools_called);
                    warn!("Success: {}", agent_result.success);
                    warn!("Execution cycles: {}", agent_result.execution.cycles);
                    warn!("Model calls: {}", agent_result.execution.model_calls);

                    // If tools were called but no response, create a fallback message
                    let response_text = if agent_result.used_tools && !agent_result.tools_called.is_empty() {
                        let tools_summary = agent_result.tools_called.join(", ");
                        format!(
                            "I've executed the following tools: {}\n\nThe tools completed successfully, but I wasn't able to generate a proper summary. Please check the debug logs for detailed tool outputs.\n\nTip: Try asking your question again with more specific instructions about what information you'd like to see.",
                            tools_summary
                        )
                    } else {
                        format!(
                            "❌ Error: Received empty response from agent\n\nDetails:\n- Response length: {} chars\n- Tools used: {}\n- Tools called: {:?}\n- Success status: {}\n- Execution cycles: {}\n- Model calls: {}",
                            agent_result.response.len(),
                            agent_result.used_tools,
                            agent_result.tools_called,
                            agent_result.success,
                            agent_result.execution.cycles,
                            agent_result.execution.model_calls
                        )
                    };
                    
                    self.add_message(Message::new_with_agent(
                        MessageRole::Assistant,
                        response_text,
                        "ControlBridge".to_string(),
                    ));
                } else {
                    // Add the assistant message to the conversation
                    let mut message = Message::new_with_agent(
                        MessageRole::Assistant,
                        agent_result.response,
                        "ControlBridge".to_string(),
                    );

                    // Add execution details as debug info if debug mode is on
                    if self.debug_mode {
                        let debug_info = format!(
                            "Execution Details:\nCycles: {}\nModel calls: {}\nTool executions: {}\nDuration: {:?}\nUsed tools: {}\nSuccess: {}",
                            agent_result.execution.cycles,
                            agent_result.execution.model_calls,
                            agent_result.execution.tool_executions,
                            agent_result.duration,
                            agent_result.used_tools,
                            agent_result.success
                        );
                        message = message.with_debug(debug_info);
                    }

                    self.add_message(message);
                }

                // Reset processing state
                self.processing_message = false;
                self.main_agent_active = false; // Reset main agent tracking
                self.last_processing_time = Some(duration);
                self.scroll_to_bottom = true;
                debug!("Set processing_message = false, main_agent_active = false, scroll_to_bottom = true");
            }
            AgentResponse::Error(error) => {
                error!(
                    "❌ Control Bridge Agent processing failed after {:?}: {}",
                    duration, error
                );

                // Create a user-friendly error message
                let error_text = if error.contains("ExpiredTokenException") {
                    "⚠️ AWS credentials have expired. Please refresh your AWS credentials and try again.".to_string()
                } else if error.contains("UnknownServiceError") {
                    "⚠️ AWS service error. Please check your AWS configuration and try again."
                        .to_string()
                } else if error.contains("timeout") || error.contains("Timeout") {
                    "⚠️ Request timed out. The model took too long to respond. Please try again."
                        .to_string()
                } else {
                    format!("⚠️ Error processing message: {}", error)
                };

                self.add_message(Message::new_with_agent(
                    MessageRole::System,
                    error_text,
                    "ControlBridge".to_string(),
                ));

                // Reset processing state
                self.processing_message = false;
                self.main_agent_active = false; // Reset main agent tracking
                self.last_processing_time = Some(duration);
                self.scroll_to_bottom = true;
                debug!("Set processing_message = false, main_agent_active = false after error");
            }
            AgentResponse::JsonDebug(json_data) => {
                debug!("📊 Received JSON debug data: {:?}", json_data.json_type);

                // Only add JSON debug messages if JSON debug mode is enabled
                if self.show_json_debug {
                    let message_role = match json_data.json_type {
                        JsonDebugType::Request => MessageRole::JsonRequest,
                        JsonDebugType::Response => MessageRole::JsonResponse,
                    };

                    let header = match json_data.json_type {
                        JsonDebugType::Request => "Model Request JSON",
                        JsonDebugType::Response => "Model Response JSON",
                    };

                    // Create content with both composed and raw JSON (if available)
                    let content = if let Some(ref raw_json) = json_data.raw_json_content {
                        format!(
                            "🔵 **Composed JSON** (Reconstructed):\n```json\n{}\n```\n\n🔴 **Raw JSON** (Provider API):\n```json\n{}\n```",
                            json_data.json_content,
                            raw_json
                        )
                    } else {
                        format!(
                            "🔵 **Composed JSON** (Reconstructed):\n```json\n{}\n```\n\n🔴 **Raw JSON**: Not available",
                            json_data.json_content
                        )
                    };

                    // Create a message with formatted JSON content
                    let mut message =
                        Message::new_with_agent(message_role, content, "JsonCapture".to_string());

                    // Set a custom summary for the header
                    message.summary = Some(header.to_string());

                    self.add_message(message);
                    self.scroll_to_bottom = true;
                }
            }
            // Removed SubAgentEvent - agents handle their own event loops
            // Removed AgentCreated and AgentDestroyed - not needed without streaming
            AgentResponse::ModelChanged { model_id } => {
                info!("✅ Model successfully changed to: {}", model_id);
                self.model_changed = false; // Reset the flag

                // Add a system message to indicate successful model change
                self.add_message(Message::new_with_agent(
                    MessageRole::System,
                    format!(
                        "🤖 Agent successfully updated to use {}",
                        ModelConfig::get_display_name(&self.available_models, &model_id)
                    ),
                    "ControlBridge".to_string(),
                ));
                self.scroll_to_bottom = true;
            }
            
            AgentResponse::ToolCallStart { parent_message } => {
                debug!("🔧 Received tool call start message: {}", parent_message.content);
                
                // Add the parent "Calling tool" message to the conversation
                self.add_message(parent_message);
                self.scroll_to_bottom = true;
            }
            
            AgentResponse::ToolCallComplete { parent_message_id, child_message } => {
                debug!("✅ Received tool call complete message for parent: {}", parent_message_id);
                
                // Find the parent message and add the child to its nested messages
                if let Some(parent_msg) = self.messages.iter_mut().find(|msg| msg.id == parent_message_id) {
                    parent_msg.add_nested_message(child_message);
                    self.scroll_to_bottom = true;
                    debug!("Added child message to parent tool call node");
                } else {
                    warn!("Could not find parent message with ID: {}", parent_message_id);
                    // Fallback: add as standalone message
                    self.add_message(child_message);
                }
            }
        }
    }

    // Removed handle_sub_agent_event - not needed without streaming
    /*fn handle_sub_agent_event(
        &mut self,
        agent_id: String,
        agent_type: String,
        event: SubAgentEvent,
    ) {
        debug!(
            "📊 Received sub-agent event from {} {}: {:?}",
            agent_type, agent_id, event
        );

        // Create user-friendly message for the sub-agent event
        let (icon, content) = match &event {
            SubAgentEvent::ProcessingStarted {
                timestamp,
                task_description,
            } => (
                "🚀",
                format!(
                    "Started processing at {}: {}",
                    timestamp.format("%H:%M:%S"),
                    task_description
                ),
            ),
            SubAgentEvent::ModelRequest {
                timestamp: _,
                messages_count,
                ..
            } => (
                "📤",
                format!(
                    "Requesting analysis from AI model ({} messages)",
                    messages_count
                ),
            ),
            SubAgentEvent::ModelResponse {
                timestamp: _,
                response_length,
                tokens_used,
            } => {
                let token_info = tokens_used
                    .map(|t| format!(" - {} tokens", t))
                    .unwrap_or_default();
                (
                    "📥",
                    format!(
                        "Received AI analysis results ({} chars{})",
                        response_length, token_info
                    ),
                )
            }
            SubAgentEvent::ToolStarted {
                timestamp,
                tool_name,
                ..
            } => {
                let action = Self::get_user_friendly_action(tool_name);
                (
                    "🔧",
                    format!("{} at {}", action, timestamp.format("%H:%M:%S")),
                )
            }
            SubAgentEvent::ToolCompleted {
                timestamp,
                tool_name,
                success,
                output_summary,
            } => {
                let action = Self::get_user_friendly_action(tool_name);
                let icon = if *success { "✅" } else { "❌" };
                let result = if *success { "completed" } else { "failed" };
                let summary = output_summary
                    .as_ref()
                    .map(|s| format!(" - {}", s))
                    .unwrap_or_default();
                (
                    icon,
                    format!(
                        "{} {} at {}{}",
                        action,
                        result,
                        timestamp.format("%H:%M:%S"),
                        summary
                    ),
                )
            }
            SubAgentEvent::TaskComplete { timestamp } => (
                "🏁",
                format!("Task completed at {}", timestamp.format("%H:%M:%S")),
            ),
            SubAgentEvent::Error { timestamp, error } => (
                "❌",
                format!("Error at {}: {}", timestamp.format("%H:%M:%S"), error),
            ),
            SubAgentEvent::JsonDebug(json_data) => {
                // Handle JSON debug data for task agents - add to parent message
                if let Some(parent_id) = self.active_agent_nodes.get(&agent_id).cloned() {
                    if let Some(parent_message) = self.messages.iter_mut().find(|m| m.id == parent_id) {
                        parent_message.json_debug_data.push(json_data.clone());
                        debug!("📊 Added JSON debug data to parent message for task agent {}", agent_id);
                    }
                }
                
                // Return a minimal display message for the tree node
                let icon = match json_data.json_type {
                    JsonDebugType::Request => "📤",
                    JsonDebugType::Response => "📥",
                };
                (
                    icon,
                    format!("JSON debug data captured ({})", 
                        match json_data.json_type {
                            JsonDebugType::Request => "request",
                            JsonDebugType::Response => "response",
                        }
                    ),
                )
            },
        };

        // Find the parent message for this sub-agent
        let parent_message_id = self.active_agent_nodes.get(&agent_id).cloned();

        if let Some(parent_id) = parent_message_id {
            // Find the parent message and add this as a nested message
            for message in &mut self.messages {
                if message.id == parent_id {
                    // Create child message with CLOSED default state
                    let child_message = Message::new_with_agent(
                        MessageRole::System,
                        format!("{} {}", icon, content),
                        format!("SubAgent-{}", agent_id),
                    );

                    // Set child messages to CLOSED by default
                    self.message_expand_states
                        .insert(child_message.id.clone(), false);

                    message.add_nested_message(child_message);
                    break;
                }
            }
        } else {
            warn!("No parent node found for sub-agent {}", agent_id);
        }

        // Trigger scroll to bottom for activity updates
        self.scroll_to_bottom = true;
    }

    */

    // Removed handle_agent_created - not needed without streaming
    /*fn handle_agent_created(&mut self, agent_id: String, agent_type: String) {
        info!("🚀 Specialized task started: {} ({})", agent_type, agent_id);

        // Track the agent in our active agents map
        {
            let mut agents = self.active_agents.lock().unwrap();
            agents.insert(agent_id.clone(), agent_type.clone());
        }

        // Create user-friendly parent message with task-focused language
        let task_message = Self::get_task_description(&agent_type, "processing your request");
        let parent_message = Message::new_with_agent(
            MessageRole::System,
            task_message,
            "ControlBridge".to_string(),
        );

        // Set parent message to OPEN by default
        self.message_expand_states
            .insert(parent_message.id.clone(), true);

        // Store the parent message ID for this agent
        self.active_agent_nodes
            .insert(agent_id.clone(), parent_message.id.clone());

        self.add_message(parent_message);
        self.scroll_to_bottom = true;
    }

    */

    // Removed handle_agent_destroyed - not needed without streaming
    /*fn handle_agent_destroyed(&mut self, agent_id: String, agent_type: String) {
        info!(
            "🏁 Specialized task completed: {} ({})",
            agent_type, agent_id
        );

        // Remove the agent from our active agents map
        {
            let mut agents = self.active_agents.lock().unwrap();
            agents.remove(&agent_id);
        }

        // Update the parent message to show completion
        if let Some(parent_message_id) = self.active_agent_nodes.get(&agent_id) {
            for message in &mut self.messages {
                if message.id == *parent_message_id {
                    // Update parent message to show completion
                    let completion_message = match agent_type.as_str() {
                        "aws-log-analyzer" => "🔍 ✅ CloudWatch log analysis complete",
                        "aws-resource-auditor" => "📊 ✅ AWS resource audit complete",
                        "aws-security-scanner" => "🔒 ✅ Security scan complete",
                        _ => "⚙️ ✅ Request processing complete",
                    };
                    message.content = completion_message.to_string();
                    message.summary = Some(completion_message.to_string());
                    break;
                }
            }
        }

        // Clean up parent-child relationship tracking
        self.active_agent_nodes.remove(&agent_id);

        self.scroll_to_bottom = true;
    }*/

    /// Complete reset when changing models - provides clean slate for new agent
    fn reset_for_model_change(&mut self, new_model_name: &str) {
        info!("🧹 Performing complete reset for model change to: {}", new_model_name);

        // Cancel any active processing and clear agents
        self.cancel_all_agents();

        // Clear conversation history for clean slate
        self.messages.clear();
        
        // Reset input state
        self.input_text.clear();
        
        // Reset processing state
        self.processing_message = false;
        self.main_agent_active = false;
        
        // Clear agent tracking
        {
            let mut agents = self.active_agents.lock().unwrap();
            agents.clear(); // Clear all active agents from UI tracking
        }
        self.active_agent_nodes.clear();
        
        // Reset message expand states
        self.message_expand_states.clear();
        
        // Reset timing information
        self.last_message_time = None;
        self.processing_start_time = None;
        
        // Ensure we scroll to show any new content
        self.scroll_to_bottom = true;
        
        // Add welcome message for new model
        self.add_message(Message::new_with_agent(
            MessageRole::System,
            format!(
                "🤖 Switched to {} - Starting fresh conversation. How can I help you with AWS infrastructure management?",
                new_model_name
            ),
            "ControlBridge".to_string(),
        ));
        
        info!("✅ Complete reset finished for model: {}", new_model_name);
    }

    fn ui_content(
        &mut self,
        ui: &mut egui::Ui,
        aws_identity: &Arc<Mutex<AwsIdentityCenter>>,
        ctx: &egui::Context,
    ) {
        // Update dark mode based on current theme
        self.dark_mode = ui.visuals().dark_mode;

        ui.vertical(|ui| {
            // Header with options
            ui.horizontal(|ui| {
                ui.label("Options:");
                ui.checkbox(&mut self.show_json_debug, "JSON Debug");
                ui.checkbox(&mut self.auto_scroll, "Auto Scroll");

                if ui.button("📤 Expand All").clicked() {
                    // Set all visible messages to expanded in persistent state
                    for message in &self.messages {
                        let should_include = match message.role {
                            MessageRole::JsonRequest | MessageRole::JsonResponse => {
                                self.show_json_debug
                            }
                            _ => true,
                        };
                        if should_include {
                            self.message_expand_states.insert(message.id.clone(), true);
                        }
                    }
                    self.force_expand_all = true;
                }
                if ui.button("📥 Collapse All").clicked() {
                    // Set all visible messages to collapsed in persistent state
                    for message in &self.messages {
                        let should_include = match message.role {
                            MessageRole::JsonRequest | MessageRole::JsonResponse => {
                                self.show_json_debug
                            }
                            _ => true,
                        };
                        if should_include {
                            self.message_expand_states.insert(message.id.clone(), false);
                        }
                    }
                    self.force_collapse_all = true;
                }
                if ui.button("⬇️ Scroll to Bottom").clicked() {
                    self.scroll_to_bottom = true;
                }
            });

            //ui.separator();

            // Message display area - calculate available space like other windows
            let current_window_height = ui.available_height();
            let input_area_height = 120.0; // Reserve space for input area, buttons, and header
            let max_scroll_height = (current_window_height - input_area_height).min(600.0); // Reasonable max, similar to chat window

            let scroll_area = ScrollArea::vertical()
                .id_salt("control_bridge_scroll")
                .auto_shrink([false, false]) // Don't auto-shrink - let user control window size
                .max_height(max_scroll_height) // Prevent expansion beyond available space
                .stick_to_bottom(self.scroll_to_bottom);

            let _scroll_response = scroll_area.show(ui, |ui| {
                // Set a fixed width to prevent content from expanding the window
                let available_width = ui.available_width();
                ui.set_max_width(available_width);
                if self.messages.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("No messages yet. Start typing below!");
                    });
                    return;
                }

                // Collect messages to render to avoid borrow checker issues
                let messages_to_render: Vec<(usize, Message, bool)> = self
                    .messages
                    .iter()
                    .enumerate()
                    .filter_map(|(i, message)| {
                        let should_render = match message.role {
                            MessageRole::JsonRequest | MessageRole::JsonResponse => {
                                self.show_json_debug
                            }
                            _ => true,
                        };
                        if should_render {
                            Some((i, message.clone(), i < self.messages.len() - 1))
                        } else {
                            None
                        }
                    })
                    .collect();

                // Render the collected messages
                for (i, message, add_space) in messages_to_render {
                    self.render_message(ui, &message, i);
                    if add_space {
                        ui.add_space(8.0);
                    }
                }

                // Show simple processing spinner since we don't have streaming
                if self.processing_message {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("🚢 Processing...").color(Color32::GRAY));
                    });
                }
            });

            // Reset expand/collapse flags after they've been applied to all messages
            if self.force_expand_all {
                self.force_expand_all = false;
            }
            if self.force_collapse_all {
                self.force_collapse_all = false;
            }

            // Reset the scroll flag - don't continuously request repaints
            if self.scroll_to_bottom {
                // Always reset the flag to prevent continuous layout updates
                self.scroll_to_bottom = false;
            }

            ui.add_space(10.0); // Empty space instead of separator

            // Model selection dropdown
            let mut model_selection_changed = None;
            ui.horizontal(|ui| {
                ui.label("🤖 Model:");

                let current_model = self
                    .model_settings
                    .get_selected_model(&self.available_models);
                let current_display_name = current_model
                    .map(|m| m.display_name.clone())
                    .unwrap_or_else(|| "Unknown Model".to_string());

                let current_selected_model = self.model_settings.selected_model.clone();
                let available_models = self.available_models.clone();

                egui::ComboBox::from_label("")
                    .selected_text(&current_display_name)
                    .show_ui(ui, |ui| {
                        for model in &available_models {
                            let is_selected = current_selected_model == model.model_id;
                            if ui
                                .selectable_value(
                                    &mut self.model_settings.selected_model,
                                    model.model_id.clone(),
                                    &model.display_name,
                                )
                                .clicked() && !is_selected {
                                model_selection_changed =
                                    Some((model.display_name.clone(), model.model_id.clone()));
                            }
                        }
                    });

            });

            // Handle model change outside the UI closure to avoid borrowing conflicts
            if let Some((display_name, model_id)) = model_selection_changed {
                self.model_changed = true;
                info!("🔄 Model changed to: {} ({})", display_name, model_id);

                // Update global model configuration
                set_global_model(model_id.clone());

                // Perform complete reset for clean agent experience
                self.reset_for_model_change(&display_name);
            }

            ui.add_space(5.0); // Small space after model selection

            // Input area - vertical layout with input box on top, buttons below
            ui.vertical(|ui| {
                // Input text box
                let text_edit = TextEdit::multiline(&mut self.input_text)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text(
                        "Type your message here...\nPress Enter to send, Shift+Enter for new line",
                    );

                let response = ui.add(text_edit);

                // Buttons below input box
                ui.horizontal(|ui| {
                    // Send button
                    let send_enabled =
                        !self.processing_message && !self.input_text.trim().is_empty();
                    ui.add_enabled_ui(send_enabled, |ui| {
                        if ui.button("Send").clicked() {
                            let input = std::mem::take(&mut self.input_text);
                            self.process_user_input(input, aws_identity);
                        }
                    });

                    // Stop button - check both UI tracking and actual running agents
                    let ui_active_count = self.active_agents.lock().unwrap().len();
                    let actual_active_count = get_global_cancellation_manager()
                        .map(|manager| manager.active_count())
                        .unwrap_or(0);
                    let has_active_work =
                        self.processing_message || ui_active_count > 0 || actual_active_count > 0;

                    if has_active_work {
                        if ui.button("🛑 Stop").clicked() {
                            self.cancel_all_agents();
                        }
                    } else {
                        ui.add_enabled_ui(false, |ui| {
                            let _ = ui.button("🛑 Stop");
                        });
                    }

                    // Clear button - recreates agent as if model changed
                    if ui.button("🗑 Clear").clicked() {
                        let current_model = self
                            .model_settings
                            .get_selected_model(&self.available_models);
                        let current_display_name = current_model
                            .map(|m| m.display_name.clone())
                            .unwrap_or_else(|| "Unknown Model".to_string());
                        
                        // Trigger the same reset logic as model change
                        self.reset_for_model_change(&current_display_name);
                        
                        // Set model_changed flag to force agent recreation on next use
                        self.model_changed = true;
                    }
                });

                // Handle Enter to send, Shift+Enter for new line
                if response.has_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
                    // Enter without shift: send message
                    let input = std::mem::take(&mut self.input_text);
                    self.process_user_input(input, aws_identity);
                }
                // Shift+Enter is handled automatically by TextEdit::multiline for new lines
            });
        });

        // Handle delayed auto-scroll after new messages (reduced delay for streaming)
        if let Some(last_msg_time) = self.last_message_time {
            // Fixed delay for auto-scroll since we don't have streaming
            let delay = std::time::Duration::from_millis(500);

            if last_msg_time.elapsed() >= delay {
                if self.auto_scroll && !self.scroll_to_bottom {
                    self.scroll_to_bottom = true;
                    // Auto-scroll triggered after delay (removed spam trace log)
                }

                // Clear the timer since we don't have streaming
                self.last_message_time = None;
            } else {
                // Normal repaint rate since we don't have streaming
                let repaint_delay = std::time::Duration::from_millis(10);
                ctx.request_repaint_after(repaint_delay);
            }
        }
    }

    fn render_message(&mut self, ui: &mut egui::Ui, message: &Message, index: usize) {
        // Clean Zen-like header: just icon and summary
        let header_text = format!(
            "{} {}",
            message.role.icon(),
            message.summary.as_ref().unwrap_or(&"Message".to_string())
        );

        let mut header = CollapsingHeader::new(
            RichText::new(header_text).color(message.role.color(self.dark_mode)),
        )
        .id_salt(&message.id);

        // Determine open state based on force flags, persistent state, or defaults
        if self.force_expand_all {
            header = header.open(Some(true));
            self.message_expand_states.insert(message.id.clone(), true);
        } else if self.force_collapse_all {
            header = header.open(Some(false));
            self.message_expand_states.insert(message.id.clone(), false);
        } else if let Some(&is_open) = self.message_expand_states.get(&message.id) {
            // Use persistent state if available
            header = header.open(Some(is_open));
        } else {
            // Assistant messages start open by default, JSON messages always start closed
            let default_open = match message.role {
                MessageRole::Assistant => true,
                MessageRole::JsonRequest | MessageRole::JsonResponse => false, // JSON always closed by default
                _ => false,
            };
            header = header.default_open(default_open);
            self.message_expand_states
                .insert(message.id.clone(), default_open);
        }

        let header_response = header.show(ui, |ui| {
            // JSON messages get special formatting
            let is_json_message = matches!(
                message.role,
                MessageRole::JsonRequest | MessageRole::JsonResponse
            );

            if is_json_message {
                // JSON content with syntax highlighting via color
                ui.style_mut().visuals.override_text_color =
                    Some(message.role.color(self.dark_mode));
            }

            // Message content - split by lines for better display
            for line in message.content.lines() {
                if line.trim().is_empty() {
                    ui.add_space(6.0); // Zen whitespace instead of separator
                } else {
                    ui.monospace(line);
                }
            }

            // Reset text color override
            if is_json_message {
                ui.style_mut().visuals.override_text_color = None;
            }

            // Debug information if available and debug mode is on
            if self.debug_mode && message.debug_info.is_some() {
                ui.add_space(8.0); // Zen whitespace instead of separator
                ui.label("Debug Information:");
                if let Some(debug_info) = &message.debug_info {
                    ui.monospace(debug_info);
                }
            }

            // Nested messages (tool results) - render without header
            if !message.nested_messages.is_empty() {
                ui.add_space(8.0); // Zen whitespace instead of separator
                for (j, nested_msg) in message.nested_messages.iter().enumerate() {
                    ui.indent(format!("nested_{}_{}", message.id, j), |ui| {
                        self.render_message(ui, nested_msg, index * 1000 + j);
                    });
                }
            }

            // JSON debug data from task agents (only show if JSON debug mode is enabled)
            if self.show_json_debug && !message.json_debug_data.is_empty() {
                ui.add_space(8.0); // Zen whitespace instead of separator
                ui.label(format!(
                    "📊 JSON Debug Data ({})",
                    message.json_debug_data.len()
                ));
                for (j, json_data) in message.json_debug_data.iter().enumerate() {
                    let json_type_str = match json_data.json_type {
                        JsonDebugType::Request => "Model Request",
                        JsonDebugType::Response => "Model Response",
                    };
                    let json_icon = match json_data.json_type {
                        JsonDebugType::Request => "📤",
                        JsonDebugType::Response => "📥",
                    };
                    
                    ui.indent(format!("json_debug_{}_{}", message.id, j), |ui| {
                        CollapsingHeader::new(format!("{} {} JSON", json_icon, json_type_str))
                            .id_salt(format!("json_{}_{}", message.id, j))
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.style_mut().visuals.override_text_color =
                                    Some(if self.dark_mode { Color32::from_rgb(144, 238, 144) } else { Color32::from_rgb(34, 139, 34) });
                                
                                // Display the JSON content with proper formatting
                                ui.monospace(&json_data.json_content);
                                
                                // Show raw JSON if available
                                if let Some(ref raw_json) = json_data.raw_json_content {
                                    ui.add_space(6.0);
                                    ui.label("🔴 Raw JSON (Provider API):");
                                    ui.monospace(raw_json);
                                }
                                
                                // Reset text color override
                                ui.style_mut().visuals.override_text_color = None;
                                
                                ui.add_space(4.0);
                                ui.label(format!("Captured: {}", json_data.timestamp.format("%H:%M:%S")));
                            });
                    });
                }
            }
        });

        // Capture user interactions with individual message expand/collapse
        if header_response.header_response.clicked() {
            // User manually clicked the header - toggle the state
            let current_state = self
                .message_expand_states
                .get(&message.id)
                .copied()
                .unwrap_or(false);
            self.message_expand_states
                .insert(message.id.clone(), !current_state);
            debug!(
                "User manually toggled message {}: {} -> {}",
                message.id, current_state, !current_state
            );
        }
    }

    // Removed render_streaming_message - no longer needed after removing streaming

    // Removed streaming functionality - using blocking execution
    // Agents now execute completely before returning response
}

impl FocusableWindow for ControlBridgeWindow {
    type ShowParams = IdentityShowParams;

    fn window_id(&self) -> &'static str {
        "control_bridge"
    }

    fn window_title(&self) -> String {
        "🚢 Control Bridge".to_string()
    }

    fn is_open(&self) -> bool {
        self.open // Control Bridge can be opened and closed
    }

    fn show_with_focus(
        &mut self,
        ctx: &egui::Context,
        params: Self::ShowParams,
        _bring_to_front: bool,
    ) {
        if let Some(aws_identity) = &params.aws_identity {
            self.show(ctx, aws_identity);
        }
    }
}

