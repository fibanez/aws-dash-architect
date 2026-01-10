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
//!
//! ## Soft-Maximize Feature
//!
//! This window implements a "soft-maximize" feature that can be reused in other windows.
//! Unlike traditional maximize which locks the window, soft-maximize simply sets the
//! window position and size to fill the available area below the menu bar.
//!
//! ### Key Requirements
//!
//! 1. **Maximize fills below menu**: When maximized, window position is set to (0, MENU_BAR_HEIGHT)
//!    and size fills the remaining screen area. The Dash menu bar remains visible.
//! 2. **Window remains interactive**: After maximizing, the window stays resizable, movable,
//!    and collapsible. User can manually resize/move away from maximized state.
//! 3. **Restore remembers position**: Before maximizing, the current position and size are saved.
//!    Clicking restore returns the window to its previous state.
//! 4. **Button in content area**: The maximize/restore button is rendered at the top-right
//!    of the window content (below the title bar), not in the title bar itself.
//!
//! ### Implementation Guide (for adding to other windows)
//!
//! 1. **Add import**: `use super::window_maximize::{WindowMaximizeState, MENU_BAR_HEIGHT};`
//!
//! 2. **Add field to struct**: `maximize_state: WindowMaximizeState,`
//!
//! 3. **Initialize in new()**: `maximize_state: WindowMaximizeState::new(),`
//!
//! 4. **In show/show_with_focus method**:
//!    ```ignore
//!    // Get window config based on maximize state
//!    let default_size = egui::Vec2::new(800.0, 600.0);
//!    let (pos, size, should_set_pos) = self.maximize_state.get_window_config(ctx, default_size);
//!
//!    // Configure window - keep resizable/movable/collapsible always true
//!    let mut window = egui::Window::new("Title")
//!        .resizable(true)
//!        .movable(true)
//!        .collapsible(true);
//!
//!    // Apply position/size based on state
//!    if self.maximize_state.is_maximized {
//!        window = window
//!            .default_size(size)
//!            .current_pos(pos.unwrap_or(egui::Pos2::new(0.0, MENU_BAR_HEIGHT)));
//!    } else {
//!        window = window.default_size(default_size);
//!        if should_set_pos { if let Some(p) = pos { window = window.current_pos(p); } }
//!    }
//!    ```
//!
//! 5. **Add maximize button at top of content**:
//!    ```ignore
//!    let response = window.show(ctx, |ui| {
//!        // Maximize button row - right-aligned
//!        ui.horizontal(|ui| {
//!            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
//!                if ui.button(self.maximize_state.button_label())
//!                    .on_hover_text(self.maximize_state.button_tooltip())
//!                    .clicked()
//!                { self.maximize_state.toggle(); }
//!            });
//!        });
//!        ui.separator();
//!        // ... rest of content
//!    });
//!    ```
//!
//! 6. **Save position for restore** (after window.show):
//!    ```ignore
//!    if !self.maximize_state.is_maximized {
//!        if let Some(inner) = response {
//!            let rect = inner.response.rect;
//!            self.maximize_state.save_restore_state(rect.min, rect.size());
//!        }
//!    }
//!    ```
//!
//! ### Button Labels
//!
//! - Maximize: `[ ]` (empty square) - tooltip "Maximize window"
//! - Restore: `[_]` (square with line) - tooltip "Restore window"

use super::agent_log_window::AgentLogWindow;
use super::window_focus::FocusableWindow;
use super::window_maximize::{WindowMaximizeState, MENU_BAR_HEIGHT};
use crate::app::agent_framework::{
    get_agent_creation_receiver, get_ui_event_receiver, render_agent_chat, AgentCreationRequest,
    AgentId, AgentInstance, AgentModel, AgentType, AgentUIEvent, InlineWorkerDisplay,
    ProcessingStatusWidget, StoodLogLevel,
};
use crate::app::aws_identity::AwsIdentityCenter;
use crate::{perf_checkpoint, perf_guard, perf_timed};
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

/// Status of a tool call within a worker
#[derive(Debug, Clone, PartialEq, Eq)]
enum ToolCallStatus {
    /// Tool is currently executing
    Running,
    /// Tool completed successfully
    Success,
    /// Tool failed with error message
    Failed(String),
}

/// Record of a single tool call by a worker agent
#[derive(Debug, Clone)]
struct ToolCallRecord {
    /// Name of the tool (e.g., "execute_javascript", "write_file")
    tool_name: String,
    /// Human-readable description of what this tool call is doing
    /// e.g., "Creating HTML structure for Lambda function table"
    intent: String,
    /// Tokens used specifically for this tool call (if available)
    tokens: Option<u32>,
    /// Current status of this tool call
    status: ToolCallStatus,
    /// When the tool call started
    started_at: std::time::Instant,
    /// When the tool call completed (if finished)
    completed_at: Option<std::time::Instant>,
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
    /// History of all tool calls made by this worker
    tool_calls: Vec<ToolCallRecord>,
    /// Worker status
    status: WorkerMessageStatus,
    /// Whether this is a ToolBuilder worker
    is_tool_builder: bool,
    /// Workspace name (for ToolBuilder workers)
    workspace_name: Option<String>,
    /// Pending tokens that arrived before any tool calls (to be attributed to next completion)
    pending_tokens: Option<u32>,
}

