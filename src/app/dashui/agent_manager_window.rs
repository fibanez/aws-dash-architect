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
    AgentId, AgentInstance, AgentType, AgentUIEvent,
};
use crate::app::aws_identity::AwsIdentityCenter;
use eframe::egui;
use egui::{Context, RichText, ScrollArea, Ui};
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

/// Worker tab metadata for auto-close behavior
#[derive(Debug, Clone)]
struct WorkerTabMetadata {
    /// Completion timestamp (None if still running)
    completed_at: Option<std::time::Instant>,
    /// Last user interaction timestamp (for timer reset)
    last_viewed_at: Option<std::time::Instant>,
    /// Auto-close timer duration in seconds
    auto_close_seconds: u32,
}

impl WorkerTabMetadata {
    /// Create new metadata for a running worker
    fn new() -> Self {
        Self {
            completed_at: None,
            last_viewed_at: None,
            auto_close_seconds: 30,
        }
    }

    /// Mark worker as completed
    fn mark_completed(&mut self) {
        self.completed_at = Some(std::time::Instant::now());
    }

    /// Update last viewed timestamp (resets timer)
    fn mark_viewed(&mut self) {
        self.last_viewed_at = Some(std::time::Instant::now());
    }

    /// Check if worker should be auto-closed
    fn should_auto_close(&self) -> bool {
        if let Some(completed_at) = self.completed_at {
            let reference_time = self.last_viewed_at.unwrap_or(completed_at);
            reference_time.elapsed().as_secs() >= self.auto_close_seconds as u64
        } else {
            false
        }
    }

    /// Get remaining seconds until auto-close
    fn remaining_seconds(&self) -> Option<u32> {
        if let Some(completed_at) = self.completed_at {
            let reference_time = self.last_viewed_at.unwrap_or(completed_at);
            let elapsed = reference_time.elapsed().as_secs();
            if elapsed < self.auto_close_seconds as u64 {
                Some(self.auto_close_seconds - elapsed as u32)
            } else {
                Some(0)
            }
        } else {
            None
        }
    }
}

pub struct AgentManagerWindow {
    open: bool,

    // AWS Identity for agent execution
    aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>,

    // Selection state - which agent is displayed in right pane
    selected_agent_id: Option<AgentId>,

    // Tab state - which conversation is shown (manager or specific worker)
    // When viewing a TaskManager, this determines which tab's conversation to display
    selected_tab_agent_id: Option<AgentId>,

    // Worker tab metadata for auto-close timers
    worker_tabs: HashMap<AgentId, WorkerTabMetadata>,

    // Agent name editing (reserved for future use)
    _editing_agent_name: Option<AgentId>,
    _temp_agent_name: String,

    // Agent log viewer
    agent_log_window: AgentLogWindow,

    // Agents
    agents: HashMap<AgentId, AgentInstance>,
    input_text: String,

    // Stood library debug traces toggle
    stood_traces_enabled: bool,

    // UI event receiver for agent framework events
    ui_event_receiver: Arc<Mutex<Receiver<AgentUIEvent>>>,

    // Agent creation request receiver
    agent_creation_receiver: Arc<Mutex<Receiver<AgentCreationRequest>>>,
}

