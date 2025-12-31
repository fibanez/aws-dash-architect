//! Standalone Agent Instance
//!
//! Simplified agent implementation that uses stood library directly.
//! This version uses stood library directly for simplicity and reliability.

#![warn(clippy::all, rust_2018_idioms)]

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};
use tokio_util::sync::CancellationToken;

use crate::app::agent_framework::agent_logger::AgentLogger;
use crate::app::agent_framework::agent_types::{
    AgentId, AgentMetadata, AgentStatus, AgentType, StoodLogLevel,
};
use crate::app::agent_framework::conversation::{ConversationMessage, ConversationResponse};
use crate::app::agent_framework::message_injection::{
    InjectionContext, InjectionTrigger, InjectionType, MessageInjector,
};
use crate::app::agent_framework::middleware::{
    ConversationLayer, LayerContext, LayerError, LayerStack,
};
use crate::app::agent_framework::status_display::ProcessingPhase;
use crate::app::agent_framework::tools::TodoItem;

/// Standalone Agent Instance
///
/// A simplified agent implementation that:
/// - Uses stood::agent::Agent directly (no wrapper layer)
/// - Has simplified message types (User/Assistant only)
/// - Uses simplified response channel (Success/Error only)
/// - Separates comprehensive logging from simple UI display
/// - Uses lazy agent initialization (agent created on first message)
pub struct AgentInstance {
    // Identity
    /// Unique identifier for this agent
    id: AgentId,
    /// Agent metadata (name, description, model, etc.)
    metadata: AgentMetadata,
    /// Current execution status
    status: AgentStatus,
    /// Type of agent (determines tools and prompts)
    agent_type: AgentType,

    // Stood integration (lazy initialization)
    /// The stood agent (None until first message sent)
    stood_agent: Arc<Mutex<Option<stood::agent::Agent>>>,

    // Communication
    /// Channel for receiving responses from background thread
    response_channel: (
        mpsc::Sender<ConversationResponse>,
        mpsc::Receiver<ConversationResponse>,
    ),

    // State
    /// Conversation history (User and Assistant messages only)
    messages: VecDeque<ConversationMessage>,
    /// Whether the agent is currently processing a message
    processing: bool,
    /// Optional status message for future callback support
    status_message: Option<String>,
    /// Todo list for task-manager agents (shared with tools)
    todo_list_shared: Arc<Mutex<Vec<TodoItem>>>,

    // Integration
    /// Logger for comprehensive debugging (separate from UI)
    logger: Arc<AgentLogger>,
    /// Tokio runtime for background async execution
    runtime: Arc<tokio::runtime::Runtime>,
    /// Stood library log level for this agent
    stood_log_level: StoodLogLevel,

    // Message injection
    /// Message injector for programmatic message injection
    message_injector: MessageInjector,
    /// Flag indicating a pending injection should be processed
    has_pending_injection: bool,
    /// Deferred injection message to be sent on next poll
    deferred_injection: Option<String>,

    // Processing phase tracking
    /// Current processing phase for UI status display
    processing_phase: ProcessingPhase,

    // Middleware
    /// Layer stack for pre/post message processing
    layer_stack: LayerStack,

    // Cancellation
    /// Cancellation token captured from stood agent (for Stop button)
    cancel_token: Option<CancellationToken>,
}

