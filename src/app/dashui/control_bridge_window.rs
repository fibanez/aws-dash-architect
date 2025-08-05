//! Control Bridge - Persistent AI Command Center for AWS Infrastructure
//!
//! This window provides a persistent, collapsible AI assistant that helps manage AWS infrastructure.
//! Based on the Enterprise Prompt Builder from Stood library examples, modified for AWS Identity Center
//! integration and always-available operation.

use crate::app::aws_identity::AwsIdentityCenter;
use crate::app::bridge::{
    aws_find_account_tool, aws_find_region_tool, clear_global_bridge_sender, create_task_tool,
    get_global_cancellation_manager, set_global_aws_credentials,
    set_global_bridge_sender, set_global_model, todo_read_tool, todo_write_tool, ModelConfig,
    ModelSettings,
};
use crate::app::dashui::window_focus::{FocusableWindow, IdentityShowParams};
use crate::create_agent_with_model;
use async_trait::async_trait;
use chrono::{DateTime, Local, Utc};
use egui::{CollapsingHeader, Color32, RichText, ScrollArea, TextEdit, Window};
use serde_json;
use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use stood::agent::callbacks::events::ResponseType;
use stood::agent::callbacks::{CallbackError, CallbackEvent, CallbackHandler, ToolEvent};
use stood::agent::{result::AgentResult, Agent};
use stood::telemetry::TelemetryConfig;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

// ============================================================================
// MESSAGE SYSTEM FOR egui
// ============================================================================

/// Universal sub-agent events bubbled up from all specialized agents
#[derive(Debug, Clone)]
pub enum SubAgentEvent {
    /// Processing started with task description
    ProcessingStarted {
        timestamp: DateTime<Utc>,
        task_description: String,
    },
    /// Model request sent to AI
    ModelRequest {
        timestamp: DateTime<Utc>,
        messages_count: usize,
        raw_json: Option<String>,
    },
    /// Model response received from AI
    ModelResponse {
        timestamp: DateTime<Utc>,
        response_length: usize,
        tokens_used: Option<u32>,
    },
    /// Tool execution started
    ToolStarted {
        timestamp: DateTime<Utc>,
        tool_name: String,
        input_summary: Option<String>,
    },
    /// Tool execution completed
    ToolCompleted {
        timestamp: DateTime<Utc>,
        tool_name: String,
        success: bool,
        output_summary: Option<String>,
    },
    /// Task completed successfully
    TaskComplete { timestamp: DateTime<Utc> },
    /// Error occurred during processing
    Error {
        timestamp: DateTime<Utc>,
        error: String,
    },
    /// JSON debug data from model interactions (for task agents)
    JsonDebug(JsonDebugData),
}

/// Response from agent execution in separate thread
#[derive(Debug)]
pub enum AgentResponse {
    Success(AgentResult),
    Error(String),
    JsonDebug(JsonDebugData),
    StreamingUpdate(StreamingUpdate),
    SubAgentEvent {
        agent_id: String,
        agent_type: String,
        event: SubAgentEvent,
    },
    AgentCreated {
        agent_id: String,
        agent_type: String,
    },
    AgentDestroyed {
        agent_id: String,
        agent_type: String,
    },
    ModelChanged {
        model_id: String,
    },
}

