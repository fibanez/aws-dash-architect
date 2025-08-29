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
    ModelSettings, BridgeDebugEvent, init_bridge_debug_logger, log_bridge_debug_event,
};
use crate::app::dashui::window_focus::{FocusableWindow, IdentityShowParams};
use crate::create_agent_with_model;
use chrono::{DateTime, Utc};
use egui::{CollapsingHeader, Color32, RichText, ScrollArea, TextEdit, Window};
use std::collections::{HashMap, VecDeque};
use std::sync::{mpsc, Arc, Mutex};
use stood::agent::{result::AgentResult, Agent};
use stood::agent::callbacks::{CallbackError, CallbackEvent, CallbackHandler, ToolEvent};
use stood::telemetry::TelemetryConfig;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;
use async_trait::async_trait;

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
                            .insert("session.id".to_string(), session_id.clone());

                        // Initialize bridge debug logger
                        if let Err(e) = init_bridge_debug_logger() {
                            warn!("Failed to initialize bridge debug logger: {}", e);
                        } else {
                            info!("🔍 Bridge debug logger initialized successfully");
                        }
                        telemetry_config
                            .service_attributes
                            .insert("deployment.environment".to_string(), "desktop-application".to_string());

                        // Configure model for this agent
                        let agent_builder = create_agent_with_model!(Agent::builder(), &selected_model)
                            .system_prompt("You are the AWS Bridge Agent - a task orchestrator for AWS infrastructure management.

🔴 CRITICAL: ALWAYS PROVIDE A FINAL RESPONSE TO THE USER!

When you use tools, you MUST follow this exact pattern:
1. Call the tool(s) needed
2. Receive the tool results
3. **ALWAYS write a final response that presents the tool results to the user**

NEVER end your turn immediately after calling a tool. You MUST summarize what the tool found.

Example of CORRECT behavior:
User: 'list aws accounts'
Assistant: I'll search for available AWS accounts.
[calls aws_find_account tool]
[receives tool results]
Assistant: I found 3 AWS accounts:
- Production (123456789012)
- Staging (234567890123)  
- Development (345678901234)

Example of WRONG behavior (DO NOT DO THIS):
User: 'list aws accounts'
Assistant: [calls aws_find_account tool]
[ends without presenting results - THIS IS WRONG!]

IMPORTANT: Always use TodoWrite to plan and track multi-step tasks. This is CRITICAL for user visibility.

DO NOT attempt complex AWS operations directly. Instead, create specialized task agents via create_task.

CRITICAL REQUIREMENTS for AWS operations:
- Account ID (use aws_find_account if user doesn't specify - then SHOW THE RESULTS!)
- Region (use aws_find_region if user doesn't specify - then SHOW THE RESULTS!)  
- Resource identifier (ID, name, or ARN)

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

RESPONSE GUIDELINES:
1. **ALWAYS present tool results** - This is your #1 priority
2. Be concise AFTER presenting the results
3. Focus on answering what the user asked

Tone and style
🔴 **TOOL RESULTS FIRST**: Always show what tools found before being concise.
You should be direct and to the point AFTER presenting tool results. When you run a non-trivial aws commands, you
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
    directly, without elaboration, explanation, or details. 
    EXCEPTION: You MUST present tool results. After tools, be concise.
    For tool results, say what you found: 'Found 3 accounts: [details]'
    Avoid unnecessary phrases like 'Based on the information...' 
    Here are some examples to demonstrate appropriate verbosity:
    user: 2 + 2 
    assistant: 4
    
    user: list aws accounts
    assistant: [calls aws_find_account]
    Found 3 AWS accounts: Production (123456789012), Staging (234567890123), Development (345678901234)

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

                        // Add tool callback handler to create tree structure for tool calls
                        let tool_callback_handler = BridgeToolCallbackHandler::new(sender.clone());
                        let agent_builder = agent_builder.with_callback_handler(tool_callback_handler);
                        info!("🔍 Bridge agent created with tool callback handler for tree visualization");
                        
                        // Only log to debug file, don't interfere with event loop
                        log_bridge_debug_event(BridgeDebugEvent::BridgeAgentStart {
                            timestamp: Utc::now(),
                            session_id: session_id.clone(),
                            user_request: input.clone(),
                        });

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

// ============================================================================
// CALLBACK HANDLERS
// ============================================================================

// Removed JsonCaptureHandler and StreamingGuiCallback - agents handle their own event loops
/*
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
*/

// Removed BridgeDebugHandler - using direct debug logging instead
/*
/// Bridge debug callback handler for detailed logging
#[derive(Debug)]
struct BridgeDebugHandler {
    session_id: String,
}

impl BridgeDebugHandler {
    fn new(session_id: String) -> Self {
        Self { session_id }
    }
}

#[async_trait]
impl CallbackHandler for BridgeDebugHandler {
    async fn on_content(&self, _content: &str, _is_complete: bool) -> Result<(), CallbackError> {
        // Bridge debug doesn't need content streaming events
        Ok(())
    }

    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        match event {
            ToolEvent::Started { name, input } => {
                log_bridge_debug_event(BridgeDebugEvent::BridgeToolCall {
                    timestamp: Utc::now(),
                    session_id: self.session_id.clone(),
                    tool_name: name,
                    input_params: input,
                    success: false, // Still running
                    output_result: None,
                    error_message: None,
                });
            },
            ToolEvent::Completed { name, output, .. } => {
                log_bridge_debug_event(BridgeDebugEvent::BridgeToolCall {
                    timestamp: Utc::now(),
                    session_id: self.session_id.clone(),
                    tool_name: name,
                    input_params: serde_json::json!({}), // We don't have input here
                    success: true,
                    output_result: output,
                    error_message: None,
                });
            },
            ToolEvent::Failed { name, error, .. } => {
                log_bridge_debug_event(BridgeDebugEvent::BridgeToolCall {
                    timestamp: Utc::now(),
                    session_id: self.session_id.clone(),
                    tool_name: name,
                    input_params: serde_json::json!({}), // We don't have input here
                    success: false,
                    output_result: None,
                    error_message: Some(error),
                });
            },
        }
        Ok(())
    }

    async fn on_complete(&self, result: &AgentResult) -> Result<(), CallbackError> {
        log_bridge_debug_event(BridgeDebugEvent::SessionEnd {
            timestamp: Utc::now(),
            session_id: self.session_id.clone(),
            total_duration_ms: result.duration.as_millis() as u64,
        });
        Ok(())
    }

    async fn on_error(&self, error: &stood::StoodError) -> Result<(), CallbackError> {
        log_bridge_debug_event(BridgeDebugEvent::BridgeToolCall {
            timestamp: Utc::now(),
            session_id: self.session_id.clone(),
            tool_name: "bridge-execution".to_string(),
            input_params: serde_json::json!({"error": "execution_error"}),
            success: false,
            output_result: None,
            error_message: Some(error.to_string()),
        });
        Ok(())
    }

    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        use crate::app::bridge::extract_tool_calls_from_response;
        
        match event {
            CallbackEvent::ModelStart { model_id, messages, .. } => {
                // Extract system prompt and user message
                let system_prompt = messages
                    .iter()
                    .find(|msg| matches!(msg.role, stood::MessageRole::System))
                    .map(|msg| content_blocks_to_string(&msg.content))
                    .unwrap_or_else(|| "no system prompt found".to_string());

                let user_message = messages
                    .iter()
                    .find(|msg| matches!(msg.role, stood::MessageRole::User))
                    .map(|msg| content_blocks_to_string(&msg.content))
                    .unwrap_or_else(|| "no user message found".to_string());

                log_bridge_debug_event(BridgeDebugEvent::BridgePromptSent {
                    timestamp: Utc::now(),
                    session_id: self.session_id.clone(),
                    system_prompt,
                    user_message,
                    model_id,
                });
            },
            CallbackEvent::ModelComplete { response, .. } => {
                let tool_calls_requested = extract_tool_calls_from_response(&response);

                log_bridge_debug_event(BridgeDebugEvent::BridgeResponseReceived {
                    timestamp: Utc::now(),
                    session_id: self.session_id.clone(),
                    full_response: response,
                    tool_calls_requested,
                });
            },
            _ => {
                // Delegate other events to the default implementation
                match event {
                    CallbackEvent::ToolStart { tool_name, input, .. } => {
                        return self.on_tool(ToolEvent::Started { name: tool_name, input }).await;
                    },
                    CallbackEvent::ToolComplete { tool_name, output, error, duration, .. } => {
                        if let Some(err) = error {
                            return self.on_tool(ToolEvent::Failed { name: tool_name, error: err, duration }).await;
                        } else {
                            return self.on_tool(ToolEvent::Completed { name: tool_name, output, duration }).await;
                        }
                    },
                    CallbackEvent::EventLoopComplete { result, .. } => {
                        let agent_result = stood::agent::result::AgentResult::from(result, Duration::ZERO);
                        return self.on_complete(&agent_result).await;
                    },
                    CallbackEvent::Error { error, .. } => {
                        return self.on_error(&error).await;
                    },
                    _ => {}, // Ignore other events
                }
            },
        }
        Ok(())
    }
}
*/

/// Bridge Tool Callback Handler - Creates tree structure for tool calls
///
/// This handler creates "Calling tool" nodes when tools start and adds
/// child nodes with tool responses when tools complete.
#[derive(Debug, Clone)]
pub struct BridgeToolCallbackHandler {
    sender: mpsc::Sender<AgentResponse>,
    active_tool_nodes: Arc<Mutex<HashMap<String, String>>>, // tool_use_id -> parent_message_id
}

impl BridgeToolCallbackHandler {
    pub fn new(sender: mpsc::Sender<AgentResponse>) -> Self {
        Self {
            sender,
            active_tool_nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Map tool names to user-friendly actions
    fn get_user_friendly_action(tool_name: &str) -> &'static str {
        match tool_name {
            "aws_list_resources" => "List",
            "aws_describe_resource" => "Describe", 
            "aws_find_account" => "Find Account",
            "aws_find_region" => "Find Region",
            "create_task" => "Task",
            "search_logs" => "Search Logs",
            "analyze_logs" => "Analyze",
            _ => "Tool", // Generic fallback
        }
    }
}

#[async_trait]
impl CallbackHandler for BridgeToolCallbackHandler {
    /// Handle streaming content - not needed for tool callbacks
    async fn on_content(&self, _content: &str, _is_complete: bool) -> Result<(), CallbackError> {
        Ok(())
    }

    /// Handle tool execution events to create tree structure
    async fn on_tool(&self, event: ToolEvent) -> Result<(), CallbackError> {
        match event {
            ToolEvent::Started { name, input } => {
                // Create "Calling tool" parent node
                let tool_node_id = format!("tool_{}_{}", name, Utc::now().timestamp_millis());
                
                // Get friendly name for display
                let friendly_name = Self::get_user_friendly_action(&name);
                
                // For create_task, show the task description prominently
                let (content, summary) = if name == "create_task" {
                    let task_description = input
                        .get("task_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Task execution");
                    
                    // Show the task description as the main content
                    let content = format!("🎯 {}", task_description);
                    let summary = format!("{}: {}", friendly_name, 
                        task_description.chars().take(50).collect::<String>() + 
                        if task_description.len() > 50 { "..." } else { "" });
                    
                    (content, summary)
                } else {
                    // For other tools, show friendly name
                    let content = format!("🔧 {}", friendly_name);
                    let summary = friendly_name.to_string();
                    (content, summary)
                };
                
                // Create nested message with JSON input parameters
                let json_input_message = Message {
                    id: format!("{}_input", tool_node_id),
                    role: MessageRole::JsonRequest,
                    content: serde_json::to_string_pretty(&input).unwrap_or_else(|_| format!("{:?}", input)),
                    timestamp: Utc::now(),
                    summary: Some("Input Parameters".to_string()),
                    debug_info: None,
                    nested_messages: Vec::new(),
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                let parent_message = Message {
                    id: tool_node_id.clone(),
                    role: MessageRole::System,
                    content,
                    timestamp: Utc::now(),
                    summary: Some(summary),
                    debug_info: None,
                    nested_messages: vec![json_input_message],
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                // Store the parent node ID for when the tool completes
                // Note: We use tool name + timestamp as a unique ID since tool_use_id may not be available
                let tool_key = format!("{}_{}", name, parent_message.timestamp.timestamp_millis());
                self.active_tool_nodes.lock().unwrap().insert(tool_key, tool_node_id.clone());

                // Send parent node to UI via ToolCallStart message
                let response = AgentResponse::ToolCallStart {
                    parent_message,
                };

                if let Err(e) = self.sender.send(response) {
                    error!("Failed to send tool start message to GUI: {}", e);
                }
            }
            
            ToolEvent::Completed { name, output, duration } => {
                // Get friendly name
                let friendly_name = Self::get_user_friendly_action(&name);
                
                // Create nested message with JSON output
                let json_output_message = Message {
                    id: format!("tool_output_{}_{}", name, Utc::now().timestamp_millis()),
                    role: MessageRole::JsonResponse,
                    content: match &output {
                        Some(value) => serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value)),
                        None => "null".to_string(),
                    },
                    timestamp: Utc::now(),
                    summary: Some("Output Result".to_string()),
                    debug_info: None,
                    nested_messages: Vec::new(),
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                let child_message = Message {
                    id: format!("tool_response_{}_{}", name, Utc::now().timestamp_millis()),
                    role: MessageRole::Assistant,
                    content: format!("✅ {} completed ({:.2}s)", friendly_name, duration.as_secs_f64()),
                    timestamp: Utc::now(),
                    summary: Some(format!("{} Result", friendly_name)),
                    debug_info: None,
                    nested_messages: vec![json_output_message],
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                // Find the most recent tool node with this name (simple matching)
                let parent_node_id = {
                    let active_nodes = self.active_tool_nodes.lock().unwrap();
                    active_nodes.iter()
                        .filter(|(key, _)| key.starts_with(&format!("{}_", name)))
                        .max_by_key(|(key, _)| {
                            key.split('_').last().unwrap_or("0").parse::<i64>().unwrap_or(0)
                        })
                        .map(|(_, id)| id.clone())
                };

                if let Some(parent_id) = parent_node_id {
                    // Send child node to UI
                    let response = AgentResponse::ToolCallComplete {
                        parent_message_id: parent_id.clone(),
                        child_message,
                    };

                    if let Err(e) = self.sender.send(response) {
                        error!("Failed to send tool complete message to GUI: {}", e);
                    }

                    // Clean up the mapping
                    self.active_tool_nodes.lock().unwrap().retain(|_, v| *v != parent_id);
                }
            }
            
            ToolEvent::Failed { name, error, duration } => {
                // Get friendly name
                let friendly_name = Self::get_user_friendly_action(&name);
                
                // Create child error node
                let child_message = Message {
                    id: format!("tool_error_{}_{}", name, Utc::now().timestamp_millis()),
                    role: MessageRole::Debug,
                    content: format!("❌ {} failed ({:.2}s):\n{}", friendly_name, duration.as_secs_f64(), error),
                    timestamp: Utc::now(),
                    summary: Some(format!("{} Error", friendly_name)),
                    debug_info: None,
                    nested_messages: Vec::new(),
                    agent_source: Some("Bridge-Tool-Callback".to_string()),
                    json_debug_data: Vec::new(),
                };

                // Find the most recent tool node with this name
                let parent_node_id = {
                    let active_nodes = self.active_tool_nodes.lock().unwrap();
                    active_nodes.iter()
                        .filter(|(key, _)| key.starts_with(&format!("{}_", name)))
                        .max_by_key(|(key, _)| {
                            key.split('_').last().unwrap_or("0").parse::<i64>().unwrap_or(0)
                        })
                        .map(|(_, id)| id.clone())
                };

                if let Some(parent_id) = parent_node_id {
                    let response = AgentResponse::ToolCallComplete {
                        parent_message_id: parent_id.clone(),
                        child_message,
                    };

                    if let Err(e) = self.sender.send(response) {
                        error!("Failed to send tool error message to GUI: {}", e);
                    }

                    // Clean up the mapping
                    self.active_tool_nodes.lock().unwrap().retain(|_, v| *v != parent_id);
                }
            }
        }
        Ok(())
    }

    /// Handle completion events - not needed for tool callbacks
    async fn on_complete(&self, _result: &stood::agent::result::AgentResult) -> Result<(), CallbackError> {
        Ok(())
    }

    /// Handle error events - not needed for tool callbacks  
    async fn on_error(&self, _error: &stood::StoodError) -> Result<(), CallbackError> {
        Ok(())
    }

    /// Handle all callback events
    async fn handle_event(&self, event: CallbackEvent) -> Result<(), CallbackError> {
        match event {
            CallbackEvent::ToolStart { tool_name, input, .. } => {
                self.on_tool(ToolEvent::Started { name: tool_name, input }).await
            }
            CallbackEvent::ToolComplete { tool_name, output, error, duration, .. } => {
                if let Some(err) = error {
                    self.on_tool(ToolEvent::Failed { name: tool_name, error: err, duration }).await
                } else {
                    self.on_tool(ToolEvent::Completed { name: tool_name, output, duration }).await
                }
            }
            _ => Ok(()), // Ignore other events
        }
    }
}