impl AgentInstance {
    /// Create a new standalone agent instance
    ///
    /// Note: The stood agent is not created until the first message is sent.
    /// This allows credential loading and setup to happen asynchronously.
    ///
    /// # Arguments
    ///
    /// * `metadata` - Agent metadata (name, description, model)
    /// * `agent_type` - Type of agent (TaskManager or TaskWorker)
    pub fn new(metadata: AgentMetadata, agent_type: AgentType) -> Self {
        let id = AgentId::new();
        let (tx, rx) = mpsc::channel();

        // Create logger for this agent
        let logger = Arc::new(
            AgentLogger::new(id, metadata.name.clone(), &agent_type)
                .expect("Failed to create agent logger"),
        );

        // Log agent creation with type
        logger.log_agent_created(&agent_type, &metadata);
        logger.log_system_message(&agent_type, &format!("Agent type: {}", agent_type));

        // Create dedicated tokio runtime for this agent
        let runtime =
            Arc::new(tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"));

        Self {
            id,
            metadata,
            status: AgentStatus::Running,
            agent_type,
            stood_agent: Arc::new(Mutex::new(None)),
            response_channel: (tx, rx),
            messages: VecDeque::new(),
            processing: false,
            status_message: None,
            todo_list_shared: Arc::new(Mutex::new(Vec::new())),
            logger,
            runtime,
            stood_log_level: StoodLogLevel::default(), // Debug by default
            message_injector: MessageInjector::new(),
            has_pending_injection: false,
            deferred_injection: None,
            processing_phase: ProcessingPhase::Idle,
            layer_stack: LayerStack::new(),
            cancel_token: None,
        }
    }

    /// Create a new agent instance that shares its parent's logger
    ///
    /// This is used for task-worker agents that should log to their parent
    /// task-manager's log file, allowing the complete conversation flow to
    /// be tracked in a single file.
    ///
    /// # Arguments
    ///
    /// * `metadata` - Agent metadata (name, description, model)
    /// * `agent_type` - Type of agent (TaskWorker)
    /// * `parent_logger` - Parent agent's logger to share
    pub fn new_with_parent_logger(
        metadata: AgentMetadata,
        agent_type: AgentType,
        parent_logger: Arc<AgentLogger>,
    ) -> Self {
        let id = AgentId::new();
        let (tx, rx) = mpsc::channel();

        // Use parent's logger instead of creating new one
        let logger = parent_logger;

        // Log worker creation to parent's log with section header
        logger.log_system_message(
            &agent_type,
            &format!("\n====== Worker Agent: {} ({}) ======", metadata.name, id),
        );
        logger.log_agent_created(&agent_type, &metadata);
        logger.log_system_message(&agent_type, &format!("Agent type: {}", agent_type));

        // Create dedicated tokio runtime for this agent
        let runtime =
            Arc::new(tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"));

        Self {
            id,
            metadata,
            status: AgentStatus::Running,
            agent_type,
            stood_agent: Arc::new(Mutex::new(None)),
            response_channel: (tx, rx),
            messages: VecDeque::new(),
            processing: false,
            status_message: None,
            todo_list_shared: Arc::new(Mutex::new(Vec::new())),
            logger,
            runtime,
            stood_log_level: StoodLogLevel::default(), // Debug by default
            message_injector: MessageInjector::new(),
            has_pending_injection: false,
            deferred_injection: None,
            processing_phase: ProcessingPhase::Idle,
            layer_stack: LayerStack::new(),
            cancel_token: None,
        }
    }

    /// Get the agent's unique ID
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Get the agent's metadata
    pub fn metadata(&self) -> &AgentMetadata {
        &self.metadata
    }

    /// Get the agent's type
    pub fn agent_type(&self) -> AgentType {
        self.agent_type
    }

    /// Get the current todo list (for display/testing)
    pub fn todo_list(&self) -> Vec<TodoItem> {
        self.todo_list_shared.lock().unwrap().clone()
    }

    /// Set the todo list (for testing)
    pub fn set_todo_list(&mut self, todos: Vec<TodoItem>) {
        *self.todo_list_shared.lock().unwrap() = todos;
    }

    /// Clear the todo list
    pub fn clear_todo_list(&mut self) {
        self.todo_list_shared.lock().unwrap().clear();
    }

    /// Get the agent's current status
    pub fn status(&self) -> &AgentStatus {
        &self.status
    }

    /// Get reference to conversation messages
    pub fn messages(&self) -> &VecDeque<ConversationMessage> {
        &self.messages
    }

    /// Check if the agent is currently processing a message
    pub fn is_processing(&self) -> bool {
        self.processing
    }

    /// Get the current status message (for future callback support)
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    /// Get the current processing phase for UI status display
    pub fn processing_phase(&self) -> &ProcessingPhase {
        &self.processing_phase
    }

    // ========== Middleware API ==========

    /// Get reference to the layer stack
    pub fn layer_stack(&self) -> &LayerStack {
        &self.layer_stack
    }

    /// Get mutable reference to the layer stack
    pub fn layer_stack_mut(&mut self) -> &mut LayerStack {
        &mut self.layer_stack
    }

    /// Add a middleware layer to the agent
    ///
    /// Layers process messages before sending and responses after receiving.
    /// Layers are processed in the order they are added for pre-send,
    /// and in reverse order for post-response.
    pub fn add_layer<L: ConversationLayer + 'static>(&mut self, layer: L) {
        self.layer_stack.add(layer);
    }

    /// Create a LayerContext from the current agent state
    fn create_layer_context(&self) -> LayerContext {
        LayerContext::builder()
            .agent_id(self.id.to_string())
            .agent_type(self.agent_type)
            .message_count(self.messages.len())
            .turn_count(self.messages.len() / 2)
            .token_count(self.estimate_token_count())
            .build()
    }

    /// Estimate total token count from conversation messages
    fn estimate_token_count(&self) -> usize {
        self.messages
            .iter()
            .map(|m| LayerContext::estimate_tokens(&m.content))
            .sum()
    }

    /// Configure the agent with a logging middleware layer
    ///
    /// This adds the LoggingLayer which logs all message flow for debugging.
    /// Returns self for method chaining.
    pub fn with_logging_layer(mut self) -> Self {
        use crate::app::agent_framework::middleware::layers::LoggingLayer;
        self.layer_stack.add(LoggingLayer::with_defaults());
        self
    }

    /// Configure the agent with recommended middleware layers
    ///
    /// This adds:
    /// - LoggingLayer for debugging
    /// - TokenTrackingLayer for monitoring token usage (default 100k token threshold)
    ///
    /// Returns self for method chaining.
    pub fn with_recommended_layers(mut self) -> Self {
        use crate::app::agent_framework::middleware::layers::{LoggingLayer, TokenTrackingLayer};
        self.layer_stack.add(LoggingLayer::with_defaults());
        self.layer_stack.add(TokenTrackingLayer::with_defaults());
        self
    }

    // ========== End Middleware API ==========

    // ========== Cancellation API ==========

    /// Cancel the current agent execution
    ///
    /// This signals the stood agent's event loop to stop at the next
    /// cancellation check point (between cycles). The execution will
    /// return with a "cancelled" status.
    ///
    /// Returns true if cancellation was requested, false if no token available.
    pub fn cancel(&mut self) -> bool {
        if let Some(token) = &self.cancel_token {
            token.cancel();
            self.logger.log_system_message(
                &self.agent_type,
                "Cancellation requested - stopping agent execution",
            );
            true
        } else {
            self.logger.log_system_message(
                &self.agent_type,
                "Cancel requested but no cancellation token available",
            );
            false
        }
    }

    /// Check if cancellation is available for this agent
    ///
    /// Returns true if the agent was initialized with cancellation support
    /// and has not been reset.
    pub fn can_cancel(&self) -> bool {
        self.cancel_token.is_some()
    }

    /// Check if cancellation has been requested
    ///
    /// Returns true if cancel() was called and the token is cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token
            .as_ref()
            .map(|t| t.is_cancelled())
            .unwrap_or(false)
    }

    // ========== End Cancellation API ==========

    /// Get reference to the agent's logger
    pub fn logger(&self) -> &Arc<AgentLogger> {
        &self.logger
    }

    // ========== Message Injection API ==========