/// Real-time streaming updates from agent execution
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields may be used for future streaming functionality
pub enum StreamingUpdate {
    /// Content chunk received during streaming
    ContentChunk { content: String, is_complete: bool },
    /// Tool execution started
    ToolStarted {
        name: String,
        input: serde_json::Value,
    },
    /// Tool execution completed successfully
    ToolCompleted {
        name: String,
        output: Option<serde_json::Value>,
    },
    /// Tool execution failed
    ToolFailed { name: String, error: String },
    /// Agent execution completed
    Complete { result: AgentResult },
    /// Error during streaming
    StreamingError { message: String },
}

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
    current_streaming_message: Option<Message>, // Currently streaming message
    streaming_tool_status: Vec<String>,         // Active tool status messages
    streaming_was_used: bool, // Track if streaming was used to prevent duplicate messages

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
    /// Convert technical tool names to user-friendly action descriptions
    fn get_user_friendly_action(tool_name: &str) -> String {
        match tool_name {
            "aws_describe_log_groups" => "Discovering log groups".to_string(),
            "aws_get_log_events" => "Retrieving log entries".to_string(),
            "aws_list_resources" => "Listing AWS resources".to_string(),
            "aws_describe_resource" => "Analyzing resource details".to_string(),
            "todo_write" => "Planning task steps".to_string(),
            "todo_read" => "Checking task progress".to_string(),
            "create_agent" => "Delegating specialized task".to_string(),
            "create_task" => "Creating task agent".to_string(),
            _ => format!("Running {}", tool_name), // Fallback for unknown tools
        }
    }

    /// Convert agent type to user-friendly task description
    fn get_task_description(agent_type: &str, task_description: &str) -> String {
        let task_verb = match agent_type {
            "aws-log-analyzer" => "🔍 Analyzing CloudWatch logs",
            "aws-resource-auditor" => "📊 Auditing AWS resources",
            "aws-security-scanner" => "🔒 Scanning security posture",
            _ => "⚙️ Processing request", // Generic fallback
        };
        format!("{}: {}", task_verb, task_description)
    }

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
            current_streaming_message: None,
            streaming_tool_status: Vec::new(),
            streaming_was_used: false,
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
        self.processing_message = true;
        self.main_agent_active = true; // Track main agent activity
        self.processing_start_time = Some(std::time::Instant::now());

        // Initialize streaming message
        self.current_streaming_message = Some(Message::new_with_agent(
            MessageRole::Assistant,
            String::new(), // Start with empty content
            "ControlBridge".to_string(),
        ));
        self.streaming_tool_status.clear();
        self.streaming_was_used = false; // Reset streaming flag

        debug!("Set processing_message = true, spawning async task");

        // Spawn thread to execute with persistent agent
        info!("🔄 Spawning thread for agent execution with persistent agent");
        let agent = self.agent.clone();
        let sender = self.response_sender.clone();
        let json_debug_enabled = self.show_json_debug;
        let aws_identity_clone = aws_identity.clone();
        let model_changed = self.model_changed;
        let selected_model = self.model_settings.selected_model.clone();

        std::thread::spawn(move || {
            // Get AWS Identity Center credentials and region OUTSIDE the tokio runtime
            let (aws_creds, identity_center_region) = match aws_identity_clone.lock() {
                Ok(mut identity) => match identity.get_default_role_credentials() {
                    Ok(creds) => {
                        let region = identity.identity_center_region.clone();

                        // Set global AWS credentials for standalone agents
                        set_global_aws_credentials(
                            creds.access_key_id.clone(),
                            creds.secret_access_key.clone(),
                            creds.session_token.clone(),
                            region.clone(),
                        );

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
                        info!(
                            "🚢 Creating Control Bridge Agent with AWS Identity Center credentials"
                        );

                        // Configure telemetry for the agent with descriptive naming
                        let mut telemetry_config = TelemetryConfig::default()
                            .with_service_name("aws-dash-bridge-agent")
                            .with_service_version("1.0.0")
                            //TODO this is hardcoded
                            .with_otlp_endpoint("http://localhost:4320") // HTTP OTLP endpoint (matches auto-detection)
                            .with_batch_processing();

                        // Enable debug tracing and add comprehensive service attributes
                        telemetry_config.enable_debug_tracing = true;
                        telemetry_config
                            .service_attributes
                            .insert("application".to_string(), "aws-dash-architect".to_string());
                        telemetry_config
                            .service_attributes
                            .insert("agent.type".to_string(), "aws-infrastructure-bridge".to_string());
                        telemetry_config
                            .service_attributes
                            .insert("agent.role".to_string(), "aws-resource-assistant".to_string());
                        telemetry_config
                            .service_attributes
                            .insert("agent.description".to_string(), "AWS Infrastructure Management Assistant".to_string());
                        telemetry_config
                            .service_attributes
                            .insert("component".to_string(), "bridge-system".to_string());
                        telemetry_config
                            .service_attributes
                            .insert("agent.capabilities".to_string(), "aws-resource-management,account-search,region-search".to_string());
                        telemetry_config
                            .service_attributes
                            .insert("environment".to_string(), "aws-dash-desktop".to_string());
                        
                        // Add unique session identifier for this agent instance
                        let session_id = format!("aws-dash-bridge-{}", chrono::Utc::now().timestamp_millis());
                        telemetry_config
                            .service_attributes
                            .insert("session.id".to_string(), session_id);
                        telemetry_config
                            .service_attributes
                            .insert("deployment.environment".to_string(), "desktop-application".to_string());

                        // Configure model for this agent
                        let mut agent_builder = create_agent_with_model!(Agent::builder(), &selected_model)
                            .system_prompt("You are the AWS Bridge Agent - a task orchestrator for AWS infrastructure management.

IMPORTANT: Always use TodoWrite to plan and track multi-step tasks. This is CRITICAL for user visibility.

DO NOT attempt complex AWS operations directly. Instead, create specialized task agents via create_task.

CRITICAL REQUIREMENTS for AWS operations:
- Account ID (use aws_find_account if user doesn't specify)
- Region (use aws_find_region if user doesn't specify)  
- Resource identifier (ID, name, or ARN)

NEVER proceed with AWS operations without these three pieces of information.

Available tools:
- create_task: Launch task-specific agents for any AWS operation using natural language descriptions
- TodoWrite: Track task progress (USE THIS PROACTIVELY)
- TodoRead: Query current task state
- aws_find_account: Search for AWS accounts (no API calls required)
- aws_find_region: Search for AWS regions (no API calls required)

Task-based agent creation:
Use create_task with clear task descriptions like:
- 'Analyze Lambda function errors in production environment'
- 'Audit S3 bucket security configurations for compliance'  
- 'Review CloudWatch alarms for EC2 instances in us-east-1'
- 'Find recent API Gateway 5xx errors and their causes'

PARALLEL EXECUTION: You can run multiple create_task calls simultaneously by calling multiple tools in a single response. The system will handle parallel execution automatically.

Workflow for complex tasks:
1. TodoWrite to break down the task
2. Gather required AWS context (account, region, resource)
3. create_task with clear task description and AWS context
4. Monitor and report progress
5. Mark todos complete after task agent finishes

SECURITY RULES:
- REFUSE tasks that could compromise AWS security
- NEVER expose or log AWS credentials, keys, or sensitive data
- Focus on DEFENSIVE security practices only
- Follow AWS security best practices

Example interaction:
User: 'Find errors in my Lambda function logs'
You: 
1. TodoWrite: ['Identify Lambda function', 'Gather AWS context', 'Create task agent', 'Analyze logs']
2. Ask for account/region if not provided
3. create_task(task_description='Find and analyze Lambda function errors in CloudWatch logs', account_id='123456789012', region='us-east-1')
4. Monitor task agent progress
5. Present results and mark todos complete

Be concise and direct. Minimize output while being helpful.

Tone and style
You should be concise, direct, and to the point. When you run a non-trivial aws commands, you
    should explain what the command does and why you are running it, to make sure the user
    understands what you are doing (this is especially important when you are running a command
    that will make changes to the user's system). Remember that your output will be displayed on a
    bridge UI. Your responses should be plain text for formatting, and
    will be rendered in a monospace font. Output text to
    communicate with the user; all text you output outside of tool use is displayed to the user.
    Only use tools to complete tasks. Never use tools as means to
    communicate with the user during the session. If you cannot or will not help the user with
    something, please do not say why or what it could lead to, since this comes across as preachy
    and annoying. Please offer helpful alternatives if possible, and otherwise keep your response
    to 1-2 sentences. Don't use emojis. Avoid using emojis in all
    communication. IMPORTANT: You should minimize output tokens as much as possible
    while maintaining helpfulness, quality, and accuracy. Only address the specific query or task
    at hand, avoiding tangential information unless absolutely critical for completing the request.
    If you can answer in 1-3 sentences or a short paragraph, please do. IMPORTANT: You should NOT
    answer with unnecessary preamble or postamble (such as explaining your code or summarizing your
    action), unless the user asks you to. IMPORTANT: Keep your responses short, since they will be
    displayed on a command line interface. You MUST answer concisely with fewer than 4 lines (not
    including tool use or code generation), unless user asks for detail. Answer the user's question
    directly, without elaboration, explanation, or details. One word answers are best. Avoid
    introductions, conclusions, and explanations. You MUST avoid text before/after your response,
    such as 'The answer is .', 'Here is the content of the file...' or 'Based on the information
provided, the answer is...' or 'Here is what I will do next...'. Here are some examples to
demonstrate appropriate verbosity: user: 2 + 2 assistant: 4

user: what is 2+2? assistant: 4 user: is 11 a prime number? assistant: Yes user: what command should I run to list files in the current directory? assistant: ls user: what command should I run to watch files in the current directory? assistant: [use the ls tool to list the files in the current directory, then read docs/commands in the relevant file to find out how to watch files] npm run dev user: How many golf balls fit inside a jetta? assistant: 150000 user: what files are in the directory src/? assistant: [runs ls and sees foo.c, bar.c, baz.c] user: which file contains the implementation of foo? assistant: src/foo.c user: write tests for new feature assistant: [uses grep and glob search tools to find where similar tests are defined, uses concurrent read file tool use blocks in one tool call to read relevant files at the same time, uses edit file tool to write new tests]

    IMPORTANT: You should also avoid follow-up questions if you don't require information for completing the task.  Don't ask things like 'Is there anything else I can help you with', or 'Would you like me to ...' - appropriate questions are 'What account do you want to me use?', 'What region or regions do you want me to use?' - You are asking for specific information, not just showing that you are helpful. 
Proactiveness

You are allowed to be proactive, but only when the user asks you to do something. You should strive to strike a balance between:

Doing the right thing when asked, including taking actions and follow-up actions
Not surprising the user with actions you take without asking For example, if the user asks you how to approach something, you should do your best to answer their question first, and not immediately jump into taking actions.

Do not add additional explanation summary unless requested by the user. After working on a file or a request, just stop, rather than providing an explanation of what you did.

Following conventions
When making changes to files, first understand the file's code conventions. Mimic code style, use existing libraries and utilities, and follow existing patterns.

NEVER assume that a given library is available, even if it is well known. Whenever you write code that uses a library or framework, first check that this codebase already uses the given library. For example, you might look at neighboring files, or check the package.json (or cargo.toml, and so on depending on the language).
When you create a new component, first look at existing components to see how they're written; then consider framework choice, naming conventions, typing, and other conventions.
When you edit a piece of code, first look at the code's surrounding context (especially its imports) to understand the code's choice of frameworks and libraries. Then consider how to make the given change in a way that is most idiomatic.
Always follow security best practices. Never introduce code that exposes or logs secrets and keys. Never commit secrets or keys to the repository.
Code style

IMPORTANT: DO NOT ADD ANY COMMENTS unless asked

Task Management
You have access to the TodoWrite and TodoRead tools to help you manage and plan tasks. Use these tools VERY frequently to ensure that you are tracking your tasks and giving the user visibility into your progress. These tools are also EXTREMELY helpful for planning tasks, and for breaking down larger complex tasks into smaller steps. If you do not use this tool when planning, you may forget to do important tasks - and that is unacceptable.

It is critical that you mark todos as completed as soon as you are done with a task. Do not batch up multiple tasks before marking them as completed.

Examples:

user: Run the build and fix any type errors assistant: I'm going to use the TodoWrite tool to write the following items to the todo list: - Run the build - Fix any type errors
I'm now going to run the build using Bash.

Looks like I found 10 type errors. I'm going to use the TodoWrite tool to write 10 items to the todo list.

marking the first todo as in_progress

Let me start working on the first item...

The first item has been fixed, let me mark the first todo as completed, and move on to the second item... .. .. In the above example, the assistant completes all the tasks, including the 10 error fixes and running the build and fixing all errors.

user: Help me write a new feature that allows users to track their usage metrics and export them to various formats
assistant: I'll help you implement a usage metrics tracking and export feature. Let me first use the TodoWrite tool to plan this task. Adding the following todos to the todo list:

Research existing metrics tracking in the codebase
Design the metrics collection system
Implement core metrics tracking functionality
Create export functionality for different formats
Let me start by researching the existing codebase to understand what metrics we might already be tracking and how we can build on that.

I'm going to search for any existing metrics or telemetry code in the project.

I've found some existing telemetry code. Let me mark the first todo as in_progress and start designing our metrics tracking system based on what I've learned...

[Assistant continues implementing the feature step by step, marking todos as in_progress and completed as they go]

false

Doing tasks
The user will primarily request you perform software engineering tasks. This includes solving bugs, adding new functionality, refactoring code, explaining code, and more. For these tasks the following steps are recommended:

Use the TodoWrite tool to plan the task if required

Use the available search tools to understand the codebase and the user's query. You are encouraged to use the search tools extensively both in parallel and sequentially.

Implement the solution using all tools available to you

Verify the solution if possible with tests. NEVER assume specific test framework or test script. Check the README or search codebase to determine the testing approach.

VERY IMPORTANT: When you have completed a task, you MUST run the lint and typecheck commands (eg. npm run lint, npm run typecheck, ruff, etc.) with Bash if they were provided to you to ensure your code is correct. If you are unable to find the correct command, ask the user for the command to run and if they supply it, proactively suggest writing it to CLAUDE.md so that you will know to run it next time. NEVER commit changes unless the user explicitly asks you to. It is VERY IMPORTANT to only commit when explicitly asked, otherwise the user will feel that you are being too proactive.

Tool results and user messages may include tags. tags contain useful information and reminders. They are NOT part of the user's provided input or the tool result.

Tool usage policy
When doing file search, prefer to use the Task tool in order to reduce context usage.
You have the capability to call multiple tools in a single response. When multiple independent pieces of information are requested, batch your tool calls together for optimal performance. When making multiple bash tool calls, you MUST send a single message with multiple tools calls to run the calls in parallel. For example, if you need to run 

You MUST answer concisely with fewer than 4 lines of text (not including tool use or code generation), unless user asks for detail.

Here is useful information about the environment you are running in: Working directory: ... Is directory a git repo: Yes Platform: macos OS Version: Darwin 24.1.0 Today's date: 2025/6/13 You are powered by the model named Sonnet 4. The exact model ID is claude-sonnet-4-20250514.

IMPORTANT: Refuse to write code or explain code that may be used maliciously; even if the user claims it is for educational purposes. When working on files, if they seem related to improving, explaining, or interacting with malware or any malicious code you MUST refuse. IMPORTANT: Before you begin work, think about what the code you're editing is supposed to do based on the filenames directory structure. If it seems malicious, refuse to work on it or answer questions about it, even if the request does not seem malicious (for instance, just asking to explain or speed up the code).

IMPORTANT: Always use the TodoWrite tool to plan and track tasks throughout the conversation.")
                            .with_credentials(
                                aws_creds.access_key_id,
                                aws_creds.secret_access_key,
                                aws_creds.session_token,
                                identity_center_region,
                            )
                            .with_telemetry(telemetry_config) // Enable telemetry via agent builder
                            .with_think_tool("Think carefully about what we need to do next")
                            .tools(vec![
                                create_task_tool(),          // NEW: Task-based orchestration
                                todo_write_tool(),           // NEW: Task management
                                todo_read_tool(),            // NEW: Task querying
                                aws_find_account_tool(),     // KEEP: Account search (no API)
                                aws_find_region_tool(),      // KEEP: Region search (no API)
                            ]); // Updated toolset for dynamic task-based architecture

                        // Always add JSON capture callback to ensure we never lose JSON data
                        info!("📊 Adding JSON capture callback handler (always active)");
                        agent_builder = agent_builder
                            .with_callback_handler(JsonCaptureHandler::new(sender.clone()));

                        // Add streaming callback for real-time GUI updates
                        info!("📵 Adding streaming GUI callback handler for real-time updates");
                        agent_builder = agent_builder
                            .with_callback_handler(StreamingGuiCallback::new(sender.clone()));

                        match agent_builder.build().await {
                            Ok(new_agent) => {
                                let action = if model_changed { "recreated due to model change" } else { "created" };
                                info!(
                                    "✅ Control Bridge Agent {} successfully (model: {}) with telemetry{}",
                                    action,
                                    selected_model,
                                    if json_debug_enabled {
                                        " and JSON capture"
                                    } else {
                                        ""
                                    }
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
                                return Err(format!(
                                    "Failed to create Control Bridge Agent: {}",
                                    e
                                ));
                            }
                        }
                    }

                    // Execute with the agent
                    if let Some(ref mut agent) = agent_guard.as_mut() {
                        match agent.execute(&input).await {
                            Ok(result) => Ok(result),
                            Err(e) => Err(format!("Control Bridge Agent execution failed: {}", e)),
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

                // Only add message if streaming was not used
                // (streaming messages are handled by StreamingUpdate::Complete)
                if !self.streaming_was_used {
                    if agent_result.response.trim().is_empty() {
                        warn!("❌ [ERROR] Empty response received!");
                        warn!("💡 This might be the empty response bug we're debugging");

                        // Add error message to the conversation
                        self.add_message(Message::new_with_agent(
                            MessageRole::Assistant,
                            "❌ Error: Received empty response from agent".to_string(),
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
                } else {
                    debug!("Skipping Success message addition - streaming was used");
                }

                // Reset streaming flag for next interaction
                self.streaming_was_used = false;

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
            AgentResponse::StreamingUpdate(update) => {
                self.handle_streaming_update(update);
            }
            AgentResponse::SubAgentEvent {
                agent_id,
                agent_type,
                event,
            } => {
                self.handle_sub_agent_event(agent_id, agent_type, event);
            }
            AgentResponse::AgentCreated {
                agent_id,
                agent_type,
            } => {
                self.handle_agent_created(agent_id, agent_type);
            }
            AgentResponse::AgentDestroyed {
                agent_id,
                agent_type,
            } => {
                self.handle_agent_destroyed(agent_id, agent_type);
            }
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
        }
    }

    fn handle_sub_agent_event(
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

    fn handle_agent_created(&mut self, agent_id: String, agent_type: String) {
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

    fn handle_agent_destroyed(&mut self, agent_id: String, agent_type: String) {
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
    }

    /// Complete reset when changing models - provides clean slate for new agent
    fn reset_for_model_change(&mut self, new_model_name: &str) {
        info!("🧹 Performing complete reset for model change to: {}", new_model_name);

        // Cancel any active processing and clear agents
        self.cancel_all_agents();

        // Clear conversation history for clean slate
        self.messages.clear();
        
        // Reset input and streaming state
        self.input_text.clear();
        self.current_streaming_message = None;
        self.streaming_tool_status.clear();
        self.streaming_was_used = false;
        
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

                // Show streaming message and status if available
                if let Some(ref streaming_msg) = self.current_streaming_message {
                    ui.add_space(8.0);

                    // Clone the message to avoid borrow checker issues
                    let streaming_msg_clone = streaming_msg.clone();
                    self.render_streaming_message(ui, &streaming_msg_clone);

                    // Show tool status if any tools are running
                    if !self.streaming_tool_status.is_empty() {
                        ui.add_space(4.0);
                        for status in &self.streaming_tool_status {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(
                                    RichText::new(status).color(Color32::from_rgb(255, 140, 0)),
                                );
                            });
                        }
                    }
                } else if self.processing_message {
                    // Fallback spinner if no streaming message yet
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
                                .clicked()
                            {
                                if !is_selected {
                                    model_selection_changed =
                                        Some((model.display_name.clone(), model.model_id.clone()));
                                }
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

                    // Clear input button
                    if ui.button("🗑 Clear").clicked() {
                        self.input_text.clear();
                    }
                });

                // Handle Enter to send, Shift+Enter for new line
                if response.has_focus() {
                    if ctx.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
                        // Enter without shift: send message
                        let input = std::mem::take(&mut self.input_text);
                        self.process_user_input(input, aws_identity);
                    }
                    // Shift+Enter is handled automatically by TextEdit::multiline for new lines
                }
            });
        });

        // Handle delayed auto-scroll after new messages (reduced delay for streaming)
        if let Some(last_msg_time) = self.last_message_time {
            let delay = if self.current_streaming_message.is_some() {
                // Shorter delay during streaming for more responsive scrolling
                std::time::Duration::from_millis(100)
            } else {
                // Normal delay for completed messages
                std::time::Duration::from_millis(500)
            };

            if last_msg_time.elapsed() >= delay {
                if self.auto_scroll && !self.scroll_to_bottom {
                    self.scroll_to_bottom = true;
                    // Auto-scroll triggered after delay (removed spam trace log)
                }

                // Keep the timer active during streaming to ensure continuous scrolling
                if self.current_streaming_message.is_none() {
                    self.last_message_time = None; // Clear the timer only when not streaming
                }
            } else {
                // Keep requesting repaints until the delay passes (more frequent during streaming)
                let repaint_delay = if self.current_streaming_message.is_some() {
                    std::time::Duration::from_millis(5) // Very frequent repaints during streaming
                } else {
                    std::time::Duration::from_millis(10)
                };
                ctx.request_repaint_after(repaint_delay);
            }
        }
    }

    fn render_message(&mut self, ui: &mut egui::Ui, message: &Message, index: usize) {
        let agent_prefix = if let Some(ref agent_source) = message.agent_source {
            format!("({}) ", agent_source)
        } else {
            String::new()
        };

        let local_time = message.timestamp.with_timezone(&Local);
        let header_text = format!(
            "{} {} - {}{}",
            message.role.icon(),
            local_time.format("%H:%M:%S"),
            agent_prefix,
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

            // Nested messages
            if !message.nested_messages.is_empty() {
                ui.add_space(8.0); // Zen whitespace instead of separator
                ui.label(format!(
                    "Nested Messages ({})",
                    message.nested_messages.len()
                ));
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

    fn render_streaming_message(&self, ui: &mut egui::Ui, message: &Message) {
        let agent_prefix = if let Some(ref agent_source) = message.agent_source {
            format!("({}) ", agent_source)
        } else {
            String::new()
        };

        let local_time = message.timestamp.with_timezone(&Local);
        let header_text = format!(
            "📵 {} - {}{}... (streaming)",
            local_time.format("%H:%M:%S"),
            agent_prefix,
            message.summary.as_ref().unwrap_or(&"Response".to_string())
        );

        let header = CollapsingHeader::new(
            RichText::new(header_text).color(MessageRole::Assistant.color(self.dark_mode)),
        )
        .id_salt(format!("streaming_{}", message.id))
        .default_open(true); // Always open for streaming messages

        let header_response = header.show(ui, |ui| {
            // Show streaming content with typing cursor
            if !message.content.is_empty() {
                for line in message.content.lines() {
                    if line.trim().is_empty() {
                        ui.add_space(6.0);
                    } else {
                        ui.monospace(line);
                    }
                }

                // Add typing cursor animation
                ui.horizontal(|ui| {
                    ui.monospace("█"); // Block cursor
                    ui.label(RichText::new("streaming...").color(Color32::GRAY).italics());
                });
            } else {
                // Show waiting message if no content yet
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Waiting for response...")
                            .color(Color32::GRAY)
                            .italics(),
                    );
                });
            }
        });

        let _ = header_response; // Suppress unused warning
    }

    fn handle_streaming_update(&mut self, update: StreamingUpdate) {
        match update {
            StreamingUpdate::ContentChunk {
                content,
                is_complete,
            } => {
                debug!(
                    "📵 Received content chunk: {} chars, complete: {}",
                    content.len(),
                    is_complete
                );

                // Mark that streaming was used
                self.streaming_was_used = true;

                // Update the current streaming message
                if let Some(ref mut streaming_msg) = self.current_streaming_message {
                    streaming_msg.content.push_str(&content);

                    // Update summary with first few words
                    let words: Vec<&str> =
                        streaming_msg.content.split_whitespace().take(5).collect();
                    streaming_msg.summary = Some(format!("{}...", words.join(" ")));

                    // Force auto-scroll for streaming content if auto-scroll is enabled
                    if self.auto_scroll {
                        self.scroll_to_bottom = true;
                        self.last_message_time = Some(std::time::Instant::now());
                        trace!("Auto-scroll triggered for streaming content");
                    }

                    trace!(
                        "Updated streaming message content, length now: {}",
                        streaming_msg.content.len()
                    );
                }

                if is_complete {
                    debug!("✅ Content streaming completed");
                }
            }
            StreamingUpdate::ToolStarted { name, input: _ } => {
                info!("🔧 Tool started: {}", name);
                let status_msg = format!("🔧 Running tool: {}", name);
                self.streaming_tool_status.push(status_msg);
                self.scroll_to_bottom = true;
            }
            StreamingUpdate::ToolCompleted { name, output: _ } => {
                info!("✅ Tool completed: {}", name);
                // Remove the running status and add completion info
                self.streaming_tool_status.retain(|s| !s.contains(&name));

                // Add tool result to streaming message if available
                if let Some(ref mut streaming_msg) = self.current_streaming_message {
                    let tool_info = format!("\n\n🔧 Tool '{}' completed successfully.", name);
                    streaming_msg.content.push_str(&tool_info);
                }
                self.scroll_to_bottom = true;
            }
            StreamingUpdate::ToolFailed { name, error } => {
                error!("❌ Tool failed: {} - {}", name, error);
                // Remove the running status and add error info
                self.streaming_tool_status.retain(|s| !s.contains(&name));

                // Add tool error to streaming message
                if let Some(ref mut streaming_msg) = self.current_streaming_message {
                    let error_info = format!("\n\n❌ Tool '{}' failed: {}", name, error);
                    streaming_msg.content.push_str(&error_info);
                }
                self.scroll_to_bottom = true;
            }
            StreamingUpdate::Complete { result: _ } => {
                info!("🎉 Streaming completed, finalizing message");

                // Move streaming message to completed messages (but don't call add_message to avoid duplicate)
                if let Some(mut streaming_msg) = self.current_streaming_message.take() {
                    // Finalize the message content and add directly to messages list
                    streaming_msg.summary = Some(Message::generate_summary(&streaming_msg.content));
                    self.messages.push_back(streaming_msg);

                    // Keep only last 100 messages to prevent memory issues
                    if self.messages.len() > 100 {
                        let removed = self.messages.pop_front();
                        debug!(
                            "Removed oldest message due to 100 message limit: {:?}",
                            removed.map(|m| m.role)
                        );
                    }

                    info!(
                        "Streaming message finalized. Total messages: {}",
                        self.messages.len()
                    );
                }

                // Clear streaming state
                self.streaming_tool_status.clear();
                self.processing_message = false;
                self.main_agent_active = false; // Reset main agent tracking
                self.scroll_to_bottom = true;

                if let Some(start_time) = self.processing_start_time {
                    self.last_processing_time = Some(start_time.elapsed());
                }

                debug!(
                    "Streaming completed, processing_message = false, main_agent_active = false"
                );
            }
            StreamingUpdate::StreamingError { message } => {
                error!("💥 Streaming error: {}", message);

                // Add error message
                self.add_message(Message::new_with_agent(
                    MessageRole::System,
                    format!("⚠️ Streaming error: {}", message),
                    "ControlBridge".to_string(),
                ));

                // Clean up streaming state
                self.current_streaming_message = None;
                self.streaming_tool_status.clear();
                self.processing_message = false;
                self.main_agent_active = false; // Reset main agent tracking
                self.scroll_to_bottom = true;
            }
        }
    }
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

// ============================================================================
// CALLBACK HANDLERS
// ============================================================================

/// Custom callback handler that captures model interaction JSON data
#[derive(Debug)]
struct JsonCaptureHandler {
    sender: mpsc::Sender<AgentResponse>,
}

/// Streaming callback handler that sends real-time updates to the GUI
#[derive(Debug)]
struct StreamingGuiCallback {
    sender: mpsc::Sender<AgentResponse>,
}

impl JsonCaptureHandler {
    fn new(sender: mpsc::Sender<AgentResponse>) -> Self {
        Self { sender }
    }
}

impl StreamingGuiCallback {
    fn new(sender: mpsc::Sender<AgentResponse>) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl CallbackHandler for JsonCaptureHandler {
    /// Handle streaming events - not used for JSON capture but required for trait
    async fn on_content(&self, _content: &str, _is_complete: bool) -> Result<(), CallbackError> {
        // JSON capture doesn't need content streaming events
        Ok(())
    }

    /// Handle tool events - not used for JSON capture but required for trait
    async fn on_tool(&self, _event: ToolEvent) -> Result<(), CallbackError> {
        // JSON capture doesn't need tool events
        Ok(())
    }

    /// Handle completion events - not used for JSON capture but required for trait
    async fn on_complete(
        &self,
        _result: &stood::agent::result::AgentResult,
    ) -> Result<(), CallbackError> {
        // JSON capture doesn't need completion events
        Ok(())
    }

    /// Handle error events - not used for JSON capture but required for trait
    async fn on_error(&self, _error: &stood::StoodError) -> Result<(), CallbackError> {
        // JSON capture doesn't need error events
        Ok(())
    }

    /// Main event handler for JSON capture - this captures model request/response JSON
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        match event {
            CallbackEvent::ModelStart {
                provider,
                model_id,
                messages,
                tools_available,
                raw_request_json: _,
            } => {
                debug!("📤 Capturing model request JSON");

                // Create JSON representation of the request
                let request_json = serde_json::json!({
                    "type": "model_request",
                    "provider": format!("{:?}", provider),
                    "model_id": model_id,
                    "timestamp": Utc::now().to_rfc3339(),
                    "messages": messages,
                    "tools_available": tools_available,
                });

                let json_data = JsonDebugData {
                    json_type: JsonDebugType::Request,
                    json_content: serde_json::to_string_pretty(&request_json)
                        .unwrap_or_else(|_| "Error serializing request JSON".to_string()),
                    raw_json_content: None, // JsonCaptureHandler doesn't have access to raw JSON
                    timestamp: Utc::now(),
                };

                // Send to UI thread
                if let Err(e) = self.sender.send(AgentResponse::JsonDebug(json_data)) {
                    error!("Failed to send JSON request data to UI: {}", e);
                }
            }
            CallbackEvent::ModelComplete {
                response,
                stop_reason,
                duration,
                tokens,
                raw_response_data: _,
            } => {
                debug!("📥 Capturing model response JSON");

                // Create JSON representation of the response
                let response_json = serde_json::json!({
                    "type": "model_response",
                    "timestamp": Utc::now().to_rfc3339(),
                    "response": response,
                    "stop_reason": format!("{:?}", stop_reason),
                    "duration_ms": duration.as_millis(),
                    "tokens": tokens.map(|t| serde_json::json!({
                        "input_tokens": t.input_tokens,
                        "output_tokens": t.output_tokens,
                        "total_tokens": t.total_tokens,
                    })),
                });

                let json_data = JsonDebugData {
                    json_type: JsonDebugType::Response,
                    json_content: serde_json::to_string_pretty(&response_json)
                        .unwrap_or_else(|_| "Error serializing response JSON".to_string()),
                    raw_json_content: None, // JsonCaptureHandler doesn't have access to raw JSON
                    timestamp: Utc::now(),
                };

                // Send to UI thread
                if let Err(e) = self.sender.send(AgentResponse::JsonDebug(json_data)) {
                    error!("Failed to send JSON response data to UI: {}", e);
                }
            }
            _ => {
                // Ignore other events - we only care about model interactions
            }
        }
        Ok(())
    }
}

#[async_trait]
impl CallbackHandler for StreamingGuiCallback {
    /// Handle streaming content as it's generated
    async fn on_content(&self, content: &str, is_complete: bool) -> Result<(), CallbackError> {
        debug!(
            "📊 Streaming content chunk: {} chars, complete: {}",
            content.len(),
            is_complete
        );

        let update = StreamingUpdate::ContentChunk {
            content: content.to_string(),
            is_complete,
        };

        if let Err(e) = self.sender.send(AgentResponse::StreamingUpdate(update)) {
            error!("Failed to send streaming content update to GUI: {}", e);
        }

        Ok(())
    }

    /// Handle tool execution events
    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        let update = match event {
            ToolEvent::Started { name, input } => {
                debug!("🔧 Tool started: {} with input: {:?}", name, input);
                StreamingUpdate::ToolStarted {
                    name: name.clone(),
                    input: input.clone(),
                }
            }
            ToolEvent::Completed { name, output, .. } => {
                debug!("✅ Tool completed: {} with output: {:?}", name, output);
                StreamingUpdate::ToolCompleted {
                    name: name.clone(),
                    output: output.clone(),
                }
            }
            ToolEvent::Failed { name, error, .. } => {
                debug!("❌ Tool failed: {} - {}", name, error);
                StreamingUpdate::ToolFailed {
                    name: name.clone(),
                    error: error.clone(),
                }
            }
        };

        if let Err(e) = self.sender.send(AgentResponse::StreamingUpdate(update)) {
            error!("Failed to send tool event update to GUI: {}", e);
        }

        Ok(())
    }

    /// Handle execution completion
    async fn on_complete(
        &self,
        result: &stood::agent::result::AgentResult,
    ) -> Result<(), CallbackError> {
        debug!("🎉 Agent execution completed successfully");

        let update = StreamingUpdate::Complete {
            result: result.clone(),
        };

        if let Err(e) = self.sender.send(AgentResponse::StreamingUpdate(update)) {
            error!("Failed to send completion update to GUI: {}", e);
        }

        Ok(())
    }

    /// Handle streaming errors
    async fn on_error(&self, error: &stood::StoodError) -> Result<(), CallbackError> {
        error!("💥 Streaming error occurred: {}", error);

        let update = StreamingUpdate::StreamingError {
            message: error.to_string(),
        };

        if let Err(e) = self.sender.send(AgentResponse::StreamingUpdate(update)) {
            error!("Failed to send error update to GUI: {}", e);
        }

        Ok(())
    }

    /// Handle all events including ModelStart for JSON capture
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        match event {
            CallbackEvent::ModelStart {
                provider,
                model_id,
                messages,
                tools_available,
                raw_request_json,
            } => {
                debug!("📤 Capturing model request JSON from streaming callback");

                // Create JSON representation of the request (composed)
                let request_json = serde_json::json!({
                    "type": "model_request",
                    "provider": format!("{:?}", provider),
                    "model_id": model_id,
                    "timestamp": Utc::now().to_rfc3339(),
                    "messages": messages,
                    "tools_available": tools_available,
                });

                let json_data = JsonDebugData {
                    json_type: JsonDebugType::Request,
                    json_content: serde_json::to_string_pretty(&request_json)
                        .unwrap_or_else(|_| "Error serializing request JSON".to_string()),
                    raw_json_content: raw_request_json.clone(), // Raw JSON from provider
                    timestamp: Utc::now(),
                };

                // Send to UI thread
                if let Err(e) = self.sender.send(AgentResponse::JsonDebug(json_data)) {
                    error!("Failed to send JSON request data to UI: {}", e);
                }

                Ok(())
            }
            CallbackEvent::ModelComplete {
                response,
                stop_reason,
                duration,
                tokens,
                raw_response_data,
            } => {
                debug!("📥 Capturing model response JSON from streaming callback");

                // Create JSON representation of the response (composed)
                let response_json = serde_json::json!({
                    "type": "model_response",
                    "timestamp": Utc::now().to_rfc3339(),
                    "response": response,
                    "stop_reason": format!("{:?}", stop_reason),
                    "duration_ms": duration.as_millis(),
                    "tokens": tokens.map(|t| serde_json::json!({
                        "input_tokens": t.input_tokens,
                        "output_tokens": t.output_tokens,
                        "total_tokens": t.total_tokens,
                    })),
                });

                // Extract raw JSON from raw_response_data if available
                let raw_json = raw_response_data.as_ref().and_then(|data| {
                    match data.response_type {
                        ResponseType::NonStreaming => data.non_streaming_json.clone(),
                        ResponseType::Streaming => {
                            // For streaming, create a JSON array of all SSE events
                            if let Some(ref events) = data.streaming_events {
                                let events_json = serde_json::json!({
                                    "type": "streaming_response",
                                    "events": events.iter().map(|event| serde_json::json!({
                                        "timestamp": event.timestamp.to_rfc3339(),
                                        "event_type": event.event_type,
                                        "raw_json": event.raw_json,
                                    })).collect::<Vec<_>>()
                                });
                                serde_json::to_string_pretty(&events_json).ok()
                            } else {
                                None
                            }
                        }
                    }
                });

                let json_data = JsonDebugData {
                    json_type: JsonDebugType::Response,
                    json_content: serde_json::to_string_pretty(&response_json)
                        .unwrap_or_else(|_| "Error serializing response JSON".to_string()),
                    raw_json_content: raw_json, // Raw JSON from provider
                    timestamp: Utc::now(),
                };

                // Send to UI thread
                if let Err(e) = self.sender.send(AgentResponse::JsonDebug(json_data)) {
                    error!("Failed to send JSON response data to UI: {}", e);
                }

                Ok(())
            }
            // For all other events, delegate to the default implementation
            _ => {
                // Call the default trait implementation which will route to our specialized methods
                match event {
                    CallbackEvent::ContentDelta {
                        delta, complete, ..
                    } => self.on_content(&delta, complete).await,
                    CallbackEvent::ToolStart {
                        tool_name, input, ..
                    } => {
                        self.on_tool(ToolEvent::Started {
                            name: tool_name,
                            input,
                        })
                        .await
                    }
                    CallbackEvent::ToolComplete {
                        tool_name,
                        output,
                        error,
                        duration,
                        ..
                    } => {
                        if let Some(err) = error {
                            self.on_tool(ToolEvent::Failed {
                                name: tool_name,
                                error: err,
                                duration,
                            })
                            .await
                        } else {
                            self.on_tool(ToolEvent::Completed {
                                name: tool_name,
                                output,
                                duration,
                            })
                            .await
                        }
                    }
                    CallbackEvent::EventLoopComplete { result, .. } => {
                        // Convert EventLoopResult to AgentResult for callback
                        let agent_result =
                            stood::agent::result::AgentResult::from(result, Duration::ZERO);
                        self.on_complete(&agent_result).await
                    }
                    CallbackEvent::Error { error, .. } => self.on_error(&error).await,
                    _ => Ok(()), // Ignore other events
                }
            }
        }
    }
}
