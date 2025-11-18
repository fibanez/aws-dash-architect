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
use crate::app::agent_framework::{render_agent_chat, AgentId, AgentInstance};
use crate::app::aws_identity::AwsIdentityCenter;
use eframe::egui;
use egui::{Context, RichText, ScrollArea, Ui};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct AgentManagerWindow {
    open: bool,

    // AWS Identity for agent execution
    aws_identity: Option<Arc<Mutex<AwsIdentityCenter>>>,

    // Selection state - which agent is displayed in right pane
    selected_agent_id: Option<AgentId>,

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
}

impl AgentManagerWindow {
    pub fn new() -> Self {
        Self {
            open: false,
            aws_identity: None,
            selected_agent_id: None,
            _editing_agent_name: None,
            _temp_agent_name: String::new(),
            agent_log_window: AgentLogWindow::new(),
            agents: HashMap::new(),
            input_text: String::new(),
            stood_traces_enabled: true, // Default: enabled (matches main.rs init_logging)
        }
    }

    /// Set AWS Identity for agent execution
    pub fn set_aws_identity(&mut self, aws_identity: Arc<Mutex<AwsIdentityCenter>>) {
        self.aws_identity = Some(aws_identity);
    }

    /// Select an agent to display in the right pane
    pub fn select_agent(&mut self, agent_id: AgentId) {
        self.selected_agent_id = Some(agent_id);
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

        let metadata = AgentMetadata {
            name: default_name.clone(),
            description: "New agent".to_string(),
            model_id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(), // Claude Sonnet 3.5 (same as V1)
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut agent = AgentInstance::new(metadata);
        let agent_id = agent.id();

        // Initialize agent with AWS credentials
        if let Some(aws_identity) = &self.aws_identity {
            // Extract result first to drop lock before using self
            let init_result = agent.initialize(&mut *aws_identity.lock().unwrap());
            match init_result {
                Ok(_) => {
                    log::info!(
                        "Created and initialized Agent: {} (ID: {})",
                        default_name,
                        agent_id
                    );
                    self.agents.insert(agent_id, agent);
                    self.select_agent(agent_id);
                }
                Err(e) => {
                    log::error!("Failed to initialize Agent: {}", e);
                }
            }
        } else {
            log::error!("Cannot create Agent: AWS Identity not set");
        }
    }

    /// Render agent chat view in the right pane
    fn render_agent_chat_view(&mut self, ui: &mut Ui, agent_id: AgentId) {
        // Render UI and handle message sending/polling in a scope to release borrow
        let (terminate_clicked, log_clicked, _clear_clicked, _model_changed_to) = {
            // Get theagent
            let agent = match self.agents.get_mut(&agent_id) {
                Some(agent) => agent,
                None => {
                    ui.label(RichText::new("Agentnot found").color(egui::Color32::RED));
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

                // Send message if AWS identity available
                if let Some(aws_identity) = &self.aws_identity {
                    log::info!("Sendingmessage to agent {}: {}", agent_id, message);
                    agent.send_message(message, aws_identity);
                } else {
                    log::warn!("Cannot sendmessage: AWS Identity not set");
                }
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

            // Note: poll_response() is now called globally in poll_agent_responses()
            // But we still track when messages are displayed in the UI
            let message_count = agent.messages().len();

            // Log timing for UI rendering of messages
            if let Some(last_msg) = agent.messages().back() {
                let now_utc = chrono::Utc::now();
                let msg_created_at = last_msg.timestamp;
                let delay_ms = (now_utc - msg_created_at).num_milliseconds();

                log::debug!(
                    "[V2 UI RENDER] Agent {} displaying {} messages | Last message created: {} | Delay from creation: {}ms",
                    agent_id,
                    message_count,
                    msg_created_at.format("%H:%M:%S%.3f"),
                    delay_ms
                );
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
                    } else {
                        log::info!("[FRAME {}] [V2 POLL] Agent {} response retrieved | poll_response took: {:?}", frame, agent_id, poll_duration);
                    }
                    total_responses += 1;
                }
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