    /// Get reference to the message injector
    pub fn message_injector(&self) -> &MessageInjector {
        &self.message_injector
    }

    /// Get mutable reference to the message injector
    pub fn message_injector_mut(&mut self) -> &mut MessageInjector {
        &mut self.message_injector
    }

    /// Queue an injection for later processing
    ///
    /// The injection will be triggered based on its trigger condition
    /// and processed on the next poll_response() call.
    pub fn queue_injection(&mut self, injection_type: InjectionType, trigger: InjectionTrigger) {
        self.logger.log_system_message(
            &self.agent_type,
            &format!(
                "Queueing injection: {} with trigger {:?}",
                injection_type.label(),
                trigger
            ),
        );
        self.message_injector.queue_injection(injection_type, trigger);
    }

    /// Queue an immediate injection
    ///
    /// The message will be injected on the next poll_response() call
    /// after the current processing completes.
    pub fn queue_immediate_injection(&mut self, injection_type: InjectionType) {
        self.queue_injection(injection_type, InjectionTrigger::Immediate);
    }

    /// Inject a message programmatically (bypassing user input)
    ///
    /// This is equivalent to send_message() but is marked as a system injection
    /// in the logs. Use this for automated follow-ups, context management, etc.
    pub fn inject_message(&mut self, message: String) {
        self.logger.log_system_message(
            &self.agent_type,
            &format!("Injecting message: {}", if message.len() > 100 {
                format!("{}...", &message[..100])
            } else {
                message.clone()
            }),
        );

        // Use the same send mechanism as user messages
        self.send_message(message);
    }

    /// Check if there are pending injections
    pub fn has_pending_injections(&self) -> bool {
        self.message_injector.has_pending()
    }

    /// Process pending injections with the given context
    ///
    /// Returns the injection message if one is ready, otherwise None.
    /// Call this from poll_response() or after tool completions.
    fn check_and_process_injection(&mut self, context: &InjectionContext) -> Option<String> {
        self.message_injector.check_triggers(context)
    }

    // ========== End Message Injection API ==========

    /// Get the current stood log level for this agent
    pub fn stood_log_level(&self) -> StoodLogLevel {
        self.stood_log_level
    }

    /// Set the stood log level for this agent
    ///
    /// This also resets the stood agent so the new log level takes effect
    /// on the next message send.
    pub fn set_stood_log_level(&mut self, level: StoodLogLevel) {
        let old_level = self.stood_log_level;
        if old_level == level {
            return; // No change needed
        }

        self.stood_log_level = level;

        // Log the change
        self.logger.log_stood_level_changed(
            &self.agent_type,
            old_level.display_name(),
            level.display_name(),
        );

        tracing::info!(
            target: "agent::stood_log_level",
            old_level = %old_level.display_name(),
            new_level = %level.display_name(),
            "Stood log level changed"
        );

        // Reset the stood agent so new log level takes effect
        self.reset_stood_agent();
    }

    /// Reset the stood agent for reinitialization
    ///
    /// This clears the stood agent instance and cancellation token.
    /// The agent will be re-created with current settings on the next
    /// message send or initialize() call.
    pub fn reset_stood_agent(&mut self) {
        *self.stood_agent.lock().unwrap() = None;
        self.cancel_token = None; // Clear token - will be recaptured on reinit
        self.logger.log_system_message(
            &self.agent_type,
            "Stood agent reset - will reinitialize on next message",
        );
    }

    /// Get tools based on agent type
    ///
    /// Tool configuration:
    /// - TaskManager: think, todo-write, todo-read, start-task tools
    /// - TaskWorker: execute_javascript tool
    fn get_tools_for_type(&self) -> Vec<Box<dyn stood::tools::Tool>> {
        match self.agent_type {
            AgentType::TaskManager => {
                // Task-manager agents get planning and orchestration tools
                let _todo_list_ref = Arc::clone(&self.todo_list_shared);

                // Think tool (no callback needed)
                let think_tool = Box::new(crate::app::agent_framework::tools::ThinkTool::new());

                // Todo tools commented out - not needed for task management
                // // Todo-write tool with callback
                // let todo_write_callback = {
                //     let todo_ref = Arc::clone(&todo_list_ref);
                //     move |todos: Vec<TodoItem>| {
                //         *todo_ref.lock().unwrap() = todos;
                //     }
                // };
                // let todo_write_tool = Box::new(
                //     crate::app::agent_framework::tools::TodoWriteTool::new()
                //         .with_callback(todo_write_callback),
                // );

                // // Todo-read tool with callback
                // let todo_read_callback = {
                //     let todo_ref = Arc::clone(&todo_list_ref);
                //     move || todo_ref.lock().unwrap().clone()
                // };
                // let todo_read_tool = Box::new(
                //     crate::app::agent_framework::tools::TodoReadTool::new()
                //         .with_callback(todo_read_callback),
                // );

                // Start-task tool for spawning worker agents
                let start_task_tool =
                    Box::new(crate::app::agent_framework::tools::StartTaskTool::new());

                vec![
                    think_tool as Box<dyn stood::tools::Tool>,
                    // todo_write_tool as Box<dyn stood::tools::Tool>,
                    // todo_read_tool as Box<dyn stood::tools::Tool>,
                    start_task_tool as Box<dyn stood::tools::Tool>,
                ]
            }
            AgentType::TaskWorker { .. } => {
                vec![Box::new(
                    crate::app::agent_framework::tools::ExecuteJavaScriptTool::new(),
                )]
            }
        }
    }