impl AgentManagerWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            aws_identity: None,
            selected_agent_id: None,
            selected_tab_agent_id: None,
            worker_tabs: HashMap::new(),
            _editing_agent_name: None,
            _temp_agent_name: String::new(),
            agent_log_window: AgentLogWindow::new(),
            agents: HashMap::new(),
            input_text: String::new(),
            stood_traces_enabled: true, // Default: enabled (matches main.rs init_logging)
            ui_event_receiver: get_ui_event_receiver(), // UI event channel
            agent_creation_receiver: get_agent_creation_receiver(), // Agent creation channel
        }
    }

    /// Set AWS Identity for agent execution
    pub fn set_aws_identity(&mut self, aws_identity: Arc<Mutex<AwsIdentityCenter>>) {
        self.aws_identity = Some(aws_identity);
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

    /// Get all worker agents for a given TaskManager parent
    fn get_workers_for_manager(&self, manager_id: AgentId) -> Vec<AgentId> {
        self.agents
            .iter()
            .filter(|(_, agent)| {
                matches!(agent.agent_type(), AgentType::TaskWorker { parent_id } if parent_id == manager_id)
            })
            .map(|(id, _)| *id)
            .collect()
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

                    // Stood traces toggle checkbox
                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(&mut self.stood_traces_enabled, "Stood Debug")
                            .on_hover_text("Toggle stood library debug traces in application log")
                            .changed()
                        {
                            // Call toggle function from main.rs
                            crate::toggle_stood_traces(self.stood_traces_enabled);
                            tracing::info!("Stood traces toggled: {}", self.stood_traces_enabled);
                        }
                    });

                    ui.add_space(5.0);

                    // Collect agent info
                    let agent_list: Vec<(AgentId, String)> = self
                        .agents
                        .iter()
                        .map(|(agent_id, agent)| (*agent_id, agent.metadata().name.clone()))
                        .collect();

                    let mut clicked_agent_id: Option<AgentId> = None;

                    // Agent list scroll area - takes remaining vertical space automatically
                    // Reserve space for [+] button at bottom
                    let scroll_height = ui.available_height() - 40.0;

                    ScrollArea::vertical()
                        .id_salt("agent_list_scroll")
                        .max_height(scroll_height)
                        .show(ui, |ui| {
                            if agent_list.is_empty() {
                                ui.label(RichText::new("No agents").weak());
                            } else {
                                for (agent_id, name) in agent_list {
                                    let is_selected = self.selected_agent_id == Some(agent_id);

                                    // Agent list item - use full width for better clickability
                                    let response = ui.selectable_label(is_selected, name);

                                    if response.clicked() {
                                        clicked_agent_id = Some(agent_id);
                                    }
                                }
                            }
                        });

                    // Handle selection after scroll area
                    if let Some(agent_id) = clicked_agent_id {
                        self.select_agent(agent_id);
                    }

                    // [+] New Agent button at bottom
                    if ui.button("+ New Agent").clicked() {
                        log::info!("New Agent button clicked");
                        self.create_new_agent();
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

        log::info!("Creating new agent: {} (current agent count: {})", default_name, agent_count);

        let metadata = AgentMetadata {
            name: default_name.clone(),
            description: "New agent".to_string(),
            model_id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(), // Claude Sonnet 3.5 (same as V1)
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Always create TaskManager agents
        // Future: Add UI for selecting agent type
        let mut agent = AgentInstance::new(metadata, AgentType::TaskManager);
        let agent_id = agent.id();

        // Initialize agent with AWS credentials
        if let Some(aws_identity) = &self.aws_identity {
            // Extract result first to drop lock before using self
            let init_result = agent.initialize(&mut *aws_identity.lock().unwrap());
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
                        let response =
                            AgentCreationResponse::error(AgentId::new(), error.clone());
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

        // Verify parent agent exists
        if !self.agents.contains_key(&request.parent_id) {
            return Err(format!("Parent agent {} not found", request.parent_id));
        }

        // Get parent agent's model ID
        let parent_model_id = {
            let parent_agent = self.agents.get(&request.parent_id)
                .ok_or_else(|| format!("Parent agent {} not found", request.parent_id))?;
            parent_agent.metadata().model_id.clone()
        };

        // Generate agent name
        let worker_count = self
            .agents
            .values()
            .filter(|a| matches!(a.agent_type(), AgentType::TaskWorker { .. }))
            .count();
        let default_name = format!("Task Worker {}", worker_count + 1);

        // Create metadata with parent's model
        let metadata = AgentMetadata {
            name: default_name.clone(),
            description: format!("Task: {}", request.task_description),
            model_id: parent_model_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Get parent agent's logger to share with worker
        let parent_logger = {
            let parent_agent = self.agents.get(&request.parent_id)
                .ok_or_else(|| format!("Parent agent {} not found", request.parent_id))?;
            parent_agent.logger().clone()
        };

        // Create TaskWorker agent with parent_id and parent's logger
        let agent_type = AgentType::TaskWorker {
            parent_id: request.parent_id,
        };
        let mut agent = AgentInstance::new_with_parent_logger(
            metadata,
            agent_type,
            parent_logger,
        );
        let agent_id = agent.id();

        // Initialize agent with AWS credentials
        if let Some(aws_identity) = &self.aws_identity {
            let init_result = agent.initialize(&mut *aws_identity.lock().unwrap());
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

        // Insert agent into map
        self.agents.insert(agent_id, agent);

        // Create worker tab metadata (but don't change focus)
        self.worker_tabs.insert(agent_id, WorkerTabMetadata::new());
        log::info!("Created worker tab for agent {} (not focused)", agent_id);

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
                if matches!(
                    agent.agent_type(),
                    AgentType::TaskWorker { .. }
                ) && agent.status() == &AgentStatus::Running
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

        // Build tab list for TaskManager: manager + workers
        let workers = self.get_workers_for_manager(agent_id);
        let has_workers = !workers.is_empty();
        let mut tabs = vec![agent_id]; // Manager first
        tabs.extend(workers.iter().copied());

        // Always show tab bar for TaskManager (shows at least Manager tab)
        // Only hide if manager has no messages AND no workers
        let agent_has_messages = self
            .agents
            .get(&agent_id)
            .map(|a| !a.messages().is_empty())
            .unwrap_or(false);
        let show_tabs = has_workers || agent_has_messages;

        if show_tabs {
            ui.horizontal(|ui| {
                ui.label("Conversations:");
                ui.add_space(5.0);

                // Manager tab
                let is_manager_tab = self.selected_tab_agent_id.is_none()
                    || self.selected_tab_agent_id == Some(agent_id);
                if ui.selectable_label(is_manager_tab, "Manager").clicked() {
                    self.selected_tab_agent_id = Some(agent_id);
                }

                // Worker tabs
                for (idx, worker_id) in workers.iter().enumerate() {
                    let is_selected = self.selected_tab_agent_id == Some(*worker_id);

                    // Create tab label - show countdown if completed
                    let label = if let Some(metadata) = self.worker_tabs.get(worker_id) {
                        if let Some(remaining) = metadata.remaining_seconds() {
                            format!("Completed - autoclose {}", remaining)
                        } else {
                            format!("Worker {}", idx + 1)
                        }
                    } else {
                        format!("Worker {}", idx + 1)
                    };

                    if ui.selectable_label(is_selected, label).clicked() {
                        self.selected_tab_agent_id = Some(*worker_id);

                        // Reset auto-close timer if user views a completed tab
                        if let Some(metadata) = self.worker_tabs.get_mut(worker_id) {
                            if metadata.completed_at.is_some() {
                                metadata.mark_viewed();
                                log::debug!(
                                    "User viewed completed worker {} tab - reset 30s timer",
                                    worker_id
                                );
                            }
                        }
                    }
                }
            });
            ui.add_space(5.0);
        }

        // Determine which agent's conversation to show
        let display_agent_id = if has_workers {
            // TaskManager with workers - show selected tab
            if let Some(tab_id) = self.selected_tab_agent_id {
                // Verify tab agent still exists
                if tabs.contains(&tab_id) {
                    tab_id
                } else {
                    // Tab agent no longer exists, reset to manager
                    self.selected_tab_agent_id = Some(agent_id);
                    agent_id
                }
            } else {
                // No tab selected, default to manager
                agent_id
            }
        } else {
            // No workers, show the selected agent
            agent_id
        };

        // Render UI and handle message sending/polling in a scope to release borrow
        let (terminate_clicked, log_clicked, _clear_clicked, _model_changed_to) = {
            // Get the agent to display
            let agent = match self.agents.get_mut(&display_agent_id) {
                Some(agent) => agent,
                None => {
                    ui.label(RichText::new("Agent not found").color(egui::Color32::RED));
                    return;
                }
            };

            // Render thechat UI and check if message should be sent, log clicked, clear clicked, or agent terminated
            let (should_send, log_clicked, clear_clicked, terminate_clicked, model_changed_to) =
                render_agent_chat(ui, agent, &mut self.input_text);

            // Send message if requested
            if should_send {
                let message = self.input_text.clone();
                self.input_text.clear();

                // Send message to agent
                log::info!("Sending message to agent {}: {}", agent_id, message);
                agent.send_message(message);
            }

            // Handle model change if requested
            if let Some(new_model_id) = &model_changed_to {
                agent.change_model(new_model_id.clone());
                // Clear conversation when changing models (context is lost)
                agent.clear_conversation();
                log::info!(
                    "Agent {} model changed to {} (conversation cleared)",
                    agent_id,
                    new_model_id
                );

                // Re-initialize agent with new model if AWS identity available
                if let Some(aws_identity) = &self.aws_identity {
                    if let Err(e) = agent.initialize(&mut *aws_identity.lock().unwrap()) {
                        log::error!("Failed to re-initialize agent with new model: {}", e);
                    }
                }
            }

            // Handle clear conversation if requested
            if clear_clicked {
                agent.clear_conversation();
                log::info!("Agent {} conversation cleared", agent_id);
            }

            // All controls are now inside render_agent_chat to prevent window growth
            (
                terminate_clicked,
                log_clicked,
                clear_clicked,
                model_changed_to,
            )
        }; // agent borrow released here

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
                            if last_msg.role == crate::app::agent_framework::ConversationRole::Assistant {
                                // Worker has completed - check if it's an error or success
                                let result = if last_msg.content.starts_with("Error: ") {
                                    // Strip "Error: " prefix and send as error
                                    Err(last_msg.content.strip_prefix("Error: ").unwrap_or(&last_msg.content).to_string())
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

            // Mark worker tab as completed (starts 30-second auto-close timer)
            if let Some(worker_tab) = self.worker_tabs.get_mut(&worker_id) {
                worker_tab.mark_completed();
                log::info!(
                    target: "agent::worker_complete",
                    "Marked worker {} tab as completed - 30 second auto-close timer started",
                    worker_id
                );
            }

            // NOTE: Worker agent and tab are NOT immediately removed
            // They will be auto-closed after 30 seconds (or when user manually closes)
            // This allows user to review completed worker's conversation
        }

        // Auto-close completed worker tabs after timeout
        let workers_to_close: Vec<AgentId> = self
            .worker_tabs
            .iter()
            .filter(|(_, metadata)| metadata.should_auto_close())
            .map(|(id, _)| *id)
            .collect();

        for worker_id in workers_to_close {
            log::info!(
                target: "agent::worker_complete",
                "Auto-closing worker {} tab after 30 second timeout",
                worker_id
            );

            // Remove worker agent
            self.agents.remove(&worker_id);

            // Remove worker tab metadata
            self.worker_tabs.remove(&worker_id);

            // Clear tab selection if closed worker was selected
            if self.selected_tab_agent_id == Some(worker_id) {
                // Find parent to switch to manager tab
                // Since we don't have parent_id here, just clear the tab selection
                // The UI will default to showing the manager
                self.selected_tab_agent_id = None;
            }

            // If the removed worker was selected in agent list, clear selection
            if self.selected_agent_id == Some(worker_id) {
                self.selected_agent_id = None;
            }
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
