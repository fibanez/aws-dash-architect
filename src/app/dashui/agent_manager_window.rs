#![warn(clippy::all, rust_2018_idioms)]

//! Agent Manager Window - Unified Two-Pane UI for Managing Multiple Agents
//!
//! This window provides a unified interface with two panes:
//! - Left Pane: Agent list with [+] button to create new agents
//! - Right Pane: Selected agent's chat view (conversation, input, controls)
//!
//! ## Architecture
//!
//! - **UI Layer** (this file): Two-pane layout, agent selection, chat interface
//! - **Business Layer**: AgentManager handles agent lifecycle
//! - **Separation**: Window lifecycle is independent of agent lifecycle

use super::agent_log_window::AgentLogWindow;
use super::window_focus::FocusableWindow;
use crate::app::agent_framework::{
    get_agent_creation_receiver, get_ui_event_receiver, render_agent_chat, AgentCreationRequest,
    AgentId, AgentInstance, AgentModel, AgentType, AgentUIEvent, InlineWorkerDisplay,
    ProcessingStatusWidget, StoodLogLevel,
};
use crate::app::aws_identity::AwsIdentityCenter;
use eframe::egui;
use egui::{Context, RichText, ScrollArea, Ui};
use egui_commonmark::CommonMarkCache;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

/// Context information for displaying task progress
#[derive(Debug, Clone)]
struct TaskContext {
    /// Current task index (0-based)
    current_index: usize,

    /// Total number of active tasks
    total_tasks: usize,

    /// IDs of all active task-agents (reserved for future use)
    _active_task_ids: Vec<AgentId>,
}

/// Status of an inline worker message
#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkerMessageStatus {
    /// Worker is running
    Running,
    /// Worker completed (success or failure)
    Completed { success: bool },
}

/// Worker inline message for displaying progress in conversation flow
#[derive(Debug, Clone)]
struct WorkerInlineMessage {
    /// Worker agent ID
    worker_id: AgentId,
    /// Parent agent ID (manager that spawned this worker)
    parent_id: AgentId,
    /// Short description for inline display
    short_description: String,
    /// Path to worker's log file
    log_path: Option<std::path::PathBuf>,
    /// Currently running tool (if any)
    current_tool: Option<String>,
    /// Worker status
    status: WorkerMessageStatus,
    /// Cumulative token usage from model calls
    total_tokens: u32,
}

impl WorkerInlineMessage {
    /// Create a new running worker message
    fn new(worker_id: AgentId, parent_id: AgentId, short_description: String) -> Self {
        Self {
            worker_id,
            parent_id,
            short_description,
            log_path: None,
            current_tool: None,
            status: WorkerMessageStatus::Running,
            total_tokens: 0,
        }
    }

    /// Mark as completed
    fn mark_completed(&mut self, success: bool) {
        self.status = WorkerMessageStatus::Completed { success };
        self.current_tool = None;
    }
}

pub struct AgentManagerWindow {
    open: bool,

    // AWS Identity for agent execution
    aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>,

    // CloudWatch Agent Logging toggle (mirrors DashApp setting)
    agent_logging_enabled: bool,

    // Selection state - which agent is displayed in right pane
    selected_agent_id: Option<AgentId>,

    // Tab state - which conversation is shown (manager or specific worker)
    // When viewing a TaskManager, this determines which tab's conversation to display
    selected_tab_agent_id: Option<AgentId>,

    // Agent name editing state
    editing_agent_name: Option<AgentId>,
    temp_agent_name: String,

    // Agent log viewer
    agent_log_window: AgentLogWindow,

    // Agents
    agents: HashMap<AgentId, AgentInstance>,
    input_text: String,

    // Model selection for new agents
    selected_model: AgentModel,

    // Stood library log level for all agents
    stood_log_level: StoodLogLevel,

    // UI event receiver for agent framework events
    ui_event_receiver: Arc<Mutex<Receiver<AgentUIEvent>>>,

    // Agent creation request receiver
    agent_creation_receiver: Arc<Mutex<Receiver<AgentCreationRequest>>>,

    // Markdown rendering cache (shared across all agents)
    markdown_cache: CommonMarkCache,

    // Processing status widgets (per-agent for animation state)
    status_widgets: HashMap<AgentId, ProcessingStatusWidget>,

    // Inline worker messages keyed by conversation message index
    // Maps message_index -> Vec<WorkerInlineMessage>
    worker_inline_messages: HashMap<usize, Vec<WorkerInlineMessage>>,
}