    /// Get system prompt based on agent type
    fn get_system_prompt_for_type(&self) -> String {
        use chrono::Utc;

        let prompt = match self.agent_type {
            AgentType::TaskManager => crate::app::agent_framework::TASK_MANAGER_PROMPT.to_string(),
            AgentType::TaskWorker { .. } => {
                crate::app::agent_framework::TASK_WORKER_PROMPT.to_string()
            }
        };

        // Replace {{CURRENT_DATETIME}} placeholder with actual current date and time
        let current_datetime = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        prompt.replace("{{CURRENT_DATETIME}}", &current_datetime)
    }

    /// Create and configure the stood Agent
    ///
    /// This method:
    /// - Retrieves AWS credentials from Identity Center
    /// - Configures Bedrock provider with Claude Haiku 3
    /// - Registers execute_javascript tool
    /// - Disables streaming to avoid hang issues
    fn create_stood_agent(
        &self,
        aws_identity: &mut crate::app::aws_identity::AwsIdentityCenter,
        agent_logging_enabled: bool,
    ) -> Result<stood::agent::Agent, String> {
        use stood::agent::{Agent, EventLoopConfig};
        use stood::llm::Bedrock;
        use stood::telemetry::{AwsCredentialSource, TelemetryConfig};

        // Get AWS credentials
        let creds = aws_identity
            .get_default_role_credentials()
            .map_err(|e| format!("Failed to get AWS credentials: {}", e))?;

        let access_key = creds.access_key_id;
        let secret_key = creds.secret_access_key;
        let session_token = creds.session_token;
        let region = "us-east-1".to_string();

        // Determine agent naming
        let (agent_type_name, agent_name) = match &self.agent_type {
            AgentType::TaskManager => ("manager", "awsdash-manager"),
            AgentType::TaskWorker { .. } => ("worker", "awsdash-worker"),
        };
        let agent_id = format!("awsdash-{}-{}", agent_type_name, self.id);

        // Configure telemetry programmatically (no environment variables)
        let telemetry_config = if agent_logging_enabled {
            // Log which account is being used for telemetry
            if let Some(account_id) = aws_identity.get_selected_account_id() {
                tracing::info!(
                    "Agent Logging enabled for account: {}, region: {}",
                    account_id,
                    region
                );
            }
            TelemetryConfig::cloudwatch(&region)
                .with_service_name("awsdash")
                .with_agent_id(&agent_id)
                .with_content_capture(true) // Enable content capture for debugging
                .with_credentials(AwsCredentialSource::Explicit {
                    access_key_id: access_key.clone(),
                    secret_access_key: secret_key.clone(),
                    session_token: session_token.clone(),
                })
        } else {
            TelemetryConfig::disabled()
        };

        self.logger.log_system_message(
            &self.agent_type,
            &format!(
                "Telemetry config: enabled={}, agent_id={}",
                telemetry_config.is_enabled(),
                agent_id
            ),
        );

        // Set global credentials for execute_javascript tool
        crate::app::agent_framework::set_global_aws_credentials(
            access_key.clone(),
            secret_key.clone(),
            session_token.clone(),
            region.clone(),
        );

        // Log agent initialization
        self.logger
            .log_system_message(&self.agent_type, "Agent initialization started");

        // Configure agent with selected model
        let system_prompt = self.get_system_prompt_for_type();
        let tools = self.get_tools_for_type();

        self.logger.log_system_message(
            &self.agent_type,
            &format!("Using model: {}", self.metadata.model),
        );

        // Configure event loop with appropriate limits based on agent type
        // TaskManager (orchestration): 1000 tool iterations, 100 cycles
        // TaskWorker: 200 tool iterations, 20 cycles
        let event_loop_config = match self.agent_type {
            AgentType::TaskManager => EventLoopConfig {
                max_cycles: 100,
                max_tool_iterations: 1000,
                ..Default::default()
            },
            AgentType::TaskWorker { .. } => EventLoopConfig {
                max_cycles: 20,
                max_tool_iterations: 200,
                ..Default::default()
            },
        };

        self.logger.log_system_message(
            &self.agent_type,
            &format!(
                "Event loop config: max_cycles={}, max_tool_iterations={}",
                event_loop_config.max_cycles, event_loop_config.max_tool_iterations
            ),
        );

        // Build agent with selected model (match on enum to get concrete type)
        // Enable cancellation support for all models
        // Include agent naming and telemetry for CloudWatch Gen AI Observability
        use crate::app::agent_framework::AgentModel;
        let agent_builder = match self.metadata.model {
            AgentModel::ClaudeSonnet45 => Agent::builder()
                .name(agent_name)
                .with_id(&agent_id)
                .with_telemetry(telemetry_config.clone())
                .model(Bedrock::ClaudeSonnet45)
                .system_prompt(&system_prompt)
                .with_streaming(false)
                .with_cancellation()
                .with_event_loop_config(event_loop_config.clone())
                .with_credentials(
                    access_key.clone(),
                    secret_key.clone(),
                    session_token.clone(),
                    region.clone(),
                )
                .tools(tools),
            AgentModel::ClaudeHaiku45 => Agent::builder()
                .name(agent_name)
                .with_id(&agent_id)
                .with_telemetry(telemetry_config.clone())
                .model(Bedrock::ClaudeHaiku45)
                .system_prompt(&system_prompt)
                .with_streaming(false)
                .with_cancellation()
                .with_event_loop_config(event_loop_config.clone())
                .with_credentials(
                    access_key.clone(),
                    secret_key.clone(),
                    session_token.clone(),
                    region.clone(),
                )
                .tools(self.get_tools_for_type()),
            AgentModel::ClaudeOpus45 => Agent::builder()
                .name(agent_name)
                .with_id(&agent_id)
                .with_telemetry(telemetry_config.clone())
                .model(Bedrock::ClaudeOpus45)
                .system_prompt(&system_prompt)
                .with_streaming(false)
                .with_cancellation()
                .with_event_loop_config(event_loop_config.clone())
                .with_credentials(
                    access_key.clone(),
                    secret_key.clone(),
                    session_token.clone(),
                    region.clone(),
                )
                .tools(self.get_tools_for_type()),
            AgentModel::NovaPro => Agent::builder()
                .name(agent_name)
                .with_id(&agent_id)
                .with_telemetry(telemetry_config.clone())
                .model(Bedrock::NovaPro)
                .system_prompt(&system_prompt)
                .with_streaming(false)
                .with_cancellation()
                .with_event_loop_config(event_loop_config.clone())
                .with_credentials(
                    access_key.clone(),
                    secret_key.clone(),
                    session_token.clone(),
                    region.clone(),
                )
                .tools(self.get_tools_for_type()),
            AgentModel::NovaLite => Agent::builder()
                .name(agent_name)
                .with_id(&agent_id)
                .with_telemetry(telemetry_config.clone())
                .model(Bedrock::NovaLite)
                .system_prompt(&system_prompt)
                .with_streaming(false)
                .with_cancellation()
                .with_event_loop_config(event_loop_config.clone())
                .with_credentials(
                    access_key.clone(),
                    secret_key.clone(),
                    session_token.clone(),
                    region.clone(),
                )
                .tools(self.get_tools_for_type()),
        };

        // For TaskWorker agents, add callback handler to capture tool events
        // and forward them to the UI for inline progress display
        let agent_builder = if let AgentType::TaskWorker { parent_id } = &self.agent_type {
            use crate::app::agent_framework::WorkerProgressCallbackHandler;
            self.logger.log_system_message(
                &self.agent_type,
                "Adding worker progress callback handler",
            );
            agent_builder.with_callback_handler(WorkerProgressCallbackHandler::new(self.id, *parent_id))
        } else {
            agent_builder
        };

        let agent = self
            .runtime
            .block_on(async { agent_builder.build().await })
            .map_err(|e| format!("Failed to build agent: {}", e))?;

        self.logger
            .log_system_message(&self.agent_type, "Agent successfully created");
        Ok(agent)
    }