impl WorkerInlineMessage {
    /// Create a new running worker message
    fn new(
        worker_id: AgentId,
        parent_id: AgentId,
        short_description: String,
        is_tool_builder: bool,
        workspace_name: Option<String>,
    ) -> Self {
        Self {
            worker_id,
            parent_id,
            short_description,
            log_path: None,
            tool_calls: Vec::new(),
            status: WorkerMessageStatus::Running,
            is_tool_builder,
            workspace_name,
            pending_tokens: None,
        }
    }

    /// Mark as completed
    fn mark_completed(&mut self, success: bool) {
        self.status = WorkerMessageStatus::Completed { success };
    }

    /// Get total tokens across all tool calls
    fn total_tokens(&self) -> u32 {
        self.tool_calls.iter().filter_map(|c| c.tokens).sum()
    }

    /// Check if worker has any running tool calls
    fn has_running_tools(&self) -> bool {
        self.tool_calls.iter().any(|c| matches!(c.status, ToolCallStatus::Running))
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

    /// Soft-maximize state - see module docs for implementation guide
    /// Tracks: is_maximized, restore_pos, restore_size
    maximize_state: WindowMaximizeState,

    // Agent type selection dialog state
    show_agent_type_dialog: bool,
    selected_agent_type: Option<AgentType>,
    new_agent_name: String,
    tool_workspace_name: String,
    dialog_selected_model: AgentModel,
    dialog_selected_log_level: StoodLogLevel,
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
            maximize_state: WindowMaximizeState::new(),
            show_agent_type_dialog: false,
            selected_agent_type: None,
            new_agent_name: String::new(),
            tool_workspace_name: String::new(),
            dialog_selected_model: AgentModel::default(),
            dialog_selected_log_level: StoodLogLevel::default(),
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
        // Delegate to show_with_focus (which handles dialogs and log window)
        self.show_with_focus(ctx, (), false);
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
                    // ================================================================
                    // SCROLLABLE SIDEBAR
                    // Wrap entire left pane in ScrollArea with horizontal scrolling.
                    // This captures any content overflow (e.g., from longer model names
                    // in the dropdown) with a scrollbar instead of expanding the window.
                    // ================================================================
                    ScrollArea::both()
                        .id_salt("left_pane_scroll")
                        .show(ui, |ui| {
                            // [+] New Agent button
                            if ui.button("+ New Agent").clicked() {
                                log::info!("New Agent button clicked - showing agent creation dialog");
                                self.show_agent_type_dialog = true;
                                self.selected_agent_type = Some(AgentType::TaskManager); // Default to TaskManager
                                self.new_agent_name = format!("Agent {}", self.agents.len() + 1);
                                // Initialize dialog with current global settings
                                self.dialog_selected_model = self.selected_model;
                                self.dialog_selected_log_level = self.stood_log_level;
                            }

                            // Space after New Agent button
                            ui.add_space(10.0);

                            // Collect agent info (only show TaskManager agents)
                            // Worker agents (TaskWorker and ToolBuilderWorker) work in background
                            let agent_list: Vec<(AgentId, String)> = self
                                .agents
                                .iter()
                                .filter(|(_, agent)| {
                                    matches!(agent.agent_type(), AgentType::TaskManager)
                                })
                                .map(|(agent_id, agent)| (*agent_id, agent.metadata().name.clone()))
                                .collect();

                            let mut clicked_agent_id: Option<AgentId> = None;
                            let mut start_editing_id: Option<AgentId> = None;
                            let mut finish_editing = false;
                            let mut cancel_editing = false;

                            // Agent list - no nested scroll area needed since parent scrolls
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
                                        // Agent list item - larger button with better styling
                                        let button_text = RichText::new(&name).size(14.0);
                                        let button = egui::Button::new(button_text)
                                            .fill(if is_selected {
                                                ui.visuals().selection.bg_fill
                                            } else {
                                                ui.visuals().widgets.inactive.bg_fill
                                            })
                                            .min_size(egui::vec2(ui.available_width(), 32.0));

                                        let response = ui.add(button);

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

                            // Handle editing state changes
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
                                            agent.logger().update_agent_name(
                                                &agent.agent_type(),
                                                new_name.clone(),
                                            );
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

                            // Handle selection
                            if let Some(agent_id) = clicked_agent_id {
                                self.select_agent(agent_id);
                            }
                        }); // Close ScrollArea::both
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

    /// Show agent creation dialog (simplified - always creates TaskManager)
    fn show_agent_type_selection_dialog(&mut self, ctx: &Context) {
        let mut should_create = false;
        let mut open = self.show_agent_type_dialog;

        egui::Window::new("Create New Agent")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(400.0);

                ui.heading("New Agent");
                ui.add_space(10.0);

                // Agent Name
                ui.label("Agent Name:");
                ui.text_edit_singleline(&mut self.new_agent_name);
                ui.add_space(10.0);

                // Model selection
                ui.horizontal(|ui| {
                    ui.label("Model:");
                    egui::ComboBox::from_id_salt("dialog_model_selector")
                        .selected_text(self.dialog_selected_model.display_name())
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for model in AgentModel::all_models() {
                                ui.selectable_value(
                                    &mut self.dialog_selected_model,
                                    *model,
                                    model.display_name(),
                                );
                            }
                        });
                });
                ui.add_space(10.0);