impl Default for AgentManagerWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentManagerWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            aws_identity: None,
            agent_logging_enabled: true,
            selected_agent_id: None,
            selected_tab_agent_id: None,
            editing_agent_name: None,
            temp_agent_name: String::new(),
            agent_log_window: AgentLogWindow::new(),
            agents: HashMap::new(),
            input_text: String::new(),
            selected_model: AgentModel::default(),
            stood_log_level: StoodLogLevel::default(), // Default: Debug level
            ui_event_receiver: get_ui_event_receiver(), // UI event channel
            agent_creation_receiver: get_agent_creation_receiver(), // Agent creation channel
            markdown_cache: CommonMarkCache::default(),
            status_widgets: HashMap::new(),
            worker_inline_messages: HashMap::new(),
        }
    }

    /// Set AWS Identity for agent execution
    pub fn set_aws_identity(&mut self, aws_identity: Arc<Mutex<AwsIdentityCenter>>) {
        self.aws_identity = Some(aws_identity);
    }

    /// Set agent logging enabled state (synced from DashApp)
    pub fn set_agent_logging_enabled(&mut self, enabled: bool) {
        self.agent_logging_enabled = enabled;
    }

    /// Select an agent to display in the right pane
    pub fn select_agent(&mut self, agent_id: AgentId) {
        self.selected_agent_id = Some(agent_id);
        // Reset tab to show manager when switching agents
        self.selected_tab_agent_id = Some(agent_id);
    }

    /// Clear the selected agent
    pub fn clear_selection(&mut self) {
        self.selected_agent_id = None;
    }

    /// Get the currently selected agent
    pub fn get_selected_agent(&self) -> Option<AgentId> {
        self.selected_agent_id
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn show(&mut self, ctx: &Context) {
        self.show_with_focus(ctx, (), false);

        // Show agent log window if open
        if self.agent_log_window.is_open() {
            self.agent_log_window.show(ctx, false);
        }
    }

    fn ui_content(&mut self, ui: &mut Ui) {
        // NOTE: Polling now happens globally in DashApp::update() before rendering
        // to ensure agents are polled even when window is closed

        // Two-pane horizontal layout using StripBuilder for proper vertical resizing
        use egui_extras::{Size, StripBuilder};

        StripBuilder::new(ui)
            .size(Size::exact(130.0)) // Left pane - narrower fixed width
            .size(Size::remainder()) // Right pane - fills remaining space
            .horizontal(|mut strip| {
                // LEFT PANE: Agent list with fixed width
                strip.cell(|ui| {
                    ui.heading(RichText::new("Agents").size(14.0));

                    // Stood log level dropdown
                    ui.horizontal(|ui| {
                        ui.label("Logs:");
                        let current_level = self.stood_log_level;
                        egui::ComboBox::from_id_salt("stood_log_level")
                            .selected_text(current_level.display_name())
                            .width(60.0)
                            .show_ui(ui, |ui| {
                                for level in StoodLogLevel::all() {
                                    if ui
                                        .selectable_label(
                                            self.stood_log_level == *level,
                                            level.display_name(),
                                        )
                                        .clicked()
                                        && self.stood_log_level != *level
                                    {
                                        let old_level = self.stood_log_level;
                                        self.stood_log_level = *level;

                                        // Update global tracing filter
                                        crate::set_stood_log_level(*level);

                                        // Update all agents with new log level
                                        for agent in self.agents.values_mut() {
                                            agent.set_stood_log_level(*level);
                                        }

                                        tracing::info!(
                                            old_level = %old_level.display_name(),
                                            new_level = %level.display_name(),
                                            "Stood log level changed"
                                        );
                                    }
                                }
                            });
                    });

                    ui.add_space(5.0);

                    // Model selection dropdown
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("model_selector")
                            .selected_text(self.selected_model.display_name())
                            .show_ui(ui, |ui| {
                                for model in AgentModel::all_models() {
                                    ui.selectable_value(
                                        &mut self.selected_model,
                                        *model,
                                        model.display_name(),
                                    );
                                }
                            });
                    });

                    ui.add_space(5.0);

                    // [+] New Agent button
                    if ui.button("+ New Agent").clicked() {
                        log::info!("New Agent button clicked");
                        self.create_new_agent();
                    }

                    // Separator after New Agent button
                    ui.separator();

                    // Collect agent info (exclude TaskWorker agents - they are shown inline)
                    let agent_list: Vec<(AgentId, String)> = self
                        .agents
                        .iter()
                        .filter(|(_, agent)| !matches!(agent.agent_type(), AgentType::TaskWorker { .. }))
                        .map(|(agent_id, agent)| (*agent_id, agent.metadata().name.clone()))
                        .collect();

                    let mut clicked_agent_id: Option<AgentId> = None;
                    let mut start_editing_id: Option<AgentId> = None;
                    let mut finish_editing = false;
                    let mut cancel_editing = false;

                    // Agent list scroll area - takes remaining vertical space
                    ScrollArea::vertical()
                        .id_salt("agent_list_scroll")
                        .show(ui, |ui| {
                            if agent_list.is_empty() {
                                ui.label(RichText::new("No agents").weak());
                            } else {
                                for (agent_id, name) in agent_list {
                                    let is_selected = self.selected_agent_id == Some(agent_id);
                                    let is_editing = self.editing_agent_name == Some(agent_id);

                                    if is_editing {
                                        // Show text edit for renaming
                                        let response = ui.add(
                                            egui::TextEdit::singleline(&mut self.temp_agent_name)
                                                .desired_width(100.0),
                                        );

                                        // Request focus on first frame of editing
                                        response.request_focus();

                                        // Check for Enter key while field has focus
                                        let enter_pressed =
                                            ui.input(|i| i.key_pressed(egui::Key::Enter));
                                        let escape_pressed =
                                            ui.input(|i| i.key_pressed(egui::Key::Escape));

                                        if escape_pressed {
                                            cancel_editing = true;
                                        } else if enter_pressed || response.lost_focus() {
                                            // Enter pressed or clicked elsewhere
                                            finish_editing = true;
                                        }
                                    } else {
                                        // Agent list item - use full width for better clickability
                                        let response = ui.selectable_label(is_selected, &name);

                                        if response.clicked() {
                                            clicked_agent_id = Some(agent_id);
                                        }

                                        // Double-click to edit name
                                        if response.double_clicked() {
                                            start_editing_id = Some(agent_id);
                                        }
                                    }
                                }
                            }
                        });

                    // Handle editing state changes after scroll area
                    if let Some(agent_id) = start_editing_id {
                        if let Some(agent) = self.agents.get(&agent_id) {
                            self.temp_agent_name = agent.metadata().name.clone();
                            self.editing_agent_name = Some(agent_id);
                        }
                    }

                    if finish_editing {
                        if let Some(agent_id) = self.editing_agent_name.take() {
                            let new_name = self.temp_agent_name.trim().to_string();
                            if !new_name.is_empty() {
                                if let Some(agent) = self.agents.get_mut(&agent_id) {
                                    let old_name = agent.metadata().name.clone();
                                    agent.metadata_mut().name = new_name.clone();
                                    // Update logger with new name
                                    agent
                                        .logger()
                                        .update_agent_name(&agent.agent_type(), new_name.clone());
                                    log::info!(
                                        "Agent {} renamed from '{}' to '{}'",
                                        agent_id,
                                        old_name,
                                        new_name
                                    );
                                }
                            }
                        }
                        self.temp_agent_name.clear();
                    }

                    if cancel_editing {
                        self.editing_agent_name = None;
                        self.temp_agent_name.clear();
                    }

                    // Handle selection after scroll area
                    if let Some(agent_id) = clicked_agent_id {
                        self.select_agent(agent_id);
                    }
                });

                // RIGHT PANE: Agent chat view - fills remaining space
                strip.cell(|ui| {
                    if let Some(agent_id) = self.selected_agent_id {
                        self.render_agent_chat_view(ui, agent_id);
                    }
                    // No empty state message - just blank space
                });
            });
    }

    /// Create a new agent instance
    fn create_new_agent(&mut self) {
        use crate::app::agent_framework::AgentMetadata;
        use chrono::Utc;

        // Generate default name
        let agent_count = self.agents.len();
        let default_name = format!("Agent {}", agent_count + 1);

        log::info!(
            "Creating new agent: {} with model {} (current agent count: {})",
            default_name,
            self.selected_model,
            agent_count
        );

        let metadata = AgentMetadata {
            name: default_name.clone(),
            description: "New agent".to_string(),
            model: self.selected_model,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Always create TaskManager agents
        // Future: Add UI for selecting agent type
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);
        let agent_id = agent.id();

        // Set the current stood log level on new agent
        agent.set_stood_log_level(self.stood_log_level);

        // Initialize agent with AWS credentials
        if let Some(aws_identity) = &self.aws_identity {
            // Extract result first to drop lock before using self
            let init_result = agent.initialize(
                &mut aws_identity.lock().unwrap(),
                self.agent_logging_enabled,
            );
            match init_result {
                Ok(_) => {
                    log::info!(
                        "Agent {} initialized successfully (ID: {})",
                        default_name,
                        agent_id
                    );
                    self.agents.insert(agent_id, agent);
                    log::info!("Agent {} inserted into agents map", agent_id);
                    self.select_agent(agent_id);
                    log::info!("Agent {} selected and should now be visible", agent_id);
                }
                Err(e) => {
                    log::error!("Failed to initialize agent {}: {}", default_name, e);
                }
            }
        } else {
            log::error!("Cannot create agent: AWS Identity not set");
        }
    }

    /// Process pending UI events from the agent framework
    ///
    /// This is called during the UI update loop to handle events sent by
    /// agent tools (like start-task creating a new agent and switching to it).
    fn process_ui_events(&mut self) {
        // Collect all pending events first, then process them
        // This avoids holding the lock while calling &mut self methods
        let events: Vec<AgentUIEvent> = {
            if let Ok(receiver) = self.ui_event_receiver.lock() {
                let mut collected = Vec::new();
                while let Ok(event) = receiver.try_recv() {
                    collected.push(event);
                }
                collected
            } else {
                Vec::new()
            }
        };

        // Now process events without holding the lock
        for event in events {
            match event {
                AgentUIEvent::SwitchToAgent(agent_id) => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        agent_id = %agent_id,
                        "UI event: Switch to agent"
                    );
                    self.select_agent(agent_id);
                }
                AgentUIEvent::SwitchToParent(parent_id) => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        parent_id = %parent_id,
                        "UI event: Switch to parent agent"
                    );
                    self.select_agent(parent_id);
                }
                AgentUIEvent::AgentCompleted(agent_id) => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        agent_id = %agent_id,
                        "UI event: Agent completed"
                    );
                    self.handle_agent_completion(agent_id);
                }
                // Worker progress events - inline display implementation pending
                AgentUIEvent::WorkerStarted {
                    worker_id,
                    parent_id,
                    short_description,
                    message_index,
                } => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        worker_id = %worker_id,
                        parent_id = %parent_id,
                        short_description = %short_description,
                        message_index = message_index,
                        "UI event: Worker started"
                    );
                    self.handle_worker_started(worker_id, parent_id, short_description, message_index);
                }
                AgentUIEvent::WorkerToolStarted {
                    worker_id,
                    parent_id,
                    tool_name,
                } => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        worker_id = %worker_id,
                        parent_id = %parent_id,
                        tool_name = %tool_name,
                        "UI event: Worker tool started"
                    );
                    self.handle_worker_tool_started(worker_id, tool_name);
                }
                AgentUIEvent::WorkerToolCompleted {
                    worker_id,
                    parent_id,
                    tool_name,
                    success,
                } => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        worker_id = %worker_id,
                        parent_id = %parent_id,
                        tool_name = %tool_name,
                        success = success,
                        "UI event: Worker tool completed"
                    );
                    self.handle_worker_tool_completed(worker_id, tool_name, success);
                }
                AgentUIEvent::WorkerCompleted {
                    worker_id,
                    parent_id,
                    success,
                } => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        worker_id = %worker_id,
                        parent_id = %parent_id,
                        success = success,
                        "UI event: Worker completed"
                    );
                    self.handle_worker_completed(worker_id, success);
                }
                AgentUIEvent::WorkerTokensUpdated {
                    worker_id,
                    parent_id,
                    total_tokens,
                    ..
                } => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        worker_id = %worker_id,
                        parent_id = %parent_id,
                        total_tokens = total_tokens,
                        "UI event: Worker tokens updated"
                    );
                    self.handle_worker_tokens_updated(worker_id, total_tokens);
                }
            }
        }
    }

    /// Handle agent completion event
    ///
    /// Called when an agent (usually a task-worker) completes its work.
    /// This method can clean up resources, update UI state, etc.
    fn handle_agent_completion(&mut self, agent_id: AgentId) {
        // Log completion event
        // Future: Remove from active tasks list, cleanup resources, etc.
        tracing::info!(
            target: "agent::lifecycle",
            agent_id = %agent_id,
            "Agent marked as completed"
        );
    }

    /// Handle worker started event
    ///
    /// Creates an inline worker message entry for display in the conversation.
    fn handle_worker_started(
        &mut self,
        worker_id: AgentId,
        parent_id: AgentId,
        short_description: String,
        message_index: usize,
    ) {
        let mut message = WorkerInlineMessage::new(worker_id, parent_id, short_description);

        // Try to get log path from the worker agent's logger
        if let Some(agent) = self.agents.get(&worker_id) {
            message.log_path = Some(agent.logger().log_path().clone());
        }

        // Add to inline messages for this message index
        self.worker_inline_messages
            .entry(message_index)
            .or_default()
            .push(message);

        tracing::info!(
            target: "agent::ui_events",
            worker_id = %worker_id,
            parent_id = %parent_id,
            message_index = message_index,
            "Added inline worker message"
        );
    }

    /// Handle worker tool started event
    ///
    /// Updates the current tool for the worker's inline message.
    fn handle_worker_tool_started(&mut self, worker_id: AgentId, tool_name: String) {
        // Find the worker in any message index and update current_tool
        for workers in self.worker_inline_messages.values_mut() {
            if let Some(worker) = workers.iter_mut().find(|w| w.worker_id == worker_id) {
                worker.current_tool = Some(tool_name.clone());
                return;
            }
        }
    }

    /// Handle worker tool completed event
    ///
    /// Clears the current tool for the worker's inline message.
    fn handle_worker_tool_completed(
        &mut self,
        worker_id: AgentId,
        _tool_name: String,
        _success: bool,
    ) {
        // Find the worker and clear current_tool
        for workers in self.worker_inline_messages.values_mut() {
            if let Some(worker) = workers.iter_mut().find(|w| w.worker_id == worker_id) {
                worker.current_tool = None;
                return;
            }
        }
    }

    /// Handle worker completed event
    ///
    /// Marks the worker as completed in its inline message.
    fn handle_worker_completed(&mut self, worker_id: AgentId, success: bool) {
        // Find the worker and mark as completed
        for workers in self.worker_inline_messages.values_mut() {
            if let Some(worker) = workers.iter_mut().find(|w| w.worker_id == worker_id) {
                worker.mark_completed(success);
                tracing::info!(
                    target: "agent::ui_events",
                    worker_id = %worker_id,
                    success = success,
                    "Worker marked as completed"
                );
                return;
            }
        }
    }

    /// Handle worker tokens updated event
    ///
    /// Updates the cumulative token count for a worker's inline message.
    fn handle_worker_tokens_updated(&mut self, worker_id: AgentId, total_tokens: u32) {
        // Find the worker and update token count
        for workers in self.worker_inline_messages.values_mut() {
            if let Some(worker) = workers.iter_mut().find(|w| w.worker_id == worker_id) {
                worker.total_tokens = total_tokens;
                return;
            }
        }
    }

    /// Convert worker inline messages to display format for rendering
    ///
    /// Filters workers for the given parent agent and converts them to InlineWorkerDisplay
    /// format, organized by message index.
    fn convert_workers_to_display(
        &self,
        parent_id: AgentId,
    ) -> HashMap<usize, Vec<InlineWorkerDisplay>> {
        let mut result: HashMap<usize, Vec<InlineWorkerDisplay>> = HashMap::new();

        for (message_index, workers) in &self.worker_inline_messages {
            let filtered: Vec<InlineWorkerDisplay> = workers
                .iter()
                .filter(|w| w.parent_id == parent_id)
                .map(|w| InlineWorkerDisplay {
                    short_description: w.short_description.clone(),
                    current_tool: w.current_tool.clone(),
                    is_running: w.status == WorkerMessageStatus::Running,
                    success: matches!(w.status, WorkerMessageStatus::Completed { success: true }),
                    log_path: w.log_path.clone(),
                    total_tokens: w.total_tokens,
                })
                .collect();

            if !filtered.is_empty() {
                result.insert(*message_index, filtered);
            }
        }

        result
    }

    /// Process agent creation requests
    ///
    /// Polls the agent creation request channel and handles any pending requests.
    /// Called during the update loop before rendering.
    fn process_agent_creation_requests(&mut self) {
        use crate::app::agent_framework::{take_response_channel, AgentCreationResponse};

        // Collect all pending requests first to avoid holding the lock
        let requests: Vec<AgentCreationRequest> = {
            if let Ok(receiver) = self.agent_creation_receiver.lock() {
                let mut collected = Vec::new();
                while let Ok(request) = receiver.try_recv() {
                    collected.push(request);
                }
                collected
            } else {
                Vec::new()
            }
        };

        // Process each request
        for request in requests {
            tracing::debug!(
                target: "agent::creation",
                request_id = request.request_id,
                parent_id = %request.parent_id,
                "Processing agent creation request"
            );

            match self.handle_agent_creation_request(&request) {
                Ok(agent_id) => {
                    // Send success response
                    if let Some(response_sender) = take_response_channel(request.request_id) {
                        let response = AgentCreationResponse::success(agent_id);
                        if let Err(e) = response_sender.send(response) {
                            tracing::error!(
                                target: "agent::creation",
                                request_id = request.request_id,
                                error = %e,
                                "Failed to send agent creation success response"
                            );
                        }
                    }
                }
                Err(error) => {
                    // Send error response
                    if let Some(response_sender) = take_response_channel(request.request_id) {
                        let response = AgentCreationResponse::error(AgentId::new(), error.clone());
                        if let Err(e) = response_sender.send(response) {
                            tracing::error!(
                                target: "agent::creation",
                                request_id = request.request_id,
                                error = %e,
                                "Failed to send agent creation error response"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Handle a single agent creation request
    ///
    /// Creates a new TaskWorker agent with the specified configuration
    /// and sends the initial task message.
    fn handle_agent_creation_request(
        &mut self,
        request: &AgentCreationRequest,
    ) -> Result<AgentId, String> {
        use crate::app::agent_framework::AgentMetadata;
        use chrono::Utc;

        // Verify parent agent exists and get its model
        let parent_model = {
            let parent_agent = self
                .agents
                .get(&request.parent_id)
                .ok_or_else(|| format!("Parent agent {} not found", request.parent_id))?;
            parent_agent.metadata().model
        };

        // Generate agent name
        let worker_count = self
            .agents
            .values()
            .filter(|a| matches!(a.agent_type(), AgentType::TaskWorker { .. }))
            .count();
        let default_name = format!("Task Worker {}", worker_count + 1);

        // Create metadata (inherit parent's model)
        let metadata = AgentMetadata {
            name: default_name.clone(),
            description: format!("Task: {}", request.task_description),
            model: parent_model,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Get parent agent's logger to share with worker
        let parent_logger = {
            let parent_agent = self
                .agents
                .get(&request.parent_id)
                .ok_or_else(|| format!("Parent agent {} not found", request.parent_id))?;
            parent_agent.logger().clone()
        };

        // Create TaskWorker agent with parent_id and parent's logger
        let agent_type = AgentType::TaskWorker {
            parent_id: request.parent_id,
        };
        let mut agent = AgentInstance::new_with_parent_logger(metadata, agent_type, parent_logger);
        let agent_id = agent.id();

        // Set the current stood log level on new worker agent
        agent.set_stood_log_level(self.stood_log_level);

        // Initialize agent with AWS credentials
        if let Some(aws_identity) = &self.aws_identity {
            let init_result = agent.initialize(
                &mut aws_identity.lock().unwrap(),
                self.agent_logging_enabled,
            );
            if let Err(e) = init_result {
                return Err(format!("Failed to initialize agent: {}", e));
            }
        } else {
            return Err("AWS identity not available".to_string());
        }

        // Send initial task message
        agent.send_message(request.task_description.clone());

        tracing::info!(
            target: "agent::creation",
            agent_id = %agent_id,
            parent_id = %request.parent_id,
            name = %default_name,
            "Created TaskWorker agent"
        );

        // Determine the message index in parent's conversation where this worker was spawned
        let message_index = if let Some(parent) = self.agents.get(&request.parent_id) {
            parent.messages().len().saturating_sub(1)
        } else {
            0
        };

        // Insert agent into map
        self.agents.insert(agent_id, agent);

        // Send WorkerStarted event for inline display (replaces tab creation)
        let _ = crate::app::agent_framework::send_ui_event(
            crate::app::agent_framework::AgentUIEvent::worker_started(
                agent_id,
                request.parent_id,
                request.short_description.clone(),
                message_index,
            ),
        );
        tracing::debug!(
            target: "agent::creation",
            agent_id = %agent_id,
            parent_id = %request.parent_id,
            message_index = message_index,
            short_description = %request.short_description,
            "Sent WorkerStarted event for inline display"
        );

        Ok(agent_id)
    }

    /// Get list of active task-agent IDs
    ///
    /// Returns agents with AgentType::TaskWorker that are currently running
    fn get_active_task_agents(&self) -> Vec<AgentId> {
        use crate::app::agent_framework::AgentStatus;

        self.agents
            .iter()
            .filter_map(|(id, agent)| {
                if matches!(agent.agent_type(), AgentType::TaskWorker { .. })
                    && agent.status() == &AgentStatus::Running
                {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Cycle to the next active task-agent
    ///
    /// If currently viewing a task-agent, switch to the next one in the list
    /// (wrapping around to the first). If not viewing a task-agent, switch to
    /// the first active task-agent.
    fn cycle_to_next_task_agent(&mut self) {
        let active_tasks = self.get_active_task_agents();
        if active_tasks.is_empty() {
            tracing::debug!(
                target: "agent::ui",
                "No active task-agents to cycle through"
            );
            return;
        }

        // Find current task index
        let current_index = self
            .selected_agent_id
            .and_then(|selected_id| active_tasks.iter().position(|id| *id == selected_id))
            .unwrap_or(0);

        // Cycle to next (wrap around)
        let next_index = (current_index + 1) % active_tasks.len();
        let next_agent_id = active_tasks[next_index];

        tracing::debug!(
            target: "agent::ui",
            current_index = current_index,
            next_index = next_index,
            next_agent_id = %next_agent_id,
            total_tasks = active_tasks.len(),
            "Cycling to next task-agent"
        );

        self.select_agent(next_agent_id);
    }

    /// Handle keyboard navigation
    ///
    /// - Tab: Cycle through active task-agents
    /// - Escape: Stop all agents (future)
    fn handle_keyboard_navigation(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Tab key: cycle through active task-agents
            if i.key_pressed(egui::Key::Tab) {
                self.cycle_to_next_task_agent();
            }

            // Future: Escape key for stopping agents
            // if i.key_pressed(egui::Key::Escape) {
            //     self.stop_all_agents();
            // }
        });
    }

    /// Get task context for the currently selected agent
    ///
    /// Returns Some if the selected agent is a task-worker, None otherwise
    fn get_task_context(&self) -> Option<TaskContext> {
        let selected_id = self.selected_agent_id?;
        let selected_agent = self.agents.get(&selected_id)?;

        // Only show context for task-workers
        if !matches!(selected_agent.agent_type(), AgentType::TaskWorker { .. }) {
            return None;
        }

        let active_tasks = self.get_active_task_agents();
        let current_index = active_tasks.iter().position(|id| *id == selected_id)?;

        Some(TaskContext {
            current_index,
            total_tasks: active_tasks.len(),
            _active_task_ids: active_tasks,
        })
    }

    /// Render task indicator at the top of the chat pane
    fn render_task_indicator(&self, ui: &mut egui::Ui) {
        if let Some(ctx) = self.get_task_context() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Task {} of {} - Press Tab to cycle",
                        ctx.current_index + 1,
                        ctx.total_tasks
                    ))
                    .strong()
                    .color(egui::Color32::from_rgb(100, 150, 255)),
                );

                // Future: Show worker agent count
            });
            ui.separator();
        }
    }

    /// Render agent chat view in the right pane
    fn render_agent_chat_view(&mut self, ui: &mut Ui, agent_id: AgentId) {
        // Render task indicator first
        self.render_task_indicator(ui);

        // Convert worker inline messages to display format for the render function
        let inline_workers_display = self.convert_workers_to_display(agent_id);

        // Always show the selected manager agent's conversation
        // (workers no longer have tabs - they're shown inline within conversation)
        let display_agent_id = agent_id;

        // Ensure status widget exists for this agent
        self.status_widgets
            .entry(display_agent_id)
            .or_default();

        // Render UI and handle message sending/polling in a scope to release borrow
        let (terminate_clicked, log_clicked, _clear_clicked, worker_log_to_open) = {
            // Get the agent and status widget to display
            let agent = match self.agents.get_mut(&display_agent_id) {
                Some(agent) => agent,
                None => {
                    ui.label(RichText::new("Agent not found").color(egui::Color32::RED));
                    return;
                }
            };

            // Get status widget (we just ensured it exists)
            let status_widget = self.status_widgets.get_mut(&display_agent_id).unwrap();

            // Render the chat UI with inline workers
            let (
                should_send,
                log_clicked,
                clear_clicked,
                terminate_clicked,
                stop_clicked,
                worker_log_clicked,
            ) = render_agent_chat(
                ui,
                agent,
                &mut self.input_text,
                &mut self.markdown_cache,
                status_widget,
                Some(&inline_workers_display),
            );

            // Send message if requested
            if should_send {
                let message = self.input_text.clone();
                self.input_text.clear();

                // Send message to agent
                log::info!("Sending message to agent {}: {}", agent_id, message);
                agent.send_message(message);
            }

            // Handle stop button click - cancel ongoing execution
            if stop_clicked {
                if agent.cancel() {
                    log::info!("Agent {} execution cancelled by user", agent_id);
                }
            }

            // Handle clear conversation if requested
            if clear_clicked {
                agent.clear_conversation();
                log::info!("Agent {} conversation cleared", agent_id);
            }

            (terminate_clicked, log_clicked, clear_clicked, worker_log_clicked)
        }; // agent borrow released here

        // Handle worker log button click
        if let Some(log_path) = worker_log_to_open {
            self.agent_log_window
                .show_log_from_path(&log_path, "Worker Log");
        }

        // Handle log button click outside the borrow scope
        if log_clicked {
            // Get the agent again to access its logger
            if let Some(agent) = self.agents.get(&agent_id) {
                self.agent_log_window.show_log_for_agent(
                    agent_id,
                    agent.metadata().name.clone(),
                    agent.logger(),
                );
                tracing::info!("Agentlog viewer opened for agent {}", agent_id);
            }
        }

        // Handle termination outside the borrow scope
        if terminate_clicked {
            self.agents.remove(&agent_id);
            log::info!("Agent{} terminated and removed", agent_id);
            // Clear selection since the agent is now deleted
            if self.selected_agent_id == Some(agent_id) {
                self.selected_agent_id = None;
            }
        }
    }

    /// Poll all agents for responses (called from DashApp::update() every frame)
    ///
    /// This method is called globally before rendering to ensure agent responses
    /// are retrieved immediately, regardless of whether the Agent Manager window is open.
    pub fn poll_agent_responses_global(&mut self) {
        static FRAME_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let poll_start = std::time::Instant::now();
        log::trace!(
            "[FRAME {}] poll_agent_responses called at {:?}",
            frame,
            poll_start
        );

        // Pollagents (every frame, regardless of selection)
        let v2_agent_ids: Vec<AgentId> = self.agents.keys().copied().collect();
        let mut total_responses = 0;
        let mut completed_workers: Vec<(AgentId, AgentId, Result<String, String>)> = Vec::new();

        for agent_id in v2_agent_ids {
            if let Some(agent) = self.agents.get_mut(&agent_id) {
                let poll_start_v2 = std::time::Instant::now();
                if agent.poll_response() {
                    let poll_duration = poll_start_v2.elapsed();

                    // Get timing info from the message
                    if let Some(last_msg) = agent.messages().back() {
                        let now_utc = chrono::Utc::now();
                        let msg_created_at = last_msg.timestamp;
                        let delay_ms = (now_utc - msg_created_at).num_milliseconds();

                        log::info!(
                            "[FRAME {}] [V2 POLL] Agent {} response retrieved | poll_response took: {:?} | Message created: {} | Total delay: {}ms",
                            frame,
                            agent_id,
                            poll_duration,
                            msg_created_at.format("%H:%M:%S%.3f"),
                            delay_ms
                        );

                        // Check if this is a completed worker agent
                        if let AgentType::TaskWorker { parent_id } = agent.agent_type() {
                            if last_msg.role
                                == crate::app::agent_framework::ConversationRole::Assistant
                            {
                                // Worker has completed - check if it's an error or success
                                let result = if last_msg.content.starts_with("Error: ") {
                                    // Strip "Error: " prefix and send as error
                                    Err(last_msg
                                        .content
                                        .strip_prefix("Error: ")
                                        .unwrap_or(&last_msg.content)
                                        .to_string())
                                } else {
                                    // Success - send raw content
                                    Ok(last_msg.content.clone())
                                };
                                let is_success = result.is_ok();
                                completed_workers.push((agent_id, parent_id, result));
                                log::info!(
                                    target: "agent::worker_complete",
                                    "Worker agent {} completed task for parent {} (success: {})",
                                    agent_id,
                                    parent_id,
                                    is_success
                                );
                            }
                        }
                    } else {
                        log::info!("[FRAME {}] [V2 POLL] Agent {} response retrieved | poll_response took: {:?}", frame, agent_id, poll_duration);
                    }
                    total_responses += 1;
                }
            }
        }

        // Process completed workers: send results to parent and terminate
        for (worker_id, parent_id, result) in completed_workers {
            // Calculate worker execution time using metadata created_at
            let execution_time = if let Some(worker_agent) = self.agents.get(&worker_id) {
                let created_at = worker_agent.metadata().created_at;
                let now = chrono::Utc::now();
                let duration_ms = (now - created_at).num_milliseconds();
                std::time::Duration::from_millis(duration_ms.max(0) as u64)
            } else {
                std::time::Duration::from_secs(0)
            };

            // Send worker completion to channel (for start_task tool to return as ToolResult)
            let is_success = result.is_ok();
            let completion = crate::app::agent_framework::WorkerCompletion {
                worker_id,
                result, // Raw result (Ok or Err), no wrapper text
                execution_time,
            };
            crate::app::agent_framework::send_worker_completion(completion);

            log::info!(
                target: "agent::worker_complete",
                "Sent worker {} completion to channel (parent: {}, execution time: {:?})",
                worker_id,
                parent_id,
                execution_time
            );

            // Send WorkerCompleted UI event for inline display update
            let _ = crate::app::agent_framework::send_ui_event(
                crate::app::agent_framework::AgentUIEvent::worker_completed(
                    worker_id,
                    parent_id,
                    is_success,
                ),
            );
            tracing::debug!(
                target: "agent::worker_complete",
                worker_id = %worker_id,
                parent_id = %parent_id,
                success = is_success,
                "Sent WorkerCompleted event for inline display"
            );

            // Remove worker instance - log path is preserved in WorkerInlineMessage
            self.agents.remove(&worker_id);
            tracing::debug!(
                target: "agent::worker_complete",
                worker_id = %worker_id,
                "Worker agent removed after completion"
            );
        }

        let poll_duration = poll_start.elapsed();
        if total_responses > 0 || poll_duration.as_millis() > 10 {
            log::debug!(
                "⏱️ [TIMING] poll_agent_responses TOTAL: {:?} ({} responses)",
                poll_duration,
                total_responses
            );
        }
    }
}

impl FocusableWindow for AgentManagerWindow {
    type ShowParams = ();

    fn window_id(&self) -> &'static str {
        "agent_manager_window"
    }

    fn window_title(&self) -> String {
        "Agent Manager".to_string()
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
        if !self.open {
            return;
        }

        // Process UI events first
        self.process_ui_events();

        // Process agent creation requests
        self.process_agent_creation_requests();

        // Handle keyboard navigation
        self.handle_keyboard_navigation(ctx);

        // Use local variable for .open() to avoid borrow checker issues
        let mut is_open = self.open;

        // Get screen constraints for proper window sizing
        let screen_rect = ctx.screen_rect();
        let max_width = screen_rect.width() * 0.9;
        let max_height = screen_rect.height() * 0.9;

        let mut window = egui::Window::new(self.window_title())
            .open(&mut is_open)
            .resizable(true)
            .default_size([max_width, max_height])
            .constrain(true) // Ensure window stays within screen bounds
            .movable(true)
            .collapsible(true); // Allow window collapse with triangle button

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        window.show(ctx, |ui| {
            self.ui_content(ui);
        });

        // Update self.open based on window state (X button click)
        self.open = is_open;

        // Show agent log window if open
        if self.agent_log_window.is_open() {
            self.agent_log_window.show(ctx, false);
        }
    }
}