    /// Send a user message and execute the agent in background
    ///
    /// This method:
    /// - Processes message through middleware layers (can modify or abort)
    /// - Adds user message to conversation
    /// - Lazily initializes stood agent if needed
    /// - Spawns background thread for execution
    /// - Sets processing flag
    pub fn send_message(&mut self, user_message: String) {
        // === Pre-send middleware processing ===
        let ctx = self.create_layer_context();
        let processed_message = match self.layer_stack.process_pre_send(&user_message, &ctx) {
            Ok(msg) => msg,
            Err(LayerError::Abort(reason)) => {
                self.logger.log_system_message(
                    &self.agent_type,
                    &format!("Message send aborted by middleware: {}", reason),
                );
                return; // Don't send if middleware aborts
            }
            Err(e) => {
                self.logger.log_system_message(
                    &self.agent_type,
                    &format!("Middleware error (continuing): {}", e),
                );
                user_message.clone() // Continue with original on non-fatal error
            }
        };
        // === End pre-send middleware processing ===

        // Log message being sent (with preview)
        let message_preview = if processed_message.len() > 100 {
            format!("{}...", &processed_message[..100])
        } else {
            processed_message.clone()
        };
        self.logger.log_system_message(
            &self.agent_type,
            &format!("Sending message: {}", message_preview),
        );

        // Add user message to conversation (use original for display, processed for sending)
        self.messages
            .push_back(ConversationMessage::user(user_message.clone()));
        self.processing = true;
        self.processing_phase = ProcessingPhase::Thinking;
        self.status_message = Some("Processing...".to_string());

        // Log message
        self.logger
            .log_user_message(&self.agent_type, &processed_message);

        // Clone what we need for the background thread
        let stood_agent = Arc::clone(&self.stood_agent);
        let sender = self.response_channel.0.clone();
        let logger = Arc::clone(&self.logger);
        let runtime = Arc::clone(&self.runtime);
        let agent_id = self.id;
        let agent_type = self.agent_type;
        let stood_log_level = self.stood_log_level;
        let message_for_agent = processed_message; // Use processed message for agent

        // Spawn background thread
        std::thread::spawn(move || {
            // Set the current agent logger for this thread (so tools can log to it)
            crate::app::agent_framework::agent_logger::set_current_agent_logger(Some(Arc::clone(
                &logger,
            )));

            // Set the current agent ID for this thread (so tools can access parent agent context)
            crate::app::agent_framework::set_current_agent_id(agent_id);

            // Set the current agent type for this thread (so tools can pass it to logger methods)
            crate::app::agent_framework::set_current_agent_type(agent_type);

            // Set the stood log level for this thread (for AgentTracingLayer to filter stood traces)
            crate::app::agent_framework::agent_tracing::set_current_log_level(stood_log_level);

            // Execute agent in tokio runtime
            // Note: We intentionally hold the MutexGuard across await because the stood agent
            // must remain locked during execution. This is safe because only one thread
            // executes the agent at a time via the background thread spawned above.
            #[allow(clippy::await_holding_lock)]
            runtime.block_on(async move {
                logger.log_system_message(&agent_type, "Background execution started");

                // Lazy initialization of stood agent
                let mut agent_guard = stood_agent.lock().unwrap();
                if agent_guard.is_none() {
                    logger.log_system_message(&agent_type, "Creating stood agent (lazy initialization)");

                    // Note: We can't create the agent here without aws_identity
                    // This will be handled differently - agent must be created before first send
                    drop(agent_guard);
                    let _ = sender.send(ConversationResponse::Error(
                        "Agent not initialized. This is a bug.".to_string()
                    ));
                    return;
                }

                let agent = agent_guard.as_mut().unwrap();

                // Prepend critical instructions to user message
                let instruction_template = "\
<critical_instructions>
When performing calculations or numerical analysis:
1. Always use the actual data returned from tool queries as your source data
2. Show your calculation process explicitly, including:
   - The raw numbers you're using from the query results
   - The mathematical operations you're performing
   - The units you're working with and any unit conversions
3. If you cannot perform a calculation because:
   - The data is missing
   - The data format is unclear
   - You're unsure about the mathematical approach
   - You don't have the right tools
   Then explicitly state:
   'I cannot calculate this because [specific reason]. To perform this calculation, I would need [missing information/tools/clarification].'
4. Never make assumptions about numerical values - only use values explicitly present in the query results

Query result presentation:
1. Show me the exact query results without interpretation
2. Only include resources that are explicitly returned in the query data
</critical_instructions>

";
                let full_message = format!("{}{}", instruction_template, message_for_agent);

                // Execute agent with full message (instructions + user query)
                logger.log_system_message(&agent_type, "Executing agent...");
                match agent.execute(&full_message).await {
                    Ok(_) => {
                        logger.log_system_message(&agent_type, "Agent execution completed");

                        // Get final response from conversation
                        if let Some(last_message) = agent.conversation().messages().last() {
                            let is_assistant = last_message.role == stood::types::MessageRole::Assistant;

                            if !is_assistant {
                                // stood execute() succeeded but didn't add an assistant response
                                let error_msg = "Model did not generate a response. This may indicate an API error or credential issue. Check the application logs for details.".to_string();
                                logger.log_error(&agent_type, "No assistant response generated");
                                let _ = sender.send(ConversationResponse::Error(error_msg));
                            } else if let Some(text) = last_message.text() {
                                logger.log_assistant_response(&agent_type, &text);
                                let _ = sender.send(ConversationResponse::Success(text.to_string()));
                            } else {
                                let error_msg = "Agent response had no text content".to_string();
                                logger.log_error(&agent_type, &error_msg);
                                let _ = sender.send(ConversationResponse::Error(error_msg));
                            }
                        } else {
                            let error_msg = "Agent response was empty".to_string();
                            logger.log_error(&agent_type, &error_msg);
                            let _ = sender.send(ConversationResponse::Error(error_msg));
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Agent execution failed: {}", e);
                        logger.log_error(&agent_type, &error_msg);
                        let _ = sender.send(ConversationResponse::Error(error_msg));
                    }
                }
            });
        });
    }

    /// Poll for responses from background thread
    ///
    /// Call this from the UI thread (every frame) to check for responses.
    /// Returns true if a response was received.
    ///
    /// Also handles deferred message injections - if an injection is queued
    /// and the agent is not currently processing, the injection will be sent.
    pub fn poll_response(&mut self) -> bool {
        // Handle deferred injections when not processing
        if !self.processing && self.has_pending_injection {
            if let Some(injection_msg) = self.deferred_injection.take() {
                self.logger.log_system_message(
                    &self.agent_type,
                    "Processing deferred injection",
                );
                self.has_pending_injection = false;
                self.inject_message(injection_msg);
                // Return true to signal that we've started a new processing cycle
                return true;
            }
            self.has_pending_injection = false;
        }

        match self.response_channel.1.try_recv() {
            Ok(response) => {
                let response_received_at = std::time::Instant::now();
                self.logger.log_system_message(
                    &self.agent_type,
                    "Response received from background thread",
                );
                self.processing = false;
                self.processing_phase = ProcessingPhase::Idle;
                self.status_message = None;

                match response {
                    ConversationResponse::Success(text) => {
                        // === Post-response middleware processing ===
                        let ctx = self.create_layer_context();
                        let (final_text, middleware_injections) =
                            match self.layer_stack.process_post_response(&text, &ctx) {
                                Ok(result) => {
                                    if result.suppress {
                                        // Don't add to messages if suppressed
                                        self.logger.log_system_message(
                                            &self.agent_type,
                                            "Response suppressed by middleware",
                                        );
                                        (None, result.injections)
                                    } else if result.was_modified {
                                        (Some(result.final_response), result.injections)
                                    } else {
                                        (Some(text.clone()), result.injections)
                                    }
                                }
                                Err(e) => {
                                    self.logger.log_system_message(
                                        &self.agent_type,
                                        &format!("Post-response middleware error: {}", e),
                                    );
                                    (Some(text.clone()), vec![])
                                }
                            };
                        // === End post-response middleware processing ===

                        // Add message to conversation (if not suppressed)
                        if let Some(response_text) = final_text {
                            // Log successful response
                            let text_preview = if response_text.len() > 200 {
                                format!("{}...", &response_text[..200])
                            } else {
                                response_text.clone()
                            };
                            self.logger.log_system_message(
                                &self.agent_type,
                                &format!("Success response: {}", text_preview),
                            );
                            let msg = ConversationMessage::assistant(response_text);
                            let message_timestamp = msg.timestamp;
                            self.messages.push_back(msg);

                            log::info!(
                                "[TIMING] Response added to messages at {:?} (message timestamp: {})",
                                response_received_at,
                                message_timestamp.format("%H:%M:%S%.3f")
                            );
                            self.logger.log_system_message(
                                &self.agent_type,
                                &format!(
                                    "Response added at {} (timestamp: {})",
                                    response_received_at.elapsed().as_millis(),
                                    message_timestamp.format("%H:%M:%S%.3f")
                                ),
                            );
                        }

                        // Queue middleware injections
                        if let Some(first_injection) = middleware_injections.into_iter().next() {
                            self.logger.log_system_message(
                                &self.agent_type,
                                &format!(
                                    "Middleware injection queued: {}",
                                    if first_injection.len() > 50 {
                                        format!("{}...", &first_injection[..50])
                                    } else {
                                        first_injection.clone()
                                    }
                                ),
                            );
                            self.has_pending_injection = true;
                            self.deferred_injection = Some(first_injection);
                        } else {
                            // Check for AfterResponse injections from MessageInjector
                            let context = InjectionContext::after_response();
                            if let Some(injection_msg) =
                                self.check_and_process_injection(&context)
                            {
                                self.logger.log_system_message(
                                    &self.agent_type,
                                    &format!(
                                        "AfterResponse injection triggered: {}",
                                        if injection_msg.len() > 50 {
                                            format!("{}...", &injection_msg[..50])
                                        } else {
                                            injection_msg.clone()
                                        }
                                    ),
                                );
                                // Store the injection message to be sent on next poll
                                self.has_pending_injection = true;
                                self.deferred_injection = Some(injection_msg);
                            }
                        }
                    }
                    ConversationResponse::Error(error) => {
                        self.messages
                            .push_back(ConversationMessage::assistant(format!("Error: {}", error)));
                        self.logger
                            .log_error(&self.agent_type, &format!("Agent error: {}", error));
                        self.status = AgentStatus::Failed(error);
                    }
                    ConversationResponse::StatusUpdate(status) => {
                        // Update status message without finishing processing
                        self.status_message = Some(status);
                        self.processing = true; // Keep processing state
                        // Status updates typically indicate tool execution in progress
                        self.processing_phase = ProcessingPhase::ExecutingTool("Running tool".to_string());
                        return false; // Don't mark as complete, continue polling
                    }
                }
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.processing = false;
                self.processing_phase = ProcessingPhase::Idle;
                self.status_message = None;
                self.logger
                    .log_error(&self.agent_type, "Response channel disconnected");
                false
            }
        }
    }

    /// Initialize the stood agent (must be called before first send_message)
    ///
    /// This is separate from construction to allow async credential loading.
    /// Also captures the cancellation token for Stop button support.
    pub fn initialize(
        &mut self,
        aws_identity: &mut crate::app::aws_identity::AwsIdentityCenter,
        agent_logging_enabled: bool,
    ) -> Result<(), String> {
        let agent = self.create_stood_agent(aws_identity, agent_logging_enabled)?;

        // Capture the cancellation token for Stop button support
        self.cancel_token = agent.cancellation_token();
        if self.cancel_token.is_some() {
            self.logger.log_system_message(
                &self.agent_type,
                "Cancellation token captured - Stop button enabled",
            );
        }

        let mut guard = self.stood_agent.lock().unwrap();
        *guard = Some(agent);

        self.logger
            .log_system_message(&self.agent_type, "Agent fully initialized and ready");
        Ok(())
    }

    /// Get mutable reference to metadata (for external updates)
    pub fn metadata_mut(&mut self) -> &mut AgentMetadata {
        &mut self.metadata
    }

    /// Clear all messages and reset the stood agent's conversation
    ///
    /// This clears the message history and resets the stood agent,
    /// effectively starting a fresh conversation while keeping the agent instance.
    pub fn clear_conversation(&mut self) {
        // Clear message history
        self.messages.clear();

        // Log the clear operation
        self.logger
            .log_system_message(&self.agent_type, "Conversation cleared by user");

        // Reset stood agent and token (will be re-initialized on next message)
        *self.stood_agent.lock().unwrap() = None;
        self.cancel_token = None;

        // Reset processing state
        self.processing = false;
        self.status_message = None;

        // Update metadata timestamp
        self.metadata.updated_at = chrono::Utc::now();
    }

    /// Terminate the agent and clean up resources
    ///
    /// This cancels any ongoing execution and marks the agent as terminated.
    pub fn terminate(&mut self) {
        // Cancel any ongoing execution
        if let Some(token) = &self.cancel_token {
            token.cancel();
            self.logger
                .log_system_message(&self.agent_type, "Cancelled ongoing execution");
        }

        self.status = AgentStatus::Cancelled;
        self.processing = false;
        self.processing_phase = ProcessingPhase::Idle;
        self.cancel_token = None;
        self.logger
            .log_system_message(&self.agent_type, "Agent terminated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::TodoStatus;
    use chrono::Utc;

    fn create_test_metadata() -> AgentMetadata {
        use crate::app::agent_framework::AgentModel;
        AgentMetadata {
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            model: AgentModel::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_create_task_manager_agent() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata.clone(), AgentType::TaskManager);

        assert_eq!(agent.metadata().name, metadata.name);
        assert_eq!(agent.agent_type(), AgentType::TaskManager);
        assert_eq!(agent.status(), &AgentStatus::Running);
        assert!(!agent.is_processing());
        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_create_task_worker_agent() {
        let metadata = create_test_metadata();
        let parent_id = AgentId::new();
        let agent = AgentInstance::new(metadata.clone(), AgentType::TaskWorker { parent_id });

        assert_eq!(agent.metadata().name, metadata.name);
        assert_eq!(agent.agent_type(), AgentType::TaskWorker { parent_id });
        assert_eq!(agent.agent_type().parent_id(), Some(parent_id));
        assert!(!agent.agent_type().is_task_manager());
    }

    #[test]
    fn test_agent_type_accessor() {
        let metadata = create_test_metadata();

        let task_manager = AgentInstance::new(metadata.clone(), AgentType::TaskManager);
        assert!(task_manager.agent_type().is_task_manager());

        let parent_id = AgentId::new();
        let task_worker = AgentInstance::new(metadata, AgentType::TaskWorker { parent_id });
        assert!(!task_worker.agent_type().is_task_manager());
        assert_eq!(task_worker.agent_type().parent_id(), Some(parent_id));
    }

    #[test]
    fn test_agent_id_unique() {
        let metadata = create_test_metadata();
        let agent1 = AgentInstance::new(metadata.clone(), AgentType::TaskManager);
        let agent2 = AgentInstance::new(metadata, AgentType::TaskManager);

        assert_ne!(agent1.id(), agent2.id());
    }

    #[test]
    fn test_agent_lifecycle() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager);

        assert_eq!(agent.status(), &AgentStatus::Running);
        assert!(!agent.is_processing());
    }

    #[test]
    fn test_agent_messages_empty_on_creation() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager);

        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new();
        let display = format!("{}", id);
        // Should be a valid UUID string
        assert_eq!(display.len(), 36); // UUID format: 8-4-4-4-12
    }