                // Log level selection
                ui.horizontal(|ui| {
                    ui.label("Debug Level:");
                    egui::ComboBox::from_id_salt("dialog_log_level_selector")
                        .selected_text(self.dialog_selected_log_level.display_name())
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for level in StoodLogLevel::all() {
                                ui.selectable_value(
                                    &mut self.dialog_selected_log_level,
                                    *level,
                                    level.display_name(),
                                );
                            }
                        });
                });
                ui.add_space(15.0);

                // Create button
                let can_create = !self.new_agent_name.trim().is_empty();

                if ui
                    .add_enabled(can_create, egui::Button::new("Create Agent"))
                    .clicked()
                {
                    should_create = true;
                }
            });

        self.show_agent_type_dialog = open;

        // Create agent after dialog closes to avoid borrow conflicts
        if should_create {
            self.create_new_agent();
            self.show_agent_type_dialog = false;
        }
    }

    /// Create a new agent instance (always creates TaskManager)
    fn create_new_agent(&mut self) {
        let _timing = perf_guard!("create_new_agent");
        use crate::app::agent_framework::AgentMetadata;
        use chrono::Utc;

        // Use agent name from dialog or generate default
        let agent_name = if !self.new_agent_name.is_empty() {
            self.new_agent_name.clone()
        } else {
            format!("Agent {}", self.agents.len() + 1)
        };

        // Always create TaskManager agents
        let agent_type = AgentType::TaskManager;

        log::info!(
            "Creating new agent: {} with model {} and log level {}",
            agent_name,
            self.dialog_selected_model,
            self.dialog_selected_log_level
        );

        perf_checkpoint!("create_new_agent.building_metadata", &agent_name);

        let metadata = AgentMetadata {
            name: agent_name.clone(),
            description: "General purpose agent".to_string(),
            model: self.dialog_selected_model,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        perf_checkpoint!("create_new_agent.creating_agent_instance");
        let mut agent = perf_timed!("create_new_agent.AgentInstance_new", {
            AgentInstance::new(metadata, agent_type, None)
        });
        let agent_id = agent.id();

        // Set the log level from dialog on new agent
        agent.set_stood_log_level(self.dialog_selected_log_level);

        // Initialize agent with AWS credentials
        if let Some(aws_identity) = &self.aws_identity {
            perf_checkpoint!("create_new_agent.acquiring_aws_identity_lock");
            // Extract result first to drop lock before using self
            let init_result = perf_timed!("create_new_agent.agent_initialize", {
                agent.initialize(
                    &mut aws_identity.lock().unwrap(),
                    self.agent_logging_enabled,
                )
            });
            match init_result {
                Ok(_) => {
                    log::info!(
                        "Agent {} initialized successfully (ID: {})",
                        agent_name,
                        agent_id
                    );
                    perf_checkpoint!("create_new_agent.inserting_into_map");
                    self.agents.insert(agent_id, agent);
                    log::info!("Agent {} inserted into agents map", agent_id);
                    self.select_agent(agent_id);
                    log::info!("Agent {} selected and should now be visible", agent_id);
                    perf_checkpoint!("create_new_agent.complete");
                }
                Err(e) => {
                    log::error!("Failed to initialize agent {}: {}", agent_name, e);
                }
            }
        } else {
            log::error!("Cannot create agent: AWS Identity not set");
        }
    }

    /// Create an agent for editing an existing page
    ///
    /// This creates a TaskManager agent with the page workspace set,
    /// opens the page preview, and sends an initial message prompting
    /// the user to describe what changes they want to make.
    fn create_agent_for_page_edit(&mut self, page_name: String) {
        use crate::app::agent_framework::AgentMetadata;
        use chrono::Utc;

        let agent_name = format!("Edit: {}", page_name);
        let agent_type = AgentType::TaskManager;

        tracing::info!(
            "Creating agent for page edit: {} with model {}",
            agent_name,
            self.dialog_selected_model
        );

        let metadata = AgentMetadata {
            name: agent_name.clone(),
            description: format!("Editing page: {}", page_name),
            model: self.dialog_selected_model,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut agent = AgentInstance::new(metadata, agent_type, None);
        let agent_id = agent.id();

        // Set the page workspace so edit_page tool knows which page to modify
        agent.set_current_page_workspace(Some(page_name.clone()));

        // Set the log level
        agent.set_stood_log_level(self.dialog_selected_log_level);

        // Initialize agent with AWS credentials
        if let Some(aws_identity) = &self.aws_identity {
            let init_result =
                agent.initialize(&mut aws_identity.lock().unwrap(), self.agent_logging_enabled);

            match init_result {
                Ok(_) => {
                    tracing::info!(
                        "Page edit agent {} initialized successfully (ID: {})",
                        agent_name,
                        agent_id
                    );

                    // Insert agent and select it
                    self.agents.insert(agent_id, agent);
                    self.select_agent(agent_id);

                    // Open the window if not already open
                    self.open();

                    // Open page preview in a separate webview
                    let page_name_clone = page_name.clone();
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new()
                            .expect("Failed to create tokio runtime");
                        rt.block_on(async move {
                            let page_url = format!(
                                "wry://localhost/pages/{}/index.html",
                                page_name_clone
                            );
                            if let Err(e) = crate::app::webview::open_page_preview(
                                &page_name_clone,
                                &page_url,
                            )
                            .await
                            {
                                tracing::warn!("Failed to open page preview: {}", e);
                            }
                        });
                    });

                    tracing::info!(
                        "Page edit agent created for '{}'. User can now describe changes.",
                        page_name
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to initialize page edit agent {}: {}", agent_name, e);
                }
            }
        } else {
            tracing::error!("Cannot create page edit agent: AWS Identity not set");
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

        // Only log if we have events to process
        if !events.is_empty() {
            perf_checkpoint!(
                "UI.process_ui_events.start",
                &format!("event_count={}", events.len())
            );
        }

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
                // Worker progress events - inline display
                AgentUIEvent::WorkerStarted {
                    worker_id,
                    parent_id,
                    short_description,
                    message_index,
                    is_tool_builder,
                    workspace_name,
                } => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        worker_id = %worker_id,
                        parent_id = %parent_id,
                        short_description = %short_description,
                        message_index = message_index,
                        is_tool_builder = is_tool_builder,
                        workspace_name = ?workspace_name,
                        "UI event: Worker started"
                    );
                    self.handle_worker_started(
                        worker_id,
                        parent_id,
                        short_description,
                        message_index,
                        is_tool_builder,
                        workspace_name,
                    );
                }
                AgentUIEvent::WorkerToolStarted {
                    worker_id,
                    parent_id,
                    tool_name,
                    intent,
                } => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        worker_id = %worker_id,
                        parent_id = %parent_id,
                        tool_name = %tool_name,
                        intent = %intent,
                        "UI event: Worker tool started"
                    );
                    self.handle_worker_tool_started(worker_id, tool_name, intent);
                }
                AgentUIEvent::WorkerToolCompleted {
                    worker_id,
                    parent_id,
                    tool_name,
                    success,
                    tokens_used,
                } => {
                    tracing::debug!(
                        target: "agent::ui_events",
                        worker_id = %worker_id,
                        parent_id = %parent_id,
                        tool_name = %tool_name,
                        success = success,
                        tokens_used = ?tokens_used,
                        "UI event: Worker tool completed"
                    );
                    self.handle_worker_tool_completed(worker_id, tool_name, success, tokens_used);
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
                // Page management events
                AgentUIEvent::OpenPageForEdit { page_name } => {
                    tracing::info!(
                        target: "agent::ui_events",
                        page_name = %page_name,
                        "UI event: Open page for edit"
                    );
                    self.create_agent_for_page_edit(page_name);
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
        is_tool_builder: bool,
        workspace_name: Option<String>,
    ) {
        let mut message = WorkerInlineMessage::new(
            worker_id,
            parent_id,
            short_description,
            is_tool_builder,
            workspace_name,
        );

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
    /// Adds a new tool call record to the worker's history.
    fn handle_worker_tool_started(&mut self, worker_id: AgentId, tool_name: String, intent: String) {
        perf_checkpoint!(
            "manager.event_received.tool_started",
            &format!("worker={} tool={} intent={}", worker_id, tool_name, intent)
        );

        // Find the worker in any message index and add tool call record
        for (msg_idx, workers) in self.worker_inline_messages.iter_mut() {
            if let Some(worker) = workers.iter_mut().find(|w| w.worker_id == worker_id) {
                worker.tool_calls.push(ToolCallRecord {
                    tool_name: tool_name.clone(),
                    intent: intent.clone(),
                    tokens: None,
                    status: ToolCallStatus::Running,
                    started_at: std::time::Instant::now(),
                    completed_at: None,
                });

                perf_checkpoint!(
                    "manager.tool_started.recorded",
                    &format!("worker={} msg_idx={} tool={} total_calls={}",
                        worker_id, msg_idx, tool_name, worker.tool_calls.len())
                );
                return;
            }
        }

        perf_checkpoint!(
            "manager.tool_started.worker_not_found",
            &format!("worker={} tool={}", worker_id, tool_name)
        );
    }

    /// Handle worker tool completed event
    ///
    /// Updates the status of the most recent running tool call.
    fn handle_worker_tool_completed(
        &mut self,
        worker_id: AgentId,
        tool_name: String,
        success: bool,
        tokens: Option<u32>,
    ) {
        perf_checkpoint!(
            "manager.event_received.tool_completed",
            &format!("worker={} tool={} success={} tokens={:?}",
                worker_id, tool_name, success, tokens)
        );

        // Find the worker and update the last running tool call with this name
        for (msg_idx, workers) in self.worker_inline_messages.iter_mut() {
            if let Some(worker) = workers.iter_mut().find(|w| w.worker_id == worker_id) {
                // Find the last Running tool call with this name
                if let Some(call) = worker
                    .tool_calls
                    .iter_mut()
                    .filter(|c| c.tool_name == tool_name)
                    .filter(|c| matches!(c.status, ToolCallStatus::Running))
                    .last()
                {
                    let duration = call.started_at.elapsed();
                    call.status = if success {
                        ToolCallStatus::Success
                    } else {
                        ToolCallStatus::Failed("Tool execution failed".to_string())
                    };

                    // Use provided tokens, or pending tokens if available
                    call.tokens = tokens.or(worker.pending_tokens.take());
                    call.completed_at = Some(std::time::Instant::now());

                    perf_checkpoint!(
                        "manager.tool_completed.recorded",
                        &format!("worker={} msg_idx={} tool={} success={} duration_ms={} tokens={:?} intent={}",
                            worker_id, msg_idx, tool_name, success, duration.as_millis(), call.tokens, call.intent)
                    );
                } else {
                    perf_checkpoint!(
                        "manager.tool_completed.no_running_call",
                        &format!("worker={} tool={}", worker_id, tool_name)
                    );
                }
                return;
            }
        }

        perf_checkpoint!(
            "manager.tool_completed.worker_not_found",
            &format!("worker={} tool={}", worker_id, tool_name)
        );
    }

    /// Handle worker completed event
    ///
    /// Marks the worker as completed in its inline message and removes
    /// the worker agent from memory to free resources.
    fn handle_worker_completed(&mut self, worker_id: AgentId, success: bool) {
        // Find the worker and mark as completed
        for workers in self.worker_inline_messages.values_mut() {
            if let Some(worker) = workers.iter_mut().find(|w| w.worker_id == worker_id) {
                worker.mark_completed(success);
                tracing::info!(
                    target: "agent::ui_events",
                    worker_id = %worker_id,
                    success = success,
                    "Worker marked as completed in inline display"
                );
                break;
            }
        }

        // Remove worker agent from memory to free resources
        // Worker agents work in background and should not be kept after completion
        if self.agents.remove(&worker_id).is_some() {
            self.status_widgets.remove(&worker_id);
            tracing::info!(
                target: "agent::ui_events",
                worker_id = %worker_id,
                "Worker agent removed from memory after completion"
            );
        }
    }

    /// Handle worker tokens updated event
    ///
    /// Tokens can arrive either:
    /// 1. BEFORE any tools (initial model thinking) → buffer as pending_tokens
    /// 2. AFTER a tool batch completes → attribute to last completed call without tokens
    fn handle_worker_tokens_updated(&mut self, worker_id: AgentId, total_tokens: u32) {
        perf_checkpoint!(
            "manager.event_received.tokens_updated",
            &format!("worker={} tokens={}", worker_id, total_tokens)
        );

        // Find the worker
        for (msg_idx, workers) in self.worker_inline_messages.iter_mut() {
            if let Some(worker) = workers.iter_mut().find(|w| w.worker_id == worker_id) {
                // Try to find a completed tool call without tokens
                if let Some(call) = worker
                    .tool_calls
                    .iter_mut()
                    .filter(|c| !matches!(c.status, ToolCallStatus::Running))  // Completed
                    .filter(|c| c.tokens.is_none())  // No tokens yet
                    .last()
                {
                    // Attribute to this call
                    call.tokens = Some(total_tokens);

                    perf_checkpoint!(
                        "manager.tokens_updated.attributed_to_call",
                        &format!("worker={} msg_idx={} tool={} tokens={} intent={}",
                            worker_id, msg_idx, call.tool_name, total_tokens, call.intent)
                    );
                } else {
                    // No completed calls yet - buffer tokens for next completion
                    worker.pending_tokens = Some(total_tokens);

                    perf_checkpoint!(
                        "manager.tokens_updated.buffered",
                        &format!("worker={} tokens={} total_calls={} (tokens will be attributed to next completion)",
                            worker_id, total_tokens, worker.tool_calls.len())
                    );
                }
                return;
            }
        }

        perf_checkpoint!(
            "manager.tokens_updated.worker_not_found",
            &format!("worker={} tokens={}", worker_id, total_tokens)
        );
    }

    /// Convert worker inline messages to display format for rendering
    ///
    /// Filters workers for the given parent agent and converts them to InlineWorkerDisplay
    /// format, organized by message index.
    fn convert_workers_to_display(
        &self,
        parent_id: AgentId,
    ) -> HashMap<usize, Vec<InlineWorkerDisplay>> {
        use crate::app::agent_framework::ui::events::{
            ToolCallDisplayRecord, ToolCallStatus as DisplayStatus,
        };

        tracing::trace!(
            target: "agent::ui_render",
            parent_id = %parent_id,
            total_message_indices = self.worker_inline_messages.len(),
            "Converting workers to display format"
        );

        let mut result: HashMap<usize, Vec<InlineWorkerDisplay>> = HashMap::new();

        for (message_index, workers) in &self.worker_inline_messages {
            let workers_for_parent = workers.iter().filter(|w| w.parent_id == parent_id).count();

            if workers_for_parent > 0 {
                tracing::trace!(
                    target: "agent::ui_render",
                    parent_id = %parent_id,
                    message_index = message_index,
                    workers_count = workers_for_parent,
                    "Converting workers at message index"
                );
            }

            let filtered: Vec<InlineWorkerDisplay> = workers
                .iter()
                .filter(|w| w.parent_id == parent_id)
                .map(|w| {
                    // Convert internal ToolCallRecord to display format
                    let tool_calls = w
                        .tool_calls
                        .iter()
                        .map(|tc| ToolCallDisplayRecord {
                            tool_name: tc.tool_name.clone(),
                            intent: tc.intent.clone(),
                            tokens: tc.tokens,
                            status: match &tc.status {
                                ToolCallStatus::Running => DisplayStatus::Running,
                                ToolCallStatus::Success => DisplayStatus::Success,
                                ToolCallStatus::Failed(err) => DisplayStatus::Failed(err.clone()),
                            },
                            started_at: tc.started_at,
                            completed_at: tc.completed_at,
                        })
                        .collect::<Vec<_>>();

                    let total_tokens: u32 = tool_calls.iter().filter_map(|tc| tc.tokens).sum();

                    tracing::trace!(
                        target: "agent::ui_render",
                        worker_id = %w.worker_id,
                        short_description = %w.short_description,
                        tool_call_count = tool_calls.len(),
                        total_tokens = total_tokens,
                        is_running = w.status == WorkerMessageStatus::Running,
                        is_tool_builder = w.is_tool_builder,
                        "Converted worker to display format"
                    );

                    InlineWorkerDisplay {
                        short_description: w.short_description.clone(),
                        tool_calls,
                        is_running: w.status == WorkerMessageStatus::Running,
                        success: matches!(w.status, WorkerMessageStatus::Completed { success: true }),
                        log_path: w.log_path.clone(),
                        is_tool_builder: w.is_tool_builder,
                        workspace_name: w.workspace_name.clone(),
                    }
                })
                .collect();

            if !filtered.is_empty() {
                tracing::trace!(
                    target: "agent::ui_render",
                    parent_id = %parent_id,
                    message_index = message_index,
                    worker_count = filtered.len(),
                    "Added workers to result"
                );
                result.insert(*message_index, filtered);
            }
        }

        let total_workers: usize = result.values().map(|v| v.len()).sum();
        tracing::trace!(
            target: "agent::ui_render",
            parent_id = %parent_id,
            message_indices = result.len(),
            total_workers = total_workers,
            "Completed conversion to display format"
        );

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

        // Only log if we have requests to process
        if !requests.is_empty() {
            perf_checkpoint!(
                "UI.process_agent_creation_requests.start",
                &format!("request_count={}", requests.len())
            );
        }

        // Process each request
        for request in requests {
            tracing::debug!(
                target: "agent::creation",
                request_id = request.request_id(),
                parent_id = %request.parent_id(),
                "Processing agent creation request"
            );

            match self.handle_agent_creation_request(&request) {
                Ok(agent_id) => {
                    // Send success response
                    if let Some(response_sender) = take_response_channel(request.request_id()) {
                        let response = AgentCreationResponse::success(agent_id);
                        if let Err(e) = response_sender.send(response) {
                            tracing::error!(
                                target: "agent::creation",
                                request_id = request.request_id(),
                                error = %e,
                                "Failed to send agent creation success response"
                            );
                        }
                    }
                }
                Err(error) => {
                    // Send error response
                    if let Some(response_sender) = take_response_channel(request.request_id()) {
                        let response = AgentCreationResponse::error(AgentId::new(), error.clone());
                        if let Err(e) = response_sender.send(response) {
                            tracing::error!(
                                target: "agent::creation",
                                request_id = request.request_id(),
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
        perf_checkpoint!(
            "UI.handle_agent_creation_request.start",
            &format!(
                "parent_id={}, task={}",
                request.parent_id(),
                request.short_description().unwrap_or("Worker")
            )
        );
        let _creation_guard = perf_guard!("UI.handle_agent_creation_request");

        use crate::app::agent_framework::AgentMetadata;
        use chrono::Utc;

        // Verify parent agent exists and get its model
        let parent_model = {
            let parent_agent = self
                .agents
                .get(&request.parent_id())
                .ok_or_else(|| format!("Parent agent {} not found", request.parent_id()))?;
            parent_agent.metadata().model
        };

        // Get parent agent's logger to share with worker
        let parent_logger = {
            let parent_agent = self
                .agents
                .get(&request.parent_id())
                .ok_or_else(|| format!("Parent agent {} not found", request.parent_id()))?;
            parent_agent.logger().clone()
        };

        // Create agent based on request type
        let (agent_type, agent_name, short_description, initial_message) = match request {
            AgentCreationRequest::TaskWorker {
                short_description,
                task_description,
                ..
            } => {
                // Generate agent name
                let worker_count = self
                    .agents
                    .values()
                    .filter(|a| matches!(a.agent_type(), AgentType::TaskWorker { .. }))
                    .count();
                let default_name = format!("Task Worker {}", worker_count + 1);

                (
                    AgentType::TaskWorker {
                        parent_id: request.parent_id(),
                    },
                    default_name,
                    short_description.clone(),
                    task_description.clone(),
                )
            }

            AgentCreationRequest::ToolBuilderWorker {
                workspace_name,
                concise_description,
                task_description,
                resource_context,
                ..
            } => {
                // Generate agent name from workspace name
                let default_name = format!("Page Builder: {}", workspace_name);

                // Build full initial message with resource context if provided
                let initial_message = if let Some(context) = resource_context {
                    format!("{}\n\nResource Context: {}", task_description, context)
                } else {
                    task_description.clone()
                };

                (
                    AgentType::PageBuilderWorker {
                        parent_id: request.parent_id(),
                        workspace_name: workspace_name.clone(),
                    },
                    default_name,
                    concise_description.clone(),
                    initial_message,
                )
            }
        };

        // Create metadata (inherit parent's model)
        let metadata = AgentMetadata {
            name: agent_name.clone(),
            description: format!("Task: {}", request.task_description()),
            model: parent_model,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Create worker agent with parent_id and parent's logger
        perf_checkpoint!("UI.handle_agent_creation_request.create_worker_instance.start");
        let mut agent = perf_timed!("UI.handle_agent_creation_request.AgentInstance_new", {
            AgentInstance::new_with_parent_logger(metadata, agent_type, parent_logger, None)
        });
        let agent_id = agent.id();
        perf_checkpoint!(
            "UI.handle_agent_creation_request.create_worker_instance.end",
            &format!("worker_id={}", agent_id)
        );

        // Set the current stood log level on new worker agent
        agent.set_stood_log_level(self.stood_log_level);

        // Initialize agent with AWS credentials
        perf_checkpoint!("UI.handle_agent_creation_request.initialize_worker.start");
        if let Some(aws_identity) = &self.aws_identity {
            let init_result = perf_timed!("UI.handle_agent_creation_request.worker_initialize", {
                agent.initialize(
                    &mut aws_identity.lock().unwrap(),
                    self.agent_logging_enabled,
                )
            });
            if let Err(e) = init_result {
                perf_checkpoint!(
                    "UI.handle_agent_creation_request.initialize_worker.failed",
                    &format!("error={}", e)
                );
                return Err(format!("Failed to initialize agent: {}", e));
            }
        } else {
            return Err("AWS identity not available".to_string());
        }
        perf_checkpoint!("UI.handle_agent_creation_request.initialize_worker.end");

        // Send initial task message
        perf_checkpoint!(
            "UI.handle_agent_creation_request.send_task_message.start",
            &format!("task_len={}", initial_message.len())
        );
        agent.send_message(initial_message);
        perf_checkpoint!("UI.handle_agent_creation_request.send_task_message.end");

        tracing::info!(
            target: "agent::creation",
            agent_id = %agent_id,
            parent_id = %request.parent_id(),
            name = %agent_name,
            "Created worker agent"
        );

        // Determine the message index in parent's conversation where this worker was spawned
        let message_index = if let Some(parent) = self.agents.get(&request.parent_id()) {
            parent.messages().len().saturating_sub(1)
        } else {
            0
        };

        // Insert agent into map
        self.agents.insert(agent_id, agent);

        // Extract workspace info for ToolBuilder workers
        let (is_tool_builder, workspace_name) = match request {
            crate::app::agent_framework::AgentCreationRequest::ToolBuilderWorker {
                workspace_name,
                ..
            } => (true, Some(workspace_name.clone())),
            _ => (false, None),
        };

        // Send WorkerStarted event for inline display (replaces tab creation)
        let _ = crate::app::agent_framework::send_ui_event(
            crate::app::agent_framework::AgentUIEvent::worker_started(
                agent_id,
                request.parent_id(),
                short_description,
                message_index,
                is_tool_builder,
                workspace_name,
            ),
        );
        tracing::debug!(
            target: "agent::creation",
            agent_id = %agent_id,
            parent_id = %request.parent_id(),
            message_index = message_index,
            short_description = %request.short_description().unwrap_or("Worker"),
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
        self.status_widgets.entry(display_agent_id).or_default();

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
                perf_checkpoint!(
                    "UI.send_message.start",
                    &format!("agent_id={}, msg_len={}", agent_id, self.input_text.len())
                );
                let message = self.input_text.clone();
                self.input_text.clear();

                // Send message to agent
                log::info!("Sending message to agent {}: {}", agent_id, message);
                agent.send_message(message);
                perf_checkpoint!("UI.send_message.end", &format!("agent_id={}", agent_id));
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

            (
                terminate_clicked,
                log_clicked,
                clear_clicked,
                worker_log_clicked,
            )
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
            // If this is a TaskManager, also remove all its worker agents
            let workers_to_remove: Vec<AgentId> = self.agents
                .iter()
                .filter_map(|(worker_id, worker)| {
                    match worker.agent_type() {
                        AgentType::TaskWorker { parent_id } if *parent_id == agent_id => Some(*worker_id),
                        AgentType::PageBuilderWorker { parent_id, .. } if *parent_id == agent_id => Some(*worker_id),
                        _ => None,
                    }
                })
                .collect();

            // Remove workers first
            for worker_id in workers_to_remove {
                self.agents.remove(&worker_id);
                self.status_widgets.remove(&worker_id);
                log::info!("Worker agent {} terminated (parent terminated)", worker_id);
            }

            // Remove the manager agent
            self.agents.remove(&agent_id);
            self.status_widgets.remove(&agent_id);
            log::info!("Agent {} terminated and removed", agent_id);

            // Clear selections if this agent was selected
            if self.selected_agent_id == Some(agent_id) {
                self.selected_agent_id = None;
            }
            if self.selected_tab_agent_id == Some(agent_id) {
                self.selected_tab_agent_id = None;
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
        // Only log checkpoint on frame boundaries to reduce noise
        if frame % 60 == 0 {
            perf_checkpoint!("UI.poll_agent_responses.frame", &format!("frame={}", frame));
        }
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
                    perf_checkpoint!(
                        "UI.poll.response_received",
                        &format!(
                            "agent_id={}, poll_ms={}",
                            agent_id,
                            poll_duration.as_millis()
                        )
                    );

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
                                completed_workers.push((agent_id, *parent_id, result));
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
            perf_checkpoint!(
                "UI.poll.worker_complete.start",
                &format!("worker_id={}, parent_id={}", worker_id, parent_id)
            );

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
            perf_checkpoint!(
                "UI.poll.worker_complete.send_to_channel",
                &format!(
                    "worker_id={}, success={}, execution_time_ms={}",
                    worker_id,
                    is_success,
                    execution_time.as_millis()
                )
            );
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
                    worker_id, parent_id, is_success,
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
            self.status_widgets.remove(&worker_id);
            perf_checkpoint!(
                "UI.poll.worker_complete.end",
                &format!("worker_id={}", worker_id)
            );
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

        // ========================================================================
        // WINDOW CONFIGURATION
        // - Standard egui title bar with collapse and close buttons
        // - Resizable, movable, and collapsible
        // - Bottom panel prevents auto-growth (matches Explorer pattern)
        // ========================================================================

        // Get screen constraints - max size leaves room for menu bar
        let screen_rect = ctx.screen_rect();
        let max_width = screen_rect.width();
        let max_height = screen_rect.height() - MENU_BAR_HEIGHT;
        let default_size = egui::Vec2::new(800.0, 600.0);

        // IMPORTANT: Keep resizable/movable/collapsible TRUE even when maximized
        // This is "soft" maximize - user can still interact with the window
        let mut open = self.open; // Local copy for close button
        let mut window = egui::Window::new(self.window_title())
            .title_bar(true) // Standard egui title bar
            .open(&mut open) // Add close button (X)
            .resizable(true)
            .movable(true)
            .collapsible(true)
            .constrain(true);

        // Apply size/position based on maximize state and resize flag
        // When needs_resize is true, use fixed_size to force the window to target size
        if self.maximize_state.needs_resize() {
            // Force resize for this frame using target values calculated at toggle time
            if let Some(target_size) = self.maximize_state.target_size() {
                window = window.fixed_size(target_size);
            }
            if let Some(target_pos) = self.maximize_state.target_pos() {
                window = window.current_pos(target_pos);
            }
        } else if self.maximize_state.is_maximized {
            // Already maximized - use maximized size as default
            let maximized_size = egui::Vec2::new(max_width, max_height);
            window = window.default_size(maximized_size).min_size([600.0, 400.0]);
        } else {
            // ================================================================
            // WINDOW SIZE CONFIGURATION (matches Explorer pattern)
            // - default_size: Initial window dimensions
            // - min_size: Prevent shrinking too small
            // - NO max_size: Allow user to resize larger
            // - Bottom panel inside content: Prevents auto-growth
            // See: https://github.com/emilk/egui/discussions/610
            // ================================================================
            window = window.default_size(default_size).min_size([600.0, 400.0]);
            // Prevent shrinking too small
        }

        if bring_to_front {
            window = window.order(egui::Order::Foreground);
        }

        let response = window.show(ctx, |ui| {
            // ================================================================
            // BOTTOM PANEL - Anchors the bottom edge to prevent auto-growth
            // This pattern matches the Explorer window which uses a bottom
            // panel for status bar. The panel has fixed height and prevents
            // the window from growing frame-by-frame.
            // See: resource_explorer/window.rs line 291-293
            // ================================================================
            egui::TopBottomPanel::bottom("agent_manager_status_bar")
                .show_separator_line(true)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Show manager agent count (not workers)
                        let manager_count = self.agents.values()
                            .filter(|a| matches!(a.agent_type(), AgentType::TaskManager))
                            .count();
                        ui.label(format!("Agents: {}", manager_count));
                    });
                });

            // Main content fills remaining space
            self.ui_content(ui);
        });

        // Update open state from local copy (in case user clicked close button)
        self.open = open;

        // Clear resize flag after window is shown (size has been applied)
        if self.maximize_state.needs_resize() {
            self.maximize_state.clear_resize_flag();
        }

        // ================================================================
        // SAVE POSITION FOR RESTORE
        // Only save when NOT maximized - this captures the "normal" state
        // that we'll restore to when user clicks restore button
        // ================================================================
        if !self.maximize_state.is_maximized && !self.maximize_state.needs_resize() {
            if let Some(inner_response) = &response {
                let rect = inner_response.response.rect;
                self.maximize_state
                    .save_restore_state(rect.min, rect.size());
            }
        }

        // Show agent log window if open
        if self.agent_log_window.is_open() {
            self.agent_log_window.show(ctx, false);
        }

        // Show agent type selection dialog if open
        if self.show_agent_type_dialog {
            log::info!("Showing agent type selection dialog");
            self.show_agent_type_selection_dialog(ctx);
        }
    }
}
