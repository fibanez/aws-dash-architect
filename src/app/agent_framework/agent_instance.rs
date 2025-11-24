//! Standalone Agent Instance
//!
//! Simplified agent implementation that uses stood library directly.
//! This version uses stood library directly for simplicity and reliability.

#![warn(clippy::all, rust_2018_idioms)]

use std::collections::VecDeque;
use std::sync::{mpsc, Arc, Mutex};

use crate::app::agent_framework::agent_logger::AgentLogger;
use crate::app::agent_framework::agent_types::{AgentId, AgentMetadata, AgentStatus, AgentType};
use crate::app::agent_framework::conversation::{ConversationMessage, ConversationResponse};
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
        logger.log_system_message(&agent_type, &format!(
            "\n====== Worker Agent: {} ({}) ======",
            metadata.name, id
        ));
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

    /// Get reference to the agent's logger
    pub fn logger(&self) -> &Arc<AgentLogger> {
        &self.logger
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
                vec![crate::app::agent_framework::execute_javascript_tool()]
            }
        }
    }

    /// Get system prompt based on agent type
    fn get_system_prompt_for_type(&self) -> String {
        use chrono::Utc;

        let prompt = match self.agent_type {
            AgentType::TaskManager => {
                crate::app::agent_framework::TASK_MANAGER_PROMPT.to_string()
            }
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
    ) -> Result<stood::agent::Agent, String> {
        use stood::agent::Agent;
        use stood::llm::Bedrock;

        // Get AWS credentials
        let creds = aws_identity
            .get_default_role_credentials()
            .map_err(|e| format!("Failed to get AWS credentials: {}", e))?;

        let access_key = creds.access_key_id;
        let secret_key = creds.secret_access_key;
        let session_token = creds.session_token;
        let region = "us-east-1".to_string(); // Default region

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
        self.logger
            .log_system_message(&self.agent_type, &format!("Using model: {}", self.metadata.model_id));

        // Configure agent with Claude Sonnet 3.5
        // Note: We can't use async in this context, so build synchronously

        // Get system prompt based on agent type
        let system_prompt = self.get_system_prompt_for_type();

        // Debug: Log the system prompt
        log::info!(
            "System prompt length: {} chars",
            system_prompt.len()
        );
        log::info!(
            "System prompt preview: {}...",
            &system_prompt.chars().take(200).collect::<String>()
        );

        let agent_builder = Agent::builder()
            .model(Bedrock::Claude35Sonnet)
            .system_prompt(&system_prompt)
            .with_streaming(false) // Disable streaming to avoid hang issues
            .with_credentials(access_key, secret_key, session_token, region)
            .tools(self.get_tools_for_type());

        // We need to build async, so wrap in runtime
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
    /// - Adds user message to conversation
    /// - Lazily initializes stood agent if needed
    /// - Spawns background thread for execution
    /// - Sets processing flag
    pub fn send_message(&mut self, user_message: String) {
        // Log message being sent (with preview)
        let message_preview = if user_message.len() > 100 {
            format!("{}...", &user_message[..100])
        } else {
            user_message.clone()
        };
        self.logger
            .log_system_message(&self.agent_type, &format!("Sending message: {}", message_preview));

        // Add user message to conversation
        self.messages
            .push_back(ConversationMessage::user(user_message.clone()));
        self.processing = true;
        self.status_message = Some("Processing...".to_string());

        // Log message
        self.logger.log_user_message(&self.agent_type, &user_message);

        // Clone what we need for the background thread
        let stood_agent = Arc::clone(&self.stood_agent);
        let sender = self.response_channel.0.clone();
        let logger = Arc::clone(&self.logger);
        let runtime = Arc::clone(&self.runtime);
        let agent_id = self.id;
        let agent_type = self.agent_type;

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

            // Execute agent in tokio runtime
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
                let full_message = format!("{}{}", instruction_template, user_message);

                // Execute agent with full message (instructions + user query)
                logger.log_system_message(&agent_type, "Executing agent...");
                match agent.execute(&full_message).await {
                    Ok(_) => {
                        let exec_completed_at = std::time::Instant::now();
                        logger.log_system_message(&agent_type, "Agent execution completed");

                        // Get final response from conversation
                        if let Some(last_message) = agent.conversation().messages().last() {
                            if let Some(text) = last_message.text() {
                                let before_log = std::time::Instant::now();
                                log::info!("[TIMING] Before log_assistant_response at {:?}", before_log);

                                logger.log_assistant_response(&agent_type, &text);

                                let after_log = std::time::Instant::now();
                                let log_duration = after_log.duration_since(before_log);
                                log::info!("[TIMING] After log_assistant_response at {:?} (took {:?})", after_log, log_duration);

                                let before_send = std::time::Instant::now();
                                let send_result = sender.send(ConversationResponse::Success(text.to_string()));
                                let after_send = std::time::Instant::now();
                                let send_duration = after_send.duration_since(before_send);

                                log::info!(
                                    "[TIMING] Channel send took {:?} | Total from exec complete: {:?} | Send result: {:?}",
                                    send_duration,
                                    after_send.duration_since(exec_completed_at),
                                    if send_result.is_ok() { "OK" } else { "ERROR" }
                                );

                                log::info!(
                                    "[BG THREAD] Message sent to channel at {:?} (Instant: tv_sec={}, tv_nsec={})",
                                    after_send,
                                    after_send.elapsed().as_secs(),
                                    after_send.elapsed().subsec_nanos()
                                );
                            } else {
                                let error_msg = "Agent response had no text content".to_string();
                                logger.log_error(&agent_type, &error_msg);
                                let _ = sender.send(ConversationResponse::Error(error_msg));
                            }
                        } else {
                            let error_msg = "Agent produced no response".to_string();
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
    pub fn poll_response(&mut self) -> bool {
        match self.response_channel.1.try_recv() {
            Ok(response) => {
                let response_received_at = std::time::Instant::now();
                self.logger
                    .log_system_message(&self.agent_type, "Response received from background thread");
                self.processing = false;
                self.status_message = None;

                match response {
                    ConversationResponse::Success(text) => {
                        // Log successful response
                        let text_preview = if text.len() > 200 {
                            format!("{}...", &text[..200])
                        } else {
                            text.clone()
                        };
                        self.logger
                            .log_system_message(&self.agent_type, &format!("Success response: {}", text_preview));
                        let msg = ConversationMessage::assistant(text);
                        let message_timestamp = msg.timestamp;
                        self.messages.push_back(msg);

                        log::info!(
                            "[TIMING] Response added to messages at {:?} (message timestamp: {})",
                            response_received_at,
                            message_timestamp.format("%H:%M:%S%.3f")
                        );
                        self.logger.log_system_message(&self.agent_type, &format!(
                            "Response added at {} (timestamp: {})",
                            response_received_at.elapsed().as_millis(),
                            message_timestamp.format("%H:%M:%S%.3f")
                        ));
                    }
                    ConversationResponse::Error(error) => {
                        self.messages
                            .push_back(ConversationMessage::assistant(format!("Error: {}", error)));
                        self.logger.log_error(&self.agent_type, &format!("Agent error: {}", error));
                        self.status = AgentStatus::Failed(error);
                    }
                    ConversationResponse::StatusUpdate(status) => {
                        // Update status message without finishing processing
                        self.status_message = Some(status);
                        self.processing = true; // Keep processing state
                        return false; // Don't mark as complete, continue polling
                    }
                }
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.processing = false;
                self.status_message = None;
                self.logger.log_error(&self.agent_type, "Response channel disconnected");
                false
            }
        }
    }

    /// Initialize the stood agent (must be called before first send_message)
    ///
    /// This is separate from construction to allow async credential loading
    pub fn initialize(
        &mut self,
        aws_identity: &mut crate::app::aws_identity::AwsIdentityCenter,
    ) -> Result<(), String> {
        let agent = self.create_stood_agent(aws_identity)?;

        let mut guard = self.stood_agent.lock().unwrap();
        *guard = Some(agent);

        self.logger
            .log_system_message(&self.agent_type, "Agent fully initialized and ready");
        Ok(())
    }

    /// Change the agent's model (requires re-initialization)
    ///
    /// This updates the agent metadata and clears the stood agent,
    /// which will be re-created with the new model on next message send.
    pub fn change_model(&mut self, new_model_id: String) {
        let old_model = self.metadata.model_id.clone();

        // Update metadata
        self.metadata.model_id = new_model_id.clone();
        self.metadata.updated_at = chrono::Utc::now();

        // Log model change
        self.logger.log_model_changed(&self.agent_type, &old_model, &new_model_id);

        // Clear stood agent - will be re-created with new model on next initialize()
        *self.stood_agent.lock().unwrap() = None;

        self.logger.log_system_message(&self.agent_type, &format!(
            "Model changed from {} to {}. Agent will re-initialize on next message.",
            old_model, new_model_id
        ));
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

        // Reset stood agent (will be re-initialized on next message)
        *self.stood_agent.lock().unwrap() = None;

        // Reset processing state
        self.processing = false;
        self.status_message = None;

        // Update metadata timestamp
        self.metadata.updated_at = chrono::Utc::now();
    }

    /// Terminate the agent and clean up resources
    pub fn terminate(&mut self) {
        self.status = AgentStatus::Cancelled;
        self.processing = false;
        self.logger.log_system_message(&self.agent_type, "Agent terminated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::TodoStatus;
    use chrono::Utc;

    fn create_test_metadata() -> AgentMetadata {
        AgentMetadata {
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            model_id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
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
}