    #[test]
    fn test_todo_list_operations() {
        let metadata = create_test_metadata();
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // Initially empty
        assert_eq!(agent.todo_list().len(), 0);

        // Add todos
        let todos = vec![
            TodoItem::new(
                "Task 1".to_string(),
                "Doing task 1".to_string(),
                TodoStatus::Pending,
            ),
            TodoItem::new(
                "Task 2".to_string(),
                "Doing task 2".to_string(),
                TodoStatus::InProgress,
            ),
        ];
        agent.set_todo_list(todos.clone());
        assert_eq!(agent.todo_list().len(), 2);
        assert_eq!(agent.todo_list()[0].content, "Task 1");

        // Clear todos
        agent.clear_todo_list();
        assert_eq!(agent.todo_list().len(), 0);
    }

    #[test]
    fn test_task_manager_has_planning_tools() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager);
        let tools = agent.get_tools_for_type();

        // TaskManager should have 2 tools: think, start_task
        // (todo_write and todo_read are commented out as of this implementation)
        assert_eq!(tools.len(), 2);

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(tool_names.contains(&"think"));
        assert!(tool_names.contains(&"start_task"));
    }

    #[test]
    fn test_task_worker_has_execution_tools() {
        let metadata = create_test_metadata();
        let parent_id = AgentId::new();
        let agent = AgentInstance::new(metadata, AgentType::TaskWorker { parent_id });
        let tools = agent.get_tools_for_type();

        // TaskWorker should have 1 tool: execute_javascript
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "execute_javascript");
    }

    // ========== Middleware Integration Tests ==========

    #[test]
    fn test_layer_stack_initialized() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // LayerStack should be initialized but empty
        assert!(agent.layer_stack().is_empty());
        assert!(agent.layer_stack().is_enabled());
    }

    #[test]
    fn test_add_layer() {
        use crate::app::agent_framework::middleware::layers::LoggingLayer;

        let metadata = create_test_metadata();
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);

        assert!(agent.layer_stack().is_empty());

        agent.add_layer(LoggingLayer::with_defaults());

        assert_eq!(agent.layer_stack().len(), 1);
        assert_eq!(agent.layer_stack().layer_names(), vec!["Logging"]);
    }

    #[test]
    fn test_with_logging_layer() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager).with_logging_layer();

        assert_eq!(agent.layer_stack().len(), 1);
        assert_eq!(agent.layer_stack().layer_names(), vec!["Logging"]);
    }

    #[test]
    fn test_with_recommended_layers() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager).with_recommended_layers();

        assert_eq!(agent.layer_stack().len(), 2);
        assert_eq!(
            agent.layer_stack().layer_names(),
            vec!["Logging", "TokenTracking"]
        );
    }

    #[test]
    fn test_create_layer_context() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager);

        let ctx = agent.create_layer_context();

        assert!(!ctx.agent_id.is_empty());
        assert!(matches!(ctx.agent_type, AgentType::TaskManager));
        assert_eq!(ctx.message_count, 0);
        assert_eq!(ctx.turn_count, 0);
        assert_eq!(ctx.token_count, 0);
    }

    #[test]
    fn test_estimate_token_count_empty() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager);

        assert_eq!(agent.estimate_token_count(), 0);
    }

    // ========== Cancellation API Tests ==========

    #[test]
    fn test_cancel_token_none_before_init() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // Before initialization, cancel token should be None
        assert!(!agent.can_cancel());
    }

    #[test]
    fn test_is_cancelled_false_initially() {
        let metadata = create_test_metadata();
        let agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // Should return false when no token available
        assert!(!agent.is_cancelled());
    }

    #[test]
    fn test_cancel_returns_false_without_token() {
        let metadata = create_test_metadata();
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // Cancel should return false when no token available
        assert!(!agent.cancel());
        assert!(!agent.can_cancel());
    }

    #[test]
    fn test_cancel_token_cleared_on_reset() {
        let metadata = create_test_metadata();
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // Manually set a token for testing
        agent.cancel_token = Some(CancellationToken::new());
        assert!(agent.can_cancel());

        // Reset should clear the token
        agent.reset_stood_agent();
        assert!(!agent.can_cancel());
    }

    #[test]
    fn test_cancel_token_cleared_on_clear_conversation() {
        let metadata = create_test_metadata();
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // Manually set a token for testing
        agent.cancel_token = Some(CancellationToken::new());
        assert!(agent.can_cancel());

        // Clear conversation should clear the token
        agent.clear_conversation();
        assert!(!agent.can_cancel());
    }

    #[test]
    fn test_cancel_with_token() {
        let metadata = create_test_metadata();
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // Manually set a token for testing
        let token = CancellationToken::new();
        agent.cancel_token = Some(token.clone());

        assert!(agent.can_cancel());
        assert!(!agent.is_cancelled());

        // Cancel should work
        assert!(agent.cancel());
        assert!(agent.is_cancelled());
        assert!(token.is_cancelled()); // Original token should also be cancelled
    }

    #[test]
    fn test_terminate_cancels_token() {
        let metadata = create_test_metadata();
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);

        // Manually set a token for testing
        let token = CancellationToken::new();
        agent.cancel_token = Some(token.clone());

        // Terminate should cancel the token
        agent.terminate();

        assert!(token.is_cancelled());
        assert!(!agent.can_cancel()); // Token should be cleared after terminate
    }
}
